use std::collections::BTreeMap;

pub struct SectionIterator<'a> {
    title: &'a mut Option<String>,
    titles: &'a mut Option<Vec<String>>,
    subtitle: &'a mut Option<String>,
    copyright: &'a mut Option<String>,
    key: &'a mut Option<String>,
    artist: &'a mut Option<String>,
    artists: &'a mut Option<Vec<String>>,
    language: &'a mut Option<String>,
    languages: &'a mut Option<Vec<String>>,
    tempo: &'a mut Option<u32>,
    time: &'a mut Option<(u32, u32)>,
    tags: &'a mut BTreeMap<String, String>,
    section_title_cache: Option<&'a str>,
    lines_cache: Vec<&'a str>,
    lines: std::str::Lines<'a>,
}

impl<'a> SectionIterator<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        content: &'a str,
        title: &'a mut Option<String>,
        titles: &'a mut Option<Vec<String>>,
        subtitle: &'a mut Option<String>,
        copyright: &'a mut Option<String>,
        key: &'a mut Option<String>,
        artist: &'a mut Option<String>,
        artists: &'a mut Option<Vec<String>>,
        language: &'a mut Option<String>,
        languages: &'a mut Option<Vec<String>>,
        tempo: &'a mut Option<u32>,
        time: &'a mut Option<(u32, u32)>,
        tags: &'a mut BTreeMap<String, String>,
    ) -> Self {
        Self {
            title,
            titles,
            subtitle,
            copyright,
            key,
            artist,
            artists,
            language,
            languages,
            tempo,
            time,
            tags,
            section_title_cache: None,
            lines_cache: Vec::default(),
            lines: content.lines(),
        }
    }
    fn parse_key_value(input: &str) -> Option<(&str, &str)> {
        let input = input.trim();
        if input.starts_with('{') && input.ends_with('}') {
            let inner = &input[1..input.len() - 1];
            if let Some(index) = inner.find(':') {
                let (key, value) = inner.split_at(index);
                let key = key.trim();
                let value = value[1..].trim();
                if !key.is_empty() {
                    return Some((key, value));
                }
            }
        }
        None
    }

    /// Parse a simple directive value, stripping a single pair of surrounding double quotes
    /// if present, but otherwise preserving whitespace.
    fn parse_simple_value(value: &str) -> String {
        let trimmed = value.trim();
        if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
            trimmed[1..trimmed.len() - 1].to_string()
        } else {
            trimmed.to_string()
        }
    }

    /// Parse keys like `title`, `title2`, `language3`, `artist4`, returning a zero-based index.
    fn parse_indexed_key(key: &str, prefix: &str) -> Option<usize> {
        if key == prefix {
            return Some(0);
        }
        key.strip_prefix(prefix)
            .and_then(|rest| rest.parse::<usize>().ok())
            .and_then(|n| (n >= 1).then(|| n.saturating_sub(1)))
    }
}

impl<'a> Iterator for SectionIterator<'a> {
    type Item = (&'a str, Vec<&'a str>);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(line) = self.lines.next() {
                if let Some((key, value)) = Self::parse_key_value(line) {
                    // Handle indexed metadata directives: title/titleN, language/languageN, artist/artistN.
                    if let Some(idx) = Self::parse_indexed_key(key, "title") {
                        let title_val = Self::parse_simple_value(value);
                        let titles_vec = self.titles.get_or_insert_with(Vec::new);
                        if titles_vec.len() <= idx {
                            titles_vec.resize(idx + 1, String::new());
                        }
                        titles_vec[idx] = title_val.clone();
                        if idx == 0 || self.title.is_none() {
                            *self.title = Some(title_val);
                        }
                    } else if let Some(idx) = Self::parse_indexed_key(key, "language") {
                        let lang_val = Self::parse_simple_value(value);
                        let langs_vec = self.languages.get_or_insert_with(Vec::new);
                        if langs_vec.len() <= idx {
                            langs_vec.resize(idx + 1, String::new());
                        }
                        langs_vec[idx] = lang_val.clone();
                        if idx == 0 || self.language.is_none() {
                            *self.language = Some(lang_val);
                        }
                    } else if let Some(idx) = Self::parse_indexed_key(key, "artist") {
                        let artist_val = Self::parse_simple_value(value);
                        let artists_vec = self.artists.get_or_insert_with(Vec::new);
                        if artists_vec.len() <= idx {
                            artists_vec.resize(idx + 1, String::new());
                        }
                        artists_vec[idx] = artist_val.clone();
                        if idx == 0 || self.artist.is_none() {
                            *self.artist = Some(artist_val);
                        }
                    } else {
                        match key {
                            "meta" => {
                                let value_trimmed = value.trim();
                                if let Some(space_pos) = value_trimmed.find(char::is_whitespace) {
                                    let (name, val) = value_trimmed.split_at(space_pos);
                                    let name = name.trim();
                                    let val = val.trim();
                                    if !name.is_empty() {
                                        self.tags.insert(name.to_string(), val.to_string());
                                    }
                                } else if !value_trimmed.is_empty() {
                                    self.tags.insert(value_trimmed.to_string(), String::new());
                                }
                            }
                            "subtitle" => *self.subtitle = Some(value.into()),
                            "copyright" => *self.copyright = Some(value.into()),
                            "key" => *self.key = Some(value.into()),
                            "tempo" => *self.tempo = value.parse().ok(),
                            "time" => {
                                *self.time = value
                                    .split_once('/')
                                    .and_then(|(a, b)| Some((a.parse().ok()?, b.parse().ok()?)))
                            }
                            "section" => {
                                if let Some(title) = self.section_title_cache {
                                    let lines_cache = std::mem::take(&mut self.lines_cache);
                                    self.section_title_cache = Some(value);
                                    return Some((title, lines_cache));
                                } else {
                                    self.section_title_cache = Some(value);
                                    self.lines_cache = Vec::default();
                                }
                            }
                            _ => self.lines_cache.push(line.trim_end()),
                        }
                    }
                } else {
                    self.lines_cache.push(line.trim_end());
                }
            } else if let Some(title) = self.section_title_cache {
                let lines_cache = std::mem::take(&mut self.lines_cache);
                self.section_title_cache = None;
                return Some((title, lines_cache));
            } else {
                return None;
            }
        }
    }
}
