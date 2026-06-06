//! Engine configuration.
//!
//! File load/save (drop-in compatible with the existing on-disk JSON) is added
//! later — for now the defaults are used.

use bambusa_core::EngineFlags;

use crate::flags::{IBFlags, InputMode};

/// Runtime configuration for an engine instance.
#[derive(Debug, Clone)]
pub struct Config {
    /// Selected input method name (e.g. `"Telex"`).
    pub input_method: String,
    /// Output charset name (e.g. `"Unicode"`).
    pub output_charset: String,
    /// Core composition flags (tone marking / auto-correct).
    pub engine_flags: EngineFlags,
    /// Engine feature flags.
    pub ib_flags: IBFlags,
    /// The single global input mode. Consulted once mode dispatch lands.
    #[allow(dead_code)]
    pub input_mode: InputMode,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            input_method: "Telex".to_string(),
            output_charset: "Unicode".to_string(),
            engine_flags: EngineFlags::STD,
            ib_flags: IBFlags::STD,
            input_mode: InputMode::Preedit,
        }
    }
}
