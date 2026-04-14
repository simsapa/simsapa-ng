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

- [ ] 1.0 Schema markers and bootstrap audit for `is_user_added`
  - [ ] 1.1 Create a new Diesel migration `backend/migrations/appdata/<date>_add_is_user_added/` with `up.sql` adding `is_user_added BOOLEAN NOT NULL DEFAULT 1` to `books`, `bookmark_folders`, and `bookmark_items` (and any other table identified during audit that holds user-generated rows but lacks the column); `down.sql` drops them.
  - [ ] 1.2 In `backend/src/db/appdata_schema.rs`, add `is_user_added -> Bool` to the table! macros for `books`, `bookmark_folders`, `bookmark_items`. Update the corresponding structs in `backend/src/db/appdata_models.rs` (queryable + insertable variants).
  - [ ] 1.3 Confirm the chanting-tables migration (`2026-03-24-000000_create_chanting_tables/up.sql`) already defines `is_user_added BOOLEAN NOT NULL DEFAULT 1`. If the default is `0`, add a follow-up migration that alters the three chanting tables to `DEFAULT 1`.
  - [ ] 1.4 Grep `cli/src/bootstrap/` for every insert targeting tables that now carry `is_user_added` (chanting tables, `books`, bookmark tables, and any user-prompt/AI-model table found). For each insert, add an explicit `is_user_added: false` so no seeded row relies on the default.
  - [ ] 1.5 Set `is_user_added: false` for any library-import bootstrap paths (`cli/src/bootstrap/library_imports.rs` or similar) that pre-seed books.
  - [ ] 1.6 Regenerate the bootstrap `appdata.sqlite3` via the CLI bootstrap tool; confirm spot-checks: seeded chanting / books rows have `is_user_added = 0`; a runtime insert (e.g. a UI-added bookmark) has `is_user_added = 1`.

- [ ] 2.0 Route `app_settings` reads/writes to `appdata`
  - [ ] 2.1 In `backend/src/app_data.rs`, change the `app_settings_cache` initializer in `AppData::new()` (the `dbm.userdata.get_app_settings()` call around line 37) to use `dbm.appdata`.
  - [ ] 2.2 Flip every `app_settings` getter/setter in `backend/src/app_data.rs` (`set_include_cst_*`, `set_include_ms_*`, `set_show_bottom_footnotes`, `set_open_find_in_sutta_results`, `set_sutta_language_filter_key`, `set_mobile_top_bar_margin_*`, and the parallel getters) from `self.dbm.userdata.get_conn()` to `self.dbm.appdata.get_conn()`.
  - [ ] 2.3 In `import_app_settings_json()` (around line 1774), change the `userdata.get_conn()` call to `appdata.get_conn()`.
  - [ ] 2.4 Ensure `backend/src/db/appdata.rs::get_app_settings()` still reads the single `app_settings` row as it does today — no code change expected, just verify the single source now being `appdata`.

- [ ] 3.0 Route remaining user-data reads/writes to `appdata` and rename user-books export
  - [ ] 3.1 Flip every remaining `self.dbm.userdata.get_conn()` site in `backend/src/app_data.rs` to `self.dbm.appdata.get_conn()`. Use grep to confirm none remain outside the legacy-bridge code that will be added in 5.0.
  - [ ] 3.2 In `export_user_data_to_assets()` and its helpers (`export_user_bookmarks`, `export_user_chanting_data`, the user-books export), rename the user-books export file from `appdata.sqlite3` to `appdata-books.sqlite3`. Update the path string in both the creation and (where present) any log messages.
  - [ ] 3.3 In `import_user_data_from_assets()` and its helpers (`import_user_books`, `import_user_bookmarks`, `import_user_chanting_data`), update the expected filename to `appdata-books.sqlite3` and flip the target connection to `appdata`.
  - [ ] 3.4 Update `backend/tests/test_search.rs` (and any other backend test) to drop the `userdata` handle — use only `dbm.appdata` / the real appdata DB path.

- [ ] 4.0 Update the normal migration export/import flow to the single-DB model
  - [ ] 4.1 Update every `SELECT` in `export_user_data_to_assets()` helpers to read from `appdata` and filter by `is_user_added.eq(true)`: bookmarks (both `bookmark_folders` and `bookmark_items` if both carry the flag; otherwise whichever is user-scoped), user books, user chanting rows (already filtered).
  - [ ] 4.2 Confirm `app_settings` export still writes the full settings blob to `app_settings.json` without filtering.
  - [ ] 4.3 In `bridges/src/asset_manager.rs`, rename `import_suttas_lang_to_userdata` → `import_suttas_lang_to_appdata`; change the target connection/DB path inside to `appdata`; update `PROJECT_MAP.md` references.
  - [ ] 4.4 In `assets/qml/DownloadAppdataWindow.qml`, update call sites invoking the renamed bridge method.
  - [ ] 4.5 In `assets/qml/com/profoundlabs/simsapa/` qmllint stubs for `AssetManager` / `SuttaBridge` (as applicable), rename the stub function to match.
  - [ ] 4.6 Confirm `download_languages.txt` generation reads language codes from `appdata` now that languages import into it (still filtering out `en` and `pli`).

- [ ] 5.0 Legacy one-shot upgrade bridge for alpha testers (`userdata.sqlite3` → `appdata.sqlite3`)
  - [ ] 5.1 In `backend/src/app_data.rs`, add a helper `legacy_userdata_exists() -> bool` using `try_exists()` against `g.paths.app_assets_dir.join("userdata.sqlite3")`.
  - [ ] 5.2 Extend `export_user_data_to_assets()` so that when the legacy file exists: open it with Diesel/`SqliteConnection::establish(...)`, then export bookmarks, chanting rows (`is_user_added = true`), user books (if any are in userdata), and app_settings from the legacy DB into the standard `import-me/` per-table files — using the same writer functions but pointed at the legacy connection.
  - [ ] 5.3 Additionally, copy the whole `userdata.sqlite3` file to `import-me/legacy-userdata.sqlite3` (`std::fs::copy`) as a safety net.
  - [ ] 5.4 Ensure `download_languages.txt` is derived from the legacy userdata if that's the source (today's behavior) — or from appdata otherwise. Document the branch in a comment.
  - [ ] 5.5 Update the `delete_files_for_upgrade.txt` marker writer in `bridges/src/sutta_bridge.rs::prepare_for_database_upgrade()` so it lists `userdata.sqlite3` in addition to `appdata.sqlite3` and existing dictionary/DPD files.
  - [ ] 5.6 Update `cpp/gui.cpp::check_delete_files_for_upgrade()` (or the Rust function it calls in `backend/src/lib.rs`) to delete `userdata.sqlite3` when listed in the marker file, using `try_exists()` before removal.
  - [ ] 5.7 Extend `import_user_data_from_assets()` with a defensive tail pass: if `import-me/legacy-userdata.sqlite3` exists, open it, and for each user-data table whose dedicated export file was absent or empty during this run, re-import any rows with `is_user_added = true` (or, for `app_settings`, the row). Keep the existing "cleanup: remove import-me/" step unchanged — it will also delete the legacy file.
  - [ ] 5.8 Add tracing/logging at each branch of the legacy path (detected-legacy, exported-from-legacy, defensive-tail-ran) so the user can verify behavior during manual upgrade testing.

- [ ] 6.0 Remove the `userdata` handle and path fields
  - [ ] 6.1 In `backend/src/db/mod.rs`: delete the `userdata: AppdataDbHandle` field on `DatabaseHandle` (or equivalent struct), the `initialize_userdata()` function, and the `userdata_exists` branch in the init function.
  - [ ] 6.2 In `backend/src/lib.rs`: delete `userdata_db_path`, `userdata_abs_path`, and `userdata_database_url` from `AppGlobalPaths`, plus the construction block in the init function. Keep `app_assets_dir` since the legacy bridge in 5.0 still computes the userdata path locally (one line, `app_assets_dir.join("userdata.sqlite3")`).
  - [ ] 6.3 Run `cargo build` and fix every resulting compile error by either removing the dead code or redirecting to `appdata`.
  - [ ] 6.4 Grep the Rust source for any residual `userdata` identifier outside the 5.0 legacy-bridge module; remove stragglers.

- [ ] 7.0 Add "Reset Settings to Default" button in the Settings window
  - [ ] 7.1 In `backend/src/app_data.rs`, add a method `reset_app_settings_to_defaults(&self) -> Result<()>` that: (a) sets `*self.app_settings_cache.write()` to `AppSettings::default()`, (b) serializes the defaults to JSON, (c) writes the JSON into the `app_settings` row (`key = "app_settings"`) in `appdata`, (d) logs success.
  - [ ] 7.2 In `bridges/src/sutta_bridge.rs`, add a `qinvokable` method `reset_app_settings_to_defaults(self: Pin<&mut Self>)` on `SuttaBridge` that calls the new AppData method and emits a signal (e.g. `app_settings_reset()`) so QML bindings refresh.
  - [ ] 7.3 In `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml`, add the matching stub function and signal declaration for qmllint.
  - [ ] 7.4 In `assets/qml/AppSettingsWindow.qml`, add a "Reset Settings to Default" `Button` (placed at the bottom of the settings list, separated from other controls). Clicking opens a `Dialog` with message "Reset all app settings to their default values? This cannot be undone." and Ok/Cancel buttons.
  - [ ] 7.5 Wire the dialog's accepted signal to `SuttaBridge.reset_app_settings_to_defaults()`. On the new `app_settings_reset` signal, re-read each setting from the bridge so currently-visible controls reflect the new defaults without an app restart.
  - [ ] 7.6 If `AppSettingsWindow.qml` is not yet listed in `bridges/build.rs::qml_files`, add it (per CLAUDE.md).

- [ ] 8.0 Startup cleanup, UI text updates, and documentation
  - [ ] 8.1 In the startup path (`backend/src/lib.rs` init or `cpp/gui.cpp` after `init_app_data()`), add a silent cleanup: if `app_assets_dir/userdata.sqlite3` exists via `try_exists()` AND there is no pending `import-me/` folder, delete it and log. This handles the case where the legacy bridge already ran but a stale file remains.
  - [ ] 8.2 Update `assets/qml/DatabaseValidationDialog.qml` text to refer to "appdata" or a neutral term (e.g. "application database") wherever it currently says "userdata".
  - [ ] 8.3 Audit `assets/qml/SuttaSearchWindow.qml` for its `userdata` reference (flagged in initial grep) and update accordingly.
  - [ ] 8.4 Update `PROJECT_MAP.md`: rewrite the "Database Upgrade Flow" section to describe the single-DB model + one-shot legacy bridge; remove `userdata.sqlite3` bullets; update the `import_suttas_lang_to_*` function name; remove the "Language Downloads: Downloads suttas_lang_{lang}.tar.bz2 files and imports into userdata.sqlite3" line.
  - [ ] 8.5 Update `docs/windows-user-data-paths.md` and `docs/language-download-implementation.md` to reflect the single-database architecture and describe the one-shot alpha-upgrade bridge.
  - [ ] 8.6 Final grep sweep: `grep -rn userdata` in Rust/C++/QML/docs must return only results inside the 5.0 legacy-bridge module and the doc sections that describe the bridge. Remove any other stragglers.
