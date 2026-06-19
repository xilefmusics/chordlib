use serde::{Deserialize, Serialize};
use std::str::FromStr;

use super::{Chord, SimpleChord};
use crate::error::Error;
use crate::text::normalize_space_separators;

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

fn normalize_whitespace(input: &str) -> String {
    let input = normalize_space_separators(input);
    let input = input.as_ref();
    let mut result = String::with_capacity(input.len());
    let mut in_whitespace = false;

    for c in input.chars() {
        if c.is_whitespace() {
            if !in_whitespace {
                result.push(' ');
                in_whitespace = true;
            }
        } else {
            result.push(c);
            in_whitespace = false;
        }
    }

    result
}

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct Part {
    pub chord: Option<Chord>,
    pub languages: Vec<String>,
    pub comment: bool,
}

impl Part {
    /// Returns the text for a language index, falling back to the first language.
    ///
    /// If the requested slot is missing or empty, the primary language (index 0)
    /// is used instead. This keeps partially translated lines visible rather than
    /// rendering them as blanks.
    pub fn text_for_language(&self, language: usize) -> &str {
        self.languages
            .get(language)
            .filter(|text| !text.is_empty())
            .map(String::as_str)
            .or_else(|| self.languages.first().map(String::as_str))
            .unwrap_or("")
    }

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

    pub fn remove_manual_spacing(
        mut self,
        mut prev: Option<Self>,
        mut next: Option<Self>,
    ) -> (Self, Option<Self>, Option<Self>) {
        for language in 0..self
            .languages
            .len()
            .min(
                prev.as_ref()
                    .map(|p| p.languages.len())
                    .unwrap_or(usize::MAX),
            )
            .min(
                next.as_ref()
                    .map(|p| p.languages.len())
                    .unwrap_or(usize::MAX),
            )
        {
            self.languages[language] = self.languages[language].replace('\t', " ");
            self.languages[language] = self.languages[language].replace(" - ", "");
            self.languages[language] = normalize_whitespace(&self.languages[language]);

            if prev.is_none() {
                self.languages[language] = self.languages[language].trim_start().to_string();
            }

            if let Some(prev) = prev.as_mut()
                && self.languages[language].starts_with("- ")
                && prev.languages[language].ends_with(' ')
            {
                let new_len = prev.languages[language].len() - 1;
                prev.languages[language].truncate(new_len);
                self.languages[language] = self.languages[language].split_off(2)
            }

            if let Some(next) = next.as_mut()
                && self.languages[language].starts_with("- ")
                && next.languages[language].ends_with(' ')
            {
                let new_len = self.languages[language].len() - 2;
                self.languages[language].truncate(new_len);
                next.languages[language] = next.languages[language].split_off(2)
            }
        }

        (self, prev, next)
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

    pub fn new_chord(c: Chord) -> Self {
        Self {
            chord: Some(c),
            languages: vec![],
            comment: false,
        }
    }
}

impl TryFrom<(&str, &str)> for Part {
    type Error = Error;

    fn try_from(value: (&str, &str)) -> Result<Self, Self::Error> {
        Ok(Self {
            chord: if value.0.is_empty() {
                None
            } else {
                Some(Chord::from_str(value.0)?)
            },
            languages: vec![value.1.to_string()],
            comment: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_for_language_uses_requested_language_when_present() {
        let part = Part {
            chord: None,
            languages: vec!["Hallo".to_string(), "Hello".to_string()],
            comment: false,
        };

        assert_eq!(part.text_for_language(1), "Hello");
    }

    #[test]
    fn text_for_language_falls_back_to_primary_when_missing() {
        let part = Part {
            chord: None,
            languages: vec!["Hallo".to_string()],
            comment: false,
        };

        assert_eq!(part.text_for_language(1), "Hallo");
    }

    #[test]
    fn text_for_language_falls_back_to_primary_when_requested_slot_is_empty() {
        let part = Part {
            chord: None,
            languages: vec!["Hallo".to_string(), String::new()],
            comment: false,
        };

        assert_eq!(part.text_for_language(1), "Hallo");
    }

    #[test]
    fn text_for_language_returns_empty_when_no_text_exists() {
        let part = Part {
            chord: None,
            languages: vec![String::new(), String::new()],
            comment: false,
        };

        assert_eq!(part.text_for_language(1), "");
    }
}
