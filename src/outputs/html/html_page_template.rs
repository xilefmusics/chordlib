use askama::Template;

#[derive(Template)]
#[template(path = "format_html_page.html", escape = "none")]
pub struct HtmlPageTemplate<'a> {
    title: &'a str,
    subtitle: &'a str,
    key: &'a str,
    tempo: &'a Option<u32>,
    time: &'a Option<(u32, u32)>,
    sections: Vec<String>,
    copyright: &'a Option<String>,
}

impl<'a> HtmlPageTemplate<'a> {
    pub fn with_capacity(sections_capacity: usize) -> Self {
        Self {
            title: "",
            subtitle: "",
            key: "",
            tempo: &None,
            time: &None,
            sections: Vec::with_capacity(sections_capacity),
            copyright: &None,
        }
    }

    pub fn title(mut self, title: &'a str) -> Self {
        self.title = title;
        self
    }

    pub fn subtitle(mut self, subtitle: &'a str) -> Self {
        self.subtitle = subtitle;
        self
    }

    pub fn key(mut self, key: &'a str) -> Self {
        self.key = key;
        self
    }

    pub fn tempo(mut self, tempo: &'a Option<u32>) -> Self {
        self.tempo = tempo;
        self
    }

    pub fn time(mut self, time: &'a Option<(u32, u32)>) -> Self {
        self.time = time;
        self
    }

    pub fn section(mut self, section: String) -> Self {
        self.sections.push(section);
        self
    }

    pub fn copyright(mut self, copyright: &'a Option<String>) -> Self {
        self.copyright = copyright;
        self
    }
}
