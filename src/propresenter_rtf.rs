use encoding_rs::WINDOWS_1252;

use crate::Error;

#[derive(Clone, Copy)]
struct State {
    skip_destination: bool,
    unicode_fallback: usize,
}

impl Default for State {
    fn default() -> Self {
        Self {
            skip_destination: false,
            unicode_fallback: 1,
        }
    }
}

pub(crate) fn decode(input: &[u8]) -> Result<String, Error> {
    if !input.starts_with(b"{\\rtf") {
        return Err(Error::Parse(
            "ProPresenter text element does not contain an RTF document".into(),
        ));
    }

    let mut output = String::new();
    let mut stack = vec![State::default()];
    let mut index = 0usize;
    let mut fallback_left = 0usize;
    let mut unicode_units = Vec::<u16>::new();

    while index < input.len() {
        match input[index] {
            b'{' => {
                let state = stack
                    .last()
                    .copied()
                    .ok_or_else(|| Error::Parse("invalid empty ProPresenter RTF state".into()))?;
                stack.push(state);
                index += 1;
            }
            b'}' => {
                if stack.len() == 1 {
                    return Err(Error::Parse(
                        "unbalanced closing brace in ProPresenter RTF".into(),
                    ));
                }
                flush_unicode(&mut output, &mut unicode_units)?;
                stack.pop();
                index += 1;
            }
            b'\\' => {
                index += 1;
                if index >= input.len() {
                    return Err(Error::Parse(
                        "trailing backslash in ProPresenter RTF".into(),
                    ));
                }
                let state = stack
                    .last_mut()
                    .ok_or_else(|| Error::Parse("invalid empty ProPresenter RTF state".into()))?;
                match input[index] {
                    b'\\' | b'{' | b'}' => {
                        emit_byte(
                            input[index],
                            state.skip_destination,
                            &mut fallback_left,
                            &mut output,
                            &mut unicode_units,
                        )?;
                        index += 1;
                    }
                    b'\'' => {
                        if index + 2 >= input.len() {
                            return Err(Error::Parse("truncated hexadecimal RTF escape".into()));
                        }
                        let value = hex(input[index + 1])?
                            .checked_mul(16)
                            .and_then(|high| hex(input[index + 2]).ok().map(|low| high + low))
                            .ok_or_else(|| Error::Parse("invalid hexadecimal RTF escape".into()))?;
                        emit_cp1252(
                            value,
                            state.skip_destination,
                            &mut fallback_left,
                            &mut output,
                            &mut unicode_units,
                        )?;
                        index += 3;
                    }
                    b'*' => {
                        state.skip_destination = true;
                        index += 1;
                    }
                    symbol if !symbol.is_ascii_alphabetic() => {
                        if !state.skip_destination {
                            match symbol {
                                b'~' => emit_char(
                                    '\u{00a0}',
                                    &mut fallback_left,
                                    &mut output,
                                    &mut unicode_units,
                                )?,
                                b'_' => emit_char(
                                    '\u{2011}',
                                    &mut fallback_left,
                                    &mut output,
                                    &mut unicode_units,
                                )?,
                                b'-' => {}
                                _ => {}
                            }
                        }
                        index += 1;
                    }
                    _ => {
                        let word_start = index;
                        while index < input.len() && input[index].is_ascii_alphabetic() {
                            index += 1;
                        }
                        let word =
                            std::str::from_utf8(&input[word_start..index]).map_err(|_| {
                                Error::Parse("invalid ProPresenter RTF control word".into())
                            })?;
                        let mut sign = 1i32;
                        if input.get(index) == Some(&b'-') {
                            sign = -1;
                            index += 1;
                        }
                        let number_start = index;
                        while index < input.len() && input[index].is_ascii_digit() {
                            index += 1;
                        }
                        let number = if number_start == index {
                            None
                        } else {
                            Some(
                                std::str::from_utf8(&input[number_start..index])
                                    .map_err(|_| {
                                        Error::Parse("invalid ProPresenter RTF number".into())
                                    })?
                                    .parse::<i32>()
                                    .map_err(|_| {
                                        Error::Parse("RTF control number is too large".into())
                                    })?
                                    * sign,
                            )
                        };
                        if input.get(index) == Some(&b' ') {
                            index += 1;
                        }

                        if is_destination(word) {
                            state.skip_destination = true;
                        } else if word == "uc" {
                            let value = number.ok_or_else(|| {
                                Error::Parse("RTF \\uc control has no value".into())
                            })?;
                            state.unicode_fallback = usize::try_from(value).map_err(|_| {
                                Error::Parse("RTF \\uc value must be non-negative".into())
                            })?;
                        } else if word == "u" && !state.skip_destination {
                            let value = number.ok_or_else(|| {
                                Error::Parse("RTF \\u control has no value".into())
                            })?;
                            let unit = (value as i16) as u16;
                            unicode_units.push(unit);
                            fallback_left = state.unicode_fallback;
                        } else if matches!(word, "par" | "line") && !state.skip_destination {
                            emit_char('\n', &mut fallback_left, &mut output, &mut unicode_units)?;
                        } else if word == "tab" && !state.skip_destination {
                            emit_char('\t', &mut fallback_left, &mut output, &mut unicode_units)?;
                        } else if word == "bin" {
                            let count = number.ok_or_else(|| {
                                Error::Parse("RTF \\bin control has no value".into())
                            })?;
                            let count = usize::try_from(count).map_err(|_| {
                                Error::Parse("RTF \\bin value must be non-negative".into())
                            })?;
                            index = index.checked_add(count).ok_or_else(|| {
                                Error::Parse("RTF binary data length overflow".into())
                            })?;
                            if index > input.len() {
                                return Err(Error::Parse("truncated RTF binary data".into()));
                            }
                        }
                    }
                }
            }
            b'\r' | b'\n' => index += 1,
            byte => {
                let skip = stack
                    .last()
                    .ok_or_else(|| Error::Parse("invalid empty ProPresenter RTF state".into()))?
                    .skip_destination;
                if byte.is_ascii() {
                    emit_byte(
                        byte,
                        skip,
                        &mut fallback_left,
                        &mut output,
                        &mut unicode_units,
                    )?;
                } else {
                    emit_cp1252(
                        byte,
                        skip,
                        &mut fallback_left,
                        &mut output,
                        &mut unicode_units,
                    )?;
                }
                index += 1;
            }
        }
    }

    if stack.len() != 1 {
        return Err(Error::Parse(
            "unbalanced opening brace in ProPresenter RTF".into(),
        ));
    }
    flush_unicode(&mut output, &mut unicode_units)?;
    Ok(output.trim_end_matches('\n').to_string())
}

fn is_destination(word: &str) -> bool {
    matches!(
        word,
        "fonttbl"
            | "colortbl"
            | "stylesheet"
            | "info"
            | "pict"
            | "object"
            | "header"
            | "footer"
            | "xmlnstbl"
            | "listtable"
            | "listoverridetable"
            | "revtbl"
            | "datastore"
            | "themedata"
            | "generator"
    )
}

fn hex(byte: u8) -> Result<u8, Error> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(Error::Parse("invalid hexadecimal RTF escape".into())),
    }
}

fn emit_byte(
    byte: u8,
    skip_destination: bool,
    fallback_left: &mut usize,
    output: &mut String,
    unicode_units: &mut Vec<u16>,
) -> Result<(), Error> {
    if skip_destination {
        return Ok(());
    }
    emit_char(byte as char, fallback_left, output, unicode_units)
}

fn emit_cp1252(
    byte: u8,
    skip_destination: bool,
    fallback_left: &mut usize,
    output: &mut String,
    unicode_units: &mut Vec<u16>,
) -> Result<(), Error> {
    if skip_destination {
        return Ok(());
    }
    let input = [byte];
    let (decoded, _, _) = WINDOWS_1252.decode(&input);
    for character in decoded.chars() {
        emit_char(character, fallback_left, output, unicode_units)?;
    }
    Ok(())
}

fn emit_char(
    character: char,
    fallback_left: &mut usize,
    output: &mut String,
    unicode_units: &mut Vec<u16>,
) -> Result<(), Error> {
    if *fallback_left > 0 {
        *fallback_left -= 1;
        return Ok(());
    }
    flush_unicode(output, unicode_units)?;
    output.push(character);
    Ok(())
}

fn flush_unicode(output: &mut String, units: &mut Vec<u16>) -> Result<(), Error> {
    for decoded in char::decode_utf16(units.drain(..)) {
        output
            .push(decoded.map_err(|_| Error::Parse("invalid UTF-16 in ProPresenter RTF".into()))?);
    }
    Ok(())
}

pub(crate) fn encode(text: &str) -> Vec<u8> {
    let mut output = String::from(
        "{\\rtf1\\ansi\\ansicpg1252\\deff0{\\fonttbl{\\f0 Arial;}}\\viewkind4\\uc1\\pard\\qc\\f0\\fs144 ",
    );
    for character in text.chars() {
        match character {
            '\\' => output.push_str("\\\\"),
            '{' => output.push_str("\\{"),
            '}' => output.push_str("\\}"),
            '\n' => output.push_str("\\line "),
            '\t' => output.push_str("\\tab "),
            character if character.is_ascii() => output.push(character),
            character => {
                let mut buffer = [0u16; 2];
                for unit in character.encode_utf16(&mut buffer) {
                    output.push_str("\\u");
                    output.push_str(&((*unit as i16) as i32).to_string());
                    output.push('?');
                }
            }
        }
    }
    output.push('}');
    output.into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_unicode_and_reserved_characters() {
        let text = "Grüße 😊\\{test}\nnext\tcolumn";
        assert_eq!(decode(&encode(text)).expect("decode generated RTF"), text);
    }

    #[test]
    fn decodes_destinations_hex_and_controls() {
        let input =
            br#"{\rtf1\ansi\ansicpg1252{\fonttbl{\f0 Arial;}}\uc1 A\'e4 \u214?\line B\tab C}"#;
        assert_eq!(decode(input).expect("decode RTF"), "Aä Ö\nB\tC");
    }

    #[test]
    fn rejects_unbalanced_and_non_rtf_input() {
        assert!(decode(b"plain").is_err());
        assert!(decode(br#"{\rtf1 broken"#).is_err());
        assert!(decode(br#"{\rtf1 broken}}"#).is_err());
    }
}
