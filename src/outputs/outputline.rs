use crate::Error;
use crate::types::{ChordRepresentation, Line, Section, SimpleChord, Song, SongFlowItem};

pub enum OutputLine {
    Keyword(String),
    Chord(String),
    Text(String),
}

pub trait FormatOutputLines {
    fn format_output_lines(
        &self,
        key: Option<&SimpleChord>,
        representation: Option<&ChordRepresentation>,
        language: Option<usize>,
        flow: Option<&[SongFlowItem]>,
    ) -> Result<Vec<OutputLine>, Error>;
}

impl FormatOutputLines for &Line {
    fn format_output_lines(
        &self,
        key: Option<&SimpleChord>,
        representation: Option<&ChordRepresentation>,
        language: Option<usize>,
        _flow: Option<&[SongFlowItem]>,
    ) -> Result<Vec<OutputLine>, Error> {
        let mut chord_line = String::default();
        let mut text_line = String::default();
        let language = language.unwrap_or(0);

        for part in &self.parts {
            if let Some(chord) = &part.chord {
                let chord_chars = chord_line.chars().count();
                let text_chars = text_line.chars().count();
                if text_chars > chord_chars {
                    for _ in 0..(text_chars - chord_chars) {
                        chord_line.push(' ');
                    }
                } else if chord_chars > 0 {
                    chord_line.push(' ');
                }
                chord_line = format!(
                    "{}{}",
                    chord_line,
                    chord.format(
                        key.unwrap_or(&SimpleChord::default()),
                        representation.unwrap_or(&ChordRepresentation::Default)
                    )
                );
            }
            text_line = format!("{}{}", text_line, part.text_for_language(language));
        }

        let mut result = Vec::default();
        if !chord_line.is_empty() {
            result.push(OutputLine::Chord(chord_line));
        }
        if !text_line.is_empty() {
            result.push(OutputLine::Text(text_line));
        }
        Ok(result)
    }
}

impl FormatOutputLines for &Section {
    fn format_output_lines(
        &self,
        key: Option<&SimpleChord>,
        representation: Option<&ChordRepresentation>,
        language: Option<usize>,
        _flow: Option<&[SongFlowItem]>,
    ) -> Result<Vec<OutputLine>, Error> {
        let mut output = vec![OutputLine::Keyword(self.title.clone())];
        for line in &self.lines {
            output.extend(line.format_output_lines(key, representation, language, None)?);
        }
        Ok(output)
    }
}

impl FormatOutputLines for &Song {
    fn format_output_lines(
        &self,
        key: Option<&SimpleChord>,
        representation: Option<&ChordRepresentation>,
        language: Option<usize>,
        flow: Option<&[SongFlowItem]>,
    ) -> Result<Vec<OutputLine>, Error> {
        let self_key = self.key.clone().unwrap_or_default();
        let key = key.unwrap_or(&self_key);
        let (sections, custom_flow) = self.sections_for_flow(flow)?;
        let mut output = Vec::new();

        for section in sections {
            output.extend((&section).format_output_lines(
                Some(key),
                representation,
                language,
                None,
            )?);
            if custom_flow && let Some(repeat_text) = repeat_label(section.repeat_count) {
                output.push(OutputLine::Text(repeat_text));
            }
        }

        Ok(output)
    }
}

pub(crate) fn repeat_label(repeat_count: u32) -> Option<String> {
    match repeat_count {
        0 | 1 => None,
        2 => Some("(repeat)".to_string()),
        repeats => Some(format!("(repeat {}x)", repeats)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inputs::chord_pro::load_string;
    use crate::types::ChordRepresentation;
    use crate::types::SongFlowItem;

    fn flow_item(title: &str, repeats: u32) -> SongFlowItem {
        SongFlowItem {
            title: title.to_string(),
            repeats,
        }
    }

    #[test]
    fn format_output_lines_partial_translation_falls_back_to_primary_language() {
        let input = r#"{title: Test}
{key: C}
{language: de}
{language2: en}
{section: Verse}
[C]Nur Deutsch
&[C]Only English
[C]Auch nur Deutsch
"#;
        let song = load_string(input).expect("parse");
        let rep = ChordRepresentation::Default;

        let lines_lang0 = (&song.sections[0])
            .format_output_lines(None, Some(&rep), None, None)
            .expect("render");
        let lines_lang1 = (&song.sections[0])
            .format_output_lines(None, Some(&rep), Some(1), None)
            .expect("render");

        let text_lang0: Vec<&str> = lines_lang0
            .iter()
            .filter_map(|l| match l {
                OutputLine::Text(t) => Some(t.as_str()),
                _ => None,
            })
            .collect();
        let text_lang1: Vec<&str> = lines_lang1
            .iter()
            .filter_map(|l| match l {
                OutputLine::Text(t) => Some(t.as_str()),
                _ => None,
            })
            .collect();

        assert_eq!(text_lang0, vec!["Nur Deutsch", "Auch nur Deutsch"]);
        assert_eq!(text_lang1, vec!["Only English", "Auch nur Deutsch"]);
    }

    #[test]
    fn format_output_lines_supports_custom_flow_and_empty_flow_matches_default() {
        let input = r#"{title: Ohne Titel}
{key: A}
{section: Tag 1}
Text 1
{section: Tag 2}
Text 2
{section: Tag 3}
Text 3
{section: Tag 1}
{section: Tag 2}
{section: Tag 3}
"#;
        let song = load_string(input).expect("parse");
        let rep = ChordRepresentation::Default;
        let flow = vec![
            flow_item("Tag 1", 1),
            flow_item("Tag 2", 1),
            flow_item("Tag 3", 1),
            flow_item("Tag 1", 2),
            flow_item("Tag 3", 1),
            flow_item("Tag 2", 1),
        ];

        let default_lines = (&song)
            .format_output_lines(None, Some(&rep), None, None)
            .expect("render");
        let empty_flow_lines = (&song)
            .format_output_lines(None, Some(&rep), None, Some(&[]))
            .expect("render");
        assert_eq!(default_lines.len(), empty_flow_lines.len());
        assert_eq!(
            default_lines
                .iter()
                .map(|line| match line {
                    OutputLine::Keyword(s) | OutputLine::Chord(s) | OutputLine::Text(s) => s,
                })
                .collect::<Vec<&String>>(),
            empty_flow_lines
                .iter()
                .map(|line| match line {
                    OutputLine::Keyword(s) | OutputLine::Chord(s) | OutputLine::Text(s) => s,
                })
                .collect::<Vec<&String>>()
        );

        let custom_lines = (&song)
            .format_output_lines(None, Some(&rep), None, Some(&flow))
            .expect("render");
        let text = custom_lines
            .iter()
            .map(|line| match line {
                OutputLine::Keyword(s) => format!("K:{s}"),
                OutputLine::Chord(s) => format!("C:{s}"),
                OutputLine::Text(s) => format!("T:{s}"),
            })
            .collect::<Vec<String>>();

        assert_eq!(
            text,
            vec![
                "K:Tag 1",
                "T:Text 1",
                "K:Tag 2",
                "T:Text 2",
                "K:Tag 3",
                "T:Text 3",
                "K:Tag 1",
                "T:(repeat)",
                "K:Tag 3",
                "K:Tag 2",
            ]
        );
    }
}
