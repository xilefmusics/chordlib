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
        current_value = current_value.get(key)?;
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

/// True if this block starts with a section title line like [Intro] or [Verse 1],
/// not [tab], [ch], etc.
fn block_is_section(block: &str) -> bool {
    let first = block.lines().next().map(str::trim).unwrap_or("");
    first.starts_with('[')
        && first.ends_with(']')
        && !first.starts_with("[tab")
        && !first.starts_with("[/tab")
        && !first.starts_with("[ch")
        && !first.starts_with("[/ch")
}

fn parse_section_block(section: &str) -> Result<Section, Error> {
    let index = section
        .find('\n')
        .ok_or_else(|| Error::Parse("section block has no newline".into()))?;
    let section_title = section[1..index - 1].to_string();
    let lines = TabIterator::new(&section[index + 1..])
        .map(|tab| {
            let parts = PartIterator::new(tab).collect::<Result<Vec<Part>, Error>>()?;
            Ok(Line::new(parts))
        })
        .collect::<Result<Vec<Line>, Error>>()?;
    Ok(Section::new(section_title, lines))
}

pub fn load_string(content: &str, title: &str, artist: &str, key: &str) -> Result<Song, Error> {
    let mut section_iter = SectionIterator::new(content);

    let mut tempo = None;
    let mut time = None;
    let mut sections = Vec::new();

    if let Some(first) = section_iter.next() {
        (tempo, time) = parse_header::parse_header(first);
        if block_is_section(first) {
            sections.push(parse_section_block(first)?);
        }
    }

    let rest: Vec<Section> = section_iter
        .map(parse_section_block)
        .collect::<Result<Vec<Section>, Error>>()?;
    sections.extend(rest);

    Ok(Song {
        titles: vec![title.into()],
        subtitle: None,  // TODO: parse subtitle
        copyright: None, // TODO: parse copyright
        key: Some(SimpleChord::guess_key(key)),
        artists: vec![artist.into()],
        languages: vec![],
        tempo,
        time,
        tags: Default::default(),
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
        assert_eq!(song.title(), "Test Song");
        assert_eq!(song.artist(), "Test Artist");
        assert_eq!(song.sections.len(), 1);
        assert_eq!(song.sections[0].title.as_str(), "Verse 1");
        assert_eq!(song.sections[0].lines.len(), 1);
    }

    /// First block can be a real section (e.g. [Intro]); it must not be dropped.
    /// See https://github.com/xilefmusics/chordlib/issues/15
    #[test]
    fn first_section_not_lost() {
        let content = r#"[Intro]
[ch]C[/ch] [ch]G[/ch]

[Verse 1]
[ch]Am[/ch] lyrics
"#;
        let song = load_string(content, "Song", "Artist", "C").expect("parse");
        assert_eq!(song.sections.len(), 2, "Intro and Verse 1");
        assert_eq!(song.sections[0].title.as_str(), "Intro");
        assert_eq!(song.sections[1].title.as_str(), "Verse 1");
    }
}
