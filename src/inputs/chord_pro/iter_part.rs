use crate::error::Error;
use crate::types::Part;

pub struct PartIterator<'a> {
    line: &'a str,
}

impl<'a> PartIterator<'a> {
    pub fn new(line: &'a str) -> Self {
        Self { line }
    }
}

impl<'a> Iterator for PartIterator<'a> {
    type Item = Result<Part, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(idx) = self.line.find("[") {
            let text = &self.line[..idx];
            if idx != 0 {
                self.line = &self.line[idx..];
                return Some(("", text).try_into());
            } else {
                self.line = &self.line[1..];
                let idx = self.line.find("]").unwrap(); // TODO
                let chord = &self.line[..idx];
                self.line = &self.line[idx + 1..];
                let idx = self.line.find("[").unwrap_or(self.line.len());
                let text = &self.line[..idx];
                self.line = &self.line[idx..];
                return Some((chord, text).try_into());
            }
        }
        if self.line.len() > 0 {
            let text = self.line;
            self.line = "";
            return Some(("", text).try_into());
        }
        None
    }
}
