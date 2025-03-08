use askama::Template;

#[derive(Template)]
#[template(path = "format_html.html", escape = "none")]
pub struct HtmlTemplate<'a> {
    title: &'a str,
    page: &'a str,
    style: &'a str,
}

impl<'a> HtmlTemplate<'a> {
    pub fn new() -> Self {
        Self {
            title: "",
            style: "",
            page: "",
        }
    }

    pub fn title(mut self, title: &'a str) -> Self {
        self.title = title;
        self
    }

    pub fn page(mut self, page: &'a str) -> Self {
        self.page = page;
        self
    }

    pub fn style(mut self, style: &'a str) -> Self {
        self.style = style;
        self
    }
}
