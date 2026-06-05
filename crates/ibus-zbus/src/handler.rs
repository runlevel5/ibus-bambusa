//! The engine behaviour contract.
//!
//! IBus method calls are turned into [`EngineHandler`] calls; the handler runs
//! synchronous engine logic and returns the [`Action`]s to emit back to IBus.
//! Keeping emission as returned data (rather than callbacks) lets the engine
//! logic stay synchronous and unit-testable, and lets the binding emit signals
//! in a single ordered place.

use crate::types::{IBusLookupTable, IBusPropList, IBusProperty};

/// Something the engine wants the binding to emit to IBus.
#[derive(Debug, Clone)]
pub enum Action {
    /// Commit finalized text to the client.
    CommitText(String),
    /// Show/replace the in-progress preedit text.
    UpdatePreedit {
        text: String,
        cursor_pos: u32,
        visible: bool,
        /// Whether to draw the single underline beneath the preedit.
        underline: bool,
    },
    /// Hide the preedit text.
    HidePreedit,
    /// Forward a raw key event back to the client.
    ForwardKeyEvent {
        keyval: u32,
        keycode: u32,
        state: u32,
    },
    /// Delete text around the cursor via the surrounding-text protocol.
    DeleteSurroundingText { offset: i32, nchars: u32 },
    /// Show/replace the auxiliary (status) text.
    UpdateAuxiliaryText { text: String, visible: bool },
    /// Hide the auxiliary text.
    HideAuxiliaryText,
    /// Show/replace the candidate lookup table.
    UpdateLookupTable {
        table: Box<IBusLookupTable>,
        visible: bool,
    },
    /// Hide the lookup table.
    HideLookupTable,
    /// Register the property-panel tree.
    RegisterProperties(Box<IBusPropList>),
    /// Update a single property.
    UpdateProperty(Box<IBusProperty>),
    /// Ask the client to send surrounding text.
    RequireSurroundingText,
}

/// The behaviour behind an IBus engine instance.
///
/// Every method returns the [`Action`]s to emit. `process_key_event` also
/// returns whether it handled the key (the IBus `ProcessKeyEvent` return).
pub trait EngineHandler: Send {
    fn process_key_event(&mut self, keyval: u32, keycode: u32, state: u32) -> (bool, Vec<Action>);

    fn focus_in(&mut self) -> Vec<Action> {
        Vec::new()
    }
    fn focus_out(&mut self) -> Vec<Action> {
        Vec::new()
    }
    fn reset(&mut self) -> Vec<Action> {
        Vec::new()
    }
    fn enable(&mut self) -> Vec<Action> {
        Vec::new()
    }
    fn disable(&mut self) -> Vec<Action> {
        Vec::new()
    }
    fn set_surrounding_text(
        &mut self,
        text: String,
        cursor_pos: u32,
        anchor_pos: u32,
    ) -> Vec<Action> {
        let _ = (text, cursor_pos, anchor_pos);
        Vec::new()
    }
    fn property_activate(&mut self, name: String, state: u32) -> Vec<Action> {
        let _ = (name, state);
        Vec::new()
    }
    fn candidate_clicked(&mut self, index: u32, button: u32, state: u32) -> Vec<Action> {
        let _ = (index, button, state);
        Vec::new()
    }
    fn page_up(&mut self) -> Vec<Action> {
        Vec::new()
    }
    fn page_down(&mut self) -> Vec<Action> {
        Vec::new()
    }
    fn cursor_up(&mut self) -> Vec<Action> {
        Vec::new()
    }
    fn cursor_down(&mut self) -> Vec<Action> {
        Vec::new()
    }
}

/// A trivial handler that commits the typed ASCII character — used to prove the
/// binding end to end before the real engine is wired in.
#[derive(Debug, Default)]
pub struct EchoHandler;

impl EngineHandler for EchoHandler {
    fn process_key_event(
        &mut self,
        keyval: u32,
        _keycode: u32,
        _state: u32,
    ) -> (bool, Vec<Action>) {
        match char::from_u32(keyval).filter(char::is_ascii_graphic) {
            Some(c) => (true, vec![Action::CommitText(c.to_string())]),
            None => (false, Vec::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn echo_commits_typed_char() {
        let mut h = EchoHandler;
        let (handled, actions) = h.process_key_event(0x61, 0, 0); // keysym 'a'
        assert!(handled);
        assert!(matches!(&actions[..], [Action::CommitText(s)] if s == "a"));
    }

    #[test]
    fn echo_ignores_non_graphic() {
        let mut h = EchoHandler;
        let (handled, actions) = h.process_key_event(0xff08, 0, 0); // BackSpace keysym
        assert!(!handled);
        assert!(actions.is_empty());
    }

    #[test]
    fn default_methods_are_noops() {
        let mut h = EchoHandler;
        assert!(h.focus_in().is_empty());
        assert!(h.reset().is_empty());
        assert!(h.set_surrounding_text("x".into(), 0, 0).is_empty());
    }
}
