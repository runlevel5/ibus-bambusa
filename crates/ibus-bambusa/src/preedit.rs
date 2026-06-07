//! Preedit-mode engine handler: the default mode, where in-progress text is
//! shown as IBus preedit and committed on a word break.

use bambusa_core::{
    BambusaEngine, Mode, encode, has_any_vietnamese_rune, has_any_vietnamese_vowel,
    is_word_break_symbol, parse_input_method,
};
use gettextrs::gettext;
use ibus_zbus::{Action, EngineHandler, IBusPropList, IBusProperty};

use bambusa_config::{Config, IBFlags};

use crate::keysyms;

/// Composes Vietnamese text and renders it through IBus preedit.
pub struct PreeditHandler {
    engine: BambusaEngine,
    config: Config,
    should_restore_key_strokes: bool,
}

impl PreeditHandler {
    pub fn new(config: Config) -> Self {
        let im = parse_input_method(&config.input_method)
            .or_else(|| parse_input_method("Telex"))
            .expect("Telex is a built-in input method");
        let engine = BambusaEngine::new(im, config.engine_flags);
        Self {
            engine,
            config,
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
        if !self.flags().contains(IBFlags::AUTO_NON_VN_RESTORE) {
            return false;
        }
        let vn_seq = self.processed(Mode::VIETNAMESE | Mode::LOWERCASE);
        if vn_seq.is_empty() {
            return false;
        }
        // Allow "dd" even outside a Vietnamese word — it's common in abbreviations.
        if self.flags().contains(IBFlags::DD_FREE_STYLE)
            && !has_any_vietnamese_vowel(&vn_seq)
            && (vn_seq.ends_with('d') || vn_seq.contains('đ'))
        {
            return false;
        }
        if check_vn_rune && !has_any_vietnamese_rune(&vn_seq) {
            return false;
        }
        !self.engine.is_valid(false)
    }

    fn must_fallback_to_english(&self) -> bool {
        if !self.flags().contains(IBFlags::AUTO_NON_VN_RESTORE) {
            return false;
        }
        let vn_seq = self.processed(Mode::VIETNAMESE | Mode::LOWERCASE);
        if vn_seq.is_empty() {
            return false;
        }
        if self.flags().contains(IBFlags::DD_FREE_STYLE) && vn_seq.contains('đ') {
            return false;
        }
        // Dictionary spell-check is not wired yet; fall back to rule validation.
        !self.engine.is_valid(true)
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

        (self.handle_non_vn_word(keyval, keycode, state), true)
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
}

impl EngineHandler for PreeditHandler {
    fn process_key_event(&mut self, keyval: u32, keycode: u32, state: u32) -> (bool, Vec<Action>) {
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
}
