pub struct SectionIterator<'a> {
    content: &'a str,
}

impl<'a> SectionIterator<'a> {
    pub fn new(content: &'a str) -> Self {
        Self { content }
    }
}

impl<'a> Iterator for SectionIterator<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        while !self.content.starts_with("[")
            || self.content.starts_with("[tab]")
            || self.content.starts_with("[/tab]")
            || self.content.starts_with("[ch]")
            || self.content.starts_with("[/ch]")
        {
            self.content = &self.content[self.content.find('\n')? + 1..]
        }

        let mut index = None;
        for (byte_index, _) in self.content.char_indices() {
            let next_section = &self.content[byte_index..];
            let first_char_len = next_section.chars().next().map_or(0, |c| c.len_utf8());
            let next_section = &next_section[first_char_len..];
            if next_section.starts_with("[")
                && !next_section.starts_with("[tab]")
                && !next_section.starts_with("[/tab]")
                && !next_section.starts_with("[ch]")
                && !next_section.starts_with("[/ch]")
            {
                index = Some(byte_index);
                break;
            }
        }
        let result = &self.content[..index?].trim_end_matches('\n');
        self.content = &self.content[index?..];

        Some(result)
    }
}
