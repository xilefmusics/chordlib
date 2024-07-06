use crate::error::Error;

use crate::types::{Line, Part, Section, SimpleChord, Song};

mod iter_part;
mod iter_section;
mod iter_tab;

use iter_part::PartIterator;
use iter_section::SectionIterator;
use iter_tab::TabIterator;

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

pub fn load(url: &str) -> Result<Song, Error> {
    let response = reqwest::blocking::get(url)?;
    if !response.status().is_success() {
        return Err(Error::Other(format!("status ({})", response.status(),)));
    }
    let html = scraper::Html::parse_document(&response.text()?);
    let selector = scraper::Selector::parse("div.js-store").unwrap();
    let element = html
        .select(&selector)
        .next()
        .ok_or(Error::Parse("div.js_store not found".into()))?;
    let json = element
        .value()
        .attr("data-content")
        .ok_or(Error::Parse("data-content not found".into()))?;
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
    load_string(&content, title, artist)
}

pub fn load_string(content: &str, title: &str, artist: &str) -> Result<Song, Error> {
    let sections = SectionIterator::new(&content)
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
    let key = SimpleChord::default();
    Ok(Song {
        title: title.into(),
        key: Some(key),
        artist: Some(artist.into()),
        language: None, // TODO: parse language
        sections,
    }
    .normalize()
    .clone())
}
