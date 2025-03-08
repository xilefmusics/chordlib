use crate::types::Key;

pub trait FormatHTML {
    fn format_html(&self, key: Option<&Key>, language: Option<usize>, scale: Option<f32>)
        -> String;
    fn format_html_page(
        &self,
        key: Option<&Key>,
        language: Option<usize>,
        scale: Option<f32>,
    ) -> (String, String);
}
