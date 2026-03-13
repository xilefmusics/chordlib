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

        if worship_pro_features
            && let Some(milliclicks) = self.get_duration()
        {
            let clicks = milliclicks as f64 / 1000.0;
            let duration_str = if (clicks - clicks.round()).abs() < f64::EPSILON {
                format!("{:.0}", clicks)
            } else {
                format!("{clicks}")
            };
            return format!("{formatted}:{duration_str}");
        }

        formatted
    }
}
