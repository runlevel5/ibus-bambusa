//! The built-in input-method layouts and the builder that compiles a
//! definition into an [`InputMethod`].

use crate::parser::parse_rules;
use crate::rules::{EffectType, Rule};

/// A compiled input method: its rules plus the key classifications the engine
/// needs at runtime.
#[derive(Clone, Debug, Default)]
pub struct InputMethod {
    pub name: String,
    pub rules: Vec<Rule>,
    /// Keys whose definition involves the `uo` cluster (drive the `uow`
    /// shortcut).
    pub super_keys: Vec<char>,
    /// Keys that apply a tone.
    pub tone_keys: Vec<char>,
    /// Keys that append a character.
    pub appending_keys: Vec<char>,
    /// Every key the method binds.
    pub keys: Vec<char>,
}

type Definition = (&'static str, &'static [(&'static str, &'static str)]);

/// The built-in layouts, in declaration order.
const DEFINITIONS: &[Definition] = &[
    (
        "Telex",
        &[
            ("z", "XoáDấuThanh"),
            ("s", "DấuSắc"),
            ("f", "DấuHuyền"),
            ("r", "DấuHỏi"),
            ("x", "DấuNgã"),
            ("j", "DấuNặng"),
            ("a", "A_Â"),
            ("e", "E_Ê"),
            ("o", "O_Ô"),
            ("w", "UOA_ƯƠĂ"),
            ("d", "D_Đ"),
        ],
    ),
    (
        "VNI",
        &[
            ("0", "XoáDấuThanh"),
            ("1", "DấuSắc"),
            ("2", "DấuHuyền"),
            ("3", "DấuHỏi"),
            ("4", "DấuNgã"),
            ("5", "DấuNặng"),
            ("6", "AEO_ÂÊÔ"),
            ("7", "UO_ƯƠ"),
            ("8", "A_Ă"),
            ("9", "D_Đ"),
        ],
    ),
    (
        "VIQR",
        &[
            ("0", "XoáDấuThanh"),
            ("'", "DấuSắc"),
            ("`", "DấuHuyền"),
            ("?", "DấuHỏi"),
            ("~", "DấuNgã"),
            (".", "DấuNặng"),
            ("^", "AEO_ÂÊÔ"),
            ("+", "UO_ƯƠ"),
            ("*", "UO_ƯƠ"),
            ("(", "A_Ă"),
            ("d", "D_Đ"),
        ],
    ),
    (
        "Microsoft layout",
        &[
            ("8", "DấuSắc"),
            ("5", "DấuHuyền"),
            ("6", "DấuHỏi"),
            ("7", "DấuNgã"),
            ("9", "DấuNặng"),
            ("1", "__ă"),
            ("!", "_Ă"),
            ("2", "__â"),
            ("@", "_Â"),
            ("3", "__ê"),
            ("#", "_Ê"),
            ("4", "__ô"),
            ("$", "_Ô"),
            ("0", "__đ"),
            (")", "_Đ"),
            ("[", "__ư"),
            ("{", "_Ư"),
            ("]", "__ơ"),
            ("}", "_Ơ"),
        ],
    ),
    (
        "Telex 2",
        &[
            ("z", "XoáDấuThanh"),
            ("s", "DấuSắc"),
            ("f", "DấuHuyền"),
            ("r", "DấuHỏi"),
            ("x", "DấuNgã"),
            ("j", "DấuNặng"),
            ("a", "A_Â"),
            ("e", "E_Ê"),
            ("o", "O_Ô"),
            ("w", "UOA_ƯƠĂ__Ư"),
            ("d", "D_Đ"),
            ("]", "__ư"),
            ("[", "__ơ"),
            ("}", "_Ư"),
            ("{", "_Ơ"),
        ],
    ),
    (
        "Telex + VNI",
        &[
            ("z", "XoáDấuThanh"),
            ("s", "DấuSắc"),
            ("f", "DấuHuyền"),
            ("r", "DấuHỏi"),
            ("x", "DấuNgã"),
            ("j", "DấuNặng"),
            ("a", "A_Â"),
            ("e", "E_Ê"),
            ("o", "O_Ô"),
            ("w", "UOA_ƯƠĂ"),
            ("d", "D_Đ"),
            ("0", "XoáDấuThanh"),
            ("1", "DấuSắc"),
            ("2", "DấuHuyền"),
            ("3", "DấuHỏi"),
            ("4", "DấuNgã"),
            ("5", "DấuNặng"),
            ("6", "AEO_ÂÊÔ"),
            ("7", "UO_ƯƠ"),
            ("8", "A_Ă"),
            ("9", "D_Đ"),
        ],
    ),
    (
        "Telex + VNI + VIQR",
        &[
            ("z", "XoáDấuThanh"),
            ("s", "DấuSắc"),
            ("f", "DấuHuyền"),
            ("r", "DấuHỏi"),
            ("x", "DấuNgã"),
            ("j", "DấuNặng"),
            ("a", "A_Â"),
            ("e", "E_Ê"),
            ("o", "O_Ô"),
            ("w", "UOA_ƯƠĂ"),
            ("d", "D_Đ"),
            ("0", "XoáDấuThanh"),
            ("1", "DấuSắc"),
            ("2", "DấuHuyền"),
            ("3", "DấuHỏi"),
            ("4", "DấuNgã"),
            ("5", "DấuNặng"),
            ("6", "AEO_ÂÊÔ"),
            ("7", "UO_ƯƠ"),
            ("8", "A_Ă"),
            ("9", "D_Đ"),
            ("'", "DấuSắc"),
            ("`", "DấuHuyền"),
            ("?", "DấuHỏi"),
            ("~", "DấuNgã"),
            (".", "DấuNặng"),
            ("^", "AEO_ÂÊÔ"),
            ("+", "UO_ƯƠ"),
            ("*", "UO_ƯƠ"),
            ("(", "A_Ă"),
            ("\\", "D_Đ"),
        ],
    ),
    (
        "VNI Bàn phím tiếng Pháp",
        &[
            ("&", "XoáDấuThanh"),
            ("é", "DấuSắc"),
            ("\"", "DấuHuyền"),
            ("'", "DấuHỏi"),
            ("(", "DấuNgã"),
            ("-", "DấuNặng"),
            ("è", "AEO_ÂÊÔ"),
            ("_", "UO_ƯƠ"),
            ("ç", "A_Ă"),
            ("à", "D_Đ"),
        ],
    ),
    (
        "Telex W",
        &[
            ("z", "XoáDấuThanh"),
            ("s", "DấuSắc"),
            ("f", "DấuHuyền"),
            ("r", "DấuHỏi"),
            ("x", "DấuNgã"),
            ("j", "DấuNặng"),
            ("a", "A_Â"),
            ("e", "E_Ê"),
            ("o", "O_Ô"),
            ("w", "UOA_ƯƠĂ__Ư"),
            ("d", "D_Đ"),
        ],
    ),
];

/// The names of every built-in input method, in declaration order.
pub fn input_method_names() -> Vec<&'static str> {
    DEFINITIONS.iter().map(|(name, _)| *name).collect()
}

/// Compile the named built-in input method, or `None` if it is not built in.
pub fn parse_input_method(name: &str) -> Option<InputMethod> {
    let pairs = DEFINITIONS.iter().find(|(n, _)| *n == name)?.1;
    Some(build_input_method(name, pairs))
}

/// Compile an arbitrary input-method definition into an [`InputMethod`].
///
/// `definition` is a list of `(key, rule-line)` pairs in the same DSL the
/// built-in layouts use, letting callers build user-defined layouts loaded
/// from configuration.
pub fn build_input_method(name: &str, definition: &[(&str, &str)]) -> InputMethod {
    let mut im = InputMethod {
        name: name.to_string(),
        ..Default::default()
    };
    for (key_str, line) in definition {
        let key = key_str.chars().next().expect("definition key is non-empty");
        im.rules.extend(parse_rules(key, line));
        if line.to_lowercase().contains("uo") {
            im.super_keys.push(key);
        }
        im.keys.push(key);
    }
    for rule in &im.rules {
        match rule.effect_type {
            EffectType::Appending => im.appending_keys.push(rule.key),
            EffectType::ToneTransformation => im.tone_keys.push(rule.key),
            _ => {}
        }
    }
    im
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_layouts_compile() {
        for name in input_method_names() {
            let im = parse_input_method(name).expect("built-in layout");
            assert!(!im.rules.is_empty(), "{name} produced no rules");
            assert!(!im.keys.is_empty());
        }
    }

    #[test]
    fn telex_classifies_keys() {
        let im = parse_input_method("Telex").unwrap();
        assert_eq!(im.super_keys, vec!['w']);
        for k in ['z', 's', 'f', 'r', 'x', 'j'] {
            assert!(im.tone_keys.contains(&k), "{k} should be a tone key");
        }
        assert!(im.keys.contains(&'d'));
        // Telex has no bare-appending lines.
        assert!(im.appending_keys.is_empty());
    }

    #[test]
    fn telex_w_has_appending_w() {
        let im = parse_input_method("Telex W").unwrap();
        assert!(im.appending_keys.contains(&'w'));
    }

    #[test]
    fn unknown_layout_is_none() {
        assert!(parse_input_method("Bogus").is_none());
    }

    #[test]
    fn custom_definition_builds() {
        // A user-defined layout assembled from raw definition lines.
        let im = build_input_method("Custom", &[("s", "DấuSắc"), ("a", "A_Â")]);
        assert_eq!(im.name, "Custom");
        assert!(im.tone_keys.contains(&'s'));
        assert!(im.keys.contains(&'a'));
        assert!(im.super_keys.is_empty());
    }
}
