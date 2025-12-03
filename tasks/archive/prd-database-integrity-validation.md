# Product Requirements Document: Database Integrity Validation on Startup

## Introduction/Overview

When the Simsapa app starts, it runs initial database queries (`appdata_first_query()` and `dpd_first_query()`) to warm up the database cache. This PRD extends this functionality to validate database integrity and guide users through recovery if databases are corrupt, incomplete, or missing. The feature will add a `dictionary_first_query()` function and transform all three queries into integrity validators that check for database existence, query execution success, and non-zero results. If validation fails, users will be prompted to re-download the affected database(s).

**Problem Statement:** Users may experience silent failures or confusing behavior when databases are corrupt, incomplete, or missing. Currently, there is no systematic validation to detect these issues early and guide users toward resolution.

**Goal:** Implement database integrity validation on every app startup that detects problematic databases and provides a clear, user-friendly recovery path.

## Goals

1. Add `dictionary_first_query()` function to validate the dictionaries database on startup
2. Add `userdata_first_query()` function to validate the userdata database on startup
3. Transform existing `appdata_first_query()` and `dpd_first_query()` into integrity validators
4. Detect all failure conditions: missing database files, query errors, and zero-result queries
5. **Run all validation queries in background threads without blocking or delaying the main app window from opening**
6. Provide clear user feedback when validation fails, specifying which database(s) are problematic
7. Enable users to selectively re-download only the database(s) that failed validation (appdata, dpd, dictionaries)
8. Enable users to reset userdata database to defaults if it fails validation
9. Log all validation results for debugging and troubleshooting
10. Maintain existing non-blocking app startup behavior - validation happens in parallel with normal app operation

## User Stories

1. **As a user with a corrupt database**, I want the app to detect the problem on startup and offer to fix it, so that I don't experience mysterious errors or missing data.

2. **As a user who interrupted a database download**, I want the app to detect the incomplete database and prompt me to re-download, so that I have a complete working database.

3. **As a developer debugging database issues**, I want validation results logged with clear information about which databases failed and why, so that I can troubleshoot user reports.

4. **As a user with multiple database problems**, I want to see which specific databases have issues and choose whether to fix them, so that I can understand what's wrong and have control over the recovery process.

5. **As a user who cancels the re-download**, I want the app to continue running (with potentially limited functionality), so that I can access whatever data is available.

6. **As a user launching the app**, I want the main window to open immediately without waiting for database validation, so that I perceive the app as fast and responsive, even if a validation dialog appears a moment later.

## Functional Requirements

### 1. Dictionary First Query Function

1.1. Create `dictionary_first_query()` function in `SuttaBridge` (following the pattern of existing `appdata_first_query()` and `dpd_first_query()`)

1.2. The query must check the `dict_words` table for the specific uid `"anidassana/dpd"` and verify the result count is greater than 0

1.3. The function must be marked as `#[qinvokable]` and declared in the bridge interface

1.4. The function must execute in a background thread to avoid blocking the Qt event loop (maintaining existing pattern)

1.5. The function should emit a signal when validation completes (either success or failure)

### 1b. Userdata First Query Function

1b.1. Create `userdata_first_query()` function in `SuttaBridge` (following the pattern of other validation functions)

1b.2. The query must check the `app_settings` table for the key `"app_settings"` and verify exactly 1 row is returned

1b.3. The function must be marked as `#[qinvokable]` and declared in the bridge interface

1b.4. The function must execute in a background thread to avoid blocking the Qt event loop (maintaining existing pattern)

1b.5. The function should emit a signal when validation completes (either success or failure)

### 2. Database Validation Logic

2.1. Each validation function (`appdata_first_query()`, `dpd_first_query()`, `dictionary_first_query()`, `userdata_first_query()`) must check for three failure conditions:
   - Database file does not exist
   - Query throws an error/exception
   - Query returns 0 results (or not exactly 1 for userdata)

2.2. Each validation function must return or emit a validation result indicating success or failure with error details

2.3. Validation must occur on every app startup (not just first-time startup)

2.4. **All validation functions run in background threads** (via `thread::spawn` as currently implemented)

2.5. **Validation does NOT block or delay the main app window from opening** - the window opens immediately while validation runs in background

2.6. If validation detects failures, a validation failure dialog is shown **on top of** the already-open main app window

2.7. Userdata validation is handled separately from downloadable databases since it cannot be re-downloaded (it's user-specific data)

### 3. Validation Failure Dialog

3.1. Create a QML dialog component that displays when any database validation fails **after the main app window is already open**

3.2. The dialog appears as a modal overlay on top of the running application window

3.3. The dialog must list the specific database(s) that failed validation and indicate the appropriate action:
   - **Downloadable databases** (can be re-downloaded):
     - "Appdata Database" (for appdata)
     - "DPD Database" (for dpd)
     - "Dictionaries Database" (for dictionaries)
   - **User-specific database** (can be reset):
     - "Userdata Database" (for userdata)

3.4. The dialog must provide different button options based on which databases failed:
   - **If only downloadable databases failed**:
     - **"Re-download"** button: Opens DownloadAppdataWindow to download the failed database(s)
     - **"Cancel"** button: Dismisses the dialog and allows the app to continue running
   - **If only userdata failed**:
     - **"Reset to Defaults"** button: Removes and re-initializes userdata database
     - **"Cancel"** button: Dismisses the dialog and allows the app to continue running
   - **If both downloadable databases AND userdata failed**:
     - **"Fix All"** button: Re-downloads failed downloadable databases AND resets userdata
     - **"Re-download Only"** button: Only re-downloads downloadable databases
     - **"Reset Userdata Only"** button: Only resets userdata to defaults
     - **"Cancel"** button: Dismisses the dialog and allows the app to continue running

3.5. The dialog message should clearly explain:
   - For downloadable databases: "The following database(s) may be incomplete or corrupted and may need to be re-downloaded."
   - For userdata: "The userdata database may be corrupted. You can reset it to default settings. WARNING: This will erase all your bookmarks, notes, and custom settings."

3.6. Only the database(s) that failed validation should be listed and scheduled for recovery

### 4. Database Removal and Recovery Flow

4.1. **When user clicks "Re-download"** (or "Re-download Only" or "Fix All") in the validation failure dialog:
   - Close the validation dialog
   - Open a `DownloadAppdataWindow` instance
   - Set `is_initial_setup` property to `false` (to skip language selection for failed databases)
   - Configure download URLs based on which database(s) failed:
     - If appdata failed: Add `appdata.tar.bz2` URL
     - If dpd failed: Add `dpd.tar.bz2` URL  
     - If dictionaries failed: Add `dictionaries.tar.bz2` URL
   - Remove the failed downloadable database file(s) before starting download (never remove userdata here)
   - Call `manager.download_urls_and_extract(urls, false)` to start download with progress bar
   - The download frame with progress bar will show download and extraction progress
   - When download completes, `onDownloadsCompleted` signal triggers view change (idx 3)
   - The completion view tells user to quit and restart the application

4.2. **When user clicks "Reset to Defaults"** (or "Reset Userdata Only" or "Fix All") in the validation failure dialog:
   - Close the validation dialog (if "Reset to Defaults" or "Reset Userdata Only")
   - Remove the userdata database file at `get_app_globals().paths.userdata_db_path`
   - Call the existing `initialize_userdata()` function to re-create the database with default settings
   - Log the reset operation with `info()`
   - If this was the only action ("Reset to Defaults" or "Reset Userdata Only"):
     - Show a confirmation message: "Userdata has been reset to defaults. The app will now restart."
     - Quit and restart the application
   - If this was part of "Fix All":
     - Userdata reset happens first, then proceed to re-download flow (step 4.1)

4.3. **When user clicks "Cancel"** in the validation failure dialog:
   - Dismiss the dialog
   - Allow the app to continue running (features requiring the missing/corrupt databases may not work)
   - No files are removed
   - Main application window opens as normal

### 5. Logging and Debugging

5.1. Log validation start for each database with `info()`:
   - "Database validation: Checking appdata..."
   - "Database validation: Checking dpd..."
   - "Database validation: Checking dictionaries..."
   - "Database validation: Checking userdata..."

5.2. Log validation success for each database with `info()`:
   - "Database validation: Appdata OK"
   - "Database validation: DPD OK"
   - "Database validation: Dictionaries OK"
   - "Database validation: Userdata OK"

5.3. Log validation failure with `error()`, including specific reason:
   - "Database validation FAILED: Appdata - Database file not found"
   - "Database validation FAILED: DPD - Query returned 0 results"
   - "Database validation FAILED: Dictionaries - Query error: [error details]"
   - "Database validation FAILED: Userdata - Query returned 0 rows (expected 1)"

5.4. Log user action from dialog with `info()`:
   - "User chose to re-download database(s): [list]"
   - "User chose to reset userdata database"
   - "User chose to fix all databases (re-download + reset userdata)"
   - "User cancelled database validation dialog"

5.5. Log userdata reset operations with `info()`:
   - "Removing corrupt userdata database..."
   - "Re-initializing userdata database with defaults..."
   - "Userdata database reset complete"

### 6. Integration Points

6.1. Call all four validation functions from the appropriate location in the startup sequence, **after the main app window is created**

6.2. Validation functions run in background threads and do NOT block the UI thread or delay window creation

6.3. Use existing CXX-Qt signal mechanisms to communicate validation results from Rust backend to QML UI:
   - When validation completes in background thread, emit signal with results
   - QML UI receives signal and shows validation failure dialog if needed (on top of main window)

6.4. Use existing database path utilities from `get_app_globals()` to locate and remove database files

6.5. Use existing `initialize_userdata()` function from `backend/src/db/mod.rs` to re-create userdata database

6.6. Validation queries should be called similar to how `appdata_first_query()` and `dpd_first_query()` are currently called - non-blocking background operations

## Non-Goals (Out of Scope)

1. Automatic database repair or recovery (only re-download/reset is offered)
2. Validation of database schema versions or migrations
3. Periodic validation during runtime (only on startup)
4. Automatic re-download/reset without user confirmation
5. Detailed progress tracking during validation queries (validation queries run quickly; progress is shown during re-download only)
6. Network connectivity checks before offering re-download (handled by existing download infrastructure)
7. Backup of corrupted databases before removal (including userdata - user is warned about data loss)
8. Validation of individual tables beyond the specific queries defined
9. UI for advanced users to skip validation or customize validation logic
10. Language selection during validation-triggered re-download (only re-downloads the failed core databases; languages can be added separately later)
11. Selective backup/restore of userdata content (e.g., exporting bookmarks before reset)

## Design Considerations

### QML Dialog Component

- Create `DatabaseValidationDialog.qml` in `assets/qml/`
- Use existing dialog styling consistent with other app dialogs (e.g., `AboutDialog.qml`, `AnkiExportDialog.qml`)
- Display failed databases in two sections:
  - **Downloadable databases**: appdata, dpd, dictionaries (shown in one list)
  - **User-specific database**: userdata (shown separately with warning about data loss)
- Ensure dialog is modal and blocks interaction with main window until dismissed
- Dialog should adapt button layout based on which types of databases failed:
  - Only downloadable → "Re-download" + "Cancel"
  - Only userdata → "Reset to Defaults" + "Cancel"
  - Both types → "Fix All" + "Re-download Only" + "Reset Userdata Only" + "Cancel"
- When action buttons are clicked, the dialog should:
  - Store which database(s) failed and which action was chosen
  - Close itself (for single-action buttons) or proceed to next step (for "Fix All")
  - Trigger appropriate recovery flow (DownloadAppdataWindow and/or userdata reset)

### Signal/Callback Mechanism

- **Option A**: Add QML signals to `SuttaBridge` for validation results (e.g., `databaseValidationFailed`)
- **Option B**: Use a new bridge component dedicated to startup validation
- **Recommended**: Use Option A for simplicity, adding signals to existing `SuttaBridge`

Example signal definition in `sutta_bridge.rs`:
```rust
#[qsignal]
#[cxx_name = "databaseValidationFailed"]
fn database_validation_failed(self: Pin<&mut SuttaBridge>, failed_databases: QString);
```

**Signal Flow**:
1. Validation queries run in background threads (via `thread::spawn`)
2. When validation completes, use `qt_thread.queue()` to emit signal to main Qt thread
3. QML UI receives signal via `Connections` block
4. QML shows validation failure dialog if failures detected
5. This matches the existing pattern used by `appdata_first_query()` and `dpd_first_query()`

### Startup Sequence Integration

Current startup flow in `cpp/gui.cpp`:
1. Check `appdata_db_exists()` → if false, show `DownloadAppdataWindow`
2. `init_app_data()` (creates AppData singleton)
3. `check_and_configure_for_first_start()`
4. Start webserver
5. Create main app window

**Modified startup flow with validation**:
1. Check `appdata_db_exists()` → if false, show `DownloadAppdataWindow` (with `is_initial_setup = true`)
2. `init_app_data()` (creates AppData singleton) - this will create userdata if it doesn't exist
3. `check_and_configure_for_first_start()`
4. Start webserver
5. **Create main app window** (this happens immediately without waiting)
6. **Start database validation queries in background threads** (new step - validate all 4 databases in parallel, non-blocking)
7. **When validation completes in background**:
   - If all validations pass: No action needed, user continues using app normally
   - If any validation fails: Emit signal to show validation failure dialog on top of main window
   - Dialog appears while app is already running
   - If user clicks "Re-download" or "Re-download Only": Remove failed downloadable database(s), open `DownloadAppdataWindow` with `is_initial_setup = false` and specific URLs, show progress, then prompt user to restart
   - If user clicks "Reset to Defaults" or "Reset Userdata Only": Remove userdata database, call `initialize_userdata()`, show confirmation, restart app
   - If user clicks "Fix All": Reset userdata first, then proceed to re-download flow
   - If user clicks "Cancel": Dialog dismisses, user continues with app (with potential limitations)

## Technical Considerations

### Database File Paths

- Use `get_app_globals().paths.appdata_db_path` to locate appdata.sqlite3
- Use `get_app_globals().paths.dpd_db_path` to locate dpd.sqlite3
- Use `get_app_globals().paths.dict_db_path` to locate dictionaries.sqlite3
- Never remove `get_app_globals().paths.userdata_db_path`

### Download URLs for Re-download

When initiating re-download via DownloadAppdataWindow, construct URLs based on the current release version:

```javascript
const version = "v0.x.x"; // Get from manager.get_current_version() or config
const github_repo = "simsapa/simsapa-release-assets"; // From config

const appdata_tar_url = `https://github.com/${github_repo}/releases/download/${version}/appdata.tar.bz2`;
const dictionaries_tar_url = `https://github.com/${github_repo}/releases/download/${version}/dictionaries.tar.bz2`;
const dpd_tar_url = `https://github.com/${github_repo}/releases/download/${version}/dpd.tar.bz2`;
```

Only add URLs for databases that failed validation.

### Query Implementations

**Appdata validation query**: Already exists in `appdata_first_query()` - performs a search for "dhamma" with ContainsMatch mode. Verify it returns results > 0.

**DPD validation query**: Already exists in `dpd_first_query()` - performs `dpd_lookup_json("dhamma")`. Verify it returns results > 0.

**Dictionaries validation query**: New - must query:
```rust
use crate::db::dictionaries_schema::dict_words::dsl::*;
dict_words
    .filter(uid.eq("anidassana/dpd"))
    .select(DictWord::as_select())
    .first(db_conn)
```
Verify result exists (count > 0).

**Userdata validation query**: New - must query:
```rust
use crate::db::appdata_schema::app_settings::dsl::*;
app_settings
    .filter(key.eq("app_settings"))
    .select(AppSetting::as_select())
    .first(db_conn)
```
Verify exactly 1 row is returned and the row contains valid data.

### Error Handling

- Wrap all database queries in `Result<T, anyhow::Error>` for proper error propagation
- Use `match` expressions to distinguish between:
  - Database connection failures (file not found)
  - Query execution errors (corrupt database, SQL errors)
  - Empty results (incomplete download, wrong uid)

### Threading Considerations

- Validation queries run in background threads (via `thread::spawn`) - **this is maintained from existing implementation**
- Validation does **NOT** block or delay the main app window from opening
- Main window opens immediately, validation runs in parallel
- Use `qt_thread.queue()` to communicate results back to main thread (existing pattern)
- When validation completes, emit signal to show dialog on top of already-open window
- All four validations can run in parallel (they are independent queries)
- If validation dialog needs to be shown, it appears as a modal overlay on the running app

### Dependencies

- Existing modules: `simsapa_backend::db`, `simsapa_backend::logger`
- Existing bridge: `bridges/src/sutta_bridge.rs`
- Existing QML components: 
  - `DownloadAppdataWindow.qml` - for download UI with progress bar
  - `AssetManager` - for `download_urls_and_extract()` function
  - Dialog patterns from `assets/qml/` (e.g., `AboutDialog.qml`)
- Existing C++ utilities: `gui.cpp`, `window_manager.cpp`
- Existing download infrastructure: `AssetManager::download_urls_and_extract()` with progress signals

## Success Metrics

1. **Detection Rate**: 100% of corrupt/incomplete/missing databases are detected on startup
2. **False Positive Rate**: 0% - no false validation failures on healthy databases
3. **User Recovery Rate**: Track how many users successfully recover via re-download vs. cancel
4. **Support Ticket Reduction**: Measure reduction in user reports about "app not working" due to database issues
5. **Log Coverage**: 100% of validation failures are logged with sufficient debugging information

## Open Questions

1. **Question**: Should validation queries have a timeout to prevent indefinite blocking on very slow systems or network filesystems?
   - **Suggested Answer**: Yes, implement a 30-second timeout per query

2. **Question**: Should the validation dialog provide more detailed error information (e.g., "Query returned 0 results" vs. "Database file not found")?
   - **Suggested Answer**: No for initial implementation - keep user message simple. Detailed info goes to logs for developer debugging.

3. **Question**: How should we handle the case where appdata doesn't exist at all (already handled by existing `appdata_db_exists()` check)?
   - **Answer**: Existing flow is fine - validation only runs if `init_app_data()` succeeds, which requires appdata to exist

4. **Question**: Should we test the validation by creating a test mode that simulates database corruption?
   - **Suggested Answer**: Yes, add a CLI flag or environment variable for testing (e.g., `SIMSAPA_TEST_CORRUPT_DB=dpd,userdata`)

5. **Question**: Should validation results be stored in userdata for future reference or analytics?
   - **Answer**: Not in initial implementation - just log to application logs. Can be added later if needed. (Note: Can't store in userdata if userdata itself is corrupt!)

6. **Question**: Should there be a "Don't ask again" option in the validation dialog?
   - **Answer**: No - validation is a safety check that should always run. If a database is truly broken, the app should prompt every time until fixed.

7. **Question**: When userdata is reset, should we attempt to preserve any user data (bookmarks, notes, custom settings)?
   - **Answer**: Out of scope for initial implementation. The reset operation assumes the database is corrupt and cannot be reliably read. Future enhancement could add export/import functionality for manual backup.

8. **Question**: Should the "Fix All" button show a combined progress view, or handle userdata reset and downloads sequentially?
   - **Suggested Answer**: Handle sequentially - reset userdata first (fast operation), then proceed to download window. This keeps the UX simple and reuses existing download UI.
