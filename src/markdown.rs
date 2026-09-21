use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FrontMatter {
    pub titles: Vec<String>,
    #[serde(default)]
    pub subtitle: Option<String>,
    #[serde(default)]
    pub copyright: Option<String>,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub artists: Vec<String>,
    #[serde(default)]
    pub languages: Vec<String>,
    #[serde(default)]
    pub tempo: Option<u32>,
    #[serde(default)]
    pub time: Option<String>,
    #[serde(default)]
    pub tags: BTreeMap<String, String>,
}

pub(crate) fn is_plausible_chord_token(token: &str) -> bool {
    let token = token
        .strip_prefix('(')
        .and_then(|value| value.strip_suffix(')'))
        .unwrap_or(token);
    let token = token.split_once(':').map_or(token, |(before, after)| {
        if after.trim().parse::<f64>().is_ok() {
            before
        } else {
            token
        }
    });

    let mut chars = token.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    if matches!(first, '#' | 'b') && chars.clone().next().is_some_and(|c| c.is_ascii_digit()) {
        return valid_chord_suffix(chars.as_str());
    }
    if first.is_ascii_digit() {
        return valid_chord_suffix(chars.as_str());
    }
    if !matches!(first, 'A'..='G' | 'H') {
        return false;
    }
    if matches!(chars.clone().next(), Some('#' | 'b')) {
        chars.next();
    }
    valid_chord_suffix(chars.as_str())
}

fn valid_chord_suffix(suffix: &str) -> bool {
    if suffix.is_empty() {
        return true;
    }
    if let Some(bass) = suffix.strip_prefix('/') {
        return is_simple_root(bass);
    }

    for quality in [
        "sus2", "sus4", "sus", "dim", "aug", "maj", "min", "add", "omit", "no", "m",
    ] {
        if let Some(rest) = suffix.strip_prefix(quality) {
            return valid_chord_suffix(rest);
        }
    }
    if suffix.starts_with('+') || suffix.starts_with('°') {
        return valid_chord_suffix(&suffix[1..]);
    }

    let digit_count = suffix.chars().take_while(char::is_ascii_digit).count();
    if digit_count > 0 {
        return valid_chord_suffix(&suffix[digit_count..]);
    }
    if matches!(suffix.chars().next(), Some('#' | 'b')) {
        let rest = &suffix[1..];
        let digit_count = rest.chars().take_while(char::is_ascii_digit).count();
        return digit_count > 0 && valid_chord_suffix(&rest[digit_count..]);
    }
    false
}

fn is_simple_root(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(root) = chars.next() else {
        return false;
    };
    if !matches!(root, 'A'..='G' | 'H') {
        return false;
    }
    match chars.next() {
        None => true,
        Some('#' | 'b') => chars.next().is_none(),
        Some(_) => false,
    }
}
