use crate::error::Error;
use crate::types::Part;

pub struct PartIterator<'a> {
    content_chord: Option<&'a str>,
    content_text: Option<&'a str>,
}

impl<'a> PartIterator<'a> {
    pub fn new(content: &'a str) -> Self {
        let vec = content.split('\n').collect::<Vec<&str>>();
        if vec.len() > 1 {
            Self {
                content_chord: Some(vec[0]),
                content_text: Some(vec[1]),
            }
        } else if vec.len() == 1 {
            if vec[0].find("[ch]").is_some() {
                Self {
                    content_chord: Some(vec[0]),
                    content_text: None,
                }
            } else {
                Self {
                    content_chord: None,
                    content_text: Some(vec[0]),
                }
            }
        } else {
            Self {
                content_chord: None,
                content_text: None,
            }
        }
    }
}

impl<'a> Iterator for PartIterator<'a> {
    type Item = Result<Part, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(mut content_chord) = self.content_chord {
            let old_content_chord = content_chord;
            let start = content_chord.find("[ch]")?;
            if start > 0 {
                if let Some(content_text) = self.content_text {
                    let byte_index = content_text
                        .chars()
                        .take(start)
                        .map(|b| b.len_utf8())
                        .sum::<usize>();
                    let text = &content_text[..byte_index];
                    self.content_text = Some(&content_text[byte_index..]);
                    self.content_chord = Some(&content_chord[start..]);
                    return Some(("", text).try_into());
                }
            }
            content_chord = &content_chord[start + 4..];
            let end = content_chord.find("[/ch]")?;
            let chord = &content_chord[..end];
            let next_start = content_chord.find("[ch]").unwrap_or(content_chord.len());
            content_chord = &content_chord[next_start..];
            self.content_chord = Some(content_chord);
            if let Some(content_text) = self.content_text {
                let text = if content_chord.len() > 0 {
                    let char_index =
                        old_content_chord.chars().count() - content_chord.chars().count() - 9;
                    let byte_index = content_text
                        .chars()
                        .take(char_index)
                        .map(|b| b.len_utf8())
                        .sum::<usize>();
                    let text = &content_text[..byte_index];
                    self.content_text = Some(&content_text[byte_index..]);
                    text
                } else {
                    let text = content_text;
                    self.content_text = None;
                    text
                };
                Some((chord, text).try_into())
            } else {
                Some((chord, "").try_into())
            }
        } else {
            Some(("", self.content_text.take()?).try_into())
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn part_iterator() {
        let inputs = vec![
            "",
            "[ch]A[/ch]",
            "[ch]A[/ch] [ch]B[/ch]",
            "[ch]A[/ch]    [ch]B[/ch]\nHeyö you",
            "[ch]A[/ch]          [ch]B[/ch]\nHey",
            "    [ch]A[/ch]\nHey you",
            "Hello World",
        ];
        let outputs = vec![
            vec![("", "").try_into().unwrap()],
            vec![("A", "").try_into().unwrap()],
            vec![("A", "").try_into().unwrap(), ("B", "").try_into().unwrap()],
            vec![
                ("A", "Heyö ").try_into().unwrap(),
                ("B", "you").try_into().unwrap(),
            ],
            vec![
                ("A", "Hey").try_into().unwrap(),
                ("B", "").try_into().unwrap(),
            ],
            vec![
                ("", "Hey ").try_into().unwrap(),
                ("A", "you").try_into().unwrap(),
            ],
            vec![("", "Hello World").try_into().unwrap()],
        ];
        for (input, output) in inputs.iter().zip(outputs.iter()) {
            assert_eq!(
                &PartIterator::new(input)
                    .collect::<Result<Vec<Part>, Error>>()
                    .unwrap(),
                output
            );
        }
    }
}
