use super::FormatChordPro;
use crate::types::{ChordRepresentation, Line, SimpleChord, chord_duration_to_layout_milliclicks};

pub fn render_line(
    line: &Line,
    key: Option<&SimpleChord>,
    representation: Option<&ChordRepresentation>,
    language: Option<usize>,
    worship_pro_features: bool,
    bar_duration: u32,
    beats_per_bar: u32,
) -> String {
    if worship_pro_features
        || line
            .parts
            .iter()
            .any(|part| part.languages.iter().any(|l| !l.is_empty()))
    {
        return line
            .parts
            .iter()
            .map(|part| part.format_chord_pro(key, representation, language, worship_pro_features))
            .collect();
    }

    let mut result = String::with_capacity(128);
    let mut chord = String::new();
    let mut chord_duration = 0;
    let mut current_bar_duration = 0i32;

    for part in &line.parts {
        if current_bar_duration <= 0 {
            result.push_str("[|]");
            if current_bar_duration < 0 {
                append_chord_block(&mut result, &chord, chord_duration, bar_duration);
            }
            current_bar_duration += bar_duration as i32;
        }

        chord = part
            .chord
            .as_ref()
            .map(|c| {
                c.format(
                    key.unwrap_or(&SimpleChord::default()),
                    representation.unwrap_or(&ChordRepresentation::default()),
                )
            })
            .unwrap_or_default();

        chord_duration = part
            .chord
            .as_ref()
            .map(|c| {
                chord_duration_to_layout_milliclicks(c.get_duration(), bar_duration, beats_per_bar)
            })
            .unwrap_or(bar_duration);

        append_chord_block(&mut result, &chord, chord_duration, bar_duration);
        current_bar_duration -= chord_duration as i32;
    }

    if current_bar_duration < 0 {
        append_chord_block(&mut result, &chord, chord_duration, bar_duration);
    }

    result.push_str("[|]");
    result
}

fn append_chord_block(result: &mut String, chord: &str, chord_duration: u32, bar_duration: u32) {
    result.push('[');
    result.push_str(chord);
    result.push(']');

    let base_width = 16 / (bar_duration / chord_duration).max(1) as usize;
    let spacing = base_width.saturating_sub(chord.chars().count() + 1);
    result.extend(std::iter::repeat_n(' ', spacing));
}

impl FormatChordPro for &Line {
    fn format_chord_pro(
        &self,
        key: Option<&SimpleChord>,
        representation: Option<&ChordRepresentation>,
        language: Option<usize>,
        worship_pro_features: bool,
    ) -> String {
        render_line(
            self,
            key,
            representation,
            language,
            worship_pro_features,
            4000,
            4,
        )
    }
}
