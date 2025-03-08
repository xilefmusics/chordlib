use super::SimpleChord;

#[derive(Clone, PartialEq)]
pub enum Key {
    Chord(SimpleChord),
    Nashville,
}

impl Default for Key {
    fn default() -> Self {
        SimpleChord::default().into()
    }
}

impl From<SimpleChord> for Key {
    fn from(chord: SimpleChord) -> Self {
        Self::Chord(chord)
    }
}
