use crate::Error;
use crate::types::{ChordRepresentation, SimpleChord};

pub trait FormatHTML {
    fn format_html(
        &self,
        key: Option<&SimpleChord>,
        representation: Option<&ChordRepresentation>,
        language: Option<usize>,
        scale: Option<f32>,
    ) -> Result<String, Error>;
    fn format_html_page(
        &self,
        key: Option<&SimpleChord>,
        representation: Option<&ChordRepresentation>,
        language: Option<usize>,
        scale: Option<f32>,
    ) -> Result<(String, String), Error>;
    /// Render each song section as HTML (`<p><span class="keyword">…</span>…</p>`).
    /// Returns `(section_htmls, css)` — no `.page` wrapper, header, or footer.
    fn format_html_sections(
        &self,
        key: Option<&SimpleChord>,
        representation: Option<&ChordRepresentation>,
        language: Option<usize>,
        scale: Option<f32>,
    ) -> Result<(Vec<String>, String), Error>;
}

/// Source-compatible capo-aware additions to [`FormatHTML`].
pub trait FormatHTMLWithCapo {
    /// Render HTML with the sounding key in the key label while preserving the
    /// selected chord shapes.
    fn format_html_with_capo(
        &self,
        key: Option<&SimpleChord>,
        representation: Option<&ChordRepresentation>,
        language: Option<usize>,
        scale: Option<f32>,
        capo: u8,
    ) -> Result<String, Error>;

    /// Render the page body and CSS with the sounding key and unchanged chord shapes.
    fn format_html_page_with_capo(
        &self,
        key: Option<&SimpleChord>,
        representation: Option<&ChordRepresentation>,
        language: Option<usize>,
        scale: Option<f32>,
        capo: u8,
    ) -> Result<(String, String), Error>;

    /// Render each song section while preserving the selected chord shapes.
    fn format_html_sections_with_capo(
        &self,
        key: Option<&SimpleChord>,
        representation: Option<&ChordRepresentation>,
        language: Option<usize>,
        scale: Option<f32>,
        capo: u8,
    ) -> Result<(Vec<String>, String), Error>;
}
