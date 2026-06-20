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
}

impl<'a> Iterator for LineRenderer<'a> {
    type Item = (String, String);

    fn next(&mut self) -> Option<Self::Item> {
        let (part, first, last, _, ends_with) = self.next_part?;
        self.update_next_part();

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
            let next_text = p.text_for_language(self.language);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Chord, Line, Part};
    use std::str::FromStr;

    #[test]
    fn render_line_falls_back_to_primary_language_for_missing_translation() {
        let line = Line::new(vec![
            Part {
                chord: Some(Chord::from_str("C").unwrap()),
                languages: vec!["Nurdeutsch".to_string()],
                comment: false,
            },
            Part {
                chord: None,
                languages: vec!["Auchnurdeutsch".to_string()],
                comment: false,
            },
        ]);
        let key = SimpleChord::default();
        let rep = ChordRepresentation::Default;

        let html = render_line(&line, &key, &rep, 1);

        assert!(html.contains("Nurdeutsch"));
        assert!(html.contains("Auchnurdeutsch"));
        assert!(
            !html.contains("<span class=\"text\"> </span>"),
            "missing translation must not render empty text placeholder: {html}"
        );
    }

    #[test]
    fn render_line_uses_translation_when_present() {
        let line = Line::new(vec![Part {
            chord: Some(Chord::from_str("C").unwrap()),
            languages: vec!["Nur Deutsch".to_string(), "Only English".to_string()],
            comment: false,
        }]);
        let key = SimpleChord::default();
        let rep = ChordRepresentation::Default;

        let html_lang1 = render_line(&line, &key, &rep, 1);

        assert!(html_lang1.contains("Only English"));
        assert!(!html_lang1.contains("Nur Deutsch"));
    }
}
