## Relevant Files

- `backend/Cargo.toml` - Add `zip` crate dependency
- `backend/src/db/chanting_export.rs` - New module: export and import logic for chanting data zip archives
- `backend/src/db/mod.rs` - Register the new `chanting_export` module
- `backend/src/db/appdata.rs` - Add helper queries for fetching selected chanting data by UID lists
- `backend/src/app_data.rs` - Extend `export_user_data_to_assets()` and `import_user_data_from_assets()` with chanting data preservation
- `bridges/src/sutta_bridge.rs` - Add `export_chanting_data` and `import_chanting_data` bridge functions
- `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml` - Add qmllint type definitions for the new bridge functions
- `assets/qml/ChantingPracticeWindow.qml` - Add Export/Import buttons, selection mode state, file dialogs, importing spinner dialog
- `assets/qml/ChantingTreeList.qml` - Add checkbox mode with hierarchical selection logic

### Notes

- The existing `export_user_books` / `import_user_books` pattern in `app_data.rs` is the closest reference for creating a standalone SQLite export database, running migrations, and re-importing.
- `FileDialog` patterns exist in `DocumentImportDialog.qml` and `ChantingPracticeReviewWindow.qml` — follow those for cross-platform file URL handling (including Android `content://` URIs).
- Use `cd backend && cargo test` to run Rust tests. Use `make qml-test` for QML tests. Use `make build -B` to verify compilation.

## Tasks

- [x] 1.0 Add `zip` crate dependency and create chanting export/import module
  - [x] 1.1 Add `zip = "2"` to `backend/Cargo.toml` under `[dependencies]`
  - [x] 1.2 Create `backend/src/db/chanting_export.rs` with the module skeleton (empty `pub fn export_chanting_to_zip(...)` and `pub fn import_chanting_from_zip(...)` signatures)
  - [x] 1.3 Register the new module in `backend/src/db/mod.rs` with `pub mod chanting_export;`
  - [x] 1.4 Verify compilation with `cd backend && cargo check`

- [x] 2.0 Implement Rust backend export logic
  - [x] 2.1 In `chanting_export.rs`, implement a helper function `create_chanting_sqlite(dest_path, collections, chants, sections, recordings)` that: creates a new SQLite database at `dest_path`, runs the appdata migrations to set up the chanting tables schema, and inserts the provided rows
  - [x] 2.2 Implement `export_chanting_to_zip(appdata_db: &AppdataDb, selected_section_uids: Vec<String>, selected_chant_uids: Vec<String>, selected_collection_uids: Vec<String>, dest_zip_path: &Path) -> Result<()>` that: queries the selected collections/chants/sections from the live DB by UID, queries all recordings for the selected sections, creates a temp directory, calls `create_chanting_sqlite` to write `appdata-chanting.sqlite3`, copies the referenced audio files into a `chanting-recordings/` subfolder in the temp dir, creates the `.zip` archive using the `zip` crate, writes it to `dest_zip_path`, and cleans up the temp directory
  - [x] 2.3 In `appdata.rs`, add helper query functions: `get_chanting_collections_by_uids(uids)`, `get_chanting_chants_by_uids(uids)`, `get_chanting_sections_by_uids(uids)`, `get_chanting_recordings_for_sections(section_uids)` — these return the full row structs filtered by the given UID lists
  - [x] 2.4 Write unit tests for `create_chanting_sqlite` verifying that data round-trips correctly (insert into export DB, read back, compare)

- [x] 3.0 Implement Rust backend import logic
  - [x] 3.1 Implement `import_chanting_from_zip(appdata_db: &AppdataDb, zip_path: &Path, recordings_dir: &Path) -> Result<ImportResult>` where `ImportResult` contains counts of imported items. The function: extracts the zip to a temp dir, validates `appdata-chanting.sqlite3` exists, opens the embedded SQLite DB read-only
  - [x] 3.2 Implement UID remapping: read all rows from the 4 chanting tables in the embedded DB, generate new UIDs for each record, build an `old_uid → new_uid` HashMap, update all foreign key references (`collection_uid`, `chant_uid`, `section_uid`) using the map
  - [x] 3.3 Update `file_name` fields in recording rows: if the filename contains the old section UID prefix, replace it with the new section UID; generate a new unique filename for each recording
  - [x] 3.4 Set `is_user_added = true` on all imported collection/chant/section records
  - [x] 3.5 Insert all remapped rows into the live appdata database using the existing `create_chanting_collection`, `create_chanting_chant`, `create_chanting_section`, `create_chanting_recording` functions
  - [x] 3.6 Copy audio files from the extracted `chanting-recordings/` folder to the app's recordings directory, renaming files to match the new filenames from step 3.3
  - [x] 3.7 Clean up the temp directory. Return `ImportResult` with counts.
  - [x] 3.8 Handle error cases: missing `appdata-chanting.sqlite3`, corrupt database, missing audio files referenced in DB (log warning, skip file, continue import)
  - [x] 3.9 Write unit tests for UID remapping logic and for a full round-trip (export then import, verify data integrity)

- [x] 4.0 Add bridge functions in `sutta_bridge.rs` and qmllint stubs
  - [x] 4.1 Declare `export_chanting_data(json_selected_uids: &QString, dest_path: &QString) -> QString` as `#[qinvokable]` in the bridge definition block. Implement it to: parse the JSON (containing `collections`, `chants`, `sections` UID arrays), call `export_chanting_to_zip`, return `{"ok": true}` or `{"error": "..."}` JSON
  - [x] 4.2 Declare `import_chanting_data(zip_path: &QString) -> QString` as `#[qinvokable]`. Implement it to: call `import_chanting_from_zip`, return `{"ok": true, "imported": {...}}` or `{"error": "..."}` JSON
  - [x] 4.3 Add the qmllint stub functions in `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml`:
    ```qml
    function export_chanting_data(json_selected_uids: string, dest_path: string): string { return '{"ok": true}'; }
    function import_chanting_data(zip_path: string): string { return '{"ok": true}'; }
    ```
  - [x] 4.4 Verify compilation with `make build -B`

- [x] 5.0 Implement selection mode UI in ChantingTreeList.qml
  - [x] 5.1 Add new properties to `ChantingTreeList.qml`: `property bool selection_mode: false` and `property var checked_items: ({})` (JS object keyed by UID, values are booleans)
  - [x] 5.2 Add a signal `checked_items_changed()` that fires whenever `checked_items` is modified
  - [x] 5.3 Add a `CheckBox` to the **collection** delegate row, visible only when `root.selection_mode`. Positioned to the left of the expand arrow. On toggled: set/unset the collection UID in `checked_items`, and also set/unset **all** child chant UIDs and **all** grandchild section UIDs. Emit `checked_items_changed()`.
  - [x] 5.4 Add a `CheckBox` to the **chant** delegate row, visible only when `root.selection_mode`. On toggled: set/unset the chant UID and **all** child section UIDs. Apply upward auto-selection: if checked, also check the parent collection; if unchecked and no sibling chants remain checked, uncheck the parent collection. Emit `checked_items_changed()`.
  - [x] 5.5 Add a `CheckBox` to the **section** delegate row, visible only when `root.selection_mode`. On toggled: set/unset only the section UID. Apply upward auto-selection: if checked, also check the parent chant and grandparent collection; if unchecked and no sibling sections remain checked, uncheck the parent chant, and if no sibling chants remain checked, uncheck the grandparent collection. Emit `checked_items_changed()`.
  - [x] 5.6 Implement helper JS functions on the root: `toggle_collection(col)`, `toggle_chant(col, chant)`, `toggle_section(col, chant, section)` that encapsulate the hierarchical check/uncheck logic and return the updated `checked_items` object
  - [x] 5.7 Add a function `get_selected_uids()` that returns `{ collections: [...], chants: [...], sections: [...] }` by iterating `checked_items` and categorizing UIDs based on the collections_list data
  - [x] 5.8 Add a function `clear_selection()` that resets `checked_items` to `{}`

- [x] 6.0 Implement Export/Import buttons and dialogs in ChantingPracticeWindow.qml
  - [x] 6.1 Add `import QtQuick.Dialogs` to the imports. Add `property bool export_selection_mode: false` to the root.
  - [x] 6.2 Add an **"Export"** button to the toolbar `RowLayout`. When not in selection mode, it shows "Export" with default styling. When in selection mode, it shows "Export Selected" with green background (`#4CAF50`, white text).
  - [x] 6.3 Implement the Export button click logic: on first click (not in selection mode), show an informational dialog explaining selection mode, set `export_selection_mode = true`, which propagates to `ChantingTreeList.selection_mode`. On second click (in selection mode), validate that at least one item is selected (otherwise show warning dialog), then open the save FileDialog.
  - [x] 6.4 Add a **"Cancel"** button next to the Export button, visible only during `export_selection_mode`. Clicking it resets `export_selection_mode = false` and calls `clear_selection()` on the tree list.
  - [x] 6.5 Add a save `FileDialog` for export with `fileMode: FileDialog.SaveFile`, `nameFilters: ["ZIP files (*.zip)"]`, and auto-generated filename `chanting-export-YYYY-MM-DDTHHMMSS.zip`. In `onAccepted`: get the selected UIDs from the tree list, call `SuttaBridge.export_chanting_data(json, dest_path)`, parse the result, show success or error dialog, exit selection mode.
  - [x] 6.6 Ensure the `.zip` extension is enforced on the export filename (append if missing).
  - [x] 6.7 Add an **"Import"** button to the toolbar. Clicking it opens an open `FileDialog` with `fileMode: FileDialog.OpenFile`, `nameFilters: ["ZIP files (*.zip)"]`.
  - [x] 6.8 Add an "Importing..." modal `Dialog` with a `BusyIndicator` and label. It has no standard buttons and `closePolicy: Popup.NoClose`.
  - [x] 6.9 In the import FileDialog `onAccepted`: show the importing dialog, call `SuttaBridge.import_chanting_data(zip_path)`, parse the result, close the importing dialog, show a success or error dialog, call `load_collections()` to refresh the tree.
  - [x] 6.10 Handle Android `content://` URIs for import by using `SuttaBridge.copy_content_uri_to_temp()` before passing the path to the import function (follow the pattern in `DocumentImportDialog.qml`).

- [x] 7.0 Extend database upgrade path for chanting data preservation
  - [x] 7.1 In `app_data.rs`, implement `export_user_chanting_data(import_dir: &Path) -> Result<()>`: query all `is_user_added = true` collections/chants/sections, query **all** recordings, create `appdata-chanting.sqlite3` in `import_dir` using `create_chanting_sqlite`, copy the entire `chanting-recordings/` directory into `import_dir/chanting-recordings/`
  - [x] 7.2 Call `export_user_chanting_data(&import_dir)` from within `export_user_data_to_assets()`, after the existing user books export
  - [x] 7.3 In `app_data.rs`, implement `import_user_chanting_data(import_dir: &Path) -> Result<()>`: check for `appdata-chanting.sqlite3` in `import_dir`, open it read-only, load all rows from the 4 chanting tables
  - [x] 7.4 For user-added collections/chants/sections: insert them with their **original UIDs preserved** (no remapping, since the target DB is fresh after upgrade)
  - [x] 7.5 For recordings: check if the referenced `section_uid` exists in the new database (it may be a pre-shipped section that was re-shipped, or a user-added section just imported). If the section exists, insert the recording row. If the section no longer exists (removed from pre-shipped data), log a warning and skip the recording.
  - [x] 7.6 Copy audio files from `import_dir/chanting-recordings/` back to the app's `chanting-recordings/` directory
  - [x] 7.7 Call `import_user_chanting_data(&import_dir)` from within `import_user_data_from_assets()`, before the cleanup step
  - [x] 7.8 Write tests for the upgrade path: create a mock appdata DB with user-added chanting data and recordings, run export, simulate a fresh DB (run migrations only), run import, verify all user data and recordings are restored

- [x] 8.0 Testing and verification
  - [x] 8.1 Run `cd backend && cargo test` to verify all Rust tests pass (all chanting tests pass; pre-existing failures in test_search.rs and TestGlossTab are unrelated)
  - [x] 8.2 Run `make build -B` to verify full compilation (Rust + C++ + QML)
  - [x] 8.3 Run `make qml-test` to verify QML tests pass (84 passed, 1 pre-existing failure unrelated to changes)
  - [ ] 8.4 Manual test: open ChantingPracticeWindow, enter selection mode, verify checkboxes appear, verify hierarchical select/deselect logic works correctly (check collection selects all children, check section auto-selects parents, uncheck all children auto-unchecks parent)
  - [ ] 8.5 Manual test: export a selection, inspect the resulting `.zip` (verify it contains `appdata-chanting.sqlite3` and the correct audio files in `chanting-recordings/`)
  - [ ] 8.6 Manual test: import a previously exported `.zip`, verify all data appears in the tree list with new UIDs and `is_user_added = true`, verify audio files are accessible
  - [ ] 8.7 Manual test: simulate a database upgrade — create user chanting data, trigger `prepare_for_database_upgrade`, verify `import-me/` contains chanting data, simulate fresh DB, run import, verify all user data and recordings are restored
