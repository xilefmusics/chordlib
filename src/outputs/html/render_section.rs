use super::{render_bars, render_line};
use crate::types::{ChordRepresentation, Line, Section, SimpleChord};

fn is_chord_only_line(line: &Line) -> bool {
    line.parts
        .iter()
        .all(|part| part.languages.iter().all(|lang| lang.is_empty()))
}

pub fn render_section(
    section: &Section,
    key: &SimpleChord,
    representation: &ChordRepresentation,
    language: usize,
    bar_duration: u32,
) -> String {
    let mut content = String::new();
    let mut chord_only_line_buffer = Vec::new();

    for line in &section.lines {
        if is_chord_only_line(line) {
            chord_only_line_buffer.push(line);
        } else {
            if !chord_only_line_buffer.is_empty() {
                content.push_str(&render_bars::render_bars(
                    &chord_only_line_buffer,
                    key,
                    representation,
                    bar_duration,
                ));
                chord_only_line_buffer.clear();
            }
            content.push_str(&render_line::render_line(
                line,
                key,
                representation,
                language,
            ));
        }
    }

    if !chord_only_line_buffer.is_empty() {
        content.push_str(&render_bars::render_bars(
            &chord_only_line_buffer,
            key,
            representation,
            bar_duration,
        ));
    }

    let title = section.title.to_uppercase();

    format!(
        "<p><span class=\"keyword\">{}</span><br>{}</p>",
        title, content
    )
}
