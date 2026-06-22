use super::{FormatOutputLines, OutputLine, repeat_label};
use crate::Error;
use crate::types::{ChordRepresentation, SimpleChord, Song};
use std::cmp::max;
use std::iter::{Chain, Repeat, Take, Zip};
use std::slice::Iter;

#[derive(Default)]
pub struct CharPageSet<'a> {
    keyword_prefix: &'a str,
    keyword_suffix: &'a str,
    text_prefix: &'a str,
    text_suffix: &'a str,
    chord_prefix: &'a str,
    chord_suffix: &'a str,
}

impl<'a> CharPageSet<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn keyword_prefix(mut self, keyword_prefix: &'a str) -> Self {
        self.keyword_prefix = keyword_prefix;
        self
    }
    pub fn keyword_suffix(mut self, keyword_suffix: &'a str) -> Self {
        self.keyword_suffix = keyword_suffix;
        self
    }
    pub fn text_prefix(mut self, text_prefix: &'a str) -> Self {
        self.text_prefix = text_prefix;
        self
    }
    pub fn text_suffix(mut self, text_suffix: &'a str) -> Self {
        self.text_suffix = text_suffix;
        self
    }
    pub fn chord_prefix(mut self, chord_prefix: &'a str) -> Self {
        self.chord_prefix = chord_prefix;
        self
    }
    pub fn chord_suffix(mut self, chord_suffix: &'a str) -> Self {
        self.chord_suffix = chord_suffix;
        self
    }
}

#[derive(Debug, Clone)]
pub enum CharPageLine {
    Keyword(String),
    Chord(String),
    Text(String),
    Empty,
}

impl CharPageLine {
    pub fn len(&self) -> usize {
        match self {
            Self::Keyword(s) => s.chars().count() + 1,
            Self::Chord(s) => s.chars().count(),
            Self::Text(s) => s.chars().count(),
            Self::Empty => 0,
        }
    }

    pub fn render(&self, set: &CharPageSet) -> String {
        match self {
            CharPageLine::Keyword(s) => {
                format!("{}{}:{}", set.keyword_prefix, s, set.keyword_suffix)
            }
            CharPageLine::Text(s) => format!("{}{}{}", set.text_prefix, s, set.text_suffix),
            CharPageLine::Chord(s) => format!("{}{}{}", set.chord_prefix, s, set.chord_suffix),
            CharPageLine::Empty => "".into(),
        }
    }
}

#[derive(Debug, Default)]
pub struct CharPage {
    first_column: Vec<CharPageLine>,
    second_column: Vec<CharPageLine>,
    first_column_width: usize,
    first_column_height: usize,
    second_column_width: usize,
    second_column_height: usize,
    max_width: usize,
    max_height: usize,
}

impl CharPage {
    pub fn new(max_width: usize, max_height: usize) -> Self {
        Self {
            max_width,
            max_height,
            ..Self::default()
        }
    }

    #[allow(clippy::type_complexity)]
    pub fn rows(
        &self,
    ) -> Take<
        Zip<
            Chain<Iter<'_, CharPageLine>, Repeat<&CharPageLine>>,
            Chain<Iter<'_, CharPageLine>, Repeat<&CharPageLine>>,
        >,
    > {
        self.first_column
            .iter()
            .chain(std::iter::repeat(&CharPageLine::Empty))
            .zip(
                self.second_column
                    .iter()
                    .chain(std::iter::repeat(&CharPageLine::Empty)),
            )
            .take(max(self.first_column_height, self.second_column_height))
    }

    pub fn render(&self, set: &CharPageSet) -> String {
        self.rows()
            .map(|(first, second)| (first.render(set), first.len(), second.render(set)))
            .map(|(first, first_len, second)| {
                format!(
                    "{}{}{}",
                    first,
                    " ".repeat(self.max_width - first_len - self.second_column_width),
                    second
                )
            })
            .collect::<Vec<String>>()
            .join("\n")
    }

    pub fn try_add_lines(&mut self, lines: Vec<CharPageLine>) -> Vec<CharPageLine> {
        let height = lines.len();
        let width = lines.iter().map(|line| line.len()).max().unwrap();
        if self.max_height - self.first_column_height > height {
            if self.first_column_height > 0 {
                self.first_column.push(CharPageLine::Empty);
                self.first_column.push(CharPageLine::Empty);
                self.first_column_height += 1;
            }
            self.first_column.extend(lines);
            self.first_column_height += height;
            self.first_column_width = max(self.first_column_width, width);
            Vec::new()
        } else if self.max_width - self.first_column_width > width
            && self.max_height - self.second_column_height > height
        {
            if self.second_column_height > 0 {
                self.second_column.push(CharPageLine::Empty);
                self.second_column.push(CharPageLine::Empty);
                self.second_column_height += 1;
            }
            self.second_column.extend(lines);
            self.second_column_height += height;
            self.second_column_width = max(self.first_column_width, width);
            Vec::new()
        } else {
            lines
        }
    }
}

pub trait FormatCharPages {
    fn format_char_pages(
        &self,
        max_width: usize,
        max_height: usize,
        key: Option<&SimpleChord>,
        representation: Option<&ChordRepresentation>,
        language: Option<usize>,
    ) -> Result<Vec<CharPage>, Error>;
}

impl FormatCharPages for &Song {
    fn format_char_pages(
        &self,
        max_width: usize,
        max_height: usize,
        key: Option<&SimpleChord>,
        representation: Option<&ChordRepresentation>,
        language: Option<usize>,
    ) -> Result<Vec<CharPage>, Error> {
        let mut char_pages = vec![CharPage::new(max_width, max_height)];

        for section in &self.sections {
            let mut lines: Vec<CharPageLine> = (section)
                .format_output_lines(key, representation, language)?
                .into_iter()
                .map(|line| match line {
                    OutputLine::Keyword(s) => CharPageLine::Keyword(s),
                    OutputLine::Chord(s) => CharPageLine::Chord(s),
                    OutputLine::Text(s) => CharPageLine::Text(s),
                })
                .collect();
            if let Some(repeat_text) = repeat_label(section.repeat_count) {
                lines.push(CharPageLine::Text(repeat_text));
            }

            let lines = char_pages.last_mut().unwrap().try_add_lines(lines);

            if !lines.is_empty() {
                let mut new_char_page = CharPage::new(max_width, max_height);
                new_char_page.try_add_lines(lines);
                char_pages.push(new_char_page);
            }
        }

        Ok(char_pages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inputs::chord_pro::load_string;
    use crate::types::SongFlowItem;

    fn flow_item(title: &str, occurrence_index: u32, repeats: u32) -> SongFlowItem {
        SongFlowItem {
            title: title.to_string(),
            occurrence_index,
            repeats,
        }
    }

    #[test]
    fn format_char_pages_supports_custom_flow_after_apply_flow() {
        let input = r#"{title: Ohne Titel}
{key: A}
{section: Tag 1}
Text 1
{section: Tag 2}
Text 2
{section: Tag 3}
Text 3
{section: Tag 1}
{section: Tag 2}
{section: Tag 3}
"#;
        let mut song = load_string(input).expect("parse");
        let rep = ChordRepresentation::Default;
        let flow = vec![
            flow_item("Tag 1", 0, 1),
            flow_item("Tag 2", 0, 1),
            flow_item("Tag 3", 0, 1),
            flow_item("Tag 1", 0, 2),
            flow_item("Tag 3", 0, 1),
            flow_item("Tag 2", 0, 1),
        ];

        song.apply_flow(flow).expect("apply flow");

        let custom_pages = (&song)
            .format_char_pages(80, 20, None, Some(&rep), None)
            .expect("render");
        let rendered = custom_pages[0].render(&CharPageSet::new());
        assert!(rendered.contains("Tag 1:"));
        assert!(rendered.contains("Text 1"));
        assert!(rendered.contains("(repeat)"));
        assert!(rendered.contains("Tag 3:"));
    }
}
