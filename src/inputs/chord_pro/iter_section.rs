pub struct SectionIterator<'a> {
    title: &'a mut Option<String>,
    key: &'a mut Option<String>,
    artist: &'a mut Option<String>,
    language: &'a mut Option<String>,
    section_title_cache: Option<&'a str>,
    lines_cache: Vec<&'a str>,
    lines: std::str::Lines<'a>,
}

impl<'a> SectionIterator<'a> {
    pub fn new(
        content: &'a str,
        title: &'a mut Option<String>,
        key: &'a mut Option<String>,
        artist: &'a mut Option<String>,
        language: &'a mut Option<String>,
    ) -> Self {
        Self {
            title,
            key,
            artist,
            language,
            section_title_cache: None,
            lines_cache: Vec::default(),
            lines: content.lines(),
        }
    }
    fn parse_key_value(input: &str) -> Option<(&str, &str)> {
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
}

impl<'a> Iterator for SectionIterator<'a> {
    type Item = (&'a str, Vec<&'a str>);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(line) = self.lines.next() {
                if let Some((key, value)) = Self::parse_key_value(line) {
                    match key {
                        "title" => *self.title = Some(value.into()),
                        "key" => *self.key = Some(value.into()),
                        "artist" => *self.artist = Some(value.into()),
                        "language" => *self.language = Some(value.into()),
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
                        _ => self.lines_cache.push(line),
                    }
                } else {
                    self.lines_cache.push(line);
                }
            } else {
                if let Some(title) = self.section_title_cache {
                    let lines_cache = std::mem::take(&mut self.lines_cache);
                    self.section_title_cache = None;
                    return Some((title, lines_cache));
                } else {
                    return None;
                }
            }
        }
    }
}
