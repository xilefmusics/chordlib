use super::FormatChordPro;
use crate::types::{ChordRepresentation, Part, SimpleChord};

impl FormatChordPro for &Part {
    fn format_chord_pro(
        &self,
        key: Option<&SimpleChord>,
        representation: Option<&ChordRepresentation>,
        language: Option<usize>,
        worship_pro_features: bool,
    ) -> String {
        let language = language.unwrap_or(0);
        let lang_text = if worship_pro_features {
            self.text_for_language_exact(language)
        } else {
            self.text_for_language(language)
        };

        let chord = self.chord.as_ref().map_or(String::new(), |chord| {
            format!(
                "[{}]",
                chord.format_chord_pro(key, representation, Some(language), worship_pro_features)
            )
        });

        if self.comment {
            if worship_pro_features {
                format!("{chord}{{comment: {lang_text}}}")
            } else {
                let mut result = String::with_capacity(chord.len() + lang_text.len() + 4);
                result.push_str(&chord);

                if !lang_text.starts_with('(') {
                    result.push('(');
                }
                result.push_str(lang_text);
                if !lang_text.ends_with(')') {
                    result.push(')');
                }
                result
            }
        } else {
            format!("{chord}{lang_text}")
        }
    }
}
