mod iter_part;
mod iter_section;
mod iter_space_section;
use iter_part::PartIterator;
use iter_section::SectionIterator;
use iter_space_section::SpaceSectionIterator;

use crate::error::Error;
use crate::types::{Line, Part, Section, SimpleChord, Song};

/// If `line` is `{repeat}` or `{repeat: N}` (N ≥ 1), returns Some(repeat_count).
/// `{repeat}` → 2. Otherwise returns None (not a repeat directive).
/// Returns Err if it looks like a repeat directive but is invalid (e.g. `{repeat: 0}`).
fn parse_repeat_directive(line: &str) -> Result<Option<u32>, Error> {
    let line = line.trim();
    if !line.starts_with('{') || !line.ends_with('}') {
        return Ok(None);
    }
    let inner = line[1..line.len() - 1].trim();
    if inner == "repeat" {
        // `{repeat}` means: play this section twice in total.
        return Ok(Some(2));
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

fn build_section_from_lines<'a, F>(
    keyword: &str,
    lines: &[&'a str],
    make_iter: F,
) -> Result<Section, Error>
where
    F: Fn(&'a str) -> PartIterator<'a>,
{
    let raw: Vec<&'a str> = lines.iter().filter(|l| !l.is_empty()).copied().collect();
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
            make_iter(line)
                .collect::<Result<Vec<Part>, Error>>()
                .map(Line::new)
        })
        .collect::<Result<Vec<Line>, Error>>()?;

    Ok(Section::new_with_repeat(
        keyword.into(),
        parsed,
        repeat_count,
    ))
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
        build_section_from_lines(keyword, &lines, |line| PartIterator::new(line, 4000))
    })
    .collect::<Result<Vec<Section>, Error>>()?;

    let sections = if !sections.is_empty() {
        sections
    } else {
        SpaceSectionIterator::new(input)
            .map(|(keyword, lines)| {
                let bar_duration = time
                    .map(|(num, denom)| 1000 * num * 4 / denom)
                    .unwrap_or(4000);
                build_section_from_lines(keyword, &lines, |line| {
                    PartIterator::new(line, bar_duration)
                })
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
        assert_eq!(song.sections[0].repeat_count, 2, "{{repeat}} defaults to 2");
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

    /// Worship Pro export keeps {repeat} / {repeat: N}; ChordPro export uses {comment: (repeat)}.
    #[test]
    fn repeat_export_worship_pro_and_chord_pro() {
        let input = r#"{title: Test}
{key: C}
{section: Chorus}
[C][G][Am][F]
{repeat}
"#;
        let song = load_string(input).expect("parse");
        assert_eq!(song.sections[0].repeat_count, 2);

        use crate::outputs::FormatChordPro;
        let wp = (&song).format_chord_pro(None, None, None, true);
        assert!(wp.contains("{repeat}"), "Worship Pro export must contain {{repeat}}");

        let cp = (&song).format_chord_pro(None, None, None, false);
        assert!(
            cp.contains("{comment: (repeat)}"),
            "ChordPro export must contain {{comment: (repeat)}}"
        );

        let input_n = r#"{title: Test}
{key: C}
{section: Verse}
[G][C][D]
{repeat: 3}
"#;
        let song_n = load_string(input_n).expect("parse");
        assert_eq!(song_n.sections[0].repeat_count, 3);
        let wp_n = (&song_n).format_chord_pro(None, None, None, true);
        assert!(wp_n.contains("{repeat: 3}"));
        let cp_n = (&song_n).format_chord_pro(None, None, None, false);
        assert!(cp_n.contains("{comment: (repeat 3x)}"));
    }

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

    /// Worship Pro duration: parse clicks (decimal) as milliclicks; round-trip.
    /// See https://github.com/xilefmusics/chordlib/issues/9
    #[test]
    fn worship_pro_duration_roundtrip() {
        let input = r#"{title: Durations}
{key: C}
{section: Verse}
[C:4][Am:1.5][G:2][F:1]
"#;
        let song = load_string(input).expect("parse");
        assert_eq!(song.sections.len(), 1);
        let parts: Vec<_> = song.sections[0].lines[0]
            .parts
            .iter()
            .filter_map(|p| p.chord.as_ref())
            .collect();
        assert_eq!(parts.len(), 4);
        assert_eq!(parts[0].get_duration(), Some(4000));
        assert_eq!(parts[1].get_duration(), Some(1500));
        assert_eq!(parts[2].get_duration(), Some(2000));
        assert_eq!(parts[3].get_duration(), Some(1000));

        use crate::outputs::FormatChordPro;
        let out = (&song).format_chord_pro(
            None,
            Some(&ChordRepresentation::Default),
            None,
            true, // worship_pro
        );
        assert!(out.contains("[C:4]"), "integer clicks");
        assert!(out.contains("[Am:1.5]"), "decimal clicks");
        let again = load_string(&out).expect("round-trip parse");
        let again_parts: Vec<_> = again.sections[0].lines[0]
            .parts
            .iter()
            .filter_map(|p| p.chord.as_ref())
            .collect();
        assert_eq!(again_parts[0].get_duration(), Some(4000));
        assert_eq!(again_parts[1].get_duration(), Some(1500));
    }
}
