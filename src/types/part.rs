use serde::{Deserialize, Serialize};
use std::str::FromStr;

use super::{Chord, SimpleChord};
use crate::error::Error;

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct Part {
    pub chord: Option<Chord>,
    pub languages: Vec<String>,
}

impl Part {
    pub fn normalize(&mut self, key: &SimpleChord) -> &mut Self {
        self.chord = self.chord.clone().map(|chord| chord.normalize(key));
        self
    }
}

impl TryFrom<(&str, &str)> for Part {
    type Error = Error;

    fn try_from(value: (&str, &str)) -> Result<Self, Self::Error> {
        Ok(Self {
            chord: if value.0.len() == 0 {
                None
            } else {
                Some(Chord::from_str(value.0)?)
            },
            languages: vec![value.1.to_string()],
        })
    }
}
