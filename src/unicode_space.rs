//! Map Unicode “space separator” (Zs) and common typographic space code points to U+0020.
//! See <https://www.unicode.org/reports/tr44/#General_Category_Values> (category Zs).
use std::borrow::Cow;

/// `true` for characters in the Unicode `Space_Separator` (Zs) class, except the ordinary
/// ASCII space (already U+0020). Unicode defines these as distinct from tabs and newlines
/// (Cc, or Zl / Zp line/paragraph break).
const fn is_non_ascii_zs(c: char) -> bool {
    matches!(
        c,
        '\u{00A0}' // NO-BREAK SPACE
            | '\u{1680}' // OGHAM SPACE MARK
            | '\u{2000}'
            ..='\u{200A}' // EN QUAD … HAIR SPACE
            | '\u{202F}' // NARROW NO-BREAK SPACE
            | '\u{205F}' // MEDIUM MATHEMATICAL SPACE
            | '\u{3000}' // IDEOGRAPHIC SPACE
    )
}

/// Replaces every non-ASCII `Space_Separator` (Zs) with `U+0020`. Tabs, newlines, and other
/// non-Zs control characters are left unchanged, so this is safe for line-oriented text.
///
/// If the string is unchanged, returns [`Cow::Borrowed`] (no allocation).
pub fn normalize_space_separators(s: &str) -> Cow<'_, str> {
    if !s.chars().any(is_non_ascii_zs) {
        return Cow::Borrowed(s);
    }
    Cow::Owned(
        s.chars()
            .map(|c| if is_non_ascii_zs(c) { ' ' } else { c })
            .collect(),
    )
}

/// Removes every `Space_Separator` (Zs) character that [`normalize_space_separators`] rewrites. Use
/// this for **machine** tokens (e.g. a chord key `C#` or `Dm`) so a typographic space between `C`
/// and `#` is dropped instead of becoming `U+0020`, which would not match a strict parse.
pub fn remove_space_separators(s: &str) -> Cow<'_, str> {
    if !s.chars().any(is_non_ascii_zs) {
        return Cow::Borrowed(s);
    }
    Cow::Owned(s.chars().filter(|&c| !is_non_ascii_zs(c)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_change_when_no_zs() {
        let s = "C D\n\t";
        let c = normalize_space_separators(s);
        assert!(matches!(c, Cow::Borrowed(_)));
        assert_eq!(c.as_ref(), s);
    }

    #[test]
    fn nnbsp_nbsp_ideograph_space_to_ascii() {
        let s = format!("A\u{00A0}B\u{202F}C\u{3000}D");
        let t = normalize_space_separators(&s);
        assert_eq!(t.as_ref(), "A B C D");
    }

    #[test]
    fn medium_mathematical_space() {
        assert_eq!(normalize_space_separators("So\u{205f}pl").as_ref(), "So pl");
    }

    #[test]
    fn remove_mms_between_letters() {
        assert_eq!(remove_space_separators("C\u{205F}#"), "C#");
    }
}
