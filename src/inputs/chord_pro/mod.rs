mod iter_part;
mod iter_section;
use iter_part::PartIterator;
use iter_section::SectionIterator;

use crate::error::Error;
use crate::types::{Line, Part, Section, Song};

pub fn load(path: &str) -> Result<Song, Error> {
    load_string(&std::fs::read_to_string(path)?)
}

pub fn load_string(input: &str) -> Result<Song, Error> {
    let mut title = None;
    let mut key = None;
    let mut artist = None;
    let mut language = None;

    let sections = SectionIterator::new(input, &mut title, &mut key, &mut artist, &mut language)
        .map(|(keyword, lines)| {
            let lines = lines
                .iter()
                .map(|line| {
                    let parts = PartIterator::new(line).collect::<Result<Vec<Part>, Error>>()?;
                    Ok(Line::new(parts))
                })
                .collect::<Result<Vec<Line>, Error>>()?;
            Ok(Section::new(keyword.into(), lines))
        })
        .collect::<Result<Vec<Section>, Error>>()?;

    Ok(Song {
        title: title.ok_or(Error::Parse("no title given".into()))?,
        key: Some((key.ok_or(Error::Parse("no key given".into()))?.as_str()).try_into()?),
        artist,
        language,
        sections,
    }
    .normalize()
    .clone())
}
