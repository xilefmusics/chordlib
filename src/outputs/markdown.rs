use serde_yaml::to_string;
use unicode_width::UnicodeWidthStr;

use crate::Error;
use crate::markdown::{FrontMatter, is_plausible_chord_token};
use crate::outputs::FormatChordPro;
use crate::types::{ChordRepresentation, Line, SimpleChord, Song};

pub trait FormatMarkdown {
    fn format_markdown(
        &self,
        key: Option<&SimpleChord>,
        representation: Option<&ChordRepresentation>,
    ) -> Result<String, Error>;
}

struct RenderedLine {
    chord_row: Option<String>,
    text_row: String,
}

impl FormatMarkdown for &Song {
    fn format_markdown(
        &self,
        key: Option<&SimpleChord>,
        representation: Option<&ChordRepresentation>,
    ) -> Result<String, Error> {
        if !self.titles.iter().any(|title| !title.trim().is_empty()) {
            return Err(Error::Other(
                "markdown export requires at least one non-empty title".into(),
            ));
        }

        let default_key = SimpleChord::default();
        let song_key = self.key.as_ref().unwrap_or(&default_key);
        let key = key.unwrap_or(song_key);
        let default_representation = ChordRepresentation::Default;
        let representation = representation.unwrap_or(&default_representation);

        let front_matter = FrontMatter {
            titles: self.titles.clone(),
            subtitle: self.subtitle.clone(),
            copyright: self.copyright.clone(),
            key: self.key.as_ref().map(|song_key| {
                song_key
                    .format(&default_key, &ChordRepresentation::Default)
                    .to_string()
            }),
            artists: self.artists.clone(),
            languages: if self.languages.iter().all(String::is_empty) {
                Vec::new()
            } else {
                self.languages.clone()
            },
            tempo: self.tempo,
            time: self
                .time
                .map(|(numerator, denominator)| format!("{numerator}/{denominator}")),
            tags: self.tags.clone(),
        };

        let yaml = to_string(&front_matter)
            .map_err(|error| Error::Serialize(format!("markdown front matter: {error}")))?;
        let yaml = yaml
            .lines()
            .map(|line| match line {
                "subtitle: null" => "subtitle:",
                "copyright: null" => "copyright:",
                "key: null" => "key:",
                "tempo: null" => "tempo:",
                "time: null" => "time:",
                line => line,
            })
            .collect::<Vec<_>>()
            .join("\n");
        let mut output = format!("---\n{}\n---\n", yaml.trim_end());

        for section in &self.sections {
            output.push_str("# ");
            output.push_str(&section.title);
            if section.repeat_count > 1 {
                output.push_str(&format!(" ({}x)", section.repeat_count));
            }
            output.push('\n');

            for line in &section.lines {
                append_line(&mut output, line, key, representation)?;
            }
        }

        Ok(output)
    }
}

fn append_line(
    output: &mut String,
    line: &Line,
    key: &SimpleChord,
    representation: &ChordRepresentation,
) -> Result<(), Error> {
    let max_language = line
        .parts
        .iter()
        .map(|part| part.languages.len())
        .max()
        .unwrap_or(1);
    let base = render_line(line, 0, key, representation)?;
    let has_chords = base.chord_row.is_some();

    if let Some(chord_row) = base.chord_row {
        output.push_str(&chord_row);
        output.push('\n');
    }
    output.push_str(&escape_line_prefix(&base.text_row, key));
    output.push('\n');

    for language in 1..max_language {
        let translated = render_line(line, language, key, representation)?;
        if has_chords {
            let Some(chord_row) = translated.chord_row else {
                return Err(Error::Other(
                    "translated chord line lost its chord sequence".into(),
                ));
            };
            output.push('&');
            output.push_str(&chord_row);
            output.push('\n');
        }
        output.push('&');
        output.push_str(&escape_line_prefix(&translated.text_row, key));
        output.push('\n');
    }

    Ok(())
}

fn render_line(
    line: &Line,
    language: usize,
    key: &SimpleChord,
    representation: &ChordRepresentation,
) -> Result<RenderedLine, Error> {
    let mut chord_row = String::new();
    let mut text_row = String::new();
    let has_text = line.parts.iter().any(|part| {
        part.languages
            .get(language)
            .is_some_and(|text| !text.is_empty())
    });

    for part in &line.parts {
        let text = part
            .languages
            .get(language)
            .map(String::as_str)
            .unwrap_or("");

        if let Some(chord) = &part.chord {
            let mut column = text_row.width();
            let current_width = chord_row.width();
            if !chord_row.is_empty() && current_width >= column {
                if !has_text {
                    // Chord-only lines have no lyric column to move. Separate adjacent
                    // chord tokens directly in the chord row so padding does not become
                    // part of the first chord's lyric when the file is parsed again.
                    column = current_width + 1;
                } else {
                    // Preserve the chord's attachment point by moving the following lyric
                    // text right until the previous chord label has room to finish and a
                    // whitespace separator keeps adjacent chord tokens parseable. A dash
                    // marks padding inside a word so the parser can remove it on a round trip.
                    let padding_width = current_width + 1 - column;
                    let inner_word = text_row
                        .chars()
                        .next_back()
                        .is_some_and(|character| !character.is_whitespace())
                        && text
                            .chars()
                            .next()
                            .is_some_and(|character| !character.is_whitespace());
                    text_row.push_str(&alignment_padding(padding_width, inner_word));
                    column = text_row.width();
                }
            }
            let current_width = chord_row.width();
            chord_row.extend(std::iter::repeat_n(' ', column - current_width));
            chord_row.push_str(&chord.format_chord_pro(
                Some(key),
                Some(representation),
                None,
                true,
            ));
        }

        if part.comment {
            text_row.push_str("**");
            text_row.push_str(&escape_comment_text(text));
            text_row.push_str("**");
        } else {
            text_row.push_str(text);
        }
    }

    Ok(RenderedLine {
        chord_row: (!chord_row.is_empty()).then_some(chord_row),
        text_row,
    })
}

fn alignment_padding(width: usize, inner_word: bool) -> String {
    if !inner_word || width == 0 {
        return " ".repeat(width);
    }

    let mut padding = String::with_capacity(width);
    padding.push('-');
    padding.extend(std::iter::repeat_n(' ', width - 1));
    padding
}

fn escape_line_prefix(text: &str, key: &SimpleChord) -> String {
    let needs_escape =
        text.starts_with('&') || text.starts_with('\\') || looks_like_chord_row(text, key);
    if needs_escape {
        format!("\\{text}")
    } else {
        text.to_string()
    }
}

fn looks_like_chord_row(text: &str, key: &SimpleChord) -> bool {
    if text.trim().is_empty() {
        return false;
    }
    text.split_whitespace().all(|token| {
        is_plausible_chord_token(token)
            && crate::types::Chord::from_str_with_key(token, Some(key)).is_ok()
    })
}

fn escape_comment_text(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(character) = chars.next() {
        if character == '\\' {
            escaped.push_str("\\\\");
        } else if character == '*' && chars.peek() == Some(&'*') {
            chars.next();
            escaped.push_str("\\*\\*");
        } else {
            escaped.push(character);
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inputs::markdown::load_string;

    #[test]
    fn exports_front_matter_repeats_and_translations() {
        let song = load_string(
            "---\ntitles: [Test]\nartists: [Writer]\nlanguages: [en, de]\nkey: C\n---\n# Verse (2x)\nC       G\nAmazing grace\n&C          G\n&Erstaunliche Gnade\n",
        )
        .expect("markdown song");
        let output = (&song)
            .format_markdown(None, Some(&ChordRepresentation::Default))
            .expect("markdown output");
        assert!(output.contains("titles:"));
        assert!(output.contains("# Verse (2x)"));
        assert!(output.contains("&C          G"));
        assert!(output.contains("&Erstaunliche Gnade"));
    }

    #[test]
    fn exports_all_front_matter_fields_when_empty() {
        let song = load_string("---\ntitles: [Test]\nlanguages: [\"\"]\n---\n# Verse\nText\n")
            .expect("markdown song");
        let output = (&song)
            .format_markdown(None, None)
            .expect("markdown output");
        let front_matter = output
            .split_once("---\n# Verse")
            .map(|(front_matter, _)| format!("{front_matter}---"))
            .expect("front matter");
        assert_eq!(
            front_matter,
            "---\ntitles:\n- Test\nsubtitle:\ncopyright:\nkey:\nartists: []\nlanguages: []\ntempo:\ntime:\ntags: {}\n---"
        );
    }

    #[test]
    fn markdown_round_trip_preserves_song() {
        let input = "---\ntitles: [Test]\nartists: [Writer]\nlanguages: [en, de]\nkey: C\ntags: {genre: hymn}\n---\n# Verse (2x)\nC       G\nAmazing grace\n&C          G\n&Erstaunliche Gnade\n**spoken**\n";
        let song = load_string(input).expect("markdown song");
        let output = (&song)
            .format_markdown(None, None)
            .expect("markdown output");
        let again = load_string(&output).expect("markdown round trip");
        assert_eq!(again, song);
    }

    #[test]
    fn exports_comments_and_escapes_ambiguous_text() {
        let input =
            "---\ntitles: [Test]\nkey: C\n---\n# Verse\nC\n**spoken**\n\n\\C G\n\\&literal\n";
        let song = load_string(input).expect("markdown song");
        let output = (&song)
            .format_markdown(None, None)
            .expect("markdown output");
        assert!(output.contains("C\n**spoken**"));
        assert!(output.contains("\\C G"));
        assert!(output.contains("\\&literal"));
        assert_eq!(load_string(&output).expect("round trip"), song);
    }

    #[test]
    fn pads_lyric_columns_when_chords_would_overlap() {
        let song = crate::inputs::chord_pro::load_string(
            "{title: Chord-only line}\n{key: C}\n{section: Intro}\n[C][G]\n",
        )
        .expect("ChordPro song");
        let output = (&song)
            .format_markdown(None, None)
            .expect("Markdown output should repair chord spacing");
        assert!(output.contains("C G\n\n"), "unexpected output:\n{output}");
        let again = load_string(&output).expect("repaired Markdown should parse");
        let parts = &again.sections[0].lines[0].parts;
        assert_eq!(parts.len(), 2);
        assert!(parts.iter().all(|part| part.chord.is_some()));
        assert!(parts.iter().all(|part| part.languages.is_empty()));
    }

    #[test]
    fn marks_inner_word_padding_and_round_trips() {
        let song = crate::inputs::chord_pro::load_string(
            "{title: Inner word}\n{key: C}\n{section: Verse}\n[Eb]H[Bb]ello\n",
        )
        .expect("ChordPro song");
        let output = (&song)
            .format_markdown(None, None)
            .expect("Markdown output should mark inner-word padding");

        assert!(
            output.contains("Eb Bb\nH- ello\n"),
            "unexpected output:\n{output}"
        );
        assert_eq!(
            load_string(&output).expect("marked Markdown should parse"),
            song
        );
    }

    #[test]
    fn preserves_worship_pro_chord_durations() {
        let song =
            load_string("---\ntitles: [Durations]\nkey: C\n---\n# Verse\nC:4   G:2\nHello world\n")
                .expect("Markdown song");

        let parts = &song.sections[0].lines[0].parts;
        assert_eq!(
            parts[0]
                .chord
                .as_ref()
                .and_then(|chord| chord.get_duration()),
            Some(4000)
        );
        assert_eq!(
            parts[1]
                .chord
                .as_ref()
                .and_then(|chord| chord.get_duration()),
            Some(2000)
        );

        let output = (&song)
            .format_markdown(None, None)
            .expect("Markdown output");
        assert!(
            output.contains("C:4   G:2\n"),
            "unexpected output:\n{output}"
        );

        let again = load_string(&output).expect("Markdown round trip");
        let again_parts = &again.sections[0].lines[0].parts;
        assert_eq!(
            again_parts[0]
                .chord
                .as_ref()
                .and_then(|chord| chord.get_duration()),
            Some(4000)
        );
        assert_eq!(
            again_parts[1]
                .chord
                .as_ref()
                .and_then(|chord| chord.get_duration()),
            Some(2000)
        );
    }

    #[test]
    fn exports_without_generated_empty_lines() {
        let song = load_string(
            "---\ntitles: [Test]\n---\n# Verse\nText\n# Empty\n# Chorus (2x)\nMore text\n",
        )
        .expect("Markdown song");
        let output = (&song)
            .format_markdown(None, None)
            .expect("Markdown output");

        assert!(!output.contains("\n\n"), "unexpected blank line:\n{output}");
        assert!(output.contains("---\n# Verse\n"));
        assert!(output.contains("Text\n# Empty\n# Chorus (2x)\nMore text\n"));
    }

    #[test]
    fn exports_empty_references_and_unicode_columns() {
        let song = load_string(
            "---\ntitles: [Test]\nkey: C\n---\n# Verse\nC       G\nÄ       😊\n# Verse (2x)\n",
        )
        .expect("markdown song");
        let output = (&song)
            .format_markdown(None, None)
            .expect("markdown output");
        assert!(output.contains("C       G\nÄ       😊"));
        assert!(output.contains("# Verse (2x)"));
        let again = load_string(&output).expect("round trip");
        assert_eq!(again, song);
        assert!(again.sections[1].lines.is_empty());
    }
}
