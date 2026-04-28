# Tasks — StarDict Dictionary Import

Source PRD: [prd-stardict-dictionary-import.md](./prd-stardict-dictionary-import.md)

## Relevant Files

- `backend/migrations/dictionaries/<new-timestamp>_add_user_dict_columns/up.sql` — New Diesel migration adding `is_user_imported BOOLEAN NOT NULL DEFAULT 0`, `language TEXT NULL`, `indexed_at TIMESTAMP NULL` to the `dictionaries` table.
- `backend/migrations/dictionaries/<new-timestamp>_add_user_dict_columns/down.sql` — Reverse migration.
- `backend/src/db/dictionaries_schema.rs` — Update Diesel schema to include the three new columns.
- `backend/src/db/dictionaries_models.rs` — Update `Dictionary` and `NewDictionary` structs; add helpers.
- `backend/src/db/dictionaries.rs` — Add: `list_user_dictionaries()`, `count_words_for_dictionary()`, `rename_dictionary_label()`, `set_indexed_at(id, ts)`, `list_dictionaries_needing_index()`, `is_label_taken_by_shipped()`.
- `backend/src/stardict_parse.rs` — Refactor `import_stardict_as_new()` to accept `is_user_imported`, `language`, optional progress callback; SQL-only — no FTS5 / Tantivy writes.
- `backend/src/search/indexer.rs` — Add `index_dict_words_into_dict_index()`, `delete_from_dict_index_by_source_uid()`, `list_orphan_source_uids_in_dict_index()`. Use `source_uid` term for filtering (per `build_dict_schema`).
- `backend/src/search/fts5_dict.rs` (or wherever the existing dictionary FTS5 helpers live) — Add `insert_dict_words_into_fts5()`, `delete_from_fts5_by_source_uid()`, `list_orphan_source_uids_in_fts5()`.
- `backend/src/dict_index_reconcile.rs` — **NEW** Startup reconciliation pass: drops orphan FTS5 + Tantivy entries, indexes any `dictionaries` row with `indexed_at IS NULL`, exposes a progress callback for the startup window.
- `backend/src/app_data.rs` — Add `export_user_dictionaries(import_dir)` (writes `import-me/user_dictionaries.sqlite3`) and `import_user_dictionaries(import_dir)` (reads it back, re-keys ids, sets `indexed_at = NULL`), following the existing `export_user_chanting_data` pattern. Wire them into `export_user_data_to_assets()` and the matching importer entry point. Confirm/ensure the dictionaries-DB replace step is gated on a successful export.
- `backend/src/dictionary_manager_core.rs` — **NEW** Backend orchestration layer used by the bridge: `import_user_zip()`, `delete_user_dictionary()`, `rename_user_dictionary()`, `validate_label()`. Holds the single `Mutex<()>` that serialises import / rename / delete.
- `backend/src/search/searcher.rs` and `backend/src/query_task.rs` — Accept a `UserDictFilter` and add a `source_uid IN (...)` constraint to the dictionary query path.
- `backend/src/app_settings.rs` — Add per-user-dict enabled-state map (`dict_search.user_dict_enabled.<label>`).
- `bridges/src/dictionary_manager.rs` — **NEW** Rust bridge exposing import/list/rename/delete + per-dict enabled-state + progress signals.
- `bridges/src/sutta_bridge.rs` — Extend the dictionary search call to accept the user-dict filter (or add a sibling method). Update qmllint stub in `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml`.
- `bridges/src/lib.rs` — Register the new bridge module.
- `bridges/build.rs` — Register the new Rust file under `rust_files` and add new QML files under `qml_files`.
- `assets/qml/com/profoundlabs/simsapa/DictionaryManager.qml` — **NEW** qmllint type stub.
- `assets/qml/com/profoundlabs/simsapa/qmldir` — Declare the new bridge type.
- `assets/qml/DictionariesWindow.qml` — **NEW** ApplicationWindow modelled on `SuttaLanguagesWindow.qml`.
- `assets/qml/DictionaryImportDialog.qml` — **NEW** dialog asking for label + language; validates label and warns on unknown tokenizer language.
- `assets/qml/DictionaryEditDialog.qml` — **NEW** edit-label dialog with re-index-on-restart warning.
- `assets/qml/DictionaryListItem.qml` — **NEW** row component for the imported-dictionary list.
- `assets/qml/SearchBarInput.qml` — Modify `advanced_options_row` in place: wrap existing children under a chevron-collapsible "Filters" sub-section; add a chevron-collapsible "Dictionaries" sub-section that mounts the new panel below.
- `assets/qml/DictionarySearchDictionariesPanel.qml` — **NEW** collapsible "Dictionaries" content panel mounted inside `SearchBarInput.qml`'s `advanced_options_row`.
- `assets/qml/DictionaryIndexProgressWindow.qml` — **NEW** modal startup window shown while the reconciliation pass runs.
- `assets/qml/SuttaSearchWindow.qml` — Add `Windows > Dictionaries…` menu item.
- `assets/qml/main.qml` (or whichever launches `SuttaSearchWindow`) — Show `DictionaryIndexProgressWindow.qml` first when the backend reports work to do.
- `backend/src/theme_colors_light.json`, `backend/src/theme_colors_dark.json` — Add a light-blue background colour key for active dictionary rows.
- `backend/src/theme_colors.rs` — Expose the new colour key to QML through the existing channel.
- `backend/src/db/dictionaries.rs` (tests) — Rust unit tests for new DB helpers.
- `backend/src/dict_index_reconcile.rs` (tests) — Tests for orphan-detection + idempotent re-indexing.
- `backend/src/stardict_parse.rs` (tests) — Test the SQL-only import path.
- `backend/tests/dictionary_manager_integration.rs` — Integration test for the bridge round-trip (import → list → rename → delete) followed by simulated startup reconciliation.

### Notes

- All filesystem existence checks must use `try_exists()` per CLAUDE.md (Android safety).
- Any new QML files must be added to `bridges/build.rs` `qml_files` list.
- New Rust bridge modules must be added to `bridges/build.rs` `rust_files` and have a `qmllint` stub + `qmldir` entry.
- Per project memory, run `make build -B` (not direct cmake) after each top-level task; don't run `make qml-test` unless explicitly asked; only run tests after the full top-level task is done.
- Per project memory, do not use bulk `sed` to rename symbols — use targeted Edit calls.
- After each top-level task the project must compile cleanly with `make build -B`.

## Tasks

> Architecture note: All FTS5 / Tantivy index writes happen exclusively in the **startup reconciliation pass** (task 3.0), shown via a modal startup progress window before `SuttaSearchWindow` opens. Inside the running app, import / rename / delete only touch SQL, then ask the user to restart. This avoids contention with the live searcher and dodges Tantivy `IndexWriter` directory-lock issues. The release-upgrade story uses the existing `app_data.rs::export_user_data_to_assets()` → `import-me/user_dictionaries.sqlite3` → re-import pattern (task 5.0), mirroring how chanting / books / bookmarks are round-tripped. The source `.zip` is not archived; SQL is the source of truth.

- [ ] 1.0 Database & migration — add `is_user_imported`, `language`, `indexed_at` columns and supporting CRUD
  - [ ] 1.1 Create new migration directory under `backend/migrations/dictionaries/<timestamp>_add_user_dict_columns/` with `up.sql` adding three columns (`is_user_imported BOOLEAN NOT NULL DEFAULT 0`, `language TEXT NULL`, `indexed_at TIMESTAMP NULL`) and `down.sql` dropping them.
  - [ ] 1.2 Update `backend/src/db/dictionaries_schema.rs` to add `is_user_imported -> Bool`, `language -> Nullable<Text>`, `indexed_at -> Nullable<Timestamp>` to the `dictionaries` table macro.
  - [ ] 1.3 Update `Dictionary` struct in `backend/src/db/dictionaries_models.rs` (and `NewDictionary`) to include the three new fields.
  - [ ] 1.4 Update existing `create_dictionary` callers (e.g. `import_stardict_as_new` in `stardict_parse.rs`, bootstrap callers in `cli/src/bootstrap/`) so they keep compiling with the new fields. Bootstrap callers leave `language = None`, `indexed_at = None`, `is_user_imported = false`.
  - [ ] 1.5 Add `pub fn list_user_dictionaries(&self) -> Result<Vec<Dictionary>>` filtering `is_user_imported = true`, ordered by `label`.
  - [ ] 1.6 Add `pub fn count_words_for_dictionary(&self, dictionary_id: i32) -> Result<i64>` and `pub fn list_dictionaries_needing_index(&self) -> Result<Vec<Dictionary>>` (returns user-imported rows where `indexed_at IS NULL`).
  - [ ] 1.7 Add `pub fn rename_dictionary_label(&self, old_label: &str, new_label: &str) -> Result<()>` that, in a single transaction: updates `dictionaries.label`, updates `dict_words.dict_label`, rewrites `dict_words.uid` from `<word>/<old_label>` to `<word>/<new_label>`, and sets `dictionaries.indexed_at = NULL`.
  - [ ] 1.8 Add `pub fn set_indexed_at(&self, dictionary_id: i32, ts: NaiveDateTime) -> Result<()>` and `pub fn is_label_taken_by_shipped(&self, label: &str) -> Result<bool>` (true if a row exists with that label and `is_user_imported = false`).
  - [ ] 1.9 Confirm migrations auto-apply at runtime via the existing Diesel runner in `backend/src/db/mod.rs` for the dictionaries connection.
  - [ ] 1.10 Verify `make build -B` succeeds.

- [ ] 2.0 SQL-only import / rename / delete pipeline (no in-app indexing) + serialisation mutex
  - [ ] 2.1 Add a `StardictImportProgress` enum (stages: `Extracting`, `Parsing`, `InsertingWords { done, total }`, `Done`, `Failed { msg }`) in `backend/src/stardict_parse.rs`. **No `IndexingFts5` / `IndexingTantivy` stages** — those are emitted only by the startup reconciliation pass.
  - [ ] 2.2 Refactor `import_stardict_as_new` to accept `is_user_imported: bool`, `language: Option<&str>`, and an optional `progress` callback. Set `indexed_at = None` on the new row. Emit progress at chunk boundaries.
  - [ ] 2.3 Create `backend/src/dictionary_manager_core.rs` with a single static `Mutex<()>` (e.g. `DICT_MGR_LOCK`) that gates all three operations below. Each public function `try_lock` and returns a `Busy` error if another op is in progress.
  - [ ] 2.4 Implement `pub fn import_user_zip(zip_path: &Path, label: &str, lang: &str, on_progress: impl Fn(StardictImportProgress)) -> Result<i32, String>`:
        1. Verify label format + uniqueness via `is_label_taken_by_shipped` (reject built-ins) and a uniqueness check against existing user dicts (caller already handled Replace via prior `delete_user_dictionary`).
        2. Extract `.zip` to a temp dir created via `tempfile::Builder::new().prefix("simsapa-stardict-").tempdir_in(<app cache dir>)` so the path lives under a directory tolerated by Android. The temp dir auto-cleans on drop. Use `try_exists()` for path checks.
        3. Call refactored `import_stardict_as_new` with `is_user_imported = true`, `language = Some(lang)`. The temp dir is dropped (deleted) at function exit regardless of outcome.
        4. On SQL failure: cascade-delete the new dictionary row if any partial state was committed. Surface original error.
        5. On success, emit `StardictImportProgress::Done` so the bridge can fire `import_finished` and the UI can show a success message.
  - [ ] 2.5 Implement `pub fn delete_user_dictionary(dictionary_id: i32) -> Result<(), String>`:
        1. Reject if `is_user_imported = false`.
        2. Delete from SQL (cascade drops `dict_words`).
        3. Do NOT touch FTS5 / Tantivy — orphan cleanup runs on next startup (task 3.0).
  - [ ] 2.6 Implement `pub fn rename_user_dictionary(dictionary_id: i32, new_label: &str) -> Result<(), String>`:
        1. Reject if `is_user_imported = false`.
        2. Validate new label (format + unique + not built-in).
        3. Call `db::dictionaries::rename_dictionary_label` (which rewrites `dict_words.uid` / `dict_words.dict_label` and sets `indexed_at = NULL` in a single transaction).
  - [ ] 2.7 Implement `pub fn validate_label(label: &str) -> Result<(), String>` enforcing ASCII alnum + `_-` + non-empty.
  - [ ] 2.8 Verify `make build -B` succeeds.

- [ ] 3.0 Startup reconciliation pass: orphan cleanup + indexing + progress window
  - [ ] 3.1 In `backend/src/search/indexer.rs`, add `pub fn index_dict_words_into_dict_index(words: &[DictWord], lang: &str, on_progress: impl Fn(usize, usize)) -> Result<()>` mirroring the field set used by the bootstrap DPD StarDict import. Use `register_tokenizers(index, lang)` for the dict index; if `lang` is unknown, fall back to the default tokenizer and log a warning — must NOT error out. Emit progress per chunk (e.g. every 1000 words) so the startup progress bar updates smoothly for large dictionaries.
  - [ ] 3.2 Add `pub fn delete_from_dict_index_by_source_uid(label: &str) -> Result<()>` that opens the dict Tantivy index writer, deletes by `Term::from_field_text(source_uid_field, label)`, commits.
  - [ ] 3.3 Add `pub fn list_indexed_source_uids_in_dict_index() -> Result<HashSet<String>>` (used to detect orphans).
  - [ ] 3.4 In the dictionary FTS5 helper module, add `insert_dict_words_into_fts5(words)`, `delete_from_fts5_by_source_uid(label)`, `list_indexed_source_uids_in_fts5()`.
  - [ ] 3.5 Create `backend/src/dict_index_reconcile.rs` with `pub fn reconcile_dict_indexes(on_progress: impl Fn(ReconcileProgress))`:
        1. Compute orphan set: `(fts5_source_uids ∪ tantivy_source_uids) - {label of every current `dictionaries` row}`. Drop those from both indexes.
        2. For each user-imported dictionary with `indexed_at IS NULL`: load its `dict_words`, delete any existing entries by `source_uid` (idempotent re-index safety), insert into FTS5, then Tantivy, then call `set_indexed_at(now())`.
  - [ ] 3.6 Add `ReconcileProgress` enum: `DroppingOrphans { done, total }`, `IndexingDictionary { label, done, total, dict_index, dict_total }`, `Done`.
  - [ ] 3.7 Wire the reconciliation pass to run at app startup **before** `SuttaSearchWindow` opens. Add an entry-point in `backend/src/lib.rs` (or wherever startup orchestration lives) that returns whether reconciliation has work to do (so the UI knows whether to show the progress window at all). Stale Tantivy lockfile handling: if `IndexWriter::open` fails with a lock error, detect via the documented Tantivy lockfile path; if no other process holds it (single-user app), remove the lockfile and retry once. Log loudly.
  - [ ] 3.8 Create `assets/qml/DictionaryIndexProgressWindow.qml` — a modal `ApplicationWindow` with a `ProgressBar` + status `Label`, hooked to bridge signals `reconcile_progress(stage_json)` and `reconcile_finished()`. On `reconcile_finished`, close itself and let the app open `SuttaSearchWindow`.
  - [ ] 3.9 In `assets/qml/main.qml` (or the equivalent root), gate the existing `SuttaSearchWindow` instantiation: first call the bridge `reconcile_needed()`; if true, show `DictionaryIndexProgressWindow.qml` and start reconciliation; on `reconcile_finished` proceed to `SuttaSearchWindow`. If false, open `SuttaSearchWindow` immediately.
  - [ ] 3.10 Verify `make build -B` succeeds.

- [ ] 4.0 `DictionaryManager` Rust bridge + QML registration + persisted enabled-state in `app_settings`
  - [ ] 4.1 Create `bridges/src/dictionary_manager.rs` modelled on `bridges/src/storage_manager.rs`, exposing a `DictionaryManager` QObject with CXX-Qt.
  - [ ] 4.2 Bridge methods (Rust → QML): `import_zip(zip_path, label, lang) -> QString` (`"ok"` or error; runs on a worker thread, emits progress + finished signals); `list_user_dictionaries() -> QString` (JSON: `[{id, label, title, language, entry_count}]`); `rename_label(id, new_label) -> QString`; `delete_dictionary(id) -> QString`; `label_status(label) -> QString` (returns `"available" | "taken_user" | "taken_shipped" | "invalid"` — single round-trip from the import dialog); `is_known_tokenizer_lang(lang) -> bool`. All mutating ops route through `dictionary_manager_core` (which holds the serialisation mutex and rejects concurrent ops with `Busy`).
  - [ ] 4.3 Bridge signals: `import_progress(stage: QString, done: i32, total: i32)`, `import_finished(dictionary_id: i32, label: QString)`, `import_failed(message: QString)`, plus `reconcile_progress(stage: QString, done: i32, total: i32)`, `reconcile_finished()`. Use the CXX-Qt `CxxQtThread` queued-invocation pattern so signals fire on the Qt main thread from the worker threads.
  - [ ] 4.4 Per-dict enabled-state in `backend/src/app_settings.rs`: add `pub fn get_user_dict_enabled(label: &str) -> bool` (default true), `pub fn set_user_dict_enabled(label: &str, enabled: bool)`, `pub fn list_user_dict_enabled() -> HashMap<String, bool>`. Use the existing settings serialisation pattern.
  - [ ] 4.5 Bridge methods for the enabled-state map: `get_user_dict_enabled(label) -> bool`, `set_user_dict_enabled(label, enabled)`, `get_user_dict_enabled_map() -> QString`.
  - [ ] 4.6 Bridge entry-points for reconciliation: `reconcile_needed() -> bool`, `start_reconcile()` (spawns the worker thread, emits `reconcile_progress` / `reconcile_finished`).
  - [ ] 4.7 Create QML stub `assets/qml/com/profoundlabs/simsapa/DictionaryManager.qml` declaring every bridge function and signal with placeholder return values for `qmllint`.
  - [ ] 4.8 Add `DictionaryManager` to `assets/qml/com/profoundlabs/simsapa/qmldir`.
  - [ ] 4.9 Register `src/dictionary_manager.rs` in `bridges/build.rs` `rust_files`. Add all new QML files (DictionaryManager.qml, DictionariesWindow.qml, DictionaryImportDialog.qml, DictionaryEditDialog.qml, DictionaryListItem.qml, DictionarySearchFilterPanel.qml, DictionarySearchDictionariesPanel.qml, DictionaryIndexProgressWindow.qml) to `qml_files`.
  - [ ] 4.10 Register the bridge in `bridges/src/lib.rs`.
  - [ ] 4.11 Verify `make build -B` succeeds.

- [ ] 5.0 Release-upgrade round-trip: `export_user_dictionaries` + `import_user_dictionaries` in `app_data.rs` (temp-sqlite, mirrors chanting)
  > **⚠ Investigation required before implementation.** Before writing any code in 5.x, read the existing upgrade flow and confirm two things: (a) the order of operations on first launch of a new build — the dictionaries-DB export step **must** run before the shipped `dictionaries.sqlite3` is replaced, otherwise user data is lost; (b) what happens when an upgrade attempt fails mid-way: a successive upgrade attempt's `export_user_data_to_assets()` currently removes any pre-existing `import-me/` (see `app_data.rs:1555`) — if a prior `user_dictionaries.sqlite3` snapshot is sitting unconsumed, it is wiped before the next export writes a fresh one. Trace through the sequence: launch → export to `import-me/` → swap shipped DBs → import from `import-me/` → cleanup. Identify the exact code paths that swap `dictionaries.sqlite3` and gate them. If the gate is missing for the dictionaries DB, **add it as part of this task** (it may already exist for `appdata.sqlite3`). Document findings in PR notes before implementing.
  - [ ] 5.1 In `backend/src/app_data.rs`, add `fn export_user_dictionaries(&self, import_dir: &Path) -> Result<()>` modelled on `export_user_chanting_data`. Behaviour:
        1. Read all user-imported `dictionaries` rows + their `dict_words` rows from the live `dictionaries.sqlite3`.
        2. Create a fresh SQLite at `import_dir/user_dictionaries.sqlite3` with the same schema (run the dictionaries migrations against it, mirroring the chanting export pattern that creates a self-contained DB file).
        3. Insert the rows with `indexed_at` cleared to `NULL` (do not carry over).
        4. Use `try_exists()` for every path check.
  - [ ] 5.2 Wire `export_user_dictionaries` into `export_user_data_to_assets()` alongside the existing `export_user_books` / `export_user_bookmarks` / `export_user_chanting_data` calls; collect per-category errors the same way. Confirm with code inspection that this export step runs before the shipped `dictionaries.sqlite3` replace step on upgrade — if the replace happens first, user data is lost. If the existing project does not gate the dictionaries-DB replace on a successful export (today this gate exists for `appdata.sqlite3` but might not for `dictionaries.sqlite3`), add the gate as part of this task.
  - [ ] 5.3 Add `pub fn import_user_dictionaries(&self, import_dir: &Path) -> Result<()>` modelled on the chanting importer. Behaviour:
        1. Open `import_dir/user_dictionaries.sqlite3` (skip if missing).
        2. For each row in its `dictionaries` table, INSERT INTO the live `dictionaries` table (let autoincrement issue a fresh `id`); remember `old_id -> new_id` in a HashMap.
        3. For each `dict_words` row, rewrite `dictionary_id` via the HashMap and INSERT.
        4. Set `indexed_at = NULL` on every inserted dictionary row so the §4.9 reconciliation indexes them.
        5. Run all inserts inside a single transaction; on failure roll back and leave the snapshot in place for the next attempt.
        6. On success, delete `import_dir/user_dictionaries.sqlite3`.
  - [ ] 5.4 Wire `import_user_dictionaries` into the existing import-me consumer entry point (mirror how `import_user_books` / `import_user_bookmarks` / `import_user_chanting_data` are dispatched at startup).
  - [ ] 5.5 **Schema-drift safety:** the snapshot reader must not assume column order matches; SELECT explicit columns, INSERT explicit columns. If a future build adds a `dict_words` column with a NULL default, old snapshots remain importable. If a future build removes a column, the importer must tolerate the missing column (use `PRAGMA table_info` if needed). Add a comment documenting this contract.
  - [ ] 5.6 **Double-upgrade snapshot preservation.** Before `export_user_dictionaries` writes a fresh snapshot, check whether `import-me/user_dictionaries.sqlite3` already exists from a previous unconsumed upgrade attempt. If yes: do NOT overwrite — instead, fail the export step with a clear error surfaced to the user ("A previous dictionary upgrade hasn't completed; please restart the app to finish it before upgrading again.") and leave the existing snapshot intact. The user-data export framework already collects per-category errors — return one here. Verify the existing `import-me/` clear-on-startup logic at `app_data.rs:1555` does NOT remove the dictionaries snapshot specifically; if it does, exclude `user_dictionaries.sqlite3` from that clear so it survives until consumed.
  - [ ] 5.7 **`dict_words` FK audit.** Before implementing the re-key step, check `backend/src/db/dictionaries_schema.rs` for any FK columns on `dict_words` other than `dictionary_id` that point to the old DB's id space; the re-key map must cover all of them.
  - [ ] 5.8 **Snapshot file move vs copy.** When the upgrade flow consumes the snapshot, prefer `std::fs::rename` over copy-then-delete to avoid doubling disk usage during the import. (File size is acceptable — typically < 50 MB per dictionary.)
  - [ ] 5.9 Verify `make build -B` succeeds.

- [ ] 6.0 `DictionariesWindow.qml` + import / edit / delete dialogs + `Windows > Dictionaries…` menu
  - [ ] 6.1 In `assets/qml/SuttaSearchWindow.qml`, locate the `Windows` menu and add a `MenuItem` "Dictionaries…" that opens `DictionariesWindow.qml` (use the existing Loader / lazy pattern of the language window menu).
  - [ ] 6.2 Create `assets/qml/DictionariesWindow.qml` as an `ApplicationWindow`, modelled on `SuttaLanguagesWindow.qml`. Layout: header with "Import StarDict…" button; below it a `ListView` of imported dictionaries; below that an inline progress area for active import.
  - [ ] 6.3 Bind the `ListView` model to `DictionaryManager.list_user_dictionaries()` results (parse JSON in QML on `Component.onCompleted` and after `import_finished` / rename / delete). Empty-state label: "No imported dictionaries yet."
  - [ ] 6.4 Create `assets/qml/DictionaryListItem.qml` showing: title, label, language, entry count, an Edit button, and a trash icon. Wire Edit to `DictionaryEditDialog.qml`; wire trash to a `MessageDialog` confirm → `DictionaryManager.delete_dictionary(id)` → show the close-and-restart `MessageDialog` → refresh list.
  - [ ] 6.5 Create `assets/qml/DictionaryImportDialog.qml`: `FileDialog` filtered to `*.zip`. On accept, open a `Dialog` with `label` and `lang` (default `pli`) `TextField`s. Live-validate label via a single `DictionaryManager.label_status(label)` call: `"invalid"` → inline error; `"taken_shipped"` → inline error "This name is reserved by a built-in dictionary."; `"taken_user"` → on submit show `MessageDialog` offering **Replace** (calls `delete_dictionary` of the existing then proceeds) or **Cancel**; `"available"` → submit allowed.
  - [ ] 6.6 Lang-warning UX: in the import dialog, after the user enters a `lang` not recognised by `DictionaryManager.is_known_tokenizer_lang(lang)`, show an inline warning: "Unknown tokenizer language. Indexing will use the default tokenizer." Non-blocking — the user may proceed.
  - [ ] 6.7 Render the inline progress area (Rectangle + `ProgressBar` + stage label) hooked to `import_progress` / `import_finished` / `import_failed`. On `import_finished`, show a brief success indication ("Import completed") and then the close-and-restart `MessageDialog`. On `import_failed`, show the error message and clear the progress area.
  - [ ] 6.8 Create `assets/qml/DictionaryEditDialog.qml`: a `TextField` for the new label, with visible warning text "Renaming takes effect after the next app restart, when the affected entries are re-indexed in FTS5 and Tantivy. This may take some time for large dictionaries." Confirm calls `DictionaryManager.rename_label(id, new_label)` and shows the close-and-restart `MessageDialog`.
  - [ ] 6.9 Disable Import / Edit / Delete buttons whenever a `Busy` response is returned (i.e. another op is in progress); re-enable on `import_finished` / error.
  - [ ] 6.10 Verify `make build -B` succeeds.

- [ ] 7.0 Advanced search options: wrap existing inputs as collapsible "Filters" sub-section in `SearchBarInput.qml`, add new "Dictionaries" sub-section
  > Scope clarification: the advanced options row already exists in `assets/qml/SearchBarInput.qml` inside `Flow { id: advanced_options_row }`, gated by the `advanced_options_btn` toggle. This task modifies that block in place — `DictionaryTab.qml` is **not** the right file. The new "Dictionaries" UI is implemented as a separate QML component but mounted inside `advanced_options_row`.
  - [ ] 7.1 In `assets/qml/SearchBarInput.qml`, wrap the existing children of `advanced_options_row` (the per-area input RowLayouts + checkbox RowLayouts) inside a new section. Add a header `Row` with a chevron icon button + "Filters" `Label` (mirror the chevron + section pattern from `ChantingPracticeReviewWindow.qml`). Add a property `is_filters_collapsed: false`. The wrapped children bind `visible: !root.is_filters_collapsed`. Existing IDs (`nikaya_prefix_input`, `uid_prefix_input`, `uid_suffix_input`, `include_ms_mula_checkbox`, …) and their bindings stay in place — only their parent wrapper changes.
  - [ ] 7.2 Inside the same `advanced_options_row`, after the Filters wrapper, add a new section header (chevron + "Dictionaries" Label) with property `is_dictionaries_collapsed: false`. Mount a new component `DictionarySearchDictionariesPanel { visible: !root.is_dictionaries_collapsed && root.search_area === "Dictionary" }` directly below it.
  - [ ] 7.3 Create `assets/qml/DictionarySearchDictionariesPanel.qml`. Properties: `user_dicts: []` (refreshed from bridge), `locked_label: ""`. Signals: `selection_changed()` (so the parent `SearchBarInput` can call its existing `advanced_options_debounce_timer.restart()`).
  - [ ] 7.4 Inside the Dictionaries panel, render a `Repeater` over `user_dicts`. Each row is a wrapper `Rectangle` (modelled on `user_repeater` in `ChantingPracticeReviewWindow.qml`) with: `CheckBox`, dictionary title + `(label)`, and a `Button` with a lock icon (`checkable: true`).
  - [ ] 7.5 Background of the wrapper: bind `color` to a light-blue colour when `CheckBox.checked` is true, otherwise transparent/default. Add the colour key (e.g. `dict_row_active_bg`) to `backend/src/theme_colors_light.json` and `backend/src/theme_colors_dark.json`. Wire it through `backend/src/theme_colors.rs` so it reaches QML via the existing theme channel.
  - [ ] 7.6 On `CheckBox` toggle: call `DictionaryManager.set_user_dict_enabled(label, checked)`; initial `checked` from `DictionaryManager.get_user_dict_enabled(label)` (default true). Emit `selection_changed()`.
  - [ ] 7.7 Lock toggle behaviour: when a lock button is clicked: if `locked_label === ""`, set `locked_label = label`; if `locked_label === label`, set `locked_label = ""` (other rows: deactivate). Locked state must NOT mutate any persisted checkbox values. Emit `selection_changed()`.
  - [ ] 7.8 When `locked_label !== ""`, every other row's `CheckBox` and lock button must render visually disabled (`enabled: false`, dimmed opacity on the wrapper); only the locked row remains interactive. Lock state is purely transient.
  - [ ] 7.9 Empty-state: when `user_dicts.length === 0`, replace the repeater with a hint `Label` containing "No imported dictionaries yet — open Windows > Dictionaries… to import one."
  - [ ] 7.10 Refresh `user_dicts` whenever `DictionaryManager.import_finished` fires, the dictionary search area becomes active, or `DictionariesWindow` is closed.
  - [ ] 7.11 Wire `DictionarySearchDictionariesPanel.selection_changed` in `SearchBarInput.qml` to call `advanced_options_debounce_timer.restart()` so the search re-runs (matching how the existing inputs trigger searches).
  - [ ] 7.12 Verify `make build -B` succeeds.

- [ ] 8.0 Wire per-dict enabled set + transient lock state into the dictionary search query layer
  - [ ] 8.1 Add an enum `UserDictFilter { OnlyLabels(Vec<String>), IncludeLabels(Vec<String>) }` (in `backend/src/types.rs` or near the search code). Extend the dictionary search query function in `backend/src/query_task.rs` / `backend/src/search/searcher.rs` to accept `Option<UserDictFilter>`.
  - [ ] 8.2 In Tantivy query construction, when a filter is supplied add an `Occur::Must` boolean over `source_uid` `Term`s (mirror how the bold-definitions / commentary filter wires `source_uid` constraints in `searcher.rs`).
  - [ ] 8.3 In FTS5 / SQL paths, add a `WHERE dict_label IN (...)` clause when filters are supplied.
  - [ ] 8.4 Pass the active filter from QML into the query: in `DictionaryTab.qml`, before issuing a search, compute the filter — if `locked_label !== ""` → `OnlyLabels([locked_label])`; else if any user dictionary exists → `IncludeLabels(enabled_user_labels ∪ all_shipped_labels)`; else `None`. Note: shipped dictionaries are always included; the simplest implementation includes them explicitly in the `IncludeLabels` set rather than special-casing.
  - [ ] 8.5 Extend the relevant `SuttaBridge` dictionary search method (or add a sibling) to accept the filter; update its qmllint stub in `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml`.
  - [ ] 8.6 Sanity check: with no user dictionaries imported, behaviour for shipped dictionaries must be unchanged (filter must be `None` in that case).
  - [ ] 8.7 Verify `make build -B` succeeds.

- [ ] 9.0 Tests + final `make build -B` + PROJECT_MAP.md / docs update
  - [ ] 9.1 Rust unit tests in `backend/src/db/dictionaries.rs` for `list_user_dictionaries`, `count_words_for_dictionary`, `rename_dictionary_label`, `set_indexed_at`, `is_label_taken_by_shipped`, `list_dictionaries_needing_index`. Use the local appdata DB; no `#[ignore]`.
  - [ ] 9.2 Test `dictionary_manager_core::import_user_zip` against a small fixture `.zip` (commit a tiny one under `backend/tests/fixtures/`). Verify SQL row exists + `indexed_at IS NULL` + temp extraction dir is cleaned up.
  - [ ] 9.3 Test `dictionary_manager_core::delete_user_dictionary` removes SQL row (cascade `dict_words`) and refuses on shipped rows.
  - [ ] 9.4 Test `dictionary_manager_core::rename_user_dictionary` rewrites uids + sets `indexed_at = NULL` + refuses on shipped rows + refuses on built-in label collision.
  - [ ] 9.5 Test `dict_index_reconcile::reconcile_dict_indexes` end-to-end against the local appdata DB: precondition rows with `indexed_at IS NULL` are indexed; orphan FTS5 / Tantivy entries (synthesised) are removed; reruns are no-ops; interrupted runs (kill mid-loop) re-converge on next call.
  - [ ] 9.6 Test `dictionary_manager_core` mutex serialisation (concurrent calls return `Busy` for all but one).
  - [ ] 9.7 Test `app_data::export_user_dictionaries` + `import_user_dictionaries` round-trip: import → export to temp dir → wipe live DB rows → import from temp dir → assert row + `dict_words` count match (with re-keyed ids and `indexed_at = NULL`).
  - [ ] 9.8 Run `cd backend && cargo test`; pre-existing failures are ignored per memory but flag any newly introduced ones.
  - [ ] 9.9 Run `make build -B` one final time; resolve any compilation issues.
  - [ ] 9.10 Update `PROJECT_MAP.md`: new bridge, new QML files (window + dialogs + panels + index progress window), `is_user_imported` / `language` / `indexed_at` columns + migration, new `dict_index_reconcile.rs` and `dictionary_manager_core.rs` modules, `export_user_dictionaries` / `import_user_dictionaries` round-trip via `import-me/user_dictionaries.sqlite3`.
  - [ ] 9.11 Update `docs/` with a brief user-facing note covering: how to import a StarDict zip, label rules (built-in labels reserved), the close-and-restart flow, how the per-dict checkboxes + lock toggle work, what to do if the startup re-indexing window appears slow (it is one-shot per change), and what to do if a release upgrade is interrupted ("restart the app to finish processing the previous upgrade before starting a new one").
