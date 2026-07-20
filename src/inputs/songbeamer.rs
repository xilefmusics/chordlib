//! SongBeamer `.sng` input support.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use base64::Engine as _;
use encoding_rs::WINDOWS_1252;

use crate::Error;
use crate::types::{Chord, Line, Part, Section, SimpleChord, Song, SongFlowItem};

#[derive(Clone, Copy)]
enum TextEncoding {
    Utf8,
    Utf16Le,
    Utf16Be,
    Windows1252,
}

#[derive(Debug)]
struct ChordPosition {
    column: f64,
    line: i32,
    chord: String,
}

#[derive(Debug)]
struct PhysicalLine {
    number: i32,
    text: String,
}

#[derive(Debug, Default)]
struct PhysicalSlide {
    lines: Vec<PhysicalLine>,
}

/// Load a SongBeamer file from disk.
pub fn load(path: impl AsRef<Path>) -> Result<Song, Error> {
    load_bytes(&std::fs::read(path)?)
}

/// Load a SongBeamer file from bytes, applying SongBeamer's BOM rules.
pub fn load_bytes(input: &[u8]) -> Result<Song, Error> {
    let (text, encoding) = decode_document(input)?;
    parse_document(&text, encoding)
}

/// Load an already decoded UTF-8 SongBeamer document.
pub fn load_string(input: &str) -> Result<Song, Error> {
    parse_document(
        input.strip_prefix('\u{feff}').unwrap_or(input),
        TextEncoding::Utf8,
    )
}

fn decode_document(input: &[u8]) -> Result<(String, TextEncoding), Error> {
    if let Some(bytes) = input.strip_prefix(&[0xef, 0xbb, 0xbf]) {
        let text = std::str::from_utf8(bytes)
            .map_err(|error| Error::Parse(format!("invalid UTF-8 SongBeamer file: {error}")))?;
        return Ok((text.to_string(), TextEncoding::Utf8));
    }
    if let Some(bytes) = input.strip_prefix(&[0xff, 0xfe]) {
        return Ok((decode_utf16(bytes, true)?, TextEncoding::Utf16Le));
    }
    if let Some(bytes) = input.strip_prefix(&[0xfe, 0xff]) {
        return Ok((decode_utf16(bytes, false)?, TextEncoding::Utf16Be));
    }

    let (text, _, _) = WINDOWS_1252.decode(input);
    Ok((text.into_owned(), TextEncoding::Windows1252))
}

fn decode_utf16(input: &[u8], little_endian: bool) -> Result<String, Error> {
    if !input.len().is_multiple_of(2) {
        return Err(Error::Parse(
            "UTF-16 SongBeamer file has an odd byte length".into(),
        ));
    }
    let units = input.chunks_exact(2).map(|bytes| {
        if little_endian {
            u16::from_le_bytes([bytes[0], bytes[1]])
        } else {
            u16::from_be_bytes([bytes[0], bytes[1]])
        }
    });
    String::from_utf16(&units.collect::<Vec<_>>())
        .map_err(|error| Error::Parse(format!("invalid UTF-16 SongBeamer file: {error}")))
}

fn decode_chord_payload(input: &[u8], encoding: TextEncoding) -> Result<String, Error> {
    match encoding {
        TextEncoding::Utf8 => std::str::from_utf8(input)
            .map(str::to_owned)
            .map_err(|error| Error::Parse(format!("invalid UTF-8 in #Chords: {error}"))),
        TextEncoding::Utf16Le => decode_utf16(input, true),
        TextEncoding::Utf16Be => decode_utf16(input, false),
        TextEncoding::Windows1252 => Ok(WINDOWS_1252.decode(input).0.into_owned()),
    }
}

fn parse_document(input: &str, encoding: TextEncoding) -> Result<Song, Error> {
    let lines = input
        .split_terminator('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line))
        .collect::<Vec<_>>();
    let body_start = lines
        .iter()
        .position(|line| line.trim_start().starts_with("---"))
        .ok_or_else(|| Error::Parse("SongBeamer file has no initial --- separator".into()))?;

    let mut headers = BTreeMap::new();
    for (index, line) in lines[..body_start].iter().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let Some(header) = line.strip_prefix('#') else {
            return Err(Error::Parse(format!(
                "invalid SongBeamer header on line {}",
                index + 1
            )));
        };
        let Some((name, value)) = header.split_once('=') else {
            return Err(Error::Parse(format!(
                "SongBeamer header #{header} has no value"
            )));
        };
        headers.insert(name.to_string(), value.to_string());
    }

    let title = headers
        .get("Title")
        .filter(|title| !title.trim().is_empty())
        .ok_or_else(|| Error::Parse("SongBeamer file has no title".into()))?
        .trim()
        .to_string();
    let lang_count = parse_number::<usize>(&headers, "LangCount")?.unwrap_or(1);
    if lang_count == 0 {
        return Err(Error::Parse("#LangCount must be at least 1".into()));
    }

    let key = headers
        .get("Key")
        .filter(|value| !value.trim().is_empty())
        .map(|value| parse_songbeamer_key(value))
        .transpose()?;
    let chord_positions = headers
        .get("Chords")
        .map(|value| parse_chords(value, encoding))
        .transpose()?
        .unwrap_or_default();

    let physical_slides = parse_slides(&lines[body_start..]);
    let primary_lines = primary_line_numbers(&physical_slides, lang_count)?;
    for position in &chord_positions {
        if position.line != -1 && !primary_lines.contains(&position.line) {
            return Err(Error::Parse(format!(
                "#Chords record targets marker, separator, translation, or missing physical line {}",
                position.line
            )));
        }
    }
    let mut sections = physical_slides
        .into_iter()
        .enumerate()
        .map(|(index, slide)| {
            map_slide(slide, index + 1, lang_count, &chord_positions, key.as_ref())
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let leading_chords = chord_positions
        .iter()
        .filter(|position| position.line == -1)
        .collect::<Vec<_>>();
    if !leading_chords.is_empty() {
        let empty_languages = vec![""; lang_count];
        let line = build_line(&empty_languages, leading_chords, lang_count, key.as_ref())?;
        let first = sections
            .first_mut()
            .ok_or_else(|| Error::Parse("#Chords uses line -1 but the song has no slide".into()))?;
        first.lines.insert(0, line);
    }

    let mut titles = vec![title];
    let mut localized_titles = headers
        .iter()
        .filter_map(|(name, value)| {
            name.strip_prefix("TitleLang")
                .and_then(|index| index.parse::<usize>().ok())
                .map(|index| (index, value.clone()))
        })
        .collect::<Vec<_>>();
    localized_titles.sort_by_key(|(index, _)| *index);
    for (index, value) in localized_titles {
        if index < 2 {
            continue;
        }
        titles.resize(index, String::new());
        titles[index - 1] = value;
    }

    let tempo = parse_number::<u32>(&headers, "Tempo")?.or(parse_number::<u32>(&headers, "Speed")?);
    let time = headers
        .get("Time")
        .map(|value| parse_time(value))
        .transpose()?;
    let mut tags = BTreeMap::new();
    for (name, value) in &headers {
        if !is_mapped_header(name) && !value.contains(['\r', '\n']) {
            tags.insert(format!("songbeamer.{name}"), value.clone());
        }
    }

    let mut song = Song {
        titles,
        subtitle: headers.get("OTitle").cloned(),
        copyright: headers.get("(c)").cloned(),
        key,
        artists: headers
            .get("Author")
            .filter(|value| !value.is_empty())
            .cloned()
            .into_iter()
            .collect(),
        languages: vec![String::new(); lang_count],
        tempo,
        time,
        tags,
        sections,
    };

    if let Some(order) = headers.get("VerseOrder") {
        let flow = parse_verse_order(order, &song.sections)?;
        song.apply_flow(flow)?;
    }
    Ok(song)
}

fn parse_number<T>(headers: &BTreeMap<String, String>, name: &str) -> Result<Option<T>, Error>
where
    T: std::str::FromStr,
{
    headers
        .get(name)
        .filter(|value| !value.trim().is_empty())
        .map(|value| {
            value
                .trim()
                .parse::<T>()
                .map_err(|_| Error::Parse(format!("invalid #{name} value {value:?}")))
        })
        .transpose()
}

fn parse_time(value: &str) -> Result<(u32, u32), Error> {
    let (numerator, denominator) = value
        .trim()
        .split_once('/')
        .ok_or_else(|| Error::Parse(format!("invalid #Time value {value:?}")))?;
    let numerator = numerator
        .trim()
        .parse()
        .map_err(|_| Error::Parse(format!("invalid #Time value {value:?}")))?;
    let denominator = denominator
        .trim()
        .parse()
        .map_err(|_| Error::Parse(format!("invalid #Time value {value:?}")))?;
    if denominator == 0 {
        return Err(Error::Parse("#Time denominator must not be zero".into()));
    }
    Ok((numerator, denominator))
}

fn parse_songbeamer_key(value: &str) -> Result<SimpleChord, Error> {
    let value = normalize_chord_name(value.trim());
    SimpleChord::try_from(value.as_str())
        .map_err(|_| Error::Parse(format!("invalid #Key value {value:?}")))
}

fn normalize_chord_name(value: &str) -> String {
    value.replace('<', "b").replace("B=", "B")
}

fn parse_chords(value: &str, encoding: TextEncoding) -> Result<Vec<ChordPosition>, Error> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(value.trim())
        .map_err(|error| Error::Parse(format!("invalid base64 in #Chords: {error}")))?;
    let decoded = decode_chord_payload(&bytes, encoding)?;
    decoded
        .split('\r')
        .filter(|record| !record.is_empty())
        .enumerate()
        .map(|(index, record)| {
            let mut fields = record.splitn(3, ',');
            let column = fields.next().and_then(|field| field.parse::<f64>().ok());
            let line = fields.next().and_then(|field| field.parse::<i32>().ok());
            let chord = fields.next();
            let (Some(column), Some(line), Some(chord)) = (column, line, chord) else {
                return Err(Error::Parse(format!(
                    "invalid #Chords record {}: {record:?}",
                    index + 1
                )));
            };
            if !column.is_finite() {
                return Err(Error::Parse(format!(
                    "non-finite column in #Chords record {}",
                    index + 1
                )));
            }
            Ok(ChordPosition {
                column,
                line,
                chord: normalize_chord_name(chord),
            })
        })
        .collect()
}

fn parse_slides(lines: &[&str]) -> Vec<PhysicalSlide> {
    let mut slides = Vec::new();
    let mut current = None::<PhysicalSlide>;
    let mut line_number = -1;

    for line in lines {
        let trimmed = line.trim_start();
        if trimmed.starts_with("---") || trimmed.starts_with("--") {
            if let Some(slide) = current.take()
                && !slide.lines.is_empty()
            {
                slides.push(slide);
            }
            current = Some(PhysicalSlide::default());
            line_number += 1;
        } else if let Some(slide) = current.as_mut() {
            slide.lines.push(PhysicalLine {
                number: line_number,
                text: (*line).to_string(),
            });
            line_number += 1;
        }
    }
    if let Some(slide) = current
        && !slide.lines.is_empty()
    {
        slides.push(slide);
    }
    slides
}

fn primary_line_numbers(
    slides: &[PhysicalSlide],
    lang_count: usize,
) -> Result<BTreeSet<i32>, Error> {
    let mut result = BTreeSet::new();
    for (index, slide) in slides.iter().enumerate() {
        let marker_count = usize::from(
            slide
                .lines
                .first()
                .is_some_and(|line| verse_marker(&line.text).is_some()),
        );
        let lyrics = &slide.lines[marker_count..];
        if !lyrics.len().is_multiple_of(lang_count) {
            return Err(Error::Parse(format!(
                "slide {} has {} lyric lines, not a multiple of #LangCount={lang_count}",
                index + 1,
                lyrics.len()
            )));
        }
        result.extend(lyrics.chunks(lang_count).map(|group| group[0].number));
    }
    Ok(result)
}

fn map_slide(
    mut slide: PhysicalSlide,
    slide_number: usize,
    lang_count: usize,
    chords: &[ChordPosition],
    key: Option<&SimpleChord>,
) -> Result<Section, Error> {
    let marker = slide
        .lines
        .first()
        .and_then(|line| verse_marker(&line.text));
    if marker.is_some() {
        slide.lines.remove(0);
    }
    let title = match marker.as_deref().and_then(parse_generated_variant_marker) {
        Some((title, _)) => Some(title.to_string()),
        None => marker,
    };
    if !slide.lines.len().is_multiple_of(lang_count) {
        return Err(Error::Parse(format!(
            "slide {slide_number} has {} lyric lines, not a multiple of #LangCount={lang_count}",
            slide.lines.len()
        )));
    }

    let mut lines = Vec::new();
    for group in slide.lines.chunks(lang_count) {
        let primary = &group[0];
        for translated in &group[1..] {
            if chords.iter().any(|chord| chord.line == translated.number) {
                return Err(Error::Parse(format!(
                    "chords on translated physical line {} are not representable",
                    translated.number
                )));
            }
        }
        let translations = group
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>();
        let line_chords = chords
            .iter()
            .filter(|chord| chord.line == primary.number)
            .collect::<Vec<_>>();
        lines.push(build_line(&translations, line_chords, lang_count, key)?);
    }

    Ok(Section::new(title.unwrap_or_default(), lines))
}

fn build_line(
    translations: &[&str],
    mut chords: Vec<&ChordPosition>,
    lang_count: usize,
    key: Option<&SimpleChord>,
) -> Result<Line, Error> {
    chords.sort_by(|left, right| left.column.total_cmp(&right.column));
    let primary = translations.first().copied().unwrap_or_default();
    let char_count = primary.chars().count();
    let mut parts = Vec::<Part>::new();
    let mut cursor = 0usize;

    for position in chords {
        let column = position.column.ceil().max(0.0) as usize;
        if column > char_count {
            return Err(Error::Parse(format!(
                "chord {:?} column {} exceeds lyric length {char_count} on physical line {}",
                position.chord, position.column, position.line
            )));
        }
        if column > cursor {
            append_primary_text(&mut parts, char_slice(primary, cursor, column));
        }
        let chord = Chord::from_str_with_key(&position.chord, key).map_err(|error| {
            Error::Parse(format!(
                "invalid chord {:?} on physical line {}: {error}",
                position.chord, position.line
            ))
        })?;
        parts.push(Part {
            chord: Some(chord),
            languages: vec![String::new(); lang_count],
            comment: false,
        });
        cursor = column;
    }
    if cursor < char_count || parts.is_empty() {
        append_primary_text(&mut parts, char_slice(primary, cursor, char_count));
    }
    for part in &mut parts {
        part.languages.resize(lang_count, String::new());
    }
    for (language, text) in translations.iter().enumerate().skip(1) {
        parts[0].languages[language] = (*text).to_string();
    }
    Ok(Line::new(parts))
}

fn append_primary_text(parts: &mut Vec<Part>, text: &str) {
    if let Some(part) = parts.last_mut() {
        part.languages[0].push_str(text);
    } else {
        parts.push(Part {
            chord: None,
            languages: vec![text.to_string()],
            comment: false,
        });
    }
}

fn char_slice(input: &str, start: usize, end: usize) -> &str {
    let start = input
        .char_indices()
        .nth(start)
        .map_or(input.len(), |(index, _)| index);
    let end = input
        .char_indices()
        .nth(end)
        .map_or(input.len(), |(index, _)| index);
    &input[start..end]
}

fn verse_marker(line: &str) -> Option<String> {
    let line = line.trim();
    if line
        .get(..4)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("$$m="))
    {
        return Some(line[4..].trim().to_string());
    }
    let words = line.split_whitespace().collect::<Vec<_>>();
    if !(1..=2).contains(&words.len()) || !is_marker_name(words[0]) {
        return None;
    }
    if words.len() == 2
        && !words[1]
            .chars()
            .all(|character| character.is_alphanumeric())
    {
        return None;
    }
    Some(line.to_string())
}

fn is_marker_name(value: &str) -> bool {
    const MARKERS: &[&str] = &[
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
    MARKERS
        .iter()
        .any(|marker| value.eq_ignore_ascii_case(marker))
}

fn parse_verse_order(order: &str, sections: &[Section]) -> Result<Vec<SongFlowItem>, Error> {
    order
        .split(',')
        .map(str::trim)
        .filter(|marker| !marker.is_empty())
        .map(|marker| {
            let (title, requested_occurrence) = parse_generated_variant_marker(marker)
                .map_or((marker, None), |(title, occurrence)| {
                    (title, Some(occurrence))
                });
            let matches = sections
                .iter()
                .enumerate()
                .filter(|(_, section)| section.title.eq_ignore_ascii_case(title))
                .collect::<Vec<_>>();
            if matches.is_empty() {
                return Err(Error::Parse(format!(
                    "#VerseOrder references unknown marker {marker:?}"
                )));
            }
            let mut distinct_bodies = Vec::new();
            for (_, section) in &matches {
                if !distinct_bodies.contains(&&section.lines) {
                    distinct_bodies.push(&section.lines);
                }
            }
            let occurrence_index = if let Some(occurrence) = requested_occurrence {
                if occurrence >= distinct_bodies.len() {
                    return Err(Error::Parse(format!(
                        "#VerseOrder references missing generated variant {marker:?}"
                    )));
                }
                occurrence as u32
            } else if distinct_bodies.len() > 1 {
                return Err(Error::Parse(format!(
                    "#VerseOrder marker {marker:?} is ambiguous"
                )));
            } else {
                0
            };
            Ok(SongFlowItem {
                title: matches[0].1.title.clone(),
                occurrence_index,
                repeats: 1,
            })
        })
        .collect()
}

fn parse_generated_variant_marker(marker: &str) -> Option<(&str, usize)> {
    let (title, suffix) = marker.rsplit_once(" [chordlib:")?;
    let occurrence = suffix.strip_suffix(']')?.parse::<usize>().ok()?;
    occurrence.checked_sub(1).map(|index| (title, index))
}

fn is_mapped_header(name: &str) -> bool {
    matches!(
        name,
        "Title"
            | "Author"
            | "(c)"
            | "Key"
            | "Tempo"
            | "Speed"
            | "Time"
            | "LangCount"
            | "Chords"
            | "VerseOrder"
            | "Version"
            | "Editor"
            | "OTitle"
    ) || name.starts_with("TitleLang")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_minimal_song_with_chords_and_verse_order() {
        let chord_data = base64::engine::general_purpose::STANDARD.encode("0,1,C\r5,1,G\r");
        let input = format!(
            "#LangCount=1\r\n#VerseOrder=Verse 1,Chorus,Verse 1\r\n#Title=Example\r\n#Key=C\r\n#Chords={chord_data}\r\n#Version=3\r\n---\r\nVerse 1\r\nHello world\r\n---\r\nChorus\r\nSing\r\n"
        );
        let song = load_string(&input).expect("parse SongBeamer");
        assert_eq!(song.title(), "Example");
        assert_eq!(song.sections.len(), 3);
        assert_eq!(song.sections[0].title, "Verse 1");
        assert_eq!(song.sections[0].lines[0].parts.len(), 2);
        assert!(song.sections[2].lines.is_empty());
    }

    #[test]
    fn loads_bomless_windows_1252() {
        let bytes =
            b"#LangCount=1\r\n#Title=Gr\xfc\xdfe\r\n#Version=3\r\n---\r\nVers 1\r\nSch\xf6n\r\n";
        let song = load_bytes(bytes).expect("parse CP1252");
        assert_eq!(song.title(), "Grüße");
        assert_eq!(song.sections[0].lines[0].parts[0].languages[0], "Schön");
    }

    #[test]
    fn rejects_chord_beyond_line() {
        let chord_data = base64::engine::general_purpose::STANDARD.encode("99,1,C\r");
        let input = format!(
            "#LangCount=1\n#Title=Example\n#Key=C\n#Chords={chord_data}\n---\nVerse 1\nShort"
        );
        let error = load_string(&input).expect_err("invalid column");
        assert!(error.to_string().contains("exceeds lyric length"));
    }

    #[test]
    fn loads_utf16le_and_multilingual_lyrics() {
        let input = "#LangCount=2\r\n#Title=Grüße\r\n#Version=3\r\n---\r\nVerse 1\r\nHello\r\nHallo\r\nWorld\r\nWelt\r\n";
        let mut bytes = vec![0xff, 0xfe];
        bytes.extend(input.encode_utf16().flat_map(u16::to_le_bytes));
        let song = load_bytes(&bytes).expect("parse UTF-16LE");
        assert_eq!(song.title(), "Grüße");
        assert_eq!(song.sections[0].lines.len(), 2);
        assert_eq!(
            song.sections[0].lines[0].parts[0].languages,
            ["Hello", "Hallo"]
        );
        assert_eq!(
            song.sections[0].lines[1].parts[0].languages,
            ["World", "Welt"]
        );
    }

    #[test]
    fn imports_songbeamer_chord_only_line_minus_one() {
        let chord_data = base64::engine::general_purpose::STANDARD.encode("-1,-1,C\r0,-1,G\r");
        let input = format!(
            "#LangCount=1\n#Title=Intro\n#Key=C\n#Chords={chord_data}\n---\nVerse 1\nLyrics"
        );
        let song = load_string(&input).expect("parse chord-only line");
        let leading = &song.sections[0].lines[0];
        assert_eq!(leading.parts.len(), 2);
        assert!(leading.parts.iter().all(|part| part.chord.is_some()));
    }

    #[test]
    fn rejects_chords_attached_to_a_marker_line() {
        let chord_data = base64::engine::general_purpose::STANDARD.encode("0,0,C\r");
        let input =
            format!("#LangCount=1\n#Title=Bad\n#Key=C\n#Chords={chord_data}\n---\nVerse 1\nLyrics");
        let error = load_string(&input).expect_err("marker chord is unsupported");
        assert!(error.to_string().contains("targets marker"));
    }
}
