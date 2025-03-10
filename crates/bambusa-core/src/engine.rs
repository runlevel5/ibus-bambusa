//! The public composition engine: feed it keystrokes, read back Vietnamese.

use crate::flatten::flatten;
use crate::input_method::InputMethod;
use crate::mode::{EngineFlags, Mode};
use crate::rules::Rule;
use crate::transform::{
    break_composition, by_id, extract_last_syllable, extract_last_word,
    extract_last_word_with_punctuation_marks, find_last_appending_trans, find_target_by_rules,
    generate_fallback_transformations, generate_transformations, is_valid as composition_is_valid,
    matches_uoh_tail, new_appending_trans, refresh_last_tone_target, IdGen, Retarget,
    Transformation,
};
use crate::unicode_tables::{can_process_key, to_lower};

/// Composes Vietnamese text from a stream of keystrokes for one input method.
#[derive(Debug)]
pub struct BambooEngine {
    composition: Vec<Transformation>,
    input_method: InputMethod,
    flags: EngineFlags,
    ids: IdGen,
}

impl BambooEngine {
    pub fn new(input_method: InputMethod, flags: EngineFlags) -> Self {
        Self {
            composition: Vec::new(),
            input_method,
            flags,
            ids: IdGen::new(),
        }
    }

    pub fn input_method(&self) -> &InputMethod {
        &self.input_method
    }

    pub fn set_flags(&mut self, flags: EngineFlags) {
        self.flags = flags;
    }

    pub fn flags(&self) -> EngineFlags {
        self.flags
    }

    /// Whether the last word is a valid (optionally fully complete) syllable.
    pub fn is_valid(&self, input_is_full_complete: bool) -> bool {
        let split = extract_last_word(&self.composition, &self.input_method.keys);
        composition_is_valid(&self.composition[split..], input_is_full_complete)
    }

    /// The composed text under `mode` (last word, last syllable, or full text).
    pub fn processed_string(&self, mode: Mode) -> String {
        if mode.contains(Mode::FULL_TEXT) {
            return flatten(&self.composition, mode);
        }
        if mode.contains(Mode::PUNCTUATION) {
            let split = extract_last_word_with_punctuation_marks(&self.composition);
            return flatten(&self.composition[split..], Mode::VIETNAMESE);
        }
        let split = extract_last_word(&self.composition, &self.input_method.keys);
        flatten(&self.composition[split..], mode)
    }

    pub fn can_process_key(&self, key: char) -> bool {
        can_process_key(key, &self.input_method.keys)
    }

    pub fn reset(&mut self) {
        self.composition.clear();
    }

    pub fn process_string(&mut self, input: &str, mode: Mode) {
        for key in input.chars() {
            self.process_key(key, mode);
        }
    }

    pub fn process_key(&mut self, key: char, mode: Mode) {
        let lower = to_lower(key);
        let is_upper = key.is_uppercase();
        if mode.contains(Mode::ENGLISH) || !self.can_process_key(lower) {
            let trans = new_appending_trans(&mut self.ids, lower, is_upper);
            if mode.contains(Mode::IN_REVERSE_ORDER) {
                self.composition.insert(0, trans);
            } else {
                self.composition.push(trans);
            }
            return;
        }
        let comp = std::mem::take(&mut self.composition);
        self.composition = self.new_composition(comp, lower, is_upper);
    }

    /// Restore the last word either to its raw keystrokes or by replaying them
    /// through the Vietnamese composer.
    pub fn restore_last_word(&mut self, to_vietnamese: bool) {
        let split = extract_last_word(&self.composition, &self.input_method.keys);
        let last_word = self.composition[split..].to_vec();
        if last_word.is_empty() {
            return;
        }
        let mut result = self.composition[..split].to_vec();
        if to_vietnamese {
            let mut rebuilt = Vec::new();
            for trans in &last_word {
                rebuilt = self.new_composition(rebuilt, trans.rule.key, trans.is_upper_case);
            }
            result.extend(rebuilt);
        } else {
            result.extend(break_composition(&mut self.ids, &last_word));
        }
        self.composition = result;
    }

    /// Remove the last appended character and the effects attached to it.
    pub fn remove_last_char(&mut self, refresh: bool) {
        let Some(last_id) = find_last_appending_trans(&self.composition) else {
            return;
        };
        let last_key = by_id(&self.composition, last_id)
            .expect("last appending trans is present")
            .rule
            .key;
        if !self.can_process_key(last_key) {
            self.composition.pop();
            return;
        }
        let split = extract_last_word(&self.composition, &self.input_method.keys);
        let mut new_comb: Vec<Transformation> = self.composition[split..]
            .iter()
            .filter(|t| t.target != Some(last_id) && t.id != last_id)
            .cloned()
            .collect();
        if refresh {
            let (extra, retarget) = self.refresh_tone(&new_comb);
            new_comb.extend(extra);
            apply_retarget(&mut new_comb, retarget);
        }
        let mut result = self.composition[..split].to_vec();
        result.extend(new_comb);
        self.composition = result;
    }

    fn applicable_rules(&self, key: char) -> Vec<Rule> {
        let lower = to_lower(key);
        self.input_method
            .rules
            .iter()
            .filter(|r| r.key == lower)
            .cloned()
            .collect()
    }

    fn new_composition(
        &mut self,
        composition: Vec<Transformation>,
        key: char,
        is_upper: bool,
    ) -> Vec<Transformation> {
        let split = extract_last_syllable(&composition);
        let mut last_syllable = composition[split..].to_vec();
        let (generated, retarget) = self.generate(&last_syllable, key, is_upper);
        last_syllable.extend(generated);

        let mut result = composition[..split].to_vec();
        result.extend(last_syllable);
        apply_retarget(&mut result, retarget);
        result
    }

    /// Generate the transformations a keypress adds: try rule-based generation,
    /// fall back to a plain append plus the uow shortcut, then reposition the
    /// last tone. Returns the new transformations plus a deferred retarget.
    fn generate(
        &mut self,
        composition: &[Transformation],
        key: char,
        is_upper: bool,
    ) -> (Vec<Transformation>, Option<Retarget>) {
        let rules = self.applicable_rules(key);
        let mut transformations = generate_transformations(
            &mut self.ids,
            composition,
            &rules,
            self.flags,
            key,
            is_upper,
        );
        if transformations.is_empty() {
            transformations =
                generate_fallback_transformations(&mut self.ids, &rules, key, is_upper);
            let mut new_comp = composition.to_vec();
            new_comp.extend(transformations.iter().cloned());
            if let Some(virtual_trans) = self.apply_uow_shortcut(&new_comp) {
                transformations.push(virtual_trans);
            }
        }
        let mut combined = composition.to_vec();
        combined.extend(transformations.iter().cloned());
        let (refresh, retarget) = self.refresh_tone(&combined);
        transformations.extend(refresh);
        (transformations, retarget)
    }

    /// The `uow` typing shortcut: a virtual horn rule targeting `u`/`o`.
    fn apply_uow_shortcut(&mut self, composition: &[Transformation]) -> Option<Transformation> {
        let str = flatten(composition, Mode::TONELESS | Mode::LOWERCASE);
        let super_key = *self.input_method.super_keys.first()?;
        if !matches_uoh_tail(&str) {
            return None;
        }
        let rules = self.applicable_rules(super_key);
        let (target, mut missing_rule) = find_target_by_rules(composition, &rules, self.flags);
        let target_id = target?;
        missing_rule.key = '\0';
        Some(Transformation {
            id: self.ids.next_id(),
            rule: missing_rule,
            target: Some(target_id),
            is_upper_case: false,
        })
    }

    /// Reposition the most recent tone, but only under free tone marking and
    /// when the syllable is already valid.
    fn refresh_tone(
        &mut self,
        composition: &[Transformation],
    ) -> (Vec<Transformation>, Option<Retarget>) {
        if self.flags.contains(EngineFlags::FREE_TONE_MARKING)
            && composition_is_valid(composition, false)
        {
            refresh_last_tone_target(
                &mut self.ids,
                composition,
                self.flags.contains(EngineFlags::STD_TONE_STYLE),
            )
        } else {
            (Vec::new(), None)
        }
    }
}

fn apply_retarget(composition: &mut [Transformation], retarget: Option<Retarget>) {
    if let Some((id, new_target)) = retarget {
        for trans in composition.iter_mut() {
            if trans.id == id {
                trans.target = new_target;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input_method::parse_input_method;

    fn engine(im: &str) -> BambooEngine {
        BambooEngine::new(parse_input_method(im).unwrap(), EngineFlags::STD)
    }

    fn typed(im: &str, keys: &str) -> String {
        let mut e = engine(im);
        e.process_string(keys, Mode::VIETNAMESE);
        e.processed_string(Mode::VIETNAMESE)
    }

    #[test]
    fn telex_words() {
        assert_eq!(typed("Telex", "tieesng"), "tiếng");
        assert_eq!(typed("Telex", "Vieejt"), "Việt");
        assert_eq!(typed("Telex", "nguoiwf"), "người");
        assert_eq!(typed("Telex", "chuyeenr"), "chuyển");
    }

    #[test]
    fn vni_and_viqr() {
        assert_eq!(typed("VNI", "tie61ng"), "tiếng");
        assert_eq!(typed("VIQR", "tie^'ng"), "tiếng");
    }

    #[test]
    fn remove_last_char_undoes_effects() {
        let mut e = engine("Telex");
        e.process_string("vieejt", Mode::VIETNAMESE);
        assert_eq!(e.processed_string(Mode::VIETNAMESE), "việt");
        e.remove_last_char(true);
        assert_eq!(e.processed_string(Mode::VIETNAMESE), "việ");
    }

    #[test]
    fn restore_last_word_to_raw() {
        let mut e = engine("Telex");
        e.process_string("dddd", Mode::VIETNAMESE);
        e.restore_last_word(false);
        assert_eq!(e.processed_string(Mode::FULL_TEXT | Mode::ENGLISH), "dddd");
    }

    #[test]
    fn english_mode_passes_through() {
        let mut e = engine("Telex");
        e.process_string("xin chao", Mode::ENGLISH);
        assert_eq!(e.processed_string(Mode::FULL_TEXT), "xin chao");
    }
}
