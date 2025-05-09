use serde::{Deserialize, Serialize};

use super::{Line, SimpleChord};

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct Section {
    pub title: String,
    pub lines: Vec<Line>,
}

impl Section {
    pub fn new(title: String, lines: Vec<Line>) -> Self {
        Self { title, lines }
    }

    pub fn normalize(&mut self, key: &SimpleChord) -> &mut Self {
        for line in &mut self.lines {
            line.normalize(key);
        }
        self
    }

    pub fn move_chords_to_next_vowels(mut self) -> Self {
        self.lines = self
            .lines
            .into_iter()
            .map(Line::move_chords_to_next_vowels)
            .collect();
        self
    }

    pub fn remove_manual_spacing(mut self) -> Self {
        self.lines = self
            .lines
            .into_iter()
            .map(Line::remove_manual_spacing)
            .collect();
        self
    }
}
