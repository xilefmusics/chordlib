use crate::types::{Line, Section, SimpleChord, Song};

pub enum OutputLine {
    Keyword(String),
    Chord(String),
    Text(String),
}

pub trait FormatOutputLines {
    fn format_output_lines(
        &self,
        key: Option<SimpleChord>,
        language: Option<usize>,
    ) -> Vec<OutputLine>;
}

impl FormatOutputLines for &Line {
    fn format_output_lines(
        &self,
        key: Option<SimpleChord>,
        language: Option<usize>,
    ) -> Vec<OutputLine> {
        let mut chord_line = String::default();
        let mut text_line = String::default();
        let language = language.unwrap_or(0);

        for part in &self.parts {
            if let Some(chord) = part.chord.clone() {
                let chord_chars = chord_line.chars().count();
                let text_chars = text_line.chars().count();
                if text_chars > chord_chars {
                    for _ in 0..(text_chars - chord_chars) {
                        chord_line.push_str(" ");
                    }
                } else if chord_chars > 0 {
                    chord_line.push_str(" ");
                }
                chord_line = format!(
                    "{}{}",
                    chord_line,
                    chord.format(key.clone().unwrap_or(SimpleChord::default()))
                );
            }
            text_line = format!("{}{}", text_line, part.languages[language]);
        }

        let mut result = Vec::default();
        if chord_line.len() > 0 {
            result.push(OutputLine::Chord(chord_line));
        }
        if text_line.len() > 0 {
            result.push(OutputLine::Text(text_line));
        }
        result
    }
}

impl FormatOutputLines for &Section {
    fn format_output_lines(
        &self,
        key: Option<SimpleChord>,
        language: Option<usize>,
    ) -> Vec<OutputLine> {
        std::iter::once(OutputLine::Keyword(self.title.clone()))
            .chain(
                self.lines
                    .iter()
                    .flat_map(|line| line.format_output_lines(key.clone(), language)),
            )
            .collect()
    }
}

impl FormatOutputLines for &Song {
    fn format_output_lines(
        &self,
        key: Option<SimpleChord>,
        language: Option<usize>,
    ) -> Vec<OutputLine> {
        let key = key.unwrap_or(self.key.clone().unwrap_or(SimpleChord::default()));
        self.sections
            .iter()
            .flat_map(|section| section.format_output_lines(Some(key.clone()), language))
            .collect()
    }
}
