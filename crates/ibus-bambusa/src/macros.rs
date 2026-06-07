//! User-defined text macros: `shortcut -> expansion` pairs, expanded on a word
//! break or Tab. The pairs come from GSettings (managed in the preferences GUI).
//!
//! With auto-capitalize on, keys are matched case-insensitively and the
//! expansion follows the typed case.

use std::collections::HashMap;

pub struct MacroTable {
    auto_capitalize: bool,
    table: HashMap<String, String>,
}

impl MacroTable {
    /// An empty table (macros disabled).
    pub fn empty(auto_capitalize: bool) -> Self {
        Self {
            auto_capitalize,
            table: HashMap::new(),
        }
    }

    /// Build a table from `(shortcut, expansion)` pairs.
    pub fn from_entries(entries: &[(String, String)], auto_capitalize: bool) -> Self {
        let mut table = HashMap::new();
        for (key, value) in entries {
            if key.is_empty() || value.is_empty() {
                continue;
            }
            let key = if auto_capitalize {
                key.to_lowercase()
            } else {
                key.clone()
            };
            table.insert(key, value.clone());
        }
        Self {
            auto_capitalize,
            table,
        }
    }

    fn norm(&self, key: &str) -> String {
        if self.auto_capitalize {
            key.to_lowercase()
        } else {
            key.to_string()
        }
    }

    /// The raw expansion for `key`, or `None` if there is no such macro.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.table
            .get(&self.norm(key))
            .map(String::as_str)
            .filter(|s| !s.is_empty())
    }

    pub fn has_key(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    /// Whether `key` is a macro key or the prefix of one (so we keep buffering).
    pub fn has_prefix(&self, key: &str) -> bool {
        let key = self.norm(key);
        self.table.keys().any(|k| k.starts_with(&key))
    }

    #[cfg(test)]
    pub fn from_pairs(pairs: &[(&str, &str)], auto_capitalize: bool) -> Self {
        let entries: Vec<(String, String)> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Self::from_entries(&entries, auto_capitalize)
    }
}
