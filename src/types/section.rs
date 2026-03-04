use serde::{Deserialize, Serialize};

use super::{Line, SimpleChord};

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct Section {
    pub title: String,
    pub lines: Vec<Line>,
    /// How many times this section is repeated (from `{repeat}` or `{repeat: N}`). Default 1.
    #[serde(default)]
    pub repeat_count: u32,
}

impl Section {
    pub fn new(title: String, lines: Vec<Line>) -> Self {
        Self {
            title,
            lines,
            repeat_count: 1,
        }
    }

    pub fn new_with_repeat(title: String, lines: Vec<Line>, repeat_count: u32) -> Self {
        Self {
            title,
            lines,
            repeat_count: repeat_count.max(1),
        }
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
