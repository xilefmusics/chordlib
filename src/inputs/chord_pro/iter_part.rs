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
        if let Some((token, idx)) = ["[", "{"]
            .iter()
            .filter_map(|&s| self.line.find(s).map(|pos| (s, pos)))
            .min_by_key(|&(_, pos)| pos)
        {
            if idx != 0 {
                let text = &self.line[..idx];
                self.line = &self.line[idx..];
                return Some(("", text).try_into());
            }

            if token == "{" {
                let idx = match self.line.find("}") {
                    Some(idx) => idx,
                    None => {
                        return Some(Err(Error::Parse(
                            "failed parsing comment: no closing }".into(),
                        )))
                    }
                };
                let comment = &self.line[1..idx];
                self.line = &self.line[idx + 1..];
                let mut iter = comment.split(":");
                let key = iter.next().unwrap().trim();
                let value = iter.next().unwrap().trim();
                if key == "c" || key == "cb" || key == "ci" || key == "comment" {
                    return Some(Ok(Part::new_comment(value.to_string())));
                }
                return Some(Err(Error::Parse(format!(
                    "failed parsing comment: {comment}"
                ))));
            }

            if token == "[" {
                self.line = &self.line[1..];
                let idx = match self.line.find("]") {
                    Some(idx) => idx,
                    None => {
                        return Some(Err(Error::Parse(
                            "failed parsing chord: no closing ".into(),
                        )))
                    }
                };
                let chord = &self.line[..idx];
                self.line = &self.line[idx + 1..];

                let idx = ["[", "{"]
                    .iter()
                    .filter_map(|&s| self.line.find(s).map(|pos| (s, pos)))
                    .min_by_key(|&(_, pos)| pos)
                    .map(|elem| elem.1)
                    .unwrap_or(self.line.len());
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
