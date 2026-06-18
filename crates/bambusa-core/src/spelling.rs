//! Vietnamese phonotactic validation: whether a (first-consonant, vowel,
//! last-consonant) decomposition forms a spellable syllable.

use crate::flatten::flatten_indices_into;
use crate::mode::Mode;
use crate::transform::Transformation;
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

/// For each first-consonant class, a bitmask of the vowel classes it may
/// precede (bit `i` set means vowel class `i` is allowed). Derived from the
/// index lists `[0,1,2,5]`, `[0,1,2,3,4,5]`, `[0,1,2,3,5]`, `[6]`, `[7]`.
const CV_MASK: &[u16] = &[
    0b0010_0111,
    0b0011_1111,
    0b0010_1111,
    0b0100_0000,
    0b1000_0000,
];

/// For each vowel class, a bitmask of the final-consonant classes it may
/// precede. Derived from `[0,2]`, `[0,1,2]`, `[1,2]`, `[1,2]`, `[]`, `[]`,
/// `[3]`, `[4]`.
const VC_MASK: &[u16] = &[
    0b0_0101, 0b0_0111, 0b0_0110, 0b0_0110, 0, 0, 0b0_1000, 0b1_0000,
];

/// Return a bitmask of every class in `seq` that `input` matches (bit `i` set
/// means class `i` matched); `0` means no class matched.
///
/// When `input_is_full` the lengths must match exactly; otherwise `input` may
/// be a prefix. When `input_is_complete` characters must match exactly,
/// otherwise a toneless-marked candidate also matches (e.g. `a` matches `â`).
///
/// Allocation-free: it compares character iterators directly rather than
/// collecting them, which matters because it runs on every spelling check.
fn lookup(seq: &[&str], input: &str, input_is_full: bool, input_is_complete: bool) -> u16 {
    let input_len = input.chars().count();
    let mut ret = 0u16;
    for (index, row) in seq.iter().enumerate() {
        for token in row.split(' ') {
            let token_len = token.chars().count();
            if token_len < input_len || (input_is_full && token_len > input_len) {
                continue;
            }
            // `zip` stops after `input_len` pairs since `token_len >= input_len`,
            // so this checks `input` against the token's leading characters.
            let is_match = input.chars().zip(token.chars()).all(|(ic, cv)| {
                ic == cv || (!input_is_complete && add_mark_to_toneless_char(cv, 0) == ic)
            });
            if is_match {
                ret |= 1 << index;
                break;
            }
        }
    }
    ret
}

/// Whether the first-consonant/vowel/last-consonant groups — given as `fc`/`vo`/
/// `lc` index slices into `comp` — form a valid (possibly partial) Vietnamese
/// syllable.
///
/// Each group is flattened through a single reused `buf` because its text is
/// only needed to compute its class mask, so the three flattens never overlap —
/// one allocation instead of three on this hot path, and the groups never need
/// to be materialised.
pub(crate) fn is_valid_cvc(
    comp: &[Transformation],
    fc: &[usize],
    vo: &[usize],
    lc: &[usize],
    input_is_full_complete: bool,
    buf: &mut String,
) -> bool {
    let m = Mode::VIETNAMESE | Mode::LOWERCASE | Mode::TONELESS;
    let fc_mask = if !fc.is_empty() {
        flatten_indices_into(comp, fc, m, buf);
        let mask = lookup(
            FIRST_CONSONANT_SEQS,
            buf,
            input_is_full_complete || !vo.is_empty(),
            true,
        );
        if mask == 0 {
            return false;
        }
        Some(mask)
    } else {
        None
    };
    let vo_mask = if !vo.is_empty() {
        flatten_indices_into(comp, vo, m, buf);
        let mask = lookup(
            VOWEL_SEQS,
            buf,
            input_is_full_complete || !lc.is_empty(),
            input_is_full_complete,
        );
        if mask == 0 {
            return false;
        }
        Some(mask)
    } else {
        None
    };
    let lc_mask = if !lc.is_empty() {
        flatten_indices_into(comp, lc, m, buf);
        let mask = lookup(LAST_CONSONANT_SEQS, buf, input_is_full_complete, true);
        if mask == 0 {
            return false;
        }
        Some(mask)
    } else {
        None
    };

    let Some(vo_m) = vo_mask else {
        // First consonant only.
        return fc_mask.is_some();
    };

    if let Some(fc_m) = fc_mask {
        let cv_ok = is_valid_cv(fc_m, vo_m);
        if !cv_ok || lc_mask.is_none() {
            return cv_ok;
        }
    }
    match lc_mask {
        Some(lc_m) => is_valid_vc(vo_m, lc_m),
        None => true,
    }
}

/// Whether any vowel class allowed after the matched first-consonant classes
/// (`fc_mask`) intersects the matched vowel classes (`vo_mask`).
fn is_valid_cv(fc_mask: u16, vo_mask: u16) -> bool {
    let mut allowed = 0u16;
    for (i, &m) in CV_MASK.iter().enumerate() {
        if fc_mask & (1 << i) != 0 {
            allowed |= m;
        }
    }
    allowed & vo_mask != 0
}

/// Whether any final-consonant class allowed after the matched vowel classes
/// (`vo_mask`) intersects the matched final-consonant classes (`lc_mask`).
fn is_valid_vc(vo_mask: u16, lc_mask: u16) -> bool {
    let mut allowed = 0u16;
    for (i, &m) in VC_MASK.iter().enumerate() {
        if vo_mask & (1 << i) != 0 {
            allowed |= m;
        }
    }
    allowed & lc_mask != 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::{EffectType, Rule};
    use crate::transform::TransId;

    /// Append plain appending transformations for each char of `s` to `comp`,
    /// returning the indices of those appenders.
    fn push_group(comp: &mut Vec<Transformation>, s: &str) -> Vec<usize> {
        let start = comp.len();
        for c in s.chars() {
            let id = comp.len() as TransId;
            comp.push(Transformation {
                id,
                rule: Rule {
                    key: c,
                    effect_on: c,
                    result: c,
                    effect_type: EffectType::Appending,
                    ..Default::default()
                },
                target: None,
                is_upper_case: false,
            });
        }
        (start..comp.len()).collect()
    }

    fn valid(fc: &str, vo: &str, lc: &str, full: bool) -> bool {
        let mut comp = Vec::new();
        let fci = push_group(&mut comp, fc);
        let voi = push_group(&mut comp, vo);
        let lci = push_group(&mut comp, lc);
        let mut buf = String::new();
        is_valid_cvc(&comp, &fci, &voi, &lci, full, &mut buf)
    }

    #[test]
    fn complete_syllables_are_valid() {
        assert!(valid("v", "iê", "t", true)); // việt
        assert!(valid("ng", "uyê", "n", true)); // nguyên
        assert!(valid("", "ê", "ch", true)); // êch
    }

    #[test]
    fn incompatible_vowel_consonant_is_invalid() {
        // "ê" cannot take a bare "c" final consonant.
        assert!(!valid("", "ê", "c", true));
    }

    #[test]
    fn first_consonant_only_is_valid_prefix() {
        assert!(valid("ng", "", "", false));
        assert!(!valid("qz", "", "", false));
    }
}
