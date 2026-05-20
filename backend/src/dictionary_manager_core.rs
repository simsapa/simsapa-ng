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
use std::sync::atomic::AtomicBool;

use crate::{get_app_data, get_app_globals};
use crate::logger::{info, error};
use crate::stardict_parse::{import_stardict_as_new, ImportOutcome, StardictImportProgress, read_ifo_description};

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
    cancel: &AtomicBool,
) -> Result<ImportOutcome, String> {
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

    // 3. Reject collisions with existing dictionaries — caller must have
    //    handled Replace before invoking this function.
    let user_dicts = app_data.dbm.dictionaries.list_dictionaries(None)
        .map_err(|e| format!("Failed to list dictionaries: {}", e))?;
    if user_dicts.iter().any(|d| d.label == label) {
        return Err(format!(
            "A dictionary with label '{}' already exists.",
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

    // 6. Locate the StarDict directory and discover the physical .ifo basename.
    //    The extracted contents may live at `<tmp>/` directly or one level deep
    //    inside a wrapper folder; the .ifo basename is whatever the upstream
    //    archive ships (e.g. `concise-eng-pli.ifo`) and need not match the
    //    user-chosen label.
    let (unzipped_dir, physical_stem) = locate_stardict_dir(&extract_dir)
        .ok_or_else(|| "No `.ifo` file found in archive.".to_string())?;

    // 7. Capture the optional .ifo description.
    let description = read_ifo_description(&unzipped_dir, &physical_stem);

    // 8. Run the SQL-only import. `physical_stem` locates the files on disk;
    //    `label` is the logical label stored on the dictionaries row and used
    //    as the `{word}/{label}` uid suffix.
    let outcome = import_stardict_as_new(
        &unzipped_dir,
        lang,
        &physical_stem,
        label,
        true,            // _ignore_synonyms (kept for parity with shipped path)
        false,           // delete_if_exists — caller has already deleted on Replace
        None,            // limit
        true,            // is_user_imported
        description.as_deref(),
        on_progress,
        cancel,
    ).map_err(|e| {
        // SQL-side failures inside import_stardict_as_new already roll back
        // the dictionaries row + dict_words. Surface the original message.
        error(&format!("import_user_zip: SQL import failed: {}", e));
        e
    })?;

    if outcome.cancelled {
        info(&format!(
            "import_user_zip: '{}' cancelled; kept {} partial entries on dict id {}",
            label, outcome.inserted, outcome.dictionary_id
        ));
    } else {
        info(&format!("import_user_zip: '{}' -> id {}", label, outcome.dictionary_id));

        // 9. Capture any bundled `res/` resources into dict_resources, keyed by
        //    the new dictionary id (stable across rename). Only on a successful
        //    import — a cancelled/0-entry import is cleaned up by the bridge.
        if let Err(e) = capture_stardict_resources(&unzipped_dir, outcome.dictionary_id) {
            // Non-fatal: the dictionary still imported; resources just won't render.
            error(&format!("import_user_zip: capturing res/ failed: {}", e));
        }
    }

    // tmp drops here; extracted files are deleted.
    drop(tmp);

    Ok(outcome)
}

/// Guess a resource mime type from its file extension.
fn guess_resource_mime_type(path: &Path) -> &'static str {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match extension.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "css" => "text/css",
        "js" => "application/javascript",
        "woff" | "woff2" => "font/woff",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        _ => "application/octet-stream",
    }
}

/// Detect a `res/` folder inside an extracted StarDict directory and store every
/// file in it as a `dict_resources` row keyed by `dictionary_id`. The stored
/// `resource_path` is relative to the `res/` folder (e.g. `mw-gd.css`,
/// `images/foo.png`), matching the `res/<path>` references in definition HTML.
///
/// The stored `definition_html` is NOT rewritten here — URL rewriting is
/// deferred to render time because the API port can change between runs.
fn capture_stardict_resources(unzipped_dir: &Path, dictionary_id: i32) -> Result<usize, String> {
    let res_dir = unzipped_dir.join("res");
    match res_dir.try_exists() {
        Ok(true) => {}
        Ok(false) => return Ok(0),
        Err(e) => return Err(format!("Cannot access {}: {}", res_dir.display(), e)),
    }

    let app_data = get_app_data();
    let mut count = 0usize;

    // Walk the res/ tree depth-first, storing each file with its path relative
    // to res/.
    let mut stack = vec![res_dir.clone()];
    while let Some(dir) = stack.pop() {
        let entries = std::fs::read_dir(&dir)
            .map_err(|e| format!("Failed to read {}: {}", dir.display(), e))?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let rel = match path.strip_prefix(&res_dir) {
                Ok(r) => r.to_string_lossy().replace('\\', "/"),
                Err(_) => continue,
            };
            let data = match std::fs::read(&path) {
                Ok(d) => d,
                Err(e) => {
                    error(&format!("capture_stardict_resources: read {} failed: {}", path.display(), e));
                    continue;
                }
            };
            let mime = guess_resource_mime_type(&path);
            let new_resource = crate::db::dictionaries_models::NewDictResource {
                dictionary_id,
                resource_path: &rel,
                mime_type: Some(mime),
                content_data: Some(&data),
            };
            if let Err(e) = app_data.dbm.dictionaries.create_dict_resource(&new_resource) {
                error(&format!("capture_stardict_resources: insert {} failed: {}", rel, e));
                continue;
            }
            count += 1;
        }
    }

    if count > 0 {
        info(&format!("capture_stardict_resources: stored {} resource(s) for dict id {}", count, dictionary_id));
    }
    Ok(count)
}

/// Find a StarDict directory inside an extracted archive and return both the
/// directory and the basename (stem) of the discovered `.ifo` file.
///
/// Many StarDict zips ship the files at the archive root; some wrap them in a
/// single folder. We scan both. The `.ifo` basename is whatever the archive
/// ships and need not match the user-chosen label.
fn locate_stardict_dir(extract_dir: &Path) -> Option<(std::path::PathBuf, String)> {
    if let Some(stem) = find_ifo_stem_in(extract_dir) {
        return Some((extract_dir.to_path_buf(), stem));
    }

    // One level deep.
    let entries = std::fs::read_dir(extract_dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir()
            && let Some(stem) = find_ifo_stem_in(&path) {
                return Some((path, stem));
            }
    }
    None
}

/// Return the file-stem of the first `*.ifo` in `dir`, if any.
fn find_ifo_stem_in(dir: &Path) -> Option<String> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("ifo")
            && let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                return Some(stem.to_string());
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

    let user_dicts = app_data.dbm.dictionaries.list_dictionaries(Some(true))
        .map_err(|e| format!("Failed to list user dictionaries: {}", e))?;
    let target = user_dicts.into_iter()
        .find(|d| d.id == dictionary_id)
        .ok_or_else(|| format!(
            "Dictionary id {} is not a user-imported dictionary; refusing to delete.",
            dictionary_id
        ))?;

    // Remove the dictionary's stored resources first. The FK is ON DELETE
    // CASCADE, so deleting the dictionaries row would also clear these, but we
    // delete explicitly for clarity and so it works regardless of PRAGMA
    // foreign_keys state.
    match app_data.dbm.dictionaries.delete_dict_resources(dictionary_id) {
        Ok(r) if r > 0 => info(&format!("delete_user_dictionary: removed {} resource(s) for '{}'", r, target.label)),
        Ok(_) => {}
        Err(e) => error(&format!("delete_user_dictionary: delete_dict_resources failed: {}", e)),
    }

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

    let user_dicts = app_data.dbm.dictionaries.list_dictionaries(Some(true))
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
