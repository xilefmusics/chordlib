use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::Error;

use super::{Line, Section, SimpleChord, SongFlowItem};

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct Song {
    #[serde(default)]
    pub titles: Vec<String>,
    pub subtitle: Option<String>,
    pub copyright: Option<String>,
    pub key: Option<SimpleChord>,
    #[serde(default)]
    pub artists: Vec<String>,
    #[serde(default)]
    pub languages: Vec<String>,
    pub tempo: Option<u32>,
    pub time: Option<(u32, u32)>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub tags: BTreeMap<String, String>,
    pub sections: Vec<Section>,
}

impl Song {
    pub fn title(&self) -> &str {
        self.titles.first().map(String::as_str).unwrap_or("")
    }

    pub fn artist(&self) -> &str {
        self.artists.first().map(String::as_str).unwrap_or("")
    }

    pub fn language(&self) -> &str {
        self.languages.first().map(String::as_str).unwrap_or("")
    }

    pub fn apply_key(&mut self, key: SimpleChord) -> &mut Self {
        self.key = Some(key);
        self
    }

    pub fn apply_flow(&mut self, flow: impl Into<Vec<SongFlowItem>>) -> Result<&mut Self, Error> {
        let flow = flow.into();
        if !flow.is_empty() {
            self.sections = self.sections_for_flow(&flow)?;
        }
        Ok(self)
    }

    pub fn fill_section_references(&mut self) -> &mut Self {
        let mut resolver = SectionNameResolver::new(self);
        let mut bodies: BTreeMap<(String, u32), Vec<Line>> = BTreeMap::new();
        let mut fills = Vec::new();

        for (index, section) in self.sections.iter().enumerate() {
            let resolution = resolver.resolve(section);
            let key = (resolution.title.clone(), resolution.occurrence_index);

            if section.lines.is_empty() {
                if bodies.contains_key(&key) {
                    fills.push((index, key));
                }
            } else {
                bodies.entry(key).or_insert_with(|| section.lines.clone());
            }
        }

        for (index, key) in fills {
            if let Some(lines) = bodies.get(&key) {
                self.sections[index].lines.clone_from(lines);
            }
        }

        self
    }

    pub fn normalize(&mut self) -> &mut Self {
        if let Some(key) = &self.key {
            for section in &mut self.sections {
                section.normalize(key);
            }
        }
        self
    }

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

    pub fn flow_items(&self) -> Vec<SongFlowItem> {
        let mut resolver = SectionNameResolver::new(self);
        let mut distinct = Vec::new();

        for section in &self.sections {
            let resolution = resolver.resolve(section);
            if resolution.is_new {
                distinct.push(SongFlowItem {
                    title: resolution.title,
                    occurrence_index: resolution.occurrence_index,
                    repeats: section.repeat_count,
                });
            }
        }

        distinct
    }

    pub fn custom_flow(&self) -> Vec<SongFlowItem> {
        let mut resolver = SectionNameResolver::new(self);
        let mut flow = Vec::with_capacity(self.sections.len());

        for section in &self.sections {
            let resolution = resolver.resolve(section);
            flow.push(SongFlowItem {
                title: resolution.title,
                occurrence_index: resolution.occurrence_index,
                repeats: section.repeat_count,
            });
        }

        flow
    }

    fn sections_for_flow(&self, flow: &[SongFlowItem]) -> Result<Vec<Section>, Error> {
        let mut resolver = SectionNameResolver::new(self);
        let mut distinct_sections = BTreeMap::new();

        for section in &self.sections {
            let resolution = resolver.resolve(section);
            let key = (resolution.title.clone(), resolution.occurrence_index);
            distinct_sections.entry(key).or_insert_with(|| {
                let mut distinct_section = section.clone();
                distinct_section.title = resolution.title;
                distinct_section
            });
        }

        let mut rendered_sections = Vec::with_capacity(flow.len());
        let mut seen_keys = BTreeSet::new();

        for item in flow {
            if item.repeats < 1 {
                return Err(Error::InvalidSongFlow(format!(
                    "repeat count for section {:?} (occurrence {}) must be at least 1",
                    item.title, item.occurrence_index
                )));
            }

            let key = (item.title.clone(), item.occurrence_index);
            let Some(template) = distinct_sections.get(&key) else {
                return Err(Error::InvalidSongFlow(format!(
                    "unknown section {:?} (occurrence {})",
                    item.title, item.occurrence_index
                )));
            };

            let mut section = template.clone();
            section.repeat_count = item.repeats;
            if !seen_keys.insert(key) {
                section.lines.clear();
            }
            rendered_sections.push(section);
        }

        Ok(rendered_sections)
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
}

struct SectionVariant<'a> {
    lines: &'a [Line],
    occurrence_index: u32,
}

struct SectionNameResolution {
    title: String,
    occurrence_index: u32,
    is_new: bool,
}

struct SectionNameResolver<'a> {
    states: BTreeMap<&'a str, SectionTitleState<'a>>,
}

impl<'a> SectionNameResolver<'a> {
    fn new(_song: &'a Song) -> Self {
        Self {
            states: BTreeMap::new(),
        }
    }

    fn resolve(&mut self, section: &'a Section) -> SectionNameResolution {
        let title = section.title.as_str();
        let state = self.states.entry(title).or_default();
        let base_title = title.to_string();

        if section.lines.is_empty() {
            let is_new = !state.emitted_base;
            if is_new {
                state.emitted_base = true;
            }

            return SectionNameResolution {
                title: base_title,
                occurrence_index: 0,
                is_new,
            };
        }

        if let Some(existing) = state
            .variants
            .iter()
            .find(|variant| variant.lines == section.lines.as_slice())
        {
            return SectionNameResolution {
                title: base_title,
                occurrence_index: existing.occurrence_index,
                is_new: false,
            };
        }

        let occurrence_index = state.variants.len() as u32;
        state.variants.push(SectionVariant {
            lines: section.lines.as_slice(),
            occurrence_index,
        });

        let is_new = if occurrence_index == 0 {
            if !state.emitted_base {
                state.emitted_base = true;
                true
            } else {
                false
            }
        } else {
            true
        };

        SectionNameResolution {
            title: base_title,
            occurrence_index,
            is_new,
        }
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

    fn flow_item(title: &str, occurrence_index: u32, repeats: u32) -> SongFlowItem {
        SongFlowItem {
            title: title.to_string(),
            occurrence_index,
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
    fn flow_items_returns_empty_for_song_without_sections() {
        let song = Song::default();
        assert!(song.flow_items().is_empty());
    }

    #[test]
    fn flow_items_returns_single_name_for_single_section() {
        let song = Song {
            sections: vec![section("Chorus", vec![text_line("This is the content")])],
            ..Song::default()
        };

        assert_eq!(song.flow_items(), vec![flow_item("Chorus", 0, 1)]);
    }

    #[test]
    fn flow_items_collapses_exact_duplicate_sections_and_keeps_first_order() {
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
            song.flow_items(),
            vec![flow_item("Chorus", 0, 1), flow_item("Chorus", 1, 1)]
        );
    }

    #[test]
    fn flow_items_ignores_repeat_count_when_contents_match() {
        let song = Song {
            sections: vec![
                section_with_repeat("Verse", vec![text_line("Same line")], 1),
                section_with_repeat("Verse", vec![text_line("Same line")], 4),
            ],
            ..Song::default()
        };

        assert_eq!(song.flow_items(), vec![flow_item("Verse", 0, 1)]);
    }

    #[test]
    fn flow_items_treats_empty_sections_as_one_name_only() {
        let song = Song {
            sections: vec![
                section("Intro", vec![]),
                section("Intro", vec![]),
                section("Intro", vec![]),
            ],
            ..Song::default()
        };

        assert_eq!(song.flow_items(), vec![flow_item("Intro", 0, 1)]);
    }

    #[test]
    fn flow_items_keeps_unsuffixed_name_when_empty_precedes_content() {
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
            song.flow_items(),
            vec![flow_item("Bridge", 0, 1), flow_item("Bridge", 1, 1)]
        );
    }

    #[test]
    fn flow_items_handles_empty_sections_between_content_variants() {
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
            song.flow_items(),
            vec![flow_item("Chorus", 0, 1), flow_item("Chorus", 1, 1)]
        );
    }

    #[test]
    fn flow_items_compares_sections_structurally_across_line_details() {
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
            song.flow_items(),
            vec![
                flow_item("Verse", 0, 1),
                flow_item("Verse", 1, 1),
                flow_item("Verse", 2, 1),
                flow_item("Verse", 3, 1),
                flow_item("Verse", 4, 1),
            ]
        );
    }

    #[test]
    fn flow_items_treats_translation_changes_as_distinct() {
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

        assert_eq!(
            song.flow_items(),
            vec![flow_item("Verse", 0, 1), flow_item("Verse", 1, 1)]
        );
    }

    #[test]
    fn flow_items_treats_line_count_changes_as_distinct() {
        let song = Song {
            sections: vec![
                section("Verse", vec![text_line("One")]),
                section("Verse", vec![text_line("One"), text_line("Two")]),
            ],
            ..Song::default()
        };

        assert_eq!(
            song.flow_items(),
            vec![flow_item("Verse", 0, 1), flow_item("Verse", 1, 1)]
        );
    }

    #[test]
    fn flow_items_treats_different_line_order_as_distinct() {
        let song = Song {
            sections: vec![
                section("Verse", vec![text_line("One"), text_line("Two")]),
                section("Verse", vec![text_line("Two"), text_line("One")]),
            ],
            ..Song::default()
        };

        assert_eq!(
            song.flow_items(),
            vec![flow_item("Verse", 0, 1), flow_item("Verse", 1, 1)]
        );
    }

    #[test]
    fn flow_items_keeps_same_content_under_different_titles_separate() {
        let song = Song {
            sections: vec![
                section("Verse", vec![text_line("Shared")]),
                section("Chorus", vec![text_line("Shared")]),
            ],
            ..Song::default()
        };

        assert_eq!(
            song.flow_items(),
            vec![flow_item("Verse", 0, 1), flow_item("Chorus", 0, 1)]
        );
    }

    #[test]
    fn flow_items_skips_collisions_with_literal_suffixed_titles() {
        let song = Song {
            sections: vec![
                section("Chorus", vec![text_line("Alpha")]),
                section("Chorus [2]", vec![text_line("Literal two")]),
                section("Chorus", vec![text_line("Beta")]),
            ],
            ..Song::default()
        };

        assert_eq!(
            song.flow_items(),
            vec![
                flow_item("Chorus", 0, 1),
                flow_item("Chorus [2]", 0, 1),
                flow_item("Chorus", 1, 1),
            ]
        );
    }

    #[test]
    fn flow_items_skips_future_literal_collisions_when_numbering_variants() {
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
            song.flow_items(),
            vec![
                flow_item("Chorus", 0, 1),
                flow_item("Chorus", 1, 1),
                flow_item("Chorus [2]", 0, 1),
                flow_item("Chorus [3]", 0, 1),
            ]
        );
    }

    #[test]
    fn flow_items_skips_multiple_occupied_suffixes() {
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
            song.flow_items(),
            vec![
                flow_item("Verse", 0, 1),
                flow_item("Verse [2]", 0, 1),
                flow_item("Verse [3]", 0, 1),
                flow_item("Verse", 1, 1),
            ]
        );
    }

    #[test]
    fn flow_items_is_case_sensitive_and_handles_empty_titles() {
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
            song.flow_items(),
            vec![
                flow_item("", 0, 1),
                flow_item("", 1, 1),
                flow_item("chorus", 0, 1),
                flow_item("Chorus", 0, 1),
            ]
        );
    }

    #[test]
    fn flow_items_does_not_mutate_song() {
        let song = Song {
            sections: vec![
                section("Verse", vec![text_line("Alpha")]),
                section("Verse", vec![text_line("Beta")]),
                section("Verse [2]", vec![text_line("Literal two")]),
            ],
            ..Song::default()
        };
        let original = song.clone();

        let items = song.flow_items();

        assert_eq!(song, original);
        assert_eq!(
            items,
            vec![
                flow_item("Verse", 0, 1),
                flow_item("Verse", 1, 1),
                flow_item("Verse [2]", 0, 1),
            ]
        );
    }

    #[test]
    fn custom_flow_returns_empty_for_song_without_sections() {
        let song = Song::default();
        assert!(song.custom_flow().is_empty());
    }

    #[test]
    fn custom_flow_returns_single_name_for_single_section() {
        let song = Song {
            sections: vec![section("Chorus", vec![text_line("This is the content")])],
            ..Song::default()
        };

        assert_eq!(song.custom_flow(), vec![flow_item("Chorus", 0, 1)]);
    }

    #[test]
    fn custom_flow_reuses_name_for_exact_duplicate_sections() {
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
            song.custom_flow(),
            vec![
                flow_item("Chorus", 0, 1),
                flow_item("Chorus", 0, 1),
                flow_item("Chorus", 1, 1),
                flow_item("Chorus", 0, 1),
            ]
        );
    }

    #[test]
    fn custom_flow_ignores_repeat_count_when_contents_match() {
        let song = Song {
            sections: vec![
                section_with_repeat("Verse", vec![text_line("Same line")], 1),
                section_with_repeat("Verse", vec![text_line("Same line")], 4),
            ],
            ..Song::default()
        };

        assert_eq!(
            song.custom_flow(),
            vec![flow_item("Verse", 0, 1), flow_item("Verse", 0, 4)]
        );
    }

    #[test]
    fn custom_flow_copies_repeat_counts_for_default_flow() {
        let song = Song {
            sections: vec![
                section_with_repeat("Verse", vec![text_line("One")], 2),
                section_with_repeat("Chorus", vec![text_line("Two")], 3),
            ],
            ..Song::default()
        };

        assert_eq!(
            song.custom_flow(),
            vec![flow_item("Verse", 0, 2), flow_item("Chorus", 0, 3)]
        );
    }

    #[test]
    fn custom_flow_keeps_empty_sections_in_flow_without_creating_variants() {
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
            song.custom_flow(),
            vec![
                flow_item("Intro", 0, 1),
                flow_item("Intro", 0, 1),
                flow_item("Intro", 0, 1),
                flow_item("Intro", 0, 1),
            ]
        );
    }

    #[test]
    fn custom_flow_keeps_unsuffixed_name_when_empty_precedes_content() {
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
            song.custom_flow(),
            vec![
                flow_item("Bridge", 0, 1),
                flow_item("Bridge", 0, 1),
                flow_item("Bridge", 0, 1),
                flow_item("Bridge", 1, 1),
                flow_item("Bridge", 0, 1),
            ]
        );
    }

    #[test]
    fn custom_flow_handles_empty_sections_between_content_variants() {
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
            song.custom_flow(),
            vec![
                flow_item("Chorus", 0, 1),
                flow_item("Chorus", 0, 1),
                flow_item("Chorus", 1, 1),
                flow_item("Chorus", 0, 1),
                flow_item("Chorus", 1, 1),
            ]
        );
    }

    #[test]
    fn custom_flow_compares_sections_structurally_across_line_details() {
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
            song.custom_flow(),
            vec![
                flow_item("Verse", 0, 1),
                flow_item("Verse", 1, 1),
                flow_item("Verse", 2, 1),
                flow_item("Verse", 3, 1),
                flow_item("Verse", 4, 1),
            ]
        );
    }

    #[test]
    fn custom_flow_treats_translation_changes_as_distinct() {
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

        assert_eq!(
            song.custom_flow(),
            vec![flow_item("Verse", 0, 1), flow_item("Verse", 1, 1)]
        );
    }

    #[test]
    fn custom_flow_treats_line_count_changes_as_distinct() {
        let song = Song {
            sections: vec![
                section("Verse", vec![text_line("One")]),
                section("Verse", vec![text_line("One"), text_line("Two")]),
            ],
            ..Song::default()
        };

        assert_eq!(
            song.custom_flow(),
            vec![flow_item("Verse", 0, 1), flow_item("Verse", 1, 1)]
        );
    }

    #[test]
    fn custom_flow_treats_different_line_order_as_distinct() {
        let song = Song {
            sections: vec![
                section("Verse", vec![text_line("One"), text_line("Two")]),
                section("Verse", vec![text_line("Two"), text_line("One")]),
            ],
            ..Song::default()
        };

        assert_eq!(
            song.custom_flow(),
            vec![flow_item("Verse", 0, 1), flow_item("Verse", 1, 1)]
        );
    }

    #[test]
    fn custom_flow_keeps_same_content_under_different_titles_separate() {
        let song = Song {
            sections: vec![
                section("Verse", vec![text_line("Shared")]),
                section("Chorus", vec![text_line("Shared")]),
            ],
            ..Song::default()
        };

        assert_eq!(
            song.custom_flow(),
            vec![flow_item("Verse", 0, 1), flow_item("Chorus", 0, 1)]
        );
    }

    #[test]
    fn custom_flow_skips_collisions_with_literal_suffixed_titles() {
        let song = Song {
            sections: vec![
                section("Chorus", vec![text_line("Alpha")]),
                section("Chorus [2]", vec![text_line("Literal two")]),
                section("Chorus", vec![text_line("Beta")]),
            ],
            ..Song::default()
        };

        assert_eq!(
            song.custom_flow(),
            vec![
                flow_item("Chorus", 0, 1),
                flow_item("Chorus [2]", 0, 1),
                flow_item("Chorus", 1, 1),
            ]
        );
    }

    #[test]
    fn custom_flow_skips_multiple_occupied_suffixes() {
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
            song.custom_flow(),
            vec![
                flow_item("Verse", 0, 1),
                flow_item("Verse [2]", 0, 1),
                flow_item("Verse [3]", 0, 1),
                flow_item("Verse", 1, 1),
            ]
        );
    }

    #[test]
    fn custom_flow_is_case_sensitive_and_handles_empty_titles() {
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
            song.custom_flow(),
            vec![
                flow_item("", 0, 1),
                flow_item("", 1, 1),
                flow_item("chorus", 0, 1),
                flow_item("Chorus", 0, 1),
            ]
        );
    }

    #[test]
    fn custom_flow_does_not_mutate_song() {
        let song = Song {
            sections: vec![
                section("Verse", vec![text_line("Alpha")]),
                section("Verse", vec![text_line("Beta")]),
                section("Verse [2]", vec![text_line("Literal two")]),
            ],
            ..Song::default()
        };
        let original = song.clone();

        let flow = song.custom_flow();

        assert_eq!(song, original);
        assert_eq!(
            flow,
            vec![
                flow_item("Verse", 0, 1),
                flow_item("Verse", 1, 1),
                flow_item("Verse [2]", 0, 1),
            ]
        );
    }

    #[test]
    fn apply_flow_reorders_sections_to_match_flow() {
        let song = Song {
            sections: vec![
                section("Verse", vec![text_line("First line")]),
                section("Chorus", vec![text_line("Second line")]),
            ],
            ..Song::default()
        };
        let flow = vec![
            flow_item("Chorus", 0, 1),
            flow_item("Verse", 0, 2),
            flow_item("Chorus", 0, 1),
        ];

        let mut song = song;
        song.apply_flow(flow.clone()).expect("valid flow");

        assert_eq!(song.sections.len(), 3);
        assert_eq!(song.sections[0].title, "Chorus");
        assert_eq!(song.sections[0].lines, vec![text_line("Second line")]);
        assert_eq!(song.sections[1].title, "Verse");
        assert_eq!(song.sections[1].lines, vec![text_line("First line")]);
        assert_eq!(song.sections[1].repeat_count, 2);
        assert!(song.sections[2].lines.is_empty());
        assert_eq!(song.custom_flow(), flow);
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
            flow_item("Verse", 0, 1),
            flow_item("Verse", 0, 2),
            flow_item("Chorus", 0, 1),
        ];

        let sections = song.sections_for_flow(&flow).expect("flow");
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
    fn fill_section_references_copies_prior_body_into_empty_sections() {
        let mut song = Song {
            sections: vec![
                section("Verse", vec![text_line("Text 1")]),
                section("Chorus", vec![text_line("Text 2")]),
                section("Verse 2", vec![text_line("Text 3")]),
                section("Chorus", vec![]),
                section("Bridge", vec![text_line("Text 4")]),
                section("Chorus", vec![]),
            ],
            ..Song::default()
        };

        song.fill_section_references();

        assert_eq!(song.sections[0].lines, vec![text_line("Text 1")]);
        assert_eq!(song.sections[1].lines, vec![text_line("Text 2")]);
        assert_eq!(song.sections[2].lines, vec![text_line("Text 3")]);
        assert_eq!(song.sections[3].lines, vec![text_line("Text 2")]);
        assert_eq!(song.sections[4].lines, vec![text_line("Text 4")]);
        assert_eq!(song.sections[5].lines, vec![text_line("Text 2")]);
    }

    #[test]
    fn fill_section_references_leaves_empty_sections_before_first_definition() {
        let mut song = Song {
            sections: vec![
                section("Intro", vec![]),
                section("Intro", vec![text_line("First content")]),
                section("Intro", vec![]),
            ],
            ..Song::default()
        };

        song.fill_section_references();

        assert!(song.sections[0].lines.is_empty());
        assert_eq!(song.sections[1].lines, vec![text_line("First content")]);
        assert_eq!(song.sections[2].lines, vec![text_line("First content")]);
    }

    #[test]
    fn fill_section_references_uses_occurrence_zero_for_empty_references() {
        let mut song = Song {
            sections: vec![
                section("Chorus", vec![text_line("Alpha")]),
                section("Chorus", vec![]),
                section("Chorus", vec![text_line("Beta")]),
                section("Chorus", vec![]),
            ],
            ..Song::default()
        };

        song.fill_section_references();

        assert_eq!(song.sections[0].lines, vec![text_line("Alpha")]);
        assert_eq!(song.sections[1].lines, vec![text_line("Alpha")]);
        assert_eq!(song.sections[2].lines, vec![text_line("Beta")]);
        assert_eq!(song.sections[3].lines, vec![text_line("Alpha")]);
    }

    #[test]
    fn fill_section_references_does_not_overwrite_existing_content() {
        let mut song = Song {
            sections: vec![
                section("Verse", vec![text_line("Original")]),
                section("Verse", vec![text_line("Different")]),
            ],
            ..Song::default()
        };
        let original = song.sections.clone();

        song.fill_section_references();

        assert_eq!(song.sections, original);
    }

    #[test]
    fn sections_for_flow_rejects_unknown_titles_and_zero_repeats() {
        let song = Song {
            sections: vec![section("Verse", vec![text_line("One")])],
            ..Song::default()
        };

        let unknown = song
            .sections_for_flow(&[flow_item("Missing", 0, 1)])
            .expect_err("unknown title should fail");
        assert!(matches!(unknown, Error::InvalidSongFlow(_)));

        let invalid_repeats = song
            .sections_for_flow(&[flow_item("Verse", 0, 0)])
            .expect_err("repeat zero should fail");
        assert!(matches!(invalid_repeats, Error::InvalidSongFlow(_)));
    }
}
