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

    std::iter::once(keyword)
        .chain(section.lines.iter().map(|line| {
            render_line(
                line,
                key,
                representation,
                language,
                worship_pro_features,
                bar_duration,
            )
        }))
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
            96,
        )
    }
}
