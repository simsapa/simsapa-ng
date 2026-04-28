//! Backend orchestration for user-imported StarDict dictionaries.
//!
//! All `import_user_zip` / `delete_user_dictionary` / `rename_user_dictionary`
//! calls are serialised by a single static `Mutex<()>` so the bridge can
//! reject overlapping operations with `Busy` and the UI can disable buttons
//! while one is in flight (PRD §4.2 req. 9).
//!
//! These functions deliberately touch only SQL — no FTS5, no Tantivy. The
//! startup reconciliation pass (`dict_index_reconcile`) owns all index
//! writes (PRD §4.9), which avoids contention with the live searcher.

use std::path::Path;
use std::sync::Mutex;

use crate::{get_app_data, get_app_globals};
use crate::logger::{info, error};
use crate::stardict_parse::{import_stardict_as_new, StardictImportProgress, read_ifo_description};

/// Single global serialisation lock for user-dictionary mutations.
///
/// We use `try_lock` so concurrent callers see a `Busy` error rather than
/// blocking the Qt main thread.
static DICT_MGR_LOCK: Mutex<()> = Mutex::new(());

/// Sentinel error string returned when the lock is already held.
pub const BUSY_MSG: &str = "Another dictionary operation is in progress.";

/// Validate a dictionary label.
///
/// Allowed characters: ASCII alnum, `_`, `-`. Must be non-empty.
pub fn validate_label(label: &str) -> Result<(), String> {
    if label.is_empty() {
        return Err("Label is empty.".to_string());
    }
    for c in label.chars() {
        let ok = c.is_ascii_alphanumeric() || c == '_' || c == '-';
        if !ok {
            return Err(format!(
                "Label contains invalid character '{}'. Allowed: ASCII letters/digits, '_', '-'.",
                c
            ));
        }
    }
    Ok(())
}

/// Sanitise a `.zip` filename stem into a label suggestion.
///
/// Replaces every non-`[A-Za-z0-9_-]` character with `_`, collapses runs of
/// `_`, then trims leading/trailing `_-`. Returns `""` if the result is empty
/// after sanitisation (so the dialog can leave the field blank).
pub fn suggested_label_for_zip(zip_path: &Path) -> String {
    let stem = match zip_path.file_stem().and_then(|s| s.to_str()) {
        Some(s) => s,
        None => return String::new(),
    };

    let mut out = String::with_capacity(stem.len());
    let mut prev_underscore = false;
    for c in stem.chars() {
        let ok = c.is_ascii_alphanumeric() || c == '_' || c == '-';
        if ok {
            out.push(c);
            prev_underscore = c == '_';
        } else if !prev_underscore {
            out.push('_');
            prev_underscore = true;
        }
    }

    out.trim_matches(|c: char| c == '_' || c == '-').to_string()
}

/// Import a user-supplied StarDict `.zip`.
///
/// Caller is responsible for resolving Replace-vs-Cancel beforehand: if the
/// label collides with an existing user dictionary, call
/// [`delete_user_dictionary`] first.
///
/// On success returns the new `dictionaries.id`. The row's `indexed_at` is
/// `NULL` so the next-startup reconciliation pass picks it up.
pub fn import_user_zip(
    zip_path: &Path,
    label: &str,
    lang: &str,
    on_progress: &dyn Fn(StardictImportProgress),
) -> Result<i32, String> {
    let _guard = match DICT_MGR_LOCK.try_lock() {
        Ok(g) => g,
        Err(_) => return Err(BUSY_MSG.to_string()),
    };

    // 1. Validate label format.
    validate_label(label)?;

    let app_data = get_app_data();

    // 2. Reject built-in / shipped collisions.
    let shipped = app_data.dbm.dictionaries.list_shipped_source_uids()
        .map_err(|e| format!("Failed to compute shipped source_uid set: {}", e))?;
    if shipped.contains(label) {
        return Err(format!(
            "Label '{}' collides with a built-in dictionary source.",
            label
        ));
    }

    // 3. Reject collisions with existing user dictionaries — caller must have
    //    handled Replace before invoking this function.
    let user_dicts = app_data.dbm.dictionaries.list_user_dictionaries()
        .map_err(|e| format!("Failed to list user dictionaries: {}", e))?;
    if user_dicts.iter().any(|d| d.label == label) {
        return Err(format!(
            "A user-imported dictionary with label '{}' already exists.",
            label
        ));
    }

    // 4. Verify the .zip exists.
    match zip_path.try_exists() {
        Ok(true) => {}
        Ok(false) => return Err(format!("Zip not found: {}", zip_path.display())),
        Err(e) => return Err(format!("Cannot access zip {}: {}", zip_path.display(), e)),
    }

    on_progress(StardictImportProgress::Extracting);

    // 5. Extract the .zip into a temp directory under the app cache so it
    //    lives somewhere Android tolerates. The TempDir auto-deletes on drop.
    let _ = app_data; // unused so far in this block; keep handle for downstream calls
    let cache_root = get_app_globals().paths.simsapa_dir.clone();
    let tmp = tempfile::Builder::new()
        .prefix("simsapa-stardict-")
        .tempdir_in(&cache_root)
        .map_err(|e| format!("Failed to create temp directory under {}: {}", cache_root.display(), e))?;
    let extract_dir = tmp.path().to_path_buf();

    let zip_file = std::fs::File::open(zip_path)
        .map_err(|e| format!("Failed to open zip {}: {}", zip_path.display(), e))?;
    let mut archive = zip::ZipArchive::new(zip_file)
        .map_err(|e| format!("Failed to read zip archive {}: {}", zip_path.display(), e))?;
    archive.extract(&extract_dir)
        .map_err(|e| format!("Failed to extract zip {}: {}", zip_path.display(), e))?;

    // 6. Locate the StarDict directory: the extracted contents may live at
    //    `<tmp>/` directly, or one level deep inside a wrapper folder.
    //    Look for a `<label>.ifo` sibling.
    let unzipped_dir = locate_stardict_dir(&extract_dir, label)
        .ok_or_else(|| format!("StarDict files for label '{}' not found in archive.", label))?;

    // 7. Capture the optional .ifo description.
    let description = read_ifo_description(&unzipped_dir, label);

    // 8. Run the SQL-only import.
    let dictionary_id = import_stardict_as_new(
        &unzipped_dir,
        lang,
        label,
        true,            // _ignore_synonyms (kept for parity with shipped path)
        false,           // delete_if_exists — caller has already deleted on Replace
        None,            // limit
        true,            // is_user_imported
        description.as_deref(),
        on_progress,
    ).map_err(|e| {
        // SQL-side failures inside import_stardict_as_new already roll back
        // the dictionaries row + dict_words. Surface the original message.
        error(&format!("import_user_zip: SQL import failed: {}", e));
        e
    })?;

    info(&format!("import_user_zip: '{}' -> id {}", label, dictionary_id));

    // tmp drops here; extracted files are deleted.
    drop(tmp);

    Ok(dictionary_id)
}

/// Find the directory containing `<label>.ifo` inside an extracted archive.
///
/// Many StarDict zips ship the files at the archive root; some wrap them in a
/// single folder. We try both.
fn locate_stardict_dir(extract_dir: &Path, label: &str) -> Option<std::path::PathBuf> {
    let direct = extract_dir.join(format!("{}.ifo", label));
    if matches!(direct.try_exists(), Ok(true)) {
        return Some(extract_dir.to_path_buf());
    }

    // One level deep.
    let entries = std::fs::read_dir(extract_dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if matches!(path.is_dir(), true) {
            let candidate = path.join(format!("{}.ifo", label));
            if matches!(candidate.try_exists(), Ok(true)) {
                return Some(path);
            }
        }
    }
    None
}

/// Delete a user-imported dictionary (SQL only).
///
/// Cascade drops the matching `dict_words`. FTS5 / Tantivy entries become
/// orphans and are cleaned up by the next startup reconciliation pass.
///
/// Refuses if the row's `is_user_imported` is false.
pub fn delete_user_dictionary(dictionary_id: i32) -> Result<(), String> {
    let _guard = match DICT_MGR_LOCK.try_lock() {
        Ok(g) => g,
        Err(_) => return Err(BUSY_MSG.to_string()),
    };

    let app_data = get_app_data();

    let user_dicts = app_data.dbm.dictionaries.list_user_dictionaries()
        .map_err(|e| format!("Failed to list user dictionaries: {}", e))?;
    let target = user_dicts.into_iter()
        .find(|d| d.id == dictionary_id)
        .ok_or_else(|| format!(
            "Dictionary id {} is not a user-imported dictionary; refusing to delete.",
            dictionary_id
        ))?;

    let n = app_data.dbm.dictionaries.delete_dictionary_by_label(&target.label)
        .map_err(|e| format!("delete_dictionary_by_label failed: {}", e))?;
    info(&format!("delete_user_dictionary: removed {} dictionaries row(s) for '{}'", n, target.label));
    Ok(())
}

/// Rename a user-imported dictionary's label (SQL only).
///
/// On success the row's `indexed_at` is set to NULL so the next startup
/// reconciliation pass re-indexes both old and new labels.
pub fn rename_user_dictionary(dictionary_id: i32, new_label: &str) -> Result<(), String> {
    let _guard = match DICT_MGR_LOCK.try_lock() {
        Ok(g) => g,
        Err(_) => return Err(BUSY_MSG.to_string()),
    };

    validate_label(new_label)?;

    let app_data = get_app_data();

    // Reject built-in collisions.
    let shipped = app_data.dbm.dictionaries.list_shipped_source_uids()
        .map_err(|e| format!("Failed to compute shipped source_uid set: {}", e))?;
    if shipped.contains(new_label) {
        return Err(format!(
            "Label '{}' collides with a built-in dictionary source.",
            new_label
        ));
    }

    let user_dicts = app_data.dbm.dictionaries.list_user_dictionaries()
        .map_err(|e| format!("Failed to list user dictionaries: {}", e))?;

    // Find the target row.
    let target = user_dicts.iter()
        .find(|d| d.id == dictionary_id)
        .ok_or_else(|| format!(
            "Dictionary id {} is not a user-imported dictionary; refusing to rename.",
            dictionary_id
        ))?;

    if target.label == new_label {
        return Ok(());
    }

    // Reject collisions with another user dict.
    if user_dicts.iter().any(|d| d.id != dictionary_id && d.label == new_label) {
        return Err(format!(
            "Another user-imported dictionary already uses label '{}'.",
            new_label
        ));
    }

    app_data.dbm.dictionaries.rename_dictionary_label(&target.label, new_label)
        .map_err(|e| format!("rename_dictionary_label failed: {}", e))?;
    info(&format!("rename_user_dictionary: '{}' -> '{}'", target.label, new_label));
    Ok(())
}
