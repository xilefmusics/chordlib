use crate::types::{Chord, Line, Part, Section, SimpleChord, Song};

pub trait FormatChordPro {
    fn format_chord_pro(&self, key: Option<SimpleChord>, language: Option<usize>) -> String;
}

impl FormatChordPro for &Chord {
    fn format_chord_pro(&self, key: Option<SimpleChord>, _: Option<usize>) -> String {
        self.format(key.unwrap_or(SimpleChord::default()))
    }
}

impl FormatChordPro for &Part {
    fn format_chord_pro(&self, key: Option<SimpleChord>, language: Option<usize>) -> String {
        let chord = self
            .chord
            .clone()
            .map(|chord| format!("[{}]", (&chord).format_chord_pro(key.clone(), language)))
            .unwrap_or("".into());
        let language = language.unwrap_or(0);
        format!("{}{}", chord, self.languages[language])
    }
}

impl FormatChordPro for &Line {
    fn format_chord_pro(&self, key: Option<SimpleChord>, language: Option<usize>) -> String {
        self.parts
            .iter()
            .map(|part| part.format_chord_pro(key.clone(), language))
            .collect()
    }
}

impl FormatChordPro for &Section {
    fn format_chord_pro(&self, key: Option<SimpleChord>, language: Option<usize>) -> String {
        std::iter::once(format!("{{section: {}}}", self.title))
            .chain(
                self.lines
                    .iter()
                    .map(|line| line.format_chord_pro(key.clone(), language)),
            )
            .collect::<Vec<String>>()
            .join("\n")
    }
}

impl FormatChordPro for &Song {
    fn format_chord_pro(&self, key: Option<SimpleChord>, language: Option<usize>) -> String {
        let key = key.unwrap_or(self.key.clone().unwrap_or(SimpleChord::default()));
        let mut meta = vec![
            format!("{{title: {}}}", self.title),
            format!("{{key: {}}}", SimpleChord::default().format(&key)),
        ];
        if let Some(artist) = self.artist.clone() {
            meta.push(format!("{{artist: {}}}", artist));
        }
        if let Some(language) = self.language.clone() {
            meta.push(format!("{{language: {}}}", language));
        }

        meta.into_iter()
            .chain(
                self.sections
                    .iter()
                    .map(|section| section.format_chord_pro(Some(key.clone()), language)),
            )
            .collect::<Vec<String>>()
            .join("\n")
    }
}
