use serde::{Deserialize, Serialize};

use super::{Section, SimpleChord};

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct Song {
    pub title: String,
    pub key: Option<SimpleChord>,
    pub artist: Option<String>,
    pub language: Option<String>,
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
}
