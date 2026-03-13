use crate::types::{ChordRepresentation, Line, SimpleChord};

/// One symbol per beat:
/// - chord name when a chord starts on that beat,
/// - "/" when continuing the same chord for a full beat,
/// - "·" when continuing the same chord for a shorter (fractional) beat.
fn format_bar_beat_slashes(bar: &[(String, u32)], bar_duration: u32, beats_per_bar: u32) -> String {
    if beats_per_bar == 0 {
        return String::new();
    }
    let mut symbols = Vec::with_capacity(beats_per_bar as usize);
    let beat_duration = if beats_per_bar > 0 {
        bar_duration / beats_per_bar
    } else {
        0
    };
    let mut prev_chord_idx: Option<usize> = None;
    for beat in 0..beats_per_bar {
        let pos = beat * bar_duration / beats_per_bar;
        let mut seg_start = 0u32;
        let mut chord_idx: Option<usize> = None;
        let mut seg_end = 0u32;

        for (idx, (_chord_str, dur)) in bar.iter().enumerate() {
            seg_end = seg_start + *dur;
            if pos < seg_end {
                chord_idx = Some(idx);
                break;
            }
            seg_start = seg_end;
        }

        let symbol = match (chord_idx, prev_chord_idx) {
            // No chord covers this beat position.
            (None, _) => "·".to_string(),
            // Same chord as on the previous beat → continuation:
            // "/" if at least a full beat of this chord remains, otherwise "·".
            (Some(idx), Some(prev)) if idx == prev => {
                let remaining = seg_end.saturating_sub(pos);
                if beat_duration > 0 && remaining >= beat_duration {
                    "/".to_string()
                } else {
                    "·".to_string()
                }
            }
            // First sampled beat within this chord's span → show the chord name.
            (Some(idx), _) => {
                prev_chord_idx = Some(idx);
                bar[idx].0.clone()
            }
        };

        symbols.push(symbol);
    }
    symbols.join(" ")
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

    /// Chords whose durations are fractional beats (e.g. 1.5 + 2.5 beats)
    /// must still show *both* chord names somewhere in the bar grid.
    #[test]
    fn bars_show_chords_starting_on_fractional_beats() {
        // 4/4 time → 4 beats, 4000 milliclicks per bar.
        let bar_duration = 4000;
        let beats_per_bar = 4;

        // First chord: C for 1.5 beats (1500 milliclicks),
        // second chord: G for 2.5 beats (2500 milliclicks).
        // Together they fill exactly one bar.
        let line = line_with_chords(&["C:1.5", "G:2.5"]);

        // Use a concrete key / representation, matching other tests in this crate.
        let key: SimpleChord = "C".try_into().unwrap();
        let rep = ChordRepresentation::Default;

        let html = render_bars(&[&line], &key, &rep, bar_duration, beats_per_bar);

        // Extract the rendered chord symbols for the (single) bar.
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
        assert_eq!(
            tokens.len(),
            beats_per_bar as usize,
            "expected one symbol per beat; got: {chord_text}"
        );

        let unique_chords: HashSet<&str> = tokens
            .iter()
            .copied()
            .filter(|t| *t != "·" && *t != "/")
            .collect();
        assert_eq!(
            unique_chords.len(),
            2,
            "fractional-beat chords must both appear somewhere in the bar grid; got: {chord_text}"
        );
    }

    /// Regression-style test with another pair of chords whose split point does
    /// not coincide with integer beat positions, to ensure later chords are not
    /// silently dropped.
    #[test]
    fn bars_do_not_drop_late_fractional_chord() {
        let bar_duration = 4000;
        let beats_per_bar = 4;

        // Two chords that together fill the bar, with the *second* starting on a
        // non-integer beat. Any reasonable rendering should show both "D" and "E".
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
        assert!(
            unique_chords.len() >= 2,
            "bar rendering must not drop later fractional chords; got: {chord_text}"
        );
    }
}
