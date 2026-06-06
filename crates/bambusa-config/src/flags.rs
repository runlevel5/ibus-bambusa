//! Engine configuration flags: the input-mode selector and the feature bits.

use bitflags::bitflags;

/// How committed text reaches the client. Values are kept stable so existing
/// config files keep working; mode 6 is no longer supported and is remapped to
/// `BackspaceForwarding` at config load.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Preedit = 1,
    SurroundingText = 2,
    BackspaceForwarding = 3,
    ShiftLeftForwarding = 4,
    ForwardAsCommit = 5,
    Us = 7,
}

impl InputMode {
    /// Map a stored integer to a mode, remapping the no-longer-supported mode 6.
    pub fn from_stored(value: i32) -> Self {
        match value {
            2 => InputMode::SurroundingText,
            3 => InputMode::BackspaceForwarding,
            4 => InputMode::ShiftLeftForwarding,
            5 => InputMode::ForwardAsCommit,
            6 => InputMode::BackspaceForwarding, // mode 6 no longer supported
            7 => InputMode::Us,
            _ => InputMode::Preedit,
        }
    }
}

bitflags! {
    /// Feature flags. Bit positions match the original engine so stored configs
    /// decode unchanged (deprecated bits are left as gaps).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct IBFlags: u32 {
        const AUTO_COMMIT_VN_NOT_MATCH = 1 << 0;
        const MACRO_ENABLED = 1 << 1;
        const SPELL_CHECK_ENABLED = 1 << 4;
        const AUTO_NON_VN_RESTORE = 1 << 5;
        const DD_FREE_STYLE = 1 << 6;
        const NO_UNDERLINE = 1 << 7;
        const SPELL_CHECK_WITH_RULES = 1 << 8;
        const SPELL_CHECK_WITH_DICTS = 1 << 9;
        const AUTO_COMMIT_WITH_DELAY = 1 << 10;
        const PREEDIT_ELIMINATION = 1 << 13;
        const AUTO_CAPITALIZE_MACRO = 1 << 15;
        const WORKAROUND_FB_MESSENGER = 1 << 19;
        const WORKAROUND_WPS = 1 << 20;
    }
}

impl IBFlags {
    /// The default feature set (`IBstdFlags`).
    pub const STD: Self = Self::SPELL_CHECK_ENABLED
        .union(Self::SPELL_CHECK_WITH_RULES)
        .union(Self::AUTO_NON_VN_RESTORE)
        .union(Self::DD_FREE_STYLE)
        .union(Self::AUTO_CAPITALIZE_MACRO)
        .union(Self::NO_UNDERLINE)
        .union(Self::WORKAROUND_WPS);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xtest_mode_is_remapped() {
        assert_eq!(InputMode::from_stored(6), InputMode::BackspaceForwarding);
        assert_eq!(InputMode::from_stored(1), InputMode::Preedit);
        assert_eq!(InputMode::from_stored(99), InputMode::Preedit);
    }

    #[test]
    fn std_flags_bits() {
        // Same numeric value the original engine's IBstdFlags resolves to.
        let expected = (1 << 4) | (1 << 8) | (1 << 5) | (1 << 6) | (1 << 15) | (1 << 7) | (1 << 20);
        assert_eq!(IBFlags::STD.bits(), expected);
    }
}
