use askama::Template;

#[derive(Template)]
#[template(path = "format_html.css", escape = "none")]
pub struct CssTemplate {
    scale: f32,
    has_footer: bool,
}

impl CssTemplate {
    pub fn new() -> Self {
        Self {
            scale: 1.0,
            has_footer: false,
        }
    }

    pub fn scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    pub fn has_footer(mut self, has_footer: bool) -> Self {
        self.has_footer = has_footer;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use askama::Template;

    #[test]
    fn chord_labels_overlay_without_changing_lyric_width() {
        let css = CssTemplate::new().render().expect("render CSS");

        assert!(css.contains("padding-top: 1.4em;"));
        assert!(css.contains("position: absolute;"));
        assert!(css.contains(".chord-word-gap {"));
    }
}
