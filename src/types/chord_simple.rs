use serde::{Deserialize, Serialize};

use crate::error::Error;

pub(crate) static CHORD_STRINGS_SHARP: &[&str] = &[
    "A", "A#", "B", "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#",
];
pub(crate) static CHORD_STRINGS_FLAT: &[&str] = &[
    "A", "Bb", "B", "C", "Db", "D", "Eb", "E", "F", "Gb", "G", "Ab",
];

static CHORD_STRINGS_NASHVILLE: &[&str] = &[
    "1", "b2", "2", "b3", "3", "4", "b5", "5", "b6", "6", "b7", "7",
];

/// Longest `CHORD_STRINGS_NASHVILLE` entry that prefixes `s` (degree index and byte length).
pub(crate) fn match_nashville_chord_prefix(s: &str) -> Option<(usize, usize)> {
    let mut best: Option<(usize, usize)> = None;
    for (idx, &name) in CHORD_STRINGS_NASHVILLE.iter().enumerate() {
        if s.starts_with(name) {
            let len = name.len();
            if best.is_none_or(|(_, l)| len > l) {
                best = Some((idx, len));
            }
        }
    }
    best
}

pub(crate) static CHORD_STRINGS_ENHARMONIC: &[&str] = &["B#", "E#", "Cb", "Fb"];
pub(crate) static CHORD_LEVELS_ENHARMONIC: &[u8] = &[3, 8, 2, 7];

const _: () = assert!(CHORD_STRINGS_ENHARMONIC.len() == CHORD_LEVELS_ENHARMONIC.len());

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
        } else if let Some(i) = CHORD_STRINGS_ENHARMONIC.iter().position(|e| *e == s) {
            Ok(Self::new(CHORD_LEVELS_ENHARMONIC[i]))
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

    /// Rewrites **absolute** pitch class (`self`) to a semitone interval from `key` (tonic of
    /// `key` at its own pitch class).
    pub fn normalize(&self, key: &Self) -> Self {
        self.transpose(12 - key.level)
    }

    pub(crate) fn combined_level(&self, key: &Self) -> u8 {
        (self.level + key.level) % 12
    }

    pub(crate) fn pitch_class(&self) -> u8 {
        self.level
    }

    pub(crate) fn root_display_name(m_abs: u8, key: &Self) -> &'static str {
        if matches!(key.level, 0 | 2 | 3 | 5 | 7 | 9 | 10) {
            CHORD_STRINGS_SHARP[m_abs as usize]
        } else {
            CHORD_STRINGS_FLAT[m_abs as usize]
        }
    }

    pub fn format(&self, key: &SimpleChord, representation: &ChordRepresentation) -> &'static str {
        match representation {
            ChordRepresentation::Nashville => {
                // Scale degree relative to the song key: after `Chord::normalize`, `self.level` is
                // the semitone interval from the key root to the chord root.
                CHORD_STRINGS_NASHVILLE[(self.level % 12) as usize]
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ChordRepresentation;

    #[test]
    fn try_from_enharmonic_string_maps_pitch_class() {
        assert_eq!(SimpleChord::try_from("B#").unwrap().pitch_class(), 3);
        assert_eq!(SimpleChord::try_from("E#").unwrap().pitch_class(), 8);
        assert_eq!(SimpleChord::try_from("Cb").unwrap().pitch_class(), 2);
        assert_eq!(SimpleChord::try_from("Fb").unwrap().pitch_class(), 7);
    }

    #[test]
    fn try_from_sharp_and_flat_tables_cover_twelve_classes() {
        let mut seen = [false; 12];
        for &name in CHORD_STRINGS_SHARP {
            let pc = SimpleChord::try_from(name).unwrap().pitch_class();
            seen[pc as usize] = true;
        }
        for &name in CHORD_STRINGS_FLAT {
            let pc = SimpleChord::try_from(name).unwrap().pitch_class();
            seen[pc as usize] = true;
        }
        assert!(seen.iter().all(|&v| v));
    }

    #[test]
    fn combined_level_adds_key_mod_twelve() {
        let note = SimpleChord::try_from("E").unwrap();
        let key = SimpleChord::try_from("G").unwrap();
        assert_eq!(note.combined_level(&key), (7 + 10) % 12);
    }

    #[test]
    fn root_display_name_sharp_vs_flat_branch() {
        let sharp_key = SimpleChord::new(0);
        let flat_key = SimpleChord::new(1);
        assert_eq!(SimpleChord::root_display_name(4, &sharp_key), "C#");
        assert_eq!(SimpleChord::root_display_name(4, &flat_key), "Db");
    }

    #[test]
    fn format_default_matches_table_for_natural_c_in_sharp_key() {
        let c = SimpleChord::try_from("C").unwrap();
        let key = SimpleChord::default();
        assert_eq!(c.format(&key, &ChordRepresentation::Default), "C");
    }

    #[test]
    fn nashville_degrees_twelve_entries() {
        assert_eq!(CHORD_STRINGS_NASHVILLE.len(), 12);
    }
}
