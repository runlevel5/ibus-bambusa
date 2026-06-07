//! Engine configuration, backed by GSettings (dconf).
//!
//! The engine reads it (on focus); the preferences GUI writes it. Both go
//! through the same schema, so a change in the GUI is picked up on the next
//! focus. The input method is *not* stored here — it is fixed by the IBus
//! engine name — and the shortcut slots stay at their defaults (switching is
//! delegated to GNOME).

use bambusa_core::EngineFlags;
use gio::prelude::*;

use crate::flags::{IBFlags, InputMode};

/// The GSettings schema id (and dconf path `/org/freedesktop/ibus/bambusa/`).
pub const SCHEMA_ID: &str = "org.freedesktop.IBus.bambusa";

/// GSettings key names, shared by the engine and the preferences GUI.
pub mod keys {
    pub const OUTPUT_CHARSET: &str = "output-charset";
    pub const INPUT_MODE: &str = "input-mode";
    pub const FREE_TONE_MARKING: &str = "free-tone-marking";
    pub const MODERN_TONE_STYLE: &str = "modern-tone-style";
    pub const SPELL_CHECK: &str = "spell-check";
    pub const SPELL_CHECK_RULES: &str = "spell-check-rules";
    pub const SPELL_CHECK_DICTS: &str = "spell-check-dicts";
    pub const AUTO_RESTORE_NON_VN: &str = "auto-restore-non-vn";
    pub const DD_FREE_STYLE: &str = "dd-free-style";
    pub const HIDE_UNDERLINE: &str = "hide-underline";
    pub const MACROS_ENABLED: &str = "macros-enabled";
    pub const AUTO_CAPITALIZE_MACROS: &str = "auto-capitalize-macros";
    pub const WORKAROUND_FB_MESSENGER: &str = "workaround-fb-messenger";
}

/// Runtime configuration for an engine instance.
#[derive(Debug, Clone)]
pub struct Config {
    /// Selected input method name (e.g. `"Telex"`), set from the engine name.
    pub input_method: String,
    /// Output charset name (e.g. `"Unicode"`).
    pub output_charset: String,
    /// Core composition flags (tone marking / auto-correct).
    pub engine_flags: EngineFlags,
    /// Engine feature flags.
    pub ib_flags: IBFlags,
    /// The single global input mode.
    pub input_mode: InputMode,
    /// Keyboard shortcut slots: five `(state, keyval)` pairs.
    pub shortcuts: [u32; 10],
}

impl Default for Config {
    fn default() -> Self {
        Self {
            input_method: "Telex".to_string(),
            output_charset: "Unicode".to_string(),
            engine_flags: EngineFlags::STD,
            ib_flags: IBFlags::STD,
            input_mode: InputMode::Preedit,
            shortcuts: [1, 126, 0, 0, 0, 0, 0, 0, 5, 117],
        }
    }
}

impl Config {
    /// Load settings from GSettings, falling back to defaults if the schema is
    /// not installed. The input method and shortcuts keep their defaults.
    pub fn load() -> Self {
        let Some(settings) = open_settings() else {
            return Self::default();
        };
        let d = Config::default();

        let mut engine_flags = EngineFlags::empty();
        engine_flags.set(
            EngineFlags::FREE_TONE_MARKING,
            settings.boolean(keys::FREE_TONE_MARKING),
        );
        // STD_TONE_STYLE is the *old* first-vowel placement (hòa); the
        // "modern tone placement" key is its inverse (hoà on the second vowel).
        engine_flags.set(
            EngineFlags::STD_TONE_STYLE,
            !settings.boolean(keys::MODERN_TONE_STYLE),
        );

        let mut ib_flags = IBFlags::empty();
        for (key, flag) in [
            (keys::SPELL_CHECK, IBFlags::SPELL_CHECK_ENABLED),
            (keys::SPELL_CHECK_RULES, IBFlags::SPELL_CHECK_WITH_RULES),
            (keys::SPELL_CHECK_DICTS, IBFlags::SPELL_CHECK_WITH_DICTS),
            (keys::AUTO_RESTORE_NON_VN, IBFlags::AUTO_NON_VN_RESTORE),
            (keys::DD_FREE_STYLE, IBFlags::DD_FREE_STYLE),
            (keys::HIDE_UNDERLINE, IBFlags::NO_UNDERLINE),
            (keys::MACROS_ENABLED, IBFlags::MACRO_ENABLED),
            (keys::AUTO_CAPITALIZE_MACROS, IBFlags::AUTO_CAPITALIZE_MACRO),
            (
                keys::WORKAROUND_FB_MESSENGER,
                IBFlags::WORKAROUND_FB_MESSENGER,
            ),
        ] {
            ib_flags.set(flag, settings.boolean(key));
        }

        Config {
            input_method: d.input_method,
            output_charset: settings.string(keys::OUTPUT_CHARSET).to_string(),
            engine_flags,
            ib_flags,
            input_mode: InputMode::from_stored(settings.enum_(keys::INPUT_MODE)),
            shortcuts: d.shortcuts,
        }
    }
}

/// Open the GSettings for our schema, or `None` if it is not installed (so the
/// engine runs on defaults instead of aborting).
fn open_settings() -> Option<gio::Settings> {
    let source = gio::SettingsSchemaSource::default()?;
    source.lookup(SCHEMA_ID, true)?;
    Some(gio::Settings::new(SCHEMA_ID))
}
