use crate::error::Error;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Sharp vs flat bias for the **written chord root** when selecting a chromatic spelling row.
///
/// Slash-bass names use the root pitch class as the spelling key; `PreferSharp` / `PreferFlat`
/// pick the enharmonic row when the root was parsed with `#` / `b` (e.g. `C#` vs `Db`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RootSpellingHint {
    #[default]
    Default,
    PreferSharp,
    PreferFlat,
}

/// Infer spelling bias from the parsed root token (e.g. `C#`, `Db`, `Bb`).
pub fn root_spelling_from_symbol(symbol: &str) -> RootSpellingHint {
    if symbol.contains('#') {
        RootSpellingHint::PreferSharp
    } else if symbol.chars().nth(1) == Some('b') {
        RootSpellingHint::PreferFlat
    } else {
        RootSpellingHint::Default
    }
}

fn default_symbols_row(key_pc: u8, hint: RootSpellingHint) -> &'static [&'static str] {
    let key_pc = key_pc as usize;
    match hint {
        RootSpellingHint::Default => &DEFAULT_SYMBOLS[key_pc],
        RootSpellingHint::PreferSharp => {
            SHARP_SPELLING_ROW[key_pc].unwrap_or(&DEFAULT_SYMBOLS[key_pc])
        }
        RootSpellingHint::PreferFlat => {
            FLAT_SPELLING_ROW[key_pc].unwrap_or(&DEFAULT_SYMBOLS[key_pc])
        }
    }
}

/// Rows for flat-key tonics when the root was written with a sharp (e.g. `C#` vs `Db` row).
const SHARP_SPELLING_ROW: [Option<&'static [&'static str]>; 12] = [
    None,
    Some(&[
        "A#", "B", "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A",
    ]),
    None,
    None,
    Some(&[
        "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B", "C",
    ]),
    None,
    Some(&[
        "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B", "C", "C#", "D",
    ]),
    None,
    Some(&[
        "E#", "F", "F#", "G", "G#", "A", "A#", "B", "C#", "D", "D#", "E",
    ]),
    None,
    None,
    Some(&[
        "G#", "A", "A#", "B", "C", "C#", "D", "D#", "E", "F", "F#", "G",
    ]),
];

/// Rows for sharp-key tonics when the root was written with a flat (e.g. `Gb` vs `F#` row).
const FLAT_SPELLING_ROW: [Option<&'static [&'static str]>; 12] = [
    None,
    None,
    Some(&[
        "Cb", "C", "Db", "D", "Eb", "E", "F", "Gb", "G", "Ab", "A", "Bb",
    ]),
    None,
    None,
    None,
    None,
    Some(&[
        "Fb", "F", "Gb", "G", "Ab", "A", "Bb", "B", "C", "Db", "D", "Eb",
    ]),
    None,
    Some(&[
        "Gb", "G", "Ab", "A", "Bb", "Cb", "C", "Db", "D", "Eb", "E", "F",
    ]),
    None,
    None,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChordRepresentation {
    #[default]
    Default,
    Nashville,
}

impl ChordRepresentation {
    pub fn symbols(&self, pitch_class: u8) -> &'static [&'static str] {
        self.symbols_with_root_spelling(pitch_class, RootSpellingHint::Default)
    }

    pub fn symbols_with_root_spelling(
        &self,
        pitch_class: u8,
        root_spelling: RootSpellingHint,
    ) -> &'static [&'static str] {
        match self {
            ChordRepresentation::Default => default_symbols_row(pitch_class, root_spelling),
            ChordRepresentation::Nashville => NASHVILLE_SYMBOLS,
        }
    }
}

impl fmt::Display for ChordRepresentation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ChordRepresentation::Default => "default",
            ChordRepresentation::Nashville => "nashville",
        };
        write!(f, "{}", s)
    }
}

static NASHVILLE_SYMBOLS: &[&str] = &[
    "1", "b2", "2", "b3", "3", "4", "b5", "5", "b6", "6", "b7", "7",
];

static DEFAULT_SYMBOLS: [[&str; 12]; 12] = [
    [
        "A", "Bb", "B", "C", "C#", "D", "Eb", "E", "F", "F#", "G", "G#",
    ],
    [
        "Bb", "Cb", "C", "Db", "D", "Eb", "Fb", "F", "Gb", "G", "Ab", "A",
    ],
    [
        "B", "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#",
    ],
    [
        "C", "Db", "D", "Eb", "E", "F", "F#", "G", "G#", "A", "Bb", "B",
    ],
    [
        "Db", "D", "Eb", "E", "F", "Gb", "G", "Ab", "A", "Bb", "B", "C",
    ],
    [
        "D", "Eb", "E", "F", "F#", "G", "G#", "A", "A#", "B", "C", "C#",
    ],
    [
        "Eb", "E", "F", "Gb", "G", "Ab", "A", "Bb", "B", "C", "Db", "D",
    ],
    [
        "E", "F", "F#", "G", "G#", "A", "A#", "B", "C", "C#", "D", "D#",
    ],
    [
        "F", "Gb", "G", "Ab", "A", "Bb", "B", "C", "Db", "D", "Eb", "E",
    ],
    [
        "F#", "G", "G#", "A", "A#", "B", "C", "C#", "D", "D#", "E", "F",
    ],
    [
        "G", "Ab", "A", "Bb", "B", "C", "Db", "D", "Eb", "E", "F", "F#",
    ],
    [
        "Ab", "A", "Bb", "Cb", "C", "Db", "D", "Eb", "E", "F", "Gb", "G",
    ],
];

pub fn symbol_to_pitch_class(symbol: &str) -> Result<u8, Error> {
    Ok(match symbol {
        "A" | "1" | "7#" => 0,
        "A#" | "Bb" | "1#" | "2b" => 1,
        "B" | "Cb" | "2" => 2,
        "C" | "B#" | "2#" | "3b" => 3,
        "C#" | "Db" | "3" | "4b" => 4,
        "D" | "4" | "3#" => 5,
        "D#" | "Eb" | "4#" | "5b" => 6,
        "E" | "Fb" | "5" => 7,
        "F" | "E#" | "5#" | "6b" => 8,
        "F#" | "Gb" | "6" => 9,
        "G" | "6#" | "7b" => 10,
        "G#" | "Ab" | "7" | "1b" => 11,
        _ => return Err(Error::Parse(format!("unknown symbol: {}", symbol))),
    })
}
