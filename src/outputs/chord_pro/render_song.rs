use super::FormatChordPro;
use super::render_section::render_section;
use crate::types::{ChordRepresentation, SimpleChord, Song};

impl FormatChordPro for &Song {
    fn format_chord_pro(
        &self,
        key: Option<&SimpleChord>,
        representation: Option<&ChordRepresentation>,
        language: Option<usize>,
        worship_pro_features: bool,
    ) -> String {
        let self_key = self.key.clone().unwrap_or_default();
        let key = key.unwrap_or(&self_key);

        let separator = if worship_pro_features { ": " } else { ":" };

        let mut meta = Vec::new();

        // Title(s)
        if worship_pro_features {
            let mut any = false;
            for (idx, title) in self.titles.iter().enumerate() {
                if title.is_empty() {
                    continue;
                }
                any = true;
                let key = if idx == 0 {
                    "title".to_string()
                } else {
                    format!("title{}", idx + 1)
                };
                meta.push(format!("{{{}{}{}}}", key, separator, title));
            }
            if !any {
                meta.push(format!("{{title{}{}}}", separator, self.title()));
            }
        } else {
            meta.push(format!("{{title{}{}}}", separator, self.title()));
        }

        // Subtitle
        if let Some(subtitle) = &self.subtitle {
            meta.push(format!("{{subtitle{}{}}}", separator, subtitle));
        }

        // Key
        meta.push(format!(
            "{{key{}{}}}",
            separator,
            SimpleChord::default().format(key, &ChordRepresentation::Default)
        ));

        if let Some(copyright) = &self.copyright {
            meta.push(format!("{{copyright{}{}}}", separator, copyright));
        }

        // Artist(s)
        if worship_pro_features {
            for (idx, artist) in self.artists.iter().enumerate() {
                if artist.is_empty() {
                    continue;
                }
                let key = if idx == 0 {
                    "artist".to_string()
                } else {
                    format!("artist{}", idx + 1)
                };
                meta.push(format!("{{{}{}{}}}", key, separator, artist));
            }
        } else if !self.artist().is_empty() {
            meta.push(format!("{{artist{}{}}}", separator, self.artist()));
        }

        // Language(s)
        if worship_pro_features {
            for (idx, lang) in self.languages.iter().enumerate() {
                if lang.is_empty() {
                    continue;
                }
                let key = if idx == 0 {
                    "language".to_string()
                } else {
                    format!("language{}", idx + 1)
                };
                meta.push(format!("{{{}{}{}}}", key, separator, lang));
            }
        } else if !self.language().is_empty() {
            meta.push(format!("{{language{}{}}}", separator, self.language()));
        }

        // Tempo & time
        if let Some(tempo) = &self.tempo {
            meta.push(format!("{{tempo{}{}}}", separator, tempo));
        }
        if let Some(time) = &self.time {
            meta.push(format!("{{time{}{}/{}}}", separator, time.0, time.1));
        }

        // Custom tags (ChordPro {meta: name value}); written for both Chord Pro and Worship Pro
        if !self.tags.is_empty() {
            for (name, value) in &self.tags {
                meta.push(format!("{{meta: {} {}}}", name, value));
            }
        }

        meta.into_iter()
            .chain(self.sections.iter().map(|section| {
                render_section(
                    section,
                    Some(key),
                    representation,
                    language,
                    worship_pro_features,
                    self.bar_duration(),
                    self.beats_per_bar(),
                )
            }))
            .chain(std::iter::once("".to_string()))
            .collect::<Vec<String>>()
            .join("\n")
    }
}
