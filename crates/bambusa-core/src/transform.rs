//! The composition: an ordered list of [`Transformation`]s and the machinery
//! that builds and edits it from keystrokes.

// Several functions are consumed only by the engine glue landing next; drop
// once that lands.
#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;

use crate::flatten::{canvas, flatten};
use crate::mode::{EngineFlags, Mode};
use crate::rules::{EffectType, Mark, Rule, Tone};
use crate::spelling::is_valid_cvc;
use crate::unicode_tables::{find_tone_from_char, is_alpha, is_space, is_vowel, to_lower};

/// Stable, position-independent identity for a [`Transformation`].
pub type TransId = u32;

/// Id reserved for throwaway transformations used only for trial flattening.
const TEMP_ID: TransId = TransId::MAX;

/// One keystroke's effect on the composition: an appended character, or a
/// tone/mark applied to an earlier transformation identified by `target`.
#[derive(Clone, Debug)]
pub struct Transformation {
    pub id: TransId,
    pub rule: Rule,
    pub target: Option<TransId>,
    pub is_upper_case: bool,
}

/// Hands out unique [`TransId`]s for a single engine's lifetime.
#[derive(Debug, Default)]
pub struct IdGen {
    next: TransId,
}

impl IdGen {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn next_id(&mut self) -> TransId {
        let id = self.next;
        self.next += 1;
        id
    }
}

static REG_UOH_TAIL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(uơ|ưo)\p{L}+").unwrap());
static REG_UH_O: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(ưo|ươ)").unwrap());

/// Whether `s` ends in the `uơ`/`ưo` cluster that triggers the uow shortcut.
pub(crate) fn matches_uoh_tail(s: &str) -> bool {
    REG_UOH_TAIL.is_match(s)
}

fn by_id(comp: &[Transformation], id: TransId) -> Option<&Transformation> {
    comp.iter().find(|t| t.id == id)
}

fn temp_effect(target: Option<TransId>, rule: Rule) -> Transformation {
    Transformation {
        id: TEMP_ID,
        rule,
        target,
        is_upper_case: false,
    }
}

/// Id of the last appending transformation, if any.
pub(crate) fn find_last_appending_trans(comp: &[Transformation]) -> Option<TransId> {
    comp.iter()
        .rev()
        .find(|t| t.rule.effect_type == EffectType::Appending)
        .map(|t| t.id)
}

pub(crate) fn new_appending_trans(
    ids: &mut IdGen,
    key: char,
    is_upper_case: bool,
) -> Transformation {
    Transformation {
        id: ids.next_id(),
        rule: Rule {
            key,
            effect_on: key,
            effect_type: EffectType::Appending,
            result: key,
            ..Default::default()
        },
        target: None,
        is_upper_case,
    }
}

fn generate_appending_trans(
    ids: &mut IdGen,
    rules: &[Rule],
    lower_key: char,
    is_upper_case: bool,
) -> Transformation {
    for rule in rules {
        if rule.key == lower_key && rule.effect_type == EffectType::Appending {
            let mut rule = rule.clone();
            let upper = is_upper_case || rule.effect_on.is_uppercase();
            rule.effect_on = to_lower(rule.effect_on);
            rule.result = rule.effect_on;
            return Transformation {
                id: ids.next_id(),
                rule,
                target: None,
                is_upper_case: upper,
            };
        }
    }
    new_appending_trans(ids, lower_key, is_upper_case)
}

fn filter_appending_composition(comp: &[Transformation]) -> Vec<Transformation> {
    comp.iter()
        .filter(|t| t.rule.effect_type == EffectType::Appending)
        .cloned()
        .collect()
}

fn find_root_target(comp: &[Transformation], id: TransId) -> TransId {
    match by_id(comp, id).and_then(|t| t.target) {
        Some(parent) => find_root_target(comp, parent),
        None => id,
    }
}

/// Whether the composition forms a spellable (possibly partial) syllable.
pub(crate) fn is_valid(comp: &[Transformation], input_is_full_complete: bool) -> bool {
    if comp.len() <= 1 {
        return true;
    }
    // The most recent tone must be compatible with the final consonant.
    for trans in comp.iter().rev() {
        if trans.rule.effect_type == EffectType::ToneTransformation {
            let last_tone = Tone::try_from(trans.rule.effect).unwrap_or_default();
            if !has_valid_tone(comp, last_tone) {
                return false;
            }
            break;
        }
    }
    let (fc, vo, lc) = extract_cvc_trans(comp);
    let m = Mode::VIETNAMESE | Mode::LOWERCASE | Mode::TONELESS;
    is_valid_cvc(
        &flatten(&fc, m),
        &flatten(&vo, m),
        &flatten(&lc, m),
        input_is_full_complete,
    )
}

fn has_valid_tone(comp: &[Transformation], tone: Tone) -> bool {
    if tone == Tone::None || tone == Tone::Acute || tone == Tone::Dot {
        return true;
    }
    let (_, _, lc) = extract_cvc_trans(comp);
    if lc.is_empty() {
        return true;
    }
    let last_consonants = flatten(&lc, Mode::ENGLISH | Mode::LOWERCASE);
    // Stop consonants may only carry the ACUTE or DOT tones.
    !matches!(last_consonants.as_str(), "c" | "k" | "p" | "t" | "ch")
}

fn get_rightmost_vowels(comp: &[Transformation]) -> Vec<Transformation> {
    extract_cvc_trans(comp).1
}

fn get_last_tone_transformation(comp: &[Transformation]) -> Option<usize> {
    (0..comp.len()).rev().find(|&i| {
        comp[i].rule.effect_type == EffectType::ToneTransformation && comp[i].target.is_some()
    })
}

fn is_free(comp: &[Transformation], target: Option<TransId>, effect_type: EffectType) -> bool {
    !comp
        .iter()
        .any(|t| t.target == target && t.rule.effect_type == effect_type)
}

fn find_tone_target(comp: &[Transformation], std_style: bool) -> Option<TransId> {
    if comp.is_empty() {
        return None;
    }
    let (_, vo, lc) = extract_cvc_trans(comp);
    let vowels = filter_appending_composition(&vo);
    let mark_mode = Mode::ENGLISH | Mode::LOWERCASE | Mode::TONELESS | Mode::MARKLESS;
    match vowels.len() {
        1 => Some(vowels[0].id),
        2 if std_style => {
            let mut target = None;
            for trans in &vo {
                if trans.rule.result == 'ơ' || trans.rule.result == 'ê' {
                    target = Some(trans.target.unwrap_or(trans.id));
                }
            }
            target.or(Some(if !lc.is_empty() {
                vowels[1].id
            } else {
                vowels[0].id
            }))
        }
        2 => {
            if !lc.is_empty() {
                Some(vowels[1].id)
            } else {
                let s = flatten(&vowels, mark_mode);
                Some(match s.as_str() {
                    "oa" | "oe" | "uy" | "ue" | "uo" => vowels[1].id,
                    _ => vowels[0].id,
                })
            }
        }
        3 => {
            let s = flatten(&vowels, mark_mode);
            Some(if s == "uye" {
                vowels[2].id
            } else {
                vowels[1].id
            })
        }
        _ => None,
    }
}

fn extract_atomic_trans(comp: &[Transformation], last_is_vowel: bool) -> usize {
    let mut i = comp.len();
    while i > 0 {
        let tmp = &comp[i - 1];
        if tmp.target.is_none() && is_vowel(tmp.rule.result) != last_is_vowel {
            break;
        }
        i -= 1;
    }
    i
}

fn extract_cvc_appending_trans(
    appending: &[Transformation],
) -> (
    Vec<Transformation>,
    Vec<Transformation>,
    Vec<Transformation>,
) {
    let head_split = extract_atomic_trans(appending, false);
    let mut last_consonant = appending[head_split..].to_vec();
    let head = &appending[..head_split];
    let fc_split = extract_atomic_trans(head, true);
    let mut first_consonant = head[..fc_split].to_vec();
    let mut vowel = head[fc_split..].to_vec();

    if !last_consonant.is_empty() && vowel.is_empty() && first_consonant.is_empty() {
        first_consonant = last_consonant;
        vowel = Vec::new();
        last_consonant = Vec::new();
    }

    // 'gi' and 'qu' are treated as qualified initial consonants:
    //   ['g', 'ia', ''] -> ['gi', 'a', ''], ['q', 'ua', ''] -> ['qu', 'a', '']
    // but not ['g', 'ie', 'ng'].
    if first_consonant.len() == 1
        && !vowel.is_empty()
        && ((first_consonant[0].rule.result == 'g'
            && vowel[0].rule.result == 'i'
            && vowel.len() > 1
            && (vowel[1].rule.result != 'e' || last_consonant.is_empty()))
            || (first_consonant[0].rule.result == 'q' && vowel[0].rule.result == 'u'))
    {
        first_consonant.push(vowel[0].clone());
        vowel = vowel[1..].to_vec();
    }
    (first_consonant, vowel, last_consonant)
}

fn extract_cvc_trans(
    comp: &[Transformation],
) -> (
    Vec<Transformation>,
    Vec<Transformation>,
    Vec<Transformation>,
) {
    let mut trans_map: HashMap<TransId, Vec<Transformation>> = HashMap::new();
    let mut appending = Vec::new();
    for trans in comp {
        match trans.target {
            None => appending.push(trans.clone()),
            Some(target) => trans_map.entry(target).or_default().push(trans.clone()),
        }
    }
    let (mut fc, mut vo, mut lc) = extract_cvc_appending_trans(&appending);
    attach_effects(&mut fc, &trans_map);
    attach_effects(&mut vo, &trans_map);
    attach_effects(&mut lc, &trans_map);
    (fc, vo, lc)
}

fn attach_effects(
    group: &mut Vec<Transformation>,
    trans_map: &HashMap<TransId, Vec<Transformation>>,
) {
    let mut extra = Vec::new();
    for t in group.iter() {
        if let Some(effects) = trans_map.get(&t.id) {
            extra.extend(effects.iter().cloned());
        }
    }
    group.extend(extra);
}

/// Split index such that `comp[..n]` is everything before the last word and
/// `comp[n..]` is the last word.
pub(crate) fn extract_last_word(comp: &[Transformation], effect_keys: &[char]) -> usize {
    for i in (0..comp.len()).rev() {
        let c = canvas(
            &comp[i..],
            Mode::VIETNAMESE | Mode::LOWERCASE | Mode::TONELESS | Mode::MARKLESS,
        );
        if c.is_empty() {
            continue;
        }
        let c0 = c[0];
        if !is_alpha(c0) && !effect_keys.contains(&c0) {
            if i == comp.len() - 1 {
                return comp.len();
            }
            return i + 1;
        }
    }
    0
}

/// Like [`extract_last_word`] but breaks only on a literal space.
pub(crate) fn extract_last_word_with_punctuation_marks(comp: &[Transformation]) -> usize {
    for i in (0..comp.len()).rev() {
        let c = canvas(&comp[i..], Mode::ENGLISH);
        if c.is_empty() {
            continue;
        }
        if is_space(c[0]) {
            if i == comp.len() - 1 {
                return comp.len();
            }
            return i + 1;
        }
    }
    0
}

/// Split index isolating the last syllable: `comp[n..]` is the last syllable.
pub(crate) fn extract_last_syllable(comp: &[Transformation]) -> usize {
    let word_split = extract_last_word(comp, &[]);
    let last = &comp[word_split..];
    let mut anchor = 0;
    for i in 0..last.len() {
        if !is_valid(&last[anchor..=i], false) {
            anchor = i;
        }
    }
    word_split + anchor
}

fn find_mark_target(comp: &[Transformation], rules: &[Rule]) -> (Option<TransId>, Rule) {
    let str = flatten(comp, Mode::VIETNAMESE);
    for i in (0..comp.len()).rev() {
        let trans = &comp[i];
        for rule in rules {
            if rule.effect_type != EffectType::MarkTransformation {
                continue;
            }
            if trans.rule.result == rule.effect_on && rule.effect > 0 {
                let target = find_root_target(comp, trans.id);
                let mut probe = comp.to_vec();
                probe.push(temp_effect(Some(target), rule.clone()));
                if str == flatten(&probe, Mode::VIETNAMESE) {
                    continue;
                }
                if is_valid(&probe, false) {
                    return (Some(target), rule.clone());
                }
            }
        }
    }
    (None, Rule::default())
}

fn find_target(
    comp: &[Transformation],
    rules: &[Rule],
    flags: EngineFlags,
) -> (Option<TransId>, Rule) {
    let str = flatten(comp, Mode::VIETNAMESE);
    for rule in rules {
        if rule.effect_type != EffectType::ToneTransformation {
            continue;
        }
        let mut target = tone_target_for(comp, rule, flags);
        let mut probe = comp.to_vec();
        probe.push(temp_effect(target, rule.clone()));
        if str == flatten(&probe, Mode::VIETNAMESE) {
            continue;
        }
        if Tone::try_from(rule.effect).unwrap_or_default() == Tone::None
            && is_free(comp, target, EffectType::ToneTransformation)
            && target.is_some_and(|tid| {
                by_id(comp, tid).is_some_and(|t| find_tone_from_char(t.rule.result) == Tone::None)
            })
        {
            target = None;
        }
        return (target, rule.clone());
    }
    find_mark_target(comp, rules)
}

/// Resolve the tone target for `rule` under the active flags.
fn tone_target_for(comp: &[Transformation], rule: &Rule, flags: EngineFlags) -> Option<TransId> {
    if flags.contains(EngineFlags::FREE_TONE_MARKING) {
        if has_valid_tone(comp, Tone::try_from(rule.effect).unwrap_or_default()) {
            return find_tone_target(comp, flags.contains(EngineFlags::STD_TONE_STYLE));
        }
        None
    } else if let Some(la) = find_last_appending_trans(comp) {
        if by_id(comp, la).is_some_and(|t| is_vowel(t.rule.effect_on)) {
            return Some(la);
        }
        None
    } else {
        None
    }
}

fn generate_undo_transformations(
    ids: &mut IdGen,
    comp: &[Transformation],
    rules: &[Rule],
    flags: EngineFlags,
) -> Vec<Transformation> {
    let mut out = Vec::new();
    let str = flatten(comp, Mode::VIETNAMESE | Mode::TONELESS | Mode::LOWERCASE);
    for rule in rules {
        match rule.effect_type {
            EffectType::ToneTransformation => {
                let target = tone_target_for(comp, rule, flags);
                if target.is_none() {
                    continue;
                }
                out.push(Transformation {
                    id: ids.next_id(),
                    target,
                    rule: Rule {
                        effect_type: EffectType::ToneTransformation,
                        effect: 0,
                        key: '\0',
                        ..Default::default()
                    },
                    is_upper_case: false,
                });
            }
            EffectType::MarkTransformation => {
                for i in (0..comp.len()).rev() {
                    if comp[i].rule.result == rule.effect_on {
                        let target = find_root_target(comp, comp[i].id);
                        let probe_rule = Rule {
                            key: '\0',
                            effect_type: EffectType::MarkTransformation,
                            effect: 0,
                            ..Default::default()
                        };
                        let mut probe = comp.to_vec();
                        probe.push(temp_effect(Some(target), probe_rule.clone()));
                        if str
                            == flatten(&probe, Mode::VIETNAMESE | Mode::TONELESS | Mode::LOWERCASE)
                        {
                            continue;
                        }
                        out.push(Transformation {
                            id: ids.next_id(),
                            target: Some(target),
                            rule: probe_rule,
                            is_upper_case: false,
                        });
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// Generate the transformations a keypress adds, or an empty vec if the key
/// should fall back to a plain append.
pub(crate) fn generate_transformations(
    ids: &mut IdGen,
    comp: &[Transformation],
    rules: &[Rule],
    flags: EngineFlags,
    lower_key: char,
    is_upper_case: bool,
) -> Vec<Transformation> {
    let mut out = Vec::new();

    // Double-typing an effect key undoes it (raw): w + w -> w.
    if let Some(last) = comp.last() {
        if last.rule.effect_type == EffectType::Appending
            && last.rule.key == lower_key
            && last.rule.key != last.rule.result
        {
            out.push(Transformation {
                id: ids.next_id(),
                target: Some(last.id),
                rule: Rule {
                    effect_type: EffectType::MarkTransformation,
                    effect: Mark::Raw as u8,
                    key: '\0',
                    ..Default::default()
                },
                is_upper_case: false,
            });
            return out;
        }
    }

    let (target, applicable_rule) = find_target(comp, rules, flags);
    if let Some(target_id) = target {
        out.push(Transformation {
            id: ids.next_id(),
            rule: applicable_rule.clone(),
            target: Some(target_id),
            is_upper_case,
        });
        if applicable_rule.effect_type != EffectType::MarkTransformation {
            return out;
        }
        let mut new_comp = comp.to_vec();
        new_comp.extend(out.iter().cloned());
        if is_valid(&new_comp, true) {
            return out;
        }
        // uow shortcut: a virtual Mark::Horn targeting 'u' or 'o'.
        let (t2, mut virtual_rule) = find_target(&new_comp, rules, flags);
        if let Some(t2_id) = t2 {
            virtual_rule.key = '\0';
            out.push(Transformation {
                id: ids.next_id(),
                rule: virtual_rule,
                target: Some(t2_id),
                is_upper_case: false,
            });
        }
        return out;
    }

    // No target: ươ/ưo(i/c/ng) + o -> uô.
    if REG_UH_O.is_match(&flatten(
        comp,
        Mode::VIETNAMESE | Mode::TONELESS | Mode::LOWERCASE,
    )) {
        let vowels = filter_appending_composition(&get_rightmost_vowels(comp));
        if !vowels.is_empty() {
            let trans = Transformation {
                id: ids.next_id(),
                target: Some(vowels[0].id),
                rule: Rule {
                    effect_type: EffectType::MarkTransformation,
                    key: '\0',
                    effect: Mark::None as u8,
                    ..Default::default()
                },
                is_upper_case: false,
            };
            let mut probe = comp.to_vec();
            probe.push(trans.clone());
            let (t, applicable_rule) = find_target(&probe, rules, flags);
            if let Some(t_id) = t {
                if t_id != vowels[0].id {
                    out.push(trans);
                    out.push(Transformation {
                        id: ids.next_id(),
                        rule: applicable_rule,
                        target: Some(t_id),
                        is_upper_case,
                    });
                    return out;
                }
            }
        }
    }

    // An effect key with no target tries to undo its effects: ươ + w -> uow.
    let undo = generate_undo_transformations(ids, comp, rules, flags);
    if !undo.is_empty() {
        out.extend(undo);
        out.push(new_appending_trans(ids, lower_key, is_upper_case));
    }
    out
}

pub(crate) fn generate_fallback_transformations(
    ids: &mut IdGen,
    rules: &[Rule],
    lower_key: char,
    is_upper_case: bool,
) -> Vec<Transformation> {
    let trans = generate_appending_trans(ids, rules, lower_key, is_upper_case);
    let appended = trans.rule.appended_rules.clone();
    let mut out = vec![trans];
    for appended_rule in &appended {
        let upper = is_upper_case || appended_rule.effect_on.is_uppercase();
        let mut rule = appended_rule.clone();
        rule.key = '\0'; // virtual key
        rule.effect_on = to_lower(rule.effect_on);
        rule.result = rule.effect_on;
        out.push(Transformation {
            id: ids.next_id(),
            rule,
            target: None,
            is_upper_case: upper,
        });
    }
    out
}

/// Rebuild a composition as plain appends of its raw keys (drops effects).
pub(crate) fn break_composition(ids: &mut IdGen, comp: &[Transformation]) -> Vec<Transformation> {
    comp.iter()
        .filter(|t| t.rule.key != '\0')
        .map(|t| new_appending_trans(ids, t.rule.key, t.is_upper_case))
        .collect()
}

/// A deferred mutation: set the transformation with `id` to target `new_target`.
pub(crate) type Retarget = (TransId, Option<TransId>);

/// Move the most recent tone onto the correct vowel if the syllable changed.
///
/// Returns the transformations to append and, if the existing tone must move,
/// the [`Retarget`] the caller applies to its owned composition.
pub(crate) fn refresh_last_tone_target(
    ids: &mut IdGen,
    comp: &[Transformation],
    std_style: bool,
) -> (Vec<Transformation>, Option<Retarget>) {
    let rightmost = get_rightmost_vowels(comp);
    let Some(lt_idx) = get_last_tone_transformation(comp) else {
        return (Vec::new(), None);
    };
    if rightmost.is_empty() {
        return (Vec::new(), None);
    }
    let lt_id = comp[lt_idx].id;
    let lt_target = comp[lt_idx].target;
    let lt_rule = comp[lt_idx].rule.clone();
    let new_target = find_tone_target(comp, std_style);
    if lt_target == new_target {
        return (Vec::new(), None);
    }
    let mut out = Vec::new();
    out.push(Transformation {
        id: ids.next_id(),
        target: new_target,
        rule: Rule {
            key: '\0',
            effect_type: EffectType::ToneTransformation,
            effect: Tone::None as u8,
            ..Default::default()
        },
        is_upper_case: false,
    });
    let mut override_rule = lt_rule;
    override_rule.key = '\0';
    out.push(Transformation {
        id: ids.next_id(),
        target: new_target,
        rule: override_rule,
        is_upper_case: false,
    });
    (out, Some((lt_id, new_target)))
}

/// Find the tone/mark target for a single key's rules (used by the uow shortcut).
pub(crate) fn find_target_by_rules(
    comp: &[Transformation],
    rules: &[Rule],
    flags: EngineFlags,
) -> (Option<TransId>, Rule) {
    find_target(comp, rules, flags)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input_method::parse_input_method;

    /// Drive a whole string through the same pipeline the engine will use and
    /// flatten the result, so these tests exercise the real algorithm.
    fn type_word(im_name: &str, keys: &str, flags: EngineFlags) -> String {
        let im = parse_input_method(im_name).unwrap();
        let mut ids = IdGen::new();
        let mut comp: Vec<Transformation> = Vec::new();
        for ch in keys.chars() {
            let lower = to_lower(ch);
            let is_upper = ch.is_uppercase();
            let applicable: Vec<Rule> = im
                .rules
                .iter()
                .filter(|r| r.key == lower)
                .cloned()
                .collect();

            // Mirror BambooEngine::new_composition for the last syllable.
            let syl_split = extract_last_syllable(&comp);
            let previous = comp[..syl_split].to_vec();
            let mut syllable = comp[syl_split..].to_vec();

            let mut generated =
                generate_transformations(&mut ids, &syllable, &applicable, flags, lower, is_upper);
            if generated.is_empty() {
                generated =
                    generate_fallback_transformations(&mut ids, &applicable, lower, is_upper);
            }
            let mut combined = syllable.clone();
            combined.extend(generated.iter().cloned());
            let (refresh, retarget) = refresh_last_tone_target(
                &mut ids,
                &combined,
                flags.contains(EngineFlags::STD_TONE_STYLE),
            );
            generated.extend(refresh);

            syllable.extend(generated);
            let mut next = previous;
            next.extend(syllable);
            if let Some((id, new_target)) = retarget {
                for t in next.iter_mut() {
                    if t.id == id {
                        t.target = new_target;
                    }
                }
            }
            comp = next;
        }
        flatten(&comp, Mode::VIETNAMESE)
    }

    #[test]
    fn telex_basic_words() {
        let f = EngineFlags::STD;
        assert_eq!(type_word("Telex", "tieesng", f), "tiếng");
        assert_eq!(type_word("Telex", "Vieejt", f), "Việt");
        assert_eq!(type_word("Telex", "ddaji", f), "đại");
        assert_eq!(type_word("Telex", "nguoiwf", f), "người");
    }

    #[test]
    fn telex_tone_reposition() {
        // Tone must migrate when the syllable gains a vowel.
        let f = EngineFlags::STD;
        assert_eq!(type_word("Telex", "chuyeenr", f), "chuyển");
    }

    #[test]
    fn double_key_undoes_effect() {
        let f = EngineFlags::STD;
        // aa -> â, aaa -> a (raw undo on third press is handled as append).
        assert_eq!(type_word("Telex", "aa", f), "â");
    }
}
