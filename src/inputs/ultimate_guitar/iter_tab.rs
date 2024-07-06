pub struct TabIterator<'a> {
    content: &'a str,
}

impl<'a> TabIterator<'a> {
    pub fn new(content: &'a str) -> Self {
        Self { content }
    }
}

impl<'a> Iterator for TabIterator<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.content.len() == 0 {
            return None;
        }
        if self.content.starts_with("[tab]") {
            self.content = &self.content[5..];
            let index = self.content.find("[/tab]")?;
            let result = &self.content[..index];
            self.content = &self.content[index + 6..].trim_start_matches('\n');
            Some(result)
        } else {
            if let Some(index) = self.content.find('\n') {
                let result = &self.content[..index];
                self.content = &self.content[index + 1..];
                Some(result)
            } else {
                let result = self.content;
                self.content = "";
                Some(result)
            }
        }
    }
}
