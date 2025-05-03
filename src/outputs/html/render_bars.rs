use crate::types::{ChordRepresentation, Line, SimpleChord};

pub fn render_bars(
    lines: &[&Line],
    key: &SimpleChord,
    representation: &ChordRepresentation,
    bar_duration: u32,
) -> String {
    let mut columns: Vec<String> = Vec::new();
    for line in lines {
        let mut bars = vec![];
        let mut current_bar = vec![];
        let mut duration_buffer = 0;

        for (chord_str, duration) in line.parts.iter().filter_map(|part| {
            part.chord.as_ref().map(|chord| {
                (
                    chord.format(key, representation),
                    chord.get_duration().unwrap_or(bar_duration),
                )
            })
        }) {
            let mut remaining_duration = duration;

            while remaining_duration > 0 {
                let chord_duration = remaining_duration.min(bar_duration - duration_buffer);

                current_bar.push((chord_str.clone(), chord_duration));
                duration_buffer += chord_duration;
                remaining_duration -= chord_duration;

                if duration_buffer == bar_duration {
                    bars.push(std::mem::take(&mut current_bar));
                    duration_buffer = 0;
                }
            }
        }
        if !current_bar.is_empty() {
            bars.push(current_bar);
        }

        for (idx, bar) in bars.into_iter().enumerate() {
            let formatted_bar = bar
                .into_iter()
                .map(|(chord, _)| chord)
                .collect::<Vec<_>>()
                .join(" ");

            match columns.get_mut(idx) {
                Some(column) => {
                    column.push_str("<br>");
                    column.push_str(&formatted_bar);
                }
                None => columns.push(formatted_bar),
            }
        }
    }

    let separators = std::iter::repeat("<br>|")
        .take(lines.len() - 1)
        .collect::<String>();
    let separators = format!("|{}", separators);

    let mut result = String::with_capacity(128);
    result.push_str("<span class=\"bars\">");
    for column in &columns {
        result.extend([
            "<span class=\"bar\">",
            &separators,
            "</span><span class=\"chord\">",
            column,
            "</span>",
        ]);
    }
    result.extend(["<span class=\"bar\">", &separators, "</span></span>"]);

    result
}
