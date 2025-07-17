use cxx_qt_lib::QString;

use simsapa_backend::logger::info;

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("utils.h");
        fn get_app_data_storage_paths_json() -> QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[namespace = "storage_manager"]
        type StorageManager = super::StorageManagerRust;
    }

    unsafe extern "RustQt" {
        #[qinvokable]
        fn get_app_data_storage_paths_json(self: &StorageManager) -> QString;

        #[qinvokable]
        fn save_storage_path(self: &StorageManager, path: &QString, is_internal: bool);
    }
}

pub struct StorageManagerRust {}

impl Default for StorageManagerRust {
    fn default() -> Self {
        Self {}
    }
}

impl qobject::StorageManager {
    pub fn get_app_data_storage_paths_json(&self) -> QString {
        qobject::get_app_data_storage_paths_json()
    }

    pub fn save_storage_path(&self, path: &QString, is_internal: bool) {
        // FIXME save to storage-path.txt
        // Should already end with .../simsapa-ng/
        // let storage_config_path = internal_app_root.join("storage-path.txt");
        // Storage path: /
        // Storage path: /home/gambhiro/.local/share/simsapa-ng
        // Storage path: /home
        info(&format!("Storage path: {}, is_internal: {}", path, is_internal));
    }
}
