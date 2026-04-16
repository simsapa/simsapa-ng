# PRD: Deprecate `userdata.sqlite3` and Consolidate on `appdata.sqlite3`

## 1. Introduction / Overview

Simsapa currently ships two SQLite databases that share a single schema:

- `appdata.sqlite3` — the bootstrapped content database (suttas, dictionaries, core data) plus user-imported ebooks.
- `userdata.sqlite3` — originally intended to hold user-generated data (app settings, bookmarks, chanting recordings metadata, custom chants/collections/sections, etc.).

In practice the split has created more complexity than value. Most code paths now operate on `appdata` while `userdata` is primarily used for `app_settings` writes and a portion of the chanting/bookmarks data. Having two databases with the same schema complicates reads, writes, migrations, and the database-upgrade flow (which already has to export/re-import user rows out of `userdata`).

This PRD defines a refactor that removes `userdata.sqlite3` entirely. All user-generated rows will live in `appdata.sqlite3`. Rows that represent user-generated data will be distinguished either by an existing `is_user_added` boolean (chanting tables, and any new table added specifically for user data) or by definition (all `app_settings` rows are user-modified, and "custom languages" are defined as any language code other than `en` and `pli`).

## 2. Goals

1. Remove all runtime use of `userdata.sqlite3` from the Rust backend, bridges, and QML.
2. Route all existing and future user-generated data writes to `appdata.sqlite3`.
3. Preserve the existing database-upgrade flow (`export_user_data_to_assets()` → fresh download → `import_user_data_from_assets()`), adapted to a single database.
4. Preserve the existing sutta-language re-download model: on upgrade, the list of user-downloaded language codes (everything except `en`/`pli`) is captured, and after the new `appdata.sqlite3` is downloaded, those language databases are re-downloaded and imported into `appdata.sqlite3`.
5. Retire the `userdata.sqlite3` file from disk after consolidation — no reads, no fallback, no legacy shim.
6. Give users a one-click "Reset Settings to Default" action in the Settings window.

## 3. User Stories

- As a Simsapa user, when I upgrade to a Simsapa version that uses the consolidated database, my bookmarks, chanting collections/chants/sections, my user-added chant recordings, my app settings, my imported ebooks, and my downloaded sutta languages remain available with no manual action required.
- As a developer, when I add code that reads or writes user data, I only need to reason about a single database handle (`dbm.appdata`).
- As a developer, when I bump the app-data schema version, the existing upgrade flow carries user data across the upgrade as it does today, without having to juggle two databases.
- As a Simsapa user, if I have made changes to the app settings that I want to undo, I can click a "Reset Settings to Default" button in the Settings window and have every setting return to its default value.

## 4. Functional Requirements

### 4.1 Database handle consolidation

1. The backend must expose a single `appdata` database handle. The `userdata` field on the DB manager (`backend/src/db/mod.rs`) must be removed.
2. Every call site that currently uses `self.dbm.userdata.get_conn()` must be changed to `self.dbm.appdata.get_conn()`. This includes (non-exhaustive, grep for current sites in `backend/src/app_data.rs`):
   - All `app_settings` getters/setters (`set_include_cst_*`, `set_show_bottom_footnotes`, `set_sutta_language_filter_key`, `set_mobile_top_bar_margin_*`, etc.).
   - The `import_app_settings_json` flow.
   - All bookmark, chanting, and other user-data read/write paths currently pointing at `userdata`.
3. `backend/src/lib.rs` must stop constructing `userdata_db_path`, `userdata_abs_path`, and `userdata_database_url` on `AppGlobalPaths`.
4. `initialize_userdata()` and any `userdata_exists` checks in `backend/src/db/mod.rs` must be removed.

### 4.2 Identifying user-generated rows in the consolidated database

5. `app_settings` rows are always treated as user data — no marker needed.
6. **Default convention:** in every table carrying an `is_user_added` column, the column must default to `true` (the assumption is that any new row written at runtime is user-generated). The **bootstrap procedure** in `cli/src/bootstrap/` is the only code path that must explicitly write `is_user_added: false` for rows it seeds.
7. Chanting tables (`chanting_collections`, `chants`, `chanting_sections`) already use `is_user_added`. The bootstrap (`cli/src/bootstrap/chanting_practice.rs`) already writes `is_user_added: false` for the defaults — this must be preserved. The column default in the schema must be `true` so that any new row added at runtime via the UI is marked user-added without the call site having to set it.
8. For **bookmarks**: add an `is_user_added` boolean column with default `true` if one does not already exist. If the bootstrap seeds any default bookmarks, that bootstrap code must explicitly set `is_user_added: false` for them. Runtime inserts from the UI do not need to set the flag (default applies).
9. For **imported ebooks** (`books`, and by association `book_spine_items` / `book_resources`): add an `is_user_added` boolean column with default `true` to `books`. Any bootstrap that pre-seeds books must set `is_user_added: false`; runtime imports via `import_epub_to_db` / `import_pdf_to_db` / `import_html_to_db` rely on the default.
10. **Custom AI model names / prompts** stored in any user-editable table must follow the same convention: schema default `true`, bootstrap writes `false` for seeded rows. Defining a new table for these is out of scope; this requirement applies to any such table that already exists.
11. **Custom sutta languages** are not distinguished by a column. A language is considered user-added if its code is not `en` or `pli`. The migration export step must write the list of such codes to `download_languages.txt` exactly as today.
12. **Bootstrap audit:** as part of this refactor, every `is_user_added` write in `cli/src/bootstrap/` must be reviewed to confirm it sets `false`. Any seeded row that omits the field (and thus would pick up the new `true` default) must be updated to explicitly set `false`.

### 4.3 Database-upgrade export/import flow (single-DB)

11. `export_user_data_to_assets()` in `backend/src/app_data.rs` must be updated so every SELECT that today reads from `userdata` reads from `appdata` filtered by `is_user_added = true` (or, for `app_settings`, selects the whole row).
13. The `import-me/` folder contents are clarified so each export target is self-describing:
    - `app_settings.json` — full user settings blob.
    - `appdata-books.sqlite3` — user-added books, their spine items, and their resources only. **Renamed** from the current `appdata.sqlite3` user-books export so the file's purpose is obvious and does not clash with the main database filename.
    - `appdata-bookmarks.sqlite3` — user-added bookmarks only.
    - `appdata-chanting.sqlite3` + `chanting-recordings/` — user-added chanting rows and recording audio files.
    - `download_languages.txt` — user-downloaded language codes (anything not in `{en, pli}`).
    - (From legacy userdata.sqlite3 upgrade path only: see §4.5.) `legacy-userdata.sqlite3` — a full copy of the old `userdata.sqlite3` preserved for a one-shot import.
14. `import_user_data_from_assets()` must import into the single `appdata` database. Every `self.dbm.userdata.get_conn()` in the import path becomes `self.dbm.appdata.get_conn()`. The user-books import must read from `appdata-books.sqlite3` (new filename). Duplicate key handling (existing rows vs. imported rows) must preserve the imported user data.
15. After a successful upgrade cycle, no `userdata.sqlite3` file must be created on disk anywhere in the app assets directory.

### 4.4 Language re-download on upgrade (unchanged contract)

16. The mechanism for capturing and re-downloading user languages (`download_languages.txt`, `auto_start_download.txt`, `DownloadAppdataWindow.qml`) must continue to function as documented in PROJECT_MAP.md, with the only change being that the per-language import target (`import_suttas_lang_to_userdata()` in `bridges/src/asset_manager.rs`) now writes into `appdata.sqlite3`. The function should be renamed to reflect the new target (e.g. `import_suttas_lang_to_appdata()`).

### 4.5 One-shot upgrade path from legacy `userdata.sqlite3` (alpha testers)

This section exists because the app is currently in alpha testing and a population of users has accumulated bookmarks, chanting recordings, custom chants/collections/sections, imported ebooks, and app settings inside `userdata.sqlite3`. When those users upgrade to the consolidated-DB release, their data must be carried across automatically. There is **no** ongoing support for `userdata.sqlite3` — just this one-shot bridge.

The upgrade for these users follows the same three-step pattern the app already uses for database upgrades (export → user restarts → download → user restarts → import), with one added branch in the export step for legacy userdata.

17. **Legacy-aware export (runs in the OLD app, version N, immediately before the upgrade tear-down):**
    - `prepare_for_database_upgrade()` in `bridges/src/sutta_bridge.rs` (which calls `export_user_data_to_assets()`) must be updated so that when `userdata.sqlite3` still exists alongside `appdata.sqlite3`:
        1. Read user rows from `userdata.sqlite3` (bookmarks, chanting tables with `is_user_added = true`, app_settings) and write the normal `import-me/` artifacts: `app_settings.json`, `appdata-bookmarks.sqlite3`, `appdata-chanting.sqlite3`, `chanting-recordings/`.
        2. Read user-added books from wherever they currently live (in practice `appdata.sqlite3`; if any exist in `userdata.sqlite3`, those too) and write them to `appdata-books.sqlite3`.
        3. Additionally, copy the entire `userdata.sqlite3` file into `import-me/legacy-userdata.sqlite3` as a safety net. This lets the new app re-run a full import if any table was missed by the per-table export.
    - `download_languages.txt` continues to be written from whichever DB currently holds sutta-language data (today: `userdata.sqlite3`). The list is derived as "all language codes present that are not `en` or `pli`".
18. **Marker files written in the old app:** `delete_files_for_upgrade.txt` must list **both** `userdata.sqlite3` and `appdata.sqlite3` (and the existing dictionaries / DPD files) so startup cleanup in version N+1 removes them before the new download. `auto_start_download.txt` remains as today.
19. **Download step (new app, version N+1):** `DownloadAppdataWindow.qml` downloads the fresh single `appdata.sqlite3` and, driven by `download_languages.txt`, re-downloads each user language and imports it into the new `appdata.sqlite3` via the renamed `import_suttas_lang_to_appdata()`.
20. **Legacy-aware import (runs in the NEW app, version N+1, after `init_app_data()`):**
    - `import_user_data_from_assets()` performs its normal imports from `app_settings.json`, `appdata-books.sqlite3`, `appdata-bookmarks.sqlite3`, `appdata-chanting.sqlite3`, and `chanting-recordings/`.
    - If `import-me/legacy-userdata.sqlite3` is present, the importer additionally opens it and replays any user-added rows that were not already covered by the per-table files — specifically any rows with `is_user_added = true` in tables whose dedicated export file is absent or empty. This is defensive: the primary path is the per-table exports.
    - On successful completion, the entire `import-me/` folder is deleted (existing behaviour), which also removes `legacy-userdata.sqlite3`.
21. **Post-upgrade:** after a successful version-N+1 startup, no `userdata.sqlite3` file exists anywhere in the app assets directory, and no code path references it at runtime. The legacy bridge is single-use.

### 4.6 Cleanup / no ongoing backwards compatibility

22. Beyond the one-shot legacy upgrade path in §4.5, there is no fallback: the refactor removes the `userdata` handle, the `userdata_*` path fields on `AppGlobalPaths`, `initialize_userdata()`, and the `userdata_exists` check in `backend/src/db/mod.rs` outright.
23. If a leftover `userdata.sqlite3` is still present on disk after the one-shot import completes, the new version must delete it at startup (same mechanism as `delete_files_for_upgrade.txt` uses today). No shim code is retained.
24. `DatabaseValidationDialog.qml` and any other UI text that references "userdata" must be updated to refer to "appdata" or a neutral term.
25. Documentation (`docs/windows-user-data-paths.md`, `docs/language-download-implementation.md`, `PROJECT_MAP.md`) must be updated to reflect the single-database architecture and the one-shot alpha-upgrade bridge.

### 4.7 "Reset Settings to Default" button

28. Add a **Reset Settings to Default** button to `assets/qml/AppSettingsWindow.qml`. Placement: visible within the window (e.g. in a footer row or at the bottom of the scrollable settings list) so it is clearly discoverable but not immediately adjacent to destructive actions.
29. Clicking the button must prompt the user with a confirmation dialog before resetting (e.g. "Reset all app settings to their default values? This cannot be undone."). Only on confirmation does the reset proceed.
30. On confirmation, the backend must replace the current `app_settings` row's JSON value with a freshly serialized `AppSettings::default()` (the existing `impl Default for AppSettings` in `backend/src/app_settings.rs`). The in-memory `app_settings_cache` must be updated to match, so every subsequent read returns defaults without requiring an app restart.
31. Expose this reset as a single bridge method (e.g. `reset_app_settings_to_defaults()` on `SuttaBridge`) that performs: (a) replace cache with `AppSettings::default()`, (b) write the serialized defaults into the `app_settings` row in `appdata.sqlite3`, (c) emit whatever signal/notification the window needs so open UI bindings refresh. Per CLAUDE.md conventions, add a matching stub in `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml` for `qmllint`.
32. This reset must write to `appdata.sqlite3` (after Stage 2 routes app_settings there) — never to `userdata.sqlite3`. The button should not be implemented before Stage 2 has flipped the app_settings handle.

### 4.8 Tests

33. Backend integration tests that today exercise `userdata` (e.g. `backend/tests/test_search.rs` and any test fixture creation) must be updated to target `appdata` only.
34. No new automated round-trip test is required for the upgrade flow — the user will verify the upgrade cycle manually (both the clean single-DB path and the one-shot legacy `userdata.sqlite3` bridge), and will manually verify the "Reset Settings to Default" button.

## 5. Non-Goals (Out of Scope)

- Designing a custom-dictionary import feature (no mechanism exists today).
- Designing browser-extension API token storage (not in use today).
- Changing the underlying schema for suttas, dictionaries, or DPD data.
- Ongoing support for `userdata.sqlite3`: only a one-shot upgrade bridge for alpha testers is in scope (§4.5). No continuing read/write compatibility.
- Restructuring the bootstrap/download pipeline beyond the handle switch for language imports.
- Introducing a new user-prompt / AI-model-name table (handled separately if and when needed; this PRD only requires the `is_user_added` convention be honored for any such table that already exists).
- Automated integration tests of the upgrade round-trip — manual testing per §4.7.

## 6. Design Considerations

- No UI surface changes are required. The work is purely backend/bridges plus text updates in dialogs and docs.
- `DatabaseValidationDialog.qml` is the main user-visible touchpoint that currently references both DBs and must be reviewed for string updates.

## 7. Technical Considerations

- All database paths flow through `AppGlobalPaths` in `backend/src/lib.rs`; removing the `userdata_*` fields there is the anchor change. Every downstream compile error after that removal is a site to migrate.
- Use `try_exists()` (per CLAUDE.md Android guidance) when checking for the legacy `userdata.sqlite3` file during startup cleanup.
- `app_settings` is a single-row table keyed by `key = "app_settings"`; consolidation is a pure handle change — no data shape change.
- The chanting export already filters by `is_user_added.eq(true)`; that pattern is the template for the bookmarks and books export queries.
- The Diesel schema in `backend/src/db/appdata_schema.rs` will need one migration to add `is_user_added` to `books` and (if missing) to `bookmarks`. Because there are no production users to migrate, this can be a straightforward schema change; regenerating the bootstrap `appdata.sqlite3` is acceptable.

## 8. Success Metrics

1. Zero runtime references to `userdata` in Rust, C++, QML, and docs after the refactor (verified by grep). The only permitted mentions are inside the one-shot legacy-import code path in §4.5 and the docs describing it.
2. `userdata.sqlite3` is never created on disk by a fresh run of the app.
3. `make build -B` succeeds; `cd backend && cargo test` passes.
4. Manual smoke test by the user on a **fresh install**: bookmark a sutta, add a chant recording, change an app setting, import an ebook, download an extra language, trigger a DB upgrade, confirm all items survive.
5. Manual smoke test by the user on an **alpha install with existing `userdata.sqlite3`**: upgrade to the new version, confirm bookmarks, chanting recordings (audio files + metadata), custom chants/collections/sections, imported ebooks, app settings, and downloaded languages all survive.

## 9. Implementation Stages

Each stage is small enough to build + test before moving on. After every stage: `make build -B` and `cd backend && cargo test`. Do not run `make qml-test` unless asked.

**Stage 1 — Schema markers + bootstrap audit.**
- Add `is_user_added` column with `DEFAULT TRUE` to `books` (and, if missing, to `bookmarks`) in `backend/src/db/appdata_schema.rs`, corresponding migrations, and models.
- Confirm existing chanting tables keep `DEFAULT TRUE` on `is_user_added`; if any are currently `DEFAULT FALSE`, flip them.
- Audit every bootstrap path in `cli/src/bootstrap/` (already known: `chanting_practice.rs` seeds with `is_user_added: false`). Ensure every seeded row that lands in a table with this column explicitly sets `false`. No seeded row may rely on the default.
- Regenerate the bootstrap appdata.sqlite3 as needed.
- Verify: spot-check bootstrapped rows have `is_user_added = 0` and runtime-inserted rows have `is_user_added = 1`.

**Stage 2 — Route `app_settings` reads/writes to `appdata`.**
- In `backend/src/app_data.rs`, flip the single-row `app_settings` getters/setters and the `app_settings_cache` initialization to `self.dbm.appdata`.
- Verify: settings round-trip across an app restart; no writes to `userdata.sqlite3` observed (file mtime unchanged after settings change).

**Stage 3 — Route remaining user-data reads/writes to `appdata`.**
- Flip every remaining `self.dbm.userdata.get_conn()` site in `backend/src/app_data.rs` to `appdata`.
- Update the bookmark/chanting/user-book import paths inside `import_user_data_from_assets()` to use `appdata`.
- Rename the user-books export file from `appdata.sqlite3` to `appdata-books.sqlite3` on both the export and import sides.
- Verify: bookmark + chanting + user-book flows work end-to-end on a fresh install that has no `userdata.sqlite3`.

**Stage 4 — Update the normal migration export to read from `appdata`.**
- Update `export_user_data_to_assets()` queries to read from `appdata` filtered by `is_user_added = true` (or, for `app_settings`, the whole row).
- Rename `bridges/src/asset_manager.rs::import_suttas_lang_to_userdata` → `import_suttas_lang_to_appdata` and update call sites in QML.
- Verify (manually): a DB-upgrade cycle on a single-DB install produces the expected `import-me/` contents (app_settings.json, appdata-books.sqlite3, appdata-bookmarks.sqlite3, appdata-chanting.sqlite3 + chanting-recordings/, download_languages.txt).

**Stage 5 — Legacy one-shot bridge for alpha testers (§4.5).**
- Extend `export_user_data_to_assets()` so that when `userdata.sqlite3` still exists, it additionally: (a) reads user rows out of `userdata` (bookmarks, chanting, app_settings, sutta-language data for `download_languages.txt`) and writes the normal `import-me/` artifacts, and (b) copies the whole `userdata.sqlite3` to `import-me/legacy-userdata.sqlite3`.
- Ensure `delete_files_for_upgrade.txt` lists `userdata.sqlite3` in addition to the current entries, so startup cleanup removes it.
- Extend `import_user_data_from_assets()` with a defensive tail step: if `legacy-userdata.sqlite3` is present, import any user-added rows not already covered by the per-table export files.
- Verify (manually): upgrade cycle starting from an alpha install with a populated `userdata.sqlite3` carries bookmarks, chants, recordings, app settings, user-added books, and downloaded languages into the new single `appdata.sqlite3`.

**Stage 6 — Remove the `userdata` handle and paths.**
- Delete `userdata: AppdataDbHandle` from the DB manager struct.
- Delete `userdata_db_path`, `userdata_abs_path`, `userdata_database_url` from `AppGlobalPaths`.
- Delete `initialize_userdata()` and the `userdata_exists` check in `backend/src/db/mod.rs`.
- Fix all resulting compile errors.
- The only surviving mentions of `userdata` must be inside the Stage 5 legacy-bridge code path and its logging/comments.

**Stage 7 — "Reset Settings to Default" button (§4.7).**
- Add `reset_app_settings_to_defaults()` to `bridges/src/sutta_bridge.rs` (writes `AppSettings::default()` into the `appdata` `app_settings` row and resets the cache). Add the matching stub in `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml`.
- Add the button + confirmation dialog in `assets/qml/AppSettingsWindow.qml` and wire it to the new bridge method.
- Verify (manually): change several settings, click Reset, confirm — all settings return to defaults without restart; `appdata.sqlite3` reflects the new `app_settings` JSON.

**Stage 8 — Startup cleanup + UI/doc text.**
- On startup, if a stale `userdata.sqlite3` is present (`try_exists()`) and the legacy bridge has already run (no `import-me/` pending), delete it and log.
- Update `DatabaseValidationDialog.qml` and any other user-visible strings.
- Update `PROJECT_MAP.md`, `docs/windows-user-data-paths.md`, `docs/language-download-implementation.md` to describe the single-database architecture and the one-shot alpha-upgrade bridge.
- Verify (manually): fresh-install run never creates `userdata.sqlite3`; alpha-upgrade run cleanly removes the legacy file after import.

## 10. Open Questions

1. Does a `bookmarks` table already have `is_user_added`? If it does, Stage 1 drops that half. (Check during Stage 1 rather than block the PRD.)
2. Are there any user-prompt / custom-AI-model rows already persisted in a table today? If yes, which table — so Stages 1/3 can include it. If no, the point is moot until that feature lands.
3. For §4.5 step 17.2: if the alpha-era code was writing user-added books into `appdata.sqlite3` (not `userdata`), the export query simply reads `appdata.books WHERE is_user_added = true`. Confirm during Stage 5 that no alpha build was writing books into `userdata.sqlite3`; if any did, the legacy bridge must also read the `books` table out of `legacy-userdata.sqlite3`.
