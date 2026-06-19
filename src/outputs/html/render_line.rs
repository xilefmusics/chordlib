use super::render_part;
use crate::types::{ChordRepresentation, Line, Part, SimpleChord};

type NextPartInfo<'a> = (&'a Part, Option<usize>, Option<usize>, bool, bool);

struct LineRenderer<'a> {
    parts: std::slice::Iter<'a, Part>,
    next_part: Option<NextPartInfo<'a>>,
    inside_word: bool,
    key: &'a SimpleChord,
    representation: &'a ChordRepresentation,
    language: usize,
    use_primary_fallback: bool,
}

impl<'a> Iterator for LineRenderer<'a> {
    type Item = (String, String);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (part, first, last, _, ends_with) = self.next_part?;
            self.update_next_part();

            let text = if self.use_primary_fallback {
                part.text_for_language(self.language)
            } else {
                part.text_for_language_exact(self.language)
            };

            if !self.use_primary_fallback
                && text.is_empty()
                && part.chord.is_none()
                && !part.comment
            {
                self.inside_word = false;
                continue;
            }

            let next_starts_with = self
                .next_part
                .is_some_and(|(_, _, _, starts_with, _)| starts_with);

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
                self.use_primary_fallback,
                start,
                end,
            );

            return Some((
                part,
                Self::suffix(
                    self.next_part.is_some(),
                    inside_word,
                    chord_chars as i32 - text_chars as i32 + 1,
                ),
            ));
        }
    }
}

impl<'a> LineRenderer<'a> {
    fn new(
        line: &'a Line,
        key: &'a SimpleChord,
        representation: &'a ChordRepresentation,
        language: usize,
    ) -> Self {
        // If this line contains any text for the requested language, render that exact
        // translation and skip empty primary-only parts. Otherwise fall back to the primary.
        let use_primary_fallback = !line
            .parts
            .iter()
            .any(|part| !part.text_for_language_exact(language).is_empty());

        let mut s = Self {
            parts: line.parts.iter(),
            next_part: None,
            inside_word: false,
            key,
            representation,
            language,
            use_primary_fallback,
        };
        s.update_next_part();
        s
    }

    fn update_next_part(&mut self) {
        self.next_part = self.parts.next().map(|p| {
            let next_text = if self.use_primary_fallback {
                p.text_for_language(self.language)
            } else {
                p.text_for_language_exact(self.language)
            };
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
        let mut result = String::with_capacity(64 + diff * 6);
        result.push_str("<span class=\"text\">");
        if inside_word {
            let spaces = (diff - 1) * 2;
            for _ in 0..spaces {
                result.push(' ');
            }
            result.push('-');
            for _ in 0..spaces {
                result.push(' ');
            }
        } else {
            let spaces = diff * 5;
            for _ in 0..spaces {
                result.push(' ');
            }
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
