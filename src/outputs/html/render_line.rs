use super::render_part;
use crate::types::{ChordRepresentation, Line, Part, SimpleChord};

struct LineRenderer<'a> {
    parts: std::slice::Iter<'a, Part>,
    next_part: Option<(&'a Part, Option<usize>, Option<usize>, bool, bool)>,
    inside_word: bool,
    key: &'a SimpleChord,
    representation: &'a ChordRepresentation,
    language: usize,
}

impl<'a> Iterator for LineRenderer<'a> {
    type Item = (String, String);

    fn next(&mut self) -> Option<Self::Item> {
        let (part, first, last, _, ends_with) = self.next_part?;
        self.update_next_part();

        let next_starts_with = self
            .next_part
            .map_or(false, |(_, _, _, starts_with, _)| starts_with);

        let inside_word = !ends_with && !next_starts_with && self.next_part.is_some();

        let end = if self.inside_word && first.is_some() {
            first
        } else {
            None
        };
        let start = if inside_word && last.is_some() {
            last
        } else {
            None
        };

        self.inside_word = inside_word;

        let (part, text_chars, chord_chars) = render_part::render_part(
            part,
            self.key,
            self.representation,
            self.language,
            start,
            end,
        );

        Some((
            part,
            Self::suffix(
                self.next_part.is_some(),
                inside_word,
                chord_chars as i32 - text_chars as i32 + 1,
            ),
        ))
    }
}

impl<'a> LineRenderer<'a> {
    fn new(
        line: &'a Line,
        key: &'a SimpleChord,
        representation: &'a ChordRepresentation,
        language: usize,
    ) -> Self {
        let mut s = Self {
            parts: line.parts.iter(),
            next_part: None,
            inside_word: false,
            key,
            representation,
            language,
        };
        s.update_next_part();
        s
    }

    fn update_next_part(&mut self) {
        self.next_part = self.parts.next().map(|p| {
            let next_text = p
                .languages
                .get(self.language)
                .map(String::as_str)
                .unwrap_or("");
            let mut first = None;
            let mut last = None;
            let mut starts_with = false;
            let mut ends_with = false;

            for (i, c) in next_text.char_indices() {
                if c.is_whitespace() {
                    if first.is_none() {
                        first = Some(i);
                    }
                    last = Some(i);
                    ends_with = true;
                    if i == 0 {
                        starts_with = true;
                    }
                } else {
                    ends_with = false;
                }
            }

            (p, first, last, starts_with, ends_with)
        });
    }

    fn suffix(has_next: bool, inside_word: bool, diff: i32) -> String {
        if !has_next || diff < 1 {
            return "".to_string();
        }
        let diff = diff as usize;
        let mut result = String::with_capacity(256);
        result.push_str("<span class=\"text\">");
        if inside_word {
            result.push_str(&" ".repeat((diff - 1) * 2));
            result.push_str("-");
            result.push_str(&" ".repeat((diff - 1) * 2));
        } else {
            result.push_str(&" ".repeat(diff * 5));
        }
        result.push_str("</span>");
        result
    }

    fn render(&mut self) -> String {
        let mut result = String::with_capacity(256);
        for (part, suffix) in self.by_ref() {
            result.push_str(&part);
            result.push_str(&suffix);
        }
        result.push_str("<br>");
        result
    }
}

pub fn render_line(
    line: &Line,
    key: &SimpleChord,
    representation: &ChordRepresentation,
    language: usize,
) -> String {
    LineRenderer::new(line, key, representation, language).render()
}
