use serde::{Deserialize, Serialize};

use super::{Section, SimpleChord};

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct Song {
    /// Primary title of the song (typically the first title in the `{title: ...}` directive).
    pub title: String,
    /// Optional list of titles for different languages, in the same order as the `{language: ...}` directive.
    /// When present, index 0 should always match `title`.
    pub titles: Option<Vec<String>>,
    pub subtitle: Option<String>,
    pub copyright: Option<String>,
    pub key: Option<SimpleChord>,
    pub artist: Option<String>,
    /// Optional list of artists; when present, index 0 should usually match `artist`.
    pub artists: Option<Vec<String>>,
    pub language: Option<String>,
    /// Optional list of language codes; when present, index 0 should usually match `language`.
    pub languages: Option<Vec<String>>,
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
                section.normalize(key);
            }
        }
        self
    }

    /// Bar duration in milliclicks (1000 per click; one bar in 4/4 = 4000).
    pub fn bar_duration(&self) -> u32 {
        if let Some((numerator, denominator)) = self.time {
            // Treat time signature as a number of quarter-note beats per bar:
            // beats = numerator * 4 / denominator, then scale to milliclicks.
            1000 * numerator * 4 / denominator
        } else {
            4000 // 4/4
        }
    }

    /// Returns the most appropriate title for the given language index.
    ///
    /// If `language` is `Some(idx)` and `self.titles` contains a non-empty
    /// title at that index, that title is returned. Otherwise, this falls
    /// back to the primary `self.title`.
    pub fn title_for_language(&self, language: Option<usize>) -> &str {
        let idx = language.unwrap_or(0);
        if let Some(titles) = &self.titles
            && let Some(candidate) = titles.get(idx)
            && !candidate.is_empty()
        {
            return candidate;
        }
        &self.title
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

    pub fn language_list(&self) -> Option<Vec<String>> {
        if let Some(langs) = &self.languages {
            if langs.is_empty() {
                return None;
            }
            return Some(langs.clone());
        }

        self.language.as_ref().map(|langs| {
            langs
                .split_whitespace()
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect()
        })
    }

    /// Returns a list of artists, preferring the structured `artists` field when present.
    pub fn artist_list(&self) -> Option<Vec<String>> {
        if let Some(artists) = &self.artists {
            if artists.is_empty() {
                return None;
            }
            return Some(artists.clone());
        }
        self.artist.as_ref().map(|a| vec![a.clone()])
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
    fn language_list_prefers_structured_languages() {
        let song = Song {
            language: Some("legacy should be ignored when vector present".to_string()),
            languages: Some(vec!["en".to_string(), "de".to_string(), "fr".to_string()]),
            ..Song::default()
        };
        let langs = song.language_list().expect("language list");
        assert_eq!(langs, vec!["en", "de", "fr"]);
    }

    #[test]
    fn title_for_language_prefers_titles_vector_and_falls_back() {
        let song = Song {
            title: "Primary".to_string(),
            titles: Some(vec![
                "Primary".to_string(),
                "Secondary".to_string(),
                String::new(),
            ]),
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
}
