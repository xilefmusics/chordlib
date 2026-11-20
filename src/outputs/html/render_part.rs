use crate::types::{ChordRepresentation, Part, SimpleChord};

pub fn render_part(
    part: &Part,
    key: &SimpleChord,
    representation: &ChordRepresentation,
    language: usize,
    starts_inside_word: bool,
    mut start_word_position: Option<usize>,
    end_word_position: Option<usize>,
) -> String {
    let mut result = String::with_capacity(128);

    let comment_class = if part.comment { "comment" } else { "text" };

    if let Some(chord) = &part.chord {
        if starts_inside_word {
            result.push_str("<span class=\"part part-has-chord part-starts-inside-word\">");
        } else {
            result.push_str("<span class=\"part part-has-chord\">");
        }
        let formatted_chord = chord.format(key, representation);
        result.push_str("<span class=\"chord\">");
        result.push_str(&formatted_chord);
        result.push_str("</span>");
    } else {
        result.push_str("<span class=\"part\">");
    }

    let text = part.text_for_language(language);
    if !text.is_empty() {
        let mut text = text;

        if let Some(end) = end_word_position {
            let (before_end, after_end) = text.split_at(end);
            text = after_end;
            start_word_position = start_word_position.map(|s| s.saturating_sub(end));

            result.push_str("<span class=\"");
            result.push_str(comment_class);
            result.push_str("\">");
            result.push_str(before_end);
            result.push_str("</span></span></span>");
            result.push_str("<span class=\"part\">");
            result.push_str("<span class=\"");
            result.push_str(comment_class);
            result.push_str("\">");
        } else {
            result.push_str("<span class=\"");
            result.push_str(comment_class);
            result.push_str("\">");
        }

        if let Some(start) = start_word_position {
            let after_whitespace = text[start..]
                .chars()
                .next()
                .map_or(start + 1, |c| start + c.len_utf8());
            let (before_start, after_start) = text.split_at(after_whitespace);
            text = after_start;

            result.push_str(before_start);
            result.push_str("</span></span>");
            result.push_str("<span class=\"word\"><span class=\"part\">");
            result.push_str("<span class=\"");
            result.push_str(comment_class);
            result.push_str("\">");
        }

        result.push_str(text);
        result.push_str("</span>");
    } else {
        result.push_str("<span class=\"text\"> </span>");
    }

    result.push_str("</span>");

    result
}
