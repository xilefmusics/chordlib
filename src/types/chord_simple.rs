use serde::{Deserialize, Serialize};

use crate::error::Error;

static CHORD_STRINGS_SHARP: &[&str] = &[
    "A", "A#", "B", "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#",
];
static CHORD_STRINGS_FLAT: &[&str] = &[
    "A", "Bb", "B", "C", "Db", "D", "Eb", "E", "F", "Gb", "G", "Ab",
];

static CHORD_STRINGS_NASHVILLE: &[&str] = &[
    "1", "b2", "2", "b3", "3", "4", "b5", "5", "b6", "6", "b7", "7",
];

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
            'B' => Ok(Self::new(2)),
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
        if let Some(level) = CHORD_STRINGS_SHARP.iter().position(|e| *e == s) {
            Ok(Self::new(level as u8))
        } else if let Some(level) = CHORD_STRINGS_FLAT.iter().position(|e| *e == s) {
            Ok(Self::new(level as u8))
        } else if let Some(level) = CHORD_STRINGS_NASHVILLE.iter().position(|e| *e == s) {
            Ok(Self::new(level as u8))
        } else {
            Err(Error::Parse(format!("unknown level, {}", s)))
        }
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

    pub fn format(&self, key: &SimpleChord, representation: &ChordRepresentation) -> &'static str {
        match representation {
            ChordRepresentation::Nashville => CHORD_STRINGS_NASHVILLE[self.level as usize],
            ChordRepresentation::Default => match key.level {
                0 | 2 | 3 | 5 | 7 | 9 | 10 => {
                    CHORD_STRINGS_SHARP[((self.level + key.level) % 12) as usize]
                }
                _ => CHORD_STRINGS_FLAT[((self.level + key.level) % 12) as usize],
            },
        }
    }

    pub fn guess_key(key: &str) -> SimpleChord {
        if key.is_empty() {
            return SimpleChord::default();
        }

        let chord = match SimpleChord::try_from(&key[..1]) {
            Ok(chord) => chord,
            Err(_) => return SimpleChord::default(),
        };

        let chord = if key.len() > 1 {
            SimpleChord::try_from(&key[..2]).unwrap_or(chord)
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

use serde::Deserializer;
use serde_json::Value;

use super::ChordRepresentation;
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
