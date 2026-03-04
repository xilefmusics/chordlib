mod iter_part;
mod iter_section;
mod iter_space_section;
use iter_part::PartIterator;
use iter_section::SectionIterator;
use iter_space_section::SpaceSectionIterator;

use crate::error::Error;
use crate::types::{Line, Part, Section, Song};

/// If `line` is `{repeat}` or `{repeat: N}` (N ≥ 1), returns Some(repeat_count).
/// `{repeat}` → 4. Otherwise returns None (not a repeat directive).
/// Returns Err if it looks like a repeat directive but is invalid (e.g. `{repeat: 0}`).
fn parse_repeat_directive(line: &str) -> Result<Option<u32>, Error> {
    let line = line.trim();
    if !line.starts_with('{') || !line.ends_with('}') {
        return Ok(None);
    }
    let inner = line[1..line.len() - 1].trim();
    if inner == "repeat" {
        return Ok(Some(4));
    }
    if let Some(rest) = inner.strip_prefix("repeat:") {
        let n: u32 = rest
            .trim()
            .parse()
            .map_err(|_| Error::Parse("invalid repeat count".into()))?;
        if n >= 1 {
            Ok(Some(n))
        } else {
            Err(Error::Parse("repeat count must be at least 1".into()))
        }
    } else {
        Ok(None)
    }
}

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
        let raw: Vec<&str> = lines.iter().filter(|l| !l.is_empty()).copied().collect();
        let last_idx = raw.len().saturating_sub(1);
        for (i, line) in raw.iter().enumerate() {
            if parse_repeat_directive(line)?.is_some() && i != last_idx {
                return Err(Error::Parse(
                    "{repeat} or {repeat: N} only allowed on last line of section".into(),
                ));
            }
        }
        let (content_lines, repeat_count) = if let Some(last) = raw.last() {
            if let Some(n) = parse_repeat_directive(last)? {
                (&raw[..raw.len() - 1], n)
            } else {
                (raw.as_slice(), 1u32)
            }
        } else {
            (raw.as_slice(), 1u32)
        };
        let parsed = content_lines
            .iter()
            .map(|line| {
                PartIterator::new(line, 96)
                    .collect::<Result<Vec<Part>, Error>>()
                    .map(Line::new)
            })
            .collect::<Result<Vec<Line>, Error>>()?;
        Ok(Section::new_with_repeat(
            keyword.into(),
            parsed,
            repeat_count,
        ))
    })
    .collect::<Result<Vec<Section>, Error>>()?;

    let sections = if !sections.is_empty() {
        sections
    } else {
        SpaceSectionIterator::new(input)
            .map(|(keyword, lines)| {
                let raw: Vec<&str> = lines.iter().filter(|l| !l.is_empty()).copied().collect();
                let last_idx = raw.len().saturating_sub(1);
                for (i, line) in raw.iter().enumerate() {
                    if parse_repeat_directive(line)?.is_some() && i != last_idx {
                        return Err(Error::Parse(
                            "{repeat} or {repeat: N} only allowed on last line of section".into(),
                        ));
                    }
                }
                let (content_lines, repeat_count) = if let Some(last) = raw.last() {
                    if let Some(n) = parse_repeat_directive(last)? {
                        (&raw[..raw.len() - 1], n)
                    } else {
                        (raw.as_slice(), 1u32)
                    }
                } else {
                    (raw.as_slice(), 1u32)
                };
                let parsed = content_lines
                    .iter()
                    .map(|line| {
                        PartIterator::new(line, time.map(|(a, b)| 96 * a / b).unwrap_or(96))
                            .collect::<Result<Vec<Part>, Error>>()
                            .map(Line::new)
                    })
                    .collect::<Result<Vec<Line>, Error>>()?;
                Ok(Section::new_with_repeat(
                    keyword.into(),
                    parsed,
                    repeat_count,
                ))
            })
            .collect::<Result<Vec<Section>, Error>>()?
    };

    Ok(Song {
        title: title.ok_or(Error::Parse("no title given".into()))?,
        subtitle,
        copyright,
        key: Some((key.ok_or(Error::Parse("no key given".into()))?.as_str()).try_into()?),
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

    /// {repeat} and {repeat: N} on last line of section set section repeat_count.
    /// See https://github.com/xilefmusics/chordlib/issues/10
    #[test]
    fn repeat_directive_last_line() {
        let input = r#"{title: Test}
{key: C}
{section: Chorus}
[C][G][Am][F]
{repeat}
"#;
        let song = load_string(input).expect("parse");
        assert_eq!(song.sections.len(), 1);
        assert_eq!(song.sections[0].repeat_count, 4, "{{repeat}} defaults to 4");
        assert_eq!(song.sections[0].lines.len(), 1);

        let input_n = r#"{title: Test}
{key: C}
{section: Verse}
[G][C][D]
{repeat: 2}
"#;
        let song_n = load_string(input_n).expect("parse");
        assert_eq!(song_n.sections[0].repeat_count, 2);
    }

    #[test]
    fn repeat_directive_not_last_rejected() {
        let input = r#"{title: Test}
{key: C}
{section: Verse}
{repeat}
[G][C]
"#;
        let r = load_string(input);
        assert!(r.is_err(), "{{repeat}} not on last line must be rejected");
    }

    #[test]
    fn no_repeat_default_one() {
        let input = r#"{title: Test}
{key: C}
{section: Verse}
[G][C][D]
"#;
        let song = load_string(input).expect("parse");
        assert_eq!(song.sections[0].repeat_count, 1);
    }
}
