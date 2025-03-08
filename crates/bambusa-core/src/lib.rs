//! Vietnamese text composition: turns a stream of keystrokes into composed
//! Vietnamese text for a chosen input method (Telex, VNI, VIQR and variants).

mod input_method;
mod mode;
mod parser;
mod rules;
mod unicode_tables;

pub use input_method::{input_method_names, parse_input_method, InputMethod};
pub use mode::{EngineFlags, Mode};
pub use rules::{EffectType, Mark, Rule, Tone};
