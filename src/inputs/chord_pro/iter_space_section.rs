pub struct SpaceSectionIterator<'a> {
    lines_cache: Vec<&'a str>,
    keyword_cache: &'a str,
    lines: std::str::Lines<'a>,
}

impl<'a> SpaceSectionIterator<'a> {
    pub fn new(content: &'a str) -> Self {
        Self {
            lines_cache: Vec::default(),
            keyword_cache: "",
            lines: content.lines(),
        }
    }
}

impl<'a> Iterator for SpaceSectionIterator<'a> {
    type Item = (&'a str, Vec<&'a str>);

    fn next(&mut self) -> Option<Self::Item> {
        let mut keyword = self.keyword_cache;

        while let Some(line) = self.lines.next() {
            let mut trimmed = line.trim();

            if trimmed.starts_with('{') && trimmed.ends_with('}') {
                continue;
            }

            if trimmed.is_empty() {
                let Some(line) = self.lines.next() else {
                    break;
                };
                trimmed = line.trim();
                if trimmed.ends_with(':') {
                    self.keyword_cache = trimmed.strip_suffix(':').unwrap_or("unknown");
                    if self.lines_cache.is_empty() {
                        keyword = self.keyword_cache;
                        continue;
                    }
                    break;
                }
            }
            self.lines_cache.push(trimmed);
        }

        if self.lines_cache.is_empty() {
            return None;
        }

        let lines = std::mem::take(&mut self.lines_cache);
        Some((keyword, lines))
    }
}
