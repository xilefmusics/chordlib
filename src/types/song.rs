use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{Section, SimpleChord};

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct Song {
    /// Titles from `{title}` / `{titleN}`; index 0 is the primary (`title()`).
    #[serde(default)]
    pub titles: Vec<String>,
    pub subtitle: Option<String>,
    pub copyright: Option<String>,
    pub key: Option<SimpleChord>,
    /// Artists from `{artist}` / `{artistN}`; index 0 is the primary (`artist()`).
    #[serde(default)]
    pub artists: Vec<String>,
    /// Language codes from `{language}` / `{languageN}`; index 0 is the primary (`language()`).
    #[serde(default)]
    pub languages: Vec<String>,
    pub tempo: Option<u32>,
    pub time: Option<(u32, u32)>,
    /// Custom tags (e.g. scripture, hymn_type) from ChordPro `{meta: name value}`.
    /// Only read/written by chord pro I/O; other formats ignore this.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub tags: BTreeMap<String, String>,
    pub sections: Vec<Section>,
}

impl Song {
    /// Primary title: first entry in `titles`, or empty when the vector is empty.
    pub fn title(&self) -> &str {
        self.titles.first().map(String::as_str).unwrap_or("")
    }

    /// Primary artist: first entry in `artists`, or empty when the vector is empty.
    pub fn artist(&self) -> &str {
        self.artists.first().map(String::as_str).unwrap_or("")
    }

    /// Primary language code: first entry in `languages`, or empty when the vector is empty.
    pub fn language(&self) -> &str {
        self.languages.first().map(String::as_str).unwrap_or("")
    }

    pub fn transpose(&mut self, key: SimpleChord) -> &mut Self {
        self.key = Some(key);
        self
    }

    pub fn normalize(&mut self) -> &mut Self {
        for section in &mut self.sections {
            if let Some(key) = &self.key {
                section.normalize(key);
            }
        }
        self
    }

    /// Bar duration in milliclicks (1000 per click; one bar in 4/4 = 4000).
    pub fn bar_duration(&self) -> u32 {
        if let Some((numerator, denominator)) = self.time {
            1000 * numerator * 4 / denominator
        } else {
            4000
        }
    }

    pub fn beats_per_bar(&self) -> u32 {
        self.time.map(|(n, _)| n).unwrap_or(4).max(1)
    }

    pub fn chord_duration_to_layout(&self, stored_millibeats: Option<u32>) -> u32 {
        chord_duration_to_layout_milliclicks(
            stored_millibeats,
            self.bar_duration(),
            self.beats_per_bar(),
        )
    }

    /// Returns the most appropriate title for the given language index.
    ///
    /// If `language` is `Some(idx)` and `self.titles` contains a non-empty
    /// title at that index, that title is returned. Otherwise, this falls
    /// back to the primary `title()`.
    pub fn title_for_language(&self, language: Option<usize>) -> &str {
        let idx = language.unwrap_or(0);
        if let Some(candidate) = self.titles.get(idx)
            && !candidate.is_empty()
        {
            return candidate;
        }
        self.title()
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

    /// Returns a slice of artists when `artists` is non-empty.
    pub fn artist_slice(&self) -> Option<&[String]> {
        if self.artists.is_empty() {
            None
        } else {
            Some(self.artists.as_slice())
        }
    }

    /// Returns a clone of language codes when `languages` is non-empty.
    pub fn language_list(&self) -> Option<Vec<String>> {
        if self.languages.is_empty() {
            None
        } else {
            Some(self.languages.clone())
        }
    }

    /// Returns a clone of artists when `artists` is non-empty.
    pub fn artist_list(&self) -> Option<Vec<String>> {
        if self.artists.is_empty() {
            None
        } else {
            Some(self.artists.clone())
        }
    }
}

pub fn chord_duration_to_layout_milliclicks(
    stored_millibeats: Option<u32>,
    bar_duration: u32,
    beats_per_bar: u32,
) -> u32 {
    let beats = beats_per_bar.max(1) as u64;
    let bar = bar_duration as u64;
    match stored_millibeats {
        None => bar_duration,
        Some(mb) => (mb as u64 * bar / (beats * 1000)) as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primary_getters_return_empty_when_vectors_empty() {
        let song = Song::default();
        assert_eq!(song.title(), "");
        assert_eq!(song.artist(), "");
        assert_eq!(song.language(), "");
    }

    #[test]
    fn serde_missing_metadata_vectors_default_empty() {
        let json = r#"{"sections":[]}"#;
        let s: Song = serde_json::from_str(json).unwrap();
        assert!(s.titles.is_empty());
        assert!(s.artists.is_empty());
        assert!(s.languages.is_empty());
    }

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

        let song_68 = Song {
            time: Some((6, 8)),
            ..Song::default()
        };
        assert_eq!(
            song_68.bar_duration(),
            3000,
            "6/8 bar should equal 3/4 bar duration"
        );

        let song_22 = Song {
            time: Some((2, 2)),
            ..Song::default()
        };
        assert_eq!(
            song_22.bar_duration(),
            4000,
            "2/2 bar should equal 4/4 bar duration"
        );

        let song_default = Song::default();
        assert_eq!(song_default.bar_duration(), 4000);
    }

    #[test]
    fn chord_duration_to_layout_respects_time_signature() {
        let song_44 = Song {
            time: Some((4, 4)),
            ..Song::default()
        };
        assert_eq!(song_44.beats_per_bar(), 4);
        assert_eq!(
            chord_duration_to_layout_milliclicks(Some(1000), song_44.bar_duration(), 4),
            1000,
        );
        assert_eq!(
            chord_duration_to_layout_milliclicks(Some(1500), song_44.bar_duration(), 4),
            1500
        );

        let song_68 = Song {
            time: Some((6, 8)),
            ..Song::default()
        };
        assert_eq!(song_68.beats_per_bar(), 6);
        assert_eq!(song_68.bar_duration(), 3000);
        assert_eq!(
            chord_duration_to_layout_milliclicks(Some(1000), 3000, 6),
            500,
        );
        assert_eq!(
            chord_duration_to_layout_milliclicks(Some(500), 3000, 6),
            250,
        );
    }

    #[test]
    fn language_list_returns_none_when_empty() {
        let song = Song::default();
        assert!(song.language_list().is_none());
    }

    #[test]
    fn language_list_clones_non_empty_languages() {
        let song = Song {
            languages: vec!["en".to_string(), "de".to_string(), "fr".to_string()],
            ..Song::default()
        };
        let langs = song.language_list().expect("language list");
        assert_eq!(langs, vec!["en", "de", "fr"]);
    }

    #[test]
    fn title_for_language_prefers_titles_vector_and_falls_back() {
        let song = Song {
            titles: vec![
                "Primary".to_string(),
                "Secondary".to_string(),
                String::new(),
            ],
            ..Song::default()
        };

        // Default / language 0 → primary title.
        assert_eq!(song.title_for_language(None), "Primary");
        assert_eq!(song.title_for_language(Some(0)), "Primary");
        // Language 1 → second title.
        assert_eq!(song.title_for_language(Some(1)), "Secondary");
        // Out-of-range or empty entries fall back to primary.
        assert_eq!(song.title_for_language(Some(2)), "Primary");
        assert_eq!(song.title_for_language(Some(10)), "Primary");
    }

    #[test]
    fn title_for_language_falls_back_when_primary_slot_empty() {
        let song = Song {
            titles: vec![String::new(), "OnlySecond".to_string()],
            ..Song::default()
        };
        assert_eq!(song.title_for_language(None), "");
        assert_eq!(song.title_for_language(Some(1)), "OnlySecond");
    }
}
