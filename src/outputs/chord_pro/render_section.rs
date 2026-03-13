use super::render_line::render_line;
use super::FormatChordPro;
use crate::types::{ChordRepresentation, Section, SimpleChord};

pub fn render_section(
    section: &Section,
    key: Option<&SimpleChord>,
    representation: Option<&ChordRepresentation>,
    language: Option<usize>,
    worship_pro_features: bool,
    bar_duration: u32,
) -> String {
    let keyword = if worship_pro_features {
        format!("{{section: {}}}", section.title)
    } else {
        format!("\n{}:", section.title)
    };

    let line_outputs: Vec<String> = section
        .lines
        .iter()
        .map(|line| {
            render_line(
                line,
                key,
                representation,
                language,
                worship_pro_features,
                bar_duration,
            )
        })
        .collect();

    let repeat_line = if section.repeat_count > 1 {
        Some(if worship_pro_features {
            if section.repeat_count == 2 {
                "{repeat}".to_string()
            } else {
                format!("{{repeat: {}}}", section.repeat_count)
            }
        } else {
            if section.repeat_count == 2 {
                "{comment: (repeat)}".to_string()
            } else {
                format!("{{comment: (repeat {}x)}}", section.repeat_count)
            }
        })
    } else {
        None
    };

    std::iter::once(keyword)
        .chain(line_outputs.into_iter())
        .chain(repeat_line.into_iter())
        .collect::<Vec<String>>()
        .join("\n")
}

impl FormatChordPro for &Section {
    fn format_chord_pro(
        &self,
        key: Option<&SimpleChord>,
        representation: Option<&ChordRepresentation>,
        language: Option<usize>,
        worship_pro_features: bool,
    ) -> String {
        render_section(
            self,
            key,
            representation,
            language,
            worship_pro_features,
            4000, // default 4/4 bar in milliclicks
        )
    }
}
