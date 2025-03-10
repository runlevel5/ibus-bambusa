//! Vietnamese text composition: turns a stream of keystrokes into composed
//! Vietnamese text for a chosen input method (Telex, VNI, VIQR and variants).

mod encoder;
mod engine;
mod flatten;
mod input_method;
mod mode;
mod parser;
mod rules;
mod spelling;
mod transform;
mod unicode_tables;

pub use encoder::{charset_names, encode};
pub use engine::BambooEngine;
pub use input_method::{input_method_names, parse_input_method, InputMethod};
pub use mode::{EngineFlags, Mode};
pub use rules::{EffectType, Mark, Rule, Tone};
pub use unicode_tables::{has_any_vietnamese_rune, has_any_vietnamese_vowel};
