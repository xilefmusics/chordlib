use serde::{Deserialize, Serialize};

use super::{Part, SimpleChord};

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct Line {
    pub parts: Vec<Part>,
}

impl Line {
    pub fn new(parts: Vec<Part>) -> Self {
        Self { parts }
    }

    pub fn normalize(&mut self, key: &SimpleChord) -> &mut Self {
        for part in &mut self.parts {
            part.normalize(key);
        }
        self
    }

    pub fn move_chords_to_next_vowels(self) -> Self {
        let mut parts = self.parts;

        for i in (1..parts.len()).rev() {
            let current = std::mem::take(&mut parts[i]);
            let prev = std::mem::take(&mut parts[i - 1]);

            let (new_current, new_prev) = current.move_chord_to_next_vowel(prev);
            parts[i] = new_current;
            parts[i - 1] = new_prev;
        }

        Line { parts }
    }

    pub fn remove_manual_spacing(mut self) -> Self {
        let len = self.parts.len();
        let mut i = 0;

        while i < len {
            let current = std::mem::take(&mut self.parts[i]);

            let prev = if i > 0 { Some(std::mem::take(&mut self.parts[i - 1])) } else { None };

            let next = if i + 1 < len {
                Some(std::mem::take(&mut self.parts[i + 1]))
            } else {
                None
            };

            let (new_current, new_prev, new_next) = current.remove_manual_spacing(prev, next);

            if let Some(p) = new_prev {
                self.parts[i - 1] = p;
            }

            self.parts[i] = new_current;

            if let Some(n) = new_next {
                self.parts[i + 1] = n;
            }

            i += 1;
        }

        self
    }
}
