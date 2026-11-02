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
        self.next_part = self.parts.find_map(|p| {
            let next_text = p.text_for_language(self.language);
            // An empty translated fragment with no chord has no visible layout
            // role. Ignore it when joining adjacent lyric parts so it cannot
            // create a synthetic word boundary or hyphen. Keep chorded empty
            // parts because their chord still needs to stay aligned.
            if next_text.is_empty() && p.chord.is_none() {
                return None;
            }

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

            Some((p, first, last, starts_with, ends_with))
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
    use crate::inputs::chord_pro::load_string;
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

    #[test]
    fn render_line_skips_an_empty_fragment_before_the_translated_chord() {
        let song = load_string(
            "{title: Minimal reproduction}\n{key: G}\n{language: de}\n{language2: en}\
             \n{section: Test}\nD[G]u\n&[G]All\n",
        )
        .expect("parse song");
        let line = &song.sections[0].lines[0];
        let html = render_line(
            line,
            song.key.as_ref().expect("song key"),
            &ChordRepresentation::Default,
            1,
        );

        assert!(
            html.contains("All"),
            "translated lyric should render: {html}"
        );
        assert!(
            !html.contains("D"),
            "primary-language leading lyric should not leak: {html}"
        );
        assert!(
            html.contains(
                "<span class=\"part part-has-chord\"><span class=\"chord\">G</span><span class=\"text\">All</span></span>"
            ),
            "G should remain attached to the translated lyric: {html}"
        );
        assert!(
            !html.contains(">-</span>"),
            "empty translated fragments must not create a hyphen: {html}"
        );

        let html_de = render_line(
            line,
            song.key.as_ref().expect("song key"),
            &ChordRepresentation::Default,
            0,
        );
        let german_lyric = line
            .parts
            .iter()
            .map(|part| part.text_for_language(0))
            .collect::<String>();
        assert_eq!(german_lyric, "Du");
        assert!(
            html_de.contains("<span class=\"text\">D</span>"),
            "German leading lyric should remain unchanged: {html_de}"
        );
        assert!(
            html_de.contains("<span class=\"chord\">G</span>"),
            "German G chord should remain present: {html_de}"
        );
    }

    #[test]
    fn render_line_joins_translated_bridge_lyrics_without_leaking_primary_text() {
        let cases = [
            (
                "D[G]u bist meine H[D]offnung\n&[G]All my hope [D]is in You\n",
                "All my hope is in You",
                "All my hope ",
                "is in You",
            ),
            (
                "D[G]u bist meine H[D]offnung\n&[G]All my hope is [D]in You\n",
                "All my hope is in You",
                "All my hope is ",
                "in You",
            ),
        ];

        for (input, expected_english, g_text, d_text) in cases {
            let song = load_string(&format!(
                "{{title: Translation}}\n{{key: G}}\n{{language: de}}\n{{language2: en}}\n{{section: Bridge}}\n{input}"
            ))
            .expect("parse song");
            let line = &song.sections[0].lines[0];
            let key = song.key.as_ref().expect("song key");
            let representation = ChordRepresentation::Default;

            let html_en = render_line(line, key, &representation, 1);
            let english_lyric = line
                .parts
                .iter()
                .map(|part| part.text_for_language(1))
                .collect::<String>();
            assert!(
                html_en.contains("All my hope"),
                "English lyric should render: {html_en}"
            );
            assert_eq!(english_lyric, expected_english);
            assert!(
                !html_en.contains(">-</span>")
                    && !html_en.contains("<span class=\"text\">D</span>")
                    && !html_en.contains("u bist meine H")
                    && !html_en.contains("<span class=\"text\">offnung</span>"),
                "English lyric must not include German text or a generated hyphen: {html_en}"
            );
            assert!(
                html_en.contains("<span class=\"chord\">G</span>")
                    && html_en.contains("<span class=\"chord\">D</span>"),
                "G and D chords should remain aligned with the translated lyric: {html_en}"
            );

            let html_de = render_line(line, key, &representation, 0);
            let german_lyric = line
                .parts
                .iter()
                .map(|part| part.text_for_language(0))
                .collect::<String>();
            assert_eq!(german_lyric, "Du bist meine Hoffnung");
            for german_fragment in [
                "<span class=\"text\">D</span>",
                "<span class=\"text\">u</span>",
                "<span class=\"text\"> bist meine </span>",
                "<span class=\"text\">H</span>",
                "<span class=\"text\">offnung</span>",
            ] {
                assert!(
                    html_de.contains(german_fragment),
                    "German rendering should preserve lyric fragment {german_fragment:?}: {html_de}"
                );
            }
            assert!(
                html_en.contains(&format!(
                    "<span class=\"chord\">G</span><span class=\"text\">{g_text}"
                )) && html_en.contains(&format!(
                    "<span class=\"chord\">D</span><span class=\"text\">{d_text}"
                )),
                "G and D should align with the translated lyric segments: {html_en}"
            );
            assert!(
                html_de.contains("<span class=\"chord\">G</span>")
                    && html_de.contains("<span class=\"chord\">D</span>"),
                "German G and D chords should remain present: {html_de}"
            );
        }
    }
}
