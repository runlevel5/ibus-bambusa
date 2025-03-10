//! Output charset encoding: convert composed Unicode text into a target
//! Vietnamese charset.

use std::collections::HashMap;
use std::sync::LazyLock;

use crate::charset::CHARSETS;

const UNICODE: &str = "Unicode";

#[allow(clippy::type_complexity)]
static CHARSET_MAPS: LazyLock<HashMap<&'static str, HashMap<char, &'static str>>> =
    LazyLock::new(|| {
        CHARSETS
            .iter()
            .map(|(name, entries)| (*name, entries.iter().copied().collect()))
            .collect()
    });

/// Encode composed Unicode `input` into the named output charset.
///
/// `"Unicode"` (and any unknown charset) passes through unchanged; otherwise
/// each character is mapped through the charset table, with unmapped
/// characters emitted as-is.
pub fn encode(charset: &str, input: &str) -> String {
    if charset == UNICODE {
        return input.to_string();
    }
    let Some(map) = CHARSET_MAPS.get(charset) else {
        return input.to_string();
    };
    let mut out = String::with_capacity(input.len());
    for chr in input.chars() {
        match map.get(&chr) {
            Some(s) => out.push_str(s),
            None => out.push(chr),
        }
    }
    out
}

/// Names of every supported output charset, `"Unicode"` first.
pub fn charset_names() -> Vec<&'static str> {
    let mut names = vec![UNICODE];
    names.extend(CHARSETS.iter().map(|(name, _)| *name));
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unicode_passes_through() {
        assert_eq!(encode("Unicode", "tiếng việt"), "tiếng việt");
        assert_eq!(encode("Totally unknown", "đ"), "đ");
    }

    #[test]
    fn ncr_decimal_and_hex() {
        assert_eq!(encode("NCR Decimal", "đ"), "&#273;");
        assert_eq!(encode("NCR Hex", "đ"), "&#x111;");
        // ASCII characters are not in the table, so they pass through.
        assert_eq!(encode("NCR Decimal", "abc"), "abc");
    }

    #[test]
    fn c_string_charsets_emit_literal_escapes() {
        assert_eq!(encode("Unicode C string Hex", "đ"), r"\x111");
        assert_eq!(encode("Unicode C string Decimal", "đ"), r"\u273");
    }

    #[test]
    fn names_include_unicode_and_legacy() {
        let names = charset_names();
        assert_eq!(names[0], "Unicode");
        assert!(names.contains(&"TCVN3 (ABC)"));
        assert!(names.contains(&"VNI Windows"));
        assert_eq!(names.len(), 17); // Unicode + 16 tables
    }
}
