use super::{CssTemplate, FormatHTML, HtmlPageTemplate, HtmlTemplate, render_section};
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

        let self_key = self.key.as_ref().unwrap_or(&SimpleChord::default()).clone();
        let key = key.unwrap_or(&self_key);
        let key_str = SimpleChord::default().format(key, &ChordRepresentation::Default);

        let selected_artist = self.artist_slice().and_then(|list| {
            let idx = language;
            if let Some(candidate) = list.get(idx)
                && !candidate.trim().is_empty()
            {
                return Some(candidate.as_str());
            }
            list.first()
                .map(String::as_str)
                .filter(|s| !s.trim().is_empty())
        });

        let subtitle = match (
            self.subtitle.as_deref().filter(|s| !s.is_empty()),
            selected_artist,
        ) {
            (Some(sub), Some(art)) => format!("{sub} | {art}"),
            (Some(sub), None) => sub.to_string(),
            (None, Some(art)) => art.to_string(),
            (None, None) => String::new(),
        };

        let beats_per_bar = self.time.map(|(n, _)| n).unwrap_or(4);
        let bar_duration = self.bar_duration();
        let display_title = self.title_for_language(Some(language));
        let page_template = self
            .sections
            .iter()
            .map(|section| {
                render_section::render_section(
                    section,
                    key,
                    representation
                        .as_ref()
                        .map_or(&ChordRepresentation::Default, |v| v),
                    language,
                    bar_duration,
                    beats_per_bar,
                )
            })
            .fold(
                HtmlPageTemplate::with_capacity(self.sections.len())
                    .title(display_title)
                    .subtitle(&subtitle)
                    .key(key_str)
                    .tempo(&self.tempo)
                    .time(&self.time)
                    .copyright(&self.copyright),
                |template, section| template.section(section),
            );

        let style_template = {
            let mut template = CssTemplate::new().has_footer(self.copyright.is_some());
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
        wrap_html(&page, &style, self.title_for_language(language))
    }
}

pub fn wrap_html(html: &str, css: &str, title: &str) -> String {
    HtmlTemplate::new()
        .title(title)
        .page(html)
        .style(css)
        .render()
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inputs::chord_pro::load_string;

    fn extract_tag(html: &str, tag: &str) -> Option<String> {
        let start = format!("<{tag}");
        let idx = html.find(&start)?;
        let after_open = &html[idx..];
        let open_end = after_open.find('>')? + idx + 1;
        let close = format!("</{tag}>");
        let close_idx = html[open_end..].find(&close)? + open_end;
        Some(html[open_end..close_idx].to_string())
    }

    #[test]
    fn single_title_used_for_all_languages() {
        let input = r#"{title: Single}
{key: C}
{language: de}
{language2: en}
{section: Verse}
[C]Line
"#;
        let song = load_string(input).expect("parse");
        let rep = ChordRepresentation::Default;

        let html_lang0 = (&song).format_html(None, Some(&rep), None, None);
        let html_lang1 = (&song).format_html(None, Some(&rep), Some(1), None);

        // <title> tag
        assert_eq!(extract_tag(&html_lang0, "title").as_deref(), Some("Single"));
        assert_eq!(extract_tag(&html_lang1, "title").as_deref(), Some("Single"));

        // <h1 class="title"> in header
        assert!(html_lang0.contains(r#"<h1 class="title">Single</h1>"#));
        assert!(html_lang1.contains(r#"<h1 class="title">Single</h1>"#));
    }

    #[test]
    fn multi_language_title_selects_matching_title_and_falls_back() {
        let input = r#"{title: "Title DE"}
{title2: "Title EN"}
{key: C}
{language: de}
{language2: en}
{artist: "Artist DE"}
{artist2: "Artist EN"}
{section: Verse}
[C]Line
"#;
        let song = load_string(input).expect("parse");
        let rep = ChordRepresentation::Default;

        let html_lang0 = (&song).format_html(None, Some(&rep), None, None);
        let html_lang1 = (&song).format_html(None, Some(&rep), Some(1), None);
        let html_lang2 = (&song).format_html(None, Some(&rep), Some(2), None);

        // Language 0: first title
        assert_eq!(
            extract_tag(&html_lang0, "title").as_deref(),
            Some("Title DE")
        );
        assert!(html_lang0.contains(r#"<h1 class="title">Title DE</h1>"#));
        assert!(html_lang0.contains(r#"<h2 class="subtitle">Artist DE</h2>"#));

        // Language 1: second title
        assert_eq!(
            extract_tag(&html_lang1, "title").as_deref(),
            Some("Title EN")
        );
        assert!(html_lang1.contains(r#"<h1 class="title">Title EN</h1>"#));
        assert!(!html_lang1.contains(r#"<h1 class="title">Title DE</h1>"#));
        // Artist follows the language index, with fallback.
        assert!(html_lang1.contains(r#"<h2 class="subtitle">Artist EN</h2>"#));

        // Out-of-range language index should fall back to primary title.
        assert_eq!(
            extract_tag(&html_lang2, "title").as_deref(),
            Some("Title DE")
        );
        assert!(html_lang2.contains(r#"<h1 class="title">Title DE</h1>"#));
        // No third artist specified, so it must fall back to the first artist.
        assert!(html_lang2.contains(r#"<h2 class="subtitle">Artist DE</h2>"#));
    }
}
