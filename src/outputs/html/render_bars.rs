use crate::types::{ChordRepresentation, Line, SimpleChord, chord_duration_to_layout_milliclicks};

fn format_bar_beat_slashes(bar: &[(String, u32)], bar_duration: u32, beats_per_bar: u32) -> String {
    if beats_per_bar == 0 {
        return String::new();
    }

    let beats_per_bar = beats_per_bar as usize;
    let beat_duration = bar_duration / beats_per_bar as u32;

    let mut out = String::with_capacity(beats_per_bar * 4);

    let mut prev_chord_idx: Option<usize> = None;

    let mut seg_idx = 0usize;
    let mut seg_end = bar.first().map(|(_, dur)| *dur).unwrap_or(0);

    for beat in 0..beats_per_bar {
        let pos = beat as u32 * bar_duration / beats_per_bar as u32;

        while seg_idx < bar.len() && pos >= seg_end {
            seg_idx += 1;
            if let Some((_, dur)) = bar.get(seg_idx) {
                seg_end += *dur;
            }
        }

        let chord_idx = if seg_idx < bar.len() {
            Some(seg_idx)
        } else {
            None
        };

        if beat > 0 {
            out.push(' ');
        }

        match (chord_idx, prev_chord_idx) {
            (None, _) => out.push('·'),

            (Some(idx), Some(prev)) if idx == prev => {
                let remaining = seg_end.saturating_sub(pos);
                if remaining >= beat_duration {
                    out.push('/');
                } else {
                    out.push('·');
                }
            }

            (Some(idx), _) => {
                prev_chord_idx = Some(idx);
                out.push_str(&bar[idx].0);
            }
        }
    }

    out
}

pub fn render_bars(
    lines: &[&Line],
    key: &SimpleChord,
    representation: &ChordRepresentation,
    bar_duration: u32,
    beats_per_bar: u32,
) -> String {
    let mut columns: Vec<String> = Vec::new();
    let beats_per_bar = beats_per_bar.max(1);

    for line in lines {
        let mut bars = vec![];
        let mut current_bar: Vec<(String, u32)> = vec![];
        let mut duration_buffer = 0u32;

        for (chord_str, duration) in line.parts.iter().filter_map(|part| {
            part.chord.as_ref().map(|chord| {
                (
                    chord.format(key, representation).to_string(),
                    chord_duration_to_layout_milliclicks(
                        chord.get_duration(),
                        bar_duration,
                        beats_per_bar,
                    ),
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
            let formatted_bar = format_bar_beat_slashes(&bar, bar_duration, beats_per_bar);

            match columns.get_mut(idx) {
                Some(column) => {
                    column.push_str("<br>");
                    column.push_str(&formatted_bar);
                }
                None => columns.push(formatted_bar),
            }
        }
    }

    let repeat_count = lines.len().saturating_sub(1);
    let separators = if repeat_count == 0 {
        "|".to_string()
    } else {
        format!("|{}", "<br>|".repeat(repeat_count))
    };

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Chord, Part};
    use std::collections::HashSet;
    use std::str::FromStr;

    fn line_with_chords(specs: &[&str]) -> Line {
        let parts: Vec<Part> = specs
            .iter()
            .map(|s| Part::new_chord(Chord::from_str(s).unwrap()))
            .collect();
        Line { parts }
    }

    #[test]
    fn bars_show_chords_starting_on_fractional_beats() {
        let bar_duration = 4000;
        let beats_per_bar = 4;

        let line = line_with_chords(&["C:1.5", "G:2.5"]);

        let key: SimpleChord = "C".try_into().unwrap();
        let rep = ChordRepresentation::Default;

        let html = render_bars(&[&line], &key, &rep, bar_duration, beats_per_bar);

        let chord_span_start = html
            .find("<span class=\"chord\">")
            .expect("chord span start not found");
        let chord_span_start = chord_span_start + "<span class=\"chord\">".len();
        let chord_span_end = html[chord_span_start..]
            .find("</span>")
            .expect("chord span end not found")
            + chord_span_start;
        let chord_text = &html[chord_span_start..chord_span_end];

        let tokens: Vec<&str> = chord_text.split_whitespace().collect();
        assert_eq!(tokens.len(), beats_per_bar as usize, "{chord_text}");

        let unique_chords: HashSet<&str> = tokens
            .iter()
            .copied()
            .filter(|t| *t != "·" && *t != "/")
            .collect();
        assert_eq!(unique_chords.len(), 2, "{chord_text}");
    }

    #[test]
    fn bars_six_eighth_one_beat_chords_fill_bar() {
        let bar_duration = 3000;
        let beats_per_bar = 6;
        let line = line_with_chords(&["C:6"]);
        let key = SimpleChord::default();
        let rep = ChordRepresentation::Default;
        let html = render_bars(&[&line], &key, &rep, bar_duration, beats_per_bar);

        let chord_span_start = html
            .find("<span class=\"chord\">")
            .expect("chord span start not found");
        let chord_span_start = chord_span_start + "<span class=\"chord\">".len();
        let chord_span_end = html[chord_span_start..]
            .find("</span>")
            .expect("chord span end not found")
            + chord_span_start;
        let chord_text = &html[chord_span_start..chord_span_end];
        let tokens: Vec<&str> = chord_text.split_whitespace().collect();
        assert_eq!(tokens, vec!["C", "/", "/", "/", "/", "/"], "{chord_text}");
    }

    #[test]
    fn bars_common_time_one_beat_per_slot() {
        let bar_duration = 4000;
        let beats_per_bar = 4;
        let line = line_with_chords(&["C:4"]);
        let key = SimpleChord::default();
        let rep = ChordRepresentation::Default;
        let html = render_bars(&[&line], &key, &rep, bar_duration, beats_per_bar);

        let chord_span_start = html
            .find("<span class=\"chord\">")
            .expect("chord span start not found");
        let chord_span_start = chord_span_start + "<span class=\"chord\">".len();
        let chord_span_end = html[chord_span_start..]
            .find("</span>")
            .expect("chord span end not found")
            + chord_span_start;
        let chord_text = &html[chord_span_start..chord_span_end];
        let tokens: Vec<&str> = chord_text.split_whitespace().collect();
        assert_eq!(tokens, vec!["C", "/", "/", "/"], "{chord_text}");
    }

    #[test]
    fn bars_six_eighth_alternating_one_beat_chords_show_both() {
        let bar_duration = 3000;
        let beats_per_bar = 6;
        let line = line_with_chords(&["C:1", "G:1", "C:1", "G:1", "C:1", "G:1"]);
        let key = SimpleChord::default();
        let rep = ChordRepresentation::Default;
        let html = render_bars(&[&line], &key, &rep, bar_duration, beats_per_bar);

        let chord_span_start = html
            .find("<span class=\"chord\">")
            .expect("chord span start not found");
        let chord_span_start = chord_span_start + "<span class=\"chord\">".len();
        let chord_span_end = html[chord_span_start..]
            .find("</span>")
            .expect("chord span end not found")
            + chord_span_start;
        let chord_text = &html[chord_span_start..chord_span_end];

        let tokens: Vec<&str> = chord_text.split_whitespace().collect();
        assert_eq!(tokens.len(), 6, "{chord_text}");

        let unique_chords: HashSet<&str> = tokens
            .iter()
            .copied()
            .filter(|t| *t != "·" && *t != "/")
            .collect();
        assert_eq!(unique_chords.len(), 2, "{chord_text}");
    }

    #[test]
    fn bars_do_not_drop_late_fractional_chord() {
        let bar_duration = 4000;
        let beats_per_bar = 4;

        let line = line_with_chords(&["D:1.5", "E:2.5"]);

        let key: SimpleChord = "D".try_into().unwrap();
        let rep = ChordRepresentation::Default;

        let html = render_bars(&[&line], &key, &rep, bar_duration, beats_per_bar);

        let chord_span_start = html
            .find("<span class=\"chord\">")
            .expect("chord span start not found");
        let chord_span_start = chord_span_start + "<span class=\"chord\">".len();
        let chord_span_end = html[chord_span_start..]
            .find("</span>")
            .expect("chord span end not found")
            + chord_span_start;
        let chord_text = &html[chord_span_start..chord_span_end];

        let tokens: Vec<&str> = chord_text.split_whitespace().collect();
        let unique_chords: HashSet<&str> = tokens
            .iter()
            .copied()
            .filter(|t| *t != "·" && *t != "/")
            .collect();
        assert!(unique_chords.len() >= 2, "{chord_text}");
    }
}
