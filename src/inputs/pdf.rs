//! Import searchable SongSelect-style chord sheets from PDF.
//!
//! The importer uses positioned characters and is intentionally limited to
//! single-column chord sheets with a text layer. It does not perform OCR or
//! recognize text drawn as vector outlines.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::Error;
use crate::types::{Chord, Line, Part, Section, SimpleChord, Song};

#[derive(Clone, Debug)]
struct Glyph {
    page: usize,
    ch: char,
    x: f32,
    end_x: f32,
    y: f32,
    font_size: f32,
    bold: bool,
    _italic: bool,
}

#[derive(Clone, Debug)]
struct Row {
    page: usize,
    y: f32,
    glyphs: Vec<Glyph>,
}

#[derive(Clone, Debug)]
enum ContentToken {
    Chord { x: f32, chord: Chord },
    Text { x: f32, text: String, comment: bool },
}

#[derive(Clone, Debug)]
struct ChordRow {
    tokens: Vec<ContentToken>,
    glyphs: Vec<Glyph>,
}

/// Load a PDF chord sheet from disk.
pub fn load(path: impl AsRef<Path>) -> Result<Song, Error> {
    load_bytes(&std::fs::read(path)?)
}

/// Load a searchable PDF chord sheet from bytes.
pub fn load_bytes(input: &[u8]) -> Result<Song, Error> {
    let document = pdf_oxide::PdfDocument::from_bytes(input.to_vec())
        .map_err(|error| Error::Parse(format!("invalid PDF document: {error}")))?;
    let page_count = document
        .page_count()
        .map_err(|error| Error::Parse(format!("could not read PDF page count: {error}")))?;
    if page_count == 0 {
        return Err(Error::Parse("PDF contains no pages".into()));
    }

    let mut glyphs = Vec::new();
    for page in 0..page_count {
        let has_text = document.has_text_layer(page).map_err(|error| {
            Error::Parse(format!(
                "PDF page {}: could not inspect text layer: {error}",
                page + 1
            ))
        })?;
        if !has_text {
            continue;
        }
        let rotation = document.get_page_rotation(page).map_err(|error| {
            Error::Parse(format!(
                "PDF page {}: could not read page rotation: {error}",
                page + 1
            ))
        })?;
        let media = document.get_page_media_box(page).map_err(|error| {
            Error::Parse(format!(
                "PDF page {}: could not read page bounds: {error}",
                page + 1
            ))
        })?;
        let chars = document.extract_chars(page).map_err(|error| {
            Error::Parse(format!(
                "PDF page {}: text extraction failed: {error}",
                page + 1
            ))
        })?;
        for ch in chars {
            let (x, y, end_x) = normalize_glyph_position(
                (ch.bbox.x, ch.bbox.y, ch.bbox.width, ch.bbox.height),
                (ch.origin_x, ch.origin_y),
                rotation,
                media,
            );
            if x.is_finite() && y.is_finite() && end_x.is_finite() {
                glyphs.push(Glyph {
                    page,
                    ch: ch.char,
                    x,
                    end_x: end_x.max(x),
                    y,
                    font_size: ch.font_size.max(1.0),
                    bold: ch.font_weight.is_bold(),
                    _italic: ch.is_italic,
                });
            }
        }
    }

    if glyphs.iter().all(|glyph| glyph.ch.is_whitespace()) {
        return Err(Error::Parse(
            "PDF has no usable text layer; searchable song text is required".into(),
        ));
    }

    let rows = cluster_rows(glyphs);
    let mut song = parse_metadata(&rows)?;
    let (sections, body_line_count) = parse_body(&rows, &song)?;
    if sections.is_empty() || body_line_count == 0 {
        return Err(Error::Parse(
            "PDF contains metadata but no recognizable section or song lines".into(),
        ));
    }
    song.sections = sections;
    Ok(song)
}

fn normalize_glyph_position(
    bbox: (f32, f32, f32, f32),
    origin: (f32, f32),
    rotation: i32,
    media: (f32, f32, f32, f32),
) -> (f32, f32, f32) {
    let (x, y, width, height) = bbox;
    let (origin_x, origin_y) = origin;
    let (x0, y0, x1, y1) = media;
    let point = |px: f32, py: f32| match rotation.rem_euclid(360) {
        90 => (py - y0, x1 - px),
        180 => (x1 - px, y1 - py),
        270 => (y1 - py, px - x0),
        _ => (px - x0, py - y0),
    };
    let (origin_x, baseline_y) = point(origin_x, origin_y);
    let corners = [
        point(x, y),
        point(x + width, y),
        point(x, y + height),
        point(x + width, y + height),
    ];
    let left = corners
        .iter()
        .map(|point| point.0)
        .fold(f32::INFINITY, f32::min);
    let right = corners
        .iter()
        .map(|point| point.0)
        .fold(f32::NEG_INFINITY, f32::max);
    (origin_x.min(left), baseline_y, origin_x.max(right))
}

fn cluster_rows(mut glyphs: Vec<Glyph>) -> Vec<Row> {
    glyphs.sort_by(|a, b| {
        a.page
            .cmp(&b.page)
            .then_with(|| b.y.total_cmp(&a.y))
            .then_with(|| a.x.total_cmp(&b.x))
    });

    let mut rows: Vec<Row> = Vec::new();
    for glyph in glyphs {
        let tolerance = (glyph.font_size * 0.16).clamp(0.65, 1.8);
        if let Some(row) = rows
            .iter_mut()
            .rev()
            .find(|row| row.page == glyph.page && (row.y - glyph.y).abs() <= tolerance)
        {
            let old_len = row.glyphs.len() as f32;
            row.y = (row.y * old_len + glyph.y) / (old_len + 1.0);
            row.glyphs.push(glyph);
        } else {
            rows.push(Row {
                page: glyph.page,
                y: glyph.y,
                glyphs: vec![glyph],
            });
        }
    }
    for row in &mut rows {
        row.glyphs.sort_by(|a, b| a.x.total_cmp(&b.x));
    }
    rows.sort_by(|a, b| a.page.cmp(&b.page).then_with(|| b.y.total_cmp(&a.y)));
    rows
}

fn row_text(row: &Row) -> String {
    row.glyphs.iter().map(|glyph| glyph.ch).collect()
}

fn parse_metadata(rows: &[Row]) -> Result<Song, Error> {
    let first_page: Vec<(usize, &Row)> = rows
        .iter()
        .enumerate()
        .filter(|(_, row)| row.page == 0 && !row_text(row).trim().is_empty())
        .collect();
    if first_page.is_empty() {
        return Err(Error::Parse("PDF page 1 has no searchable text".into()));
    }

    let metadata_index = first_page
        .iter()
        .position(|(_, row)| is_metadata_row(&row_text(row)));
    let title = row_text(first_page[0].1).trim().to_string();
    if title.is_empty() || is_metadata_or_footer(&title) || title.contains(" | ") {
        return Err(Error::Parse("PDF song title is empty".into()));
    }

    let mut song = Song {
        titles: vec![title],
        ..Song::default()
    };
    if first_page.len() > 1 {
        let artist_line = row_text(first_page[1].1).trim().to_string();
        if !artist_line.is_empty()
            && !is_metadata_or_footer(&artist_line)
            && !artist_line
                .chars()
                .filter(|ch| ch.is_alphabetic())
                .all(char::is_uppercase)
        {
            song.artists = artist_line
                .split(" | ")
                .map(str::trim)
                .filter(|artist| !artist.is_empty())
                .map(str::to_string)
                .collect();
        }
    }

    if let Some(metadata_index) = metadata_index {
        let line = row_text(first_page[metadata_index].1);
        let mut seen_fields = BTreeSet::new();
        for segment in line.split(" | ") {
            let Some((label, value)) = parse_metadata_segment(segment)? else {
                continue;
            };
            let field_name = label.trim().to_ascii_lowercase();
            if !seen_fields.insert(field_name.clone()) {
                return Err(Error::Parse(format!(
                    "PDF metadata field {field_name:?} appears more than once"
                )));
            }
            let value = value.trim();
            match field_name.as_str() {
                "key" => {
                    song.key = Some(SimpleChord::try_from(value).map_err(|error| {
                        Error::Parse(format!("PDF metadata key {value:?} is invalid: {error}"))
                    })?);
                }
                "tempo" => {
                    let (tempo_text, beat_unit) = parse_tempo(value)?;
                    song.tempo = Some(parse_positive_u32(tempo_text, "tempo")?);
                    if let Some(beat_unit) = beat_unit {
                        song.tags.insert("pdf.tempo_beat_unit".into(), beat_unit);
                    }
                }
                "time" => song.time = Some(parse_time(value)?),
                _ => {}
            }
        }
    }

    for row in rows {
        let line = row_text(row).trim().to_string();
        let lowercase = line.to_ascii_lowercase();
        if is_copyright_line(&line) {
            song.copyright = Some(line.clone());
            let copyright_body = line
                .strip_prefix("© ")
                .or_else(|| line.strip_prefix("Copyright "));
            if let Some(copyright_body) = copyright_body {
                let (_, publishers) = copyright_body
                    .split_once(' ')
                    .unwrap_or((copyright_body, ""));
                let publishers = publishers.trim();
                if !publishers.is_empty() {
                    song.tags
                        .insert("pdf.publishers".into(), publishers.to_string());
                }
            }
        }
        if lowercase.contains("ccli") {
            let digits = line
                .split_whitespace()
                .rev()
                .find(|part| part.chars().all(|ch| ch.is_ascii_digit()) && !part.is_empty());
            if let Some(digits) = digits {
                if lowercase.contains("lizenz") || lowercase.contains("license") {
                    song.tags
                        .insert("pdf.ccli_license_number".into(), digits.to_string());
                } else if lowercase.contains("liednummer")
                    || lowercase.contains("song number")
                    || lowercase.contains("songnummer")
                {
                    song.tags
                        .insert("pdf.ccli_song_number".into(), digits.to_string());
                }
            }
        }
    }
    Ok(song)
}

fn is_metadata_row(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    ["key", "tempo", "time"]
        .iter()
        .any(|field| lower.contains(&format!("{field} -")) || lower.contains(&format!("{field}:")))
}

fn parse_metadata_segment(segment: &str) -> Result<Option<(&str, &str)>, Error> {
    let trimmed = segment.trim();
    if let Some((label, value)) = trimmed
        .split_once(" - ")
        .or_else(|| trimmed.split_once(": "))
    {
        return Ok(Some((label.trim(), value.trim())));
    }
    let lower = trimmed.to_ascii_lowercase();
    if ["key", "tempo", "time"].iter().any(|field| {
        lower == *field
            || lower.starts_with(&format!("{field} "))
            || lower.starts_with(&format!("{field}:"))
    }) {
        return Err(Error::Parse(format!(
            "PDF metadata field {trimmed:?} is malformed; expected `Field - value`"
        )));
    }
    Ok(None)
}

fn parse_tempo(value: &str) -> Result<(&str, Option<String>), Error> {
    let value = value.trim();
    if let Some(open) = value.find('(') {
        if !value.ends_with(')') || open == 0 {
            return Err(Error::Parse(format!(
                "PDF metadata tempo {value:?} must be an integer optionally followed by a beat unit such as `(1/8)`"
            )));
        }
        let tempo = value[..open].trim();
        let beat = value[open + 1..value.len() - 1].trim();
        let (numerator, denominator) = beat.split_once('/').ok_or_else(|| {
            Error::Parse(format!(
                "PDF tempo beat unit {beat:?} must use numerator/denominator"
            ))
        })?;
        let numerator = parse_positive_u32(numerator, "tempo beat-unit numerator")?;
        let denominator = parse_positive_u32(denominator, "tempo beat-unit denominator")?;
        if !tempo.chars().all(|ch| ch.is_ascii_digit()) {
            return Err(Error::Parse(format!(
                "PDF metadata tempo {tempo:?} is not an integer"
            )));
        }
        Ok((tempo, Some(format!("{numerator}/{denominator}"))))
    } else {
        if !value.chars().all(|ch| ch.is_ascii_digit()) {
            return Err(Error::Parse(format!(
                "PDF metadata tempo {value:?} is not an integer"
            )));
        }
        Ok((value, None))
    }
}

fn parse_positive_u32(value: &str, field: &str) -> Result<u32, Error> {
    let parsed = value.parse::<u32>().map_err(|_| {
        Error::Parse(format!(
            "PDF metadata {field} {value:?} is not a positive integer"
        ))
    })?;
    if parsed == 0 {
        return Err(Error::Parse(format!(
            "PDF metadata {field} must be greater than zero"
        )));
    }
    Ok(parsed)
}

fn parse_time(value: &str) -> Result<(u32, u32), Error> {
    let (numerator, denominator) = value.split_once('/').ok_or_else(|| {
        Error::Parse(format!(
            "PDF metadata time {value:?} must use numerator/denominator"
        ))
    })?;
    if denominator.contains('/') {
        return Err(Error::Parse(format!(
            "PDF metadata time {value:?} is malformed"
        )));
    }
    Ok((
        parse_positive_u32(numerator.trim(), "time numerator")?,
        parse_positive_u32(denominator.trim(), "time denominator")?,
    ))
}

fn is_metadata_or_footer(line: &str) -> bool {
    let line = line.to_ascii_lowercase();
    line.contains("key -")
        || line.contains("tempo -")
        || line.contains("time -")
        || line.contains("ccli")
        || is_copyright_line(&line)
}

fn is_copyright_line(line: &str) -> bool {
    line.starts_with('©') || line.to_ascii_lowercase().starts_with("copyright ")
}

fn parse_body(rows: &[Row], song: &Song) -> Result<(Vec<Section>, usize), Error> {
    let first_page_meta_y = rows
        .iter()
        .filter(|row| row.page == 0)
        .find(|row| is_metadata_row(&row_text(row)))
        .map(|row| row.y);

    let mut body_indices = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        let text = row_text(row);
        let line = text.trim();
        if line.is_empty() || is_footer_line(line) {
            continue;
        }
        if row.page == 0 {
            if let Some(meta_y) = first_page_meta_y {
                if row.y >= meta_y {
                    continue;
                }
            } else if text.trim() == song.title()
                || row.y > rows.iter().find(|r| r.page == 0).unwrap().y - 20.0
            {
                continue;
            }
        }
        body_indices.push(index);
    }

    let mut chord_rows = BTreeMap::new();
    for &index in &body_indices {
        let row = &rows[index];
        if let Some(tokens) = parse_content_tokens(&row.glyphs, song.key.as_ref())
            && (tokens
                .iter()
                .any(|token| matches!(token, ContentToken::Chord { .. }))
                || tokens
                    .iter()
                    .any(|token| matches!(token, ContentToken::Text { text, .. } if is_bar(text))))
        {
            chord_rows.insert(
                index,
                ChordRow {
                    tokens,
                    glyphs: row.glyphs.clone(),
                },
            );
        }
    }

    // Small raised glyph rows (for example the `sus` and `7(4)` in the
    // sample) belong to the nearest chord row above the lyrics.
    let root_rows: Vec<usize> = chord_rows.keys().copied().collect();
    for &root_index in &root_rows {
        let root = &rows[root_index];
        let mut combined = chord_rows[&root_index].glyphs.clone();
        for &candidate in &body_indices {
            if candidate == root_index {
                continue;
            }
            let raised = &rows[candidate];
            if raised.page == root.page && (raised.y - root.y).abs() <= 5.8 {
                let text = row_text(raised);
                if is_chord_fragment(&text) {
                    combined.extend(raised.glyphs.iter().cloned());
                }
            }
        }
        if let Some(tokens) = parse_content_tokens(&combined, song.key.as_ref()) {
            chord_rows.insert(
                root_index,
                ChordRow {
                    tokens,
                    glyphs: combined,
                },
            );
        }
    }

    let mut attached_to: BTreeMap<usize, usize> = BTreeMap::new();
    let mut consumed_roots = BTreeSet::new();
    for root_index in chord_rows.keys() {
        let root = &rows[*root_index];
        let following: Vec<(usize, f32)> = body_indices
            .iter()
            .filter_map(|&candidate| {
                if candidate == *root_index {
                    return None;
                }
                let target = &rows[candidate];
                if target.page != root.page || !is_lyric_row(target, song.key.as_ref()) {
                    return None;
                }
                let delta = root.y - target.y;
                (5.0..=20.0).contains(&delta).then_some((candidate, delta))
            })
            .collect();
        if following.len() > 1 {
            return Err(pdf_row_error(
                root.page,
                root.y,
                "chord row is close to multiple lyric rows; chord placement is ambiguous",
            ));
        }
        if let Some((target, _)) = following.first() {
            if attached_to.insert(*target, *root_index).is_some() {
                return Err(pdf_row_error(
                    root.page,
                    root.y,
                    "multiple chord rows map to one lyric row; chord placement is ambiguous",
                ));
            }
            consumed_roots.insert(*root_index);
        }
    }

    let mut consumed_fragments = BTreeSet::new();
    for (&root_index, chord_row) in &chord_rows {
        let root = &rows[root_index];
        for &candidate in &body_indices {
            if candidate == root_index || chord_rows.contains_key(&candidate) {
                continue;
            }
            let fragment = &rows[candidate];
            if fragment.page == root.page
                && (fragment.y - root.y).abs() <= 5.8
                && is_chord_fragment(&row_text(fragment))
                && horizontal_overlap(&chord_row.glyphs, &fragment.glyphs, 12.0)
            {
                consumed_fragments.insert(candidate);
            }
        }
    }

    let mut sections = Vec::new();
    let mut current: Option<Section> = None;
    let mut body_line_count = 0;
    for &index in &body_indices {
        let row = &rows[index];
        let line = row_text(row);
        let trimmed = line.trim();

        if !chord_rows.contains_key(&index)
            && let Some(title) = section_heading(trimmed, row)
        {
            if let Some(section) = current.take() {
                sections.push(section);
            }
            current = Some(Section::new(title, Vec::new()));
            continue;
        }
        if consumed_fragments.contains(&index) || consumed_roots.contains(&index) {
            continue;
        }
        if current.is_none() {
            if is_standalone_cue(trimmed) || is_standalone_nc(trimmed) {
                return Err(pdf_row_error(
                    row.page,
                    row.y,
                    "cue appears before a section heading",
                ));
            }
            if is_lyric_row(row, song.key.as_ref()) || chord_rows.contains_key(&index) {
                return Err(pdf_row_error(
                    row.page,
                    row.y,
                    "song content appears before a section heading",
                ));
            }
            continue;
        }

        let is_chord_only = chord_rows.contains_key(&index) && !consumed_roots.contains(&index);
        let is_lyric = is_lyric_row(row, song.key.as_ref());
        let result = if is_lyric {
            let chord_row = attached_to
                .get(&index)
                .and_then(|root| chord_rows.get(root));
            lyric_line(row, chord_row, row.page)?
        } else if is_chord_only {
            chord_only_line(&chord_rows[&index], row.page)?
        } else if trimmed.starts_with('(') && trimmed.ends_with(')') {
            Line::new(vec![Part::new_comment(trimmed.to_string())])
        } else if is_standalone_nc(trimmed) {
            Line::new(vec![Part::new_comment("N.C.".into())])
        } else {
            return Err(pdf_row_error(
                row.page,
                row.y,
                format!(
                    "could not classify song text {trimmed:?} as a section, lyric, chord row, or cue"
                ),
            ));
        };
        current
            .as_mut()
            .expect("section checked above")
            .lines
            .push(result);
        body_line_count += 1;
    }

    if let Some(section) = current {
        sections.push(section);
    }
    Ok((sections, body_line_count))
}

fn is_footer_line(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    is_copyright_line(line)
        || lower.contains("ccli")
        || lower.starts_with("nutzung ausschließlich")
        || lower == "music"
}

fn is_standalone_cue(line: &str) -> bool {
    line.starts_with('(') && line.ends_with(')')
}

fn is_standalone_nc(line: &str) -> bool {
    let compact: String = line.chars().filter(|ch| !ch.is_whitespace()).collect();
    compact.eq_ignore_ascii_case("N.C.") || compact.eq_ignore_ascii_case("NC")
}

fn section_heading(line: &str, row: &Row) -> Option<String> {
    if line.is_empty() || line.starts_with('(') || is_footer_line(line) {
        return None;
    }
    let letters: Vec<char> = line.chars().filter(|ch| ch.is_alphabetic()).collect();
    if letters.is_empty() || !letters.iter().all(|ch| ch.is_uppercase()) {
        return None;
    }
    if line.contains('/') || line.contains('|') || line.contains('.') {
        return None;
    }
    let _emphasized = row.glyphs.iter().any(|glyph| glyph.bold);
    Some(line.to_string())
}

fn is_lyric_row(row: &Row, key: Option<&SimpleChord>) -> bool {
    let line = row_text(row);
    let trimmed = line.trim();
    if trimmed.is_empty() || section_heading(trimmed, row).is_some() || is_footer_line(trimmed) {
        return false;
    }
    if parse_content_tokens(&row.glyphs, key).is_some() {
        return false;
    }
    if is_chord_fragment(trimmed) {
        return false;
    }
    if trimmed.starts_with('(') && trimmed.ends_with(')') {
        return false;
    }
    trimmed.chars().any(|ch| ch.is_lowercase()) || trimmed.split_whitespace().count() > 1
}

fn is_chord_fragment(text: &str) -> bool {
    let text = text.trim();
    if text.is_empty() {
        return false;
    }
    let compact: String = text.chars().filter(|ch| !ch.is_whitespace()).collect();
    !compact.is_empty()
        && compact.chars().all(|ch| {
            ch.is_ascii_alphanumeric()
                || matches!(
                    ch,
                    '#' | '♯' | 'b' | '♭' | '(' | ')' | '/' | '+' | '-' | '.'
                )
        })
        && (compact.chars().any(|ch| ch.is_ascii_digit())
            || compact.to_ascii_lowercase().contains("sus")
            || compact.to_ascii_lowercase().contains("maj")
            || compact.to_ascii_lowercase().contains("dim")
            || compact.to_ascii_lowercase().contains("aug"))
}

fn horizontal_overlap(a: &[Glyph], b: &[Glyph], margin: f32) -> bool {
    let (a_min, a_max) = glyph_x_bounds(a);
    let (b_min, b_max) = glyph_x_bounds(b);
    a_min <= b_max + margin && b_min <= a_max + margin
}

fn glyph_x_bounds(glyphs: &[Glyph]) -> (f32, f32) {
    let min = glyphs
        .iter()
        .map(|glyph| glyph.x)
        .fold(f32::INFINITY, f32::min);
    let max = glyphs
        .iter()
        .map(|glyph| glyph.end_x)
        .fold(f32::NEG_INFINITY, f32::max);
    (min, max)
}

fn parse_content_tokens(glyphs: &[Glyph], key: Option<&SimpleChord>) -> Option<Vec<ContentToken>> {
    let mut ordered: Vec<&Glyph> = glyphs
        .iter()
        .filter(|glyph| !glyph.ch.is_whitespace())
        .collect();
    ordered.sort_by(|a, b| a.x.total_cmp(&b.x).then_with(|| b.y.total_cmp(&a.y)));
    if ordered.is_empty() {
        return None;
    }

    let mut removed = BTreeSet::new();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i + 3 < ordered.len() {
        let candidate: String = ordered[i..i + 4].iter().map(|glyph| glyph.ch).collect();
        if candidate.eq_ignore_ascii_case("N.C.") {
            for glyph in &ordered[i..i + 4] {
                removed.insert(*glyph as *const Glyph as usize);
            }
            tokens.push(ContentToken::Text {
                x: ordered[i].x,
                text: "N.C.".into(),
                comment: true,
            });
            i += 4;
        } else {
            i += 1;
        }
    }

    // Parenthetical cues include words; chord suffixes such as `(4)` remain
    // attached to their chord token.
    let mut all_by_x: Vec<&Glyph> = glyphs.iter().collect();
    all_by_x.sort_by(|a, b| a.x.total_cmp(&b.x).then_with(|| b.y.total_cmp(&a.y)));
    let mut j = 0;
    while j < all_by_x.len() {
        if all_by_x[j].ch != '(' {
            j += 1;
            continue;
        }
        let Some(end) = all_by_x[j + 1..].iter().position(|glyph| glyph.ch == ')') else {
            j += 1;
            continue;
        };
        let end = j + 1 + end;
        let cue_glyphs = &all_by_x[j..=end];
        let cue: String = cue_glyphs.iter().map(|glyph| glyph.ch).collect();
        let inner = cue.trim_matches(['(', ')']);
        let is_word_cue = inner.chars().any(char::is_whitespace)
            || (inner.chars().filter(|ch| ch.is_alphabetic()).count() >= 2
                && inner.chars().any(char::is_lowercase)
                && !inner.to_ascii_lowercase().starts_with("maj"));
        if is_word_cue {
            for glyph in cue_glyphs {
                removed.insert(*glyph as *const Glyph as usize);
            }
            tokens.push(ContentToken::Text {
                x: all_by_x[j].x,
                text: cue.trim().to_string(),
                comment: true,
            });
            j = end + 1;
        } else {
            j += 1;
        }
    }

    let remaining: Vec<&Glyph> = glyphs
        .iter()
        .filter(|glyph| {
            !glyph.ch.is_whitespace() && !removed.contains(&(*glyph as *const Glyph as usize))
        })
        .collect();
    let atoms = visual_atoms(remaining);
    if atoms.is_empty() {
        return Some(tokens);
    }
    for atom in atoms {
        let text: String = atom.iter().map(|glyph| glyph.ch).collect();
        if text.is_empty() {
            continue;
        }
        if is_bar(&text) {
            tokens.push(ContentToken::Text {
                x: atom[0].x,
                text,
                comment: false,
            });
        } else if let Ok(chord) = parse_pdf_chord(&text, key) {
            tokens.push(ContentToken::Chord {
                x: atom[0].x,
                chord,
            });
        } else {
            return None;
        }
    }
    tokens.sort_by(|a, b| token_x(a).total_cmp(&token_x(b)));
    Some(tokens)
}

fn visual_atoms(mut glyphs: Vec<&Glyph>) -> Vec<Vec<&Glyph>> {
    glyphs.sort_by(|a, b| a.x.total_cmp(&b.x).then_with(|| b.y.total_cmp(&a.y)));
    let mut atoms: Vec<Vec<&Glyph>> = Vec::new();
    for glyph in glyphs {
        let should_join = atoms
            .last()
            .and_then(|atom| atom.last())
            .is_some_and(|previous| {
                let gap = glyph.x - previous.end_x;
                let tolerance = (glyph.font_size.min(previous.font_size) * 0.48).max(2.0);
                gap <= tolerance
            });
        if should_join {
            atoms.last_mut().expect("last atom").push(glyph);
        } else {
            atoms.push(vec![glyph]);
        }
    }
    atoms
}

fn token_x(token: &ContentToken) -> f32 {
    match token {
        ContentToken::Chord { x, .. } | ContentToken::Text { x, .. } => *x,
    }
}

fn is_bar(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|ch| matches!(ch, '|' | ':' | '¦'))
}

fn parse_pdf_chord(token: &str, key: Option<&SimpleChord>) -> Result<Chord, Error> {
    let parsed = Chord::from_str_with_key(token, key)?;
    let mut symbol = token;
    if symbol.starts_with('(') && symbol.ends_with(')') {
        symbol = &symbol[1..symbol.len() - 1];
    }
    if let Some((before, duration)) = symbol.split_once(':') {
        symbol = before;
        if duration.is_empty()
            || !duration
                .chars()
                .all(|ch| ch.is_ascii_digit() || matches!(ch, '.' | ','))
        {
            return Err(Error::Parse(format!("invalid chord duration in {token:?}")));
        }
    }
    let mut slash_parts = symbol.split('/');
    let main = slash_parts.next().unwrap_or_default();
    let bass = slash_parts.next();
    if slash_parts.next().is_some() {
        return Err(Error::Parse(format!("invalid slash chord {token:?}")));
    }
    let root_len = root_prefix_len(main)
        .ok_or_else(|| Error::Parse(format!("invalid chord root in {token:?}")))?;
    if !valid_suffix(&main[root_len..])
        || bass.is_some_and(|bass| root_prefix_len(bass) != Some(bass.len()))
    {
        return Err(Error::Parse(format!(
            "unrecognized chord spelling {token:?}"
        )));
    }
    Ok(parsed)
}

fn root_prefix_len(symbol: &str) -> Option<usize> {
    let mut chars = symbol.char_indices();
    let (start, root) = chars.next()?;
    if !matches!(root, 'A'..='H') {
        return None;
    }
    let mut end = start + root.len_utf8();
    if let Some((index, accidental @ ('#' | 'b' | '♯' | '♭'))) = chars.next() {
        end = index + accidental.len_utf8();
    }
    Some(end)
}

fn valid_suffix(suffix: &str) -> bool {
    let suffix = suffix.trim_matches(['(', ')']);
    if suffix.is_empty() {
        return true;
    }
    let suffix = suffix.to_ascii_lowercase();
    [
        "", "m", "maj", "sus", "sus2", "sus4", "dim", "aug", "add", "no", "omit", "°", "+",
    ]
    .iter()
    .any(|prefix| {
        suffix.strip_prefix(prefix).is_some_and(|rest| {
            rest.is_empty()
                || (rest
                    .chars()
                    .all(|ch| ch.is_ascii_digit() || matches!(ch, 'b' | '#' | '+' | '-'))
                    && rest.chars().any(|ch| ch.is_ascii_digit()))
        })
    })
}

fn lyric_line(row: &Row, chord_row: Option<&ChordRow>, page: usize) -> Result<Line, Error> {
    let (lyric_glyphs, inline_cues) = remove_inline_cues(&row.glyphs);
    let mut lyric_glyphs = lyric_glyphs;
    lyric_glyphs.sort_by(|a, b| a.x.total_cmp(&b.x));
    let lyric_text: String = lyric_glyphs.iter().map(|glyph| glyph.ch).collect();
    let mut entries: Vec<(f32, Part)> = Vec::new();
    for cue in inline_cues {
        entries.push((cue.0, Part::new_comment(cue.1)));
    }

    let mut onsets = Vec::new();
    let mut side_tokens = Vec::new();
    if let Some(chord_row) = chord_row {
        for token in &chord_row.tokens {
            match token {
                ContentToken::Chord { x, chord } => {
                    let exact = lyric_glyphs
                        .iter()
                        .enumerate()
                        .filter(|(_, glyph)| glyph.x <= *x + 0.8 && glyph.end_x >= *x - 0.8)
                        .min_by(|(_, a), (_, b)| {
                            (a.x - *x)
                                .abs()
                                .total_cmp(&(b.x - *x).abs())
                                .then_with(|| a.ch.is_whitespace().cmp(&b.ch.is_whitespace()))
                        });
                    let target = exact.or_else(|| {
                        lyric_glyphs
                            .iter()
                            .enumerate()
                            .find(|(_, glyph)| !glyph.ch.is_whitespace() && glyph.x + 0.5 >= *x)
                    });
                    let Some((char_index, glyph)) = target else {
                        return Err(pdf_row_error(
                            page,
                            row.y,
                            format!("chord at x={x:.1} has no following lyric position"),
                        ));
                    };
                    if glyph.x - *x > (glyph.font_size * 4.0).max(40.0) {
                        return Err(pdf_row_error(
                            page,
                            row.y,
                            format!("chord at x={x:.1} is too far from the next lyric position"),
                        ));
                    }
                    onsets.push((char_index, *x, chord.clone()));
                }
                ContentToken::Text { x, text, comment } => {
                    side_tokens.push((*x, text.clone(), *comment));
                }
            }
        }
    }
    onsets.sort_by_key(|(index, _, _)| *index);
    for pair in onsets.windows(2) {
        if pair[0].0 == pair[1].0 {
            return Err(pdf_row_error(
                page,
                row.y,
                "multiple chords point to the same lyric position; chord placement is ambiguous",
            ));
        }
    }

    let chars: Vec<char> = lyric_text.chars().collect();
    let mut cursor = 0;
    for (onset_index, x, chord) in &onsets {
        if *onset_index > cursor {
            entries.push((
                lyric_glyphs.get(cursor).map(|glyph| glyph.x).unwrap_or(*x),
                Part {
                    chord: None,
                    languages: vec![chars[cursor..*onset_index].iter().collect()],
                    comment: false,
                },
            ));
        }
        let next_index = onsets
            .iter()
            .find(|(index, _, _)| index > onset_index)
            .map(|(index, _, _)| *index)
            .unwrap_or(chars.len());
        entries.push((
            *x,
            Part {
                chord: Some(chord.clone()),
                languages: vec![chars[*onset_index..next_index].iter().collect()],
                comment: false,
            },
        ));
        cursor = next_index;
    }
    if cursor < chars.len() {
        entries.push((
            lyric_glyphs.get(cursor).map(|glyph| glyph.x).unwrap_or(0.0),
            Part {
                chord: None,
                languages: vec![chars[cursor..].iter().collect()],
                comment: false,
            },
        ));
    }
    for (x, text, comment) in side_tokens {
        entries.push((
            x,
            if comment {
                Part::new_comment(text)
            } else {
                Part {
                    chord: None,
                    languages: vec![text],
                    comment: false,
                }
            },
        ));
    }
    entries.sort_by(|a, b| a.0.total_cmp(&b.0));
    Ok(Line::new(
        entries.into_iter().map(|(_, part)| part).collect(),
    ))
}

fn remove_inline_cues(glyphs: &[Glyph]) -> (Vec<Glyph>, Vec<(f32, String)>) {
    let mut ordered: Vec<&Glyph> = glyphs.iter().collect();
    ordered.sort_by(|a, b| a.x.total_cmp(&b.x));
    let mut removed = BTreeSet::new();
    let mut cues = Vec::new();
    let mut index = 0;
    while index < ordered.len() {
        if ordered[index].ch != '(' {
            index += 1;
            continue;
        }
        if let Some(relative_end) = ordered[index + 1..]
            .iter()
            .position(|glyph| glyph.ch == ')')
        {
            let end = index + 1 + relative_end;
            let cue: String = ordered[index..=end].iter().map(|glyph| glyph.ch).collect();
            let inner = cue.trim_matches(['(', ')']);
            if inner.chars().any(char::is_whitespace)
                || inner.chars().filter(|ch| ch.is_alphabetic()).count() >= 2
            {
                for glyph in &ordered[index..=end] {
                    removed.insert(*glyph as *const Glyph as usize);
                }
                cues.push((ordered[index].x, cue));
                index = end + 1;
                continue;
            }
        }
        index += 1;
    }
    let result = glyphs
        .iter()
        .filter(|glyph| !removed.contains(&(*glyph as *const Glyph as usize)))
        .cloned()
        .collect();
    (result, cues)
}

fn chord_only_line(chord_row: &ChordRow, page: usize) -> Result<Line, Error> {
    let mut parts = Vec::new();
    for token in &chord_row.tokens {
        match token {
            ContentToken::Chord { chord, .. } => parts.push(Part::new_chord(chord.clone())),
            ContentToken::Text { text, comment, .. } if *comment => {
                parts.push(Part::new_comment(text.clone()));
            }
            ContentToken::Text { text, .. } => parts.push(Part {
                chord: None,
                languages: vec![text.clone()],
                comment: false,
            }),
        }
    }
    if parts.is_empty() {
        return Err(pdf_row_error(page, 0.0, "empty chord-only row"));
    }
    Ok(Line::new(parts))
}

fn pdf_row_error(page: usize, y: f32, message: impl Into<String>) -> Error {
    Error::Parse(format!(
        "PDF page {} near y={y:.1}: {}",
        page + 1,
        message.into()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(size: u8, x: u16, y: u16, value: &str) -> String {
        let escaped = value
            .replace('\\', "\\\\")
            .replace('(', "\\(")
            .replace(')', "\\)");
        format!("BT /F1 {size} Tf {x} {y} Td ({escaped}) Tj ET\n")
    }

    fn pdf(pages: &[String]) -> Vec<u8> {
        let mut objects = vec![
            "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
            String::new(),
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
        ];
        let page_ids: Vec<usize> = (0..pages.len()).map(|index| 4 + index * 2).collect();
        objects[1] = format!(
            "<< /Type /Pages /Kids [{}] /Count {} >>",
            page_ids
                .iter()
                .map(|id| format!("{id} 0 R"))
                .collect::<Vec<_>>()
                .join(" "),
            pages.len()
        );
        for (index, content) in pages.iter().enumerate() {
            let page_id = 4 + index * 2;
            let content_id = page_id + 1;
            objects.push(format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 3 0 R >> >> /Contents {content_id} 0 R >>"
            ));
            objects.push(format!(
                "<< /Length {} >>\nstream\n{}\nendstream",
                content.len(),
                content
            ));
        }
        let mut output = b"%PDF-1.4\n".to_vec();
        let mut offsets = vec![0usize];
        for (index, object) in objects.iter().enumerate() {
            offsets.push(output.len());
            output
                .extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", index + 1, object).as_bytes());
        }
        let xref = output.len();
        output.extend_from_slice(
            format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
        );
        for offset in offsets.iter().skip(1) {
            output.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        output.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
                objects.len() + 1
            )
            .as_bytes(),
        );
        output
    }

    fn sample_pages() -> Vec<String> {
        let mut first = String::new();
        // Write body objects first to ensure import follows visual coordinates,
        // not content-stream insertion order.
        first.push_str(&text(14, 72, 664, "Hello world"));
        first.push_str(&text(10, 72, 644, "(REPEAT)"));
        first.push_str(&text(14, 72, 676, "G"));
        first.push_str(&text(8, 78, 680, "sus4"));
        first.push_str(&text(12, 72, 690, "VERSE"));
        first.push_str(&text(
            10,
            72,
            716,
            "Key - G | Tempo - 120 (1/8) | Time - 6/8",
        ));
        first.push_str(&text(10, 72, 732, "One Artist | Another Artist"));
        first.push_str(&text(18, 72, 750, "Sample Song"));
        let mut second = String::new();
        second.push_str(&text(14, 72, 740, "We continue here"));
        vec![first, second]
    }

    #[test]
    fn imports_metadata_superscript_chord_cue_and_page_continuation() {
        let song = load_bytes(&pdf(&sample_pages())).expect("import generated PDF");
        assert_eq!(song.title(), "Sample Song");
        assert_eq!(song.artists, ["One Artist", "Another Artist"]);
        assert_eq!(song.key, Some(SimpleChord::try_from("G").unwrap()));
        assert_eq!(song.tempo, Some(120));
        assert_eq!(song.time, Some((6, 8)));
        assert_eq!(
            song.tags.get("pdf.tempo_beat_unit").map(String::as_str),
            Some("1/8")
        );
        assert_eq!(song.sections.len(), 1);
        assert_eq!(song.sections[0].title, "VERSE");
        assert_eq!(song.sections[0].lines.len(), 3);
        let first = &song.sections[0].lines[0];
        assert_eq!(
            first.parts[0].chord.as_ref(),
            Some(&Chord::from_str_with_key("Gsus4", Some(song.key.as_ref().unwrap())).unwrap())
        );
        assert_eq!(first.parts[0].languages[0], "Hello world");
        assert!(song.sections[0].lines[1].parts[0].comment);
        assert_eq!(
            song.sections[0].lines[2].parts[0].languages[0],
            "We continue here"
        );
    }

    #[test]
    fn rejects_invalid_present_metadata_and_empty_title() {
        let mut pages = sample_pages();
        pages[0] = pages[0].replace("120", "fast");
        assert!(
            load_bytes(&pdf(&pages))
                .unwrap_err()
                .to_string()
                .contains("tempo")
        );

        let mut pages = sample_pages();
        pages[0] = pages[0].replace("Key - G", "Key - Hx");
        assert!(
            load_bytes(&pdf(&pages))
                .unwrap_err()
                .to_string()
                .contains("key")
        );

        let mut pages = sample_pages();
        pages[0] = pages[0].replace("Sample Song", "   ");
        assert!(
            load_bytes(&pdf(&pages))
                .unwrap_err()
                .to_string()
                .contains("title")
        );

        let mut pages = sample_pages();
        pages[0] = pages[0].replace("6/8", "4/0");
        assert!(
            load_bytes(&pdf(&pages))
                .unwrap_err()
                .to_string()
                .contains("denominator")
        );
    }

    #[test]
    fn accepts_absent_optional_metadata_and_preserves_footer_fields() {
        let mut page = String::new();
        page.push_str(&text(18, 72, 750, "Metadata Optional"));
        page.push_str(&text(12, 72, 710, "VERSE"));
        page.push_str(&text(14, 72, 690, "Words"));
        page.push_str(&text(
            7,
            40,
            90,
            "Copyright 2024 Example Publishing | Another Publisher",
        ));
        page.push_str(&text(7, 40, 80, "CCLI Song Number 123456"));
        page.push_str(&text(7, 40, 70, "CCLI License Number 987654"));
        let song = load_bytes(&pdf(&[page])).expect("metadata fields are optional");
        assert!(song.key.is_none());
        assert!(song.tempo.is_none());
        assert!(song.time.is_none());
        assert_eq!(
            song.copyright.as_deref(),
            Some("Copyright 2024 Example Publishing | Another Publisher")
        );
        assert_eq!(
            song.tags.get("pdf.publishers").map(String::as_str),
            Some("Example Publishing | Another Publisher")
        );
        assert_eq!(
            song.tags.get("pdf.ccli_song_number").map(String::as_str),
            Some("123456")
        );
        assert_eq!(
            song.tags.get("pdf.ccli_license_number").map(String::as_str),
            Some("987654")
        );
    }

    #[test]
    fn maps_multiple_slash_chords_and_keeps_instrumental_bars() {
        let mut page = String::new();
        page.push_str(&text(18, 72, 750, "Positioned Chords"));
        page.push_str(&text(12, 72, 710, "VERSE"));
        page.push_str(&text(14, 72, 680, "C#m7"));
        page.push_str(&text(14, 140, 680, "D/F#"));
        page.push_str(&text(14, 72, 668, "A"));
        page.push_str(&text(14, 140, 668, "B"));
        page.push_str(&text(12, 72, 640, "INTRO"));
        page.push_str(&text(14, 72, 620, "||:"));
        page.push_str(&text(14, 110, 620, "C"));
        page.push_str(&text(14, 150, 620, "|"));
        page.push_str(&text(14, 180, 620, "D/F#"));
        page.push_str(&text(14, 250, 620, ":||"));
        page.push_str(&text(12, 72, 600, "N.C."));
        let song = load_bytes(&pdf(&[page])).expect("import chord variants");
        let verse = &song.sections[0];
        assert_eq!(verse.title, "VERSE");
        assert_eq!(verse.lines[0].parts.len(), 2);
        assert_eq!(
            verse.lines[0].parts[0].chord.as_ref(),
            Some(&Chord::from_str_with_key("C#m7", None).unwrap())
        );
        assert_eq!(
            verse.lines[0].parts[1].chord.as_ref(),
            Some(&Chord::from_str_with_key("D/F#", None).unwrap())
        );

        let intro = &song.sections[1];
        assert!(
            intro.lines[0]
                .parts
                .iter()
                .any(|part| part.languages == ["||:"])
        );
        assert!(
            intro.lines[0]
                .parts
                .iter()
                .any(|part| part.languages == ["|"])
        );
        assert!(
            intro.lines[0]
                .parts
                .iter()
                .any(|part| part.languages == [":||"])
        );
        assert!(intro.lines[0].parts.iter().any(|part| part.chord.is_some()));
        assert!(intro.lines[1].parts[0].comment);
        assert_eq!(intro.lines[1].parts[0].languages[0], "N.C.");
    }

    #[test]
    fn rejects_malformed_and_textless_documents() {
        assert!(
            load_bytes(b"not a PDF")
                .unwrap_err()
                .to_string()
                .contains("invalid PDF")
        );
        assert!(
            load_bytes(&pdf(&[String::new()]))
                .unwrap_err()
                .to_string()
                .contains("no usable text layer")
        );
    }

    #[test]
    fn rejects_chords_without_a_following_lyric_position() {
        let mut page = String::new();
        page.push_str(&text(18, 72, 750, "Ambiguous"));
        page.push_str(&text(12, 72, 720, "VERSE"));
        page.push_str(&text(14, 300, 700, "G"));
        page.push_str(&text(14, 72, 688, "Hi"));
        assert!(
            load_bytes(&pdf(&[page]))
                .unwrap_err()
                .to_string()
                .contains("following lyric position")
        );
    }

    #[test]
    fn strict_metadata_helpers_reject_zero_or_extra_values() {
        assert!(parse_time("6/0").is_err());
        assert!(parse_time("6/8/4").is_err());
        assert!(parse_tempo("120 (3)").is_err());
        assert!(parse_tempo("120bpm").is_err());
    }
}
