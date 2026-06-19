use super::FormatChordPro;
use super::render_line::render_line;
use crate::types::{ChordRepresentation, Section, SimpleChord};

pub fn render_section(
    section: &Section,
    key: Option<&SimpleChord>,
    representation: Option<&ChordRepresentation>,
    language: Option<usize>,
    worship_pro_features: bool,
    bar_duration: u32,
    beats_per_bar: u32,
) -> String {
    let keyword = if worship_pro_features {
        format!("{{section: {}}}", section.title)
    } else {
        format!("\n{}:", section.title)
    };

    let mut line_outputs: Vec<String> = Vec::new();

    for line in &section.lines {
        if worship_pro_features {
            // For Worship Pro exports, preserve all available language texts by
            // emitting one base line (language 0) and additional `&`-prefixed
            // lines for higher language indices.
            let max_langs = line
                .parts
                .iter()
                .map(|p| p.languages.len())
                .max()
                .unwrap_or(1);

            if max_langs > 1 {
                for lang_idx in 0..max_langs {
                    let rendered = render_line(
                        line,
                        key,
                        representation,
                        Some(lang_idx),
                        worship_pro_features,
                        bar_duration,
                        beats_per_bar,
                    );
                    if lang_idx == 0 {
                        line_outputs.push(rendered);
                    } else {
                        line_outputs.push(format!("&{}", rendered));
                    }
                }
                continue;
            }
        }

        line_outputs.push(render_line(
            line,
            key,
            representation,
            language,
            worship_pro_features,
            bar_duration,
            beats_per_bar,
        ));
    }

    let repeat_line = if section.repeat_count > 1 {
        Some(if worship_pro_features {
            if section.repeat_count == 2 {
                "{repeat}".to_string()
            } else {
                format!("{{repeat: {}}}", section.repeat_count)
            }
        } else if section.repeat_count == 2 {
            "{comment: (repeat)}".to_string()
        } else {
            format!("{{comment: (repeat {}x)}}", section.repeat_count)
        })
    } else {
        None
    };

    std::iter::once(keyword)
        .chain(line_outputs)
        .chain(repeat_line)
        .collect::<Vec<String>>()
        .join("\n")
}

impl FormatChordPro for &Section {
    fn format_chord_pro(
        &self,
        key: Option<&SimpleChord>,
        representation: Option<&ChordRepresentation>,
        language: Option<usize>,
        worship_pro_features: bool,
    ) -> String {
        render_section(
            self,
            key,
            representation,
            language,
            worship_pro_features,
            4000,
            4,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inputs::chord_pro::load_string;
    use crate::outputs::FormatChordPro;

    #[test]
    fn format_chord_pro_falls_back_to_primary_language_text() {
        let input = r#"{title: Fallback}
{key: C}
{language: de}
{language2: en}
{section: Verse}
[C]Hallo
"#;
        let song = load_string(input).expect("parse");
        let rep = ChordRepresentation::Default;

        let rendered = (&song).format_chord_pro(None, Some(&rep), Some(1), false);

        assert!(rendered.contains("Hallo"));
        assert!(!rendered.contains("()"));
    }
}
