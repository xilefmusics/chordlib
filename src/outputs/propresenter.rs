//! Deterministic, experimental output for modern ProPresenter `.pro` files.

use std::collections::BTreeMap;

use prost::Message;
use uuid::Uuid;

use crate::Error;
use crate::propresenter_proto::*;
use crate::propresenter_rtf;
use crate::types::{ChordRepresentation, Line, Section, SimpleChord, Song};

/// Format a song as a modern protobuf-based ProPresenter `.pro` presentation.
pub trait FormatProPresenter {
    fn format_propresenter(
        &self,
        key: Option<&SimpleChord>,
        representation: Option<&ChordRepresentation>,
        language: Option<usize>,
    ) -> Result<Vec<u8>, Error>;
}

impl FormatProPresenter for &Song {
    fn format_propresenter(
        &self,
        key: Option<&SimpleChord>,
        representation: Option<&ChordRepresentation>,
        language: Option<usize>,
    ) -> Result<Vec<u8>, Error> {
        if self.title().trim().is_empty() {
            return Err(Error::Serialize(
                "ProPresenter output requires a non-empty song title".into(),
            ));
        }
        if self.sections.is_empty() {
            return Err(Error::Serialize(
                "ProPresenter output requires at least one song section".into(),
            ));
        }

        let render_key = key.or(self.key.as_ref()).cloned().unwrap_or_default();
        let representation = representation.copied().unwrap_or_default();
        let language = language.unwrap_or(0);
        let (bodies, flow) = collect_bodies_and_flow(self)?;
        if bodies.is_empty() {
            return Err(Error::Serialize(
                "ProPresenter output contains no section bodies".into(),
            ));
        }

        let seed = format!(
            "{}|{:?}|{representation:?}|{language}",
            serde_json::to_string(self)?,
            render_key
        );
        let document_uuid = stable_uuid(&seed, "presentation");
        let arrangement_uuid = stable_uuid(&seed, "arrangement");
        let mut cue_groups = Vec::with_capacity(bodies.len());
        let mut cues = Vec::with_capacity(bodies.len());
        let mut group_ids = Vec::with_capacity(bodies.len());

        for (index, body) in bodies.iter().enumerate() {
            let group_uuid = stable_uuid(&seed, &format!("group:{index}"));
            let cue_uuid = stable_uuid(&seed, &format!("cue:{index}"));
            let action_uuid = stable_uuid(&seed, &format!("action:{index}"));
            let slide_uuid = stable_uuid(&seed, &format!("slide:{index}"));
            let element_uuid = stable_uuid(&seed, &format!("element:{index}"));
            let rendered =
                render_lines(&body.section.lines, language, &render_key, representation)?;
            let slide = canonical_slide(rendered, slide_uuid, element_uuid, representation);
            cues.push(Cue {
                uuid: Some(cue_uuid.clone()),
                name: body.title.clone(),
                actions: vec![Action {
                    uuid: Some(action_uuid),
                    name: body.title.clone(),
                    is_enabled: true,
                    action_type: 11,
                    slide: Some(SlideType {
                        presentation: Some(PresentationSlide {
                            base_slide: Some(slide),
                        }),
                    }),
                }],
                is_enabled: true,
            });
            cue_groups.push(CueGroup {
                group: Some(Group {
                    uuid: Some(group_uuid.clone()),
                    name: body.title.clone(),
                    color: Some(group_color(index)),
                    application_group_identifier: None,
                    application_group_name: String::new(),
                }),
                cue_identifiers: vec![cue_uuid],
            });
            group_ids.push(group_uuid);
        }

        let arrangement_groups = flow.iter().map(|index| group_ids[*index].clone()).collect();
        let key_name = SimpleChord::default()
            .format(&render_key, &ChordRepresentation::Default)
            .to_string();
        let key_scale = MusicKeyScale {
            music_key: music_key_enum(render_key.pitch_class()),
            music_scale: 0,
        };
        let presentation = Presentation {
            application_info: Some(ApplicationInfo {
                platform: 0,
                platform_version: None,
                application: 1,
                application_version: None,
            }),
            uuid: Some(document_uuid),
            name: self.title().to_string(),
            category: self
                .tags
                .get("propresenter.category")
                .cloned()
                .unwrap_or_default(),
            notes: self
                .tags
                .get("propresenter.notes")
                .cloned()
                .unwrap_or_default(),
            selected_arrangement: Some(arrangement_uuid.clone()),
            arrangements: vec![Arrangement {
                uuid: Some(arrangement_uuid),
                name: "chordlib".into(),
                group_identifiers: arrangement_groups,
            }],
            cue_groups,
            cues,
            ccli: Some(ccli_from_song(self)),
            content_destination: 0,
            music_key: key_name.clone(),
            music: Some(Music {
                original_music_key: key_name.clone(),
                user_music_key: key_name,
                original: Some(key_scale.clone()),
                user: Some(key_scale),
            }),
        };
        Ok(presentation.encode_to_vec())
    }
}

struct Body<'a> {
    title: String,
    section: &'a Section,
}

fn collect_bodies_and_flow(song: &Song) -> Result<(Vec<Body<'_>>, Vec<usize>), Error> {
    let mut bodies = Vec::<Body<'_>>::new();
    let mut flow = Vec::new();
    for section in &song.sections {
        let body_index = if section.lines.is_empty() {
            bodies
                .iter()
                .position(|body| body.title == section.title)
                .ok_or_else(|| {
                    Error::InvalidSongFlow(format!(
                        "section reference {:?} appears before its body",
                        section.title
                    ))
                })?
        } else if let Some(index) = bodies
            .iter()
            .position(|body| body.title == section.title && body.section.lines == section.lines)
        {
            index
        } else {
            let index = bodies.len();
            bodies.push(Body {
                title: section.title.clone(),
                section,
            });
            index
        };
        for _ in 0..section.repeat_count.max(1) {
            flow.push(body_index);
        }
    }
    Ok((bodies, flow))
}

struct RenderedText {
    plain: String,
    chords: Vec<CustomAttribute>,
}

fn render_lines(
    lines: &[Line],
    language: usize,
    key: &SimpleChord,
    representation: ChordRepresentation,
) -> Result<RenderedText, Error> {
    let mut plain = String::new();
    let mut chord_starts = Vec::<(usize, String)>::new();
    for (line_index, line) in lines.iter().enumerate() {
        if line_index > 0 {
            plain.push('\n');
        }
        for part in &line.parts {
            if let Some(chord) = &part.chord {
                chord_starts.push((
                    plain.encode_utf16().count(),
                    chord.format(key, &representation),
                ));
            }
            plain.push_str(part.text_for_language(language));
        }
    }
    let text_end = plain.encode_utf16().count();
    let mut chords = Vec::with_capacity(chord_starts.len());
    for (index, (start, chord)) in chord_starts.iter().enumerate() {
        let end = chord_starts
            .get(index + 1)
            .map(|(start, _)| *start)
            .unwrap_or(text_end);
        chords.push(CustomAttribute {
            range: Some(IntRange {
                start: checked_i32(*start, "chord start")?,
                end: checked_i32(end, "chord end")?,
            }),
            chord: chord.clone(),
        });
    }
    Ok(RenderedText { plain, chords })
}

fn checked_i32(value: usize, context: &str) -> Result<i32, Error> {
    i32::try_from(value).map_err(|_| {
        Error::Serialize(format!(
            "ProPresenter {context} exceeds the supported text range"
        ))
    })
}

fn canonical_slide(
    rendered: RenderedText,
    slide_uuid: RvUuid,
    element_uuid: RvUuid,
    representation: ChordRepresentation,
) -> Slide {
    let has_chords = !rendered.chords.is_empty();
    let text = GraphicsText {
        attributes: Some(TextAttributes {
            font: Some(Font {
                name: "Arial".into(),
                size: 72.0,
                italic: false,
                bold: false,
                family: "Arial".into(),
                face: "Regular".into(),
            }),
            text_solid_fill: Some(Color {
                red: 1.0,
                green: 1.0,
                blue: 1.0,
                alpha: 1.0,
            }),
            paragraph_style: Some(ParagraphStyle {
                alignment: 2,
                line_height_multiple: 1.0,
            }),
            custom_attributes: rendered.chords,
        }),
        rtf_data: propresenter_rtf::encode(&rendered.plain),
        vertical_alignment: 1,
        scale_behavior: 2,
        margins: Some(EdgeInsets {
            left: 24.0,
            right: 24.0,
            top: 24.0,
            bottom: 24.0,
        }),
        chord_pro: has_chords.then_some(ChordProSettings {
            enabled: true,
            notation: match representation {
                ChordRepresentation::Default => 0,
                ChordRepresentation::Nashville => 1,
            },
            color: Some(Color {
                red: 0.4,
                green: 1.0,
                blue: 0.4,
                alpha: 1.0,
            }),
        }),
    };
    Slide {
        elements: vec![SlideElement {
            element: Some(GraphicsElement {
                uuid: Some(element_uuid),
                name: "Lyrics".into(),
                bounds: Some(Rect {
                    origin: Some(Point { x: 96.0, y: 96.0 }),
                    size: Some(Size {
                        width: 1728.0,
                        height: 888.0,
                    }),
                }),
                opacity: 1.0,
                text: Some(text),
                hidden: false,
            }),
            info: 2,
        }],
        size: Some(Size {
            width: 1920.0,
            height: 1080.0,
        }),
        uuid: Some(slide_uuid),
    }
}

fn ccli_from_song(song: &Song) -> Ccli {
    let tags = &song.tags;
    Ccli {
        author: tags
            .get("propresenter.ccli.author")
            .cloned()
            .unwrap_or_else(|| song.artist().to_string()),
        artist_credits: tags
            .get("propresenter.ccli.artist_credits")
            .cloned()
            .or_else(|| song.artists.get(1).cloned())
            .unwrap_or_default(),
        song_title: song.title().to_string(),
        publisher: tags
            .get("propresenter.ccli.publisher")
            .cloned()
            .or_else(|| song.copyright.clone())
            .unwrap_or_default(),
        copyright_year: parse_tag(tags, "propresenter.ccli.copyright_year"),
        song_number: parse_tag(tags, "propresenter.ccli.song_number"),
        display: tags
            .get("propresenter.ccli.display")
            .and_then(|value| value.parse().ok())
            .unwrap_or(false),
        album: tags
            .get("propresenter.ccli.album")
            .cloned()
            .unwrap_or_default(),
        artwork: Vec::new(),
    }
}

fn parse_tag(tags: &BTreeMap<String, String>, name: &str) -> u32 {
    tags.get(name)
        .and_then(|value| value.parse().ok())
        .unwrap_or(0)
}

fn music_key_enum(pitch_class: u8) -> i32 {
    const ENUMS: [i32; 12] = [1, 2, 4, 7, 8, 10, 11, 13, 16, 17, 19, 20];
    ENUMS[pitch_class as usize % 12]
}

fn stable_uuid(seed: &str, role: &str) -> RvUuid {
    RvUuid {
        string: Uuid::new_v5(&Uuid::NAMESPACE_URL, format!("{seed}|{role}").as_bytes()).to_string(),
    }
}

fn group_color(index: usize) -> Color {
    const COLORS: [(f32, f32, f32); 6] = [
        (0.23, 0.51, 0.96),
        (0.63, 0.36, 0.94),
        (0.93, 0.31, 0.47),
        (0.96, 0.62, 0.20),
        (0.25, 0.72, 0.50),
        (0.20, 0.68, 0.78),
    ];
    let (red, green, blue) = COLORS[index % COLORS.len()];
    Color {
        red,
        green,
        blue,
        alpha: 1.0,
    }
}

#[cfg(test)]
mod tests {
    use crate::inputs::{chord_pro, propresenter};

    use super::*;

    #[test]
    fn writes_deterministic_semantic_round_trip_with_native_chords() {
        let song = chord_pro::load_string(
            "{title: Grüße}\n{artist: Writer}\n{key: C}\n{section: Verse 1}\n[C]Hi 😊 [G/B]world\n{section: Chorus}\n[F]Sing\n{section: Verse 1}\n",
        )
        .expect("parse ChordPro");
        let first = (&song)
            .format_propresenter(None, None, None)
            .expect("format ProPresenter");
        let second = (&song)
            .format_propresenter(None, None, None)
            .expect("format ProPresenter again");
        assert_eq!(first, second);

        let decoded = Presentation::decode(first.as_slice()).expect("decode protobuf");
        let text = decoded.cues[0].actions[0]
            .slide
            .as_ref()
            .and_then(|slide| slide.presentation.as_ref())
            .and_then(|slide| slide.base_slide.as_ref())
            .and_then(|slide| slide.elements[0].element.as_ref())
            .and_then(|element| element.text.as_ref())
            .expect("lyrics text");
        assert!(!String::from_utf8_lossy(&text.rtf_data).contains("[C]"));
        assert_eq!(text.attributes.as_ref().unwrap().custom_attributes.len(), 2);

        let again = propresenter::load_bytes(&first).expect("round-trip ProPresenter");
        assert_eq!(again.title(), "Grüße");
        assert_eq!(again.sections.len(), 3);
        assert_eq!(again.sections[0].lines, song.sections[0].lines);
        assert!(again.sections[2].lines.is_empty());
    }

    #[test]
    fn expands_repeat_counts_in_selected_arrangement() {
        let mut song =
            chord_pro::load_string("{title: Repeat}\n{key: C}\n{section: Chorus}\n[C]Sing\n")
                .expect("parse ChordPro");
        song.sections[0].repeat_count = 3;
        let bytes = (&song)
            .format_propresenter(None, None, None)
            .expect("format ProPresenter");
        let decoded = Presentation::decode(bytes.as_slice()).expect("decode protobuf");
        assert_eq!(decoded.arrangements[0].group_identifiers.len(), 3);
    }
}
