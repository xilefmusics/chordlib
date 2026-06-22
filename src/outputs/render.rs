use super::{FormatOutputLines, OutputLine};
use crate::Error;
use crate::types::{ChordRepresentation, SimpleChord, Song, SongFlowItem};

pub trait FormatRender {
    fn format_render(
        &self,
        key: Option<&SimpleChord>,
        representation: Option<&ChordRepresentation>,
        language: Option<usize>,
        flow: Option<&[SongFlowItem]>,
    ) -> Result<String, Error>;
}

impl FormatRender for Song {
    fn format_render(
        &self,
        key: Option<&SimpleChord>,
        representation: Option<&ChordRepresentation>,
        language: Option<usize>,
        flow: Option<&[SongFlowItem]>,
    ) -> Result<String, Error> {
        let lines = self.format_output_lines(key, representation, language, flow)?;
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

    fn flow_item(title: &str, repeats: u32) -> SongFlowItem {
        SongFlowItem {
            title: title.to_string(),
            repeats,
        }
    }

    #[test]
    fn format_render_supports_custom_flow_and_empty_flow_matches_default() {
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
        let song = load_string(input).expect("parse");
        let rep = ChordRepresentation::Default;
        let flow = vec![
            flow_item("Tag 1", 1),
            flow_item("Tag 2", 1),
            flow_item("Tag 3", 1),
            flow_item("Tag 1", 2),
            flow_item("Tag 3", 1),
            flow_item("Tag 2", 1),
        ];

        let default_render = song
            .format_render(None, Some(&rep), None, None)
            .expect("render");
        let empty_flow_render = song
            .format_render(None, Some(&rep), None, Some(&[]))
            .expect("render");
        assert_eq!(default_render, empty_flow_render);

        let custom_render = song
            .format_render(None, Some(&rep), None, Some(&flow))
            .expect("render");
        assert!(custom_render.contains("\x1b[31;1mTag 1\x1b[0m"));
        assert!(custom_render.contains("\x1b[32mText 1\x1b[0m"));
        assert!(custom_render.contains("\x1b[32m(repeat)\x1b[0m"));
    }
}
