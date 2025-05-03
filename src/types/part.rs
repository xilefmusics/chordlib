use serde::{Deserialize, Serialize};
use std::str::FromStr;

use super::{Chord, SimpleChord};
use crate::error::Error;

fn find_first_vowel(text: &str) -> Option<usize> {
    text.char_indices()
        .find(|&(_, c)| {
            matches!(
                c.to_ascii_lowercase(),
                'a' | 'e'
                    | 'i'
                    | 'o'
                    | 'u'
                    | 'ä'
                    | 'ö'
                    | 'ü'
                    | 'á'
                    | 'é'
                    | 'í'
                    | 'ó'
                    | 'ú'
                    | 'à'
                    | 'è'
                    | 'ì'
                    | 'ò'
                    | 'ù'
                    | 'â'
                    | 'ê'
                    | 'î'
                    | 'ô'
                    | 'û'
                    | 'ã'
                    | 'õ'
                    | 'ë'
                    | 'ï'
                    | 'ÿ'
            )
        })
        .map(|(i, _)| i)
}

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct Part {
    pub chord: Option<Chord>,
    pub languages: Vec<String>,
    pub comment: bool,
}

impl Part {
    pub fn move_chord_to_next_vowel(mut self, mut prev: Self) -> (Self, Self) {
        for language in 0..self.languages.len().min(prev.languages.len()) {
            let text = &self.languages[language];

            let first_vowel = match find_first_vowel(text) {
                Some(i) => i,
                None => continue,
            };

            let first_whitespace = text
                .char_indices()
                .find(|&(_, c)| c.is_whitespace())
                .map(|(i, _)| i)
                .unwrap_or_else(|| text.len());

            if first_vowel > first_whitespace {
                continue;
            }

            let (before_vowel, after_vowel) = text.split_at(first_vowel);
            prev.languages[language].push_str(before_vowel);
            self.languages[language] = after_vowel.to_string();
        }

        (self, prev)
    }

    pub fn normalize(&mut self, key: &SimpleChord) -> &mut Self {
        self.chord = self.chord.clone().map(|chord| chord.normalize(key));
        self
    }

    pub fn new_comment(c: String) -> Self {
        Self {
            chord: None,
            languages: vec![c],
            comment: true,
        }
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
            comment: false,
        })
    }
}
