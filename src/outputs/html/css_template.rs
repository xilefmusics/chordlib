use askama::Template;

#[derive(Template)]
#[template(path = "format_html.css", escape = "none")]
pub struct CssTemplate {
    scale: f32,
}

impl CssTemplate {
    pub fn new() -> Self {
        Self { scale: 1.0 }
    }

    pub fn scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }
}
