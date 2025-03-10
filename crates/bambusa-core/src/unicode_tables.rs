//! Vietnamese character tables and the tone/mark arithmetic that operates on
//! individual characters.

use std::collections::HashMap;
use std::sync::LazyLock;

use crate::rules::{Mark, Tone};

/// Every Vietnamese vowel, grouped in runs of six: base vowel followed by its
/// grave, acute, hook, tilde and dot variants. Tone arithmetic relies on this
/// layout (`position % 6` is the tone, integer division selects the vowel).
pub(crate) static VOWELS: LazyLock<Vec<char>> = LazyLock::new(|| {
    "aàáảãạăằắẳẵặâầấẩẫậeèéẻẽẹêềếểễệiìíỉĩịoòóỏõọôồốổỗộơờớởỡợuùúủũụưừứửữựyỳýỷỹỵ"
        .chars()
        .collect()
});

/// Symbols that break a word boundary.
pub(crate) const PUNCTUATION_MARKS: &[char] = &[
    ',', ';', ':', '.', '"', '\'', '!', '?', ' ', '<', '>', '=', '+', '-', '*', '/', '\\', '_',
    '~', '`', '@', '#', '$', '%', '^', '&', '(', ')', '{', '}', '[', ']', '|',
];

/// For each markable character, the five-character family indexed by mark
/// (none, hat, breve, horn, dash); `_` marks an absent slot.
static MARKS_MAPS: LazyLock<HashMap<char, &'static str>> = LazyLock::new(|| {
    HashMap::from([
        ('a', "aâă__"),
        ('â', "aâă__"),
        ('ă', "aâă__"),
        ('e', "eê___"),
        ('ê', "eê___"),
        ('o', "oô_ơ_"),
        ('ô', "oô_ơ_"),
        ('ơ', "oô_ơ_"),
        ('u', "u__ư_"),
        ('ư', "u__ư_"),
        ('d', "d___đ"),
        ('đ', "d___đ"),
    ])
});

pub(crate) fn is_space(key: char) -> bool {
    key == ' '
}

pub(crate) fn is_punctuation_mark(key: char) -> bool {
    PUNCTUATION_MARKS.contains(&key)
}

pub(crate) fn is_word_break_symbol(key: char) -> bool {
    is_punctuation_mark(key) || key.is_ascii_digit()
}

pub(crate) fn is_vowel(chr: char) -> bool {
    VOWELS.contains(&chr)
}

pub(crate) fn find_vowel_position(chr: char) -> Option<usize> {
    VOWELS.iter().position(|&v| v == chr)
}

fn mark_family_str(chr: char) -> Option<&'static str> {
    MARKS_MAPS.get(&chr).copied()
}

pub(crate) fn get_mark_family(chr: char) -> Vec<char> {
    match mark_family_str(chr) {
        Some(s) => s.chars().filter(|&c| c != '_').collect(),
        None => Vec::new(),
    }
}

pub(crate) fn find_mark_position(chr: char) -> Option<usize> {
    mark_family_str(chr).and_then(|s| s.chars().position(|v| v == chr))
}

pub(crate) fn find_mark_from_char(chr: char) -> Option<Mark> {
    find_mark_position(chr).map(|pos| Mark::try_from(pos as u8).expect("mark position is 0..=4"))
}

pub(crate) fn add_mark_to_toneless_char(chr: char, mark: u8) -> char {
    if let Some(s) = mark_family_str(chr) {
        if let Some(m) = s.chars().nth(mark as usize) {
            if m != '_' {
                return m;
            }
        }
    }
    chr
}

pub(crate) fn add_mark_to_char(chr: char, mark: u8) -> char {
    let tone = find_tone_from_char(chr);
    let toneless = add_tone_to_char(chr, 0);
    let marked = add_mark_to_toneless_char(toneless, mark);
    add_tone_to_char(marked, tone as u8)
}

pub(crate) fn is_alpha(c: char) -> bool {
    c.is_ascii_alphabetic()
}

pub(crate) fn find_tone_from_char(chr: char) -> Tone {
    match find_vowel_position(chr) {
        None => Tone::None,
        Some(pos) => Tone::try_from((pos % 6) as u8).expect("tone index is 0..=5"),
    }
}

pub(crate) fn add_tone_to_char(chr: char, tone: u8) -> char {
    match find_vowel_position(chr) {
        Some(pos) => {
            let current_tone = (pos % 6) as isize;
            let offset = tone as isize - current_tone;
            VOWELS[(pos as isize + offset) as usize]
        }
        None => chr,
    }
}

pub(crate) fn can_process_key(lower_key: char, effect_keys: &[char]) -> bool {
    if is_alpha(lower_key) || effect_keys.contains(&lower_key) {
        return true;
    }
    if is_word_break_symbol(lower_key) {
        return false;
    }
    is_vietnamese_rune(lower_key)
}

pub(crate) fn is_vietnamese_rune(lower_key: char) -> bool {
    if find_tone_from_char(lower_key) != Tone::None {
        return true;
    }
    lower_key != add_mark_to_toneless_char(lower_key, 0)
}

/// Whether `word` contains any Vietnamese-specific character.
pub fn has_any_vietnamese_rune(word: &str) -> bool {
    word.chars().any(|chr| is_vietnamese_rune(to_lower(chr)))
}

/// Whether `word` contains any Vietnamese vowel.
pub fn has_any_vietnamese_vowel(word: &str) -> bool {
    word.chars().any(|chr| is_vowel(to_lower(chr)))
}

/// Lowercase a single character following Unicode rules, taking the first
/// resulting character (Vietnamese letters are single-scalar lowercase).
pub(crate) fn to_lower(chr: char) -> char {
    chr.to_lowercase().next().unwrap_or(chr)
}

/// Uppercase a single character, taking the first resulting character.
pub(crate) fn to_upper(chr: char) -> char {
    chr.to_uppercase().next().unwrap_or(chr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tone_arithmetic_stays_in_vowel_group() {
        // á (acute a) -> strip tone -> a -> add grave -> à
        assert_eq!(add_tone_to_char('á', 0), 'a');
        assert_eq!(add_tone_to_char('a', Tone::Grave as u8), 'à');
        assert_eq!(find_tone_from_char('ả'), Tone::Hook);
        assert_eq!(find_tone_from_char('z'), Tone::None);
    }

    #[test]
    fn mark_arithmetic_preserves_tone() {
        // add hat to 'a' -> 'â'; adding to a toned vowel keeps the tone.
        assert_eq!(add_mark_to_char('a', Mark::Hat as u8), 'â');
        assert_eq!(add_mark_to_char('á', Mark::Hat as u8), 'ấ');
        assert_eq!(add_mark_to_char('o', Mark::Horn as u8), 'ơ');
        assert_eq!(add_mark_to_char('d', Mark::Dash as u8), 'đ');
    }

    #[test]
    fn mark_lookup() {
        assert_eq!(find_mark_from_char('â'), Some(Mark::Hat));
        assert_eq!(find_mark_from_char('ơ'), Some(Mark::Horn));
        assert_eq!(find_mark_from_char('đ'), Some(Mark::Dash));
        assert_eq!(find_mark_from_char('a'), Some(Mark::None));
        assert_eq!(find_mark_from_char('z'), None);
        assert_eq!(get_mark_family('o'), vec!['o', 'ô', 'ơ']);
    }

    #[test]
    fn vietnamese_rune_detection() {
        assert!(is_vietnamese_rune('â'));
        assert!(is_vietnamese_rune('đ'));
        assert!(is_vietnamese_rune('ạ'));
        assert!(!is_vietnamese_rune('a'));
        assert!(!is_vietnamese_rune('d'));
        assert!(has_any_vietnamese_rune("việt"));
        assert!(!has_any_vietnamese_rune("viet"));
    }

    #[test]
    fn word_break_and_processable() {
        assert!(is_word_break_symbol('5'));
        assert!(is_word_break_symbol('.'));
        assert!(!is_word_break_symbol('a'));
        assert!(can_process_key('a', &[]));
        assert!(!can_process_key('5', &[]));
    }

    #[test]
    fn every_vowel_is_a_vowel() {
        assert!(is_vowel('a'));
        assert!(is_vowel('á'));
        assert!(!is_vowel('b'));
        for v in VOWELS.iter() {
            assert!(is_vowel(*v), "{v} should be a vowel");
        }
    }

    #[test]
    fn tone_from_char_each_tone() {
        assert_eq!(find_tone_from_char('e'), Tone::None);
        assert_eq!(find_tone_from_char('è'), Tone::Grave);
        assert_eq!(find_tone_from_char('é'), Tone::Acute);
        assert_eq!(find_tone_from_char('ẽ'), Tone::Tilde);
        assert_eq!(find_tone_from_char('ẻ'), Tone::Hook);
        assert_eq!(find_tone_from_char('ạ'), Tone::Dot);
    }

    #[test]
    fn tone_and_mark_identities() {
        assert_eq!(add_tone_to_char('a', Tone::Dot as u8), 'ạ');
        assert_eq!(add_tone_to_char('y', 0), 'y');
        assert_eq!(add_mark_to_char('y', 0), 'y');
        assert_eq!(add_mark_to_char('ạ', Mark::Breve as u8), 'ặ');
    }
}
