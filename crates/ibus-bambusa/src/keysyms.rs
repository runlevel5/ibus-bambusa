//! IBus keysym values and modifier-mask bits, plus key-state helpers.

// Modifier-mask bits.
pub const LOCK_MASK: u32 = 1 << 1;
pub const CONTROL_MASK: u32 = 1 << 2;
pub const MOD1_MASK: u32 = 1 << 3;
pub const MOD4_MASK: u32 = 1 << 6;
pub const FORWARD_MASK: u32 = 1 << 25;
pub const IGNORED_MASK: u32 = FORWARD_MASK;
pub const SUPER_MASK: u32 = 1 << 26;
pub const HYPER_MASK: u32 = 1 << 27;
pub const META_MASK: u32 = 1 << 28;
pub const RELEASE_MASK: u32 = 1 << 30;

// Keysyms we react to.
pub const BACKSPACE: u32 = 0xff08;
pub const TAB: u32 = 0xff09;

/// Modifiers that mean the key is a shortcut/command, not text input.
const SUPPRESSING: u32 =
    CONTROL_MASK | MOD1_MASK | MOD4_MASK | IGNORED_MASK | SUPER_MASK | HYPER_MASK | META_MASK;

/// Whether the modifier state allows the key to produce text.
pub fn is_valid_state(state: u32) -> bool {
    state & SUPPRESSING == 0
}

/// Whether this is a key-release event (which engines ignore).
pub fn is_release(state: u32) -> bool {
    state & RELEASE_MASK != 0
}
