mod iter_part;
mod iter_section;
mod iter_space_section;
use iter_part::PartIterator;
use iter_section::SectionIterator;
use iter_space_section::SpaceSectionIterator;

use crate::error::Error;
use crate::types::{Line, Part, Section, SimpleChord, Song};

pub fn load(path: &str) -> Result<Song, Error> {
    load_string(&std::fs::read_to_string(path)?)
}

pub fn load_string(input: &str) -> Result<Song, Error> {
    let mut title = None;
    let mut subtitle = None;
    let mut copyright = None;
    let mut key = None;
    let mut artist = None;
    let mut language = None;
    let mut tempo = None;
    let mut time = None;

    let sections = SectionIterator::new(
        input,
        &mut title,
        &mut subtitle,
        &mut copyright,
        &mut key,
        &mut artist,
        &mut language,
        &mut tempo,
        &mut time,
    )
    .map(|(keyword, lines)| {
        let lines = lines
            .iter()
            .filter(|line| line.len() > 0)
            .map(|line| {
                let parts = PartIterator::new(line, 96).collect::<Result<Vec<Part>, Error>>()?;
                Ok(Line::new(parts))
            })
            .collect::<Result<Vec<Line>, Error>>()?;
        Ok(Section::new(keyword.into(), lines))
    })
    .collect::<Result<Vec<Section>, Error>>()?;

    let sections = if !sections.is_empty() {
        sections
    } else {
        SpaceSectionIterator::new(input)
            .map(|(keyword, lines)| {
                let lines = lines
                    .iter()
                    .filter(|line| line.len() > 0)
                    .map(|line| {
                        let parts =
                            PartIterator::new(line, time.map(|(a, b)| 96 * a / b).unwrap_or(96))
                                .collect::<Result<Vec<Part>, Error>>()?;
                        Ok(Line::new(parts))
                    })
                    .collect::<Result<Vec<Line>, Error>>()?;
                Ok(Section::new(keyword.into(), lines))
            })
            .collect::<Result<Vec<Section>, Error>>()?
    };

    let key_str = key.ok_or(Error::Parse("no key given".into()))?;
    let key_str = key_str.trim();
    if key_str
        .chars()
        .next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(true)
    {
        return Err(Error::Parse(
            "key must be a letter name (e.g. C, G), not a Nashville number".into(),
        ));
    }
    let key_simple: SimpleChord = key_str.try_into()?;

    let title = title.ok_or(Error::Parse("no title given".into()))?;
    Ok(Song {
        title,
        subtitle,
        copyright,
        key: Some(key_simple),
        artist,
        language,
        tempo,
        time,
        sections,
    }
    .normalize()
    .clone())
}

#[cfg(test)]
mod tests {
    use crate::types::ChordRepresentation;

    use super::*;

    /// ChordPro with CCLI-style repeat markers [||:] and [:||] must import without error;
    /// markers are stripped and only real chords (e.g. [G][C][D]) are parsed.
    /// See: https://github.com/xilefmusics/chordlib/issues/6
    #[test]
    fn load_string_accepts_repeat_markers() {
        let input = r#"{title: Test}
{key: C}
{section: Verse}
[||:][G][C][D][ :||]
"#;
        let song = load_string(input).expect("import must not fail on repeat markers");
        assert_eq!(song.title.as_str(), "Test");
        assert_eq!(song.sections.len(), 1);
        assert_eq!(song.sections[0].lines.len(), 1);

        let key = song.key.as_ref().unwrap();
        let rep = ChordRepresentation::Default;
        let chord_parts: Vec<&crate::types::Chord> = song.sections[0].lines[0]
            .parts
            .iter()
            .filter_map(|p| p.chord.as_ref())
            .collect();
        assert_eq!(
            chord_parts.len(),
            3,
            "expected exactly three chords G, C, D"
        );
        assert_eq!(chord_parts[0].format(key, &rep), "G");
        assert_eq!(chord_parts[1].format(key, &rep), "C");
        assert_eq!(chord_parts[2].format(key, &rep), "D");
    }

    /// Nashville ChordPro: chords as numbers, key must be letter. Round-trip.
    /// See https://github.com/xilefmusics/chordlib/issues/12
    #[test]
    fn nashville_chord_pro_roundtrip() {
        let input = r#"{title: Nashville Test}
{key: C}
{section: Verse}
[1][4][5][1]
[1m][4][5]
"#;
        let song = load_string(input).expect("parse");
        assert_eq!(song.sections.len(), 1);
        let key = song.key.as_ref().unwrap();
        let rep = ChordRepresentation::Nashville;
        let line0: Vec<String> = song.sections[0].lines[0]
            .parts
            .iter()
            .filter_map(|p| p.chord.as_ref())
            .map(|c| c.format(key, &rep).to_string())
            .collect();
        assert_eq!(line0, ["1", "4", "5", "1"], "Nashville chords in key C");
        use crate::outputs::FormatChordPro;
        let out = (&song).format_chord_pro(None, Some(&rep), None, false);
        assert!(
            out.contains("{key:C}") || out.contains("{key: C}"),
            "key stays letter in output"
        );
        assert!(out.contains("[1]") && out.contains("[4]") && out.contains("[5]"));
        let again = load_string(&out).expect("round-trip");
        assert_eq!(again.key, song.key);
    }

    #[test]
    fn key_must_be_letter_rejects_nashville_number() {
        let input = r#"{title: Test}
{key: 1}
{section: Verse}
[ C]
"#;
        let r = load_string(input);
        assert!(r.is_err(), "{{key: 1}} must be rejected");
    }
}
