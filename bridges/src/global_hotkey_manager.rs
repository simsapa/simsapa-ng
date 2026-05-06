//! CXX-Qt bridge for the OS-level global hotkey settings.
//!
//! For now this exposes only the JSON-based settings API. The C++
//! `GlobalHotkeyManager` (added in task 4.0) will be wired in here later;
//! this module already owns the persistence side so the QML settings UI
//! (task 2.0) and capture dialog (task 3.0) can be developed independently.
//!
//! Goldendict-ng (GPLv3) is the reference for the OS-level grabbing
//! design. No source files are copied verbatim — see the per-platform
//! C++/.mm files added later.

use core::pin::Pin;

use cxx_qt_lib::QString;

use simsapa_backend::get_app_data;
use simsapa_backend::global_hotkeys::GlobalHotkeysConfig;
use simsapa_backend::logger::error;

unsafe extern "C" {
    /// Defined in `cpp/gui.cpp`. Tells the C++ `GlobalHotkeyManager` to
    /// unregister all OS-level grabs and re-register from current settings,
    /// so that changes made via the QML settings UI take effect without an
    /// app restart.
    fn reregister_global_hotkeys_c();
    /// Defined in `cpp/gui.cpp`. Clears the "registration error already
    /// shown this session" flag.
    fn reset_global_hotkey_error_flag_c();
}

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("utils.h");
        fn get_qt_platform_name() -> QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[namespace = "global_hotkey_manager"]
        type GlobalHotkeyManager = super::GlobalHotkeyManagerRust;

        #[qinvokable]
        fn get_global_hotkeys_json(self: &GlobalHotkeyManager) -> QString;

        #[qinvokable]
        fn get_default_global_hotkeys_json(self: &GlobalHotkeyManager) -> QString;

        #[qinvokable]
        fn set_global_hotkey(
            self: Pin<&mut GlobalHotkeyManager>,
            action_id: &QString,
            sequence: &QString,
        );

        #[qinvokable]
        fn set_global_hotkeys_enabled(self: Pin<&mut GlobalHotkeyManager>, enabled: bool);

        #[qinvokable]
        fn is_wayland(self: &GlobalHotkeyManager) -> bool;

        #[qinvokable]
        fn get_api_url(self: &GlobalHotkeyManager) -> QString;

        #[qsignal]
        #[cxx_name = "globalHotkeysChanged"]
        fn global_hotkeys_changed(self: Pin<&mut GlobalHotkeyManager>);

        #[qsignal]
        #[cxx_name = "globalDictionaryLookupRequested"]
        fn global_dictionary_lookup_requested(
            self: Pin<&mut GlobalHotkeyManager>,
            query: QString,
        );
    }
}

#[derive(Default)]
pub struct GlobalHotkeyManagerRust;

fn config_to_json(cfg: &GlobalHotkeysConfig) -> String {
    serde_json::to_string(cfg).unwrap_or_else(|e| {
        error(&format!("GlobalHotkeyManager serialize: {}", e));
        "{}".to_string()
    })
}

/// Validate that a captured key sequence is a non-empty, reasonably-sized
/// string. The full `QKeySequence` parse happens C++-side when the hotkey
/// is registered; here we just keep obvious garbage out of the JSON file.
pub fn is_valid_sequence_string(s: &str) -> bool {
    let s = s.trim();
    !s.is_empty() && s.len() <= 64
}

impl qobject::GlobalHotkeyManager {
    pub fn get_global_hotkeys_json(&self) -> QString {
        let cfg = get_app_data().get_global_hotkeys();
        QString::from(&config_to_json(&cfg))
    }

    pub fn get_default_global_hotkeys_json(&self) -> QString {
        QString::from(&config_to_json(&GlobalHotkeysConfig::default()))
    }

    pub fn set_global_hotkey(
        self: Pin<&mut Self>,
        action_id: &QString,
        sequence: &QString,
    ) {
        let action_id = action_id.to_string();
        let sequence = sequence.to_string();
        if !is_valid_sequence_string(&sequence) {
            error(&format!(
                "GlobalHotkeyManager::set_global_hotkey rejecting invalid sequence: {:?}",
                sequence
            ));
            return;
        }
        get_app_data().set_global_hotkey_binding(&action_id, &sequence);
        // Reset the one-shot "registration failed" dialog flag so a new
        // sequence that conflicts surfaces a fresh dialog.
        unsafe { reset_global_hotkey_error_flag_c(); }
        unsafe { reregister_global_hotkeys_c(); }
        self.global_hotkeys_changed();
    }

    pub fn set_global_hotkeys_enabled(self: Pin<&mut Self>, enabled: bool) {
        get_app_data().set_global_hotkeys_enabled(enabled);
        unsafe { reregister_global_hotkeys_c(); }
        self.global_hotkeys_changed();
    }

    pub fn is_wayland(&self) -> bool {
        let name = qobject::get_qt_platform_name().to_string();
        name == "wayland" || name == "wayland-egl"
    }

    pub fn get_api_url(&self) -> QString {
        QString::from(&get_app_data().api_url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use simsapa_backend::global_hotkeys::{
        DEFAULT_DICTIONARY_LOOKUP_SEQUENCE, DICTIONARY_LOOKUP_ACTION,
    };

    #[test]
    fn rejects_empty_sequence() {
        assert!(!is_valid_sequence_string(""));
        assert!(!is_valid_sequence_string("   "));
    }

    #[test]
    fn accepts_typical_sequences() {
        assert!(is_valid_sequence_string("Ctrl+C+C"));
        assert!(is_valid_sequence_string("Ctrl+Alt+L"));
        assert!(is_valid_sequence_string("Ctrl+Shift+D"));
    }

    #[test]
    fn rejects_overly_long_sequence() {
        let long = "X".repeat(200);
        assert!(!is_valid_sequence_string(&long));
    }

    #[test]
    fn default_json_has_dictionary_lookup() {
        let json = config_to_json(&GlobalHotkeysConfig::default());
        assert!(json.contains(DICTIONARY_LOOKUP_ACTION));
        assert!(json.contains(DEFAULT_DICTIONARY_LOOKUP_SEQUENCE));
        assert!(json.contains("\"enabled\":false"));
    }
}
