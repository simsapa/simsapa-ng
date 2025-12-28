# PRD: Update Notifications for Application and Database

## Introduction/Overview

Users need to be informed when new versions of the Simsapa application or database are available. This feature will implement automatic update checking on startup and provide menu options to manually check for updates or disable automatic notifications.

**Problem:** Users are unaware when new versions of the application or database are released, potentially missing important updates, bug fixes, and new content.

**Goal:** Provide a non-intrusive update notification system that checks for updates on startup and allows users to manually check, with the ability to disable automatic checks.

## Goals

1. Automatically check for application and database updates on startup (with configurable disable option)
2. Display informative dialogs when updates are available, showing version info and release notes
3. Warn users when their local database is incompatible with the current application version
4. Provide manual "Check for Updates" menu option in the Help menu
5. Store user preference for automatic update checking in AppSettings
6. Use a new 'simsapa-ng' release channel to avoid interference with legacy Python version

## User Stories

1. **As a user**, I want to be notified when a new version of Simsapa is available, so that I can download the latest features and bug fixes.

2. **As a user**, I want to be notified when updated database content is available, so that I can optionally download new sutta translations.

3. **As a user**, I want to be warned if my database is incompatible with my app version, so that I understand why certain features may not work correctly.

4. **As a user**, I want to manually check for updates from a menu option, so that I can check at my convenience.

5. **As a user**, I want to disable automatic update checks, so that I'm not disturbed by notifications or to reduce network usage.

## Legacy Python Implementation Reference

### Python Files and Functions

The legacy implementation in `./simsapa-legacy` provides the reference implementation:

#### File: `simsapa/app/windows.py`

| Function | Description |
|----------|-------------|
| `_init_tasks()` | Called 500ms after window init, triggers update check initialization |
| `_init_check_simsapa_updates(include_no_updates=False)` | Creates worker, connects signals to handlers |
| `_check_simsapa_updates()` | Starts worker if `notify_about_simsapa_updates` setting is enabled |
| `_handle_check_simsapa_updates()` | Manual check handler (from menu), re-initializes with `include_no_updates=True` |
| `show_app_update_message(value: dict)` | Displays app update dialog with version info and GitHub link |
| `show_db_update_message(value: dict)` | Displays database update dialog, offers download option |
| `show_local_db_obsolete_message(value: dict)` | Warns about incompatible database version |
| `show_no_simsapa_updates_message()` | Shows "no updates" message (for manual checks only) |

#### File: `simsapa/app/check_simsapa_updates_worker.py`

| Class/Function | Description |
|----------------|-------------|
| `UpdatesWorkerSignals` | Qt signals: `have_app_update`, `have_db_update`, `local_db_obsolete`, `no_updates` |
| `CheckSimsapaUpdatesWorker` | QRunnable that performs update check in background thread |
| `CheckSimsapaUpdatesWorker.run()` | Main execution: connectivity check, fetch releases, compare versions, emit signals |

#### File: `simsapa/layouts/gui_helpers.py`

| Function | Description |
|----------|-------------|
| `to_version(ver: str) -> Version` | Parse version string into components (major, minor, patch, alpha) |
| `get_app_version() -> Optional[str]` | Get current application version |
| `get_db_version() -> Optional[str]` | Get database version from appdata.sqlite3 |
| `get_release_channel() -> str` | Determine release channel ('main', 'development', or env override) |
| `get_simsapa_releases_info(save_stats, screen_size) -> ReleasesInfo` | POST request to releases API |
| `get_latest_release(info, entry_type) -> Optional[ReleaseEntry]` | Get latest release for app/assets |
| `get_latest_app_compatible_assets_release(info) -> Optional[ReleaseEntry]` | Filter assets by app compatibility |
| `is_app_version_compatible_with_db_version(app, db) -> bool` | Check major.minor version match |
| `has_update(info, entry_type) -> Optional[UpdateInfo]` | Compare versions, return update info |
| `is_local_db_obsolete() -> Optional[UpdateInfo]` | Check if database is too old for app |

### Call Sequence (Legacy Python)

```
Window.__init__()
    │
    ├─► QTimer (500ms delay)
    │
    ▼
_init_tasks()
    │
    ├─► _init_check_simsapa_updates()  [Create worker, connect signals]
    │
    └─► _check_simsapa_updates()       [Start worker if setting enabled]
              │
              ▼
CheckSimsapaUpdatesWorker.run()  [Background thread]
    │
    ├─► HEAD request to test connectivity
    │
    ├─► POST request to /releases endpoint
    │
    ├─► is_local_db_obsolete() → emit local_db_obsolete signal
    │
    ├─► has_update(Application) → emit have_app_update signal
    │
    ├─► has_update(Assets) → emit have_db_update signal
    │
    └─► emit no_updates signal (if include_no_updates=True)
              │
              ▼
Signal Handlers → Show appropriate message dialog
```

### API Endpoint

```
POST https://simsapa.eu.pythonanywhere.com/releases
```

**Request Body:**
```json
{
    "channel": "simsapa-ng",
    "app_version": "0.1.0",
    "system": "Linux",
    "machine": "x86_64",
    "cpu_max": "3200.00",
    "cpu_cores": "8",
    "mem_total": "16777216000",
    "screen": "1920 x 1080",
    "no_stats": false
}
```

**Response:**
```json
{
    "application": {
        "releases": [
            {
                "date": "2024-04-05T15:00:00",
                "description": "Combined search",
                "github_repo": "simsapa/simsapa",
                "title": "Minor bugfix",
                "version_tag": "v0.5.2-alpha.1"
            }
        ]
    },
    "assets": {
        "releases": [
            {
                "date": "2025-06-07T06:42:30",
                "description": "Updated new sutta content from SuttaCentral.",
                "github_repo": "simsapa/simsapa-assets",
                "suttas_lang": ["af", "ar", "bn", "ca"],
                "title": "Updated SuttaCentral content",
                "version_tag": "v0.5.1-alpha.1"
            }
        ]
    }
}
```

## Functional Requirements

### Backend (Rust) Implementation

1. **FR-1:** Create a new module `backend/src/update_checker.rs` containing:
   - `Version` struct with `major`, `minor`, `patch`, `alpha` fields
   - `to_version(ver: &str) -> Result<Version>` function to parse version strings
   - `compare_versions(a: &Version, b: &Version) -> Ordering` function

2. **FR-2:** Implement `get_app_version() -> String` function:
   - Return the current application version from a constant or build-time value
   - Handle format like "v0.1.0" or "0.1.0-alpha.1"

3. **FR-3:** Implement `get_db_version() -> Option<String>` function:
   - Query appdata database `app_settings` table for 'db_version' key
   - Return None if database doesn't exist or version not found

4. **FR-4:** Implement `get_release_channel() -> String` function:
   - Check `RELEASE_CHANNEL` environment variable first
   - Check `release_channel` in AppSettings
   - Default to `"simsapa-ng"` (NOT 'main' or 'development' to avoid legacy interference)

5. **FR-5:** Implement `ReleasesRequestParams` struct and `ReleasesInfo` response types:
   ```rust
   struct ReleasesRequestParams {
       channel: String,
       app_version: String,
       system: String,
       machine: String,
       cpu_max: String,
       cpu_cores: String,
       mem_total: String,
       screen: String,
       no_stats: bool,
   }

   struct ReleaseEntry {
       version_tag: String,
       github_repo: String,
       suttas_lang: Vec<String>,
       date: String,
       title: String,
       description: String,
   }

   struct ReleasesInfo {
       application: ReleaseSection,
       assets: ReleaseSection,
   }
   ```

6. **FR-6:** Implement `fetch_releases_info(screen_size: &str, save_stats: bool) -> Result<ReleasesInfo>` function:
   - Collect system information (OS, architecture, CPU, memory)
   - Make POST request to `https://simsapa.eu.pythonanywhere.com/releases`
   - Parse JSON response into `ReleasesInfo`
   - Handle network errors gracefully, return Err with descriptive message

7. **FR-7:** Implement `is_app_version_compatible_with_db_version(app: &Version, db: &Version) -> bool`:
   - Return true if major and minor version numbers match
   - Patch/alpha differences are acceptable

8. **FR-8:** Implement `has_app_update(info: &ReleasesInfo) -> Option<UpdateInfo>`:
   - Get latest application release from `info.application.releases[0]`
   - Compare with current app version using semver
   - Return `UpdateInfo` with version, message, and visit_url if update available

9. **FR-9:** Implement `has_db_update(info: &ReleasesInfo) -> Option<UpdateInfo>`:
   - Filter releases by app compatibility (matching major.minor)
   - Get latest compatible release
   - Compare with current db version
   - Return `UpdateInfo` if update available

10. **FR-10:** Implement `is_local_db_obsolete() -> Option<UpdateInfo>`:
    - Compare app version with db version
    - Return warning if db major.minor is less than app major.minor

### App Settings

11. **FR-11:** Add to `AppSettings` struct in `backend/src/app_settings.rs`:
    ```rust
    pub notify_about_simsapa_updates: bool,  // Default: true
    pub release_channel: Option<String>,      // Default: None (uses "simsapa-ng")
    ```

12. **FR-12:** Add getter/setter functions in `AppData`:
    - `get_notify_about_simsapa_updates() -> bool`
    - `set_notify_about_simsapa_updates(enabled: bool)`

### Bridge Implementation

13. **FR-13:** Add update check functions to `sutta_bridge.rs`:
    
    **State tracking:**
    ```rust
    // Property to track if updates were already checked this session
    updates_checked: bool,
    ```
    
    **Signals:**
    ```rust
    #[qsignal]
    fn app_update_available(self: Pin<&mut SuttaBridge>, update_info_json: QString);
    
    #[qsignal]
    fn db_update_available(self: Pin<&mut SuttaBridge>, update_info_json: QString);
    
    #[qsignal]
    fn local_db_obsolete(self: Pin<&mut SuttaBridge>, update_info_json: QString);
    
    #[qsignal]
    fn no_updates_available(self: Pin<&mut SuttaBridge>);
    
    #[qsignal]
    fn update_check_error(self: Pin<&mut SuttaBridge>, error_message: QString);
    ```
    
    **Functions:**
    ```rust
    #[qinvokable]
    fn check_for_updates(self: Pin<&mut Self>, include_no_updates: bool);
    
    #[qinvokable]
    fn get_updates_checked(self: &Self) -> bool;
    
    #[qinvokable]
    fn set_updates_checked(self: Pin<&mut Self>, checked: bool);
    
    #[qinvokable]
    fn get_notify_about_simsapa_updates(self: &Self) -> bool;
    
    #[qinvokable]
    fn set_notify_about_simsapa_updates(self: Pin<&mut Self>, enabled: bool);
    ```

14. **FR-14:** Implement `check_for_updates()` as async operation:
    - Spawn background thread
    - Perform connectivity check (HEAD request)
    - Fetch releases info
    - Run update checks
    - Emit appropriate signals with JSON payloads
    - Handle errors by emitting `update_check_error` signal

15. **FR-15:** Add corresponding qmllint stub functions in `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml`:
    ```qml
    signal appUpdateAvailable(string update_info_json)
    signal dbUpdateAvailable(string update_info_json)
    signal localDbObsolete(string update_info_json)
    signal noUpdatesAvailable()
    signal updateCheckError(string error_message)
    
    function check_for_updates(include_no_updates: bool) {}
    function get_updates_checked(): bool { return false; }
    function set_updates_checked(checked: bool) {}
    function get_notify_about_simsapa_updates(): bool { return true; }
    function set_notify_about_simsapa_updates(enabled: bool) {}
    ```

### Frontend (QML) Implementation

16. **FR-16:** Create `assets/qml/UpdateNotificationDialog.qml`:
    - ApplicationWindow with `flags: Qt.Dialog`
    - Modal dialog (blocks other windows)
    - Properties for: `dialog_type` (app/db/obsolete/no_updates), `version_info`, `release_notes`, `download_url`
    - Layout similar to `DatabaseValidationDialog.qml`
    - Different content and buttons based on dialog type

17. **FR-17:** Implement dialog variants in `UpdateNotificationDialog.qml`:
    
    **App Update Dialog:**
    - Title: "Application Update Available"
    - Show current version and new version
    - Show release notes (truncated to 500 chars if needed)
    - Buttons: "Download" (opens GitHub URL), "Close"
    
    **DB Update Dialog:**
    - Title: "Database Update Available"
    - Show that update is optional
    - Show release notes with available language updates
    - Buttons: "Download Now" (triggers download), "Later", "Skip This Version"
    
    **Obsolete DB Dialog:**
    - Title: "Database Compatibility Warning"
    - Explain that database needs update for full compatibility
    - Buttons: "Download Now", "Continue Anyway"
    
    **No Updates Dialog:**
    - Title: "No Updates Available"
    - Message: "Simsapa application and database are up to date."
    - Button: "OK"

18. **FR-18:** Add delayed update check in `SuttaSearchWindow.qml`:
    ```qml
    Timer {
        id: update_check_timer
        interval: 500  // 0.5 second delay
        repeat: false
        onTriggered: {
            if (!SuttaBridge.get_updates_checked()) {
                SuttaBridge.set_updates_checked(true);
                if (SuttaBridge.get_notify_about_simsapa_updates()) {
                    SuttaBridge.check_for_updates(false);
                }
            }
        }
    }
    
    Component.onCompleted: {
        // ... existing code ...
        update_check_timer.start();
    }
    ```

19. **FR-19:** Add signal handlers in `SuttaSearchWindow.qml`:
    ```qml
    Connections {
        target: SuttaBridge
        
        function onAppUpdateAvailable(update_info_json) {
            update_notification_dialog.show_app_update(update_info_json);
        }
        
        function onDbUpdateAvailable(update_info_json) {
            update_notification_dialog.show_db_update(update_info_json);
        }
        
        function onLocalDbObsolete(update_info_json) {
            update_notification_dialog.show_obsolete_warning(update_info_json);
        }
        
        function onNoUpdatesAvailable() {
            update_notification_dialog.show_no_updates();
        }
        
        function onUpdateCheckError(error_message) {
            console.log("Update check error:", error_message);
            // Silently fail on startup, only show error for manual checks
        }
    }
    ```

20. **FR-20:** Add to Help menu in `SuttaSearchWindow.qml`:
    ```qml
    Menu {
        id: help_menu
        title: "&Help"
        
        // ... existing items ...
        
        MenuSeparator {}
        
        CMenuItem {
            action: Action {
                text: "Check for Simsapa Updates..."
                onTriggered: SuttaBridge.check_for_updates(true)  // include_no_updates=true
            }
        }
        
        CMenuItem {
            action: Action {
                id: notify_about_updates
                text: "Notify About Simsapa Updates"
                checkable: true
                checked: true  // Updated in Component.onCompleted
                onToggled: SuttaBridge.set_notify_about_simsapa_updates(checked)
            }
        }
        
        MenuSeparator {}
        
        CMenuItem {
            action: Action {
                text: "&About"
                onTriggered: about_dialog.show()
            }
        }
    }
    ```

21. **FR-21:** Initialize menu checkbox state in `Component.onCompleted`:
    ```qml
    notify_about_updates.checked = SuttaBridge.get_notify_about_simsapa_updates();
    ```

### Build Configuration

22. **FR-22:** Add `UpdateNotificationDialog.qml` to `bridges/build.rs`:
    ```rust
    qml_files.push("../assets/qml/UpdateNotificationDialog.qml");
    ```

23. **FR-23:** Add tests in `backend/tests/test_update_checker.rs`:
    - Test version parsing for various formats
    - Test version comparison
    - Test compatibility checking
    - Test update detection logic

## Non-Goals (Out of Scope)

1. **NG-1:** Automatic downloading and installing of updates (user must download manually)

2. **NG-2:** DPD (Digital Pali Dictionary) update checking (to be implemented later per user request)

3. **NG-3:** In-app update installation mechanism

4. **NG-4:** Delta/incremental database updates (full database download only)

5. **NG-5:** Push notifications or system tray notifications

6. **NG-6:** Update scheduling or deferred update reminders

7. **NG-7:** Statistics collection customization UI (controlled by env variables only)

## Design Considerations

### UI/UX

- **Non-intrusive:** Update checks happen silently on startup; dialogs only appear when updates are available
- **User control:** Users can disable automatic checks and check manually at any time
- **Clear messaging:** Dialogs clearly explain what the update contains and what actions are available
- **Consistent styling:** Dialog follows existing app design patterns (similar to DatabaseValidationDialog)

### Release Channel

- **Default:** `simsapa-ng` - This is a new release channel specifically for this Rust+QML rewrite
- **Purpose:** Prevents interference with legacy Python Simsapa versions which use 'main' and 'development' channels
- **Override:** Can be changed via `RELEASE_CHANNEL` environment variable or app settings

### Version Compatibility

- App and database versions follow semver: `major.minor.patch` or `major.minor.patch-alpha.N`
- Compatibility requires matching `major.minor` - patch differences are allowed
- This allows content updates (patch version changes) without app updates

### Error Handling

- Network errors are logged but don't show error dialogs on startup (silent fail)
- Manual checks show error message if network fails
- Invalid JSON responses are handled gracefully

## Technical Considerations

### HTTP Client

Use `reqwest` with proper timeout configuration:
```rust
reqwest::Client::builder()
    .timeout(std::time::Duration::from_secs(30))
    .build()
```

### System Information Collection

Collect for statistics (can be disabled with `NO_STATS=true`):
- `std::env::consts::OS` - Operating system
- `std::env::consts::ARCH` - CPU architecture
- CPU cores from `sysinfo` crate if available
- Memory from `sysinfo` crate if available
- Screen size passed from QML

### Thread Safety

- Update check runs in background thread via `thread::spawn`
- Results communicated back via Qt signal queue
- `updates_checked` flag prevents multiple simultaneous checks

### Single Check Per Session

- The `updates_checked` flag on SuttaBridge ensures only one update check per app session
- Multiple SuttaSearchWindow instances share the same SuttaBridge singleton
- First window to check sets the flag, subsequent windows skip the check

## Success Metrics

1. **SM-1:** Update check completes within 30 seconds (including network latency)
2. **SM-2:** Users are correctly notified when updates are available
3. **SM-3:** No crashes or freezes during update check
4. **SM-4:** Update check setting persists correctly between sessions
5. **SM-5:** Multiple window instances don't trigger duplicate checks

## Open Questions

1. **OQ-1:** Should we cache the last successful releases response for offline scenarios?
   - **Recommendation:** Not for MVP, can be added later

2. **OQ-2:** Should we track "skipped versions" to not show the same update repeatedly?
   - **Recommendation:** Yes for DB updates (add "Skip This Version" button), not for app updates

3. **OQ-3:** Should the "Download" button for app updates open the browser or show a download progress?
   - **Recommendation:** Open browser to GitHub releases page (simpler, safer)

4. **OQ-4:** Should we show a brief "checking for updates" indicator?
   - **Recommendation:** No, keep it silent for better UX

5. **OQ-5:** What should happen if the API response format changes?
   - **Recommendation:** Fail gracefully with error logging, don't crash

## Implementation Phases

**Phase 1: Backend Core**
- Create `update_checker.rs` module
- Implement version parsing and comparison
- Implement release info fetching
- Add tests for version logic

**Phase 2: App Settings**
- Add `notify_about_simsapa_updates` to AppSettings
- Implement getter/setter in AppData
- Verify persistence works correctly

**Phase 3: Bridge Integration**
- Add signals and functions to SuttaBridge
- Implement async check_for_updates
- Add `updates_checked` tracking
- Create qmllint stub file updates

**Phase 4: Dialog Implementation**
- Create UpdateNotificationDialog.qml
- Implement all dialog variants
- Style consistently with existing dialogs

**Phase 5: Window Integration**
- Add Timer and signal handlers to SuttaSearchWindow
- Add Help menu items
- Test complete flow

**Phase 6: Testing and Polish**
- Test on all platforms
- Test error scenarios
- Test with slow/no network
- Verify single-check-per-session behavior

## Appendix: Data Types

### UpdateInfo (returned to QML as JSON)

```json
{
    "version": "v0.2.0",
    "message": "New version available with improved search features.",
    "visit_url": "https://github.com/simsapa/simsapa/releases/tag/v0.2.0",
    "current_version": "v0.1.0",
    "release_notes": "Bug fixes and performance improvements...",
    "languages": ["en", "pli", "de"]
}
```

### Version String Formats

Supported version string formats:
- `0.1.0`
- `v0.1.0`
- `0.1.0-alpha.1`
- `v0.1.0-alpha.1`

All are normalized to `Version { major: 0, minor: 1, patch: 0, alpha: Some(1) }` structure.
