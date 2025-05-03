use super::{FormatOutputLines, OutputLine};
use crate::types::{ChordRepresentation, SimpleChord, Song};

pub trait FormatRender {
    fn format_render(
        &self,
        key: Option<&SimpleChord>,
        representation: Option<&ChordRepresentation>,
        language: Option<usize>,
    ) -> String;
}

impl FormatRender for Song {
    fn format_render(
        &self,
        key: Option<&SimpleChord>,
        representation: Option<&ChordRepresentation>,
        language: Option<usize>,
    ) -> String {
        self.format_output_lines(key, representation, language)
            .iter()
            .map(|line| match line {
                OutputLine::Keyword(keyword) => format!("\x1b[31;1m{}\x1b[0m", keyword),
                OutputLine::Chord(chord) => format!("\x1b[32;1m{}\x1b[0m", chord),
                OutputLine::Text(text) => format!("\x1b[32m{}\x1b[0m", text),
            })
            .collect::<Vec<String>>()
            .join("\n")
    }
}
