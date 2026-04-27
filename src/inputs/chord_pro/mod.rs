mod iter_part;
mod iter_section;
mod iter_space_section;
use iter_part::PartIterator;
use iter_section::SectionIterator;
use iter_space_section::SpaceSectionIterator;

use std::collections::BTreeMap;

use crate::error::Error;
use crate::text::remove_space_separators;
use crate::types::{Line, Part, Section, SimpleChord, Song};

/// First `{key: ...}` directive in the file (same ordering as `SectionIterator`).
fn chordpro_key_directive(input: &str) -> Option<String> {
    for line in input.lines() {
        let line = line.trim();
        let Some(inner) = line.strip_prefix('{').and_then(|s| s.strip_suffix('}')) else {
            continue;
        };
        let Some((k, v)) = inner.split_once(':') else {
            continue;
        };
        if k.trim() == "key" {
            return Some(v.trim().to_string());
        }
    }
    None
}

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

    let mut parsed_lines: Vec<Line> = Vec::new();

    for line in content_lines.iter().copied() {
        let trimmed = line.trim_start();

        if let Some(rest) = trimmed.strip_prefix('&') {
            if parsed_lines.is_empty() {
                return Err(Error::Parse(
                    "Worship Pro &-line cannot be the first line of a section".into(),
                ));
            }

            let has_chords = rest.contains('[') && rest.contains(']');

            let last_line = parsed_lines
                .last_mut()
                .expect("parsed_lines is not empty, qed");

            if !has_chords {
                let new_lang_idx = last_line
                    .parts
                    .iter()
                    .map(|p| p.languages.len())
                    .max()
                    .unwrap_or(0);

                for part in &mut last_line.parts {
                    if part.languages.len() < new_lang_idx {
                        part.languages.resize(new_lang_idx, String::new());
                    }
                }

                let mut languages = vec![String::new(); new_lang_idx.saturating_add(1)];
                languages[new_lang_idx] = rest.to_string();

                last_line.parts.push(Part {
                    chord: None,
                    languages,
                    comment: false,
                });
            } else {
                let new_parts: Vec<Part> = make_iter(rest).collect::<Result<Vec<Part>, Error>>()?;

                let prev_chords: Vec<_> = last_line
                    .parts
                    .iter()
                    .filter_map(|p| p.chord.as_ref())
                    .collect();
                let new_chords: Vec<_> =
                    new_parts.iter().filter_map(|p| p.chord.as_ref()).collect();

                if prev_chords.len() != new_chords.len()
                    || !prev_chords
                        .iter()
                        .zip(new_chords.iter())
                        .all(|(a, b)| a == b)
                {
                    return Err(Error::Parse(
                        "Worship Pro &-line chord sequence must match previous line".into(),
                    ));
                }

                let new_lang_idx = last_line
                    .parts
                    .iter()
                    .map(|p| p.languages.len())
                    .max()
                    .unwrap_or(0);

                for (prev_part, new_part) in last_line.parts.iter_mut().zip(new_parts) {
                    if prev_part.languages.len() < new_lang_idx.saturating_add(1) {
                        prev_part
                            .languages
                            .resize(new_lang_idx.saturating_add(1), String::new());
                    }
                    let text = new_part.languages.first().cloned().unwrap_or_default();
                    prev_part.languages[new_lang_idx] = text;
                }
            }
        } else {
            let parts = make_iter(line).collect::<Result<Vec<Part>, Error>>()?;
            parsed_lines.push(Line::new(parts));
        }
    }

    Ok(Section::new_with_repeat(
        keyword.into(),
        parsed_lines,
        repeat_count,
    ))
}

pub fn load(path: &str) -> Result<Song, Error> {
    load_string(&std::fs::read_to_string(path)?)
}

pub fn load_string(input: &str) -> Result<Song, Error> {
    let mut titles = Vec::new();
    let mut subtitle = None;
    let mut copyright = None;
    let mut key = None;
    let mut artists = Vec::new();
    let mut languages = Vec::new();
    let mut tempo = None;
    let mut time = None;
    let mut tags = BTreeMap::new();

    let song_key = chordpro_key_directive(input)
        .as_ref()
        .map(|k| k.trim())
        .filter(|k| !k.is_empty())
        .and_then(|k| SimpleChord::try_from(k).ok());

    let sections = SectionIterator::new(
        input,
        &mut titles,
        &mut subtitle,
        &mut copyright,
        &mut key,
        &mut artists,
        &mut languages,
        &mut tempo,
        &mut time,
        &mut tags,
    )
    .map(|(keyword, lines)| {
        build_section_from_lines(keyword, &lines, |line| {
            PartIterator::new(line, 4000, song_key.clone())
        })
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
                    PartIterator::new(line, bar_duration, song_key.clone())
                })
            })
            .collect::<Result<Vec<Section>, Error>>()?
    };

    let key_stored = key.ok_or(Error::Parse("no key given".into()))?;
    let key_clean = remove_space_separators(&key_stored);
    let key_str = key_clean.as_ref().trim();
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

    if !titles.iter().any(|t| !t.is_empty()) {
        return Err(Error::Parse("no title given".into()));
    }
    let mut song = Song {
        titles,
        subtitle,
        copyright,
        key: Some(key_simple),
        artists,
        languages,
        tempo,
        time,
        tags,
        sections,
    };
    // Chords parsed with `song_key` are already key-relative; absolute roots need one normalize.
    if song_key.is_none() {
        song.normalize();
    }
    Ok(song)
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

    #[test]
    fn chordpro_preserves_enharmonic_slash_bass() {
        let input = r#"{title: Test}
{key: C}
{section: Verse}
[G#/B#]Lyric
"#;
        let song = load_string(input).expect("parse");
        use crate::outputs::FormatChordPro;
        let out = (&song).format_chord_pro(None, Some(&ChordRepresentation::Default), None, false);
        assert!(
            out.contains("[G#/B#]"),
            "expected G#/B# in export, got:\n{out}"
        );
    }

    #[test]
    fn chordpro_parse_line_with_many_slash_chords() {
        let input = r#"{title: Slash line}
{key: C}
{section: Verse}
[C/G][D/F#][G/B][Am/E][F/Cb][E/G][Bb/D]Lyrics here
"#;
        let song = load_string(input).expect("parse");
        assert_eq!(song.sections.len(), 1);
        assert_eq!(song.sections[0].lines.len(), 1);
        assert_eq!(song.sections[0].lines[0].parts.len(), 7);
        use crate::outputs::FormatChordPro;
        let out = (&song).format_chord_pro(None, Some(&ChordRepresentation::Default), None, false);
        assert!(
            out.contains("Verse") && out.contains("Lyrics here"),
            "unexpected export:\n{out}"
        );
    }

    /// `[]` after chords in a pipe bar (SongSelect-style layout) is a spacer, not a chord.
    #[test]
    fn chordpro_empty_brackets_inside_pipe_bar() {
        let input = r#"{title: Pipe + empty}
{key: B}
{section: Intro}
[|][C][D]     []
"#;
        let song = load_string(input).expect("parse");
        assert_eq!(song.sections[0].lines.len(), 1);
    }

    /// Copyright meta must round-trip with ChordPro spelling `{copyright:...}`, not `coptyright`.
    /// See https://github.com/xilefmusics/chordlib/issues/51
    #[test]
    fn copyright_directive_uses_correct_spelling_in_export() {
        let input = r#"{title: Test}
{key: C}
{copyright: © 2024 Example}
{section: Verse}
[C]Line
"#;
        let song = load_string(input).expect("parse");
        assert_eq!(
            song.copyright.as_deref(),
            Some("© 2024 Example"),
            "parser must accept {{copyright:...}}"
        );

        use crate::outputs::FormatChordPro;
        let cp = (&song).format_chord_pro(None, None, None, false);
        assert!(
            cp.contains("{copyright:© 2024 Example}"),
            "Chord Pro export must use {{copyright:...}}, got:\n{cp}"
        );
        assert!(
            !cp.contains("coptyright"),
            "must not emit misspelled coptyright"
        );

        let wp = (&song).format_chord_pro(None, None, None, true);
        assert!(
            wp.contains("{copyright: © 2024 Example}"),
            "Worship Pro export must use {{copyright: ...}}, got:\n{wp}"
        );
        assert!(
            !wp.contains("coptyright"),
            "must not emit misspelled coptyright"
        );
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
        assert!(
            wp.contains("{repeat}"),
            "Worship Pro export must contain {{repeat}}"
        );

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

    #[test]
    fn worship_pro_export_preserves_multilingual_metadata() {
        let input = r#"{title: "Title DE"}
{title2: "Title EN"}
{key: C}
{language: de}
{language2: en}
{artist: "Artist DE"}
{artist2: "Artist EN"}
{section: Verse}
[C]Line
"#;
        let song = load_string(input).expect("parse multilingual song");

        use crate::outputs::FormatChordPro;
        let wp = (&song).format_chord_pro(
            None,
            Some(&ChordRepresentation::Default),
            None,
            true, // worship_pro
        );

        // Primary metadata must be present.
        assert!(
            wp.contains("{title: Title DE}"),
            "Worship Pro export must contain primary title"
        );
        assert!(
            wp.contains("{language: de}"),
            "Worship Pro export must contain primary language"
        );
        assert!(
            wp.contains("{artist: Artist DE}"),
            "Worship Pro export must contain primary artist"
        );

        // Secondary metadata must be preserved with numbered directives.
        assert!(
            wp.contains("{title2: Title EN}"),
            "Worship Pro export must contain secondary title"
        );
        assert!(
            wp.contains("{language2: en}"),
            "Worship Pro export must contain secondary language"
        );
        assert!(
            wp.contains("{artist2: Artist EN}"),
            "Worship Pro export must contain secondary artist"
        );
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
        assert_eq!(song.title(), "Test");
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

    /// Letter chords with `{key: C}` must render as scale degrees relative to C (not as if key were A).
    /// See https://github.com/xilefmusics/chordlib/issues/49
    #[test]
    fn nashville_letter_chords_key_c_issue_49() {
        let input = r#"{title: Repro}
{key: C}
{section: Verse}
[G][C]
"#;
        let song = load_string(input).expect("parse");
        let key = song.key.as_ref().unwrap();
        let rep = ChordRepresentation::Nashville;
        let parts: Vec<_> = song.sections[0].lines[0]
            .parts
            .iter()
            .filter_map(|p| p.chord.as_ref())
            .collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].format(key, &rep), "5");
        assert_eq!(parts[1].format(key, &rep), "1");
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

    #[test]
    fn key_directive_strips_typographic_space_before_sharp() {
        let input = "{title: T}\n{key: C\u{205F}#}\n{section: Verse}\n[C]\n";
        let song = load_string(input).expect("parse");
        assert_eq!(song.key.as_ref().unwrap().pitch_class(), 4);
    }

    #[test]
    fn worship_pro_duration_roundtrip() {
        let input = r#"{title: Durations}
{key: C}
{section: Verse}
[C:4][Am:1,5][G:2][F:1]
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
        assert!(out.contains("[C:4]"));
        assert!(out.contains("[Am:1.5]"));
        let again = load_string(&out).expect("round-trip parse");
        let again_parts: Vec<_> = again.sections[0].lines[0]
            .parts
            .iter()
            .filter_map(|p| p.chord.as_ref())
            .collect();
        assert_eq!(again_parts[0].get_duration(), Some(4000));
        assert_eq!(again_parts[1].get_duration(), Some(1500));
    }

    #[test]
    fn language_directive_supports_multiple_directives() {
        let input = r#"{title: Test}
{key: C}
{language: en}
{language2: de}
{language3: fr}
{section: Verse}
[C]Line
"#;
        let song = load_string(input).expect("parse");
        assert_eq!(song.language(), "en");
        let langs = song.language_list().expect("language list");
        assert_eq!(langs, vec!["en", "de", "fr"]);
    }

    #[test]
    fn artist_directive_supports_multiple_directives() {
        let input = r#"{title: Test}
{key: C}
{artist: First Artist}
{artist2: Second Artist}
{artist3: Third Artist}
{section: Verse}
[C]Line
"#;
        let song = load_string(input).expect("parse");
        assert_eq!(song.artist(), "First Artist");
        assert_eq!(
            song.artists,
            vec![
                "First Artist".to_string(),
                "Second Artist".to_string(),
                "Third Artist".to_string()
            ]
        );
        let artist_list = song.artist_list().expect("artist list");
        assert_eq!(
            artist_list,
            vec!["First Artist", "Second Artist", "Third Artist"]
        );
    }

    #[test]
    fn title_directive_supports_quoted_and_multiple() {
        let input_single = r#"{title: "My Song Title"}
{key: C}
{section: Verse}
[C]Line
"#;
        let song_single = load_string(input_single).expect("parse single");
        assert_eq!(song_single.title(), "My Song Title");
        assert_eq!(song_single.titles, vec!["My Song Title".to_string()]);

        let input_multi = r#"{title: Main}
{title2: "Secondary Title"}
{key: C}
{section: Verse}
[C]Line
"#;
        let song_multi = load_string(input_multi).expect("parse multi");
        assert_eq!(song_multi.title(), "Main");
        assert_eq!(
            song_multi.titles,
            vec!["Main".to_string(), "Secondary Title".to_string()]
        );
        // Language-aware helper should pick the second title for language index 1
        // and fall back to the primary for out-of-range indices.
        assert_eq!(song_multi.title_for_language(Some(0)), "Main");
        assert_eq!(song_multi.title_for_language(Some(1)), "Secondary Title");
        assert_eq!(song_multi.title_for_language(Some(2)), "Main");
    }

    #[test]
    fn worship_pro_ampersand_first_line_rejected() {
        let input = r#"{title: Test}
{key: C}
{section: Verse}
&Free translation as first line
"#;
        let r = load_string(input);
        assert!(r.is_err());
    }

    #[test]
    fn worship_pro_ampersand_free_translation_without_chords() {
        let input = r#"{title: Test}
{key: C}
{section: Verse}
[C]Hallo
&Hello
"#;
        let song = load_string(input).expect("parse");
        let line = &song.sections[0].lines[0];
        assert!(
            line.parts.len() >= 2,
            "expected base parts plus free-translation part"
        );
        let base_part = &line.parts[0];
        assert_eq!(base_part.languages.first().unwrap(), "Hallo");

        let free_part = &line.parts[line.parts.len() - 1];
        assert!(free_part.chord.is_none());
        assert_eq!(free_part.languages.get(1).unwrap(), "Hello");
    }

    #[test]
    fn worship_pro_ampersand_with_matching_chords_merges_languages() {
        let input = r#"{title: Test}
{key: C}
{section: Verse}
[C]Hallo [G]Welt
&[C]Hello [G]World
"#;
        let song = load_string(input).expect("parse");
        let line = &song.sections[0].lines[0];
        assert_eq!(line.parts.len(), 2);
        assert_eq!(line.parts[0].languages.first().unwrap(), "Hallo ");
        assert_eq!(line.parts[0].languages.get(1).unwrap(), "Hello ");
        assert_eq!(line.parts[1].languages.first().unwrap(), "Welt");
        assert_eq!(line.parts[1].languages.get(1).unwrap(), "World");
    }

    #[test]
    fn worship_pro_ampersand_with_mismatching_chords_is_error() {
        let input = r#"{title: Test}
{key: C}
{section: Verse}
[C]Hallo [G]Welt
&[C]Hello [Am]World
"#;
        let r = load_string(input);
        assert!(r.is_err());
    }

    #[test]
    fn worship_pro_ampersand_stacks_multiple_languages() {
        let input = r#"{title: Test}
{key: C}
{language: de en fr}
{section: Verse}
[C]Hallo
&[C]Hello
&[C]Bonjour
"#;
        let song = load_string(input).expect("parse");
        let line = &song.sections[0].lines[0];
        assert_eq!(line.parts[0].languages.first().unwrap(), "Hallo");
        assert_eq!(line.parts[0].languages.get(1).unwrap(), "Hello");
        assert_eq!(line.parts[0].languages.get(2).unwrap(), "Bonjour");
    }

    /// Custom tags via ChordPro {meta: name value}: parsed into song.tags and round-trip.
    /// See https://github.com/xilefmusics/chordlib/issues/8
    #[test]
    fn custom_meta_tags_roundtrip() {
        let input = r#"{title: Tagged Song}
{key: C}
{meta: scripture John 3:16}
{meta: hymn_type praise}
{section: Verse}
[C]Line
"#;
        let song = load_string(input).expect("parse");
        assert_eq!(
            song.tags.get("scripture").map(String::as_str),
            Some("John 3:16")
        );
        assert_eq!(
            song.tags.get("hymn_type").map(String::as_str),
            Some("praise")
        );
        assert_eq!(song.sections.len(), 1);
        assert_eq!(
            song.sections[0].lines.len(),
            1,
            "meta lines must not become content"
        );

        use crate::outputs::FormatChordPro;
        let wp = (&song).format_chord_pro(None, None, None, true);
        assert!(wp.contains("{meta: scripture John 3:16}"));
        assert!(wp.contains("{meta: hymn_type praise}"));

        let cp = (&song).format_chord_pro(None, None, None, false);
        assert!(
            cp.contains("{meta: scripture John 3:16}"),
            "Chord Pro export must contain custom meta tags"
        );
        assert!(cp.contains("{meta: hymn_type praise}"));

        for output in [&wp, &cp] {
            let again = load_string(output).expect("round-trip");
            assert_eq!(
                again.tags.get("scripture").map(String::as_str),
                Some("John 3:16")
            );
            assert_eq!(
                again.tags.get("hymn_type").map(String::as_str),
                Some("praise")
            );
        }
    }

    #[test]
    fn worship_pro_export_preserves_multilingual_lyrics() {
        let input = r#"{title: Test}
{key: C}
{language: de}
{language2: en}
{section: Verse}
[C]Hallo
&[C]Hello
"#;
        let song = load_string(input).expect("parse multilingual lyrics");

        use crate::outputs::FormatChordPro;
        let wp = (&song).format_chord_pro(
            None,
            Some(&ChordRepresentation::Default),
            None,
            true, // worship_pro
        );

        assert!(
            wp.contains("[C]Hallo"),
            "Worship Pro export must contain base language lyrics"
        );
        assert!(
            wp.contains("&[C]Hello"),
            "Worship Pro export must contain second language lyrics as &-line"
        );
    }
}
