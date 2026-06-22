use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::Error;

use super::{Line, Section, SimpleChord};

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

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct SongFlowItem {
    pub title: String,
    pub repeats: u32,
}

impl PartialEq<&str> for SongFlowItem {
    fn eq(&self, other: &&str) -> bool {
        self.title == *other
    }
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

    /// Returns the distinct section names in first-seen order.
    ///
    /// Sections are grouped by exact title. Empty sections only establish the
    /// base title, while later non-empty variants may receive numeric suffixes
    /// like ` [2]` when their content differs.
    pub fn distinct_section_names(&self) -> Vec<String> {
        let mut resolver = SectionNameResolver::new(self);
        let mut distinct = Vec::new();

        for section in &self.sections {
            let resolution = resolver.resolve(section);
            if resolution.is_new {
                distinct.push(resolution.name);
            }
        }

        distinct
    }

    /// Returns the section labels for the full flow of the song, preserving
    /// every section occurrence in order.
    pub fn section_flow_names(&self) -> Vec<SongFlowItem> {
        let mut resolver = SectionNameResolver::new(self);
        let mut flow = Vec::with_capacity(self.sections.len());

        for section in &self.sections {
            let resolution = resolver.resolve(section);
            flow.push(SongFlowItem {
                title: resolution.name,
                repeats: section.repeat_count,
            });
        }

        flow
    }

    pub(crate) fn sections_for_flow(
        &self,
        flow: Option<&[SongFlowItem]>,
    ) -> Result<(Vec<Section>, bool), Error> {
        let Some(flow) = flow.filter(|flow| !flow.is_empty()) else {
            return Ok((self.sections.clone(), false));
        };

        let mut resolver = SectionNameResolver::new(self);
        let mut distinct_sections = BTreeMap::new();

        for section in &self.sections {
            let resolution = resolver.resolve(section);
            let title = resolution.name;
            distinct_sections.entry(title.clone()).or_insert_with(|| {
                let mut distinct_section = section.clone();
                distinct_section.title = title.clone();
                distinct_section
            });
        }

        let mut rendered_sections = Vec::with_capacity(flow.len());
        let mut seen_titles = BTreeSet::new();

        for item in flow {
            if item.repeats < 1 {
                return Err(Error::InvalidSongFlow(format!(
                    "repeat count for section {:?} must be at least 1",
                    item.title
                )));
            }

            let Some(template) = distinct_sections.get(&item.title) else {
                return Err(Error::InvalidSongFlow(format!(
                    "unknown section title {:?}",
                    item.title
                )));
            };

            let mut section = template.clone();
            section.repeat_count = item.repeats;
            if !seen_titles.insert(item.title.clone()) {
                section.lines.clear();
            }
            rendered_sections.push(section);
        }

        Ok((rendered_sections, true))
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

#[derive(Default)]
struct SectionTitleState<'a> {
    variants: Vec<SectionVariant<'a>>,
    emitted_base: bool,
    next_suffix: usize,
}

struct SectionVariant<'a> {
    lines: &'a [Line],
    name: String,
}

struct SectionNameResolution {
    name: String,
    is_new: bool,
}

struct SectionNameResolver<'a> {
    literal_titles: BTreeSet<&'a str>,
    states: BTreeMap<&'a str, SectionTitleState<'a>>,
    used_names: BTreeSet<String>,
}

impl<'a> SectionNameResolver<'a> {
    fn new(song: &'a Song) -> Self {
        Self {
            literal_titles: song
                .sections
                .iter()
                .map(|section| section.title.as_str())
                .collect(),
            states: BTreeMap::new(),
            used_names: BTreeSet::new(),
        }
    }

    fn resolve(&mut self, section: &'a Section) -> SectionNameResolution {
        let title = section.title.as_str();
        let state = self.states.entry(title).or_default();

        if section.lines.is_empty() {
            let is_new = !state.emitted_base;
            if is_new {
                state.emitted_base = true;
                let name = title.to_string();
                self.used_names.insert(name.clone());
                return SectionNameResolution { name, is_new };
            }

            return SectionNameResolution {
                name: title.to_string(),
                is_new,
            };
        }

        if let Some(existing) = state
            .variants
            .iter()
            .find(|variant| variant.lines == section.lines.as_slice())
        {
            return SectionNameResolution {
                name: existing.name.clone(),
                is_new: false,
            };
        }

        let is_first_non_empty_variant = state.variants.is_empty();
        if is_first_non_empty_variant {
            state.variants.push(SectionVariant {
                lines: section.lines.as_slice(),
                name: title.to_string(),
            });

            if !state.emitted_base {
                state.emitted_base = true;
                let name = title.to_string();
                self.used_names.insert(name.clone());
                return SectionNameResolution { name, is_new: true };
            }

            return SectionNameResolution {
                name: title.to_string(),
                is_new: false,
            };
        }

        let mut suffix = state.next_suffix.max(2);
        let name = loop {
            let candidate = format!("{title} [{suffix}]");
            if !self.literal_titles.contains(candidate.as_str())
                && !self.used_names.contains(&candidate)
            {
                break candidate;
            }
            suffix += 1;
        };

        state.next_suffix = suffix + 1;
        state.variants.push(SectionVariant {
            lines: section.lines.as_slice(),
            name: name.clone(),
        });
        self.used_names.insert(name.clone());

        SectionNameResolution { name, is_new: true }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Error;
    use crate::types::{Chord, Line, Part, Section};
    use std::str::FromStr;

    fn text_part(text: &str) -> Part {
        Part {
            chord: None,
            languages: vec![text.to_string()],
            comment: false,
        }
    }

    fn chord_part(chord: &str, text: &str) -> Part {
        Part {
            chord: Some(Chord::from_str(chord).unwrap()),
            languages: vec![text.to_string()],
            comment: false,
        }
    }

    fn comment_part(text: &str) -> Part {
        Part {
            chord: None,
            languages: vec![text.to_string()],
            comment: true,
        }
    }

    fn multilingual_part(texts: &[&str]) -> Part {
        Part {
            chord: None,
            languages: texts.iter().map(|text| (*text).to_string()).collect(),
            comment: false,
        }
    }

    fn text_line(text: &str) -> Line {
        Line::new(vec![text_part(text)])
    }

    fn chord_line(chord: &str, text: &str) -> Line {
        Line::new(vec![chord_part(chord, text)])
    }

    fn comment_line(text: &str) -> Line {
        Line::new(vec![comment_part(text)])
    }

    fn section(title: &str, lines: Vec<Line>) -> Section {
        Section::new(title.to_string(), lines)
    }

    fn section_with_repeat(title: &str, lines: Vec<Line>, repeat_count: u32) -> Section {
        Section::new_with_repeat(title.to_string(), lines, repeat_count)
    }

    fn flow_item(title: &str, repeats: u32) -> SongFlowItem {
        SongFlowItem {
            title: title.to_string(),
            repeats,
        }
    }

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

    #[test]
    fn distinct_section_names_returns_empty_for_song_without_sections() {
        let song = Song::default();
        assert!(song.distinct_section_names().is_empty());
    }

    #[test]
    fn distinct_section_names_returns_single_name_for_single_section() {
        let song = Song {
            sections: vec![section("Chorus", vec![text_line("This is the content")])],
            ..Song::default()
        };

        assert_eq!(song.distinct_section_names(), vec!["Chorus"]);
    }

    #[test]
    fn distinct_section_names_collapses_exact_duplicate_sections_and_keeps_first_order() {
        let song = Song {
            sections: vec![
                section("Chorus", vec![text_line("This is the content")]),
                section("Chorus", vec![text_line("This is the content")]),
                section(
                    "Chorus",
                    vec![text_line("This is the content, but different")],
                ),
                section("Chorus", vec![text_line("This is the content")]),
            ],
            ..Song::default()
        };

        assert_eq!(song.distinct_section_names(), vec!["Chorus", "Chorus [2]"]);
    }

    #[test]
    fn distinct_section_names_ignores_repeat_count_when_contents_match() {
        let song = Song {
            sections: vec![
                section_with_repeat("Verse", vec![text_line("Same line")], 1),
                section_with_repeat("Verse", vec![text_line("Same line")], 4),
            ],
            ..Song::default()
        };

        assert_eq!(song.distinct_section_names(), vec!["Verse"]);
    }

    #[test]
    fn distinct_section_names_treats_empty_sections_as_one_name_only() {
        let song = Song {
            sections: vec![
                section("Intro", vec![]),
                section("Intro", vec![]),
                section("Intro", vec![]),
            ],
            ..Song::default()
        };

        assert_eq!(song.distinct_section_names(), vec!["Intro"]);
    }

    #[test]
    fn distinct_section_names_keeps_unsuffixed_name_when_empty_precedes_content() {
        let song = Song {
            sections: vec![
                section("Bridge", vec![]),
                section("Bridge", vec![text_line("First content")]),
                section("Bridge", vec![text_line("First content")]),
                section("Bridge", vec![text_line("Second content")]),
                section("Bridge", vec![]),
            ],
            ..Song::default()
        };

        assert_eq!(song.distinct_section_names(), vec!["Bridge", "Bridge [2]"]);
    }

    #[test]
    fn distinct_section_names_handles_empty_sections_between_content_variants() {
        let song = Song {
            sections: vec![
                section("Chorus", vec![text_line("Alpha")]),
                section("Chorus", vec![]),
                section("Chorus", vec![text_line("Beta")]),
                section("Chorus", vec![]),
                section("Chorus", vec![text_line("Beta")]),
            ],
            ..Song::default()
        };

        assert_eq!(song.distinct_section_names(), vec!["Chorus", "Chorus [2]"]);
    }

    #[test]
    fn distinct_section_names_compares_sections_structurally_across_line_details() {
        let song = Song {
            sections: vec![
                section("Verse", vec![text_line("Same text")]),
                section("Verse", vec![chord_line("C", "Same text")]),
                section("Verse", vec![comment_line("Same text")]),
                section(
                    "Verse",
                    vec![Line::new(vec![text_part("Same"), text_part(" text")])],
                ),
                section(
                    "Verse",
                    vec![Line::new(vec![text_part("Same text"), text_part(" extra")])],
                ),
            ],
            ..Song::default()
        };

        assert_eq!(
            song.distinct_section_names(),
            vec!["Verse", "Verse [2]", "Verse [3]", "Verse [4]", "Verse [5]"]
        );
    }

    #[test]
    fn distinct_section_names_treats_translation_changes_as_distinct() {
        let song = Song {
            sections: vec![
                section(
                    "Verse",
                    vec![Line::new(vec![multilingual_part(&["Deutsch", "English"])])],
                ),
                section(
                    "Verse",
                    vec![Line::new(vec![multilingual_part(&["Deutsch", "Français"])])],
                ),
            ],
            ..Song::default()
        };

        assert_eq!(song.distinct_section_names(), vec!["Verse", "Verse [2]"]);
    }

    #[test]
    fn distinct_section_names_treats_line_count_changes_as_distinct() {
        let song = Song {
            sections: vec![
                section("Verse", vec![text_line("One")]),
                section("Verse", vec![text_line("One"), text_line("Two")]),
            ],
            ..Song::default()
        };

        assert_eq!(song.distinct_section_names(), vec!["Verse", "Verse [2]"]);
    }

    #[test]
    fn distinct_section_names_treats_different_line_order_as_distinct() {
        let song = Song {
            sections: vec![
                section("Verse", vec![text_line("One"), text_line("Two")]),
                section("Verse", vec![text_line("Two"), text_line("One")]),
            ],
            ..Song::default()
        };

        assert_eq!(song.distinct_section_names(), vec!["Verse", "Verse [2]"]);
    }

    #[test]
    fn distinct_section_names_keeps_same_content_under_different_titles_separate() {
        let song = Song {
            sections: vec![
                section("Verse", vec![text_line("Shared")]),
                section("Chorus", vec![text_line("Shared")]),
            ],
            ..Song::default()
        };

        assert_eq!(song.distinct_section_names(), vec!["Verse", "Chorus"]);
    }

    #[test]
    fn distinct_section_names_skips_collisions_with_literal_suffixed_titles() {
        let song = Song {
            sections: vec![
                section("Chorus", vec![text_line("Alpha")]),
                section("Chorus [2]", vec![text_line("Literal two")]),
                section("Chorus", vec![text_line("Beta")]),
            ],
            ..Song::default()
        };

        assert_eq!(
            song.distinct_section_names(),
            vec!["Chorus", "Chorus [2]", "Chorus [3]"]
        );
    }

    #[test]
    fn distinct_section_names_skips_future_literal_collisions_when_numbering_variants() {
        let song = Song {
            sections: vec![
                section("Chorus", vec![text_line("Alpha")]),
                section("Chorus", vec![text_line("Beta")]),
                section("Chorus [2]", vec![text_line("Literal two")]),
                section("Chorus [3]", vec![text_line("Literal three")]),
            ],
            ..Song::default()
        };

        assert_eq!(
            song.distinct_section_names(),
            vec!["Chorus", "Chorus [4]", "Chorus [2]", "Chorus [3]"]
        );
    }

    #[test]
    fn distinct_section_names_skips_multiple_occupied_suffixes() {
        let song = Song {
            sections: vec![
                section("Verse", vec![text_line("Alpha")]),
                section("Verse [2]", vec![text_line("Literal two")]),
                section("Verse [3]", vec![text_line("Literal three")]),
                section("Verse", vec![text_line("Beta")]),
            ],
            ..Song::default()
        };

        assert_eq!(
            song.distinct_section_names(),
            vec!["Verse", "Verse [2]", "Verse [3]", "Verse [4]"]
        );
    }

    #[test]
    fn distinct_section_names_is_case_sensitive_and_handles_empty_titles() {
        let song = Song {
            sections: vec![
                section("", vec![text_line("Blank title content")]),
                section("", vec![text_line("Different blank title content")]),
                section("chorus", vec![text_line("lowercase")]),
                section("Chorus", vec![text_line("uppercase")]),
            ],
            ..Song::default()
        };

        assert_eq!(
            song.distinct_section_names(),
            vec!["", " [2]", "chorus", "Chorus"]
        );
    }

    #[test]
    fn distinct_section_names_does_not_mutate_song() {
        let song = Song {
            sections: vec![
                section("Verse", vec![text_line("Alpha")]),
                section("Verse", vec![text_line("Beta")]),
                section("Verse [2]", vec![text_line("Literal two")]),
            ],
            ..Song::default()
        };
        let original = song.clone();

        let names = song.distinct_section_names();

        assert_eq!(song, original);
        assert_eq!(names, vec!["Verse", "Verse [3]", "Verse [2]"]);
    }

    #[test]
    fn section_flow_names_returns_empty_for_song_without_sections() {
        let song = Song::default();
        assert!(song.section_flow_names().is_empty());
    }

    #[test]
    fn section_flow_names_returns_single_name_for_single_section() {
        let song = Song {
            sections: vec![section("Chorus", vec![text_line("This is the content")])],
            ..Song::default()
        };

        assert_eq!(song.section_flow_names(), vec!["Chorus"]);
    }

    #[test]
    fn section_flow_names_reuses_name_for_exact_duplicate_sections() {
        let song = Song {
            sections: vec![
                section("Chorus", vec![text_line("This is the content")]),
                section("Chorus", vec![text_line("This is the content")]),
                section(
                    "Chorus",
                    vec![text_line("This is the content, but different")],
                ),
                section("Chorus", vec![text_line("This is the content")]),
            ],
            ..Song::default()
        };

        assert_eq!(
            song.section_flow_names(),
            vec!["Chorus", "Chorus", "Chorus [2]", "Chorus"]
        );
    }

    #[test]
    fn section_flow_names_ignores_repeat_count_when_contents_match() {
        let song = Song {
            sections: vec![
                section_with_repeat("Verse", vec![text_line("Same line")], 1),
                section_with_repeat("Verse", vec![text_line("Same line")], 4),
            ],
            ..Song::default()
        };

        assert_eq!(
            song.section_flow_names(),
            vec![flow_item("Verse", 1), flow_item("Verse", 4)]
        );
    }

    #[test]
    fn section_flow_names_copies_repeat_counts_for_default_flow() {
        let song = Song {
            sections: vec![
                section_with_repeat("Verse", vec![text_line("One")], 2),
                section_with_repeat("Chorus", vec![text_line("Two")], 3),
            ],
            ..Song::default()
        };

        assert_eq!(
            song.section_flow_names(),
            vec![flow_item("Verse", 2), flow_item("Chorus", 3)]
        );
    }

    #[test]
    fn section_flow_names_keeps_empty_sections_in_flow_without_creating_variants() {
        let song = Song {
            sections: vec![
                section("Intro", vec![]),
                section("Intro", vec![]),
                section("Intro", vec![text_line("First content")]),
                section("Intro", vec![]),
            ],
            ..Song::default()
        };

        assert_eq!(
            song.section_flow_names(),
            vec!["Intro", "Intro", "Intro", "Intro"]
        );
    }

    #[test]
    fn section_flow_names_keeps_unsuffixed_name_when_empty_precedes_content() {
        let song = Song {
            sections: vec![
                section("Bridge", vec![]),
                section("Bridge", vec![text_line("First content")]),
                section("Bridge", vec![text_line("First content")]),
                section("Bridge", vec![text_line("Second content")]),
                section("Bridge", vec![]),
            ],
            ..Song::default()
        };

        assert_eq!(
            song.section_flow_names(),
            vec!["Bridge", "Bridge", "Bridge", "Bridge [2]", "Bridge"]
        );
    }

    #[test]
    fn section_flow_names_handles_empty_sections_between_content_variants() {
        let song = Song {
            sections: vec![
                section("Chorus", vec![text_line("Alpha")]),
                section("Chorus", vec![]),
                section("Chorus", vec![text_line("Beta")]),
                section("Chorus", vec![]),
                section("Chorus", vec![text_line("Beta")]),
            ],
            ..Song::default()
        };

        assert_eq!(
            song.section_flow_names(),
            vec!["Chorus", "Chorus", "Chorus [2]", "Chorus", "Chorus [2]"]
        );
    }

    #[test]
    fn section_flow_names_compares_sections_structurally_across_line_details() {
        let song = Song {
            sections: vec![
                section("Verse", vec![text_line("Same text")]),
                section("Verse", vec![chord_line("C", "Same text")]),
                section("Verse", vec![comment_line("Same text")]),
                section(
                    "Verse",
                    vec![Line::new(vec![text_part("Same"), text_part(" text")])],
                ),
                section(
                    "Verse",
                    vec![Line::new(vec![text_part("Same text"), text_part(" extra")])],
                ),
            ],
            ..Song::default()
        };

        assert_eq!(
            song.section_flow_names(),
            vec!["Verse", "Verse [2]", "Verse [3]", "Verse [4]", "Verse [5]"]
        );
    }

    #[test]
    fn section_flow_names_treats_translation_changes_as_distinct() {
        let song = Song {
            sections: vec![
                section(
                    "Verse",
                    vec![Line::new(vec![multilingual_part(&["Deutsch", "English"])])],
                ),
                section(
                    "Verse",
                    vec![Line::new(vec![multilingual_part(&["Deutsch", "Français"])])],
                ),
            ],
            ..Song::default()
        };

        assert_eq!(song.section_flow_names(), vec!["Verse", "Verse [2]"]);
    }

    #[test]
    fn section_flow_names_treats_line_count_changes_as_distinct() {
        let song = Song {
            sections: vec![
                section("Verse", vec![text_line("One")]),
                section("Verse", vec![text_line("One"), text_line("Two")]),
            ],
            ..Song::default()
        };

        assert_eq!(song.section_flow_names(), vec!["Verse", "Verse [2]"]);
    }

    #[test]
    fn section_flow_names_treats_different_line_order_as_distinct() {
        let song = Song {
            sections: vec![
                section("Verse", vec![text_line("One"), text_line("Two")]),
                section("Verse", vec![text_line("Two"), text_line("One")]),
            ],
            ..Song::default()
        };

        assert_eq!(song.section_flow_names(), vec!["Verse", "Verse [2]"]);
    }

    #[test]
    fn section_flow_names_keeps_same_content_under_different_titles_separate() {
        let song = Song {
            sections: vec![
                section("Verse", vec![text_line("Shared")]),
                section("Chorus", vec![text_line("Shared")]),
            ],
            ..Song::default()
        };

        assert_eq!(song.section_flow_names(), vec!["Verse", "Chorus"]);
    }

    #[test]
    fn section_flow_names_skips_collisions_with_literal_suffixed_titles() {
        let song = Song {
            sections: vec![
                section("Chorus", vec![text_line("Alpha")]),
                section("Chorus [2]", vec![text_line("Literal two")]),
                section("Chorus", vec![text_line("Beta")]),
            ],
            ..Song::default()
        };

        assert_eq!(
            song.section_flow_names(),
            vec!["Chorus", "Chorus [2]", "Chorus [3]"]
        );
    }

    #[test]
    fn section_flow_names_skips_multiple_occupied_suffixes() {
        let song = Song {
            sections: vec![
                section("Verse", vec![text_line("Alpha")]),
                section("Verse [2]", vec![text_line("Literal two")]),
                section("Verse [3]", vec![text_line("Literal three")]),
                section("Verse", vec![text_line("Beta")]),
            ],
            ..Song::default()
        };

        assert_eq!(
            song.section_flow_names(),
            vec!["Verse", "Verse [2]", "Verse [3]", "Verse [4]"]
        );
    }

    #[test]
    fn section_flow_names_is_case_sensitive_and_handles_empty_titles() {
        let song = Song {
            sections: vec![
                section("", vec![text_line("Blank title content")]),
                section("", vec![text_line("Different blank title content")]),
                section("chorus", vec![text_line("lowercase")]),
                section("Chorus", vec![text_line("uppercase")]),
            ],
            ..Song::default()
        };

        assert_eq!(
            song.section_flow_names(),
            vec!["", " [2]", "chorus", "Chorus"]
        );
    }

    #[test]
    fn section_flow_names_does_not_mutate_song() {
        let song = Song {
            sections: vec![
                section("Verse", vec![text_line("Alpha")]),
                section("Verse", vec![text_line("Beta")]),
                section("Verse [2]", vec![text_line("Literal two")]),
            ],
            ..Song::default()
        };
        let original = song.clone();

        let flow = song.section_flow_names();

        assert_eq!(song, original);
        assert_eq!(flow, vec!["Verse", "Verse [3]", "Verse [2]"]);
    }

    #[test]
    fn sections_for_flow_reuses_first_body_and_emits_empty_references() {
        let song = Song {
            sections: vec![
                section("Verse", vec![text_line("First line")]),
                section("Chorus", vec![text_line("Second line")]),
            ],
            ..Song::default()
        };
        let flow = vec![
            flow_item("Verse", 1),
            flow_item("Verse", 2),
            flow_item("Chorus", 1),
        ];

        let (sections, custom) = song.sections_for_flow(Some(&flow)).expect("flow");
        assert!(custom);
        assert_eq!(sections.len(), 3);
        assert_eq!(sections[0].title, "Verse");
        assert_eq!(sections[0].lines, vec![text_line("First line")]);
        assert_eq!(sections[0].repeat_count, 1);
        assert!(sections[1].lines.is_empty());
        assert_eq!(sections[1].repeat_count, 2);
        assert_eq!(sections[2].title, "Chorus");
        assert_eq!(sections[2].lines, vec![text_line("Second line")]);
    }

    #[test]
    fn sections_for_flow_rejects_unknown_titles_and_zero_repeats() {
        let song = Song {
            sections: vec![section("Verse", vec![text_line("One")])],
            ..Song::default()
        };

        let unknown = song
            .sections_for_flow(Some(&[flow_item("Missing", 1)]))
            .expect_err("unknown title should fail");
        assert!(matches!(unknown, Error::InvalidSongFlow(_)));

        let invalid_repeats = song
            .sections_for_flow(Some(&[flow_item("Verse", 0)]))
            .expect_err("repeat zero should fail");
        assert!(matches!(invalid_repeats, Error::InvalidSongFlow(_)));
    }
}
