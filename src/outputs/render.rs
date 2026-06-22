use super::{FormatOutputLines, OutputLine};
use crate::Error;
use crate::types::{ChordRepresentation, SimpleChord, Song};

pub trait FormatRender {
    fn format_render(
        &self,
        key: Option<&SimpleChord>,
        representation: Option<&ChordRepresentation>,
        language: Option<usize>,
    ) -> Result<String, Error>;
}

impl FormatRender for Song {
    fn format_render(
        &self,
        key: Option<&SimpleChord>,
        representation: Option<&ChordRepresentation>,
        language: Option<usize>,
    ) -> Result<String, Error> {
        let lines = self.format_output_lines(key, representation, language)?;
        Ok(lines
            .iter()
            .map(|line| match line {
                OutputLine::Keyword(keyword) => format!("\x1b[31;1m{}\x1b[0m", keyword),
                OutputLine::Chord(chord) => format!("\x1b[32;1m{}\x1b[0m", chord),
                OutputLine::Text(text) => format!("\x1b[32m{}\x1b[0m", text),
            })
            .collect::<Vec<String>>()
            .join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inputs::chord_pro::load_string;
    use crate::types::SongFlowItem;

    fn flow_item(title: &str, occurrence_index: u32, repeats: u32) -> SongFlowItem {
        SongFlowItem {
            title: title.to_string(),
            occurrence_index,
            repeats,
        }
    }

    #[test]
    fn format_render_supports_custom_flow_after_apply_flow() {
        let input = r#"{title: Ohne Titel}
{key: A}
{section: Tag 1}
Text 1
{section: Tag 2}
Text 2
{section: Tag 3}
Text 3
{section: Tag 1}
{section: Tag 2}
{section: Tag 3}
"#;
        let mut song = load_string(input).expect("parse");
        let rep = ChordRepresentation::Default;
        let flow = vec![
            flow_item("Tag 1", 0, 1),
            flow_item("Tag 2", 0, 1),
            flow_item("Tag 3", 0, 1),
            flow_item("Tag 1", 0, 2),
            flow_item("Tag 3", 0, 1),
            flow_item("Tag 2", 0, 1),
        ];

        song.apply_flow(flow).expect("apply flow");

        let custom_render = song.format_render(None, Some(&rep), None).expect("render");
        assert!(custom_render.contains("\x1b[31;1mTag 1\x1b[0m"));
        assert!(custom_render.contains("\x1b[32mText 1\x1b[0m"));
        assert!(custom_render.contains("\x1b[32m(repeat)\x1b[0m"));
    }
}
