use serde::{Deserialize, Serialize};
use std::str::FromStr;

use super::chord_simple::{CHORD_STRINGS_ENHARMONIC, CHORD_STRINGS_FLAT, CHORD_STRINGS_SHARP};
use super::{ChordRepresentation, SimpleChord};
use crate::error::Error;

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

fn format_slash_bass_default(
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

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub enum Kind {
    #[default]
    Major,
    Minor,
    Diminished,
    Augmented,
    Suspended2,
    Suspended4,
}

impl Kind {
    pub fn format(&self) -> &str {
        match self {
            Kind::Major => "",
            Kind::Minor => "m",
            Kind::Diminished => "dim",
            Kind::Augmented => "aug",
            Kind::Suspended2 => "sus2",
            Kind::Suspended4 => "sus4",
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct Chord {
    main: SimpleChord,
    base: Option<SimpleChord>,
    kind: Kind,
    var: String,
    duration: Option<u32>,
    #[serde(default)]
    optional: bool,
}

impl Chord {
    pub fn new(level: u8) -> Self {
        Self::default().transpose(level)
    }

    pub fn transpose(self, level: u8) -> Self {
        let mut result = self;
        result.main = result.main.transpose(level);
        if let Some(base) = result.base.clone() {
            result.base = Some(base.transpose(level));
        }
        result
    }

    pub fn normalize(self, key: &SimpleChord) -> Self {
        let mut result = self;
        result.main = result.main.normalize(key);
        result.base = result.base.clone().map(|base| base.normalize(key));
        result
    }

    pub fn major(self) -> Self {
        let mut result = self;
        result.kind = Kind::Major;
        result
    }

    pub fn minor(self) -> Self {
        let mut result = self;
        result.kind = Kind::Minor;
        result
    }

    pub fn dim(self) -> Self {
        let mut result = self;
        result.kind = Kind::Diminished;
        result
    }

    pub fn aug(self) -> Self {
        let mut result = self;
        result.kind = Kind::Augmented;
        result
    }

    pub fn sus4(self) -> Self {
        let mut result = self;
        result.kind = Kind::Suspended4;
        result
    }

    pub fn sus2(self) -> Self {
        let mut result = self;
        result.kind = Kind::Suspended2;
        result
    }

    pub fn base(self, base: SimpleChord) -> Self {
        let mut result = self;
        result.base = Some(base);
        result
    }

    pub fn var(self, var: String) -> Self {
        let mut result = self;
        result.var = var;
        result
    }

    pub fn duration(self, duration: u32) -> Self {
        let mut result = self;
        result.duration = Some(duration);
        result
    }

    pub fn get_duration(&self) -> Option<u32> {
        self.duration
    }

    pub fn format(&self, key: &SimpleChord, representation: &ChordRepresentation) -> String {
        let (optional_start, optional_end) = if self.optional { ("(", ")") } else { ("", "") };

        let slash = self.base.as_ref().map(|base| {
            let bass = match representation {
                ChordRepresentation::Default => format_slash_bass_default(&self.main, base, key),
                _ => base.format(key, representation),
            };
            format!("/{bass}")
        });

        format!(
            "{}{}{}{}{}{}",
            optional_start,
            self.main.format(key, representation),
            self.kind.format(),
            slash.unwrap_or_default(),
            self.var,
            optional_end,
        )
    }

    fn parse_simple_chord(s: &str) -> Result<(SimpleChord, &str), Error> {
        let l1 = s.chars().next().map_or(0, |c| c.len_utf8());
        let l2 = s.chars().nth(1).map_or(0, |c| c.len_utf8());

        if l2 > 0 && SimpleChord::try_from(&s[..l1 + l2]).is_ok() {
            let chord = SimpleChord::try_from(&s[..l1 + l2])?;
            return Ok((chord, &s[l1 + l2..]));
        }
        if l1 > 0 && SimpleChord::try_from(&s[..l1]).is_ok() {
            let chord = SimpleChord::try_from(&s[..l1])?;
            return Ok((chord, &s[l1..]));
        }
        Err(Error::Parse(format!(
            "can not parse a simple chord from the string: {}",
            s
        )))
    }

    fn parse_kind(s: &str) -> (Kind, &str) {
        if s.len() >= 4 {
            let l4: usize = s.chars().take(4).map(|c| c.len_utf8()).sum();
            match &s[..l4] {
                "sus2" => return (Kind::Suspended2, &s[l4..]),
                "sus4" => return (Kind::Suspended4, &s[l4..]),
                _ => (),
            }
        }

        if s.len() >= 3 {
            let l3: usize = s.chars().take(3).map(|c| c.len_utf8()).sum();
            match &s[..l3] {
                "dim" => return (Kind::Diminished, &s[l3..]),
                "aug" => return (Kind::Augmented, &s[l3..]),
                "sus" => return (Kind::Suspended4, &s[l3..]),
                _ => (),
            }
        }

        if !s.is_empty() {
            let l1 = s.chars().next().map_or(0, |c| c.len_utf8());
            match &s[..l1] {
                "m" => return (Kind::Minor, &s[l1..]),
                "°" => return (Kind::Diminished, &s[l1..]),
                "+" => return (Kind::Augmented, &s[l1..]),
                _ => (),
            }
        }

        (Kind::Major, s)
    }

    fn parse_slash_bass(s: &str) -> Result<Option<SimpleChord>, Error> {
        if s.is_empty() {
            return Ok(None);
        }
        let (chord, rest) = Self::parse_simple_chord(s)?;
        if !rest.is_empty() {
            return Err(Error::Parse(format!(
                "invalid characters after bass note: {rest}",
            )));
        }
        Ok(Some(chord))
    }

    fn parse_var(s: &str) -> (&str, &str) {
        match s.split_once('/') {
            Some((var, s)) => (var, s),
            None => (s, ""),
        }
    }

    fn parse_duration(s: &str) -> Result<(Option<u32>, &str), Error> {
        if let Some((before, after)) = s.split_once(':') {
            let duration_token = after.trim().replace(',', ".");
            let clicks: f64 = duration_token
                .parse()
                .map_err(|_| Error::Parse("failed to parse the duration".into()))?;
            let milliclicks = (clicks * 1000.0).round().max(0.0) as u32;
            return Ok((Some(milliclicks), before));
        }
        Ok((None, s))
    }
}

impl FromStr for Chord {
    type Err = Error;

    fn from_str(mut s: &str) -> Result<Self, Self::Err> {
        let optional = s.starts_with('(') && s.ends_with(')');
        if optional {
            s = &s[1..s.len() - 1];
        }

        let (duration, s) = Self::parse_duration(s)?;
        let (main, s) = Self::parse_simple_chord(s)?;
        let (kind, s) = Self::parse_kind(s);
        let (var, s) = Self::parse_var(s);
        let base = Self::parse_slash_bass(s)?;

        Ok(Self {
            main,
            base,
            kind,
            var: var.to_string(),
            duration,
            optional,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::types::ChordRepresentation;

    fn fmt_default(sym: &str, key: &SimpleChord) -> String {
        Chord::from_str(sym)
            .unwrap_or_else(|e| panic!("parse {sym:?}: {e}"))
            .format(key, &ChordRepresentation::Default)
    }

    fn fmt_nashville(sym: &str, key: &SimpleChord) -> String {
        Chord::from_str(sym)
            .unwrap()
            .format(key, &ChordRepresentation::Nashville)
    }

    fn assert_default(sym: &str, key: &SimpleChord, want: &str) {
        let got = fmt_default(sym, key);
        assert_eq!(
            got,
            want,
            "from_str({sym:?}) with key level {}",
            key.pitch_class()
        );
    }

    #[test]
    fn chord_from_str() {
        let inputs = vec![
            "", "A", "Bb", "C#", "D°", "E+", "Fdim", "Gaug", "Gbaug", "A/B", "C#m/G#", "Asus",
            "Asus4", "Asus2", "A/", "Cadd9", "Cm47/F",
        ];
        let outputs = vec![
            Err(Error::Parse(
                "can not parse a simple chord from the string: ".into(),
            )),
            Ok(Chord::new(0)),
            Ok(Chord::new(1)),
            Ok(Chord::new(4)),
            Ok(Chord::new(5).dim()),
            Ok(Chord::new(7).aug()),
            Ok(Chord::new(8).dim()),
            Ok(Chord::new(10).aug()),
            Ok(Chord::new(9).aug()),
            Ok(Chord::new(0).base(SimpleChord::new(2))),
            Ok(Chord::new(4).minor().base(SimpleChord::new(11))),
            Ok(Chord::new(0).sus4()),
            Ok(Chord::new(0).sus4()),
            Ok(Chord::new(0).sus2()),
            Ok(Chord::new(0)),
            Ok(Chord::new(3).var("add9".into())),
            Ok(Chord::new(3)
                .minor()
                .var("47".into())
                .base('F'.try_into().unwrap())),
        ];

        for (input, output) in inputs.iter().zip(outputs.iter()) {
            assert_eq!(&Chord::from_str(input), output);
        }
    }

    #[test]
    fn chord_duration_milliclicks() {
        let c4 = Chord::from_str("C:4").unwrap();
        assert_eq!(c4.get_duration(), Some(4000), "4 clicks = 4000 milliclicks");

        let am_1_5 = Chord::from_str("Am:1.5").unwrap();
        assert_eq!(
            am_1_5.get_duration(),
            Some(1500),
            "1.5 clicks = 1500 milliclicks"
        );

        let g_2_25 = Chord::from_str("G:2.25").unwrap();
        assert_eq!(g_2_25.get_duration(), Some(2250));

        // Locale-style decimal comma (same values as dot form). See issue #29.
        let am_1_5_comma = Chord::from_str("Am:1,5").unwrap();
        assert_eq!(am_1_5_comma.get_duration(), Some(1500));

        let g_2_25_comma = Chord::from_str("G:2,25").unwrap();
        assert_eq!(g_2_25_comma.get_duration(), Some(2250));

        let paren_comma = Chord::from_str("(C:1,25)").unwrap();
        assert!(paren_comma.optional);
        assert_eq!(paren_comma.get_duration(), Some(1250));
    }

    #[test]
    fn slash_bass_enharmonics_parse_and_format() {
        let c = Chord::from_str("G#/B#").unwrap();
        let key = SimpleChord::default();
        assert_eq!(
            c.format(&key, &ChordRepresentation::Default),
            "G#/B#",
            "default notation preserves written B# bass"
        );
        assert_eq!(
            c.format(&key, &ChordRepresentation::Nashville),
            "7/b3",
            "Nashville uses scale-degree names (not letter bass spellings)"
        );
    }

    #[test]
    fn slash_bass_enharmonics_after_normalize_match_key_c() {
        let c = Chord::from_str("G#/B#").unwrap();
        let key = SimpleChord::try_from("C").unwrap();
        let normalized = c.normalize(&key);
        assert_eq!(
            normalized.format(&key, &ChordRepresentation::Default),
            "G#/B#"
        );
    }

    #[test]
    fn slash_bass_trailing_garbage_is_error() {
        let err = Chord::from_str("C/Gx").unwrap_err();
        assert!(
            err.to_string().contains("invalid characters after bass"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn slash_bass_diminished_fifth_and_major_sixth_spelling() {
        let key = SimpleChord::default();
        assert_eq!(
            Chord::from_str("F/Cb")
                .unwrap()
                .format(&key, &ChordRepresentation::Default),
            "F/Cb",
            "tritone bass: lowered fifth letter (C→Cb)"
        );
        assert_eq!(
            Chord::from_str("Ab/C")
                .unwrap()
                .format(&key, &ChordRepresentation::Default),
            "G#/B#",
            "same pitch class as Ab/C; key C uses sharp root name so the third is B#"
        );
        assert_eq!(
            Chord::from_str("G/Fb")
                .unwrap()
                .format(&key, &ChordRepresentation::Default),
            "G/E",
            "Fb and E share pitch class; sixth is spelled with the diatonic letter E"
        );
    }

    /// `SimpleChord::default()` is level 0 (A in internal encoding). Sharp key branch; combined
    /// roots match treating the song key as A for these assertions.
    #[test]
    fn slash_bass_default_key_c_major_chords() {
        let key = SimpleChord::default();
        for (sym, want) in [
            ("C/D", "C/D"),
            ("C/E", "C/E"),
            ("C/F", "C/F"),
            ("C/G", "C/G"),
            ("C/A", "C/A"),
            ("C/B", "C/B"),
            ("D/E", "D/E"),
            ("D/F#", "D/F#"),
            ("D/G", "D/G"),
            ("D/A", "D/A"),
            ("D/B", "D/B"),
            ("E/F#", "E/F#"),
            ("E/G#", "E/G#"),
            ("E/A", "E/A"),
            ("E/B", "E/B"),
            ("F/G", "F/G"),
            ("F/A", "F/A"),
            ("F/B", "F/Cb"),
            ("F/C", "F/C"),
            ("G/A", "G/A"),
            ("G/B", "G/B"),
            ("G/C", "G/C"),
            ("G/D", "G/D"),
            ("G/F", "G/E#"),
            ("A/B", "A/B"),
            ("A/C#", "A/C#"),
            ("A/D", "A/D"),
            ("A/E", "A/E"),
            ("B/C#", "B/C#"),
            ("B/D#", "B/D#"),
            ("B/E", "B/E"),
            ("B/F#", "B/F#"),
            ("C#/E#", "C#/E#"),
            ("C#/F#", "C#/F#"),
            ("C#/G#", "C#/G#"),
            ("F#/A#", "F#/A#"),
            ("F#/B", "F#/B"),
            ("G#/C", "G#/B#"),
            ("G#/D#", "G#/D#"),
            ("Bb/D", "A#/D"),
            ("Db/F", "C#/E#"),
            ("Eb/G", "D#/G"),
        ] {
            assert_default(sym, &key, want);
        }
    }

    /// ChordPro `{key: C}` uses `SimpleChord` level 3; slash spelling uses combined pitch + key.
    #[test]
    fn slash_bass_song_key_c_try_from_c() {
        let key = SimpleChord::try_from("C").unwrap();
        assert_eq!(key.pitch_class(), 3);
        for (sym, want) in [("C/G", "D#/A#"), ("G/C", "A#/D#"), ("D/A", "F/C")] {
            assert_default(sym, &key, want);
        }
    }

    #[test]
    fn slash_bass_default_key_c_minor_sus_dim_aug() {
        let key = SimpleChord::default();
        for (sym, want) in [
            ("Am/C", "Am/C"),
            ("Am/E", "Am/E"),
            ("Am/G", "Am/G"),
            ("Dm/F", "Dm/F"),
            ("Dm/A", "Dm/A"),
            ("Em/G", "Em/G"),
            ("Em/B", "Em/B"),
            ("Fm/Ab", "Fm/Ab"),
            ("Gm/Bb", "Gm/Bb"),
            ("Asus4/D", "Asus4/D"),
            ("Asus4/E", "Asus4/E"),
            ("Asus2/E", "Asus2/E"),
            ("Dsus4/G", "Dsus4/G"),
            ("Cdim/Gb", "Cdim/Gb"),
            ("Caug/G#", "Caug/Ab"),
            ("Daug/A#", "Daug/Bb"),
        ] {
            assert_default(sym, &key, want);
        }
    }

    #[test]
    fn slash_bass_default_flat_key_f() {
        let key = SimpleChord::try_from("F").unwrap();
        for (sym, want) in [
            ("C/E", "Ab/C"),
            ("F/C", "Db/Ab"),
            ("Bb/F", "Gb/Db"),
            ("Dm/G", "Bbm/Eb"),
        ] {
            assert_default(sym, &key, want);
        }
    }

    #[test]
    fn slash_bass_default_flat_key_db() {
        let key = SimpleChord::try_from("Db").unwrap();
        for (sym, want) in [("Db/F", "F/A"), ("Ab/Eb", "C/G"), ("Eb/G", "G/B")] {
            assert_default(sym, &key, want);
        }
    }

    #[test]
    fn slash_bass_nashville_key_c_table() {
        let key = SimpleChord::default();
        for (sym, want) in [
            ("C/G", "b3/b7"),
            ("D/F#", "4/6"),
            ("G/B", "b7/2"),
            ("Am/C", "1m/b3"),
            ("G#/B#", "7/b3"),
            ("F/Cb", "b6/2"),
        ] {
            assert_eq!(fmt_nashville(sym, &key), want, "{sym}");
        }
    }

    #[test]
    fn slash_bass_optional_parens_and_duration() {
        let key = SimpleChord::default();
        assert_eq!(
            Chord::from_str("(C/G)")
                .unwrap()
                .format(&key, &ChordRepresentation::Default),
            "(C/G)"
        );
        let with_dur = Chord::from_str("C/G:2").unwrap();
        assert_eq!(with_dur.get_duration(), Some(2000));
        assert_eq!(
            with_dur.format(&key, &ChordRepresentation::Default),
            "C/G",
            "Chord::format omits duration; Worship Pro layer adds it"
        );
        let am = Chord::from_str("Am/E:1.5").unwrap();
        assert_eq!(am.get_duration(), Some(1500));
        assert_eq!(am.format(&key, &ChordRepresentation::Default), "Am/E");
    }

    #[test]
    fn slash_bass_parse_rejects_trailing_bass_junk() {
        for bad in ["C/Gx", "D/F#z", "E/G#xy", "Am/Cq", "G/B#extra"] {
            let err = Chord::from_str(bad).unwrap_err();
            assert!(
                err.to_string().contains("invalid characters after bass"),
                "{bad}: {err}"
            );
        }
    }

    #[test]
    fn slash_bass_transpose_then_format_key_c() {
        let key = SimpleChord::default();
        let c = Chord::from_str("C/G").unwrap().transpose(2);
        assert_eq!(c.format(&key, &ChordRepresentation::Default), "D/A");
        let g = Chord::from_str("G/B").unwrap().transpose(5);
        assert_eq!(g.format(&key, &ChordRepresentation::Default), "C/E");
    }

    #[test]
    fn slash_bass_double_normalize_idempotent_format() {
        let key = SimpleChord::default();
        let c = Chord::from_str("C/G").unwrap();
        let once = c.normalize(&key);
        let twice = once.clone().normalize(&key);
        assert_eq!(
            once.format(&key, &ChordRepresentation::Default),
            twice.format(&key, &ChordRepresentation::Default)
        );
    }

    #[test]
    fn chord_serde_json_roundtrip_slash_chords() {
        let key = SimpleChord::default();
        for sym in ["C/G", "G#/B#", "Am/C", "F/Cb", "Asus4/D"] {
            let c = Chord::from_str(sym).unwrap();
            let json = serde_json::to_string(&c).unwrap();
            let back: Chord = serde_json::from_str(&json).unwrap();
            assert_eq!(
                back.format(&key, &ChordRepresentation::Default),
                fmt_default(sym, &key)
            );
        }
    }
}
