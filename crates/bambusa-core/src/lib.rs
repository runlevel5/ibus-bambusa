//! Vietnamese text composition: turns a stream of keystrokes into composed
//! Vietnamese text for a chosen input method (Telex, VNI, VIQR and variants).

mod mode;
mod rules;

pub use mode::{EngineFlags, Mode};
pub use rules::{EffectType, Mark, Rule, Tone};
