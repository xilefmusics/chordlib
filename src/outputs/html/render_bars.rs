use crate::types::{ChordRepresentation, Line, SimpleChord, chord_duration_to_layout_milliclicks};

#[derive(Debug)]
enum BarSym {
    Chord(String),
    Slash,
    Dot,
}

fn build_bar_symbols(bar: &[(String, u32)], bar_duration: u32, beats_per_bar: u32) -> Vec<BarSym> {
    let beats_per_bar = beats_per_bar as usize;
    let beat_duration = bar_duration / beats_per_bar as u32;

    let mut syms = Vec::with_capacity(beats_per_bar);

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

        match (chord_idx, prev_chord_idx) {
            (None, _) => syms.push(BarSym::Dot),

            (Some(idx), Some(prev)) if idx == prev => {
                let remaining = seg_end.saturating_sub(pos);
                if remaining >= beat_duration {
                    syms.push(BarSym::Slash);
                } else {
                    syms.push(BarSym::Dot);
                }
            }

            (Some(idx), _) => {
                prev_chord_idx = Some(idx);
                syms.push(BarSym::Chord(bar[idx].0.clone()));
            }
        }
    }

    syms
}

fn format_bar_symbols_default(syms: &[BarSym]) -> String {
    let mut out = String::with_capacity(syms.len() * 4);
    for (i, s) in syms.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        match s {
            BarSym::Chord(name) => out.push_str(name),
            BarSym::Slash => out.push('/'),
            BarSym::Dot => out.push('·'),
        }
    }
    out
}

fn flush_six_eight_slash_runs(out: &mut String, run: &mut u32) {
    while *run >= 3 {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push('/');
        *run -= 3;
    }
    while *run > 0 {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push('·');
        *run -= 1;
    }
}

/// Per-eighth grid fallback: merge continuation `/` in runs of up to three eighths, remainder `·`.
fn six_eight_merge_slash_runs_in_symbols(syms: &[BarSym]) -> String {
    let mut out = String::with_capacity(syms.len() * 2);
    let mut slash_run: u32 = 0;

    for s in syms {
        match s {
            BarSym::Chord(name) => {
                flush_six_eight_slash_runs(&mut out, &mut slash_run);
                if !out.is_empty() {
                    out.push(' ');
                }
                out.push_str(name);
            }
            BarSym::Slash => slash_run += 1,
            BarSym::Dot => {
                flush_six_eight_slash_runs(&mut out, &mut slash_run);
                if !out.is_empty() {
                    out.push(' ');
                }
                out.push('·');
            }
        }
    }
    flush_six_eight_slash_runs(&mut out, &mut slash_run);
    out
}

fn fill_six_eighth_chord_grid(bar: &[(String, u32)], bar_duration: u32) -> [Option<String>; 6] {
    let eighth_dur = bar_duration / 6;
    let mut grid: [Option<String>; 6] = std::array::from_fn(|_| None);

    let mut seg_idx = 0usize;
    let mut seg_end = bar.first().map(|(_, dur)| *dur).unwrap_or(0);

    for (eighth, cell) in grid.iter_mut().enumerate() {
        let pos = eighth as u32 * eighth_dur;
        while seg_idx < bar.len() && pos >= seg_end {
            seg_idx += 1;
            if let Some((_, dur)) = bar.get(seg_idx) {
                seg_end += dur;
            }
        }
        if seg_idx < bar.len() {
            *cell = Some(bar[seg_idx].0.clone());
        }
    }

    grid
}

fn first_three_same_chord(grid: &[Option<String>; 6]) -> Option<String> {
    let a = grid[0].clone()?;
    if grid[1].as_ref() == Some(&a) && grid[2].as_ref() == Some(&a) {
        Some(a)
    } else {
        None
    }
}

fn count_leading_matching(grid: &[Option<String>; 6], from: usize, name: &str) -> usize {
    grid[from..6]
        .iter()
        .take_while(|c| c.as_deref() == Some(name))
        .count()
}

fn two_eighth_pairs_all_uniform(grid: &[Option<String>; 6]) -> bool {
    for pair in 0..3 {
        let i = pair * 2;
        match (&grid[i], &grid[i + 1]) {
            (Some(a), Some(b)) if a == b => {}
            _ => return false,
        }
    }
    true
}

fn grid_suffix_as_bar(
    grid: &[Option<String>; 6],
    start: usize,
    eighth_dur: u32,
) -> Vec<(String, u32)> {
    let mut out = Vec::new();
    let mut i = start;
    while i < 6 {
        let name = match grid[i].clone() {
            Some(n) => n,
            None => {
                i += 1;
                continue;
            }
        };
        let mut eighths: u32 = 0;
        while i < 6 && grid[i].as_ref() == Some(&name) {
            eighths += 1;
            i += 1;
        }
        out.push((name, eighths * eighth_dur));
    }
    out
}

fn push_remainder_tokens(
    tokens: &mut Vec<String>,
    grid: &[Option<String>; 6],
    start: usize,
    eighth_dur: u32,
) {
    if start >= 6 {
        return;
    }

    let slice_len = 6 - start;
    let uniform = grid[start].is_some() && (start..6).all(|j| grid[j] == grid[start]);
    if uniform {
        tokens.push(grid[start].clone().unwrap());
        return;
    }

    let sub_bar = grid_suffix_as_bar(grid, start, eighth_dur);
    let sub_bar_dur = slice_len as u32 * eighth_dur;
    let sub_syms = build_bar_symbols(&sub_bar, sub_bar_dur, slice_len as u32);
    let s = six_eight_merge_slash_runs_in_symbols(&sub_syms);
    for t in s.split_whitespace() {
        tokens.push(t.to_string());
    }
}

/// 6/8 chord-only bars: prefer two compound beats (three eighths each) when the first beat is one
/// chord (a whole-bar single harmony is formatted as one chord token before this layer); continuation in the second dotted-quarter uses `/` for one
/// or three eighths of the same chord, and `·` for each eighth when exactly two eighths remain
/// before the next chord (e.g. `C · · G`). Else three two-eighth cells when each pair matches
/// (`C D E`); else per-eighth layout with merged continuation slashes.
fn format_six_eight_compact(bar: &[(String, u32)], bar_duration: u32) -> String {
    let eighth_dur = bar_duration / 6;

    let grid = fill_six_eighth_chord_grid(bar, bar_duration);

    if let Some(first_name) = first_three_same_chord(&grid) {
        let mut tokens = vec![first_name.clone()];
        let k = count_leading_matching(&grid, 3, first_name.as_str());
        if k > 0 {
            if k == 2 {
                tokens.push("·".to_string());
                tokens.push("·".to_string());
            } else {
                tokens.push("/".to_string());
            }
        }
        let next = 3 + k;
        push_remainder_tokens(&mut tokens, &grid, next, eighth_dur);
        return tokens.join(" ");
    }

    if two_eighth_pairs_all_uniform(&grid) {
        return format!(
            "{} {} {}",
            grid[0].as_ref().unwrap(),
            grid[2].as_ref().unwrap(),
            grid[4].as_ref().unwrap()
        );
    }

    let syms = build_bar_symbols(bar, bar_duration, 6);
    six_eight_merge_slash_runs_in_symbols(&syms)
}

fn format_bar_beat_slashes(
    bar: &[(String, u32)],
    bar_duration: u32,
    beats_per_bar: u32,
    compact_six_eight: bool,
) -> String {
    if beats_per_bar == 0 {
        return String::new();
    }

    // One harmonic segment for the full bar: omit per-beat `/` and `·` fillers (#38).
    if bar.len() == 1 && bar[0].1 == bar_duration {
        return bar[0].0.clone();
    }

    if compact_six_eight && beats_per_bar == 6 {
        format_six_eight_compact(bar, bar_duration)
    } else {
        let syms = build_bar_symbols(bar, bar_duration, beats_per_bar);
        format_bar_symbols_default(&syms)
    }
}

pub fn render_bars(
    lines: &[&Line],
    key: &SimpleChord,
    representation: &ChordRepresentation,
    bar_duration: u32,
    beats_per_bar: u32,
    compact_six_eight: bool,
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
            let formatted_bar =
                format_bar_beat_slashes(&bar, bar_duration, beats_per_bar, compact_six_eight);

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

    fn chord_tokens(html: &str) -> Vec<&str> {
        let chord_span_start = html
            .find("<span class=\"chord\">")
            .expect("chord span start not found");
        let chord_span_start = chord_span_start + "<span class=\"chord\">".len();
        let chord_span_end = html[chord_span_start..]
            .find("</span>")
            .expect("chord span end not found")
            + chord_span_start;
        let chord_text = &html[chord_span_start..chord_span_end];
        chord_text.split_whitespace().collect()
    }

    #[test]
    fn bars_show_chords_starting_on_fractional_beats() {
        let bar_duration = 4000;
        let beats_per_bar = 4;

        let line = line_with_chords(&["C:1.5", "G:2.5"]);

        let key: SimpleChord = "C".try_into().unwrap();
        let rep = ChordRepresentation::Default;

        let html = render_bars(&[&line], &key, &rep, bar_duration, beats_per_bar, false);

        let tokens = chord_tokens(&html);
        assert_eq!(tokens.len(), beats_per_bar as usize, "{tokens:?}");

        let unique_chords: HashSet<&str> = tokens
            .iter()
            .copied()
            .filter(|t| *t != "·" && *t != "/")
            .collect();
        assert_eq!(unique_chords.len(), 2, "{tokens:?}");
    }

    #[test]
    fn bars_six_eighth_full_bar_chord_single_token_without_compact() {
        let bar_duration = 3000;
        let beats_per_bar = 6;
        let line = line_with_chords(&["C:6"]);
        let key = SimpleChord::default();
        let rep = ChordRepresentation::Default;
        let html = render_bars(&[&line], &key, &rep, bar_duration, beats_per_bar, false);

        let tokens = chord_tokens(&html);
        assert_eq!(tokens, vec!["C"], "{tokens:?}");
    }

    #[test]
    fn bars_six_eighth_compact_full_bar_chord_single_token() {
        let bar_duration = 3000;
        let beats_per_bar = 6;
        let line = line_with_chords(&["C:6"]);
        let key = SimpleChord::default();
        let rep = ChordRepresentation::Default;
        let html = render_bars(&[&line], &key, &rep, bar_duration, beats_per_bar, true);

        let tokens = chord_tokens(&html);
        assert_eq!(tokens, vec!["C"], "{tokens:?}");
    }

    #[test]
    fn bars_common_time_full_bar_chord_single_token() {
        let bar_duration = 4000;
        let beats_per_bar = 4;
        let line = line_with_chords(&["C:4"]);
        let key = SimpleChord::default();
        let rep = ChordRepresentation::Default;
        let html = render_bars(&[&line], &key, &rep, bar_duration, beats_per_bar, false);

        let tokens = chord_tokens(&html);
        assert_eq!(tokens, vec!["C"], "{tokens:?}");
    }

    #[test]
    fn bars_common_time_two_half_bar_chords_keeps_beat_fillers() {
        let bar_duration = 4000;
        let beats_per_bar = 4;
        let line = line_with_chords(&["C:2", "G:2"]);
        let key = SimpleChord::default();
        let rep = ChordRepresentation::Default;
        let html = render_bars(&[&line], &key, &rep, bar_duration, beats_per_bar, false);

        let tokens = chord_tokens(&html);
        assert!(
            tokens.iter().any(|t| *t == "/" || *t == "·"),
            "expected continuation markers between changes: {tokens:?}"
        );
        assert!(
            tokens.iter().filter(|t| **t == "C" || **t == "G").count() >= 2,
            "{tokens:?}"
        );
    }

    #[test]
    fn bars_six_eighth_alternating_one_beat_chords_show_both() {
        let bar_duration = 3000;
        let beats_per_bar = 6;
        let line = line_with_chords(&["C:1", "G:1", "C:1", "G:1", "C:1", "G:1"]);
        let key = SimpleChord::default();
        let rep = ChordRepresentation::Default;
        let html = render_bars(&[&line], &key, &rep, bar_duration, beats_per_bar, true);

        let tokens = chord_tokens(&html);
        assert_eq!(tokens.len(), 6, "{tokens:?}");

        let unique_chords: HashSet<&str> = tokens
            .iter()
            .copied()
            .filter(|t| *t != "·" && *t != "/")
            .collect();
        assert_eq!(unique_chords.len(), 2, "{tokens:?}");
    }

    #[test]
    fn bars_six_eighth_compact_two_compound_beats_different_chords() {
        let bar_duration = 3000;
        let beats_per_bar = 6;
        let line = line_with_chords(&["C:3", "G:3"]);
        let key = SimpleChord::default();
        let rep = ChordRepresentation::Default;
        let html = render_bars(&[&line], &key, &rep, bar_duration, beats_per_bar, true);

        let tokens = chord_tokens(&html);
        assert_eq!(tokens, vec!["C", "G"], "{tokens:?}");
    }

    #[test]
    fn bars_six_eighth_compact_four_and_two_eighths_is_chord_slash_chord() {
        let bar_duration = 3000;
        let beats_per_bar = 6;
        let line = line_with_chords(&["C:4", "G:2"]);
        let key = SimpleChord::default();
        let rep = ChordRepresentation::Default;
        let html = render_bars(&[&line], &key, &rep, bar_duration, beats_per_bar, true);

        let tokens = chord_tokens(&html);
        assert_eq!(tokens, vec!["C", "/", "G"], "{tokens:?}");
    }

    #[test]
    fn bars_six_eighth_compact_three_two_eighth_cells() {
        let bar_duration = 3000;
        let beats_per_bar = 6;
        let line = line_with_chords(&["C:2", "D:2", "E:2"]);
        let key = SimpleChord::default();
        let rep = ChordRepresentation::Default;
        let html = render_bars(&[&line], &key, &rep, bar_duration, beats_per_bar, true);

        let tokens = chord_tokens(&html);
        assert_eq!(tokens, vec!["C", "D", "E"], "{tokens:?}");
    }

    #[test]
    fn bars_six_eighth_compact_five_and_one_eighth_continuation() {
        let bar_duration = 3000;
        let beats_per_bar = 6;
        let line = line_with_chords(&["C:5", "G:1"]);
        let key = SimpleChord::default();
        let rep = ChordRepresentation::Default;
        let html = render_bars(&[&line], &key, &rep, bar_duration, beats_per_bar, true);

        let tokens = chord_tokens(&html);
        assert_eq!(tokens, vec!["C", "·", "·", "G"], "{tokens:?}");
    }

    #[test]
    fn bars_do_not_drop_late_fractional_chord() {
        let bar_duration = 4000;
        let beats_per_bar = 4;

        let line = line_with_chords(&["D:1.5", "E:2.5"]);

        let key: SimpleChord = "D".try_into().unwrap();
        let rep = ChordRepresentation::Default;

        let html = render_bars(&[&line], &key, &rep, bar_duration, beats_per_bar, false);

        let tokens = chord_tokens(&html);
        let unique_chords: HashSet<&str> = tokens
            .iter()
            .copied()
            .filter(|t| *t != "·" && *t != "/")
            .collect();
        assert!(unique_chords.len() >= 2, "{tokens:?}");
    }
}
