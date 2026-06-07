//! Preedit-mode engine handler: the default mode, where in-progress text is
//! shown as IBus preedit and committed on a word break.

use bambusa_core::{
    BambusaEngine, Mode, encode, has_any_vietnamese_rune, has_any_vietnamese_vowel,
    is_word_break_symbol, parse_input_method,
};
use gettextrs::gettext;
use ibus_zbus::{Action, EngineHandler, IBusPropList, IBusProperty};

use bambusa_config::{Config, IBFlags, InputMode};

use crate::keysyms;
use crate::macros::MacroTable;

/// Composes Vietnamese text and renders it through IBus preedit.
pub struct PreeditHandler {
    engine: BambusaEngine,
    config: Config,
    macros: MacroTable,
    should_restore_key_strokes: bool,
}

impl PreeditHandler {
    pub fn new(config: Config) -> Self {
        let im = parse_input_method(&config.input_method)
            .or_else(|| parse_input_method("Telex"))
            .expect("Telex is a built-in input method");
        let engine = BambusaEngine::new(im, config.engine_flags);
        let macros = build_macros(&config);
        Self {
            engine,
            config,
            macros,
            should_restore_key_strokes: false,
        }
    }

    fn flags(&self) -> IBFlags {
        self.config.ib_flags
    }

    fn encode_text(&self, text: &str) -> String {
        encode(&self.config.output_charset, text)
    }

    fn processed(&self, mode: Mode) -> String {
        self.engine.processed_string(mode)
    }

    fn raw_key_len(&self) -> usize {
        self.processed(Mode::ENGLISH | Mode::FULL_TEXT)
            .chars()
            .count()
    }

    fn rune_count(&self) -> usize {
        self.preedit_string().chars().count()
    }

    fn preedit_string(&self) -> String {
        if self.flags().contains(IBFlags::MACRO_ENABLED) {
            return self.processed(Mode::PUNCTUATION);
        }
        if self.should_fallback_to_english(true) {
            return self.processed(Mode::ENGLISH);
        }
        self.processed(Mode::VIETNAMESE)
    }

    fn bamboo_input_mode(&self) -> Mode {
        if self.should_fallback_to_english(false) {
            Mode::ENGLISH
        } else {
            Mode::VIETNAMESE
        }
    }

    fn should_fallback_to_english(&self, check_vn_rune: bool) -> bool {
        let f = self.flags();
        if !f.contains(IBFlags::AUTO_NON_VN_RESTORE) || !f.contains(IBFlags::SPELL_CHECK_ENABLED) {
            return false;
        }
        let vn_seq = self.processed(Mode::VIETNAMESE | Mode::LOWERCASE);
        if vn_seq.is_empty() {
            return false;
        }
        // Allow "dd" even outside a Vietnamese word — it's common in abbreviations.
        if f.contains(IBFlags::DD_FREE_STYLE)
            && !has_any_vietnamese_vowel(&vn_seq)
            && (vn_seq.ends_with('d') || vn_seq.contains('đ'))
        {
            return false;
        }
        if check_vn_rune && !has_any_vietnamese_rune(&vn_seq) {
            return false;
        }
        // While typing, validity is rule-based only — the dictionary needs a
        // complete word, so it is consulted at commit time (must_fallback).
        if !f.contains(IBFlags::SPELL_CHECK_WITH_RULES) {
            return false;
        }
        !self.engine.is_valid(false)
    }

    fn must_fallback_to_english(&self) -> bool {
        let f = self.flags();
        if !f.contains(IBFlags::AUTO_NON_VN_RESTORE) || !f.contains(IBFlags::SPELL_CHECK_ENABLED) {
            return false;
        }
        let vn_seq = self.processed(Mode::VIETNAMESE | Mode::LOWERCASE);
        if vn_seq.is_empty() {
            return false;
        }
        if f.contains(IBFlags::DD_FREE_STYLE) && vn_seq.contains('đ') {
            return false;
        }
        // The dictionary takes precedence: when enabled, a word must be a known
        // dictionary entry (syllable rules alone are not enough). With only rules
        // enabled, rule validation decides; with neither, nothing is restored.
        match (
            f.contains(IBFlags::SPELL_CHECK_WITH_RULES),
            f.contains(IBFlags::SPELL_CHECK_WITH_DICTS),
        ) {
            (_, true) => !crate::dict::contains(&vn_seq),
            (true, false) => !self.engine.is_valid(true),
            (false, false) => false,
        }
    }

    fn composed_string(&self, old_text: &str) -> String {
        if has_any_vietnamese_rune(old_text) && self.must_fallback_to_english() {
            self.processed(Mode::ENGLISH)
        } else {
            old_text.to_string()
        }
    }

    fn is_printable_key(&self, state: u32, keyval: u32) -> bool {
        keysyms::is_valid_state(state) && self.is_valid_key_val(keyval)
    }

    fn is_valid_key_val(&self, keyval: u32) -> bool {
        let key = char::from_u32(keyval).unwrap_or('\0');
        if keyval == keysyms::BACKSPACE || is_word_break_symbol(key) {
            return true;
        }
        if keyval == keysyms::TAB && self.macro_text().is_some() {
            return true;
        }
        self.engine.can_process_key(key)
    }

    /// Map `[`/`]` to `{`/`}` (and back) for input methods that bind brackets.
    fn to_upper(&self, key: char) -> char {
        let mapped = match key {
            '[' => '{',
            ']' => '}',
            '{' => '[',
            '}' => ']',
            _ => return key,
        };
        if self.engine.input_method().appending_keys.contains(&key) {
            mapped
        } else {
            key
        }
    }

    /// Process one key into the composer and return `(text, is_word_break)`.
    fn commit_text(&mut self, keyval: u32, keycode: u32, state: u32) -> (String, bool) {
        let key = char::from_u32(keyval).unwrap_or('\0');
        let is_printable = self.is_printable_key(state, keyval);
        let old_text = self.preedit_string();

        if self.should_restore_key_strokes {
            self.should_restore_key_strokes = false;
            self.engine
                .restore_last_word(!has_any_vietnamese_rune(&old_text));
            return (self.preedit_string(), false);
        }

        if is_printable && self.engine.can_process_key(key) {
            let key = if state & keysyms::LOCK_MASK != 0 {
                self.to_upper(key)
            } else {
                key
            };
            self.engine.process_key(key, self.bamboo_input_mode());

            if self.engine.input_method().appending_keys.contains(&key) {
                let new_text = if self.should_fallback_to_english(true) {
                    self.processed(Mode::ENGLISH)
                } else {
                    self.processed(Mode::VIETNAMESE)
                };
                let full_seq = self.processed(Mode::VIETNAMESE);
                if full_seq.ends_with(key) {
                    // e.g. `[[` -> `[`
                    let ret = self.preedit_string();
                    let is_wbs = ret.chars().last().is_some_and(is_word_break_symbol);
                    if is_wbs {
                        self.engine.remove_last_char(false);
                        self.engine.process_key(' ', Mode::ENGLISH);
                    }
                    return (ret, is_wbs);
                }
                if new_text.ends_with(key) {
                    // e.g. `f]` -> `f]`
                    let is_wbs = is_word_break_symbol(key);
                    if is_wbs {
                        self.engine.remove_last_char(false);
                        self.engine.process_key(' ', Mode::ENGLISH);
                    }
                    return (format!("{old_text}{key}"), is_wbs);
                }
                return (self.preedit_string(), false);
            }
            return (self.preedit_string(), false);
        }

        // Macro processing for keys the composer can't take: keep buffering while
        // the text is still a macro-key prefix, and expand once it is a full key.
        if self.flags().contains(IBFlags::MACRO_ENABLED) {
            let key_s = if is_printable {
                key.to_string()
            } else {
                String::new()
            };
            if is_printable && self.macros.has_prefix(&format!("{old_text}{key_s}")) {
                self.engine.process_key(key, Mode::ENGLISH);
                return (format!("{old_text}{key_s}"), false);
            }
            if self.macros.has_key(&old_text) {
                return (format!("{}{key_s}", self.expand_macro(&old_text)), true);
            }
        }

        (self.handle_non_vn_word(keyval, keycode, state), true)
    }

    /// Expand a macro key to its value, applying the typed key's case when
    /// auto-capitalize is on (all-lower → lower, all-upper → upper).
    fn expand_macro(&self, key: &str) -> String {
        let value = self.macros.get(key).unwrap_or_default().to_string();
        if !self.flags().contains(IBFlags::AUTO_CAPITALIZE_MACRO) {
            return value;
        }
        match macro_case(key) {
            MacroCase::AllLower => value.to_lowercase(),
            MacroCase::AllUpper => value.to_uppercase(),
            MacroCase::Mixed => value,
        }
    }

    /// The macro expansion for the current buffer, if it is a complete macro key.
    fn macro_text(&self) -> Option<String> {
        if !self.flags().contains(IBFlags::MACRO_ENABLED) {
            return None;
        }
        let text = self.processed(Mode::PUNCTUATION);
        self.macros.has_key(&text).then(|| self.expand_macro(&text))
    }

    fn handle_non_vn_word(&mut self, keyval: u32, _keycode: u32, state: u32) -> String {
        let key = char::from_u32(keyval).unwrap_or('\0');
        let is_printable = self.is_printable_key(state, keyval);
        let old_text = self.preedit_string();
        let key_s = if is_printable {
            key.to_string()
        } else {
            String::new()
        };

        if has_any_vietnamese_rune(&old_text) && self.must_fallback_to_english() {
            self.engine.restore_last_word(false);
            let new_text = self.processed(Mode::PUNCTUATION | Mode::ENGLISH) + &key_s;
            if is_printable {
                self.engine.process_key(key, Mode::ENGLISH);
            }
            return new_text;
        }
        if is_printable {
            self.engine.process_key(key, Mode::ENGLISH);
        }
        format!("{old_text}{key_s}")
    }

    fn update_preedit(&self, text: &str) -> Vec<Action> {
        let encoded = self.encode_text(text);
        let len = encoded.chars().count() as u32;
        if len == 0 {
            return vec![Action::HidePreedit, Action::HideAuxiliaryText];
        }
        vec![Action::UpdatePreedit {
            text: encoded,
            cursor_pos: len,
            visible: true,
            underline: !self.flags().contains(IBFlags::NO_UNDERLINE),
        }]
    }

    fn commit_and_reset(&mut self, s: &str) -> Vec<Action> {
        let mut actions = vec![
            Action::HidePreedit,
            Action::HideAuxiliaryText,
            Action::HideLookupTable,
        ];
        let committed = self.encode_text(s);
        if !committed.is_empty() {
            actions.push(Action::CommitText(committed));
        }
        self.engine.reset();
        actions
    }

    fn commit_and_reset_for_word_break(&mut self, s: &str, is_word_break: bool) -> Vec<Action> {
        let committed = self.encode_text(s);
        let mut actions = Vec::new();
        // Some clients (e.g. FB Messenger) need the commit before the preedit
        // is hidden, or the first word is lost.
        if self.flags().contains(IBFlags::WORKAROUND_FB_MESSENGER) || is_word_break {
            if !committed.is_empty() {
                actions.push(Action::CommitText(committed));
            }
            actions.push(Action::HidePreedit);
        } else {
            actions.push(Action::HidePreedit);
            if !committed.is_empty() {
                actions.push(Action::CommitText(committed));
            }
        }
        actions.push(Action::HideAuxiliaryText);
        actions.push(Action::HideLookupTable);
        self.engine.reset();
        actions
    }

    /// Re-read settings from disk so changes made in the preferences GUI take
    /// effect on the next focus, without restarting IBus. The input method is
    /// fixed by the engine name (not the file), so it is preserved; the composer
    /// is rebuilt only when the engine flags actually changed.
    fn reload_config(&mut self) {
        let mut fresh = Config::load();
        fresh.input_method = self.config.input_method.clone();
        if fresh.engine_flags != self.config.engine_flags
            && let Some(im) = parse_input_method(&fresh.input_method)
        {
            self.engine = BambusaEngine::new(im, fresh.engine_flags);
        }
        self.config = fresh;
        // Pick up macro edits made in the preferences GUI on the same focus.
        self.macros = build_macros(&self.config);
    }

    /// The property panel: a single "Preferences" button shown in the GNOME
    /// input menu. Activating it launches the setup GUI.
    fn setup_property(&self) -> Action {
        let prefs = IBusProperty::normal("setup", &gettext("Preferences"));
        Action::RegisterProperties(Box::new(IBusPropList::new(vec![prefs])))
    }

    /// Launch the preferences GUI, which lives next to this binary.
    fn spawn_setup(&self) {
        if let Ok(exe) = std::env::current_exe()
            && let Some(dir) = exe.parent()
            && let Ok(mut child) =
                std::process::Command::new(dir.join("ibus-setup-bambusa")).spawn()
        {
            // The GUI is single-instance, so a repeat launch hands off and exits
            // immediately; reap it in the background so it does not linger as a
            // zombie (and the long-lived primary is reaped when its window closes).
            std::thread::spawn(move || {
                let _ = child.wait();
            });
        }
    }

    /// Surrounding-text delivery: instead of showing preedit, commit the
    /// composition incrementally and rewrite it via DeleteSurroundingText as it
    /// changes — for clients that handle surrounding text but not preedit well.
    fn process_key_surrounding(
        &mut self,
        keyval: u32,
        keycode: u32,
        state: u32,
    ) -> (bool, Vec<Action>) {
        if keysyms::is_release(state) {
            return (false, Vec::new());
        }
        let raw_key_len = self.raw_key_len();
        let key = char::from_u32(keyval).unwrap_or('\0');
        // The text currently in the client equals the composition we last sent.
        let old_text = self.preedit_string();

        if !self.should_restore_key_strokes
            && !self.engine.can_process_key(key)
            && raw_key_len == 0
            && !self.flags().contains(IBFlags::MACRO_ENABLED)
        {
            return (false, Vec::new());
        }

        if keyval == keysyms::BACKSPACE {
            if raw_key_len > 0 {
                self.engine.remove_last_char(true);
                let new_text = self.preedit_string();
                return (true, self.update_previous_text(&old_text, &new_text));
            }
            // Nothing composed: let the client delete a real character.
            return (false, Vec::new());
        }

        if keyval == keysyms::TAB {
            self.engine.reset();
            return (false, Vec::new());
        }

        let (new_text, is_word_break) = self.commit_text(keyval, keycode, state);
        let is_printable = self.is_printable_key(state, keyval);
        let actions = self.update_previous_text(&old_text, &new_text);
        if is_word_break {
            self.engine.reset();
        }
        (is_printable, actions)
    }

    /// Rewrite the client text from `old` to `new` (both composed): delete the
    /// changed trailing characters and commit the new ones, in the charset.
    fn update_previous_text(&self, old: &str, new: &str) -> Vec<Action> {
        let old_enc = self.encode_text(old);
        let new_enc = self.encode_text(new);
        let (suffix, n_backspace) = offset_runes(&new_enc, &old_enc);
        let mut actions = Vec::new();
        if n_backspace > 0 {
            actions.push(self.send_backspace(n_backspace));
        }
        if !suffix.is_empty() {
            actions.push(Action::CommitText(suffix.to_string()));
        }
        actions
    }

    /// Delete `n` characters before the cursor for the active input mode.
    fn send_backspace(&self, n: u32) -> Action {
        Action::DeleteSurroundingText {
            offset: -(n as i32),
            nchars: n,
        }
    }
}

/// Common-prefix diff: the differing suffix of `new`, and how many trailing
/// characters of `old` to delete to reach it.
fn offset_runes<'a>(new: &'a str, old: &str) -> (&'a str, u32) {
    let new_chars: Vec<char> = new.chars().collect();
    let old_chars: Vec<char> = old.chars().collect();
    let min = new_chars.len().min(old_chars.len());
    let mut offset = 0;
    while offset < min && new_chars[offset] == old_chars[offset] {
        offset += 1;
    }
    let n_backspace = (old_chars.len() - offset) as u32;
    let byte_offset = new.char_indices().nth(offset).map_or(new.len(), |(i, _)| i);
    (&new[byte_offset..], n_backspace)
}

/// Build the macro table for a config: the configured pairs when macros are
/// enabled, otherwise empty (auto-capitalize is preserved either way).
fn build_macros(config: &Config) -> MacroTable {
    let auto_cap = config.ib_flags.contains(IBFlags::AUTO_CAPITALIZE_MACRO);
    if config.ib_flags.contains(IBFlags::MACRO_ENABLED) {
        MacroTable::from_entries(&config.macros, auto_cap)
    } else {
        MacroTable::empty(auto_cap)
    }
}

enum MacroCase {
    AllLower,
    AllUpper,
    Mixed,
}

/// The letter case of a macro key, to mirror it onto the expansion.
fn macro_case(key: &str) -> MacroCase {
    let letters: Vec<char> = key.chars().filter(|c| c.is_alphabetic()).collect();
    if letters.is_empty() {
        MacroCase::Mixed
    } else if letters.iter().all(|c| c.is_lowercase()) {
        MacroCase::AllLower
    } else if letters.iter().all(|c| c.is_uppercase()) {
        MacroCase::AllUpper
    } else {
        MacroCase::Mixed
    }
}

impl EngineHandler for PreeditHandler {
    fn process_key_event(&mut self, keyval: u32, keycode: u32, state: u32) -> (bool, Vec<Action>) {
        if self.config.input_mode == InputMode::SurroundingText {
            return self.process_key_surrounding(keyval, keycode, state);
        }
        if keysyms::is_release(state) {
            return (false, Vec::new());
        }
        let raw_key_len = self.raw_key_len();
        let key = char::from_u32(keyval).unwrap_or('\0');

        // Don't swallow special characters when there's nothing composed
        // (workaround for browser address bars and spreadsheets).
        if !self.should_restore_key_strokes
            && !self.engine.can_process_key(key)
            && raw_key_len == 0
            && !self.flags().contains(IBFlags::MACRO_ENABLED)
        {
            return (false, Vec::new());
        }

        if keyval == keysyms::BACKSPACE {
            if self.rune_count() == 1 {
                return (true, self.commit_and_reset(""));
            }
            if raw_key_len > 0 {
                self.engine.remove_last_char(true);
                let preedit = self.preedit_string();
                return (true, self.update_preedit(&preedit));
            }
            return (false, Vec::new());
        }

        if keyval == keysyms::TAB {
            if let Some(expansion) = self.macro_text() {
                return (true, self.commit_and_reset(&expansion));
            }
            let old_text = self.preedit_string();
            let composed = self.composed_string(&old_text);
            return (false, self.commit_and_reset(&composed));
        }

        let (new_text, is_word_break) = self.commit_text(keyval, keycode, state);
        let is_printable = self.is_printable_key(state, keyval);
        if is_word_break {
            return (
                is_printable,
                self.commit_and_reset_for_word_break(&new_text, is_printable),
            );
        }
        (is_printable, self.update_preedit(&new_text))
    }

    fn focus_in(&mut self) -> Vec<Action> {
        self.reload_config();
        let mut actions = vec![self.setup_property()];
        if self.config.input_mode == InputMode::SurroundingText {
            actions.push(Action::RequireSurroundingText);
        }
        actions.append(&mut self.reset());
        actions
    }

    fn focus_out(&mut self) -> Vec<Action> {
        self.reset()
    }

    fn reset(&mut self) -> Vec<Action> {
        self.engine.reset();
        vec![Action::HidePreedit, Action::HideAuxiliaryText]
    }

    fn property_activate(&mut self, name: String, _state: u32) -> Vec<Action> {
        if name == "setup" {
            self.spawn_setup();
        }
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn handler() -> PreeditHandler {
        PreeditHandler::new(Config::default())
    }

    /// Drive a string of ASCII keysyms and return the actions for the last key.
    fn type_keys(h: &mut PreeditHandler, keys: &str) -> Vec<Action> {
        let mut last = Vec::new();
        for c in keys.chars() {
            let (_, actions) = h.process_key_event(c as u32, 0, 0);
            last = actions;
        }
        last
    }

    fn preedit_of(actions: &[Action]) -> Option<&str> {
        actions.iter().find_map(|a| match a {
            Action::UpdatePreedit { text, .. } => Some(text.as_str()),
            _ => None,
        })
    }

    #[test]
    fn composes_vietnamese_in_preedit() {
        let mut h = handler();
        let actions = type_keys(&mut h, "tieesng");
        assert_eq!(preedit_of(&actions), Some("tiếng"));
    }

    #[test]
    fn commits_on_space() {
        let mut h = handler();
        type_keys(&mut h, "Vieejt");
        let (_, actions) = h.process_key_event(0x20, 0, 0); // space
        assert!(
            actions
                .iter()
                .any(|a| matches!(a, Action::CommitText(s) if s == "Việt "))
        );
    }

    #[test]
    fn backspace_edits_preedit() {
        let mut h = handler();
        type_keys(&mut h, "vieejt"); // việt
        let (_, actions) = h.process_key_event(keysyms::BACKSPACE, 0, 0);
        assert_eq!(preedit_of(&actions), Some("việ"));
    }

    #[test]
    fn non_vietnamese_falls_back_to_english() {
        // "catr" is invalid (hook tone can't sit on a final stop consonant),
        // so auto-restore shows the raw keystrokes instead of a bad syllable.
        let mut h = handler();
        let actions = type_keys(&mut h, "catr");
        assert_eq!(preedit_of(&actions), Some("catr"));
    }

    #[test]
    fn disabling_spell_check_keeps_the_composition() {
        // "tools" composes to the invalid "tôls"; with spell-check on it is
        // restored to the raw keystrokes, and with it off the composed form is
        // kept — so the SPELL_CHECK_ENABLED gate is observable here.
        let actions_on = type_keys(&mut handler(), "tools");
        assert_eq!(preedit_of(&actions_on), Some("tools"));

        let cfg = Config {
            ib_flags: IBFlags::STD.difference(IBFlags::SPELL_CHECK_ENABLED),
            ..Config::default()
        };
        let actions_off = type_keys(&mut PreeditHandler::new(cfg), "tools");
        assert_eq!(preedit_of(&actions_off), Some("tôls"));
    }

    #[test]
    fn macro_expands_on_word_break() {
        let cfg = Config {
            ib_flags: (IBFlags::STD | IBFlags::MACRO_ENABLED)
                .difference(IBFlags::AUTO_CAPITALIZE_MACRO),
            ..Config::default()
        };
        let mut h = PreeditHandler::new(cfg);
        h.macros = MacroTable::from_pairs(&[("vn", "Vietnam")], false);
        type_keys(&mut h, "vn");
        let (_, actions) = h.process_key_event(0x20, 0, 0); // space triggers expansion
        assert!(
            actions
                .iter()
                .any(|a| matches!(a, Action::CommitText(s) if s == "Vietnam "))
        );
    }

    #[test]
    fn macro_auto_capitalizes_to_match_the_key() {
        let cfg = Config {
            ib_flags: IBFlags::STD | IBFlags::MACRO_ENABLED, // STD has AUTO_CAPITALIZE_MACRO
            ..Config::default()
        };
        let mut h = PreeditHandler::new(cfg);
        h.macros = MacroTable::from_pairs(&[("vn", "Vietnam")], true);
        assert_eq!(h.expand_macro("vn"), "vietnam"); // all-lower key → lower
        assert_eq!(h.expand_macro("VN"), "VIETNAM"); // all-upper key → upper
        assert_eq!(h.expand_macro("Vn"), "Vietnam"); // mixed → unchanged
    }

    #[test]
    fn offset_runes_computes_diff() {
        assert_eq!(offset_runes("á", "a"), ("á", 1));
        assert_eq!(offset_runes("tiếng ", "tiếng"), (" ", 0));
        assert_eq!(offset_runes("", "a"), ("", 1));
        assert_eq!(offset_runes("abc", "abc"), ("", 0));
    }

    #[test]
    fn surrounding_text_rewrites_via_diff() {
        let cfg = Config {
            input_mode: InputMode::SurroundingText,
            ..Config::default()
        };
        let mut h = PreeditHandler::new(cfg);
        // Telex: 'a' commits "a"; 's' (acute) deletes it and commits "á".
        let (_, a1) = h.process_key_event('a' as u32, 0, 0);
        let (_, a2) = h.process_key_event('s' as u32, 0, 0);
        assert!(
            a1.iter()
                .any(|x| matches!(x, Action::CommitText(t) if t.as_str() == "a"))
        );
        assert!(
            a2.iter()
                .any(|x| matches!(x, Action::DeleteSurroundingText { nchars, .. } if *nchars == 1))
        );
        assert!(
            a2.iter()
                .any(|x| matches!(x, Action::CommitText(t) if t.as_str() == "á"))
        );
    }
}
