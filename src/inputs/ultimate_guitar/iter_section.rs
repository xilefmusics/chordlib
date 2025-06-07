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
        if self.content.is_empty() {
            return None;
        }

        let cutoff = self
            .content
            .lines()
            .enumerate()
            .take_while(|(idx, line)| {
                *idx == 0
                    || !line.starts_with('[')
                    || line.starts_with("[tab")
                    || line.starts_with("[/tab")
                    || line.starts_with("[ch")
                    || line.starts_with("[/ch")
            })
            .map(|(_, line)| line.len() + 1)
            .sum::<usize>();

        let section = &self.content[..cutoff.min(self.content.len())];
        self.content = &self.content[cutoff.min(self.content.len())..];
        Some(section)
    }
}
