use super::{render_bars, render_line};
use crate::types::{ChordRepresentation, Line, Part, Section, SimpleChord};

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
    beats_per_bar: u32,
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
                    beats_per_bar,
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
            beats_per_bar,
        ));
    }

    // Add a separate repeat line at the end of the section if needed.
    if section.repeat_count > 1 {
        let repeat_text = if section.repeat_count == 2 {
            "(repeat)".to_string()
        } else {
            format!("(repeat {}x)", section.repeat_count)
        };

        // Ensure the languages vector has an entry for the requested language index.
        let mut languages = vec![String::new(); language.saturating_add(1)];
        languages[language] = repeat_text;

        let repeat_line = Line {
            parts: vec![Part {
                chord: None,
                languages,
                comment: true,
            }],
        };

        content.push_str(&render_line::render_line(
            &repeat_line,
            key,
            representation,
            language,
        ));
    }

    format!(
        "<p><span class=\"keyword\">{}</span><br>{}</p>",
        section.title, content
    )
}

#[cfg(test)]
mod tests {
    use crate::outputs::FormatHTML;
    use crate::types::{ChordRepresentation, Line, Part, Section, SimpleChord, Song};

    fn make_song_with_section(title: &str, repeat_count: u32) -> Song {
        let section = Section {
            title: title.to_string(),
            lines: vec![Line {
                parts: vec![Part {
                    chord: None,
                    languages: vec!["Line".to_string()],
                    comment: false,
                }],
            }],
            repeat_count,
        };

        Song {
            title: "Test".to_string(),
            titles: None,
            subtitle: None,
            copyright: None,
            key: Some(SimpleChord::default()),
            artist: None,
            artists: None,
            language: None,
            languages: None,
            tempo: None,
            time: None,
            sections: vec![section],
        }
    }

    #[test]
    fn html_does_not_show_repeat_marker_for_default() {
        let song = make_song_with_section("Chorus", 1);
        let html = (&song).format_html(None, Some(&ChordRepresentation::Default), None, None);

        assert!(html.contains("<span class=\"keyword\">Chorus</span>"));
        assert!(!html.contains("(repeat"));
    }

    #[test]
    fn html_shows_repeat_marker_for_section_with_repeat_count() {
        let song = make_song_with_section("Chorus", 2);
        let html = (&song).format_html(None, Some(&ChordRepresentation::Default), None, None);

        assert!(html.contains("<span class=\"comment\">(repeat)</span>"));
    }
}
