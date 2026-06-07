//! Vietnamese word dictionary for spell-check (`SPELL_CHECK_WITH_DICTS`).
//!
//! Loaded lazily on first lookup from the installed data file (so the cost is
//! only paid when dictionary checking is actually enabled). A missing file
//! yields an empty set, in which case dictionary checks simply never match.

use std::collections::HashSet;
use std::sync::LazyLock;

const DICT_PATH: &str = "/usr/share/ibus-bambusa/vietnamese.cm.dict";

static DICTIONARY: LazyLock<HashSet<String>> = LazyLock::new(|| {
    std::fs::read_to_string(DICT_PATH)
        .map(|s| {
            s.lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
});

/// Whether `word` (lowercase composed Vietnamese) is a known dictionary word.
pub fn contains(word: &str) -> bool {
    DICTIONARY.contains(word)
}
