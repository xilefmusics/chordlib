//! Experimental input support for modern protobuf-based ProPresenter `.pro` files.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use prost::Message;

use crate::Error;
use crate::propresenter_proto::{Ccli, Cue, CustomAttribute, GraphicsText, Presentation, RvUuid};
use crate::propresenter_rtf;
use crate::types::{Chord, Line, Part, Section, SimpleChord, Song};

/// Load a modern ProPresenter `.pro` presentation from disk.
pub fn load(path: impl AsRef<Path>) -> Result<Song, Error> {
    load_bytes(&std::fs::read(path)?)
}

/// Load a modern ProPresenter `.pro` presentation from protobuf bytes.
pub fn load_bytes(input: &[u8]) -> Result<Song, Error> {
    let presentation = Presentation::decode(input)
        .map_err(|error| Error::Parse(format!("invalid ProPresenter protobuf: {error}")))?;
    presentation_to_song(&presentation)
}

fn presentation_to_song(presentation: &Presentation) -> Result<Song, Error> {
    let ccli = presentation.ccli.as_ref();
    let title = ccli
        .map(|metadata| metadata.song_title.trim())
        .filter(|title| !title.is_empty())
        .unwrap_or_else(|| presentation.name.trim());
    if title.is_empty() {
        return Err(Error::Parse(
            "ProPresenter presentation has no song title or document name".into(),
        ));
    }

    let key = parse_presentation_key(presentation)?;
    let mut tags = BTreeMap::new();
    if !presentation.category.is_empty() {
        tags.insert(
            "propresenter.category".into(),
            presentation.category.clone(),
        );
    }
    if !presentation.notes.is_empty() {
        tags.insert("propresenter.notes".into(), presentation.notes.clone());
    }
    append_ccli_tags(&mut tags, ccli);

    let mut artists = Vec::new();
    if let Some(metadata) = ccli {
        if !metadata.author.trim().is_empty() {
            artists.push(metadata.author.clone());
        }
        if !metadata.artist_credits.trim().is_empty()
            && !artists
                .iter()
                .any(|artist| artist == &metadata.artist_credits)
        {
            artists.push(metadata.artist_credits.clone());
        }
    }

    let copyright = ccli.and_then(copyright_from_ccli);
    let cues = cue_index(&presentation.cues)?;
    let mut bodies = Vec::<ImportedGroup>::new();
    let mut all_group_ids = BTreeSet::new();
    let mut skipped_group_ids = BTreeSet::new();

    for (group_index, cue_group) in presentation.cue_groups.iter().enumerate() {
        let group = cue_group.group.as_ref().ok_or_else(|| {
            Error::Parse(format!(
                "ProPresenter cue group {} has no group metadata",
                group_index + 1
            ))
        })?;
        let group_id = required_uuid(group.uuid.as_ref(), "cue group")?;
        if !all_group_ids.insert(group_id.to_string()) {
            return Err(Error::Parse(format!(
                "duplicate ProPresenter cue group UUID {group_id}"
            )));
        }

        let mut lines = Vec::new();
        for cue_id in &cue_group.cue_identifiers {
            let cue_id = uuid_value(cue_id)?;
            let cue = cues.get(cue_id).ok_or_else(|| {
                Error::Parse(format!(
                    "ProPresenter cue group {:?} references unknown cue UUID {cue_id}",
                    group.name
                ))
            })?;
            if !cue.is_enabled {
                continue;
            }
            if let Some(mut cue_lines) = cue_lines(cue, key.as_ref())? {
                if !lines.is_empty() {
                    lines.push(empty_line());
                }
                lines.append(&mut cue_lines);
            }
        }

        if lines.is_empty() {
            skipped_group_ids.insert(group_id.to_string());
            continue;
        }
        bodies.push(ImportedGroup {
            uuid: group_id.to_string(),
            section: Section::new(group.name.clone(), lines),
        });
    }

    if bodies.is_empty() {
        return Err(Error::Parse(
            "ProPresenter presentation contains no enabled lyric slides".into(),
        ));
    }

    let sections =
        apply_selected_arrangement(presentation, &bodies, &all_group_ids, &skipped_group_ids)?;

    Ok(Song {
        titles: vec![title.to_string()],
        subtitle: None,
        copyright,
        key,
        artists,
        languages: Vec::new(),
        tempo: None,
        time: None,
        tags,
        sections,
    })
}

struct ImportedGroup {
    uuid: String,
    section: Section,
}

fn cue_index(cues: &[Cue]) -> Result<BTreeMap<&str, &Cue>, Error> {
    let mut index = BTreeMap::new();
    for cue in cues {
        let uuid = required_uuid(cue.uuid.as_ref(), "cue")?;
        if index.insert(uuid, cue).is_some() {
            return Err(Error::Parse(format!(
                "duplicate ProPresenter cue UUID {uuid}"
            )));
        }
    }
    Ok(index)
}

fn cue_lines(cue: &Cue, key: Option<&SimpleChord>) -> Result<Option<Vec<Line>>, Error> {
    for action in &cue.actions {
        let Some(slide) = action
            .slide
            .as_ref()
            .and_then(|slide| slide.presentation.as_ref())
            .and_then(|slide| slide.base_slide.as_ref())
        else {
            continue;
        };
        for element in &slide.elements {
            let Some(graphics) = element.element.as_ref() else {
                continue;
            };
            let Some(text) = graphics.text.as_ref().filter(|_| !graphics.hidden) else {
                continue;
            };
            if text.rtf_data.is_empty() {
                continue;
            }
            let plain = propresenter_rtf::decode(&text.rtf_data).map_err(|error| {
                Error::Parse(format!(
                    "invalid RTF in ProPresenter cue {:?}: {error}",
                    cue.name
                ))
            })?;
            if plain.trim().is_empty() {
                continue;
            }
            return Ok(Some(lines_with_chords(&plain, text, key)?));
        }
    }
    Ok(None)
}

fn lines_with_chords(
    plain: &str,
    text: &GraphicsText,
    key: Option<&SimpleChord>,
) -> Result<Vec<Line>, Error> {
    let notation = text
        .chord_pro
        .as_ref()
        .filter(|settings| settings.enabled)
        .map(|settings| settings.notation)
        .unwrap_or(0);
    let attributes = text.attributes.as_ref();
    let chords = attributes
        .map(|attributes| attributes.custom_attributes.as_slice())
        .unwrap_or_default();
    validate_chord_ranges(plain, chords)?;
    if !chords.is_empty() && !matches!(notation, 0 | 1) {
        return Err(Error::Parse(format!(
            "unsupported ProPresenter chord notation {notation}; chord and number notation are supported"
        )));
    }
    if !chords.is_empty() && notation == 1 && key.is_none() {
        return Err(Error::Parse(
            "ProPresenter number chords require a valid presentation key".into(),
        ));
    }

    let mut events = chords
        .iter()
        .filter(|attribute| !attribute.chord.trim().is_empty())
        .map(|attribute| {
            let range = attribute.range.as_ref().ok_or_else(|| {
                Error::Parse("ProPresenter chord attribute has no text range".into())
            })?;
            Ok((
                range.start as usize,
                Chord::from_str_with_key(attribute.chord.trim(), key)?,
            ))
        })
        .collect::<Result<Vec<_>, Error>>()?;
    events.sort_by_key(|(position, _)| *position);

    let mut result = Vec::new();
    let mut line_start = 0usize;
    for line in plain.split('\n') {
        let line_len = line.encode_utf16().count();
        let line_end = line_start + line_len;
        let line_events = events
            .iter()
            .filter(|(position, _)| *position >= line_start && *position <= line_end)
            .map(|(position, chord)| (*position - line_start, chord.clone()))
            .collect::<Vec<_>>();
        result.push(line_from_events(line, &line_events)?);
        line_start = line_end.saturating_add(1);
    }
    Ok(result)
}

fn validate_chord_ranges(text: &str, attributes: &[CustomAttribute]) -> Result<(), Error> {
    let text_len = text.encode_utf16().count();
    for (index, attribute) in attributes.iter().enumerate() {
        if attribute.chord.is_empty() {
            continue;
        }
        let range = attribute.range.as_ref().ok_or_else(|| {
            Error::Parse(format!(
                "ProPresenter chord attribute {} has no text range",
                index + 1
            ))
        })?;
        if range.start < 0 || range.end < range.start || range.end as usize > text_len {
            return Err(Error::Parse(format!(
                "ProPresenter chord attribute {} has invalid UTF-16 range {}..{} for text length {text_len}",
                index + 1,
                range.start,
                range.end
            )));
        }
        utf16_byte_index(text, range.start as usize)?;
        utf16_byte_index(text, range.end as usize)?;
    }
    Ok(())
}

fn line_from_events(line: &str, events: &[(usize, Chord)]) -> Result<Line, Error> {
    if events.is_empty() {
        return Ok(Line::new(vec![Part {
            chord: None,
            languages: vec![line.to_string()],
            comment: false,
        }]));
    }

    let mut parts = Vec::new();
    let first_byte = utf16_byte_index(line, events[0].0)?;
    if first_byte > 0 {
        parts.push(Part {
            chord: None,
            languages: vec![line[..first_byte].to_string()],
            comment: false,
        });
    }
    for (index, (position, chord)) in events.iter().enumerate() {
        let start = utf16_byte_index(line, *position)?;
        let end_position = events
            .get(index + 1)
            .map(|(position, _)| *position)
            .unwrap_or_else(|| line.encode_utf16().count());
        let end = utf16_byte_index(line, end_position)?;
        parts.push(Part {
            chord: Some(chord.clone()),
            languages: vec![line[start..end].to_string()],
            comment: false,
        });
    }
    Ok(Line::new(parts))
}

fn utf16_byte_index(text: &str, target: usize) -> Result<usize, Error> {
    if target == 0 {
        return Ok(0);
    }
    let mut units = 0usize;
    for (byte, character) in text.char_indices() {
        if units == target {
            return Ok(byte);
        }
        units += character.len_utf16();
        if units > target {
            return Err(Error::Parse(format!(
                "ProPresenter UTF-16 offset {target} splits a surrogate pair"
            )));
        }
    }
    if units == target {
        Ok(text.len())
    } else {
        Err(Error::Parse(format!(
            "ProPresenter UTF-16 offset {target} exceeds text length {units}"
        )))
    }
}

fn apply_selected_arrangement(
    presentation: &Presentation,
    bodies: &[ImportedGroup],
    all_group_ids: &BTreeSet<String>,
    skipped_group_ids: &BTreeSet<String>,
) -> Result<Vec<Section>, Error> {
    let Some(selected) = presentation.selected_arrangement.as_ref() else {
        return Ok(bodies.iter().map(|body| body.section.clone()).collect());
    };
    let selected = uuid_value(selected)?;
    let arrangement = presentation
        .arrangements
        .iter()
        .find(|arrangement| {
            arrangement
                .uuid
                .as_ref()
                .is_some_and(|uuid| uuid.string == selected)
        })
        .ok_or_else(|| {
            Error::InvalidSongFlow(format!(
                "selected ProPresenter arrangement UUID {selected} does not exist"
            ))
        })?;

    let body_index = bodies
        .iter()
        .map(|body| (body.uuid.as_str(), &body.section))
        .collect::<BTreeMap<_, _>>();
    let mut seen = BTreeSet::new();
    let mut sections = Vec::new();
    for group_id in &arrangement.group_identifiers {
        let group_id = uuid_value(group_id)?;
        if skipped_group_ids.contains(group_id) {
            continue;
        }
        if !all_group_ids.contains(group_id) {
            return Err(Error::InvalidSongFlow(format!(
                "ProPresenter arrangement {:?} references unknown group UUID {group_id}",
                arrangement.name
            )));
        }
        let template = body_index.get(group_id).ok_or_else(|| {
            Error::InvalidSongFlow(format!(
                "ProPresenter arrangement group UUID {group_id} has no lyric body"
            ))
        })?;
        let mut section = (*template).clone();
        if !seen.insert(group_id.to_string()) {
            section.lines.clear();
        }
        sections.push(section);
    }
    if sections.is_empty() {
        return Err(Error::InvalidSongFlow(format!(
            "selected ProPresenter arrangement {:?} contains no lyric groups",
            arrangement.name
        )));
    }
    Ok(sections)
}

fn parse_presentation_key(presentation: &Presentation) -> Result<Option<SimpleChord>, Error> {
    let mut candidates = Vec::new();
    if let Some(music) = &presentation.music {
        if let Some(user) = &music.user {
            candidates.push(music_key_scale_name(user));
        }
        if !music.user_music_key.trim().is_empty() {
            candidates.push(music.user_music_key.trim().to_string());
        }
        if let Some(original) = &music.original {
            candidates.push(music_key_scale_name(original));
        }
        if !music.original_music_key.trim().is_empty() {
            candidates.push(music.original_music_key.trim().to_string());
        }
    }
    if !presentation.music_key.trim().is_empty() {
        candidates.push(presentation.music_key.trim().to_string());
    }

    if let Some(candidate) = candidates.first() {
        return parse_key_name(candidate).map(Some);
    }
    Ok(None)
}

fn parse_key_name(value: &str) -> Result<SimpleChord, Error> {
    let token = value
        .split_whitespace()
        .next()
        .unwrap_or(value)
        .trim_end_matches(['m', 'M']);
    SimpleChord::try_from(token)
        .map_err(|_| Error::Parse(format!("invalid ProPresenter music key {value:?}")))
}

fn music_key_scale_name(scale: &crate::propresenter_proto::MusicKeyScale) -> String {
    const NAMES: [&str; 21] = [
        "Ab", "A", "A#", "Bb", "B", "B#", "Cb", "C", "C#", "Db", "D", "D#", "Eb", "E", "E#", "Fb",
        "F", "F#", "Gb", "G", "G#",
    ];
    usize::try_from(scale.music_key)
        .ok()
        .and_then(|index| NAMES.get(index))
        .copied()
        .unwrap_or("")
        .to_string()
}

fn append_ccli_tags(tags: &mut BTreeMap<String, String>, ccli: Option<&Ccli>) {
    let Some(ccli) = ccli else { return };
    for (name, value) in [
        ("author", ccli.author.as_str()),
        ("artist_credits", ccli.artist_credits.as_str()),
        ("publisher", ccli.publisher.as_str()),
        ("album", ccli.album.as_str()),
    ] {
        if !value.is_empty() {
            tags.insert(format!("propresenter.ccli.{name}"), value.to_string());
        }
    }
    if ccli.copyright_year != 0 {
        tags.insert(
            "propresenter.ccli.copyright_year".into(),
            ccli.copyright_year.to_string(),
        );
    }
    if ccli.song_number != 0 {
        tags.insert(
            "propresenter.ccli.song_number".into(),
            ccli.song_number.to_string(),
        );
    }
    tags.insert("propresenter.ccli.display".into(), ccli.display.to_string());
}

fn copyright_from_ccli(ccli: &Ccli) -> Option<String> {
    match (ccli.copyright_year, ccli.publisher.trim()) {
        (0, "") => None,
        (0, publisher) => Some(publisher.to_string()),
        (year, "") => Some(format!("© {year}")),
        (year, publisher) => Some(format!("© {year} {publisher}")),
    }
}

fn required_uuid<'a>(uuid: Option<&'a RvUuid>, context: &str) -> Result<&'a str, Error> {
    let uuid = uuid.ok_or_else(|| Error::Parse(format!("ProPresenter {context} has no UUID")))?;
    uuid_value(uuid)
}

fn uuid_value(uuid: &RvUuid) -> Result<&str, Error> {
    if uuid.string.trim().is_empty() {
        Err(Error::Parse("ProPresenter UUID is empty".into()))
    } else {
        Ok(uuid.string.as_str())
    }
}

fn empty_line() -> Line {
    Line::new(vec![Part {
        chord: None,
        languages: vec![String::new()],
        comment: false,
    }])
}

#[cfg(test)]
mod tests {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;

    use super::*;
    use crate::propresenter_proto::{
        Action, Arrangement, ChordProSettings, CueGroup, GraphicsElement, Group, IntRange,
        PresentationSlide, Slide, SlideElement, SlideType, TextAttributes,
    };

    fn uuid(value: &str) -> RvUuid {
        RvUuid {
            string: value.into(),
        }
    }

    fn lyric_cue(id: &str, name: &str, text: &str, enabled: bool) -> Cue {
        Cue {
            uuid: Some(uuid(id)),
            name: name.into(),
            actions: vec![Action {
                uuid: None,
                name: String::new(),
                is_enabled: true,
                action_type: 11,
                slide: Some(SlideType {
                    presentation: Some(PresentationSlide {
                        base_slide: Some(Slide {
                            elements: vec![SlideElement {
                                element: Some(GraphicsElement {
                                    text: Some(GraphicsText {
                                        rtf_data: propresenter_rtf::encode(text),
                                        ..Default::default()
                                    }),
                                    ..Default::default()
                                }),
                                ..Default::default()
                            }],
                            ..Default::default()
                        }),
                    }),
                }),
            }],
            is_enabled: enabled,
        }
    }

    fn group(id: &str, name: &str, cues: &[&str]) -> CueGroup {
        CueGroup {
            group: Some(Group {
                uuid: Some(uuid(id)),
                name: name.into(),
                ..Default::default()
            }),
            cue_identifiers: cues.iter().map(|id| uuid(id)).collect(),
        }
    }

    #[test]
    fn applies_utf16_chord_ranges() {
        let text = GraphicsText {
            attributes: Some(TextAttributes {
                custom_attributes: vec![CustomAttribute {
                    range: Some(IntRange { start: 3, end: 7 }),
                    chord: "G".into(),
                }],
                ..Default::default()
            }),
            chord_pro: Some(ChordProSettings {
                enabled: true,
                ..Default::default()
            }),
            ..Default::default()
        };
        let lines = lines_with_chords("😊 Ahoj", &text, Some(&SimpleChord::default()))
            .expect("parse chord ranges");
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].parts[0].languages[0], "😊 ");
        assert!(lines[0].parts[1].chord.is_some());
        assert_eq!(lines[0].parts[1].languages[0], "Ahoj");
    }

    #[test]
    fn rejects_range_that_splits_surrogate_pair() {
        let attributes = [CustomAttribute {
            range: Some(IntRange { start: 1, end: 2 }),
            chord: "C".into(),
        }];
        assert!(validate_chord_ranges("😊", &attributes).is_err());
    }

    #[test]
    fn imports_metadata_unicode_slides_arrangement_repeats_and_skipped_groups() {
        let presentation = Presentation {
            name: "Document name".into(),
            category: "Songs".into(),
            notes: "Fixture note".into(),
            ccli: Some(Ccli {
                author: "Writer".into(),
                artist_credits: "Artist".into(),
                song_title: "Fixture title".into(),
                publisher: "Publisher".into(),
                copyright_year: 2026,
                song_number: 12345,
                display: true,
                album: "Album".into(),
                artwork: Vec::new(),
            }),
            music: Some(crate::propresenter_proto::Music {
                user_music_key: "D".into(),
                original_music_key: "C".into(),
                ..Default::default()
            }),
            selected_arrangement: Some(uuid("arrangement")),
            arrangements: vec![Arrangement {
                uuid: Some(uuid("arrangement")),
                name: "Selected".into(),
                group_identifiers: vec![uuid("verse"), uuid("skipped"), uuid("verse")],
            }],
            cue_groups: vec![
                group("verse", "Verse", &["slide-1", "slide-2"]),
                group("skipped", "Video", &["disabled", "media"]),
            ],
            cues: vec![
                lyric_cue("slide-1", "one", "Grüße 😊", true),
                lyric_cue("slide-2", "two", "Second slide", true),
                lyric_cue("disabled", "disabled", "Do not import", false),
                Cue {
                    uuid: Some(uuid("media")),
                    name: "media".into(),
                    is_enabled: true,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        let song = load_bytes(&presentation.encode_to_vec()).expect("import fixture");
        assert_eq!(song.title(), "Fixture title");
        assert_eq!(song.artists, ["Writer", "Artist"]);
        assert_eq!(song.tags["propresenter.category"], "Songs");
        assert_eq!(song.tags["propresenter.ccli.album"], "Album");
        assert_eq!(song.tags["propresenter.ccli.song_number"], "12345");
        assert_eq!(song.copyright.as_deref(), Some("© 2026 Publisher"));
        assert_eq!(
            song.key.as_ref().unwrap().pitch_class(),
            SimpleChord::try_from("D").unwrap().pitch_class()
        );
        assert_eq!(song.sections.len(), 2);
        assert_eq!(song.sections[0].lines.len(), 3);
        assert_eq!(song.sections[0].lines[0].parts[0].languages[0], "Grüße 😊");
        assert!(song.sections[0].lines[1].parts[0].languages[0].is_empty());
        assert_eq!(
            song.sections[0].lines[2].parts[0].languages[0],
            "Second slide"
        );
        assert!(song.sections[1].lines.is_empty());
    }

    #[test]
    fn imports_independently_encoded_wire_fixture() {
        let encoded = include_str!("../../tests/fixtures/propresenter/current-schema.pro.b64");
        let bytes = STANDARD
            .decode(encoded.trim())
            .expect("decode fixture base64");
        let song = load_bytes(&bytes).expect("import independent fixture");

        assert_eq!(song.title(), "Fixture title");
        assert_eq!(song.sections.len(), 2);
        assert_eq!(song.sections[0].lines.len(), 4);
        assert_eq!(song.sections[0].lines[0].parts[0].languages[0], "Grüe");
        assert!(song.sections[0].lines[0].parts[0].chord.is_some());
        assert_eq!(song.sections[0].lines[2].parts[0].languages[0], "Second");
        assert_eq!(song.sections[0].lines[3].parts[0].languages[0], "slide");
        assert!(song.sections[1].lines.is_empty());
    }

    #[test]
    fn falls_back_to_document_order_and_keeps_same_name_bodies_distinct() {
        let presentation = Presentation {
            name: "No arrangement".into(),
            cue_groups: vec![
                group("first", "Verse", &["first-cue"]),
                group("second", "Verse", &["second-cue"]),
            ],
            cues: vec![
                lyric_cue("first-cue", "first", "First", true),
                lyric_cue("second-cue", "second", "Second", true),
            ],
            ..Default::default()
        };
        let song = load_bytes(&presentation.encode_to_vec()).expect("import fixture");
        assert_eq!(song.sections.len(), 2);
        assert_eq!(song.sections[0].title, "Verse");
        assert_eq!(song.sections[1].title, "Verse");
        assert_ne!(song.sections[0].lines, song.sections[1].lines);
    }

    #[test]
    fn rejects_unresolved_selected_arrangement_group() {
        let presentation = Presentation {
            name: "Broken arrangement".into(),
            selected_arrangement: Some(uuid("arrangement")),
            arrangements: vec![Arrangement {
                uuid: Some(uuid("arrangement")),
                name: "Broken".into(),
                group_identifiers: vec![uuid("missing")],
            }],
            cue_groups: vec![group("verse", "Verse", &["cue"])],
            cues: vec![lyric_cue("cue", "cue", "Lyrics", true)],
            ..Default::default()
        };
        assert!(matches!(
            load_bytes(&presentation.encode_to_vec()),
            Err(Error::InvalidSongFlow(_))
        ));
    }
}
