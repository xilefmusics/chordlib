use serde::{Deserialize, Serialize};
use std::str::FromStr;

use super::SimpleChord;
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
}

impl Chord {
    pub fn new(level: u8) -> Self {
        Self::default().transpose(level)
    }

    pub fn transpose(self: Self, level: u8) -> Self {
        let mut result = self;
        result.main = result.main.transpose(level);
        result.base.clone().map(|base| base.transpose(level));
        result
    }

    pub fn normalize(self: Self, key: &SimpleChord) -> Self {
        let mut result = self;
        result.main = result.main.normalize(&key);
        result.base = result.base.clone().map(|base| base.normalize(&key));
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

    pub fn format(&self, key: SimpleChord) -> String {
        format!(
            "{}{}{}{}",
            self.main.format(&key),
            self.kind.format(),
            self.base
                .clone()
                .map(|base| format!("/{}", base.format(&key)))
                .unwrap_or("".into()),
            self.var
        )
    }

    fn parse_simple_chord<'a>(s: &'a str) -> Result<(SimpleChord, &'a str), Error> {
        let l1 = s.chars().next().map_or(0, |c| c.len_utf8());
        let l2 = s.chars().nth(1).map_or(0, |c| c.len_utf8());

        if l2 > 0 {
            if let Ok(chord) = SimpleChord::try_from(&s[..l1 + l2]) {
                return Ok((chord, &s[l1 + l2..]));
            }
        }
        if l1 > 0 {
            if let Ok(chord) = SimpleChord::try_from(&s[..l1]) {
                return Ok((chord, &s[l1..]));
            }
        }
        Err(Error::Parse(
            "can not parse a simple chord from an empty string".into(),
        ))
    }

    fn parse_kind<'a>(s: &'a str) -> (Kind, &'a str) {
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

        if s.len() >= 1 {
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

    fn parse_base<'a>(s: &'a str) -> Result<(Option<SimpleChord>, &'a str), Error> {
        if s.len() == 0 {
            return Ok((None, s));
        }
        Self::parse_simple_chord(s).map(|(chord, s)| (Some(chord), s))
    }

    fn parse_var<'a>(s: &'a str) -> (&'a str, &'a str) {
        match s.split_once('/') {
            Some((var, s)) => (var, s),
            None => (s, ""),
        }
    }
}

impl FromStr for Chord {
    type Err = Error;

    fn from_str(s_in: &str) -> Result<Self, Self::Err> {
        let s = s_in;
        let (main, s) = Self::parse_simple_chord(s)?;
        let (kind, s) = Self::parse_kind(s);
        let (var, s) = Self::parse_var(s);
        let (base, _) = Self::parse_base(s)?;

        Ok(Self {
            main,
            base,
            kind,
            var: var.to_string(),
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
                "can not parse a simple chord from an empty string".into(),
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
}
