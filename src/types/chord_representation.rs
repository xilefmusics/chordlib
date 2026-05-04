use crate::error::Error;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChordRepresentation {
    #[default]
    Default,
    Nashville,
}

impl ChordRepresentation {
    pub fn symbols(&self, pitch_class: u8) -> &'static [&'static str] {
        match self {
            ChordRepresentation::Default => &DEFAULT_SYMBOLS[pitch_class as usize],
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
        "B", "C", "C#", "D", "D#", "E", "F", "Gb", "G", "G#", "A", "A#",
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
