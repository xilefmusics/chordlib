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

        // Handle the first part separately
        if !parts.is_empty() {
            let current = std::mem::take(&mut parts[0]);

            let empty = Part {
                chord: None,
                comment: current.comment,
                languages: vec![String::new(); current.languages.len()],
            };

            let (new_current, new_insert) = current.move_chord_to_next_vowel(empty);

            if new_insert.languages.iter().any(|s| !s.is_empty()) {
                parts[0] = new_current;
                parts.insert(0, new_insert);
            } else {
                parts[0] = new_current;
            }
        }

        Line { parts }
    }
}
