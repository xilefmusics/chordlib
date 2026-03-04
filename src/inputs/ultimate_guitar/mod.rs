use crate::error::Error;

use crate::types::{Line, Part, Section, SimpleChord, Song};

mod iter_part;
mod iter_section;
mod iter_tab;
mod parse_header;

use iter_part::PartIterator;
use iter_section::SectionIterator;
use iter_tab::TabIterator;

#[cfg(feature = "html")]
fn get_nested_field<'a>(json: &'a serde_json::Value, keys: &[&str]) -> Option<&'a str> {
    let mut current_value = json;
    for key in keys {
        current_value = match current_value.get(key) {
            Some(value) => value,
            None => return None,
        };
    }
    current_value.as_str()
}

#[cfg(feature = "html")]
pub fn load_html(html: &str) -> Result<Song, Error> {
    if html.contains("Just a moment") && html.contains("cf_chl_opt") {
        return Err(Error::Parse(
            "Ultimate Guitar returned a challenge page (e.g. Cloudflare). \
             The site may block simple HTTP clients. \
             Try saving the tab page from a browser and pass the HTML, or use a different network."
                .into(),
        ));
    }
    let html = scraper::Html::parse_document(html);
    let selector = scraper::Selector::parse("div.js-store").unwrap();
    let element = html.select(&selector).next().ok_or_else(|| {
        Error::Parse("div.js-store not found (page may be a challenge or format changed)".into())
    })?;
    let json = element.value().attr("data-content").ok_or_else(|| {
        Error::Parse("data-content not found in js-store (page structure may have changed)".into())
    })?;
    let json: serde_json::Value = serde_json::from_str(json)?;
    let content = get_nested_field(
        &json,
        &["store", "page", "data", "tab_view", "wiki_tab", "content"],
    )
    .ok_or(Error::Parse("content not found".into()))?
    .replace("\r\n", "\n");
    let title = get_nested_field(&json, &["store", "page", "data", "tab", "song_name"])
        .ok_or(Error::Parse("title not found".into()))?;
    let artist = get_nested_field(&json, &["store", "page", "data", "tab", "artist_name"])
        .ok_or(Error::Parse("artist not found".into()))?;
    let key = get_nested_field(&json, &["store", "page", "data", "tab", "tonality_name"])
        .ok_or(Error::Parse("key not found".into()))?;
    load_string(&content, title, artist, key)
}

pub fn load_string(content: &str, title: &str, artist: &str, key: &str) -> Result<Song, Error> {
    let mut section_iter = SectionIterator::new(content);

    let mut tempo = None;
    let mut time = None;
    if let Some(header) = section_iter.next() {
        (tempo, time) = parse_header::parse_header(header);
    }

    let sections = section_iter
        .map(|section| {
            let index = section.find('\n').unwrap();
            let title = section[1..index - 1].to_string();
            let lines = TabIterator::new(&section[index + 1..])
                .map(|tab| {
                    let parts = PartIterator::new(tab).collect::<Result<Vec<Part>, Error>>()?;
                    Ok(Line::new(parts))
                })
                .collect::<Result<Vec<Line>, Error>>()?;
            Ok(Section::new(title, lines))
        })
        .collect::<Result<Vec<Section>, Error>>()?;

    Ok(Song {
        title: title.into(),
        subtitle: None,  // TODO: parse subtitle
        copyright: None, // TODO: parse copyright
        key: Some(SimpleChord::guess_key(key)),
        artist: Some(artist.into()),
        language: None, // TODO: parse language
        tempo,
        time,
        sections,
    }
    .normalize()
    .clone())
}

#[cfg(all(feature = "html", test))]
mod tests {
    use super::*;

    /// Minimal Ultimate Guitar HTML fixture: div.js-store with data-content JSON.
    /// Used to test parsing without network; structure matches UG's store.page.data.*.
    fn fixture_html_with_store() -> String {
        // First block is consumed as header (tempo/time); second block is [Verse 1].
        let content = r#"Tempo: 120
[Verse 1]
[ch]C[/ch] Test lyrics"#;
        let json = serde_json::json!({
            "store": {
                "page": {
                    "data": {
                        "tab_view": {
                            "wiki_tab": {
                                "content": content
                            }
                        },
                        "tab": {
                            "song_name": "Test Song",
                            "artist_name": "Test Artist",
                            "tonality_name": "C"
                        }
                    }
                }
            }
        });
        let json_str = json.to_string().replace('"', "&quot;");
        format!(
            r#"<!DOCTYPE html><html><body><div class="js-store" data-content="{}"></div></body></html>"#,
            json_str
        )
    }

    #[test]
    fn load_html_parses_fixture_ug_page() {
        let html = fixture_html_with_store();
        let song = load_html(&html).expect("load_html should parse fixture");
        assert_eq!(song.title.as_str(), "Test Song");
        assert_eq!(song.artist.as_deref(), Some("Test Artist"));
        assert_eq!(song.sections.len(), 1);
        assert_eq!(song.sections[0].title.as_str(), "Verse 1");
        assert_eq!(song.sections[0].lines.len(), 1);
    }
}
