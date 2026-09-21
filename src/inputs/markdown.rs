use std::path::Path;

use serde_yaml::from_str;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::Error;
use crate::markdown::{FrontMatter, is_plausible_chord_token};
use crate::types::{Chord, Line, Part, Section, SimpleChord, Song};

#[derive(Debug)]
struct ParsedChordRow {
    chords: Vec<(usize, Chord)>,
}

struct SectionSource {
    title: String,
    repeat_count: u32,
    lines: Vec<String>,
}

pub fn load(path: impl AsRef<Path>) -> Result<Song, Error> {
    load_string(&std::fs::read_to_string(path)?)
}

pub fn load_string(input: &str) -> Result<Song, Error> {
    let (front_matter, body) = split_front_matter(input)?;
    let (key, mut song) = song_from_front_matter(front_matter)?;

    let mut sections = Vec::new();
    let mut current: Option<SectionSource> = None;

    for (index, line) in body.iter().enumerate() {
        if line.starts_with('#') {
            let (title, repeat_count) = parse_section_header(line)
                .map_err(|message| parse_line_error(index + 1, message))?;
            if let Some(source) = current.take() {
                sections.push(parse_section(source, key.as_ref())?);
            }
            current = Some(SectionSource {
                title,
                repeat_count,
                lines: Vec::new(),
            });
        } else if let Some(section) = current.as_mut() {
            if line.contains('\t') {
                return Err(parse_line_error(
                    index + 1,
                    "tabs are not allowed in aligned song text",
                ));
            }
            section.lines.push(line.clone());
        } else if !line.trim().is_empty() {
            return Err(parse_line_error(
                index + 1,
                "song text must follow a `# Section` header",
            ));
        }
    }

    if let Some(source) = current {
        sections.push(parse_section(source, key.as_ref())?);
    }

    song.sections = sections;
    Ok(song)
}

fn split_front_matter(input: &str) -> Result<(FrontMatter, Vec<String>), Error> {
    let input = input.strip_prefix('\u{feff}').unwrap_or(input);
    let mut lines = input.lines();
    if lines.next() != Some("---") {
        return Err(Error::Parse(
            "markdown song must start with YAML front matter (`---`)".into(),
        ));
    }

    let mut yaml_lines = Vec::new();
    let mut body = Vec::new();
    let mut closed = false;

    for line in lines.by_ref() {
        if line.trim() == "---" {
            closed = true;
            break;
        }
        yaml_lines.push(line);
    }

    if !closed {
        return Err(Error::Parse(
            "markdown song front matter has no closing `---`".into(),
        ));
    }

    for line in lines {
        body.push(line.to_string());
    }

    let front_matter: FrontMatter = from_str(&yaml_lines.join("\n"))
        .map_err(|error| Error::Parse(format!("invalid markdown front matter: {error}")))?;

    Ok((front_matter, body))
}

fn song_from_front_matter(front_matter: FrontMatter) -> Result<(Option<SimpleChord>, Song), Error> {
    if !front_matter
        .titles
        .iter()
        .any(|title| !title.trim().is_empty())
    {
        return Err(Error::Parse(
            "markdown song needs at least one non-empty title".into(),
        ));
    }

    let key = match front_matter.key.as_deref().map(str::trim) {
        Some("") => None,
        Some(value) if value.chars().next().is_some_and(|c| c.is_ascii_digit()) => {
            return Err(Error::Parse(
                "markdown key must be a letter name, not a Nashville number".into(),
            ));
        }
        Some(value) => Some(
            value
                .try_into()
                .map_err(|error: Error| Error::Parse(format!("invalid markdown key: {error}")))?,
        ),
        None => None,
    };

    let time = front_matter
        .time
        .as_deref()
        .map(parse_time_signature)
        .transpose()?;

    let song = Song {
        titles: front_matter.titles,
        subtitle: front_matter.subtitle,
        copyright: front_matter.copyright,
        key: key.clone(),
        artists: front_matter.artists,
        languages: front_matter.languages,
        tempo: front_matter.tempo,
        time,
        tags: front_matter.tags,
        sections: Vec::new(),
    };

    Ok((key, song))
}

fn parse_time_signature(value: &str) -> Result<(u32, u32), Error> {
    let (numerator, denominator) = value
        .trim()
        .split_once('/')
        .ok_or_else(|| Error::Parse("time must use the form numerator/denominator".into()))?;
    let numerator = numerator
        .trim()
        .parse::<u32>()
        .map_err(|_| Error::Parse("time numerator must be a positive integer".into()))?;
    let denominator = denominator
        .trim()
        .parse::<u32>()
        .map_err(|_| Error::Parse("time denominator must be a positive integer".into()))?;
    if numerator == 0 || denominator == 0 {
        return Err(Error::Parse(
            "time numerator and denominator must be positive".into(),
        ));
    }
    Ok((numerator, denominator))
}

fn parse_section_header(line: &str) -> Result<(String, u32), String> {
    let Some(rest) = line.strip_prefix("# ") else {
        return Err("only single-level `# Section` headers are supported".into());
    };

    let mut title = rest.trim().to_string();
    while title.ends_with('#') {
        let before = &title[..title.len() - 1];
        if !before.chars().last().is_some_and(char::is_whitespace) {
            break;
        }
        title = before.trim_end().to_string();
    }

    let (title, repeat_count) = parse_repeat_suffix(&title)?;
    if title.is_empty() {
        return Err("section title must not be empty".into());
    }
    Ok((title, repeat_count))
}

fn parse_repeat_suffix(title: &str) -> Result<(String, u32), String> {
    let Some(open) = title.rfind('(') else {
        return Ok((title.trim().to_string(), 1));
    };
    let suffix = &title[open..];
    if !suffix.ends_with(")") {
        return Ok((title.trim().to_string(), 1));
    }
    let inner = &suffix[1..suffix.len() - 1];
    if !inner.ends_with('x') {
        return Ok((title.trim().to_string(), 1));
    }

    let count = inner[..inner.len() - 1]
        .parse::<u32>()
        .map_err(|_| "repeat count must use the form `(2x)`".to_string())?;
    if count == 0 {
        return Err("repeat count must be at least 1".into());
    }
    let title = title[..open].trim_end().to_string();
    Ok((title, count))
}

fn parse_section(source: SectionSource, key: Option<&SimpleChord>) -> Result<Section, Error> {
    let mut lines = Vec::new();
    let mut index = 0;

    while index < source.lines.len() {
        if source.lines[index].trim().is_empty() {
            index += 1;
            continue;
        }
        if source.lines[index].starts_with('&') {
            return Err(Error::Parse(
                "a translation line must follow a base lyric line".into(),
            ));
        }

        let (mut line, next_index) = parse_base_line(&source.lines, index, key)?;
        index = next_index;

        while index < source.lines.len() && source.lines[index].starts_with('&') {
            let (translation, next_index) = parse_translation(&source.lines, index, &line, key)?;
            merge_language(&mut line, translation)?;
            index = next_index;
        }

        lines.push(line);
    }

    Ok(Section::new_with_repeat(
        source.title,
        lines,
        source.repeat_count,
    ))
}

fn parse_base_line(
    lines: &[String],
    index: usize,
    key: Option<&SimpleChord>,
) -> Result<(Line, usize), Error> {
    let source = &lines[index];
    if let Some(chord_row) = parse_chord_row(source, key)? {
        if index + 1 >= lines.len() {
            return Ok((parse_aligned_line(&chord_row, "", key, true)?, index + 1));
        }
        if lines[index + 1].starts_with('&') {
            return Err(Error::Parse(
                "a chord row must be followed by its base lyric row".into(),
            ));
        }
        if lines[index + 1].is_empty() {
            return Ok((parse_aligned_line(&chord_row, "", key, true)?, index + 2));
        }
        return Ok((
            parse_aligned_line(&chord_row, &lines[index + 1], key, false)?,
            index + 2,
        ));
    }

    let source = unescape_literal_prefix(source, key)?;
    Ok((parse_text_line(&source)?, index + 1))
}

fn parse_translation(
    lines: &[String],
    index: usize,
    base: &Line,
    key: Option<&SimpleChord>,
) -> Result<(Line, usize), Error> {
    let translated_row = lines[index]
        .strip_prefix('&')
        .expect("translation prefix checked by caller");

    if base.parts.iter().any(|part| part.chord.is_some()) {
        let Some(chord_row) = parse_chord_row(translated_row, key)? else {
            return Err(Error::Parse(
                "a chorded translation must include an `&` chord row".into(),
            ));
        };
        if index + 1 >= lines.len() || !lines[index + 1].starts_with('&') {
            return Err(Error::Parse(
                "a translated chord row must be followed by an `&` lyric row".into(),
            ));
        }
        let lyric = lines[index + 1]
            .strip_prefix('&')
            .expect("translation prefix checked above");
        let translation = parse_aligned_line(&chord_row, lyric, key, false)?;
        ensure_matching_chords(base, &translation)?;
        Ok((translation, index + 2))
    } else {
        if parse_chord_row(translated_row, key)?.is_some() {
            return Err(Error::Parse(
                "a chordless line cannot have a translated chord row".into(),
            ));
        }
        Ok((
            parse_text_line(&unescape_literal_prefix(translated_row, key)?)?,
            index + 1,
        ))
    }
}

fn parse_chord_row(
    source: &str,
    key: Option<&SimpleChord>,
) -> Result<Option<ParsedChordRow>, Error> {
    if source.trim().is_empty() {
        return Ok(None);
    }

    let mut chords = Vec::new();
    let mut token_start = None;
    let mut token_start_column = 0;
    let mut column = 0;

    for (byte_index, character) in source.char_indices() {
        let width = UnicodeWidthChar::width(character).unwrap_or(0);
        if character.is_whitespace() {
            if let Some(start) = token_start.take() {
                let token = &source[start..byte_index];
                if !is_plausible_chord_token(token) {
                    return Ok(None);
                }
                let Ok(chord) = Chord::from_str_with_key(token, key) else {
                    return Ok(None);
                };
                chords.push((token_start_column, chord));
            }
        } else if token_start.is_none() {
            token_start = Some(byte_index);
            token_start_column = column;
        }
        column += width;
    }

    if let Some(start) = token_start {
        let token = &source[start..];
        if !is_plausible_chord_token(token) {
            return Ok(None);
        }
        let Ok(chord) = Chord::from_str_with_key(token, key) else {
            return Ok(None);
        };
        chords.push((token_start_column, chord));
    }

    if chords.is_empty() {
        Ok(None)
    } else {
        Ok(Some(ParsedChordRow { chords }))
    }
}

fn parse_aligned_line(
    chord_row: &ParsedChordRow,
    lyric: &str,
    _key: Option<&SimpleChord>,
    allow_padding: bool,
) -> Result<Line, Error> {
    let lyric_width = lyric.width();
    let mut parts = Vec::new();
    let mut previous_column = 0;
    let mut previous_byte = 0;

    for (chord_index, (column, chord)) in chord_row.chords.iter().enumerate() {
        if *column < previous_column {
            return Err(Error::Parse("chord columns must be ordered".into()));
        }

        let start_byte = match byte_index_at_column(lyric, *column) {
            Some(byte_index) => byte_index,
            None if allow_padding => lyric.len(),
            None => {
                return Err(Error::Parse(
                    "a chord must align with a valid lyric column".into(),
                ));
            }
        };

        if *column > lyric_width && !allow_padding {
            return Err(Error::Parse(
                "a chord must align with a valid lyric column".into(),
            ));
        }

        if start_byte > previous_byte {
            let segment = &lyric[previous_byte..start_byte];
            parts.extend(parse_marked_text(segment)?);
        } else if *column > previous_column && allow_padding {
            parts.push(text_part(&" ".repeat(column - previous_column)));
        }

        let end_column = chord_row
            .chords
            .get(chord_index + 1)
            .map(|(next_column, _)| *next_column)
            .unwrap_or(lyric_width.max(*column));
        let end_byte = match byte_index_at_column(lyric, end_column) {
            Some(byte_index) => byte_index,
            None if allow_padding => lyric.len(),
            None => lyric.len(),
        };

        let mut chord_parts = if start_byte < end_byte {
            let segment =
                remove_inner_word_padding(&lyric[start_byte..end_byte], &lyric[end_byte..]);
            parse_marked_text(&segment)?
        } else {
            Vec::new()
        };

        if chord_parts.is_empty() {
            chord_parts.push(Part::new_chord(chord.clone()));
        } else {
            chord_parts[0].chord = Some(chord.clone());
        }
        parts.extend(chord_parts);
        previous_column = end_column;
        previous_byte = end_byte;
    }

    if previous_byte < lyric.len() {
        parts.extend(parse_marked_text(&lyric[previous_byte..])?);
    }
    if parts.is_empty() {
        parts.push(Part::new_chord(
            chord_row
                .chords
                .last()
                .map(|(_, chord)| chord.clone())
                .expect("chord row is non-empty"),
        ));
    }
    Ok(Line::new(parts))
}

fn remove_inner_word_padding(segment: &str, following_text: &str) -> String {
    // A dash immediately before a chord boundary marks formatter padding when the
    // following segment continues with non-whitespace text. Discard the dash and
    // all whitespace around it so the original word is restored.
    let following_starts_word = following_text
        .chars()
        .next()
        .is_some_and(|character| !character.is_whitespace());
    if !following_starts_word {
        return segment.to_string();
    }

    let Some(dash_index) = segment.rfind('-') else {
        return segment.to_string();
    };
    let (before_dash, after_dash) = segment.split_at(dash_index);
    let after_dash = &after_dash[1..];
    if !after_dash.chars().all(char::is_whitespace) {
        return segment.to_string();
    }

    let before_dash = before_dash.trim_end_matches(char::is_whitespace);
    if before_dash
        .chars()
        .next_back()
        .is_none_or(char::is_whitespace)
    {
        return segment.to_string();
    }

    before_dash.to_string()
}

fn parse_text_line(text: &str) -> Result<Line, Error> {
    let parts = parse_marked_text(text)?;
    if parts.is_empty() {
        Ok(Line::new(vec![text_part("")]))
    } else {
        Ok(Line::new(parts))
    }
}

fn parse_marked_text(text: &str) -> Result<Vec<Part>, Error> {
    let mut parts = Vec::new();
    let mut segment_start = 0;
    let mut cursor = 0;
    let mut in_comment = false;

    while cursor < text.len() {
        if text[cursor..].starts_with("**") && !is_escaped(text, cursor) {
            if cursor > segment_start {
                parts.push(Part {
                    chord: None,
                    languages: vec![unescape_comment_text(&text[segment_start..cursor])],
                    comment: in_comment,
                });
            }
            in_comment = !in_comment;
            cursor += 2;
            segment_start = cursor;
        } else {
            let character = text[cursor..]
                .chars()
                .next()
                .expect("cursor remains within UTF-8 text");
            cursor += character.len_utf8();
        }
    }

    if in_comment {
        return Err(Error::Parse("unmatched `**` comment delimiter".into()));
    }
    if segment_start < text.len() {
        parts.push(Part {
            chord: None,
            languages: vec![unescape_comment_text(&text[segment_start..])],
            comment: false,
        });
    }
    Ok(parts)
}

fn merge_language(base: &mut Line, translation: Line) -> Result<(), Error> {
    if base.parts.len() != translation.parts.len() {
        return Err(Error::Parse(
            "translation text must use the same comment and chord structure as the base line"
                .into(),
        ));
    }
    let language_index = base
        .parts
        .iter()
        .map(|part| part.languages.len())
        .max()
        .unwrap_or(0);

    for (base_part, translation_part) in base.parts.iter_mut().zip(translation.parts) {
        if base_part.chord != translation_part.chord
            || base_part.comment != translation_part.comment
        {
            return Err(Error::Parse(
                "translation text must use the same comment and chord structure as the base line"
                    .into(),
            ));
        }
        base_part
            .languages
            .resize(language_index + 1, String::new());
        base_part.languages[language_index] = translation_part
            .languages
            .first()
            .cloned()
            .unwrap_or_default();
    }
    Ok(())
}

fn ensure_matching_chords(base: &Line, translation: &Line) -> Result<(), Error> {
    let base_chords: Vec<_> = base
        .parts
        .iter()
        .filter_map(|part| part.chord.as_ref())
        .collect();
    let translation_chords: Vec<_> = translation
        .parts
        .iter()
        .filter_map(|part| part.chord.as_ref())
        .collect();
    if base_chords == translation_chords {
        Ok(())
    } else {
        Err(Error::Parse(
            "translation chord sequence does not match the base line".into(),
        ))
    }
}

fn unescape_literal_prefix(source: &str, key: Option<&SimpleChord>) -> Result<String, Error> {
    if let Some(rest) = source.strip_prefix("\\&") {
        return Ok(format!("&{rest}"));
    }
    if let Some(rest) = source.strip_prefix('\\')
        && parse_chord_row(rest, key)?.is_some()
    {
        return Ok(rest.to_string());
    }
    Ok(source.to_string())
}

fn byte_index_at_column(text: &str, requested_column: usize) -> Option<usize> {
    if requested_column == 0 {
        return Some(0);
    }
    let mut column = 0;
    for (byte_index, character) in text.char_indices() {
        if column == requested_column {
            return Some(byte_index);
        }
        column += UnicodeWidthChar::width(character).unwrap_or(0);
        if column > requested_column {
            return None;
        }
    }
    (column == requested_column).then_some(text.len())
}

fn text_part(text: &str) -> Part {
    Part {
        chord: None,
        languages: vec![text.to_string()],
        comment: false,
    }
}

fn is_escaped(text: &str, byte_index: usize) -> bool {
    let mut backslashes = 0;
    let mut index = byte_index;
    while index > 0 && text.as_bytes()[index - 1] == b'\\' {
        backslashes += 1;
        index -= 1;
    }
    backslashes % 2 == 1
}

fn unescape_comment_text(text: &str) -> String {
    text.replace("\\*\\*", "**")
}

fn parse_line_error(line: usize, message: impl Into<String>) -> Error {
    Error::Parse(format!("markdown line {line}: {}", message.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_front_matter_and_sections() {
        let song = load_string("---\ntitles: [Test]\nkey: C\n---\n# Verse (2x)\n\nHello\n")
            .expect("markdown song");
        assert_eq!(song.title(), "Test");
        assert_eq!(song.sections[0].title, "Verse");
        assert_eq!(song.sections[0].repeat_count, 2);
        assert_eq!(song.sections[0].lines[0].parts[0].languages[0], "Hello");

        let lyric_song = load_string("---\ntitles: [Test]\n---\n# Verse\nGood morning\n")
            .expect("ordinary lyric");
        assert!(
            lyric_song.sections[0].lines[0]
                .parts
                .iter()
                .all(|part| part.chord.is_none())
        );
    }

    #[test]
    fn parses_song_without_sections() {
        let song = load_string("---\ntitles: [Metadata only]\n---\n")
            .expect("metadata-only markdown song");
        assert_eq!(song.titles, vec!["Metadata only"]);
        assert!(song.sections.is_empty());
    }

    #[test]
    fn parses_chord_columns_and_translations() {
        let song = load_string(
            "---\ntitles: [Test]\nlanguages: [en, de]\nkey: C\n---\n# Verse\nC       G\nAmazing grace\n&C            G\n&Erstaunliche Gnade\n",
        )
        .expect("markdown song");
        let line = &song.sections[0].lines[0];
        assert_eq!(
            line.parts[0].chord.as_ref().expect("C").format(
                song.key.as_ref().expect("key"),
                &crate::types::ChordRepresentation::Default
            ),
            "C"
        );
        assert_eq!(line.parts[0].languages, vec!["Amazing ", "Erstaunliche "]);
        assert_eq!(line.parts[1].languages, vec!["grace", "Gnade"]);
    }

    #[test]
    fn parses_inline_comments() {
        let song = load_string("---\ntitles: [Test]\n---\n# Verse\nHello **spoken** world\n")
            .expect("markdown song");
        assert_eq!(song.sections[0].lines[0].parts.len(), 3);
        assert!(!song.sections[0].lines[0].parts[0].comment);
        assert!(song.sections[0].lines[0].parts[1].comment);
        assert_eq!(song.sections[0].lines[0].parts[1].languages[0], "spoken");
    }

    #[test]
    fn rejects_tabs_and_unmatched_comments() {
        let tab =
            load_string("---\ntitles: [Test]\n---\n# Verse\nA\tB\n").expect_err("tab should fail");
        assert!(tab.to_string().contains("tabs"));

        let comment = load_string("---\ntitles: [Test]\n---\n# Verse\n**open\n")
            .expect_err("comment should fail");
        assert!(comment.to_string().contains("unmatched"));
    }

    #[test]
    fn parses_all_metadata_fields_and_unicode() {
        let song = load_string(
            "---\ntitles: [\"Grüße\", \"Greetings\"]\nsubtitle: \"Live 😊\"\ncopyright: \"© 2026\"\nkey: F#\nartists: [\"Mākslinieks\", \"Writer\"]\nlanguages: [de, en]\ntempo: 96\ntime: 6/8\ntags:\n  genre: hymn\n  source: test\n---\n# Verse\nText\n",
        )
        .expect("metadata");
        assert_eq!(song.titles, vec!["Grüße", "Greetings"]);
        assert_eq!(song.subtitle.as_deref(), Some("Live 😊"));
        assert_eq!(song.copyright.as_deref(), Some("© 2026"));
        assert_eq!(song.artists, vec!["Mākslinieks", "Writer"]);
        assert_eq!(song.languages, vec!["de", "en"]);
        assert_eq!(song.tempo, Some(96));
        assert_eq!(song.time, Some((6, 8)));
        assert_eq!(song.tags.get("genre").map(String::as_str), Some("hymn"));
    }

    #[test]
    fn parses_nashville_chords_when_key_is_present() {
        let song = load_string("---\ntitles: [Test]\nkey: C\n---\n# Verse\n1       5\nLine here\n")
            .expect("Nashville chords");
        assert_eq!(song.sections[0].lines[0].parts.len(), 2);
        assert!(song.sections[0].lines[0].parts[0].chord.is_some());
        assert!(song.sections[0].lines[0].parts[1].chord.is_some());
    }

    #[test]
    fn parses_chord_only_and_escaped_chord_like_lines() {
        let song = load_string("---\ntitles: [Test]\n---\n# Intro\nC       G\n\n\\C G\n")
            .expect("chord-only line");
        assert!(
            song.sections[0].lines[0]
                .parts
                .iter()
                .any(|part| part.chord.is_some())
        );
        assert_eq!(song.sections[0].lines[1].parts[0].languages[0], "C G");
    }

    #[test]
    fn rejects_invalid_front_matter_and_headers() {
        let cases = [
            ("---\nartists: [Writer]\n---\n# Verse\nText\n", "titles"),
            (
                "---\ntitles: [Test]\nunknown: value\n---\n# Verse\nText\n",
                "unknown field",
            ),
            ("---\ntitles: [Test]\n---\n## Verse\nText\n", "single-level"),
            ("---\ntitles: [Test]\n---\n# Verse (0x)\nText\n", "repeat"),
        ];
        for (input, expected) in cases {
            let error = load_string(input).expect_err("invalid Markdown");
            assert!(error.to_string().contains(expected), "{error}");
        }
    }

    #[test]
    fn rejects_bad_alignment_and_translation_sequences() {
        let alignment = load_string("---\ntitles: [Test]\n---\n# Verse\nC          G\nHi\n")
            .expect_err("bad alignment");
        assert!(alignment.to_string().contains("valid lyric column"));

        let mismatch = load_string(
            "---\ntitles: [Test]\n---\n# Verse\nC       G\nAmazing grace\n&C       F\n&Amazing phrase\n",
        )
        .expect_err("mismatched translation");
        assert!(mismatch.to_string().contains("does not match"));

        let missing = load_string(
            "---\ntitles: [Test]\n---\n# Verse\nC       G\nAmazing grace\n&Translation\n",
        )
        .expect_err("missing translated chord row");
        assert!(missing.to_string().contains("chorded translation"));
    }

    #[test]
    fn preserves_empty_sections_and_escaped_comment_delimiters() {
        let song = load_string(
            "---\ntitles: [Test]\n---\n# Verse\nText\n# Verse (2x)\n# Outro\n**literal \\*\\* markers**\n",
        )
        .expect("references and comments");
        assert_eq!(song.sections.len(), 3);
        assert!(song.sections[1].lines.is_empty());
        assert_eq!(
            song.sections[2].lines[0].parts[0].languages[0],
            "literal ** markers"
        );
    }
}
