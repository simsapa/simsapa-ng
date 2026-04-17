# Tasks: Migration Fixes for the v0.2.0 → v0.3.0 Database Upgrade

Source PRD: [prd-upgrade-from-v0-2-0-to-v0-3-0-migration-fixes.md](./prd-upgrade-from-v0-2-0-to-v0-3-0-migration-fixes.md)

## Relevant Files

- `backend/src/app_data.rs` — `export_user_chanting_data()` (~line 2305) and `import_user_chanting_data()` (~line 2409); extend to include seeded ancestors on export, and add skip-if-exists + orphan-parent creation on import. Also `export_user_data_to_assets()` (~line 1484) for grouped-error propagation.
- `backend/src/db/chanting_export.rs` — `create_chanting_sqlite()` (~line 20); disable FK checks on the export connection, convert per-table insert loops to per-row warn-and-continue with summary counters.
- `backend/src/db/chanting_export.rs` (tests module, ~line 561 onward) — extend existing roundtrip tests to cover FK-off behaviour and per-row failure tolerance.
- `backend/src/db/appdata.rs` — existing `create_chanting_{collection,chant,section,recording}` helpers (~lines 921, 975, 1026, 1077); may add `..._exists_by_uid(&str) -> bool` sibling helpers for the import-side skip-if-exists check.
- `backend/src/lib.rs` — `check_delete_files_for_upgrade()` (~line 686); extend the deletion list to include `paths.index_dir` and update the doc comment.
- `bridges/src/sutta_bridge.rs` — `prepare_for_database_upgrade()` (~line 3338); add `export_failed` qsignal (near the existing `app_settings_reset` signal at line 321) and a new `force_database_upgrade()` qinvokable; change the export-failure path to emit instead of silently continuing.
- `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml` — add stub signal `exportFailed(reason: string)` and stub function `force_database_upgrade()` (near the existing `appSettingsReset` signal at line 29 and the `prepare_for_database_upgrade` stub at line 805) so qmllint passes.
- `assets/qml/UpdateNotificationDialog.qml` — two call sites at lines 472 and 557 must subscribe to `SuttaBridge.exportFailed` and open the new export-failure dialog. May add a new dialog Frame section (a new `dialog_type: "export_failed"`) alongside the existing "closing" dialog.
- `assets/qml/DatabaseValidationDialog.qml` — call site at line 157 (`handle_remove_all_and_redownload`) must subscribe to `SuttaBridge.exportFailed` and show the new dialog.
- `assets/qml/ExportFailedDialog.qml` (optional new file, if extracted as a reusable component) — confirmation dialog with the four buttons from §4.4 #14. If added, register in `bridges/build.rs::qml_files`.
- `bridges/build.rs` — `qml_files` list; add any newly created QML file per CLAUDE.md.
- `docs/upgrade-and-migration-flow.md` (new file) — developer-facing documentation of the upgrade/migration logic with code-location references, per §4.2 of the PRD.
- `CLAUDE.md` — reference the new docs file in the documentation index.
- `PROJECT_MAP.md` — update the "Database Upgrade Flow" section (~line 375) to reflect the new export/import behaviour, `index/` cleanup, and `export_failed` signal.

### Notes

- After every top-level task: `make build -B` and `cd backend && cargo test`. Skip `make qml-test` unless explicitly asked (per CLAUDE.md and user feedback memory).
- Do not gate integration tests behind `#[ignore]` just because they need the real appdata DB — follow existing patterns in `backend/src/db/chanting_export.rs` tests.
- Manual verification steps from PRD §9 Stages must be run by the user; Claude should not launch the GUI (per CLAUDE.md).
- Always use `try_exists()` instead of `.exists()` for file-existence checks (Android safety, per CLAUDE.md).

## Tasks

- [ ] 1.0 Chanting export: include seeded ancestors of user recordings, harden inserts with FK-off and per-row resilience, and copy audio before the sqlite build
  - [ ] 1.1 In `export_user_chanting_data()` (backend/src/app_data.rs:2305), after loading `user_recordings`, collect the distinct `section_uid` set referenced by those recordings and load the matching sections from appdata (regardless of `is_user_added`), preserving each row's original `is_user_added` flag.
  - [ ] 1.2 From that section set, collect the distinct `chant_uid` set and load the matching chants from appdata (regardless of `is_user_added`), preserving flags.
  - [ ] 1.3 From that chant set, collect the distinct `collection_uid` set and load the matching collections from appdata (regardless of `is_user_added`), preserving flags.
  - [ ] 1.4 Merge the ancestor rows with the existing `user_collections` / `user_chants` / `user_sections` vectors, deduplicating by `uid`. Preserve the original flag values for ancestor rows (do NOT flip seeded rows to `is_user_added = true`).
  - [ ] 1.5 Add an `info` log showing per-table counts: "user-added" vs "seeded ancestors pulled in" vs "total exported" (per PRD §4.5).
  - [ ] 1.6 At the start of the export (before any writes), remove any pre-existing `import-me/` directory to clear artifacts from a previous cancelled upgrade attempt. Log the deletion at `info`.
  - [ ] 1.7 Reorder the audio-copy block so it runs **before** `create_chanting_sqlite()`. Keep the existing per-recording loop and its `warn`-on-missing-file behaviour intact.
  - [ ] 1.8 In `create_chanting_sqlite()` (backend/src/db/chanting_export.rs:20), immediately after `run_pending_migrations()` execute `diesel::sql_query("PRAGMA foreign_keys = OFF").execute(&mut conn)` so FKs are disabled for the remainder of the function.
  - [ ] 1.9 In each of the four insert loops (collections, chants, sections, recordings), replace `.execute(&mut conn).with_context(...)?` with a per-row `match`: on `Err`, emit a `warn!` with the row uid and the error string, and increment a per-table failure counter; on `Ok`, increment a success counter. Continue iterating.
  - [ ] 1.10 After the loops, emit a one-line `info!` summary with the per-table insert-success and insert-failure counts.
  - [ ] 1.11 Change the function's success criteria: return `Ok(())` whenever the sqlite file was successfully created and migrations ran (even if every row failed). Only return `Err` for I/O failures (file creation, migration failure).
  - [ ] 1.12 Extend `backend/src/db/chanting_export.rs` tests: add a test that seeded ancestors passed to `create_chanting_sqlite()` round-trip with their original `is_user_added = false`; add a test that confirms one bad row (e.g. missing NOT NULL) does not abort the remaining inserts.

- [ ] 2.0 Chanting import: skip-if-exists for collections/chants/sections/recordings, and create missing parent ancestors for orphan recordings so no recording is dropped
  - [ ] 2.1 In `import_user_chanting_data()` (backend/src/app_data.rs:2409), introduce per-table counters: `inserted`, `skipped_existing`, and (for recordings) `created_parent_for_orphan`.
  - [ ] 2.2 For each **collection** read from the import DB: query the live appdata for an existing row with the same `uid`; if present, log `info("skipped existing seeded collection <uid>")` and increment `skipped_existing`, otherwise call the existing `create_chanting_collection()` and increment `inserted`.
  - [ ] 2.3 Same skip-if-exists logic for each **chant** against the `chanting_chants` table.
  - [ ] 2.4 Same skip-if-exists logic for each **section** against the `chanting_sections` table.
  - [ ] 2.5 For each **recording**: first check if a recording with the same `uid` already exists in the live DB — if so, skip (this handles the duplicate-of-seeded `chanting-rec-itipiso-reference` case). Otherwise resolve `section_uid`: if the section exists in the live DB, insert the recording as-is; if it does not, look up the section (and, transitively, its chant and collection) in the *exported* DB's rows, insert whichever ancestors are missing (bumping `created_parent_for_orphan`), then insert the recording. Never skip/drop a recording.
  - [ ] 2.6 Add private helpers on `AppdataDbHandle` or as local closures in `import_user_chanting_data()`: `chanting_collection_exists_by_uid(uid) -> bool`, `chanting_chant_exists_by_uid(uid) -> bool`, `chanting_section_exists_by_uid(uid) -> bool`, `chanting_recording_exists_by_uid(uid) -> bool`. Each uses a single `SELECT 1 ... WHERE uid = ? LIMIT 1`.
  - [ ] 2.7 Emit a final `info!` per-table summary line with the three counters (inserted / skipped-existing / created-parent-for-orphan), per PRD §4.5.
  - [ ] 2.8 Add a backend test that constructs a minimal temp-file "import" chanting sqlite containing (a) a collection/chant/section/recording whose uids already exist in a fixture live DB (must be skipped), (b) a recording whose parent section is absent from the live DB (parent section + chant + collection must be created and the recording inserted). Assert per-table counter values.

- [ ] 3.0 Index cleanup: extend `check_delete_files_for_upgrade()` to remove `paths.index_dir` when the upgrade marker is present
  - [ ] 3.1 In `check_delete_files_for_upgrade()` (backend/src/lib.rs:686), after the database-file deletion block, attempt to remove `g.paths.index_dir` via `std::fs::remove_dir_all`, guarded by `try_exists() == Ok(true)`.
  - [ ] 3.2 On success, log `info(&format!("Removed index directory: {}", g.paths.index_dir.display()))`. On removal failure, log `error(...)`. `Ok(false)` is a no-op.
  - [ ] 3.3 Update the doc comment above `check_delete_files_for_upgrade()` to list `index/` alongside the database files it removes, and note that the next asset download extracts a fresh index.
  - [ ] 3.4 Audit the repo for any other code path that writes `delete_files_for_upgrade.txt` or unconditionally removes the index: `grep` for `delete_files_for_upgrade` and for `index_dir`. Confirm in the docs file (Task 6) that the only marker-writer after this PRD will be `prepare_for_database_upgrade()` and `force_database_upgrade()` — both on the upgrade-proceed path.

- [ ] 4.0 Export-failure bridge plumbing: add `export_failed` qsignal and `force_database_upgrade()` qinvokable; stop writing marker files on export error
  - [ ] 4.1 In the `cxx_qt::bridge` block of `bridges/src/sutta_bridge.rs` (near line 321, alongside `app_settings_reset`), declare `#[qsignal] #[cxx_name = "exportFailed"] fn export_failed(self: Pin<&mut SuttaBridge>, reason: QString);`.
  - [ ] 4.2 Declare `#[qinvokable] fn force_database_upgrade(self: &SuttaBridge);` in the same bridge block (near line 739 alongside the `prepare_for_database_upgrade` declaration).
  - [ ] 4.3 Implement `pub fn force_database_upgrade(&self)` in the `impl` block. It writes both `delete_files_for_upgrade.txt` and `auto_start_download.txt` unconditionally (identical to the existing marker-writing block in `prepare_for_database_upgrade()`), but does **not** re-run the export and does **not** delete `import-me/` (partial export data for non-failed categories must be preserved for import).
  - [ ] 4.4 Refactor `export_user_data_to_assets()` (backend/src/app_data.rs:1484) to collect per-category errors instead of short-circuiting on the first `Err`. Minimum categories: `app_settings`, `download_languages`, `books`, `bookmarks`, `chanting`, `legacy_bridge`. Return type becomes something like `Result<(), Vec<(String /* category */, String /* error_message */)>>` (or a struct with the same shape) so callers see every failure.
  - [ ] 4.5 Refactor `prepare_for_database_upgrade()` (bridges/src/sutta_bridge.rs:3338):
    - If the new export result is `Ok`: proceed to write both marker files (existing behaviour).
    - If it is `Err(categories_errors)`: format a multi-line human-readable string (e.g. one line per category with "CATEGORY: message"), emit `export_failed(QString::from(&reason))` and return **without** writing either marker file. Log at `error` with the same reason, per PRD §4.5.
  - [ ] 4.6 In `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml`, add the matching stub signal `signal exportFailed(reason: string)` (near line 29 alongside `signal appSettingsReset()`) and stub function `function force_database_upgrade() { console.log("force_database_upgrade()"); }` (near the existing `prepare_for_database_upgrade` stub at line 805), so qmllint passes.
  - [ ] 4.7 Run `make build -B` to confirm cxx-qt regeneration succeeds and the bridge compiles.

- [ ] 5.0 Export-failure dialog UI: add a confirmation dialog with four buttons and wire the three existing call sites of `prepare_for_database_upgrade()` to it
  - [ ] 5.1 Add an export-failure dialog — either as a new `dialog_type: "export_failed"` Frame section inside `UpdateNotificationDialog.qml` (keeping one file), or as a new standalone `assets/qml/ExportFailedDialog.qml`. Use the styling of existing dialog sections for consistency. If a new file is added, register it in `bridges/build.rs::qml_files` per CLAUDE.md.
  - [ ] 5.2 Dialog content per PRD §4.4 #16: header "Errors during user data export", body "Exporting user data before database upgrade reported errors:", followed by a read-only scrollable `TextArea` containing the error reason text (one category per line). Truncate extremely long messages if needed.
  - [ ] 5.3 Four buttons per PRD §4.4 #14, with `Cancel Upgrade` as the default ENTER/focus target:
    - **Cancel Upgrade** — closes the dialog, does nothing further. Old DB and any partial `import-me/` remain on disk.
    - **Copy Error Message** — `Clipboard.setText(reason_text)` with the full error string.
    - **Copy Exported Path** — `Clipboard.setText(...)` with the `import-me/` absolute path (fetch via a small bridge getter, or pass it alongside the `exportFailed` reason).
    - **Continue Anyway** — calls `SuttaBridge.force_database_upgrade()` and transitions to the existing "closing" dialog state (quit-and-restart prompt).
  - [ ] 5.4 In `UpdateNotificationDialog.qml`, attach a `Connections { target: SuttaBridge }` block handling `onExportFailed: (reason) => { ... }` that opens the export-failure dialog (or sets `root.dialog_type = "export_failed"` and stashes `reason`). Do NOT transition to "closing" eagerly in the Yes/Download Now button handlers (lines 472 and 557) — instead, wait for either `exportFailed` (show dialog) or a short timer / a paired success signal before transitioning. Default approach: add a second qsignal `exportSucceeded()` in Task 4 and transition to "closing" on that signal for deterministic behaviour.
  - [ ] 5.5 In `DatabaseValidationDialog.qml` (line 157), add the same `Connections { target: SuttaBridge }` handler: when `exportFailed` fires, open the new dialog instead of `remove_all_success_dialog`. On success (no signal or `exportSucceeded`), keep opening `remove_all_success_dialog` as today.
  - [ ] 5.6 If Task 5.4 added `exportSucceeded`, update both the bridge (Task 4.1) and `SuttaBridge.qml` stub (Task 4.6) accordingly. Keep the bridge surface area minimal — only add `exportSucceeded` if it simplifies the QML control flow.
  - [ ] 5.7 Run `make build -B` to verify the QML and bridge changes compile and resolve against each other.

- [ ] 6.0 Documentation: write `docs/upgrade-and-migration-flow.md`, reference it from `CLAUDE.md`, and update `PROJECT_MAP.md`
  - [ ] 6.1 Create `docs/upgrade-and-migration-flow.md` covering the full round-trip: (1) trigger (`prepare_for_database_upgrade` and the three QML call sites), (2) export pipeline (`export_user_data_to_assets` → per-category sub-exports including `export_user_chanting_data` and seeded-ancestor inclusion), (3) chanting export internals (`create_chanting_sqlite`, FK-off, per-row resilience, audio-copy ordering), (4) marker files, (5) restart path (`check_delete_files_for_upgrade` and `index/` deletion), (6) download and extract, (7) import pipeline (`import_user_data_from_assets`, skip-if-exists, orphan-parent creation in `import_user_chanting_data`), (8) failure path (`export_failed` signal, `force_database_upgrade` qinvokable, the export-failure dialog). Each section should cite concrete code locations.
  - [ ] 6.2 Add a bullet to `CLAUDE.md`'s documentation area (or the existing "Documentation is in the `docs/` folder" line) pointing to the new `docs/upgrade-and-migration-flow.md`.
  - [ ] 6.3 Update `PROJECT_MAP.md` "Database Upgrade Flow" (around line 375): mention seeded-ancestor inclusion in chanting export, the skip-if-exists + orphan-parent import strategy, `index/` deletion in `check_delete_files_for_upgrade`, and the `export_failed` signal + `force_database_upgrade` qinvokable. Link to `docs/upgrade-and-migration-flow.md`.
  - [ ] 6.4 Final verification: run `make build -B` and `cd backend && cargo test` to confirm the whole tree compiles and tests pass. Skip `make qml-test` unless explicitly requested.
