//! Output charset encoding.
//!
//! Only Unicode is implemented today; the `charset` parameter reserves the
//! seam for the legacy Vietnamese charsets (TCVN3, VNI-Win, VIQR, …).

/// Encode composed Unicode text into the named output charset.
pub fn encode(_charset: &str, input: &str) -> String {
    input.to_string()
}

/// Names of the supported output charsets.
pub fn charset_names() -> Vec<&'static str> {
    vec!["Unicode"]
}
