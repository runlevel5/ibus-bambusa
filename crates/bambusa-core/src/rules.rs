//! Transformation rules and their effect classification.

/// What a rule does to the composition it is applied to.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum EffectType {
    /// Append a character.
    #[default]
    Appending,
    /// Add a diacritical mark to a character.
    MarkTransformation,
    /// Add a tone to a character.
    ToneTransformation,
    /// Replace a character outright.
    Replacing,
}

/// A diacritical mark.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Mark {
    #[default]
    None,
    Hat,
    Breve,
    Horn,
    Dash,
    Raw,
}

/// A tone marking.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Tone {
    #[default]
    None,
    Grave,
    Acute,
    Hook,
    Tilde,
    Dot,
}

/// Raised when a byte does not correspond to a known enum value.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct OutOfRange(pub u8);

impl TryFrom<u8> for Mark {
    type Error = OutOfRange;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Mark::None),
            1 => Ok(Mark::Hat),
            2 => Ok(Mark::Breve),
            3 => Ok(Mark::Horn),
            4 => Ok(Mark::Dash),
            5 => Ok(Mark::Raw),
            other => Err(OutOfRange(other)),
        }
    }
}

impl TryFrom<u8> for Tone {
    type Error = OutOfRange;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Tone::None),
            1 => Ok(Tone::Grave),
            2 => Ok(Tone::Acute),
            3 => Ok(Tone::Hook),
            4 => Ok(Tone::Tilde),
            5 => Ok(Tone::Dot),
            other => Err(OutOfRange(other)),
        }
    }
}

/// A single transformation rule keyed on an input character.
///
/// `effect` holds either a [`Tone`] or a [`Mark`] as a raw byte, interpreted
/// according to `effect_type`.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Rule {
    pub key: char,
    pub effect: u8,
    pub effect_type: EffectType,
    pub effect_on: char,
    pub result: char,
    pub appended_rules: Vec<Rule>,
}

impl Rule {
    pub fn set_tone(&mut self, tone: Tone) {
        self.effect = tone as u8;
    }

    pub fn set_mark(&mut self, mark: Mark) {
        self.effect = mark as u8;
    }

    pub fn tone(&self) -> Tone {
        Tone::try_from(self.effect).unwrap_or_default()
    }

    pub fn mark(&self) -> Mark {
        Mark::try_from(self.effect).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enum_discriminants() {
        assert_eq!(EffectType::Appending as u8, 0);
        assert_eq!(EffectType::MarkTransformation as u8, 1);
        assert_eq!(EffectType::ToneTransformation as u8, 2);
        assert_eq!(EffectType::Replacing as u8, 3);

        assert_eq!(Mark::None as u8, 0);
        assert_eq!(Mark::Raw as u8, 5);
        assert_eq!(Tone::None as u8, 0);
        assert_eq!(Tone::Dot as u8, 5);
    }

    #[test]
    fn tone_mark_roundtrip() {
        for v in 0u8..=5 {
            assert_eq!(Tone::try_from(v).unwrap() as u8, v);
            assert_eq!(Mark::try_from(v).unwrap() as u8, v);
        }
        assert_eq!(Tone::try_from(6), Err(OutOfRange(6)));
        assert_eq!(Mark::try_from(6), Err(OutOfRange(6)));
    }

    #[test]
    fn rule_effect_accessors() {
        let mut r = Rule::default();
        r.set_tone(Tone::Acute);
        assert_eq!(r.tone(), Tone::Acute);
        r.set_mark(Mark::Horn);
        assert_eq!(r.mark(), Mark::Horn);
    }
}
