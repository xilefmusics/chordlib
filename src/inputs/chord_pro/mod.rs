mod iter_part;
mod iter_section;
mod iter_space_section;
use iter_part::PartIterator;
use iter_section::SectionIterator;
use iter_space_section::SpaceSectionIterator;

use crate::error::Error;
use crate::types::{Line, Part, Section, Song};

pub fn load(path: &str) -> Result<Song, Error> {
    load_string(&std::fs::read_to_string(path)?)
}

pub fn load_string(input: &str) -> Result<Song, Error> {
    let mut title = None;
    let mut subtitle = None;
    let mut copyright = None;
    let mut key = None;
    let mut artist = None;
    let mut language = None;
    let mut tempo = None;
    let mut time = None;

    let sections = SectionIterator::new(
        input,
        &mut title,
        &mut subtitle,
        &mut copyright,
        &mut key,
        &mut artist,
        &mut language,
        &mut tempo,
        &mut time,
    )
    .map(|(keyword, lines)| {
        let lines = lines
            .iter()
            .filter(|line| line.len() > 0)
            .map(|line| {
                let parts = PartIterator::new(line, 96).collect::<Result<Vec<Part>, Error>>()?;
                Ok(Line::new(parts))
            })
            .collect::<Result<Vec<Line>, Error>>()?;
        Ok(Section::new(keyword.into(), lines))
    })
    .collect::<Result<Vec<Section>, Error>>()?;

    let sections = if !sections.is_empty() {
        sections
    } else {
        SpaceSectionIterator::new(input)
            .map(|(keyword, lines)| {
                let lines = lines
                    .iter()
                    .filter(|line| line.len() > 0)
                    .map(|line| {
                        let parts =
                            PartIterator::new(line, time.map(|(a, b)| 96 * a / b).unwrap_or(96))
                                .collect::<Result<Vec<Part>, Error>>()?;
                        Ok(Line::new(parts))
                    })
                    .collect::<Result<Vec<Line>, Error>>()?;
                Ok(Section::new(keyword.into(), lines))
            })
            .collect::<Result<Vec<Section>, Error>>()?
    };

    Ok(Song {
        title: title.ok_or(Error::Parse("no title given".into()))?,
        subtitle,
        copyright,
        key: Some((key.ok_or(Error::Parse("no key given".into()))?.as_str()).try_into()?),
        artist,
        language,
        tempo,
        time,
        sections,
    }
    .normalize()
    .clone())
}
