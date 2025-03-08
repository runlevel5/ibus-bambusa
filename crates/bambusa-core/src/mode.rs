use bitflags::bitflags;

bitflags! {
    /// Output mode flags controlling how a composition is flattened to text.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Mode: u32 {
        const VIETNAMESE = 1 << 0;
        const ENGLISH = 1 << 1;
        const TONELESS = 1 << 2;
        const MARKLESS = 1 << 3;
        const LOWERCASE = 1 << 4;
        const FULL_TEXT = 1 << 5;
        const PUNCTUATION = 1 << 6;
        const IN_REVERSE_ORDER = 1 << 7;
    }
}

bitflags! {
    /// Engine behaviour flags (tone marking and auto-correction).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct EngineFlags: u32 {
        const FREE_TONE_MARKING = 1 << 0;
        const STD_TONE_STYLE = 1 << 1;
        const AUTO_CORRECT = 1 << 2;
    }
}

impl EngineFlags {
    /// The default flag set: free tone marking, standard tone style and
    /// auto-correction all enabled.
    pub const STD: Self = Self::FREE_TONE_MARKING
        .union(Self::STD_TONE_STYLE)
        .union(Self::AUTO_CORRECT);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_bit_values() {
        assert_eq!(Mode::VIETNAMESE.bits(), 1);
        assert_eq!(Mode::ENGLISH.bits(), 2);
        assert_eq!(Mode::TONELESS.bits(), 4);
        assert_eq!(Mode::MARKLESS.bits(), 8);
        assert_eq!(Mode::LOWERCASE.bits(), 16);
        assert_eq!(Mode::FULL_TEXT.bits(), 32);
        assert_eq!(Mode::PUNCTUATION.bits(), 64);
        assert_eq!(Mode::IN_REVERSE_ORDER.bits(), 128);
    }

    #[test]
    fn engine_flag_values() {
        assert_eq!(EngineFlags::FREE_TONE_MARKING.bits(), 1);
        assert_eq!(EngineFlags::STD_TONE_STYLE.bits(), 2);
        assert_eq!(EngineFlags::AUTO_CORRECT.bits(), 4);
        assert_eq!(EngineFlags::STD.bits(), 7);
    }
}
