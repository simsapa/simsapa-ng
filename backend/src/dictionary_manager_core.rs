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

use serde::Serialize;
use stardict::{self, Ifo};

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

/// Sanitise an arbitrary name into a label suggestion.
///
/// Replaces every non-`[A-Za-z0-9_-]` character with `_`, collapses runs of
/// `_`, then trims leading/trailing `_-`. Returns `""` if the result is empty
/// after sanitisation (so the dialog can leave the field blank).
fn sanitise_label_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut prev_underscore = false;
    for c in name.chars() {
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

/// Sanitise a `.zip` filename stem into a label suggestion.
pub fn suggested_label_for_zip(zip_path: &Path) -> String {
    match zip_path.file_stem().and_then(|s| s.to_str()) {
        Some(s) => sanitise_label_name(s),
        None => String::new(),
    }
}

/// Sanitise a directory name into a label suggestion.
///
/// Uses the full folder name (`file_name`), not `file_stem`, so a dotted
/// folder name isn't truncated; otherwise applies the same sanitisation as
/// [`suggested_label_for_zip`].
pub fn suggested_label_for_dir(dir_path: &Path) -> String {
    match dir_path.file_name().and_then(|s| s.to_str()) {
        Some(s) => sanitise_label_name(s),
        None => String::new(),
    }
}

/// Reject a label that is invalid, collides with a shipped source, or already
/// exists as a user dictionary. Shared by the zip and directory import paths.
///
/// Caller is responsible for resolving Replace-vs-Cancel beforehand: if the
/// label collides with an existing user dictionary, call
/// [`delete_user_dictionary`] first.
fn check_label_available(label: &str) -> Result<(), String> {
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

    Ok(())
}

/// Shared tail for the zip and directory import paths.
///
/// Locates the StarDict directory inside `search_root` (root or one level
/// deep), reads the optional `.ifo` description, runs the SQL-only import, and
/// captures any bundled `res/` resources. The `physical_stem` locates the files
/// on disk; `label` is the logical label stored on the dictionaries row and
/// used as the `{word}/{label}` uid suffix.
fn import_located_stardict(
    search_root: &Path,
    label: &str,
    lang: &str,
    on_progress: &dyn Fn(StardictImportProgress),
    cancel: &AtomicBool,
) -> Result<ImportOutcome, String> {
    // The contents may live at `search_root/` directly or one level deep inside
    // a wrapper folder; the .ifo basename is whatever the upstream archive ships
    // (e.g. `concise-eng-pli.ifo`) and need not match the user-chosen label.
    let (unzipped_dir, physical_stem) = locate_stardict_dir(search_root)
        .ok_or_else(|| "No `.ifo` file found.".to_string())?;

    let description = read_ifo_description(&unzipped_dir, &physical_stem);

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
        error(&format!("import_located_stardict: SQL import failed: {}", e));
        e
    })?;

    if outcome.cancelled {
        info(&format!(
            "import_located_stardict: '{}' cancelled; kept {} partial entries on dict id {}",
            label, outcome.inserted, outcome.dictionary_id
        ));
    } else {
        info(&format!("import_located_stardict: '{}' -> id {}", label, outcome.dictionary_id));

        // Capture any bundled `res/` resources into dict_resources, keyed by
        // the new dictionary id (stable across rename). Only on a successful
        // import — a cancelled/0-entry import is cleaned up by the bridge.
        if let Err(e) = capture_stardict_resources(&unzipped_dir, outcome.dictionary_id) {
            // Non-fatal: the dictionary still imported; resources just won't render.
            error(&format!("import_located_stardict: capturing res/ failed: {}", e));
        }

        // Refresh SQLite stats: a large StarDict import shifts the selectivity
        // of `dict_label` / `dict_words.word` enough to matter for the
        // Headword Match plan. See docs/user-data-and-sqlite-analyze.md.
        get_app_data().dbm.dictionaries.analyze("dictionaries");
    }

    Ok(outcome)
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

    check_label_available(label)?;

    // Verify the .zip exists.
    match zip_path.try_exists() {
        Ok(true) => {}
        Ok(false) => return Err(format!("Zip not found: {}", zip_path.display())),
        Err(e) => return Err(format!("Cannot access zip {}: {}", zip_path.display(), e)),
    }

    on_progress(StardictImportProgress::Extracting);

    // Extract the .zip into a temp directory under the app cache so it lives
    // somewhere Android tolerates. The TempDir auto-deletes on drop.
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

    let outcome = import_located_stardict(&extract_dir, label, lang, on_progress, cancel)?;

    // tmp drops here; extracted files are deleted.
    drop(tmp);

    Ok(outcome)
}

/// Import directly from an already-extracted StarDict directory (PRD §4.5,
/// req. 21). Skips the unzip step of [`import_user_zip`] but otherwise shares
/// the same serialisation lock, label checks, SQL import, and `res/` capture.
///
/// `dir` may be the StarDict directory itself or a parent containing it one
/// level deep (matching `locate_stardict_dir`).
pub fn import_user_dir(
    dir: &Path,
    label: &str,
    lang: &str,
    on_progress: &dyn Fn(StardictImportProgress),
    cancel: &AtomicBool,
) -> Result<ImportOutcome, String> {
    let _guard = match DICT_MGR_LOCK.try_lock() {
        Ok(g) => g,
        Err(_) => return Err(BUSY_MSG.to_string()),
    };

    check_label_available(label)?;

    // Verify the directory exists.
    match dir.try_exists() {
        Ok(true) => {}
        Ok(false) => return Err(format!("Directory not found: {}", dir.display())),
        Err(e) => return Err(format!("Cannot access directory {}: {}", dir.display(), e)),
    }

    import_located_stardict(dir, label, lang, on_progress, cancel)
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

/// Metadata for one discovered StarDict candidate, returned by [`scan_source`]
/// to populate the import checklist without committing anything to the DB.
///
/// `source_kind` is `"zip"` or `"dir"` so the QML batch driver knows whether to
/// call `import_zip` or `import_dir` for the item. Language is intentionally
/// omitted — the dialog defaults every row to `pli` (PRD §4.2 req. 7).
#[derive(Debug, Clone, Serialize)]
pub struct CandidateMeta {
    pub title: String,
    pub entry_count: i64,
    pub suggested_label: String,
    pub source_path: String,
    pub source_kind: String,
}

/// The four source kinds accepted by [`scan_source`] (PRD §4.2 req. 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanKind {
    /// A single `.zip` archive (one candidate).
    SingleZip,
    /// A single already-extracted dictionary folder (one candidate).
    SingleDir,
    /// A folder containing multiple `.zip` archives (direct children only).
    ZipFolder,
    /// A folder containing multiple extracted dictionary folders (direct
    /// children only).
    DirFolder,
}

impl ScanKind {
    /// Parse the string kind passed from QML. Returns `None` for unknown kinds.
    pub fn from_str(s: &str) -> Option<ScanKind> {
        match s {
            "single_zip" => Some(ScanKind::SingleZip),
            "single_dir" => Some(ScanKind::SingleDir),
            "zip_folder" => Some(ScanKind::ZipFolder),
            "dir_folder" => Some(ScanKind::DirFolder),
            _ => None,
        }
    }
}

/// Parse the `.ifo` + index of a located StarDict directory and return its
/// bookname title and raw index item count. Cheap — does not iterate
/// definitions (PRD §4.2 req. 6). Returns `None` if no `.ifo` is found or it
/// fails to parse.
fn probe_stardict_dir(search_root: &Path) -> Option<(String, i64)> {
    let (unzipped_dir, physical_stem) = locate_stardict_dir(search_root)?;
    let ifo_path = unzipped_dir.join(format!("{}.ifo", physical_stem));
    let ifo = Ifo::new(ifo_path.clone()).ok()?;
    let dict = stardict::no_cache(ifo_path).ok()?;
    let count = dict.idx.items.len() as i64;
    Some((ifo.bookname, count))
}

/// Probe a single `.zip` candidate by extracting it into a temp directory and
/// parsing the StarDict files inside. Returns `None` (silently skipped) if the
/// archive is not a valid StarDict.
fn probe_zip_candidate(zip_path: &Path) -> Option<CandidateMeta> {
    let cache_root = get_app_globals().paths.simsapa_dir.clone();
    let tmp = tempfile::Builder::new()
        .prefix("simsapa-stardict-probe-")
        .tempdir_in(&cache_root)
        .ok()?;
    let extract_dir = tmp.path().to_path_buf();

    let zip_file = std::fs::File::open(zip_path).ok()?;
    let mut archive = zip::ZipArchive::new(zip_file).ok()?;
    archive.extract(&extract_dir).ok()?;

    let (title, entry_count) = probe_stardict_dir(&extract_dir)?;
    Some(CandidateMeta {
        title,
        entry_count,
        suggested_label: suggested_label_for_zip(zip_path),
        source_path: zip_path.to_string_lossy().to_string(),
        source_kind: "zip".to_string(),
    })
    // tmp drops here.
}

/// Probe a single extracted-directory candidate. Returns `None` (silently
/// skipped) if no valid StarDict `.ifo` is found inside.
fn probe_dir_candidate(dir_path: &Path) -> Option<CandidateMeta> {
    let (title, entry_count) = probe_stardict_dir(dir_path)?;
    Some(CandidateMeta {
        title,
        entry_count,
        suggested_label: suggested_label_for_dir(dir_path),
        source_path: dir_path.to_string_lossy().to_string(),
        source_kind: "dir".to_string(),
    })
}

/// Discover and probe StarDict candidates for the given source kind (PRD §4.2,
/// req. 4–6). Non-StarDict files/folders are silently skipped. Does NOT mutate
/// the DB. Folder scans are non-recursive (direct children only).
pub fn scan_source(kind: ScanKind, path: &Path) -> Result<Vec<CandidateMeta>, String> {
    match path.try_exists() {
        Ok(true) => {}
        Ok(false) => return Err(format!("Path not found: {}", path.display())),
        Err(e) => return Err(format!("Cannot access {}: {}", path.display(), e)),
    }

    let mut candidates: Vec<CandidateMeta> = Vec::new();

    match kind {
        ScanKind::SingleZip => {
            if let Some(c) = probe_zip_candidate(path) {
                candidates.push(c);
            }
        }
        ScanKind::SingleDir => {
            if let Some(c) = probe_dir_candidate(path) {
                candidates.push(c);
            }
        }
        ScanKind::ZipFolder => {
            let entries = std::fs::read_dir(path)
                .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file()
                    && p.extension().and_then(|s| s.to_str()).map(|e| e.eq_ignore_ascii_case("zip")) == Some(true)
                    && let Some(c) = probe_zip_candidate(&p) {
                        candidates.push(c);
                    }
            }
        }
        ScanKind::DirFolder => {
            let entries = std::fs::read_dir(path)
                .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir()
                    && let Some(c) = probe_dir_candidate(&p) {
                        candidates.push(c);
                    }
            }
        }
    }

    // Stable ordering for predictable checklist display (folders enumerate in
    // arbitrary order across platforms).
    candidates.sort_by(|a, b| a.suggested_label.cmp(&b.suggested_label));

    Ok(candidates)
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

    // Refresh stats: a user dictionary delete cascades to thousands of
    // `dict_words` rows (and via FTS triggers, the same count from
    // `dict_words_fts`), which shifts selectivity for the Headword / Contains
    // queries. See docs/user-data-and-sqlite-analyze.md.
    app_data.dbm.dictionaries.analyze("dictionaries");

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
