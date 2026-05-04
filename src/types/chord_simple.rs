use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::text::remove_space_separators;

use super::ChordRepresentation;

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

const DIATONIC_LETTERS: &str = "CDEFGAB";

fn letter_natural_pc(c: char) -> Option<u8> {
    match c {
        'A' => Some(0),
        'B' => Some(2),
        'C' => Some(3),
        'D' => Some(5),
        'E' => Some(7),
        'F' => Some(8),
        'G' => Some(10),
        _ => None,
    }
}

fn advance_diatonic_letter(root: char, steps: usize) -> Option<char> {
    let i = DIATONIC_LETTERS.find(root)?;
    Some(DIATONIC_LETTERS.as_bytes()[(i + steps) % 7] as char)
}

fn parse_root_nominal_letter(root_display: &str, expect_m_abs: u8) -> Option<char> {
    let mut cs = root_display.chars();
    let letter = cs.next()?;
    if !matches!(letter, 'A'..='G') {
        return None;
    }
    let nat = letter_natural_pc(letter)?;
    let acc: i16 = cs.fold(0i16, |a, ch| match ch {
        '#' => a + 1,
        'b' => a - 1,
        _ => a,
    });
    let pc = (nat as i16 + acc).rem_euclid(12) as u8;
    if pc != expect_m_abs {
        return None;
    }
    Some(letter)
}

fn spell_letter_to_pc(letter: char, target_pc: u8) -> Option<&'static str> {
    for &name in CHORD_STRINGS_SHARP
        .iter()
        .chain(CHORD_STRINGS_FLAT.iter())
        .chain(CHORD_STRINGS_ENHARMONIC.iter())
    {
        if !name.starts_with(letter) {
            continue;
        }
        let Ok(sc) = SimpleChord::try_from(name) else {
            continue;
        };
        if sc.pitch_class() == target_pc {
            return Some(name);
        }
    }
    None
}

fn spell_slash_bass_from_intervals(
    root_letter: char,
    _m_abs: u8,
    b_abs: u8,
    interval: u8,
) -> Option<&'static str> {
    match interval {
        0 => spell_letter_to_pc(root_letter, b_abs),
        1 | 2 => {
            let t = advance_diatonic_letter(root_letter, 1)?;
            spell_letter_to_pc(t, b_abs)
        }
        3 | 4 => {
            let t = advance_diatonic_letter(root_letter, 2)?;
            spell_letter_to_pc(t, b_abs)
        }
        5 => {
            let t = advance_diatonic_letter(root_letter, 3)?;
            spell_letter_to_pc(t, b_abs)
        }
        6 => {
            let fifth = advance_diatonic_letter(root_letter, 4)?;
            if let Some(s) = spell_letter_to_pc(fifth, b_abs) {
                return Some(s);
            }
            let fourth = advance_diatonic_letter(root_letter, 3)?;
            spell_letter_to_pc(fourth, b_abs)
        }
        7 => {
            let t = advance_diatonic_letter(root_letter, 4)?;
            spell_letter_to_pc(t, b_abs)
        }
        8..=11 => {
            let t = advance_diatonic_letter(root_letter, 5)?;
            spell_letter_to_pc(t, b_abs)
        }
        _ => None,
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

    /// Best-effort key from a free-form label (e.g. `"Dm"`, `"F# / major"`). Uses the first
    /// 1–2 **Unicode characters** (not bytes) to match `TryFrom<&str>`, so multi-byte names do not
    /// cause panics. [Unicode `Space_Separator`](crate::text::remove_space_separators) (Zs) “fake
    /// spaces” (e.g. U+00A0, U+205F) are **removed** so tokens like `C#` and `C #` both resolve.
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

fn spell_root_diatonic_nominal(interval: u8, key: &SimpleChord, m_abs: u8) -> Option<&'static str> {
    let interval = interval % 12;
    let tonic_pc = key.level;
    let tonic_display = SimpleChord::root_display_name(tonic_pc, key);
    let tonic_letter = parse_root_nominal_letter(tonic_display, tonic_pc)?;
    let diatonic_steps = match interval {
        0 => 0,
        1 | 2 => 1,
        3 | 4 => 2,
        5 => 3,
        6 | 7 => 4,
        8 | 9 => 5,
        10 | 11 => 6,
        _ => return None,
    };
    let letter = if diatonic_steps == 0 {
        tonic_letter
    } else {
        advance_diatonic_letter(tonic_letter, diatonic_steps)?
    };
    spell_letter_to_pc(letter, m_abs)
}

/// Letter-name root when the main was parsed as a keyed Nashville numeral (e.g. `b7`).
pub(crate) fn format_nashville_root_default(main: &SimpleChord, key: &SimpleChord) -> &'static str {
    let m_abs = (main.level + key.level) % 12;
    if main.level % 12 == 10
        && let Some(s) = spell_root_diatonic_nominal(10, key, m_abs)
    {
        return s;
    }
    SimpleChord::root_display_name(m_abs, key)
}

/// Slash bass spelling for [`ChordRepresentation::Default`].
pub(crate) fn format_slash_bass_default(
    main: &SimpleChord,
    base: &SimpleChord,
    key: &SimpleChord,
) -> &'static str {
    let m_abs = main.combined_level(key);
    let b_abs = base.combined_level(key);
    let interval = (b_abs + 12 - m_abs) % 12;
    let root_display = SimpleChord::root_display_name(m_abs, key);
    let Some(root_letter) = parse_root_nominal_letter(root_display, m_abs) else {
        return base.format(key, &ChordRepresentation::Default);
    };
    spell_slash_bass_from_intervals(root_letter, m_abs, b_abs, interval)
        .unwrap_or_else(|| base.format(key, &ChordRepresentation::Default))
}

use serde::Deserializer;
use serde_json::Value;

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
    fn guess_key_uses_chars_not_bytes_for_two_letter_names() {
        assert_eq!(
            SimpleChord::guess_key("Bb").pitch_class(),
            SimpleChord::try_from("Bb").unwrap().pitch_class()
        );
        assert_eq!(
            SimpleChord::guess_key("D#").pitch_class(),
            SimpleChord::try_from("D#").unwrap().pitch_class()
        );
    }

    #[test]
    fn guess_key_does_not_panic_on_medium_mathematical_space() {
        // U+205F is 3 UTF-8 bytes; old byte slices would panic on similar strings.
        let s = format!("D\u{205F} ");
        assert_eq!(SimpleChord::guess_key(&s).pitch_class(), 5);
        assert_eq!(SimpleChord::guess_key("C\u{205F}#").pitch_class(), 4);
        assert_eq!(SimpleChord::guess_key("C#").pitch_class(), 4);
        assert_eq!(
            SimpleChord::guess_key("So\u{205f}pl"),
            SimpleChord::default()
        );
    }

    #[test]
    fn nashville_degrees_twelve_entries() {
        assert_eq!(CHORD_STRINGS_NASHVILLE.len(), 12);
    }
}
