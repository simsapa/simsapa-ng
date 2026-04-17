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

- [x] 1.0 Chanting export: include seeded ancestors of user recordings, harden inserts with FK-off and per-row resilience, and copy audio before the sqlite build
  - [x] 1.1 In `export_user_chanting_data()` (backend/src/app_data.rs:2305), after loading `user_recordings`, collect the distinct `section_uid` set referenced by those recordings and load the matching sections from appdata (regardless of `is_user_added`), preserving each row's original `is_user_added` flag.
  - [x] 1.2 From that section set, collect the distinct `chant_uid` set and load the matching chants from appdata (regardless of `is_user_added`), preserving flags.
  - [x] 1.3 From that chant set, collect the distinct `collection_uid` set and load the matching collections from appdata (regardless of `is_user_added`), preserving flags.
  - [x] 1.4 Merge the ancestor rows with the existing `user_collections` / `user_chants` / `user_sections` vectors, deduplicating by `uid`. Preserve the original flag values for ancestor rows (do NOT flip seeded rows to `is_user_added = true`).
  - [x] 1.5 Add an `info` log showing per-table counts: "user-added" vs "seeded ancestors pulled in" vs "total exported" (per PRD §4.5).
  - [x] 1.6 At the start of the export (before any writes), remove any pre-existing `import-me/` directory to clear artifacts from a previous cancelled upgrade attempt. Log the deletion at `info`.
  - [x] 1.7 Reorder the audio-copy block so it runs **before** `create_chanting_sqlite()`. Keep the existing per-recording loop and its `warn`-on-missing-file behaviour intact.
  - [x] 1.8 In `create_chanting_sqlite()` (backend/src/db/chanting_export.rs:20), immediately after `run_pending_migrations()` execute `diesel::sql_query("PRAGMA foreign_keys = OFF").execute(&mut conn)` so FKs are disabled for the remainder of the function.
  - [x] 1.9 In each of the four insert loops (collections, chants, sections, recordings), replace `.execute(&mut conn).with_context(...)?` with a per-row `match`: on `Err`, emit a `warn!` with the row uid and the error string, and increment a per-table failure counter; on `Ok`, increment a success counter. Continue iterating.
  - [x] 1.10 After the loops, emit a one-line `info!` summary with the per-table insert-success and insert-failure counts.
  - [x] 1.11 Change the function's success criteria: return `Ok(())` whenever the sqlite file was successfully created and migrations ran (even if every row failed). Only return `Err` for I/O failures (file creation, migration failure).
  - [x] 1.12 Extend `backend/src/db/chanting_export.rs` tests: add a test that seeded ancestors passed to `create_chanting_sqlite()` round-trip with their original `is_user_added = false`; add a test that confirms one bad row (e.g. missing NOT NULL) does not abort the remaining inserts.

- [x] 2.0 Chanting import: skip-if-exists for collections/chants/sections/recordings, and create missing parent ancestors for orphan recordings so no recording is dropped
  - [x] 2.1 In `import_user_chanting_data()` (backend/src/app_data.rs:2409), introduce per-table counters: `inserted`, `skipped_existing`, and (for recordings) `created_parent_for_orphan`.
  - [x] 2.2 For each **collection** read from the import DB: query the live appdata for an existing row with the same `uid`; if present, log `info("skipped existing seeded collection <uid>")` and increment `skipped_existing`, otherwise call the existing `create_chanting_collection()` and increment `inserted`.
  - [x] 2.3 Same skip-if-exists logic for each **chant** against the `chanting_chants` table.
  - [x] 2.4 Same skip-if-exists logic for each **section** against the `chanting_sections` table.
  - [x] 2.5 For each **recording**: first check if a recording with the same `uid` already exists in the live DB — if so, skip (this handles the duplicate-of-seeded `chanting-rec-itipiso-reference` case). Otherwise resolve `section_uid`: if the section exists in the live DB, insert the recording as-is; if it does not, look up the section (and, transitively, its chant and collection) in the *exported* DB's rows, insert whichever ancestors are missing (bumping `created_parent_for_orphan`), then insert the recording. Never skip/drop a recording.
  - [x] 2.6 Add private helpers on `AppdataDbHandle` or as local closures in `import_user_chanting_data()`: `chanting_collection_exists_by_uid(uid) -> bool`, `chanting_chant_exists_by_uid(uid) -> bool`, `chanting_section_exists_by_uid(uid) -> bool`, `chanting_recording_exists_by_uid(uid) -> bool`. Each uses a single `SELECT 1 ... WHERE uid = ? LIMIT 1`.
  - [x] 2.7 Emit a final `info!` per-table summary line with the three counters (inserted / skipped-existing / created-parent-for-orphan), per PRD §4.5.
  - [x] 2.8 Add a backend test that constructs a minimal temp-file "import" chanting sqlite containing (a) a collection/chant/section/recording whose uids already exist in a fixture live DB (must be skipped), (b) a recording whose parent section is absent from the live DB (parent section + chant + collection must be created and the recording inserted). Assert per-table counter values.

- [x] 3.0 Index cleanup: extend `check_delete_files_for_upgrade()` to remove `paths.index_dir` when the upgrade marker is present
  - [x] 3.1 In `check_delete_files_for_upgrade()` (backend/src/lib.rs:686), after the database-file deletion block, attempt to remove `g.paths.index_dir` via `std::fs::remove_dir_all`, guarded by `try_exists() == Ok(true)`.
  - [x] 3.2 On success, log `info(&format!("Removed index directory: {}", g.paths.index_dir.display()))`. On removal failure, log `error(...)`. `Ok(false)` is a no-op.
  - [x] 3.3 Update the doc comment above `check_delete_files_for_upgrade()` to list `index/` alongside the database files it removes, and note that the next asset download extracts a fresh index.
  - [x] 3.4 Audit the repo for any other code path that writes `delete_files_for_upgrade.txt` or unconditionally removes the index: `grep` for `delete_files_for_upgrade` and for `index_dir`. Confirm in the docs file (Task 6) that the only marker-writer after this PRD will be `prepare_for_database_upgrade()` and `force_database_upgrade()` — both on the upgrade-proceed path.

- [x] 4.0 Export-failure bridge plumbing: add `export_failed` qsignal and `force_database_upgrade()` qinvokable; stop writing marker files on export error
  - [x] 4.1 In the `cxx_qt::bridge` block of `bridges/src/sutta_bridge.rs` (near line 321, alongside `app_settings_reset`), declare `#[qsignal] #[cxx_name = "exportFailed"] fn export_failed(self: Pin<&mut SuttaBridge>, reason: QString);`.
  - [x] 4.2 Declare `#[qinvokable] fn force_database_upgrade(self: &SuttaBridge);` in the same bridge block (near line 739 alongside the `prepare_for_database_upgrade` declaration).
  - [x] 4.3 Implement `pub fn force_database_upgrade(&self)` in the `impl` block. It writes both `delete_files_for_upgrade.txt` and `auto_start_download.txt` unconditionally (identical to the existing marker-writing block in `prepare_for_database_upgrade()`), but does **not** re-run the export and does **not** delete `import-me/` (partial export data for non-failed categories must be preserved for import).
  - [x] 4.4 Refactor `export_user_data_to_assets()` (backend/src/app_data.rs:1484) to collect per-category errors instead of short-circuiting on the first `Err`. Minimum categories: `app_settings`, `download_languages`, `books`, `bookmarks`, `chanting`, `legacy_bridge`. Return type becomes something like `Result<(), Vec<(String /* category */, String /* error_message */)>>` (or a struct with the same shape) so callers see every failure.
  - [x] 4.5 Refactor `prepare_for_database_upgrade()` (bridges/src/sutta_bridge.rs:3338):
    - If the new export result is `Ok`: proceed to write both marker files (existing behaviour).
    - If it is `Err(categories_errors)`: format a multi-line human-readable string (e.g. one line per category with "CATEGORY: message"), emit `export_failed(QString::from(&reason))` and return **without** writing either marker file. Log at `error` with the same reason, per PRD §4.5.
  - [x] 4.6 In `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml`, add the matching stub signal `signal exportFailed(reason: string)` (near line 29 alongside `signal appSettingsReset()`) and stub function `function force_database_upgrade() { console.log("force_database_upgrade()"); }` (near the existing `prepare_for_database_upgrade` stub at line 805), so qmllint passes.
  - [x] 4.7 Run `make build -B` to confirm cxx-qt regeneration succeeds and the bridge compiles.

- [x] 5.0 Export-failure dialog UI: add a confirmation dialog with four buttons and wire the three existing call sites of `prepare_for_database_upgrade()` to it
  - [x] 5.1 Add an export-failure dialog — either as a new `dialog_type: "export_failed"` Frame section inside `UpdateNotificationDialog.qml` (keeping one file), or as a new standalone `assets/qml/ExportFailedDialog.qml`. Use the styling of existing dialog sections for consistency. If a new file is added, register it in `bridges/build.rs::qml_files` per CLAUDE.md.
  - [x] 5.2 Dialog content per PRD §4.4 #16: header "Errors during user data export", body "Exporting user data before database upgrade reported errors:", followed by a read-only scrollable `TextArea` containing the error reason text (one category per line). Truncate extremely long messages if needed.
  - [x] 5.3 Four buttons per PRD §4.4 #14, with `Cancel Upgrade` as the default ENTER/focus target:
    - **Cancel Upgrade** — closes the dialog, does nothing further. Old DB and any partial `import-me/` remain on disk.
    - **Copy Error Message** — `Clipboard.setText(reason_text)` with the full error string.
    - **Copy Exported Path** — `Clipboard.setText(...)` with the `import-me/` absolute path (fetch via a small bridge getter, or pass it alongside the `exportFailed` reason).
    - **Continue Anyway** — calls `SuttaBridge.force_database_upgrade()` and transitions to the existing "closing" dialog state (quit-and-restart prompt).
  - [x] 5.4 In `UpdateNotificationDialog.qml`, attach a `Connections { target: SuttaBridge }` block handling `onExportFailed: (reason) => { ... }` that opens the export-failure dialog (or sets `root.dialog_type = "export_failed"` and stashes `reason`). Do NOT transition to "closing" eagerly in the Yes/Download Now button handlers (lines 472 and 557) — instead, wait for either `exportFailed` (show dialog) or a short timer / a paired success signal before transitioning. Default approach: add a second qsignal `exportSucceeded()` in Task 4 and transition to "closing" on that signal for deterministic behaviour.
  - [x] 5.5 In `DatabaseValidationDialog.qml` (line 157), add the same `Connections { target: SuttaBridge }` handler: when `exportFailed` fires, open the new dialog instead of `remove_all_success_dialog`. On success (no signal or `exportSucceeded`), keep opening `remove_all_success_dialog` as today.
  - [x] 5.6 If Task 5.4 added `exportSucceeded`, update both the bridge (Task 4.1) and `SuttaBridge.qml` stub (Task 4.6) accordingly. Keep the bridge surface area minimal — only add `exportSucceeded` if it simplifies the QML control flow.
  - [x] 5.7 Run `make build -B` to verify the QML and bridge changes compile and resolve against each other.

- [x] 6.0 Post-implementation fixes from PRD §11 (dialog dedup, processing UI, marker I/O, orphan recovery, export orphan tolerance, bypass logging)
  - [x] 6.1 Fix duplicate dialog reactions (PRD §11.1):
    - [x] 6.1.1 Add `property bool upgrade_initiated_here: false` to `UpdateNotificationDialog.qml` and to `DatabaseValidationDialog.qml`.
    - [x] 6.1.2 At every call site of `SuttaBridge.prepare_for_database_upgrade()` (UpdateNotificationDialog.qml ~line 479 and ~563; DatabaseValidationDialog.qml `handle_remove_all_and_redownload`), set `upgrade_initiated_here = true` immediately before the bridge call.
    - [x] 6.1.3 In each dialog's `Connections { target: SuttaBridge }` block, prepend `if (!root.upgrade_initiated_here) return;` to both `onExportFailed` and `onExportSucceeded`, and reset `upgrade_initiated_here = false` once the dialog has acted on the signal (or on cancel / dialog close).
    - [x] 6.1.4 Add a QML comment above each `Connections` block: "Both UpdateNotificationDialog and DatabaseValidationDialog are siblings in SuttaSearchWindow and both receive SuttaBridge signals. The `upgrade_initiated_here` guard ensures only the initiator reacts."

  - [x] 6.2 Fix UI dead-time during async export (PRD §11.2):
    - [x] 6.2.1 Add `property bool export_in_progress: false` to `UpdateNotificationDialog.qml` and `DatabaseValidationDialog.qml`.
    - [x] 6.2.2 Update the three buttons that call `prepare_for_database_upgrade()` (UpdateNotificationDialog.qml ~line 479 "Yes / Download Now", UpdateNotificationDialog.qml ~line 563 "Download Now", DatabaseValidationDialog.qml "Remove All and Re-Download"): set `export_in_progress = true` on click (alongside `upgrade_initiated_here = true`); bind `enabled: !root.export_in_progress`; bind `text:` to show "Exporting user data…" when `export_in_progress`, otherwise the original label.
    - [x] 6.2.3 In each dialog's `Connections` block, set `export_in_progress = false` inside both `onExportFailed` and `onExportSucceeded` after the guard check.
    - [x] 6.2.4 Add a QML comment explaining the reason: "Export is async (runs on the bridge thread and emits `exportSucceeded` / `exportFailed`). Disable the trigger button and relabel it so the user cannot re-trigger and knows work is underway."

  - [x] 6.3 Fix silent marker-file I/O failure (PRD §11.3):
    - [x] 6.3.1 Change `write_upgrade_marker_files()` in `bridges/src/sutta_bridge.rs` (~line 40) to return `Result<(), Vec<(String, String)>>` where the vector contains `(category, error_message)` entries. Categories: `marker_delete_files`, `marker_auto_start_download`.
    - [x] 6.3.2 Add a doc comment to the helper explaining: "a silently-missing marker file produces a silently-failed upgrade on the next start (the old DB is not deleted and the new download is not auto-started), so marker-write failures must surface to the UI."
    - [x] 6.3.3 Update `prepare_for_database_upgrade()` (happy path): after a successful export, call the helper; if it returns `Err`, format the marker-category errors the same way export errors are formatted and emit `export_failed(reason)` instead of `export_succeeded()`.
    - [x] 6.3.4 Update `force_database_upgrade()` similarly: on marker-write error, emit `export_failed(reason)` so the user sees the failure instead of restarting into a no-op upgrade.
    - [x] 6.3.5 Verify with `make build -B`; no QML changes are required (existing `onExportFailed` handlers already cover this case).

  - [x] 6.4 Fix orphan-repair still dropping recordings (PRD §11.4):
    - [x] 6.4.1 In `import_user_chanting_data()` (backend/src/app_data.rs), remove the `continue` on `!orphan_repair_ok`. Every recording must proceed to the insert step.
    - [x] 6.4.2 Add a helper (local closure or private method) `ensure_orphan_recovery_ancestors(rec: &ChantingRecording, exported_sections, exported_chants, exported_collections, counters) -> String` returning the final `section_uid` to use for the recording (normally the original `rec.section_uid`, possibly a synthetic one if nothing else is recoverable).
    - [x] 6.4.3 The helper creates missing ancestors in order collection → chant → section, preferring exported-DB metadata when available and otherwise synthesising placeholders with deterministic uids: `col-orphan-recovery`, `chant-orphan-recovery`, and a section whose uid is the original `rec.section_uid` (so the recording's FK still resolves). Each synthetic row sets `is_user_added = true` and `sort_index = 9999`; collection `language = "pali"`, section `content_pali = ""`.
    - [x] 6.4.4 Before creating any synthetic ancestor, skip-if-exists on the synthetic uid so repeated imports converge on the same placeholders.
    - [x] 6.4.5 Bump the existing `rec_orphan_parents_created` counter for each ancestor (real or synthetic) created. Log at `warn` listing the recording uid, the missing ancestor uid(s), and the chosen placeholder uid(s).
    - [x] 6.4.6 Add a Rust comment above the helper explaining: "PRD §10.2 / §11.4 require that no recording is ever dropped. When the exported DB lacks an ancestor, synthesise a deterministic placeholder so the audio file stays linked to a visible section in the new DB."

  - [x] 6.5 Fix user-added chants/sections dropped on export and extend orphan tolerance to the export side (PRD §11.5):
    - [x] 6.5.1 In `export_user_chanting_data()` change the early-return condition from `user_collections.is_empty() && user_recordings.is_empty()` to `user_collections.is_empty() && user_chants.is_empty() && user_sections.is_empty() && user_recordings.is_empty()`.
    - [x] 6.5.2 Extend the ancestor-collection logic so the section-uid set is seeded from `user_recordings.section_uid` AND `user_sections.uid` is used to trace up to `user_sections.chant_uid` (pulling ancestor chants), and `user_chants.collection_uid` is used to pull ancestor collections — covering user-added rows that have **no** user recording beneath them.
    - [x] 6.5.3 After loading the ancestor rows from appdata, for any ancestor uid referenced by a user row but still missing in the live DB, synthesise a placeholder using the same deterministic rule as Task 6.4 and include it in the exported sqlite. Log at `warn`. This guarantees the exported sqlite always has a complete FK subgraph regardless of live-DB damage.
    - [x] 6.5.4 Update the "exporting chanting data" info log to also cover seeded-but-missing placeholders pulled in on the export side.

  - [x] 6.6 Log the errors bypassed by `force_database_upgrade` (PRD §11.6):
    - [x] 6.6.1 In `bridges/src/sutta_bridge.rs`, add `last_export_failure: std::sync::Mutex<Option<String>>` (or similar) to the `SuttaBridge` module state (matching the existing `RESULTS_PAGE_CACHE` pattern if a free static is simpler than a struct field).
    - [x] 6.6.2 In `prepare_for_database_upgrade()`: on export failure, set `last_export_failure = Some(reason.clone())` right before emitting `export_failed`; on success (after marker writes also succeed), clear it to `None`.
    - [x] 6.6.3 In `force_database_upgrade()`: read the stored reason; if present, log it at `error` prefixed with `force_database_upgrade(): user is bypassing the following export errors:`. Then proceed to write the markers (respecting the Task 6.3 error-surfacing behaviour).
    - [x] 6.6.4 Clear `last_export_failure` after `force_database_upgrade()` completes successfully.

  - [x] 6.7 Extend backend tests:
    - [x] 6.7.1 Add a test in `backend/tests/test_chanting_import.rs` that constructs an import sqlite containing a recording whose `section_uid` is absent from both the live DB and the exported ancestor rows. Assert that after import the recording row exists in the live DB and that synthetic `col-orphan-recovery` / `chant-orphan-recovery` and a section with the recording's original `section_uid` are present, all with `is_user_added = true`.
    - [x] 6.7.2 Add a test covering a user-added section without user-added collections/chants/recordings, confirming the new early-return condition in `export_user_chanting_data()` no longer drops it (may live in `backend/src/db/chanting_export.rs` tests or a new integration test, following the pattern of existing tests).

  - [x] 6.8 Final verification: `make build -B` and `cd backend && cargo test`. Skip `make qml-test` unless explicitly requested. Confirm no regressions in existing chanting export/import tests.

- [ ] 7.0 Documentation: write `docs/upgrade-and-migration-flow.md`, reference it from `CLAUDE.md`, and update `PROJECT_MAP.md`
  - [ ] 7.1 Create `docs/upgrade-and-migration-flow.md` covering the full round-trip: (1) trigger (`prepare_for_database_upgrade` and the three QML call sites, including the `upgrade_initiated_here` and `export_in_progress` dialog state from Task 6.1/6.2), (2) export pipeline (`export_user_data_to_assets` → per-category sub-exports including `export_user_chanting_data` and seeded-ancestor inclusion with export-side orphan synthesis from Task 6.5), (3) chanting export internals (`create_chanting_sqlite`, FK-off, per-row resilience, audio-copy ordering), (4) marker files (including the Task 6.3 error-surfacing behaviour), (5) restart path (`check_delete_files_for_upgrade` and `index/` deletion), (6) download and extract, (7) import pipeline (`import_user_data_from_assets`, skip-if-exists, orphan-parent creation and synthetic placeholder recovery in `import_user_chanting_data` from Task 6.4), (8) failure path (`export_failed` / `export_succeeded` signals, `force_database_upgrade` qinvokable including the bypass logging from Task 6.6, the export-failure dialog). Each section should cite concrete code locations.
  - [ ] 7.2 Add a bullet to `CLAUDE.md`'s documentation area (or the existing "Documentation is in the `docs/` folder" line) pointing to the new `docs/upgrade-and-migration-flow.md`.
  - [ ] 7.3 Update `PROJECT_MAP.md` "Database Upgrade Flow" (around line 375): mention seeded-ancestor inclusion in chanting export, the skip-if-exists + orphan-parent import strategy with synthetic placeholder recovery, `index/` deletion in `check_delete_files_for_upgrade`, the `export_failed` / `export_succeeded` signals + `force_database_upgrade` qinvokable, and the dialog-dedup + processing-state conventions. Link to `docs/upgrade-and-migration-flow.md`.
  - [ ] 7.4 Final verification: run `make build -B` and `cd backend && cargo test` to confirm the whole tree compiles and tests pass. Skip `make qml-test` unless explicitly requested.
