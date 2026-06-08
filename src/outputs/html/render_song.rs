use super::{CssTemplate, FormatHTML, HtmlPageTemplate, HtmlTemplate, render_section};
use crate::types::{ChordRepresentation, SimpleChord, Song};
use askama::Template;

struct HtmlRenderContext {
    key: SimpleChord,
    representation: ChordRepresentation,
    language: usize,
    bar_duration: u32,
    beats_per_bar: u32,
    compact_six_eight: bool,
}

impl Song {
    fn html_render_context(
        &self,
        key: Option<&SimpleChord>,
        representation: Option<&ChordRepresentation>,
        language: Option<usize>,
    ) -> (HtmlRenderContext, String) {
        let language = language.unwrap_or(0);

        let self_key = self.key.as_ref().unwrap_or(&SimpleChord::default()).clone();
        let key = key.unwrap_or(&self_key).clone();
        let key_str = SimpleChord::default()
            .format(&key, &ChordRepresentation::Default)
            .to_string();

        let ctx = HtmlRenderContext {
            key,
            representation: representation
                .cloned()
                .unwrap_or(ChordRepresentation::Default),
            language,
            bar_duration: self.bar_duration(),
            beats_per_bar: self.beats_per_bar(),
            compact_six_eight: self.time == Some((6, 8)),
        };

        (ctx, key_str)
    }

    fn render_section_htmls(&self, ctx: &HtmlRenderContext) -> Vec<String> {
        self.sections
            .iter()
            .map(|section| {
                render_section::render_section(
                    section,
                    &ctx.key,
                    &ctx.representation,
                    ctx.language,
                    ctx.bar_duration,
                    ctx.beats_per_bar,
                    ctx.compact_six_eight,
                )
            })
            .collect()
    }

    fn html_css(&self, scale: Option<f32>) -> String {
        let mut template = CssTemplate::new().has_footer(self.copyright.is_some());
        if let Some(scale) = scale {
            template = template.scale(scale);
        }
        template.render().unwrap()
    }
}

impl FormatHTML for &Song {
    fn format_html_sections(
        &self,
        key: Option<&SimpleChord>,
        representation: Option<&ChordRepresentation>,
        language: Option<usize>,
        scale: Option<f32>,
    ) -> (Vec<String>, String) {
        let (ctx, _) = self.html_render_context(key, representation, language);
        let sections = self.render_section_htmls(&ctx);
        let css = self.html_css(scale);
        (sections, css)
    }

    fn format_html_page(
        &self,
        key: Option<&SimpleChord>,
        representation: Option<&ChordRepresentation>,
        language: Option<usize>,
        scale: Option<f32>,
    ) -> (String, String) {
        let language = language.unwrap_or(0);
        let (ctx, key_str) = self.html_render_context(key, representation, Some(language));
        let section_htmls = self.render_section_htmls(&ctx);

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

        let display_title = self.title_for_language(Some(language));
        let page_template = section_htmls.into_iter().fold(
            HtmlPageTemplate::with_capacity(self.sections.len())
                .title(display_title)
                .subtitle(&subtitle)
                .key(&key_str)
                .tempo(&self.tempo)
                .time(&self.time)
                .copyright(&self.copyright),
            |template, section| template.section(section),
        );

        (page_template.render().unwrap(), self.html_css(scale))
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
    fn format_html_sections_returns_one_fragment_per_section() {
        let input = r#"{title: Test}
{key: C}
{section: Verse}
[C]Line one
{section: Chorus}
[C]Line two
"#;
        let song = load_string(input).expect("parse");
        let rep = ChordRepresentation::Default;

        let (sections, css) = (&song).format_html_sections(None, Some(&rep), None, None);
        assert_eq!(sections.len(), 2);
        assert!(sections[0].contains("<span class=\"keyword\">Verse</span>"));
        assert!(sections[1].contains("<span class=\"keyword\">Chorus</span>"));
        assert!(!sections[0].contains("<div class=\"page\">"));
        assert!(!css.is_empty());
    }

    #[test]
    fn format_html_sections_empty_when_no_sections() {
        let input = r#"{title: Empty}
{key: C}
"#;
        let song = load_string(input).expect("parse");
        let rep = ChordRepresentation::Default;

        let (sections, css) = (&song).format_html_sections(None, Some(&rep), None, None);
        assert!(sections.is_empty());
        assert!(!css.is_empty());
    }

    #[test]
    fn format_html_page_sections_match_format_html_sections() {
        let input = r#"{title: Match}
{key: C}
{section: A}
[C]One
{section: B}
[D]Two
"#;
        let song = load_string(input).expect("parse");
        let rep = ChordRepresentation::Default;

        let (section_htmls, css_sections) =
            (&song).format_html_sections(None, Some(&rep), None, None);
        let (page_html, css_page) = (&song).format_html_page(None, Some(&rep), None, None);

        for section in &section_htmls {
            assert!(page_html.contains(section.as_str()));
        }
        assert_eq!(css_sections, css_page);
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

    #[test]
    fn html_six_eight_chord_only_full_bar_is_single_chord_token() {
        let input = r#"{title: Six Eight}
{key: C}
{time: 6/8}
{section: Prog}
[C:6]
"#;
        let song = load_string(input).expect("parse");
        let rep = ChordRepresentation::Default;
        let html = (&song).format_html(None, Some(&rep), None, None);

        assert!(
            html.contains(r#"<span class="chord">C</span>"#),
            "expected full-bar 6/8 chord-only cell to be a single C token, got: {}",
            html.chars().take(2000).collect::<String>()
        );
        assert!(
            !html.contains("C /"),
            "full-bar chord should not use beat fillers (C /), got: {}",
            html.chars().take(2000).collect::<String>()
        );
    }

    /// After a multi-chord pipe bar that partitions the measure exactly, the next `[|]…[|]`
    /// group’s chord must render as a full bar (single token, no spurious `F /` or `F ·` tails).
    #[test]
    fn html_pipe_bar_next_group_full_bar_no_continuation_fillers() {
        let input = r#"{title: Pipe}
{key: C}
{time: 4/4}
{section: S}
[|][C][D][E][|][F][|]
"#;
        let song = load_string(input).expect("parse");
        let rep = ChordRepresentation::Default;
        let html = (&song).format_html(None, Some(&rep), None, None);

        assert!(
            html.contains(r#"<span class="chord">F</span><span class="bar">|</span></span></p>"#),
            "second bar should be a lone F in its cell; got: {}",
            html.split(r#"<span class="keyword">S</span>"#)
                .nth(1)
                .unwrap_or(&html)
                .chars()
                .take(800)
                .collect::<String>()
        );
        assert!(
            !html.contains("F /") && !html.contains("F ·"),
            "full-bar F after a complete pipe group must not use beat continuation tokens; got: {}",
            html.split(r#"<span class="keyword">S</span>"#)
                .nth(1)
                .unwrap_or(&html)
                .chars()
                .take(800)
                .collect::<String>()
        );
    }

    /// U+2005 (FOUR-PER-EM SPACE) is Zs whitespace; splitting word spans must not assume
    /// a single-byte space when the next chord follows immediately.
    #[test]
    fn html_four_per_em_space_inside_word_does_not_panic() {
        let mid_space = '\u{2005}';
        let input = format!("{{title: T}}\n{{key: C}}\n{{section: V}}\n[C]a{mid_space}b[D]c\n");
        let song = load_string(&input).expect("parse");
        let rep = ChordRepresentation::Default;
        let html = (&song).format_html(None, Some(&rep), None, None);

        assert!(html.contains(r#"<span class="chord">C</span>"#));
        assert!(html.contains(r#"<span class="chord">D</span>"#));
        assert!(html.contains("class=\"word\""));
    }
}
