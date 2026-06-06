//! Engine configuration: in-memory settings plus load/save of the on-disk
//! JSON. The file format is drop-in compatible with the existing engine's
//! config (same field names and value encodings), so an existing config is
//! imported on first run.

use std::path::PathBuf;
use std::{env, fs, io};

use bambusa_core::EngineFlags;
use serde::{Deserialize, Serialize};

use crate::flags::{IBFlags, InputMode, Keyboard};

const CONFIG_SUBDIR: &str = "ibus-bambusa";
const CONFIG_FILE: &str = "ibus-bambusa.config.json";
const LEGACY_SUBDIR: &str = "ibus-bamboo";
const LEGACY_FILE: &str = "ibus-bamboo.config.json";

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
    /// The single global input mode.
    pub input_mode: InputMode,
    /// Physical keyboard assumed by the VNI method.
    pub vni_keyboard: Keyboard,
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
            vni_keyboard: Keyboard::Qwerty,
            shortcuts: [1, 126, 0, 0, 0, 0, 0, 0, 5, 117],
        }
    }
}

impl Config {
    /// Load the config: our file if present, otherwise the legacy file, else
    /// defaults. Any field absent from the file keeps its default.
    pub fn load() -> Self {
        let raw = fs::read_to_string(config_path())
            .ok()
            .or_else(|| fs::read_to_string(legacy_path()).ok());
        let file = raw
            .as_deref()
            .and_then(|s| serde_json::from_str::<ConfigFile>(s).ok())
            .unwrap_or_default();
        Self::from_file(file)
    }

    /// Persist the config to our namespace. Used when settings change (e.g.
    /// from the property panel).
    #[allow(dead_code)]
    pub fn save(&self) -> io::Result<()> {
        let file = ConfigFile {
            input_method: Some(self.input_method.clone()),
            output_charset: Some(self.output_charset.clone()),
            flags: Some(self.engine_flags.bits()),
            ib_flags: Some(self.ib_flags.bits()),
            shortcuts: Some(self.shortcuts),
            default_input_mode: Some(self.input_mode as i32),
            vni_keyboard: Some(match self.vni_keyboard {
                Keyboard::Azerty => "AZERTY".to_string(),
                Keyboard::Qwerty => "QWERTY".to_string(),
            }),
        };
        let path = config_path();
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_string_pretty(&file).map_err(io::Error::other)?;
        fs::write(path, json)
    }

    fn from_file(f: ConfigFile) -> Self {
        let d = Config::default();
        Config {
            input_method: f.input_method.unwrap_or(d.input_method),
            output_charset: f.output_charset.unwrap_or(d.output_charset),
            engine_flags: f
                .flags
                .map(EngineFlags::from_bits_truncate)
                .unwrap_or(d.engine_flags),
            ib_flags: f
                .ib_flags
                .map(IBFlags::from_bits_truncate)
                .unwrap_or(d.ib_flags),
            input_mode: f
                .default_input_mode
                .map(InputMode::from_stored)
                .unwrap_or(d.input_mode),
            vni_keyboard: match f.vni_keyboard.as_deref() {
                Some("AZERTY") => Keyboard::Azerty,
                Some("QWERTY") => Keyboard::Qwerty,
                _ => d.vni_keyboard,
            },
            shortcuts: f.shortcuts.unwrap_or(d.shortcuts),
        }
    }
}

/// The on-disk JSON shape. `Option` fields let an absent key fall back to the
/// default rather than to a zero value.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ConfigFile {
    #[serde(skip_serializing_if = "Option::is_none")]
    input_method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    output_charset: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    flags: Option<u32>,
    #[serde(rename = "IBflags", skip_serializing_if = "Option::is_none")]
    ib_flags: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shortcuts: Option<[u32; 10]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    default_input_mode: Option<i32>,
    #[serde(rename = "VniKeyboard", skip_serializing_if = "Option::is_none")]
    vni_keyboard: Option<String>,
}

fn config_home() -> PathBuf {
    match env::var("XDG_CONFIG_HOME") {
        Ok(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => PathBuf::from(env::var("HOME").unwrap_or_default()).join(".config"),
    }
}

fn config_path() -> PathBuf {
    config_home().join(CONFIG_SUBDIR).join(CONFIG_FILE)
}

fn legacy_path() -> PathBuf {
    config_home().join(LEGACY_SUBDIR).join(LEGACY_FILE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_used_for_missing_fields() {
        let json = r#"{"InputMethod":"VNI"}"#;
        let file: ConfigFile = serde_json::from_str(json).unwrap();
        let c = Config::from_file(file);
        assert_eq!(c.input_method, "VNI");
        // Everything else falls back to defaults.
        assert_eq!(c.output_charset, "Unicode");
        assert_eq!(c.ib_flags, IBFlags::STD);
        assert_eq!(c.input_mode, InputMode::Preedit);
        assert_eq!(c.shortcuts, [1, 126, 0, 0, 0, 0, 0, 0, 5, 117]);
    }

    #[test]
    fn decodes_full_legacy_config() {
        let json = r#"{
            "InputMethod": "VIQR",
            "OutputCharset": "TCVN3 (ABC)",
            "Flags": 7,
            "IBflags": 2,
            "Shortcuts": [1, 126, 0, 0, 0, 0, 0, 0, 5, 117],
            "DefaultInputMode": 2,
            "InputModeMapping": {"firefox": 3},
            "InputMethodDefinitions": {}
        }"#;
        let file: ConfigFile = serde_json::from_str(json).unwrap();
        let c = Config::from_file(file);
        assert_eq!(c.input_method, "VIQR");
        assert_eq!(c.output_charset, "TCVN3 (ABC)");
        assert_eq!(c.ib_flags, IBFlags::MACRO_ENABLED);
        assert_eq!(c.input_mode, InputMode::SurroundingText); // stored 2
    }

    #[test]
    fn xtest_mode_in_config_is_remapped() {
        let json = r#"{"DefaultInputMode": 6}"#;
        let file: ConfigFile = serde_json::from_str(json).unwrap();
        let c = Config::from_file(file);
        assert_eq!(c.input_mode, InputMode::BackspaceForwarding);
    }
}
