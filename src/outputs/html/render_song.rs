use super::{render_section, CssTemplate, FormatHTML, HtmlPageTemplate, HtmlTemplate};
use crate::types::{ChordRepresentation, SimpleChord, Song};
use askama::Template;

impl FormatHTML for &Song {
    fn format_html_page(
        &self,
        key: Option<&SimpleChord>,
        representation: Option<&ChordRepresentation>,
        language: Option<usize>,
        scale: Option<f32>,
    ) -> (String, String) {
        let language = language.unwrap_or(0);

        let self_key = self
            .key
            .as_ref()
            .unwrap_or(&SimpleChord::default())
            .clone()
            .into();
        let key = key.unwrap_or(&self_key);
        let key_str = SimpleChord::default().format(key, &ChordRepresentation::Default);

        let subtitle = self.artist.as_deref().unwrap_or("").to_string();

        let page_template = self
            .sections
            .iter()
            .map(|section| {
                render_section::render_section(
                    &section,
                    key,
                    representation
                        .as_ref()
                        .unwrap_or(&&ChordRepresentation::Default),
                    language,
                    self.bar_duration(),
                )
            })
            .fold(
                HtmlPageTemplate::new()
                    .title(&self.title)
                    .subtitle(&subtitle)
                    .key(key_str)
                    .tempo(&self.tempo)
                    .time(&self.time),
                |template, section| template.section(section),
            );

        let style_template = {
            let mut template = CssTemplate::new();
            if let Some(scale) = scale {
                template = template.scale(scale);
            }
            template
        };

        (
            page_template.render().unwrap(),
            style_template.render().unwrap(),
        )
    }

    fn format_html(
        &self,
        key: Option<&SimpleChord>,
        representation: Option<&ChordRepresentation>,
        language: Option<usize>,
        scale: Option<f32>,
    ) -> String {
        let (page, style) = self.format_html_page(key, representation, language, scale);
        HtmlTemplate::new()
            .title(&self.title)
            .page(&page)
            .style(&style)
            .render()
            .unwrap()
    }
}
