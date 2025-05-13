use crate::error::Error;
use crate::types::{Chord, Part};

pub struct PartIterator<'a> {
    bar_duration: u32,
    line: &'a str,
    chord_cache: Vec<Chord>,
}

impl<'a> PartIterator<'a> {
    pub fn new(line: &'a str, bar_duration: u32) -> Self {
        Self {
            bar_duration,
            line: line.trim(),
            chord_cache: Vec::new(),
        }
    }

    fn handle_parenthesis(&mut self) -> Result<Part, Error> {
        let Some(idx) = self.line.find(')') else {
            return Err(Error::Parse(
                "failed parsing comment: no closing `)`".into(),
            ));
        };

        let comment = &self.line[..idx + 1];
        self.line = &self.line[idx + 1..];
        return Ok(Part::new_comment(comment.to_string()));
    }

    fn handle_curly_brace(&mut self) -> Result<Part, Error> {
        let Some(idx) = self.line.find('}') else {
            return Err(Error::Parse(
                "failed parsing comment: no closing `}`".into(),
            ));
        };

        let comment = &self.line[1..idx];
        self.line = &self.line[idx + 1..];

        let Some(colon_idx) = comment.find(':') else {
            return Err(Error::Parse(format!(
                "failed parsing comment (no `:`): {comment}"
            )));
        };
        let (key, value) = comment.split_at(colon_idx);

        if matches!(key, "c" | "cb" | "ci" | "comment") {
            return Ok(Part::new_comment(value.to_string()));
        }

        Err(Error::Parse(format!("failed parsing comment: {comment}")))
    }

    fn handle_bracket(&mut self) -> Option<Result<Part, Error>> {
        self.line = &self.line[1..];

        let Some(idx) = self.line.find(']') else {
            return Some(Err(Error::Parse("failed parsing chord: no closing".into())));
        };

        let chord = &self.line[..idx];
        self.line = &self.line[idx + 1..];

        if chord == "|" {
            return self.handle_pipe_chord();
        }

        let idx = self.line.find(['[', '{']).unwrap_or(self.line.len());
        let (text, rest) = self.line.split_at(idx);
        self.line = rest;

        Some((chord, text).try_into())
    }

    fn handle_pipe_chord(&mut self) -> Option<Result<Part, Error>> {
        while let Some(start_idx) = self.line.find('[') {
            let Some(end_idx) = self.line.find(']') else {
                return Some(Err(Error::Parse("failed parsing chord: no closing".into())));
            };
            let chord = &self.line[start_idx + 1..end_idx];

            if chord == "|" {
                self.line = &self.line[start_idx..];
                break;
            }

            self.chord_cache.push(match chord.parse::<Chord>() {
                Ok(c) => c,
                Err(e) => return Some(Err(e)),
            });

            self.line = &self.line[end_idx + 1..];
        }

        if self.line.trim().len() == 0 {
            return None;
        }

        if self.chord_cache.len() == 0 {
            return Some(Err(Error::Parse("bar does not contain chords".into())));
        };

        let duration = self.bar_duration / self.chord_cache.len() as u32;
        let cache = std::mem::take(&mut self.chord_cache); // replaces it with an empty vec
        self.chord_cache = cache
            .into_iter()
            .map(|chord| chord.duration(duration))
            .collect();

        Some(Ok(Part::new_chord(self.chord_cache.remove(0))))
    }
}

impl<'a> Iterator for PartIterator<'a> {
    type Item = Result<Part, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.chord_cache.len() > 0 {
            return Some(Ok(Part::new_chord(self.chord_cache.remove(0))));
        }

        if let Some((token, idx)) = ["[", "{", "("]
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
                return Some(self.handle_curly_brace());
            }

            if token == "(" {
                return Some(self.handle_parenthesis());
            }

            if token == "[" {
                return self.handle_bracket();
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
