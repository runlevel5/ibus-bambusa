//! Reconstructing displayable text from a composition.

#![allow(dead_code)]

use std::collections::HashMap;

use crate::mode::Mode;
use crate::rules::{EffectType, Mark};
use crate::transform::{TransId, Transformation};
use crate::unicode_tables::{add_mark_to_char, add_tone_to_char, to_lower, to_upper};

/// Flatten a composition into its textual form under `mode`.
pub(crate) fn flatten(composition: &[Transformation], mode: Mode) -> String {
    canvas(composition, mode).into_iter().collect()
}

fn canvas(composition: &[Transformation], mode: Mode) -> Vec<char> {
    let english = mode.contains(Mode::ENGLISH);

    // Appending transformations carry the output characters; every other
    // transformation contributes an effect to the appender it targets.
    let mut appending: Vec<usize> = Vec::new();
    let mut effects: HashMap<TransId, Vec<usize>> = HashMap::new();
    for (i, trans) in composition.iter().enumerate() {
        if english || trans.rule.effect_type == EffectType::Appending {
            if trans.rule.key == '\0' {
                continue; // virtual key
            }
            appending.push(i);
        } else if let Some(target) = trans.target {
            effects.entry(target).or_default().push(i);
        }
    }

    let mut out = Vec::with_capacity(appending.len());
    for &ai in &appending {
        let app = &composition[ai];
        let mut chr = if english {
            app.rule.key
        } else {
            let mut chr = app.rule.effect_on;
            for &ei in effects.get(&app.id).into_iter().flatten() {
                let effect = &composition[ei].rule;
                match effect.effect_type {
                    EffectType::MarkTransformation => {
                        if effect.effect == Mark::Raw as u8 {
                            chr = app.rule.key;
                        } else {
                            chr = add_mark_to_char(chr, effect.effect);
                        }
                    }
                    EffectType::ToneTransformation => {
                        chr = add_tone_to_char(chr, effect.effect);
                    }
                    _ => {}
                }
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
        out.push(chr);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::{EffectType, Rule, Tone};

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
