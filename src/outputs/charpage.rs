use super::{FormatOutputLines, OutputLine};
use crate::types::{SimpleChord, Song};
use std::cmp::max;
use std::iter::{Chain, Repeat, Take, Zip};
use std::slice::Iter;

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
            Self::Keyword(s) => s.chars().count(),
            Self::Chord(s) => s.chars().count(),
            Self::Text(s) => s.chars().count(),
            Self::Empty => 0,
        }
    }

    pub fn terminal(&self) -> String {
        match self {
            CharPageLine::Keyword(s) => format!("\x1b[1m{}\x1b[0m", s),
            CharPageLine::Text(s) => format!("{}", s),
            CharPageLine::Chord(s) => format!("\x1b[1m{}\x1b[0m", s),
            CharPageLine::Empty => "".into(),
        }
    }

    pub fn html(&self) -> String {
        match self {
            CharPageLine::Keyword(s) => format!("<b>{}</b>", s),
            CharPageLine::Text(s) => format!("{}>", s),
            CharPageLine::Chord(s) => format!("<b>{}</b>", s),
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
        let mut char_page = Self::default();
        char_page.max_width = max_width;
        char_page.max_height = max_height;
        return char_page;
    }

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

    pub fn html(&self) -> String {
        self.rows()
            .map(|(first, second)| (first.html(), first.len(), second.html()))
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

    pub fn terminal(&self) -> String {
        self.rows()
            .map(|(first, second)| (first.terminal(), first.len(), second.terminal()))
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
        key: Option<SimpleChord>,
        language: Option<usize>,
    ) -> Vec<CharPage>;
}

impl FormatCharPages for &Song {
    fn format_char_pages(
        &self,
        max_width: usize,
        max_height: usize,
        key: Option<SimpleChord>,
        language: Option<usize>,
    ) -> Vec<CharPage> {
        let mut char_pages = vec![CharPage::new(max_width, max_height)];

        for section in &self.sections {
            let lines = char_pages.last_mut().unwrap().try_add_lines(
                section
                    .format_output_lines(key.clone(), language)
                    .into_iter()
                    .map(|line| match line {
                        OutputLine::Keyword(s) => CharPageLine::Keyword(s),
                        OutputLine::Chord(s) => CharPageLine::Chord(s),
                        OutputLine::Text(s) => CharPageLine::Text(s),
                    })
                    .collect(),
            );

            if lines.len() > 0 {
                let mut new_char_page = CharPage::new(max_width, max_height);
                new_char_page.try_add_lines(lines);
                char_pages.push(new_char_page);
            }
        }

        char_pages
    }
}
