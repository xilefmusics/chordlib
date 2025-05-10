use super::render_section::render_section;
use super::FormatChordPro;
use crate::types::{ChordRepresentation, SimpleChord, Song};

impl FormatChordPro for &Song {
    fn format_chord_pro(
        &self,
        key: Option<&SimpleChord>,
        representation: Option<&ChordRepresentation>,
        language: Option<usize>,
        worship_pro_features: bool,
    ) -> String {
        let self_key = self.key.clone().unwrap_or(SimpleChord::default()).into();
        let key = key.unwrap_or(&self_key);

        let separator = if worship_pro_features { ": " } else { ":" };

        let mut meta = vec![format!("{{title{}{}}}", separator, self.title)];
        if let Some(subtitle) = &self.subtitle {
            meta.push(format!("{{subtitle{}{}}}", separator, subtitle));
        }
        meta.push(format!(
            "{{key{}{}}}",
            separator,
            SimpleChord::default().format(&key, &ChordRepresentation::default())
        ));
        if let Some(copyright) = &self.copyright {
            meta.push(format!("{{coptyright{}{}}}", separator, copyright));
        }
        if let Some(artist) = &self.artist {
            meta.push(format!("{{artist{}{}}}", separator, artist));
        }
        if let Some(language) = &self.language {
            meta.push(format!("{{language{}{}}}", separator, language));
        }
        if let Some(tempo) = &self.tempo {
            meta.push(format!("{{tempo{}{}}}", separator, tempo));
        }
        if let Some(time) = &self.time {
            meta.push(format!("{{time{}{}/{}}}", separator, time.0, time.1));
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
                )
            }))
            .chain(std::iter::once("".to_string()))
            .collect::<Vec<String>>()
            .join("\n")
    }
}
