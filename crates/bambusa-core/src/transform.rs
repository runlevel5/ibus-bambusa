//! The composition: an ordered list of [`Transformation`]s and the machinery
//! that builds and edits it from keystrokes.

use std::sync::LazyLock;

use regex::Regex;

use crate::flatten::{
    first_char, flatten, flatten_appenders_into, flatten_indices_into, flatten_with_extra,
};
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
#[derive(Clone, Debug, PartialEq)]
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

pub(crate) fn by_id(comp: &[Transformation], id: TransId) -> Option<&Transformation> {
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

fn find_root_target(comp: &[Transformation], id: TransId) -> TransId {
    match by_id(comp, id).and_then(|t| t.target) {
        Some(parent) => find_root_target(comp, parent),
        None => id,
    }
}

/// Whether the composition forms a spellable (possibly partial) syllable.
pub(crate) fn is_valid(comp: &[Transformation], input_is_full_complete: bool) -> bool {
    let mut buf = String::new();
    is_valid_buf(comp, input_is_full_complete, &mut buf)
}

/// As [`is_valid`], but reusing a caller-owned scratch buffer so a hot loop
/// (e.g. [`extract_last_syllable`]) allocates once instead of per call.
fn is_valid_buf(comp: &[Transformation], input_is_full_complete: bool, buf: &mut String) -> bool {
    if comp.len() <= 1 {
        return true;
    }
    // Reuse a single CVC split for both the tone check and the spelling check.
    let split = cvc_split(comp);
    // The most recent tone must be compatible with the final consonant.
    for trans in comp.iter().rev() {
        if trans.rule.effect_type == EffectType::ToneTransformation {
            let last_tone = Tone::try_from(trans.rule.effect).unwrap_or_default();
            if !tone_compatible_with_lc(comp, split.lc(), last_tone, buf) {
                return false;
            }
            break;
        }
    }
    is_valid_cvc(
        comp,
        split.fc(),
        split.vo(),
        split.lc(),
        input_is_full_complete,
        buf,
    )
}

fn has_valid_tone(comp: &[Transformation], tone: Tone) -> bool {
    if tone == Tone::None || tone == Tone::Acute || tone == Tone::Dot {
        return true;
    }
    let split = cvc_split(comp);
    let mut buf = String::new();
    tone_compatible_with_lc(comp, split.lc(), tone, &mut buf)
}

/// Whether `tone` may sit on a syllable whose last consonant group is the
/// appenders at `lc` (indices into `comp`). Stop consonants only admit the
/// acute or dot tones. `buf` is a caller-owned scratch buffer.
fn tone_compatible_with_lc(
    comp: &[Transformation],
    lc: &[usize],
    tone: Tone,
    buf: &mut String,
) -> bool {
    if tone == Tone::None || tone == Tone::Acute || tone == Tone::Dot {
        return true;
    }
    if lc.is_empty() {
        return true;
    }
    flatten_indices_into(comp, lc, Mode::ENGLISH | Mode::LOWERCASE, buf);
    !matches!(buf.as_str(), "c" | "k" | "p" | "t" | "ch")
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
    let split = cvc_split(comp);
    find_tone_target_in(comp, &split, std_style)
}

/// As [`find_tone_target`], but reusing a CVC split the caller already computed.
fn find_tone_target_in(
    comp: &[Transformation],
    split: &CvcSplit,
    std_style: bool,
) -> Option<TransId> {
    let lc_empty = split.lc().is_empty();
    // The vowel appenders only (matching the historical `filter_appending`).
    let vowels: Vec<usize> = split
        .vo()
        .iter()
        .copied()
        .filter(|&i| comp[i].rule.effect_type == EffectType::Appending)
        .collect();
    let mark_mode = Mode::ENGLISH | Mode::LOWERCASE | Mode::TONELESS | Mode::MARKLESS;
    match vowels.len() {
        1 => Some(comp[vowels[0]].id),
        2 if std_style => {
            // A horn/hat carrier ('ơ'/'ê') takes the tone. The carrier may be a
            // vowel appender or an effect that produces it, so scan both — the
            // appenders first, then the effects targeting them in composition
            // order — mirroring the historical materialised vowel group, where
            // the last match wins.
            let mut target = None;
            for &i in split.vo() {
                let t = &comp[i];
                if t.rule.result == 'ơ' || t.rule.result == 'ê' {
                    target = Some(t.id);
                }
            }
            for effect in comp {
                if let Some(tgt) = effect.target
                    && (effect.rule.result == 'ơ' || effect.rule.result == 'ê')
                    && split.vo().iter().any(|&i| comp[i].id == tgt)
                {
                    target = Some(tgt);
                }
            }
            target.or(Some(if !lc_empty {
                comp[vowels[1]].id
            } else {
                comp[vowels[0]].id
            }))
        }
        2 => {
            if !lc_empty {
                Some(comp[vowels[1]].id)
            } else {
                let mut buf = String::new();
                flatten_appenders_into(comp, &vowels, mark_mode, &mut buf);
                Some(match buf.as_str() {
                    "oa" | "oe" | "uy" | "ue" | "uo" => comp[vowels[1]].id,
                    _ => comp[vowels[0]].id,
                })
            }
        }
        3 => {
            let mut buf = String::new();
            flatten_appenders_into(comp, &vowels, mark_mode, &mut buf);
            Some(if buf == "uye" {
                comp[vowels[2]].id
            } else {
                comp[vowels[1]].id
            })
        }
        _ => None,
    }
}

/// An index-based decomposition of a composition into first-consonant, vowel
/// and last-consonant groups.
///
/// `appenders` holds the indices (into the source composition) of the appending
/// transformations, in order; the two boundaries partition them so a caller can
/// inspect or flatten a group by index without cloning any transformation.
struct CvcSplit {
    appenders: Vec<usize>,
    fc_end: usize,
    vo_end: usize,
}

impl CvcSplit {
    /// Indices of the first-consonant appenders.
    fn fc(&self) -> &[usize] {
        &self.appenders[..self.fc_end]
    }

    /// Indices of the vowel appenders.
    fn vo(&self) -> &[usize] {
        &self.appenders[self.fc_end..self.vo_end]
    }

    /// Indices of the last-consonant appenders.
    fn lc(&self) -> &[usize] {
        &self.appenders[self.vo_end..]
    }
}

/// Length of the trailing run of `appenders[..upto]` whose result vowel-ness
/// equals `last_is_vowel`; returns the index where that run begins.
///
/// Every entry of `appenders` is an appending transformation (target `None`),
/// so unlike the historical slice form this need not re-check the target.
fn atomic_split_idx(
    comp: &[Transformation],
    appenders: &[usize],
    upto: usize,
    last_is_vowel: bool,
) -> usize {
    let mut i = upto;
    while i > 0 {
        if is_vowel(comp[appenders[i - 1]].rule.result) != last_is_vowel {
            break;
        }
        i -= 1;
    }
    i
}

/// Partition the appending transformations of `comp` into the CVC groups.
fn cvc_split(comp: &[Transformation]) -> CvcSplit {
    let appenders: Vec<usize> = comp
        .iter()
        .enumerate()
        .filter(|(_, t)| t.target.is_none())
        .map(|(i, _)| i)
        .collect();
    let n = appenders.len();
    let head_split = atomic_split_idx(comp, &appenders, n, false);
    let fc_split = atomic_split_idx(comp, &appenders, head_split, true);
    let mut fc_end = fc_split;
    let mut vo_end = head_split;

    // All-consonant run (no vowel): the trailing consonants become the first
    // consonant. Matches the historical reshuffle, which only fires when both
    // the first consonant and vowel are empty (`fc_split == head_split == 0`).
    if head_split < n && fc_split == 0 && head_split == 0 {
        fc_end = n;
        vo_end = n;
    }

    // 'gi' and 'qu' are treated as qualified initial consonants:
    //   ['g', 'ia', ''] -> ['gi', 'a', ''], ['q', 'ua', ''] -> ['qu', 'a', '']
    // but not ['g', 'ie', 'ng'].
    let vo_len = vo_end - fc_end;
    if fc_end == 1 && vo_len >= 1 {
        let fc0 = comp[appenders[0]].rule.result;
        let vo0 = comp[appenders[fc_end]].rule.result;
        let last_consonant_empty = vo_end == n;
        let gi = fc0 == 'g'
            && vo0 == 'i'
            && vo_len > 1
            && (comp[appenders[fc_end + 1]].rule.result != 'e' || last_consonant_empty);
        let qu = fc0 == 'q' && vo0 == 'u';
        if gi || qu {
            fc_end += 1;
        }
    }

    CvcSplit {
        appenders,
        fc_end,
        vo_end,
    }
}

/// Clone the appenders at `indices` and append the effects in `comp` that
/// target them, reproducing one historical CVC group. Test-only: production
/// code works with [`CvcSplit`] index ranges and never materialises groups.
#[cfg(test)]
fn materialize_group(comp: &[Transformation], indices: &[usize]) -> Vec<Transformation> {
    let mut group: Vec<Transformation> = indices.iter().map(|&i| comp[i].clone()).collect();
    attach_effects(&mut group, comp);
    group
}

/// Materialise the three CVC groups. Test-only reference used to prove
/// [`cvc_split`] is equivalent to the historical decomposition.
#[cfg(test)]
fn extract_cvc_trans(
    comp: &[Transformation],
) -> (
    Vec<Transformation>,
    Vec<Transformation>,
    Vec<Transformation>,
) {
    let split = cvc_split(comp);
    (
        materialize_group(comp, split.fc()),
        materialize_group(comp, split.vo()),
        materialize_group(comp, split.lc()),
    )
}

/// Append every effect in `comp` that targets one of the current members of
/// `group`. Test-only (see [`materialize_group`]).
#[cfg(test)]
fn attach_effects(group: &mut Vec<Transformation>, comp: &[Transformation]) {
    let appenders = group.len();
    for effect in comp {
        if let Some(target) = effect.target
            && group[..appenders].iter().any(|t| t.id == target)
        {
            group.push(effect.clone());
        }
    }
}

/// Split index such that `comp[..n]` is everything before the last word and
/// `comp[n..]` is the last word.
pub(crate) fn extract_last_word(comp: &[Transformation], effect_keys: &[char]) -> usize {
    for i in (0..comp.len()).rev() {
        let Some(c0) = first_char(
            &comp[i..],
            Mode::VIETNAMESE | Mode::LOWERCASE | Mode::TONELESS | Mode::MARKLESS,
        ) else {
            continue;
        };
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
        let Some(c0) = first_char(&comp[i..], Mode::ENGLISH) else {
            continue;
        };
        if is_space(c0) {
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
    let mut buf = String::new();
    for i in 0..last.len() {
        if !is_valid_buf(&last[anchor..=i], false, &mut buf) {
            anchor = i;
        }
    }
    word_split + anchor
}

/// `str` is the caller's `flatten(comp, VIETNAMESE)`, threaded in to avoid
/// recomputing it.
fn find_mark_target(comp: &[Transformation], rules: &[Rule], str: &str) -> (Option<TransId>, Rule) {
    for i in (0..comp.len()).rev() {
        let trans = &comp[i];
        for rule in rules {
            if rule.effect_type != EffectType::MarkTransformation {
                continue;
            }
            if trans.rule.result == rule.effect_on && rule.effect > 0 {
                let target = find_root_target(comp, trans.id);
                let extra = temp_effect(Some(target), rule.clone());
                if *str == flatten_with_extra(comp, &extra, Mode::VIETNAMESE) {
                    continue;
                }
                let mut probe = comp.to_vec();
                probe.push(extra);
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
        let extra = temp_effect(target, rule.clone());
        if str == flatten_with_extra(comp, &extra, Mode::VIETNAMESE) {
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
    find_mark_target(comp, rules, &str)
}

/// Resolve the tone target for `rule` under the active flags.
fn tone_target_for(comp: &[Transformation], rule: &Rule, flags: EngineFlags) -> Option<TransId> {
    if flags.contains(EngineFlags::FREE_TONE_MARKING) {
        if has_valid_tone(comp, Tone::try_from(rule.effect).unwrap_or_default()) {
            return find_tone_target(comp, flags.contains(EngineFlags::STD_TONE_STYLE));
        }
        None
    } else if let Some(la) = find_last_appending_trans(comp)
        && by_id(comp, la).is_some_and(|t| is_vowel(t.rule.effect_on))
    {
        Some(la)
    } else {
        None
    }
}

/// `str` is the caller's `flatten(comp, VIETNAMESE|TONELESS|LOWERCASE)`,
/// threaded in to avoid recomputing it.
fn generate_undo_transformations(
    ids: &mut IdGen,
    comp: &[Transformation],
    rules: &[Rule],
    flags: EngineFlags,
    str: &str,
) -> Vec<Transformation> {
    let mut out = Vec::new();
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
                        let extra = temp_effect(Some(target), probe_rule.clone());
                        if str
                            == flatten_with_extra(
                                comp,
                                &extra,
                                Mode::VIETNAMESE | Mode::TONELESS | Mode::LOWERCASE,
                            )
                            .as_str()
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
    if let Some(last) = comp.last()
        && last.rule.effect_type == EffectType::Appending
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

    // No target: ươ/ưo(i/c/ng) + o -> uô. The toneless form is reused by the
    // undo pass below.
    let toneless = flatten(comp, Mode::VIETNAMESE | Mode::TONELESS | Mode::LOWERCASE);
    if REG_UH_O.is_match(&toneless) {
        let split = cvc_split(comp);
        let first_vowel = split
            .vo()
            .iter()
            .copied()
            .find(|&i| comp[i].rule.effect_type == EffectType::Appending);
        if let Some(vi) = first_vowel {
            let vowel0_id = comp[vi].id;
            let trans = Transformation {
                id: ids.next_id(),
                target: Some(vowel0_id),
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
            if let Some(t_id) = t
                && t_id != vowel0_id
            {
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

    // An effect key with no target tries to undo its effects: ươ + w -> uow.
    let undo = generate_undo_transformations(ids, comp, rules, flags, &toneless);
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
    let Some(lt_idx) = get_last_tone_transformation(comp) else {
        return (Vec::new(), None);
    };
    let split = cvc_split(comp);
    if split.vo().is_empty() {
        return (Vec::new(), None);
    }
    let lt_id = comp[lt_idx].id;
    let lt_target = comp[lt_idx].target;
    let lt_rule = comp[lt_idx].rule.clone();
    let new_target = find_tone_target_in(comp, &split, std_style);
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

    /// Drive a whole string through the same pipeline the engine will use,
    /// returning the composition after each keystroke so tests can inspect the
    /// real intermediate states.
    fn drive(im_name: &str, keys: &str, flags: EngineFlags) -> Vec<Vec<Transformation>> {
        let im = parse_input_method(im_name).unwrap();
        let mut ids = IdGen::new();
        let mut comp: Vec<Transformation> = Vec::new();
        let mut steps = Vec::new();
        for ch in keys.chars() {
            let lower = to_lower(ch);
            let is_upper = ch.is_uppercase();
            let applicable: Vec<Rule> = im
                .rules
                .iter()
                .filter(|r| r.key == lower)
                .cloned()
                .collect();

            // Mirror BambusaEngine::new_composition for the last syllable.
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
            steps.push(comp.clone());
        }
        steps
    }

    /// Drive a string and flatten the final composition.
    fn type_word(im_name: &str, keys: &str, flags: EngineFlags) -> String {
        match drive(im_name, keys, flags).last() {
            Some(comp) => flatten(comp, Mode::VIETNAMESE),
            None => String::new(),
        }
    }

    // --- Reference implementation of the historical CVC split, frozen here so a
    // --- differential test can prove the new index-based split is equivalent.

    fn ref_atomic(comp: &[Transformation], last_is_vowel: bool) -> usize {
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

    fn ref_extract_cvc(
        comp: &[Transformation],
    ) -> (
        Vec<Transformation>,
        Vec<Transformation>,
        Vec<Transformation>,
    ) {
        let appending: Vec<Transformation> = comp
            .iter()
            .filter(|t| t.target.is_none())
            .cloned()
            .collect();
        let head_split = ref_atomic(&appending, false);
        let mut last_consonant = appending[head_split..].to_vec();
        let head = &appending[..head_split];
        let fc_split = ref_atomic(head, true);
        let mut first_consonant = head[..fc_split].to_vec();
        let mut vowel = head[fc_split..].to_vec();

        if !last_consonant.is_empty() && vowel.is_empty() && first_consonant.is_empty() {
            first_consonant = last_consonant;
            vowel = Vec::new();
            last_consonant = Vec::new();
        }

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

        let mut fc = first_consonant;
        let mut vo = vowel;
        let mut lc = last_consonant;
        attach_effects(&mut fc, comp);
        attach_effects(&mut vo, comp);
        attach_effects(&mut lc, comp);
        (fc, vo, lc)
    }

    /// Frozen copy of the historical `find_tone_target`, for differential testing.
    fn ref_find_tone_target(comp: &[Transformation], std_style: bool) -> Option<TransId> {
        if comp.is_empty() {
            return None;
        }
        let (_, vo, lc) = ref_extract_cvc(comp);
        let vowels: Vec<Transformation> = vo
            .iter()
            .filter(|t| t.rule.effect_type == EffectType::Appending)
            .cloned()
            .collect();
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

    /// Tiny deterministic PRNG so the fuzz corpus is reproducible.
    struct Lcg(u64);
    impl Lcg {
        fn next(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            self.0 >> 33
        }
    }

    #[test]
    fn cvc_split_and_tone_target_match_reference() {
        // Effect keys vary by method; a broad alphabet drives the engine into
        // many composition shapes (gi/qu, all-consonant, multi-vowel, marks).
        let configs: &[(&str, &str)] = &[
            ("Telex", "abcdeghiklmnopqrstuvxysfrxjwzAEO"),
            ("Telex 2", "aeiouwdsfrxjbcghklmnpqtv][}{"),
            ("VNI", "aeioudbcghklmnpqstvx0123456789"),
            ("VIQR", "aeioudbcghklmnpqstvx^'`?~.+()"),
        ];
        let flag_sets = [
            EngineFlags::STD,
            EngineFlags::empty(),
            EngineFlags::FREE_TONE_MARKING,
            EngineFlags::FREE_TONE_MARKING | EngineFlags::STD_TONE_STYLE,
        ];

        let mut rng = Lcg(0x1234_5678_9abc_def0);
        let mut checked = 0u64;
        for (im, alphabet) in configs {
            let alpha: Vec<char> = alphabet.chars().collect();
            for &flags in &flag_sets {
                for _ in 0..120 {
                    let len = 1 + (rng.next() % 9) as usize;
                    let keys: String = (0..len)
                        .map(|_| alpha[(rng.next() as usize) % alpha.len()])
                        .collect();
                    for comp in drive(im, &keys, flags) {
                        // Every contiguous sub-range: is_valid runs on such slices.
                        for a in 0..=comp.len() {
                            for b in a..=comp.len() {
                                let slice = &comp[a..b];
                                assert_eq!(
                                    extract_cvc_trans(slice),
                                    ref_extract_cvc(slice),
                                    "cvc im={im} keys={keys:?} flags={flags:?} range={a}..{b}"
                                );
                                for std_style in [false, true] {
                                    assert_eq!(
                                        find_tone_target(slice, std_style),
                                        ref_find_tone_target(slice, std_style),
                                        "tone_target im={im} keys={keys:?} std={std_style} range={a}..{b}"
                                    );
                                }
                                checked += 1;
                            }
                        }
                    }
                }
            }
        }
        assert!(checked > 100_000, "corpus too small: {checked}");
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
