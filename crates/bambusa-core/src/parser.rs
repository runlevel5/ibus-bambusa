//! Parsing of the input-method rule DSL into [`Rule`] lists.

use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;

use crate::rules::{EffectType, Mark, Rule, Tone};
use crate::unicode_tables::{add_tone_to_char, find_mark_from_char, get_mark_family, is_vowel};

/// Named tone effects recognised in a definition line.
static TONES: LazyLock<HashMap<&'static str, Tone>> = LazyLock::new(|| {
    HashMap::from([
        ("XoáDấuThanh", Tone::None),
        ("DấuSắc", Tone::Acute),
        ("DấuHuyền", Tone::Grave),
        ("DấuNgã", Tone::Tilde),
        ("DấuNặng", Tone::Dot),
        ("DấuHỏi", Tone::Hook),
    ])
});

/// `effective-chars _ result-chars [appended]`, e.g. `uoa_ươă__ư`.
static REG_DSL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"([a-zA-Z]+)_(\p{L}+)([_\p{L}]*)").unwrap());

/// A bare appending clause, e.g. `_ă` or `__ư`.
static REG_DSL_APPENDING: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(_?)_(\p{L}+)").unwrap());

/// Parse one definition line keyed on `key` into its rules.
pub(crate) fn parse_rules(key: char, line: &str) -> Vec<Rule> {
    if let Some(&tone) = TONES.get(line) {
        vec![Rule {
            key,
            effect_type: EffectType::ToneTransformation,
            effect: tone as u8,
            ..Default::default()
        }]
    } else {
        parse_toneless_rules(key, line)
    }
}

fn parse_toneless_rules(key: char, line: &str) -> Vec<Rule> {
    let mut rules = Vec::new();
    let lower = line.to_lowercase();
    if let Some(caps) = REG_DSL.captures(&lower) {
        let effective_ons: Vec<char> = caps[1].chars().collect();
        let results: Vec<char> = caps[2].chars().collect();
        for (i, &effective_on) in effective_ons.iter().enumerate() {
            let Some(&result) = results.get(i) else {
                continue;
            };
            let Some(effect) = find_mark_from_char(result) else {
                continue;
            };
            rules.extend(parse_toneless_rule(key, effective_on, result, effect));
        }
        let appended = caps.get(3).map_or("", |m| m.as_str());
        if let Some(rule) = get_appending_rule(key, appended) {
            rules.push(rule);
        }
    } else if let Some(rule) = get_appending_rule(key, line) {
        rules.push(rule);
    }
    rules
}

fn parse_toneless_rule(key: char, effective_on: char, result: char, effect: Mark) -> Vec<Rule> {
    let mut rules = Vec::new();
    for chr in get_mark_family(effective_on) {
        if chr == result {
            // Re-pressing the key toggles the mark back off.
            rules.push(Rule {
                key,
                effect_type: EffectType::MarkTransformation,
                effect: 0,
                effect_on: result,
                result: effective_on,
                ..Default::default()
            });
        } else if is_vowel(chr) {
            // The mark must survive every tone, so emit one rule per tone.
            for tone in 0u8..6 {
                rules.push(Rule {
                    key,
                    effect_type: EffectType::MarkTransformation,
                    effect: effect as u8,
                    effect_on: add_tone_to_char(chr, tone),
                    result: add_tone_to_char(result, tone),
                    ..Default::default()
                });
            }
        } else {
            rules.push(Rule {
                key,
                effect_type: EffectType::MarkTransformation,
                effect: effect as u8,
                effect_on: chr,
                result,
                ..Default::default()
            });
        }
    }
    rules
}

fn get_appending_rule(key: char, value: &str) -> Option<Rule> {
    let caps = REG_DSL_APPENDING.captures(value)?;
    let chars: Vec<char> = caps[2].chars().collect();
    let mut rule = Rule {
        key,
        effect_type: EffectType::Appending,
        effect_on: chars[0],
        result: chars[0],
        ..Default::default()
    };
    for &chr in &chars[1..] {
        rule.appended_rules.push(Rule {
            key,
            effect_type: EffectType::Appending,
            effect_on: chr,
            result: chr,
            ..Default::default()
        });
    }
    Some(rule)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tone_line_yields_single_tone_rule() {
        let rules = parse_rules('s', "DấuSắc");
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].effect_type, EffectType::ToneTransformation);
        assert_eq!(rules[0].tone(), Tone::Acute);
        assert_eq!(rules[0].key, 's');
    }

    #[test]
    fn mark_line_maps_vowel_across_tones() {
        // Telex "a": A_Â -> 'a' adds hat, including a toggle-off rule.
        let rules = parse_rules('a', "A_Â");
        assert!(
            rules
                .iter()
                .all(|r| r.effect_type == EffectType::MarkTransformation)
        );
        // a -> â among the generated mappings.
        assert!(
            rules
                .iter()
                .any(|r| r.effect_on == 'a' && r.result == 'â' && r.mark() == Mark::Hat)
        );
        // toggle: â -> a with no mark.
        assert!(
            rules
                .iter()
                .any(|r| r.effect_on == 'â' && r.result == 'a' && r.effect == 0)
        );
    }

    #[test]
    fn appending_clause_preserves_case() {
        // Microsoft "!": "_Ă" appends a capital Ă (else-branch keeps case).
        let rules = parse_rules('!', "_Ă");
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].effect_type, EffectType::Appending);
        assert_eq!(rules[0].result, 'Ă');
        // "__ă" stays lowercase.
        let lower = parse_rules('1', "__ă");
        assert_eq!(lower[0].result, 'ă');
    }

    #[test]
    fn trailing_append_clause_is_captured() {
        // Telex W "w": UOA_ƯƠĂ__Ư -> mark rules plus an appending 'ư'.
        let rules = parse_rules('w', "UOA_ƯƠĂ__Ư");
        assert!(
            rules
                .iter()
                .any(|r| r.effect_type == EffectType::Appending && r.result == 'ư')
        );
        assert!(
            rules
                .iter()
                .any(|r| r.effect_on == 'u' && r.result == 'ư' && r.mark() == Mark::Horn)
        );
    }

    #[test]
    fn tone_rules() {
        let rules = parse_rules('z', "XoáDấuThanh");
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].effect_type, EffectType::ToneTransformation);
        assert_eq!(rules[0].tone(), Tone::None);

        let rules = parse_rules('x', "DấuNgã");
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].effect_type, EffectType::ToneTransformation);
        assert_eq!(rules[0].tone(), Tone::Tilde);
    }

    #[test]
    fn toneless_dd_and_append() {
        let rules = parse_toneless_rules('d', "D_Đ");
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].effect_type, EffectType::MarkTransformation);
        assert_eq!(rules[0].mark(), Mark::Dash);
        assert_eq!(rules[0].effect_on, 'd');

        let rules = parse_toneless_rules('{', "_Ư");
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].effect_type, EffectType::Appending);
        assert_eq!(rules[0].effect_on, 'Ư');
    }

    #[test]
    fn uoa_rule_layout() {
        let rules = parse_toneless_rules('w', "UOA_ƯƠĂ");
        assert_eq!(rules.len(), 33);
        assert_eq!(rules[0].mark(), Mark::Horn);
        assert_eq!(rules[0].effect_on, 'u');
        assert_eq!(rules[7].mark(), Mark::Horn);
        assert_eq!(rules[7].effect_on, 'o');
        assert_eq!(rules[20].mark(), Mark::Breve);
        assert_eq!(rules[20].effect_on, 'a');

        let rules = parse_toneless_rules('w', "UOA_ƯƠĂ__Ư");
        assert_eq!(rules.len(), 34);
        assert_eq!(rules[20].mark(), Mark::Breve);
        assert_eq!(rules[20].effect_on, 'a');
        assert_eq!(rules[33].effect_type, EffectType::Appending);
        assert_eq!(rules[33].effect_on, 'ư');
    }

    #[test]
    fn nested_appended_rules() {
        let rules = parse_toneless_rules('[', "__ươ");
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].appended_rules.len(), 1);
        assert_eq!(
            rules[0].appended_rules[0].effect_type,
            EffectType::Appending
        );
        assert_eq!(rules[0].appended_rules[0].effect_on, 'ơ');

        let rules = parse_toneless_rules('{', "__ƯƠ");
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].appended_rules.len(), 1);
        assert_eq!(rules[0].appended_rules[0].effect_on, 'Ơ');
    }
}
