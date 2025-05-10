use super::FormatChordPro;
use crate::types::{Chord, ChordRepresentation, SimpleChord};

impl FormatChordPro for &Chord {
    fn format_chord_pro(
        &self,
        key: Option<&SimpleChord>,
        representation: Option<&ChordRepresentation>,
        _: Option<usize>,
        worship_pro_features: bool,
    ) -> String {
        let formatted = self.format(
            key.unwrap_or(&SimpleChord::default()),
            representation.unwrap_or(&ChordRepresentation::default()),
        );

        if worship_pro_features {
            if let Some(duration) = self.get_duration() {
                return format!("{formatted}:{duration}");
            }
        }

        formatted
    }
}
