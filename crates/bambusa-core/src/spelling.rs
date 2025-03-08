//! Vietnamese phonotactic validation: whether a (first-consonant, vowel,
//! last-consonant) decomposition forms a spellable syllable.

// Consumed by the transformation engine landing next; drop once that lands.
#![allow(dead_code)]

use crate::unicode_tables::add_mark_to_toneless_char;

/// Initial consonant clusters, grouped into compatibility classes.
const FIRST_CONSONANT_SEQS: &[&str] = &[
    "b d đ g gh m n nh p ph r s t tr v z",
    "c h k kh qu th",
    "ch gi l ng ngh x",
    "đ l",
    "h",
];

/// Vowel nuclei, grouped into compatibility classes.
const VOWEL_SEQS: &[&str] = &[
    "ê i ua uê uy y",
    "a iê oa uyê yê",
    "â ă e o oo ô ơ oe u ư uâ uô ươ",
    "oă",
    "uơ",
    "ai ao au âu ay ây eo êu ia iêu iu oai oao oay oeo oi ôi ơi ưa uây ui ưi uôi ươi ươu ưu uya uyu yêu",
    "ă",
    "i",
];

/// Final consonant clusters, grouped into compatibility classes.
const LAST_CONSONANT_SEQS: &[&str] = &["ch nh", "c ng", "m n p t", "k", "c"];

/// For each first-consonant class, the vowel classes it may precede.
const CV_MATRIX: &[&[usize]] = &[
    &[0, 1, 2, 5],
    &[0, 1, 2, 3, 4, 5],
    &[0, 1, 2, 3, 5],
    &[6],
    &[7],
];

/// For each vowel class, the final-consonant classes it may precede.
const VC_MATRIX: &[&[usize]] = &[&[0, 2], &[0, 1, 2], &[1, 2], &[1, 2], &[], &[], &[3], &[4]];

/// Return the indices of every class in `seq` that `input` matches.
///
/// When `input_is_full` the lengths must match exactly; otherwise `input` may
/// be a prefix. When `input_is_complete` characters must match exactly,
/// otherwise a toneless-marked candidate also matches (e.g. `a` matches `â`).
fn lookup(seq: &[&str], input: &str, input_is_full: bool, input_is_complete: bool) -> Vec<usize> {
    let input_chars: Vec<char> = input.chars().collect();
    let input_len = input_chars.len();
    let mut ret = Vec::new();
    for (index, row) in seq.iter().enumerate() {
        for token in row.split(' ') {
            let canvas: Vec<char> = token.chars().collect();
            if canvas.len() < input_len || (input_is_full && canvas.len() > input_len) {
                continue;
            }
            let is_match = input_chars.iter().enumerate().all(|(k, &ic)| {
                let cv = canvas[k];
                ic == cv || (!input_is_complete && add_mark_to_toneless_char(cv, 0) == ic)
            });
            if is_match {
                ret.push(index);
                break;
            }
        }
    }
    ret
}

/// Whether `fc`/`vo`/`lc` form a valid (possibly partial) Vietnamese syllable.
pub(crate) fn is_valid_cvc(fc: &str, vo: &str, lc: &str, input_is_full_complete: bool) -> bool {
    let fc_indexes = if !fc.is_empty() {
        let r = lookup(
            FIRST_CONSONANT_SEQS,
            fc,
            input_is_full_complete || !vo.is_empty(),
            true,
        );
        if r.is_empty() {
            return false;
        }
        Some(r)
    } else {
        None
    };
    let vo_indexes = if !vo.is_empty() {
        let r = lookup(
            VOWEL_SEQS,
            vo,
            input_is_full_complete || !lc.is_empty(),
            input_is_full_complete,
        );
        if r.is_empty() {
            return false;
        }
        Some(r)
    } else {
        None
    };
    let lc_indexes = if !lc.is_empty() {
        let r = lookup(LAST_CONSONANT_SEQS, lc, input_is_full_complete, true);
        if r.is_empty() {
            return false;
        }
        Some(r)
    } else {
        None
    };

    let Some(vo_idx) = vo_indexes.as_ref() else {
        // First consonant only.
        return fc_indexes.is_some();
    };

    if let Some(fc_idx) = fc_indexes.as_ref() {
        let cv_ok = is_valid_cv(fc_idx, vo_idx);
        if !cv_ok || lc_indexes.is_none() {
            return cv_ok;
        }
    }
    match lc_indexes.as_ref() {
        Some(lc_idx) => is_valid_vc(vo_idx, lc_idx),
        None => true,
    }
}

fn is_valid_cv(fc_indexes: &[usize], vo_indexes: &[usize]) -> bool {
    fc_indexes
        .iter()
        .flat_map(|&fc| CV_MATRIX[fc])
        .any(|c| vo_indexes.contains(c))
}

fn is_valid_vc(vo_indexes: &[usize], lc_indexes: &[usize]) -> bool {
    vo_indexes
        .iter()
        .flat_map(|&vo| VC_MATRIX[vo])
        .any(|c| lc_indexes.contains(c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_syllables_are_valid() {
        assert!(is_valid_cvc("v", "iê", "t", true)); // việt
        assert!(is_valid_cvc("ng", "uyê", "n", true)); // nguyên
        assert!(is_valid_cvc("", "ê", "ch", true)); // êch
    }

    #[test]
    fn incompatible_vowel_consonant_is_invalid() {
        // "ê" cannot take a bare "c" final consonant.
        assert!(!is_valid_cvc("", "ê", "c", true));
    }

    #[test]
    fn first_consonant_only_is_valid_prefix() {
        assert!(is_valid_cvc("ng", "", "", false));
        assert!(!is_valid_cvc("qz", "", "", false));
    }
}
