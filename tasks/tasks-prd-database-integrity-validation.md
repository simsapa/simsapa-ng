# Tasks: Database Integrity Validation on Startup

## Relevant Files

- `bridges/src/sutta_bridge.rs` - Main bridge component where validation functions will be added
- `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml` - QML type definition for qmllint (needs new validation function signatures)
- `assets/qml/DatabaseValidationDialog.qml` - New QML dialog component for showing validation failures
- `backend/src/db/mod.rs` - Contains `initialize_userdata()` function and database initialization logic
- `backend/src/db/dictionaries.rs` - Dictionary database operations
- `backend/src/db/dictionaries_schema.rs` - Dictionary database schema (dict_words table)
- `backend/src/db/appdata_schema.rs` - Appdata schema (app_settings table for userdata validation)
- `backend/src/lib.rs` - Contains `AppGlobals` and database path utilities
- `cpp/gui.cpp` - Application startup sequence where validation will be triggered
- `assets/qml/DownloadAppdataWindow.qml` - Existing download window to be reused for re-download flow
- `bridges/src/asset_manager.rs` - AssetManager for `download_urls_and_extract()` function
- `bridges/build.rs` - Build script where new QML files must be registered
- `backend/tests/test_database_validation.rs` - New test file for validation logic
- `backend/tests/helpers/mod.rs` - Test helpers for creating test databases

### Notes

- Rust tests are run with `cd backend && cargo test` for all tests
- Single test: `cd backend && cargo test test_name`
- QML tests: `make qml-test` (runs with offscreen platform)
- Build: `make build -B` to verify compilation
- Avoid running GUI for testing (use unit tests instead)

## Tasks

- [ ] 1.0 Implement dictionary and userdata first query validation functions in Rust backend
  - [ ] 1.1 Add `dictionary_first_query()` function in `bridges/src/sutta_bridge.rs` marked as `#[qinvokable]`
  - [ ] 1.2 Implement dictionary validation query checking `dict_words` table for uid `"anidassana/dpd"` with result count > 0
  - [ ] 1.3 Use `thread::spawn` to run query in background thread (following existing pattern)
  - [ ] 1.4 Add `userdata_first_query()` function in `bridges/src/sutta_bridge.rs` marked as `#[qinvokable]`
  - [ ] 1.5 Implement userdata validation query checking `app_settings` table for key `"app_settings"` returning exactly 1 row
  - [ ] 1.6 Use `thread::spawn` to run userdata query in background thread
  - [ ] 1.7 Add corresponding function signatures to `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml` for qmllint

- [ ] 2.0 Transform existing first query functions into comprehensive validators with error handling
  - [ ] 2.1 Refactor `appdata_first_query()` to return validation results instead of just running query
  - [ ] 2.2 Add error handling to check for three conditions: database file missing, query error, zero results
  - [ ] 2.3 Use `Result<(), anyhow::Error>` pattern for proper error propagation in appdata validation
  - [ ] 2.4 Refactor `dpd_first_query()` with same three-condition error handling pattern
  - [ ] 2.5 Update dictionary and userdata query functions with comprehensive error handling
  - [ ] 2.6 Ensure all validation functions check database file existence using `get_app_globals().paths.*_db_path`

- [ ] 3.0 Create QML DatabaseValidationDialog component with multi-database failure handling
  - [ ] 3.1 Create `assets/qml/DatabaseValidationDialog.qml` following dialog patterns from `AboutDialog.qml`
  - [ ] 3.2 Add dialog properties to track which databases failed (downloadable: appdata, dpd, dictionaries; user-specific: userdata)
  - [ ] 3.3 Implement conditional button layout logic: show different buttons based on which database types failed
  - [ ] 3.4 Add "Re-download" button that triggers when only downloadable databases failed
  - [ ] 3.5 Add "Reset to Defaults" button that triggers when only userdata failed (with warning about data loss)
  - [ ] 3.6 Add "Fix All", "Re-download Only", "Reset Userdata Only" buttons when both types failed
  - [ ] 3.7 Add "Cancel" button that dismisses dialog and allows app to continue
  - [ ] 3.8 Display clear user-friendly messages distinguishing downloadable vs userdata failures
  - [ ] 3.9 Make dialog modal to block main window interaction until user chooses action
  - [ ] 3.10 Register `DatabaseValidationDialog.qml` in `bridges/build.rs` qml_files list

- [ ] 4.0 Integrate validation into startup sequence with background threading and signal handling
  - [ ] 4.1 Add new signal `databaseValidationFailed` to `SuttaBridge` in `bridges/src/sutta_bridge.rs` with parameter for failed database names (e.g., QString with comma-separated list)
  - [ ] 4.2 Add signal declaration in `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml` for qmllint
  - [ ] 4.3 Modify validation functions to emit `databaseValidationFailed` signal when validation fails (using `qt_thread.queue()` pattern)
  - [ ] 4.4 Pass detailed failure information through signal (which databases failed and why)
  - [ ] 4.5 Update `cpp/gui.cpp` startup sequence to call all four validation functions after main window is created
  - [ ] 4.6 Ensure validation calls are non-blocking (they already use `thread::spawn` internally)
  - [ ] 4.7 Add QML `Connections` block in main app window to listen for `databaseValidationFailed` signal
  - [ ] 4.8 Show `DatabaseValidationDialog` when signal received with failed database information
  - [ ] 4.9 Ensure dialog appears on top of already-open main application window as modal overlay

- [ ] 5.0 Implement database recovery flows (re-download and userdata reset)
  - [ ] 5.1 Implement "Re-download" button handler in `DatabaseValidationDialog.qml` to open `DownloadAppdataWindow`
  - [ ] 5.2 Configure `DownloadAppdataWindow` with `is_initial_setup: false` to skip language selection
  - [ ] 5.3 Build download URLs dynamically based on which databases failed (appdata.tar.bz2, dpd.tar.bz2, dictionaries.tar.bz2)
  - [ ] 5.4 Get version from `manager.get_current_version()` or application config for URL construction
  - [ ] 5.5 Remove failed downloadable database files before starting download (using paths from `get_app_globals().paths`)
  - [ ] 5.6 Call `manager.download_urls_and_extract(urls, false)` to trigger download with progress bar
  - [ ] 5.7 Implement "Reset to Defaults" button handler to remove userdata database at `get_app_globals().paths.userdata_db_path`
  - [ ] 5.8 Call existing `initialize_userdata()` function from backend to recreate userdata with defaults
  - [ ] 5.9 Show confirmation message after userdata reset: "Userdata has been reset to defaults. The app will now restart."
  - [ ] 5.10 Implement "Fix All" button handler to perform userdata reset first, then proceed to re-download flow
  - [ ] 5.11 Add app quit/restart logic after userdata reset (for "Reset to Defaults" and after downloads complete)
  - [ ] 5.12 Ensure download completion view prompts user to quit and restart application

- [ ] 6.0 Add comprehensive logging for all validation operations
  - [ ] 6.1 Add `info()` log at start of each validation: "Database validation: Checking appdata...", "Checking dpd...", "Checking dictionaries...", "Checking userdata..."
  - [ ] 6.2 Add `info()` log on validation success: "Database validation: Appdata OK", "DPD OK", "Dictionaries OK", "Userdata OK"
  - [ ] 6.3 Add `error()` log on validation failure with specific reason: "Database validation FAILED: Appdata - Database file not found", "Query returned 0 results", "Query error: [details]"
  - [ ] 6.4 Add `info()` log for user actions from dialog: "User chose to re-download database(s): [list]", "User chose to reset userdata database", "User cancelled database validation dialog"
  - [ ] 6.5 Add `info()` log for userdata reset operations: "Removing corrupt userdata database...", "Re-initializing userdata database with defaults...", "Userdata database reset complete"
  - [ ] 6.6 Ensure all logs use consistent format with "Database validation:" prefix for easy filtering

- [ ] 7.0 Create integration tests for validation scenarios
  - [ ] 7.1 Create `backend/tests/test_database_validation.rs` with test helper setup
  - [ ] 7.2 Add test for successful validation of all four databases (appdata, dpd, dictionaries, userdata)
  - [ ] 7.3 Add test for appdata validation failure when database file is missing
  - [ ] 7.4 Add test for dpd validation failure when query returns 0 results (simulate by deleting specific data)
  - [ ] 7.5 Add test for dictionaries validation failure when uid "anidassana/dpd" is not found
  - [ ] 7.6 Add test for userdata validation failure when app_settings table is empty or has wrong data
  - [ ] 7.7 Add test for query error handling (simulate corrupt database with invalid SQL)
  - [ ] 7.8 Add test helper in `backend/tests/helpers/mod.rs` for creating minimal test databases
  - [ ] 7.9 Add test for userdata reset flow: verify database is removed and re-initialized with defaults
  - [ ] 7.10 Mark tests with `#[serial]` attribute if they interact with file system to avoid race conditions
  - [ ] 7.11 Verify all tests pass with `cd backend && cargo test test_database_validation`
