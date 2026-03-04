use serde::{Deserialize, Serialize};

use super::{Section, SimpleChord};

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct Song {
    pub title: String,
    pub subtitle: Option<String>,
    pub copyright: Option<String>,
    pub key: Option<SimpleChord>,
    pub artist: Option<String>,
    pub language: Option<String>,
    pub tempo: Option<u32>,
    pub time: Option<(u32, u32)>,
    pub sections: Vec<Section>,
}

impl Song {
    pub fn transpose(&mut self, key: SimpleChord) -> &mut Self {
        self.key = Some(key);
        self
    }

    pub fn normalize(&mut self) -> &mut Self {
        for section in &mut self.sections {
            if let Some(key) = &self.key {
                section.normalize(&key);
            }
        }
        self
    }

    /// Bar duration in milliclicks (1000 per click; one bar in 4/4 = 4000).
    pub fn bar_duration(&self) -> u32 {
        if let Some((numerator, _denominator)) = self.time {
            1000 * numerator
        } else {
            4000 // 4/4
        }
    }

    pub fn move_chords_to_next_vowels(mut self) -> Self {
        self.sections = self
            .sections
            .into_iter()
            .map(Section::move_chords_to_next_vowels)
            .collect();
        self
    }

    pub fn remove_manual_spacing(mut self) -> Self {
        self.sections = self
            .sections
            .into_iter()
            .map(Section::remove_manual_spacing)
            .collect();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bar_duration_milliclicks() {
        let song_44 = Song {
            time: Some((4, 4)),
            ..Song::default()
        };
        assert_eq!(
            song_44.bar_duration(),
            4000,
            "4/4 bar = 4 clicks = 4000 milliclicks"
        );

        let song_34 = Song {
            time: Some((3, 4)),
            ..Song::default()
        };
        assert_eq!(song_34.bar_duration(), 3000, "3/4 bar = 3000 milliclicks");

        let song_default = Song::default();
        assert_eq!(song_default.bar_duration(), 4000);
    }
}
