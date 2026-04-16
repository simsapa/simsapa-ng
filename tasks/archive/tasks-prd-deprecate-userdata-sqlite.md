# Tasks: Deprecate `userdata.sqlite3` and Consolidate on `appdata.sqlite3`

PRD: [prd-deprecate-userdata-sqlite.md](./prd-deprecate-userdata-sqlite.md)

## Relevant Files

- `backend/src/db/appdata_schema.rs` — Diesel schema for appdata tables; needs `is_user_added` added to `books` and bookmark tables.
- `backend/src/db/appdata_models.rs` — Diesel models matching the schema; updated alongside schema changes.
- `backend/src/db/appdata.rs` — Appdata DB handle + queries; the surviving handle after removal of `userdata`.
- `backend/src/db/mod.rs` — `DatabaseHandle` struct; remove `userdata` field and `initialize_userdata()`.
- `backend/src/lib.rs` — `AppGlobalPaths`; remove `userdata_db_path`/`userdata_abs_path`/`userdata_database_url` and the `userdata` init path.
- `backend/src/app_data.rs` — Majority of `self.dbm.userdata.get_conn()` call sites; export/import user data routines (`export_user_data_to_assets`, `import_user_data_from_assets`, `import_app_settings_json`, `import_user_books`, `import_user_bookmarks`, `import_user_chanting_data`).
- `backend/src/app_settings.rs` — `AppSettings` + `impl Default for AppSettings` (used by the Reset-to-default action).
- `backend/migrations/appdata/` — New migration directory for `is_user_added` on books / bookmark tables.
- `bridges/src/sutta_bridge.rs` — `prepare_for_database_upgrade()`; new `reset_app_settings_to_defaults()` method.
- `bridges/src/asset_manager.rs` — `import_suttas_lang_to_userdata()` → rename to `import_suttas_lang_to_appdata()`.
- `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml` — qmllint stub for the new `reset_app_settings_to_defaults()`.
- `assets/qml/AppSettingsWindow.qml` — Add Reset Settings button + confirmation dialog.
- `assets/qml/DownloadAppdataWindow.qml` — References to `import_suttas_lang_to_userdata`; update to renamed function.
- `assets/qml/DatabaseValidationDialog.qml` — User-visible text referring to userdata/appdata.
- `assets/qml/SuttaSearchWindow.qml` — Contains a reference to `userdata`; audit/update.
- `cli/src/bootstrap/chanting_practice.rs` — Already writes `is_user_added: false`; verify after DEFAULT flip.
- `cli/src/bootstrap/appdata.rs`, `cli/src/bootstrap/library_imports.rs`, and sibling bootstrap files — audit any inserts into tables that now carry `is_user_added` and explicitly set `false`.
- `cpp/gui.cpp` — `check_delete_files_for_upgrade()`, `import_user_data_after_upgrade()` wiring; check for userdata path references.
- `backend/tests/test_search.rs` — Uses userdata handle; update to appdata-only.
- `backend/tests/test_chanting_crud.rs` — Uses `is_user_added`; ensure still passes after DEFAULT flip.
- `docs/windows-user-data-paths.md` — Update.
- `docs/language-download-implementation.md` — Update.
- `PROJECT_MAP.md` — Update the "Database Upgrade Flow" section and any userdata references.

### Notes

- After every top-level task: `make build -B` (per CLAUDE.md — always `make build -B`, not direct cmake) and `cd backend && cargo test`. Skip `make qml-test` unless explicitly asked.
- Do not run backend tests between sub-tasks — only at the end of each top-level task (per memory: feedback_skip_tests_between_subtasks).
- Do not gate new/modified tests behind `#[ignore]` just because they need the real appdata DB (per memory: feedback_local_integration_tests). The real DB path is `/home/gambhiro/prods/apps/simsapa-ng-project/bootstrap-assets-resources/dist/simsapa-ng/app-assets/appdata.sqlite3`.
- Use `try_exists()` (not `.exists()`) for every filesystem existence check touched during this work (per CLAUDE.md Android guidance).
- Manual verification only for upgrade round-trip and Reset Settings (PRD §4.7, §4.8 item 34).

## Tasks

- [x] 1.0 Schema markers and bootstrap audit for `is_user_added`
  - [x] 1.1 Create a new Diesel migration `backend/migrations/appdata/<date>_add_is_user_added/` with `up.sql` adding `is_user_added BOOLEAN NOT NULL DEFAULT 1` to `books`, `bookmark_folders`, and `bookmark_items` (and any other table identified during audit that holds user-generated rows but lacks the column); `down.sql` drops them.
  - [x] 1.2 In `backend/src/db/appdata_schema.rs`, add `is_user_added -> Bool` to the table! macros for `books`, `bookmark_folders`, `bookmark_items`. Update the corresponding structs in `backend/src/db/appdata_models.rs` (queryable + insertable variants).
  - [x] 1.3 Confirm the chanting-tables migration (`2026-03-24-000000_create_chanting_tables/up.sql`) already defines `is_user_added BOOLEAN NOT NULL DEFAULT 1`. If the default is `0`, add a follow-up migration that alters the three chanting tables to `DEFAULT 1`. (Follow-up migration `2026-04-14-000001_chanting_is_user_added_default_true` created; not wired into the runtime `upgrade_appdata_schema` idempotent path because the rebuild isn't idempotent — functionally a no-op since all Diesel inserts set `is_user_added` explicitly. Will apply on a fresh diesel-CLI bootstrap.)
  - [x] 1.4 Grep `cli/src/bootstrap/` for every insert targeting tables that now carry `is_user_added` (chanting tables, `books`, bookmark tables, and any user-prompt/AI-model table found). For each insert, add an explicit `is_user_added: false` so no seeded row relies on the default. (Chanting bootstrap already sets `is_user_added: false`. No bookmark or user-prompt/AI-model tables are seeded by bootstrap. Books handled in 1.5.)
  - [x] 1.5 Set `is_user_added: false` for any library-import bootstrap paths (`cli/src/bootstrap/library_imports.rs` or similar) that pre-seed books. (After each `import_*_to_db` the bootstrap now flips `books.is_user_added = false` via an UPDATE.)
  - [x] 1.6 Regenerate the bootstrap `appdata.sqlite3` via the CLI bootstrap tool; confirm spot-checks: seeded chanting / books rows have `is_user_added = 0`; a runtime insert (e.g. a UI-added bookmark) has `is_user_added = 1`. (User regenerated. Verified: 10 seeded books, 2/2/2 chanting collections/chants/sections all `is_user_added = 0`; column default is 1 for runtime inserts.)

- [x] 2.0 Route `app_settings` reads/writes to `appdata`
  - [x] 2.1 Changed `app_settings_cache` initializer in `AppData::new()` to use `dbm.appdata`.
  - [x] 2.2 Flipped every `app_settings` getter/setter from `self.dbm.userdata.get_conn()` to `self.dbm.appdata.get_conn()` (via replace_all).
  - [x] 2.3 `import_app_settings_json()` flipped to `appdata.get_conn()`.
  - [x] 2.4 Verified `appdata.rs::get_app_settings()` reads correctly — no code change needed.

- [x] 3.0 Route remaining user-data reads/writes to `appdata` and rename user-books export
  - [x] 3.1 All `self.dbm.userdata.get_conn()` sites flipped to `appdata` (confirmed by grep).
  - [x] 3.2 Renamed user-books export file from `appdata.sqlite3` to `appdata-books.sqlite3`.
  - [x] 3.3 `import_user_books` updated to look for `appdata-books.sqlite3`; bookmark/chanting importers already use `appdata`.
  - [x] 3.4 Tests don't reference `dbm.userdata` (confirmed by grep). `userdata_db_path` struct fields in test helpers remain until task 6.0 removes them from `AppGlobalPaths`.

- [x] 4.0 Update the normal migration export/import flow to the single-DB model
  - [x] 4.1 `export_user_books` now filters by `is_user_added.eq(true)` (replaces the UID blacklist). `export_user_bookmarks` filters folders and items by `is_user_added.eq(true)`. Chanting export was already filtered.
  - [x] 4.2 `export_app_settings_json()` writes the full cached settings blob — verified.
  - [x] 4.3 `import_suttas_lang_to_appdata` (in `backend/src/asset_helpers.rs`) was already renamed and targets appdata.
  - [x] 4.4 `DownloadAppdataWindow.qml` already calls `import_suttas_lang_to_appdata`.
  - [x] 4.5 qmllint stub in `AssetManager.qml` already matches the renamed function.
  - [x] 4.6 `download_languages.txt` is derived from `dbm.get_sutta_languages()`, which reads from appdata.

- [x] 5.0 Legacy one-shot upgrade bridge for alpha testers (`userdata.sqlite3` → `appdata.sqlite3`)
  - [x] 5.1 Added `legacy_userdata_exists()` and `legacy_userdata_path()` helpers in `app_data.rs` using `try_exists()`.
  - [x] 5.2 Added `export_from_legacy_userdata()`: applies `upgrade_appdata_schema` to the copy so Diesel models line up, extracts `app_settings.json` from the legacy row, and aliases the migrated copy under `appdata-bookmarks.sqlite3` / `appdata-books.sqlite3` / `appdata-chanting.sqlite3` (only when not already present) so the standard per-table importers pick them up. Runs at the end of `export_user_data_to_assets()`.
  - [x] 5.3 Copies the whole `userdata.sqlite3` to `import-me/legacy-userdata.sqlite3` as safety net.
  - [x] 5.4 `download_languages.txt` is derived from `dbm.get_sutta_languages()` (appdata only); the legacy userdata never held sutta tables, so no branch is needed.
  - [x] 5.5 Marker file is an empty signal (no file list in it); file list lives in `check_delete_files_for_upgrade()`. No change needed in `prepare_for_database_upgrade()`.
  - [x] 5.6 `backend/src/lib.rs::check_delete_files_for_upgrade()` already deletes `userdata.sqlite3` (via `g.paths.userdata_db_path`) using `try_exists()`. No change needed until task 6.0 removes that path field; this file-deletion will move to a hardcoded path at that point.
  - [x] 5.7 `import_user_data_from_assets()` now runs a defensive tail pass: if `legacy-userdata.sqlite3` exists after the standard importers, `defensive_reapply_legacy_app_settings()` re-reads the legacy `app_settings` row and writes it into appdata (guards against silent JSON-extraction failure). Bookmark/book/chanting tables are already covered by the file-alias copies in 5.2.
  - [x] 5.8 Added `info`/`error` tracing at each branch: detected-legacy, safety-copy-made, app_settings-extracted, per-table aliases, defensive-tail-ran, tail re-applied.

- [x] 6.0 Remove the `userdata` handle and path fields
  - [x] 6.1 Removed `userdata: AppdataDbHandle` from `DbManager`, `initialize_userdata()`, `insert_default_settings()`, `run_appdata_migrations()` helper, and the `userdata_exists` branch in `DbManager::new()`. Also fixed pre-existing overlooked `dbm.userdata` sites in `bridges/src/sutta_bridge.rs` (validation dialog + common-words json) and `backend/src/db/mod.rs::get_theme_name` + free `get_app_settings()`. Rewired `reset_userdata_database()` (called from `DatabaseValidationDialog.qml`) to call a new `AppdataDbHandle::reset_app_settings_to_defaults()` helper — no longer deletes/recreates a DB file, just upserts defaults into the appdata `app_settings` row. Updated stale bootstrap comment in `cli/src/bootstrap/appdata.rs:49`. Build passes.
  - [x] 6.2 Removed `userdata_db_path` / `userdata_abs_path` / `userdata_database_url` from `AppGlobalPaths` and the corresponding construction in `with_simsapa_dir()`. `ensure_no_empty_db_files()` and `check_delete_files_for_upgrade()` now compute the legacy userdata path inline via `app_assets_dir.join("userdata.sqlite3")` (same approach as the 5.0 legacy bridge). `userdata_first_query()` in `sutta_bridge.rs` now validates the appdata file. Removed obsolete fields from `test_search.rs` fixture builders. Build + test compile green.
  - [x] 6.3 `cargo build` is green. All compile errors caused by removing the `userdata` field / paths were redirected (get_theme_name → appdata, free get_app_settings → appdata, sutta_bridge validation + common_words → appdata, reset_userdata_database now upserts defaults via new `AppdataDbHandle::reset_app_settings_to_defaults()`). Dead-code helpers `initialize_userdata`, `insert_default_settings`, `run_appdata_migrations` removed.
  - [x] 6.4 Grep audit done. Remaining Rust `userdata` references are: (a) the §5.0 legacy-bridge module in `app_data.rs:1501-1838` (legitimate, task 5.0 scope); (b) legacy startup cleanup in `lib.rs:654,680,704,707` (legitimate, handles stale on-disk `userdata.sqlite3` via `app_assets_dir.join(...)`); (c) QML-facing bridge symbols `userdata_first_query` / `reset_userdata_database` in `sutta_bridge.rs:319,322,1155-1219` — deliberately deferred to task 8.2 because they are bound to the `DatabaseValidationDialog.qml` 2-track UI (userdata_failed / has_userdata_failure / get_userdata_message + "Userdata" section) which needs a coordinated rewrite. Fixed the stale `types.rs:125` comment ("appdata or userdata" → "appdata, dictionaries, or dpd").

- [x] 7.0 Add "Reset Settings to Default" button in the Settings window
  - [x] 7.1 Added `AppData::reset_app_settings_to_defaults()` that updates `app_settings_cache` in place and delegates the DB upsert to `AppdataDbHandle::reset_app_settings_to_defaults()` (added earlier in 6.1). Logs success on completion.
  - [x] 7.2 Added `#[qinvokable] reset_app_settings_to_defaults(self: Pin<&mut SuttaBridge>) -> bool` plus a new `#[qsignal] app_settings_reset` (cxx_name `appSettingsReset`). The invokable calls `AppData::reset_app_settings_to_defaults()` and, on success, emits `app_settings_reset()` to trigger QML refresh.
  - [x] 7.3 Added matching `function reset_app_settings_to_defaults(): bool` stub and `signal appSettingsReset()` in `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml`.
  - [x] 7.4 Added a "Reset Settings to Default" `Button` at the bottom-left of `AppSettingsWindow.qml` (same footer row as Close). Clicking opens a confirmation `Dialog` with the required prompt and Ok/Cancel buttons. Added a secondary error dialog for the failure path.
  - [x] 7.5 Wired the dialog's `onAccepted` to `SuttaBridge.reset_app_settings_to_defaults()`. Extracted the existing initial-load logic into a new `reload_settings_from_backend()` function and called it from both `Component.onCompleted` and a new `Connections { target: SuttaBridge; function onAppSettingsReset() { ... } }` — so every visible control re-reads defaults without an app restart.
  - [x] 7.6 `AppSettingsWindow.qml` was already listed in `bridges/build.rs::qml_files` (line 48). No change needed.

- [x] 8.0 Startup cleanup, UI text updates, and documentation
  - [x] 8.1 Added `cleanup_stale_legacy_userdata()` FFI in `backend/src/lib.rs` + extern + call in `cpp/gui.cpp` after `init_app_data()` / `import_user_data_after_upgrade()`. Uses `try_exists()` and skips when `import-me/` is pending.
  - [x] 8.2 `assets/qml/DatabaseValidationDialog.qml` now says "App Settings" / "application database" wherever it previously said "userdata". Reset button renamed to "Reset App Settings"; warning softened.
  - [x] 8.3 Cascaded symbol rename: `userdata_first_query` → `app_settings_first_query`, `reset_userdata_database` → `reset_app_settings_database` across `bridges/src/sutta_bridge.rs` (CXX block + impl + log strings + emitted QString "app_settings"), `SuttaBridge.qml` stubs, `DatabaseValidationDialog.qml` (properties, `expected_databases`, keys, handler), and `SuttaSearchWindow.qml:998`.
  - [x] 8.4 `PROJECT_MAP.md`: rewrote "Database Upgrade Flow" for the single-DB model, added a "One-Shot Legacy Userdata Bridge" section, and replaced the `import_suttas_lang_to_userdata`/`userdata.sqlite3` bullets.
  - [x] 8.5 Updated `docs/windows-user-data-paths.md` (directory listing + verification checklist) and `docs/language-download-implementation.md` (function/variable names + new "Single-Database Architecture" note).
  - [x] 8.6 Final grep: remaining `userdata` hits are only in (a) `backend/src/app_data.rs` §5.0 legacy bridge, (b) `backend/src/lib.rs` + `cpp/gui.cpp` stale-file cleanup, (c) docs describing the bridge, (d) the PRD/task files themselves, (e) archived PRDs under `tasks/archive/`. Also cleaned the stale comment in `simsapa-installer.iss`. `make build -B` and `cargo test` green (pre-existing `test_database_comparison` sutta_range_group failures unrelated).
