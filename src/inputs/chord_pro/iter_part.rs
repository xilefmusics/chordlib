use crate::error::Error;
use crate::types::{Chord, Part, SimpleChord};

pub struct PartIterator<'a> {
    bar_duration: u32,
    line: &'a str,
    chord_cache: Vec<Chord>,
    song_key: Option<SimpleChord>,
}

impl<'a> PartIterator<'a> {
    pub fn new(line: &'a str, bar_duration: u32, song_key: Option<SimpleChord>) -> Self {
        Self {
            bar_duration,
            line: line.trim(),
            chord_cache: Vec::new(),
            song_key,
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
        Ok(Part::new_comment(comment.to_string()))
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
            return Ok(Part::new_comment((&value[1..].trim()).to_string()));
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

        // CCLI SongSelect repeat markers [||:] and [:||] are not chords; strip them
        // so import does not fail. See https://github.com/xilefmusics/chordlib/issues/6
        let is_repeat_marker = matches!(chord.trim(), "||:" | ":||");

        let idx = self.line.find(['[', '{']).unwrap_or(self.line.len());
        let (text, rest) = self.line.split_at(idx);
        self.line = rest;

        if is_repeat_marker {
            return Some(("", text).try_into());
        }
        let chord_opt = match chord.trim() {
            "" => None,
            s => match Chord::from_str_with_key(s, self.song_key.as_ref()) {
                Ok(c) => Some(c),
                Err(e) => return Some(Err(e)),
            },
        };
        Some(Ok(Part {
            chord: chord_opt,
            languages: vec![text.to_string()],
            comment: false,
        }))
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

            // `[]` inside a pipe bar: same as standalone `[]` in `handle_bracket` (no chord)
            if chord.trim().is_empty() {
                self.line = &self.line[end_idx + 1..];
                continue;
            }

            self.chord_cache.push(
                match Chord::from_str_with_key(chord, self.song_key.as_ref()) {
                    Ok(c) => c,
                    Err(e) => return Some(Err(e)),
                },
            );

            self.line = &self.line[end_idx + 1..];
        }

        if self.line.trim().is_empty() {
            return None;
        }

        if self.chord_cache.is_empty() {
            return Some(Err(Error::Parse("bar does not contain chords".into())));
        };

        let n = self.chord_cache.len();
        let base = self.bar_duration / n as u32;
        // `bar_duration / n` truncates; assign the remainder to the last chord so each pipe bar
        // partitions the measure exactly and the next `[|]` never inherits a sub-beat gap.
        let last_duration = self.bar_duration - base * (n as u32 - 1);
        let cache = std::mem::take(&mut self.chord_cache);
        self.chord_cache = cache
            .into_iter()
            .enumerate()
            .map(|(i, chord)| {
                let duration = if i + 1 == n { last_duration } else { base };
                chord.duration(duration)
            })
            .collect();

        Some(Ok(Part::new_chord(self.chord_cache.remove(0))))
    }
}

impl<'a> Iterator for PartIterator<'a> {
    type Item = Result<Part, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.chord_cache.is_empty() {
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

        if !self.line.is_empty() {
            let text = self.line;
            self.line = "";
            return Some(("", text).try_into());
        }
        None
    }
}
