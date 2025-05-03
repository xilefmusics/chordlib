use crate::types::{Chord, ChordRepresentation, Line, Part, Section, SimpleChord, Song};

pub trait FormatChordPro {
    fn format_chord_pro(
        &self,
        key: Option<&SimpleChord>,
        representation: Option<&ChordRepresentation>,
        language: Option<usize>,
        worship_pro_features: bool,
    ) -> String;
}

impl FormatChordPro for &Chord {
    fn format_chord_pro(
        &self,
        key: Option<&SimpleChord>,
        representation: Option<&ChordRepresentation>,
        _: Option<usize>,
        worship_pro_features: bool,
    ) -> String {
        if worship_pro_features {
            if let Some(duration) = self.get_duration() {
                return format!(
                    "{}:{}",
                    self.format(
                        key.unwrap_or(&SimpleChord::default()),
                        representation.unwrap_or(&ChordRepresentation::default())
                    ),
                    duration
                );
            }
        }
        self.format(
            key.unwrap_or(&SimpleChord::default()),
            representation.unwrap_or(&ChordRepresentation::default()),
        )
    }
}

impl FormatChordPro for &Part {
    fn format_chord_pro(
        &self,
        key: Option<&SimpleChord>,
        representation: Option<&ChordRepresentation>,
        language: Option<usize>,
        worship_pro_features: bool,
    ) -> String {
        let chord = self
            .chord
            .clone()
            .map(|chord| {
                format!(
                    "[{}]",
                    (&chord).format_chord_pro(key, representation, language, worship_pro_features)
                )
            })
            .unwrap_or("".into());
        let language = language.unwrap_or(0);
        if self.comment {
            format!("{}{{comment: {}}}", chord, self.languages[language])
        } else {
            format!("{}{}", chord, self.languages[language])
        }
    }
}

impl FormatChordPro for &Line {
    fn format_chord_pro(
        &self,
        key: Option<&SimpleChord>,
        representation: Option<&ChordRepresentation>,
        language: Option<usize>,
        worship_pro_features: bool,
    ) -> String {
        self.parts
            .iter()
            .map(|part| part.format_chord_pro(key, representation, language, worship_pro_features))
            .collect()
    }
}

impl FormatChordPro for &Section {
    fn format_chord_pro(
        &self,
        key: Option<&SimpleChord>,
        representation: Option<&ChordRepresentation>,
        language: Option<usize>,
        worship_pro_features: bool,
    ) -> String {
        std::iter::once(format!("{{section: {}}}", self.title))
            .chain(self.lines.iter().map(|line| {
                line.format_chord_pro(key, representation, language, worship_pro_features)
            }))
            .collect::<Vec<String>>()
            .join("\n")
    }
}

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
        let mut meta = vec![
            format!("{{title: {}}}", self.title),
            format!(
                "{{key: {}}}",
                SimpleChord::default().format(&key, &ChordRepresentation::default())
            ),
        ];
        if let Some(artist) = &self.artist {
            meta.push(format!("{{artist: {}}}", artist));
        }
        if let Some(language) = &self.language {
            meta.push(format!("{{language: {}}}", language));
        }
        if let Some(tempo) = &self.tempo {
            meta.push(format!("{{tempo: {}}}", tempo));
        }
        if let Some(time) = &self.time {
            meta.push(format!("{{time: {}/{}}}", time.0, time.1));
        }

        meta.into_iter()
            .chain(self.sections.iter().map(|section| {
                section.format_chord_pro(Some(key), representation, language, worship_pro_features)
            }))
            .collect::<Vec<String>>()
            .join("\n")
    }
}
