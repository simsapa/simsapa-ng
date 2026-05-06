//! OS-level global hotkey settings.
//!
//! Stored as a sub-struct of [`crate::app_settings::AppSettings`] so it
//! persists alongside `app_keybindings` in the appdata DB row, rather than in
//! a separate sidecar file. The struct lives in its own module because it
//! carries an extra `enabled` flag and is managed independently from the
//! in-app keybindings (see PRD §4.2).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

pub const DICTIONARY_LOOKUP_ACTION: &str = "dictionary_lookup";
pub const DEFAULT_DICTIONARY_LOOKUP_SEQUENCE: &str = "Ctrl+C+C";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct GlobalHotkeysConfig {
    pub enabled: bool,
    pub bindings: HashMap<String, String>,
}

impl Default for GlobalHotkeysConfig {
    fn default() -> Self {
        let mut bindings = HashMap::new();
        bindings.insert(
            DICTIONARY_LOOKUP_ACTION.to_string(),
            DEFAULT_DICTIONARY_LOOKUP_SEQUENCE.to_string(),
        );
        GlobalHotkeysConfig {
            enabled: false,
            bindings,
        }
    }
}

impl GlobalHotkeysConfig {
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn set_binding(&mut self, action_id: &str, sequence: &str) {
        self.bindings
            .insert(action_id.to_string(), sequence.to_string());
    }

    pub fn get_binding(&self, action_id: &str) -> Option<&str> {
        self.bindings.get(action_id).map(|s| s.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_dictionary_lookup() {
        let cfg = GlobalHotkeysConfig::default();
        assert!(!cfg.enabled);
        assert_eq!(
            cfg.get_binding(DICTIONARY_LOOKUP_ACTION),
            Some(DEFAULT_DICTIONARY_LOOKUP_SEQUENCE)
        );
    }

    #[test]
    fn binding_mutation() {
        let mut cfg = GlobalHotkeysConfig::default();
        cfg.set_binding(DICTIONARY_LOOKUP_ACTION, "Ctrl+Shift+D");
        assert_eq!(cfg.get_binding(DICTIONARY_LOOKUP_ACTION), Some("Ctrl+Shift+D"));
        cfg.set_enabled(true);
        assert!(cfg.enabled);
    }

    #[test]
    fn json_roundtrip_through_app_settings_field() {
        let mut cfg = GlobalHotkeysConfig::default();
        cfg.set_enabled(true);
        cfg.set_binding(DICTIONARY_LOOKUP_ACTION, "Ctrl+Alt+L");
        let json = serde_json::to_string(&cfg).unwrap();
        let back: GlobalHotkeysConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back, cfg);
    }
}
