//! Reconstructing displayable text from a composition.

use crate::mode::Mode;
use crate::rules::{EffectType, Mark};
use crate::transform::Transformation;
use crate::unicode_tables::{add_mark_to_char, add_tone_to_char, to_lower, to_upper};

/// Whether `trans` contributes a visible character under `english`.
///
/// Appending transformations carry the output characters; in English mode every
/// non-virtual transformation is emitted as its raw key. Virtual transformations
/// (`key == '\0'`) never produce a character.
#[inline]
fn is_emitted(trans: &Transformation, english: bool) -> bool {
    (english || trans.rule.effect_type == EffectType::Appending) && trans.rule.key != '\0'
}

/// Fold a single effect transformation into `chr` if it targets `app`.
#[inline]
fn apply_effect(chr: char, app: &Transformation, effect_trans: &Transformation) -> char {
    if effect_trans.rule.effect_type == EffectType::Appending || effect_trans.target != Some(app.id)
    {
        return chr;
    }
    let effect = &effect_trans.rule;
    match effect.effect_type {
        EffectType::MarkTransformation => {
            if effect.effect == Mark::Raw as u8 {
                app.rule.key
            } else {
                add_mark_to_char(chr, effect.effect)
            }
        }
        EffectType::ToneTransformation => add_tone_to_char(chr, effect.effect),
        _ => chr,
    }
}

/// Resolve the final character emitted by appender `app`, applying every effect
/// in `composition` (plus an optional `extra` appended transformation) that
/// targets it, in order, then the `mode` post-processing (toneless/markless/case).
fn resolve_char(
    composition: &[Transformation],
    extra: Option<&Transformation>,
    app: &Transformation,
    mode: Mode,
) -> char {
    let english = mode.contains(Mode::ENGLISH);
    let mut chr = if english {
        app.rule.key
    } else {
        let mut chr = app.rule.effect_on;
        for effect_trans in composition.iter().chain(extra) {
            chr = apply_effect(chr, app, effect_trans);
        }
        chr
    };

    if mode.contains(Mode::TONELESS) {
        chr = add_tone_to_char(chr, 0);
    }
    if mode.contains(Mode::MARKLESS) {
        chr = add_mark_to_char(chr, 0);
    }
    if mode.contains(Mode::LOWERCASE) {
        chr = to_lower(chr);
    } else if app.is_upper_case {
        chr = to_upper(chr);
    }
    chr
}

/// Flatten a composition into its textual form under `mode`.
pub(crate) fn flatten(composition: &[Transformation], mode: Mode) -> String {
    let mut out = String::with_capacity(composition.len());
    flatten_into(composition, mode, &mut out);
    out
}

/// Flatten into a caller-owned buffer (cleared first), so a hot path can reuse
/// one allocation across many flattens instead of allocating a fresh `String`.
pub(crate) fn flatten_into(composition: &[Transformation], mode: Mode, out: &mut String) {
    out.clear();
    let english = mode.contains(Mode::ENGLISH);
    for trans in composition {
        if is_emitted(trans, english) {
            out.push(resolve_char(composition, None, trans, mode));
        }
    }
}

/// Flatten `composition` with one `extra` transformation appended, without
/// cloning the composition. Equivalent to building `[composition, extra]` and
/// flattening it; used by the probe-and-compare paths in `transform`.
pub(crate) fn flatten_with_extra(
    composition: &[Transformation],
    extra: &Transformation,
    mode: Mode,
) -> String {
    let english = mode.contains(Mode::ENGLISH);
    let mut out = String::with_capacity(composition.len() + 1);
    for trans in composition.iter().chain(std::iter::once(extra)) {
        if is_emitted(trans, english) {
            out.push(resolve_char(composition, Some(extra), trans, mode));
        }
    }
    out
}

/// The first character [`flatten`] would emit, without allocating. Used by the
/// word-boundary scans that only need to inspect the leading character.
pub(crate) fn first_char(composition: &[Transformation], mode: Mode) -> Option<char> {
    let english = mode.contains(Mode::ENGLISH);
    composition
        .iter()
        .find(|trans| is_emitted(trans, english))
        .map(|trans| resolve_char(composition, None, trans, mode))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::{EffectType, Rule, Tone};
    use crate::transform::TransId;

    fn appending(id: TransId, key: char, upper: bool) -> Transformation {
        Transformation {
            id,
            rule: Rule {
                key,
                effect_type: EffectType::Appending,
                effect_on: key,
                result: key,
                ..Default::default()
            },
            target: None,
            is_upper_case: upper,
        }
    }

    fn tone(id: TransId, key: char, tone: Tone, target: TransId) -> Transformation {
        Transformation {
            id,
            rule: Rule {
                key,
                effect_type: EffectType::ToneTransformation,
                effect: tone as u8,
                ..Default::default()
            },
            target: Some(target),
            is_upper_case: false,
        }
    }

    #[test]
    fn applies_tone_to_targeted_appender() {
        let comp = vec![appending(0, 'a', false), tone(1, 's', Tone::Acute, 0)];
        assert_eq!(flatten(&comp, Mode::VIETNAMESE), "á");
    }

    #[test]
    fn english_mode_emits_raw_keys() {
        let comp = vec![appending(0, 'a', false), tone(1, 's', Tone::Acute, 0)];
        assert_eq!(flatten(&comp, Mode::ENGLISH), "as");
    }

    #[test]
    fn uppercase_flag_capitalises() {
        let comp = vec![appending(0, 'a', true), tone(1, 's', Tone::Acute, 0)];
        assert_eq!(flatten(&comp, Mode::VIETNAMESE), "Á");
    }

    #[test]
    fn toneless_strips_tone() {
        let comp = vec![appending(0, 'a', false), tone(1, 's', Tone::Acute, 0)];
        assert_eq!(flatten(&comp, Mode::VIETNAMESE | Mode::TONELESS), "a");
    }
}
