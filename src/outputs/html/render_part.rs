use crate::types::{Key, Part};

pub fn render_part(part: &Part, key: &Key, language: usize) -> (String, usize, usize, bool, bool) {
    let mut result = String::with_capacity(128);
    let mut text_chars = 0;
    let mut chord_chars = 0;
    let mut first_whitespace = true;
    let mut last_whitespace = true;

    result.push_str("<span class=\"part\">");

    if let Some(chord) = &part.chord {
        let formatted_chord = chord.format(key);
        chord_chars = formatted_chord.chars().count();
        result.push_str("<span class=\"chord\">");
        result.push_str(&formatted_chord);
        result.push_str("</span>");
    }

    if let Some(text) = part.languages.get(language).filter(|t| !t.is_empty()) {
        text_chars = text.chars().count();
        first_whitespace = text.chars().next().map_or(true, |c| c.is_whitespace());
        last_whitespace = text.chars().last().map_or(true, |c| c.is_whitespace());

        result.push_str("<span class=\"");
        result.push_str(if part.comment { "comment" } else { "text" });
        result.push_str("\">");
        result.push_str(text);
        result.push_str("</span>");
    }

    result.push_str("</span>");

    (
        result,
        text_chars,
        chord_chars,
        first_whitespace,
        last_whitespace,
    )
}
