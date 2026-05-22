use crate::types::{ChordRepresentation, SimpleChord};

pub trait FormatHTML {
    fn format_html(
        &self,
        key: Option<&SimpleChord>,
        representation: Option<&ChordRepresentation>,
        language: Option<usize>,
        scale: Option<f32>,
    ) -> String;
    fn format_html_page(
        &self,
        key: Option<&SimpleChord>,
        representation: Option<&ChordRepresentation>,
        language: Option<usize>,
        scale: Option<f32>,
    ) -> (String, String);
    /// Render each song section as HTML (`<p><span class="keyword">…</span>…</p>`).
    /// Returns `(section_htmls, css)` — no `.page` wrapper, header, or footer.
    fn format_html_sections(
        &self,
        key: Option<&SimpleChord>,
        representation: Option<&ChordRepresentation>,
        language: Option<usize>,
        scale: Option<f32>,
    ) -> (Vec<String>, String);
}
