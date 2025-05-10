use crate::types::{ChordRepresentation, SimpleChord};

pub trait FormatChordPro {
    fn format_chord_pro(
        &self,
        key: Option<&SimpleChord>,
        representation: Option<&ChordRepresentation>,
        language: Option<usize>,
        worship_pro_features: bool,
    ) -> String;
}
