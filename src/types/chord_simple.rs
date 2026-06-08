use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::error::Error;
use crate::text::remove_space_separators;

use super::chord_representation::{ChordRepresentation, RootSpellingHint, symbol_to_pitch_class};

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct SimpleChord {
    #[serde(deserialize_with = "float_or_int_to_int")]
    level: u8,
}

impl TryFrom<char> for SimpleChord {
    type Error = Error;

    fn try_from(c: char) -> Result<Self, Self::Error> {
        match c {
            'A' => Ok(Self::new(0)),
            'B' | 'H' => Ok(Self::new(2)),
            'C' => Ok(Self::new(3)),
            'D' => Ok(Self::new(5)),
            'E' => Ok(Self::new(7)),
            'F' => Ok(Self::new(8)),
            'G' => Ok(Self::new(10)),
            _ => Err(Error::Parse(format!("unknown level, {}", c))),
        }
    }
}

impl TryFrom<&str> for SimpleChord {
    type Error = Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Ok(Self::new(symbol_to_pitch_class(s)?))
    }
}

impl SimpleChord {
    pub fn new(level: u8) -> Self {
        Self { level: level % 12 }
    }

    pub fn transpose(&self, level: u8) -> Self {
        Self::new(self.level + level)
    }

    pub fn normalize(&self, key: &Self) -> Self {
        self.transpose(12 - key.level)
    }

    pub fn combined_level(&self, key: &Self) -> u8 {
        (self.level + key.level) % 12
    }

    pub fn pitch_class(&self) -> u8 {
        self.level
    }

    pub fn format(&self, key: &SimpleChord, representation: &ChordRepresentation) -> &'static str {
        representation.symbols(key.level)[(self.level % 12) as usize]
    }

    pub fn format_with_key_root_spelling(
        &self,
        key: &SimpleChord,
        representation: &ChordRepresentation,
        key_root_spelling: RootSpellingHint,
    ) -> &'static str {
        representation.symbols_with_root_spelling(key.level, key_root_spelling)
            [(self.level % 12) as usize]
    }

    pub fn guess_key(key: &str) -> SimpleChord {
        let cleaned = remove_space_separators(key);
        let key = cleaned.as_ref().trim_start();
        if key.is_empty() {
            return SimpleChord::default();
        }

        let mut it = key.char_indices();
        let (i0, c0) = match it.next() {
            Some(p) => p,
            None => return SimpleChord::default(),
        };
        let one_end = i0 + c0.len_utf8();
        let one = &key[i0..one_end];

        let chord = match SimpleChord::try_from(one) {
            Ok(chord) => chord,
            Err(_) => return SimpleChord::default(),
        };

        let chord = if let Some((i1, c1)) = it.next() {
            let two_end = i1 + c1.len_utf8();
            let two = &key[i0..two_end];
            SimpleChord::try_from(two).unwrap_or(chord)
        } else {
            chord
        };

        if key.contains('m') {
            chord.transpose(3)
        } else {
            chord
        }
    }
}

fn float_or_int_to_int<'de, D>(deserializer: D) -> Result<u8, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::Number(num) => {
            if let Some(int_value) = num.as_i64() {
                Ok(int_value as u8)
            } else if let Some(float_value) = num.as_f64() {
                Ok(float_value as u8)
            } else {
                Err(serde::de::Error::custom("Invalid number"))
            }
        }
        _ => Err(serde::de::Error::custom("Expected a number")),
    }
}
