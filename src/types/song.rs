use serde::{Deserialize, Serialize};

use super::{Section, SimpleChord};

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct Song {
    pub title: String,
    pub subtitle: Option<String>,
    pub copyright: Option<String>,
    pub key: Option<SimpleChord>,
    pub artist: Option<String>,
    pub language: Option<String>,
    pub tempo: Option<u32>,
    pub time: Option<(u32, u32)>,
    pub sections: Vec<Section>,
}

impl Song {
    pub fn transpose(&mut self, key: SimpleChord) -> &mut Self {
        self.key = Some(key);
        self
    }

    pub fn normalize(&mut self) -> &mut Self {
        for section in &mut self.sections {
            if let Some(key) = &self.key {
                section.normalize(&key);
            }
        }
        self
    }

    pub fn bar_duration(&self) -> u32 {
        if let Some((numerator, denominator)) = self.time {
            96 * numerator / denominator
        } else {
            96
        }
    }

    pub fn move_chords_to_next_vowels(mut self) -> Self {
        self.sections = self
            .sections
            .into_iter()
            .map(Section::move_chords_to_next_vowels)
            .collect();
        self
    }
}
