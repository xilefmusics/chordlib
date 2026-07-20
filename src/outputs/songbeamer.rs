//! Deterministic SongBeamer `.sng` output.

use std::collections::BTreeMap;

use base64::Engine as _;

use crate::Error;
use crate::types::{ChordRepresentation, Line, Section, SimpleChord, Song};

/// Format a song as a SongBeamer `#Version=3` document.
pub trait FormatSongBeamer {
    /// Return a UTF-8 document with a BOM and CRLF line endings.
    fn format_songbeamer(
        &self,
        key: Option<&SimpleChord>,
        representation: Option<&ChordRepresentation>,
    ) -> Result<Vec<u8>, Error>;
}

struct Body<'a> {
    title: String,
    marker: String,
    section: &'a Section,
}

impl FormatSongBeamer for &Song {
    fn format_songbeamer(
        &self,
        key: Option<&SimpleChord>,
        representation: Option<&ChordRepresentation>,
    ) -> Result<Vec<u8>, Error> {
        if self.title().is_empty() {
            return Err(Error::Other(
                "SongBeamer output requires a non-empty song title".into(),
            ));
        }
        let representation = representation.unwrap_or(&ChordRepresentation::Default);
        let render_key = key.or(self.key.as_ref()).cloned().unwrap_or_default();
        let (mut bodies, flow) = collect_bodies_and_flow(self)?;
        assign_markers(&mut bodies);
        let verse_order = flow
            .iter()
            .map(|body_index| bodies[*body_index].marker.as_str())
            .collect::<Vec<_>>()
            .join(",");
        let lang_count = song_language_count(self);

        let mut body_lines = Vec::new();
        let mut chord_records = Vec::new();
        let mut physical_line = -1i32;
        for body in &bodies {
            body_lines.push("---".to_string());
            physical_line += 1;
            body_lines.push(marker_line(&body.marker));
            physical_line += 1;
            for line in &body.section.lines {
                for language in 0..lang_count {
                    let (lyrics, chords) =
                        render_line(line, language, &render_key, representation, physical_line);
                    body_lines.push(lyrics);
                    if language == 0 {
                        chord_records.extend(chords);
                    }
                    physical_line += 1;
                }
            }
        }
        if body_lines.is_empty() {
            body_lines.push("---".to_string());
        }

        let chord_payload = if chord_records.is_empty() {
            None
        } else {
            let decoded = format!("{}\r", chord_records.join("\r"));
            Some(base64::engine::general_purpose::STANDARD.encode(decoded.as_bytes()))
        };

        let mut headers = vec![format!("#LangCount={lang_count}")];
        if !verse_order.is_empty() {
            headers.push(format!("#VerseOrder={verse_order}"));
        }
        headers.push(format!("#Title={}", clean_header_value(self.title())));
        for (index, title) in self.titles.iter().enumerate().skip(1) {
            if !title.is_empty() {
                headers.push(format!(
                    "#TitleLang{}={}",
                    index + 1,
                    clean_header_value(title)
                ));
            }
        }
        if let Some(subtitle) = &self.subtitle {
            headers.push(format!("#OTitle={}", clean_header_value(subtitle)));
        }
        if !self.artist().is_empty() {
            headers.push(format!("#Author={}", clean_header_value(self.artist())));
        }
        if key.is_some() || self.key.is_some() {
            headers.push(format!(
                "#Key={}",
                SimpleChord::default().format(&render_key, &ChordRepresentation::Default)
            ));
        }
        if let Some(tempo) = self.tempo {
            headers.push(format!("#Tempo={tempo}"));
        }
        if let Some((numerator, denominator)) = self.time {
            headers.push(format!("#Time={numerator}/{denominator}"));
        }
        if let Some(chords) = chord_payload {
            headers.push(format!("#Chords={chords}"));
        }
        append_preserved_headers(&mut headers, &self.tags);
        headers.push(format!("#Editor=chordlib {}", env!("CARGO_PKG_VERSION")));
        headers.push("#Version=3".to_string());
        if let Some(copyright) = &self.copyright {
            headers.push(format!("#(c)={}", clean_header_value(copyright)));
        }

        let document = headers
            .into_iter()
            .chain(body_lines)
            .collect::<Vec<_>>()
            .join("\r\n");
        let mut output = Vec::with_capacity(document.len() + 3);
        output.extend_from_slice(&[0xef, 0xbb, 0xbf]);
        output.extend_from_slice(document.as_bytes());
        output.extend_from_slice(b"\r\n");
        Ok(output)
    }
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
                marker: String::new(),
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

fn assign_markers(bodies: &mut [Body<'_>]) {
    let mut title_counts = BTreeMap::<String, usize>::new();
    for body in bodies.iter() {
        *title_counts.entry(body.title.clone()).or_default() += 1;
    }
    let mut occurrences = BTreeMap::<String, usize>::new();
    for (index, body) in bodies.iter_mut().enumerate() {
        let occurrence = occurrences.entry(body.title.clone()).or_default();
        *occurrence += 1;
        body.marker = if body.title.is_empty() {
            format!("Unknown {}", index + 1)
        } else if title_counts[&body.title] == 1 {
            body.title.clone()
        } else {
            format!("{} [chordlib:{}]", body.title, occurrence)
        };
    }
}

fn marker_line(marker: &str) -> String {
    if is_native_marker(marker) {
        marker.to_string()
    } else {
        format!("$$M={marker}")
    }
}

fn is_native_marker(marker: &str) -> bool {
    let words = marker.split_whitespace().collect::<Vec<_>>();
    if !(1..=2).contains(&words.len()) {
        return false;
    }
    const NAMES: &[&str] = &[
        "intro",
        "vers",
        "verse",
        "strophe",
        "pre-bridge",
        "bridge",
        "misc",
        "pre-refrain",
        "refrain",
        "pre-chorus",
        "chorus",
        "pre-coda",
        "zwischenspiel",
        "instrumental",
        "interlude",
        "coda",
        "ending",
        "ende",
        "outro",
        "teil",
        "part",
        "chor",
        "solo",
        "breakdown",
        "vamp",
        "turnaround",
        "tag",
        "andere",
        "unknown",
        "unbekannt",
        "unbenannt",
        "hidden",
        "invisible",
        "comment",
    ];
    NAMES.iter().any(|name| words[0].eq_ignore_ascii_case(name))
        && (words.len() == 1
            || words[1]
                .chars()
                .all(|character| character.is_alphanumeric()))
}

fn song_language_count(song: &Song) -> usize {
    song.sections
        .iter()
        .flat_map(|section| &section.lines)
        .flat_map(|line| &line.parts)
        .map(|part| part.languages.len())
        .chain(std::iter::once(song.languages.len()))
        .max()
        .unwrap_or(1)
        .max(1)
}

fn render_line(
    line: &Line,
    language: usize,
    key: &SimpleChord,
    representation: &ChordRepresentation,
    physical_line: i32,
) -> (String, Vec<String>) {
    let mut lyrics = String::new();
    let mut chords = Vec::new();
    for part in &line.parts {
        if language == 0
            && let Some(chord) = &part.chord
        {
            chords.push(format!(
                "{},{physical_line},{}",
                lyrics.chars().count(),
                chord.format(key, representation)
            ));
        }
        if let Some(text) = part.languages.get(language) {
            lyrics.push_str(text);
        }
    }
    (lyrics, chords)
}

fn clean_header_value(value: &str) -> String {
    value.replace(['\r', '\n'], " ")
}

fn append_preserved_headers(headers: &mut Vec<String>, tags: &BTreeMap<String, String>) {
    for (name, value) in tags {
        let Some(name) = name.strip_prefix("songbeamer.") else {
            continue;
        };
        if name.is_empty()
            || name.contains(['=', '\r', '\n'])
            || value.contains(['\r', '\n'])
            || is_reserved_header(name)
        {
            continue;
        }
        headers.push(format!("#{name}={value}"));
    }
}

fn is_reserved_header(name: &str) -> bool {
    matches!(
        name,
        "LangCount"
            | "VerseOrder"
            | "Title"
            | "Author"
            | "Key"
            | "Tempo"
            | "Time"
            | "Chords"
            | "Editor"
            | "Version"
            | "(c)"
            | "OTitle"
    ) || name.starts_with("TitleLang")
}

#[cfg(test)]
mod tests {
    use crate::inputs::chord_pro;
    use crate::inputs::songbeamer;

    use super::*;

    #[test]
    fn renders_bom_crlf_chords_and_semantic_round_trip() {
        let song = chord_pro::load_string(
            r#"{title: Example}
{key: C}
{artist: Writer}
{section: Verse 1}
[C]Hello [G]world
{section: Chorus}
[F]Sing
{section: Verse 1}
"#,
        )
        .expect("parse ChordPro");
        let output = (&song)
            .format_songbeamer(None, Some(&ChordRepresentation::Default))
            .expect("render SongBeamer");
        assert!(output.starts_with(&[0xef, 0xbb, 0xbf]));
        assert!(output.windows(2).any(|window| window == b"\r\n"));
        let text = std::str::from_utf8(&output[3..]).expect("UTF-8 output");
        assert!(text.contains("#Chords="));
        assert!(text.contains("#VerseOrder=Verse 1,Chorus,Verse 1"));

        let again = songbeamer::load_bytes(&output).expect("round-trip SongBeamer");
        assert_eq!(again.title(), "Example");
        assert_eq!(again.sections.len(), 3);
        assert!(again.sections[2].lines.is_empty());
        assert_eq!(again.sections[0].lines, song.sections[0].lines);
    }

    #[test]
    fn repeats_are_expanded_in_verse_order() {
        let mut song = chord_pro::load_string(
            "{title: Repeat}\n{key: C}\n{section: Chorus}\n[C]Sing\n{repeat: 3}\n",
        )
        .expect("parse ChordPro");
        song.sections[0].repeat_count = 3;
        let output = (&song)
            .format_songbeamer(None, None)
            .expect("render SongBeamer");
        let text = std::str::from_utf8(&output[3..]).expect("UTF-8");
        assert!(text.contains("#VerseOrder=Chorus,Chorus,Chorus"));
    }

    #[test]
    fn same_title_variants_and_unicode_chord_columns_round_trip() {
        let song = chord_pro::load_string(
            "{title: Varianten}\n{key: C}\n{section: Chorus}\n[C]Grüße [G]😊\n\
             {section: Chorus}\n[F]Anderer Text\n{section: Chorus}\n",
        )
        .expect("parse ChordPro variants");
        let output = (&song)
            .format_songbeamer(None, None)
            .expect("render SongBeamer");
        let text = std::str::from_utf8(&output[3..]).expect("UTF-8");
        assert!(text.contains("$$M=Chorus [chordlib:1]"));
        assert!(text.contains("$$M=Chorus [chordlib:2]"));

        let again = songbeamer::load_bytes(&output).expect("round-trip variants");
        assert_eq!(again.sections.len(), 3);
        assert_eq!(again.sections[0].title, "Chorus");
        assert_eq!(again.sections[1].title, "Chorus");
        assert_eq!(again.sections[0].lines, song.sections[0].lines);
        assert_eq!(again.sections[1].lines, song.sections[1].lines);
        assert!(again.sections[2].lines.is_empty());
    }
}
