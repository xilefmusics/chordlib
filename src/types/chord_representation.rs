use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChordRepresentation {
    #[default]
    Default,
    Nashville,
}

impl fmt::Display for ChordRepresentation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ChordRepresentation::Default => "default",
            ChordRepresentation::Nashville => "nashville",
        };
        write!(f, "{}", s)
    }
}
