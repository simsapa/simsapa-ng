//! Bridge for the user-imported StarDict dictionary feature.
//!
//! Exposes import/list/rename/delete operations to QML, plus the persisted
//! per-dictionary enabled flags used by the dictionary search UI. Mutating
//! ops route through [`simsapa_backend::dictionary_manager_core`], which
//! holds the global serialisation mutex.

use std::path::PathBuf;
use std::thread;

use core::pin::Pin;
use cxx_qt_lib::QString;
use cxx_qt::Threading;

use serde::Serialize;

use simsapa_backend::dictionary_manager_core::{
    self, BUSY_MSG, suggested_label_for_zip as core_suggested_label_for_zip,
    validate_label as core_validate_label,
};
use simsapa_backend::dict_index_reconcile::{self, ReconcileProgress};
use simsapa_backend::get_app_data;
use simsapa_backend::logger::{error, info};
use simsapa_backend::stardict_parse::StardictImportProgress;

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[namespace = "dictionary_manager"]
        type DictionaryManager = super::DictionaryManagerRust;
    }

    impl cxx_qt::Threading for DictionaryManager {}

    extern "RustQt" {
        // Mutating operations (run on a worker thread, emit signals).
        #[qinvokable]
        fn import_zip(self: Pin<&mut DictionaryManager>, zip_path: &QString, label: &QString, lang: &QString) -> QString;

        #[qinvokable]
        fn delete_dictionary(self: &DictionaryManager, dictionary_id: i32) -> QString;

        #[qinvokable]
        fn rename_label(self: &DictionaryManager, dictionary_id: i32, new_label: &QString) -> QString;

        // Read-only / pure helpers.
        #[qinvokable]
        fn list_user_dictionaries(self: &DictionaryManager) -> QString;

        #[qinvokable]
        fn list_shipped_source_uids(self: &DictionaryManager) -> QString;

        #[qinvokable]
        fn dpd_source_uids(self: &DictionaryManager) -> QString;

        #[qinvokable]
        fn commentary_definitions_source_uids(self: &DictionaryManager) -> QString;

        #[qinvokable]
        fn label_status(self: &DictionaryManager, label: &QString) -> QString;

        #[qinvokable]
        fn suggested_label_for_zip(self: &DictionaryManager, zip_path: &QString) -> QString;

        #[qinvokable]
        fn is_known_tokenizer_lang(self: &DictionaryManager, lang: &QString) -> bool;

        // Per-dictionary enabled flags.
        #[qinvokable]
        fn get_user_dict_enabled(self: &DictionaryManager, label: &QString) -> bool;

        #[qinvokable]
        fn set_user_dict_enabled(self: &DictionaryManager, label: &QString, enabled: bool);

        #[qinvokable]
        fn get_user_dict_enabled_map(self: &DictionaryManager) -> QString;

        #[qinvokable]
        fn get_dpd_enabled(self: &DictionaryManager) -> bool;

        #[qinvokable]
        fn set_dpd_enabled(self: &DictionaryManager, enabled: bool);

        #[qinvokable]
        fn get_commentary_definitions_enabled(self: &DictionaryManager) -> bool;

        #[qinvokable]
        fn set_commentary_definitions_enabled(self: &DictionaryManager, enabled: bool);

        // Startup reconciliation entry-points.
        #[qinvokable]
        fn reconcile_needed(self: &DictionaryManager) -> bool;

        #[qinvokable]
        fn start_reconcile(self: Pin<&mut DictionaryManager>);

        // Signals emitted from worker threads via CxxQtThread.
        #[qsignal]
        #[cxx_name = "importProgress"]
        fn import_progress(self: Pin<&mut DictionaryManager>, stage: QString, done: i32, total: i32);

        #[qsignal]
        #[cxx_name = "importFinished"]
        fn import_finished(self: Pin<&mut DictionaryManager>, dictionary_id: i32, label: QString);

        #[qsignal]
        #[cxx_name = "importFailed"]
        fn import_failed(self: Pin<&mut DictionaryManager>, message: QString);

        #[qsignal]
        #[cxx_name = "reconcileProgress"]
        fn reconcile_progress(self: Pin<&mut DictionaryManager>, stage: QString, done: i32, total: i32);

        #[qsignal]
        #[cxx_name = "reconcileFinished"]
        fn reconcile_finished(self: Pin<&mut DictionaryManager>);
    }
}

#[derive(Default)]
pub struct DictionaryManagerRust;

#[derive(Serialize)]
struct UserDictRowJson {
    id: i32,
    label: String,
    title: String,
    language: Option<String>,
    entry_count: i64,
    description: Option<String>,
}

/// ASCII tokenizer language codes that `register_tokenizers` supports
/// directly (mirrors `snowball::lang_to_algorithm`). Anything else falls
/// back to English at indexing time — which is acceptable, but the import
/// dialog warns the user.
const KNOWN_TOKENIZER_LANGS: &[&str] = &[
    "pli", "san",
    "ar", "hy", "eu", "ca", "da", "nl", "en", "eo", "et",
    "fi", "fr", "de", "el", "hi", "hu", "id", "ga", "it",
    "lt", "ne", "no", "pl", "pt", "ro", "ru", "sr", "es",
    "sv", "ta", "tr", "yi",
];

fn stardict_progress_to_signal(p: &StardictImportProgress) -> (String, i32, i32) {
    match p {
        StardictImportProgress::Extracting => ("Extracting".to_string(), 0, 0),
        StardictImportProgress::Parsing => ("Parsing".to_string(), 0, 0),
        StardictImportProgress::InsertingWords { done, total } => {
            ("Inserting words".to_string(), *done as i32, *total as i32)
        }
        StardictImportProgress::Done => ("Done".to_string(), 0, 0),
        StardictImportProgress::Failed { msg } => (format!("Failed: {}", msg), 0, 0),
    }
}

fn reconcile_progress_to_signal(p: &ReconcileProgress) -> (String, i32, i32) {
    match p {
        ReconcileProgress::DroppingOrphans { done, total, label } => {
            let stage = match label {
                Some(l) => format!("Dropping orphan: {}", l),
                None => "Dropping orphans".to_string(),
            };
            (stage, *done as i32, *total as i32)
        }
        ReconcileProgress::IndexingDictionary { label, done, total, dict_index, dict_total } => (
            format!("Indexing {} ({}/{})", label, dict_index, dict_total),
            *done as i32,
            *total as i32,
        ),
        ReconcileProgress::Done => ("Done".to_string(), 0, 0),
    }
}

impl qobject::DictionaryManager {
    fn import_zip(self: Pin<&mut Self>, zip_path: &QString, label: &QString, lang: &QString) -> QString {
        let qt_thread = self.qt_thread();
        let zip_path = PathBuf::from(zip_path.to_string());
        let label = label.to_string();
        let lang = lang.to_string();

        thread::spawn(move || {
            let progress_thread = qt_thread.clone();
            let on_progress = move |p: StardictImportProgress| {
                let (stage, done, total) = stardict_progress_to_signal(&p);
                let qs = QString::from(&stage);
                let _ = progress_thread.queue(move |mut qo| {
                    qo.as_mut().import_progress(qs, done, total);
                });
            };

            match dictionary_manager_core::import_user_zip(&zip_path, &label, &lang, &on_progress) {
                Ok(dictionary_id) => {
                    let label_qs = QString::from(&label);
                    let _ = qt_thread.queue(move |mut qo| {
                        qo.as_mut().import_finished(dictionary_id, label_qs);
                    });
                }
                Err(msg) => {
                    error(&format!("import_zip failed: {}", msg));
                    let qs = QString::from(&msg);
                    let _ = qt_thread.queue(move |mut qo| {
                        qo.as_mut().import_failed(qs);
                    });
                }
            }
        });

        QString::from("ok")
    }

    fn delete_dictionary(&self, dictionary_id: i32) -> QString {
        match dictionary_manager_core::delete_user_dictionary(dictionary_id) {
            Ok(()) => QString::from("ok"),
            Err(msg) => QString::from(&msg),
        }
    }

    fn rename_label(&self, dictionary_id: i32, new_label: &QString) -> QString {
        match dictionary_manager_core::rename_user_dictionary(dictionary_id, &new_label.to_string()) {
            Ok(()) => QString::from("ok"),
            Err(msg) => QString::from(&msg),
        }
    }

    fn list_user_dictionaries(&self) -> QString {
        let app_data = get_app_data();
        let rows = match app_data.dbm.dictionaries.list_user_dictionaries() {
            Ok(rs) => rs,
            Err(e) => {
                error(&format!("list_user_dictionaries: {}", e));
                return QString::from("[]");
            }
        };

        let json_rows: Vec<UserDictRowJson> = rows.into_iter().map(|d| {
            let entry_count = app_data.dbm.dictionaries
                .count_words_for_dictionary(d.id)
                .unwrap_or(0);
            UserDictRowJson {
                id: d.id,
                label: d.label,
                title: d.title,
                language: d.language,
                entry_count,
                description: d.description,
            }
        }).collect();

        match serde_json::to_string(&json_rows) {
            Ok(s) => QString::from(&s),
            Err(e) => {
                error(&format!("list_user_dictionaries serialize: {}", e));
                QString::from("[]")
            }
        }
    }

    fn list_shipped_source_uids(&self) -> QString {
        let app_data = get_app_data();
        match app_data.dbm.dictionaries.list_shipped_source_uids() {
            Ok(set) => {
                let v: Vec<&String> = set.iter().collect();
                match serde_json::to_string(&v) {
                    Ok(s) => QString::from(&s),
                    Err(e) => {
                        error(&format!("list_shipped_source_uids serialize: {}", e));
                        QString::from("[]")
                    }
                }
            }
            Err(e) => {
                error(&format!("list_shipped_source_uids: {}", e));
                QString::from("[]")
            }
        }
    }

    fn dpd_source_uids(&self) -> QString {
        // DPD's dict_words rows are inserted with `dict_label = "dpd"` by
        // `find_or_create_dpd_dictionary`. The set is fixed and small.
        QString::from("[\"dpd\"]")
    }

    fn commentary_definitions_source_uids(&self) -> QString {
        let app_data = get_app_data();
        match app_data.dbm.dpd.list_distinct_bold_def_ref_codes() {
            Ok(set) => {
                let v: Vec<&String> = set.iter().collect();
                match serde_json::to_string(&v) {
                    Ok(s) => QString::from(&s),
                    Err(e) => {
                        error(&format!("commentary_definitions_source_uids serialize: {}", e));
                        QString::from("[]")
                    }
                }
            }
            Err(e) => {
                error(&format!("commentary_definitions_source_uids: {}", e));
                QString::from("[]")
            }
        }
    }

    fn label_status(&self, label: &QString) -> QString {
        let label_str = label.to_string();
        if core_validate_label(&label_str).is_err() {
            return QString::from("invalid");
        }
        let app_data = get_app_data();
        match app_data.dbm.dictionaries.is_label_taken_by_shipped(&label_str) {
            Ok(true) => return QString::from("taken_shipped"),
            Ok(false) => {}
            Err(e) => {
                error(&format!("label_status shipped check: {}", e));
                return QString::from("invalid");
            }
        }
        match app_data.dbm.dictionaries.list_user_dictionaries() {
            Ok(rows) => {
                if rows.iter().any(|d| d.label == label_str) {
                    QString::from("taken_user")
                } else {
                    QString::from("available")
                }
            }
            Err(e) => {
                error(&format!("label_status user check: {}", e));
                QString::from("invalid")
            }
        }
    }

    fn suggested_label_for_zip(&self, zip_path: &QString) -> QString {
        let p = PathBuf::from(zip_path.to_string());
        QString::from(&core_suggested_label_for_zip(&p))
    }

    fn is_known_tokenizer_lang(&self, lang: &QString) -> bool {
        let s = lang.to_string();
        let s = s.trim().to_ascii_lowercase();
        KNOWN_TOKENIZER_LANGS.contains(&s.as_str())
    }

    fn get_user_dict_enabled(&self, label: &QString) -> bool {
        get_app_data().get_user_dict_enabled(&label.to_string())
    }

    fn set_user_dict_enabled(&self, label: &QString, enabled: bool) {
        get_app_data().set_user_dict_enabled(&label.to_string(), enabled);
    }

    fn get_user_dict_enabled_map(&self) -> QString {
        let map = get_app_data().list_user_dict_enabled();
        match serde_json::to_string(&map) {
            Ok(s) => QString::from(&s),
            Err(e) => {
                error(&format!("get_user_dict_enabled_map serialize: {}", e));
                QString::from("{}")
            }
        }
    }

    fn get_dpd_enabled(&self) -> bool {
        get_app_data().get_dpd_enabled()
    }

    fn set_dpd_enabled(&self, enabled: bool) {
        get_app_data().set_dpd_enabled(enabled);
    }

    fn get_commentary_definitions_enabled(&self) -> bool {
        get_app_data().get_commentary_definitions_enabled()
    }

    fn set_commentary_definitions_enabled(&self, enabled: bool) {
        get_app_data().set_commentary_definitions_enabled(enabled);
    }

    fn reconcile_needed(&self) -> bool {
        dict_index_reconcile::reconcile_needed()
    }

    fn start_reconcile(self: Pin<&mut Self>) {
        let qt_thread = self.qt_thread();
        thread::spawn(move || {
            let progress_thread = qt_thread.clone();
            let on_progress = move |p: ReconcileProgress| {
                let (stage, done, total) = reconcile_progress_to_signal(&p);
                let qs = QString::from(&stage);
                let _ = progress_thread.queue(move |mut qo| {
                    qo.as_mut().reconcile_progress(qs, done, total);
                });
            };
            if let Err(e) = dict_index_reconcile::reconcile_dict_indexes(on_progress) {
                error(&format!("reconcile_dict_indexes failed: {:#}", e));
            }
            info("start_reconcile: complete");
            let _ = qt_thread.queue(move |mut qo| {
                qo.as_mut().reconcile_finished();
            });
        });
    }
}

// Silence "unused" warning for BUSY_MSG re-export when not consumed here.
#[allow(dead_code)]
const _BUSY_MSG_DOC: &str = BUSY_MSG;
