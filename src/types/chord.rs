use serde::{Deserialize, Serialize};
use std::str::FromStr;

use super::{ChordRepresentation, SimpleChord};
use crate::error::Error;

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

        format!(
            "{}{}{}{}{}{}",
            optional_start,
            self.main.format(key, representation),
            self.kind.format(),
            self.base
                .clone()
                .map(|base| format!("/{}", base.format(key, representation)))
                .unwrap_or_default(),
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

    fn parse_base(s: &str) -> Result<(Option<SimpleChord>, &str), Error> {
        if s.is_empty() {
            return Ok((None, s));
        }
        Self::parse_simple_chord(s).map(|(chord, s)| (Some(chord), s))
    }

    fn parse_var(s: &str) -> (&str, &str) {
        match s.split_once('/') {
            Some((var, s)) => (var, s),
            None => (s, ""),
        }
    }

    /// Parse duration after ':' as clicks (decimal allowed); returns milliclicks (clicks * 1000).
    fn parse_duration(s: &str) -> Result<(Option<u32>, &str), Error> {
        if let Some((before, after)) = s.split_once(':') {
            let clicks: f64 = after
                .trim()
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
        let (base, _) = Self::parse_base(s)?;

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

    /// Duration is stored as milliclicks; format uses clicks (decimal allowed).
    /// See https://github.com/xilefmusics/chordlib/issues/9
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
    }
}
