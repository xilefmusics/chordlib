use super::render_part;
use crate::types::{Key, Line};

pub fn render_line(line: &Line, key: &Key, language: usize) -> String {
    let mut result = String::with_capacity(256);

    let mut last_text_chars = 0;
    let mut last_chord_chars = 0;
    let mut last_last_whitespace = false;
    for part in &line.parts {
        let (part, tc, cc, fws, lws) = render_part::render_part(part, key, language);
        let diff = last_text_chars as i32 - last_chord_chars as i32 - 1;
        let inside_word = !last_last_whitespace && !fws;

        if diff < 1 && last_chord_chars > 0 {
            result.push_str("<span class=\"text\">");
            if inside_word {
                result.push_str(&" ".repeat((-diff).min(1) as usize));
                result.push_str("-");
                result.push_str(&" ".repeat((-diff) as usize));
            } else {
                result.push_str(&" ".repeat((-diff) as usize * 3));
            }
            result.push_str("</span>");
        }

        result.push_str(&part);
        last_text_chars = tc;
        last_chord_chars = cc;
        if tc > 0 {
            last_last_whitespace = lws;
        }
    }

    result.push_str("<br>");

    result
}
