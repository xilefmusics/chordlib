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
}
