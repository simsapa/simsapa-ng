# Tasks: Keep Screen On During Asset Install (replace wake lock with FLAG_KEEP_SCREEN_ON)

PRD: [2026-06-11-152051-prd-keep-screen-on-during-asset-install.md](./2026-06-11-152051-prd-keep-screen-on-during-asset-install.md)

## Relevant Files

- `cpp/screen.h` - **New.** Declares the `keep_screen_on(bool)` helper (mirrors `cpp/wake_lock.h`).
- `cpp/screen.cpp` - **New.** Implements `keep_screen_on(bool)` via `QJniObject` + `FLAG_KEEP_SCREEN_ON` on the Android UI thread; no-op on desktop.
- `cpp/wake_lock.h` - **Delete.** Old PowerManager wake-lock helper header.
- `cpp/wake_lock.cpp` - **Delete.** Old PowerManager wake-lock implementation.
- `cpp/android_helpers.cpp` / `cpp/android_helpers.h` - Reference for the JNI helper + bridge pattern (and `open_android_display_settings()` which is being unwired from the warning screen).
- `CMakeLists.txt` - Add `cpp/screen.cpp` to `cpp_files`; remove `cpp/wake_lock.cpp`.
- `bridges/src/asset_manager.rs` - Add `keep_screen_on` to the `extern "C++"` block + `set_keep_screen_on(bool)` invokable; remove the three `*wake_lock*` C++ decls and the `*_rust` wrappers.
- `bridges/src/sutta_bridge.rs` - Remove the `*wake_lock*` invokable decls and impls (lines ~1098-1104, ~3676-3685).
- `assets/qml/com/profoundlabs/simsapa/AssetManager.qml` - Add `set_keep_screen_on` stub; remove wake-lock stubs.
- `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml` - Remove wake-lock stubs.
- `assets/qml/DownloadAppdataWindow.qml` - Swap wake-lock calls for keep-screen-on; remove `wake_lock_acquired`; revise Idx 2 warning screen; add Back guard.
- `assets/qml/SuttaLanguagesWindow.qml` - Swap wake-lock calls for keep-screen-on; remove `wake_lock_acquired`; add removal "in progress" flag; add Back guard.
- `assets/qml/DownloadProgressFrame.qml` - Remove `wake_lock_acquired` property and the obsolete "tap the device periodically" message; keep the "keep the app in the foreground" note.
- `assets/qml/AppSettingsWindow.qml` - Remove `wake_lock_acquired` property, the commented-out wake-lock debug UI, and the `is_wake_lock_acquired_rust()` call.
- `android/AndroidManifest.xml` - Remove the `WAKE_LOCK` `uses-permission` line.

### Notes

- The new C++ helper follows the **exact build pattern of `cpp/wake_lock.cpp`**: it is compiled by `CMakeLists.txt` (in `cpp_files`) and its header is `include!`-ed from the CXX-Qt bridge (`asset_manager.rs`). It is **not** added to `bridges/build.rs` (that list only carries `utils.cpp` / `system_palette.cpp` / `gui.cpp`).
- New QML components are not being added, so no `bridges/build.rs` `qml_files` change is needed. The qmllint type stub for the new bridge function **is** required (see `CLAUDE.md`).
- Per project rules: build only with `make build -B`; run tests only after all sub-tasks of a top-level task are done; do not run `make qml-test` unless asked; use `try_exists()` not `.exists()`; QML logging via `Logger`, not `console.*` (except the `com/profoundlabs/simsapa/` stub files).
- `CLAUDE.md` is a symlink — edit `AGENTS.md` if instructions there need updating (not expected for this feature).

## Tasks

### Specs & dependencies for 1.0

- **Helper API:** `void keep_screen_on(bool on)` in namespace-free form matching `wake_lock.cpp` (free functions `acquire_wake_lock()` etc. are declared without a namespace and `include!`-ed directly). Use the same `extern "C"` logging shims (`log_info_c` / `log_error_c`) already used in `wake_lock.cpp`.
- **JNI body:** on `Q_OS_ANDROID`, run on the UI thread via `QtAndroidPrivate::runOnAndroidMainThread`; get the activity from `QNativeInterface::QAndroidApplication::context()` (or `QtNative.activity()` as `wake_lock.cpp` does), call `getWindow()`, then `addFlags`/`clearFlags` with `FLAG_KEEP_SCREEN_ON = 0x00000080`. Non-Android: log + return.
- **Bridge:** add `fn keep_screen_on(on: bool);` to the existing `unsafe extern "C++"` block in `asset_manager.rs` (alongside the `android_helpers.h` includes), and a `#[qinvokable] fn set_keep_screen_on(self: Pin<&mut AssetManager>, on: bool);` that delegates to it.
- **Dependency:** none (additive). Must compile on desktop and Android before later tasks remove the old code.

- [x] 1.0 Add the `keep_screen_on` C++ helper and expose it through the AssetManager bridge (FLAG_KEEP_SCREEN_ON)
  - [x] 1.1 Create `cpp/screen.h` declaring `bool`/`void keep_screen_on(bool on);` (header guard, mirror `cpp/wake_lock.h`).
  - [x] 1.2 Create `cpp/screen.cpp` implementing `keep_screen_on(bool)`: `#ifdef Q_OS_ANDROID` set/clear `FLAG_KEEP_SCREEN_ON` (0x80) on the activity window, marshalled onto the Android UI thread; `#else` log and return. Reuse the `extern "C" log_info_c/log_error_c` pattern from `wake_lock.cpp`.
  - [x] 1.3 Add `cpp/screen.cpp` to the `cpp_files` list in `CMakeLists.txt`.
  - [x] 1.4 In `bridges/src/asset_manager.rs`, add `include!("screen.h");` and `fn keep_screen_on(on: bool);` to the `unsafe extern "C++"` block.
  - [x] 1.5 In `bridges/src/asset_manager.rs`, declare `#[qinvokable] fn set_keep_screen_on(self: Pin<&mut AssetManager>, on: bool);` and implement it to call `qobject::keep_screen_on(on)`.
  - [x] 1.6 Add the qmllint stub `function set_keep_screen_on(on: bool) {}` to `assets/qml/com/profoundlabs/simsapa/AssetManager.qml`.
  - [x] 1.7 `make build -B` to confirm the new helper + bridge compile cleanly.

### Specs & dependencies for 2.0

- **State:** both windows are `ApplicationWindow` with `is_mobile` already defined and an `AssetManager { id: manager }` instance. They currently call `manager.acquire_wake_lock_rust()` in `Component.onCompleted` and `manager.release_wake_lock_rust()` in `Component.onDestruction`.
- **Scope decision (PRD FR6-8):** keep-screen-on is set for the whole window lifetime, gated on `is_mobile`.
- **Dependency:** 1.0 (the `set_keep_screen_on` invokable must exist). This task leaves the old wake-lock code present but unused (still compiles).

- [x] 2.0 Wire keep-screen-on into the install windows and remove the wake-lock calls from QML
  - [x] 2.1 In `DownloadAppdataWindow.qml` `Component.onCompleted`, replace `root.wake_lock_acquired = manager.acquire_wake_lock_rust();` with `manager.set_keep_screen_on(true);` (still gated on `root.is_mobile`).
  - [x] 2.2 In `DownloadAppdataWindow.qml` `Component.onDestruction`, replace `manager.release_wake_lock_rust();` with `manager.set_keep_screen_on(false);`.
  - [x] 2.3 In `SuttaLanguagesWindow.qml` `Component.onCompleted`, replace the `acquire_wake_lock_rust()` call with `manager.set_keep_screen_on(true);` (gated on `root.is_mobile`).
  - [x] 2.4 In `SuttaLanguagesWindow.qml` `Component.onDestruction`, replace `release_wake_lock_rust()` with `manager.set_keep_screen_on(false);`.
  - [x] 2.5 `make build -B` to confirm the windows compile with the new calls.

### Specs & dependencies for 3.0

- **Removal surface (from PRD FR9-13):** `cpp/wake_lock.{cpp,h}`; `CMakeLists.txt` entry; `asset_manager.rs` C++ decls (`acquire_wake_lock`/`release_wake_lock`/`is_wake_lock_acquired`) + `acquire_wake_lock_rust`/`release_wake_lock_rust` invokables and impls; `sutta_bridge.rs` `acquire_wake_lock_rust`/`release_wake_lock_rust`/`is_wake_lock_acquired_rust` decls + impls; qmllint stubs in `AssetManager.qml` and `SuttaBridge.qml`; `wake_lock_acquired` properties in `DownloadAppdataWindow.qml`, `SuttaLanguagesWindow.qml`, `DownloadProgressFrame.qml`, `AppSettingsWindow.qml` (+ commented debug block + `is_wake_lock_acquired_rust()` call in `AppSettingsWindow.qml`); `WAKE_LOCK` permission in `AndroidManifest.xml`.
- **Dependency:** 2.0 (all QML callers of the wake-lock invokables must be gone first, so removing the bridge functions doesn't break QML/qmllint). Make targeted per-site edits (no bulk `sed`).

- [x] 3.0 Remove the old PowerManager wake-lock implementation, bridge functions, QML stubs/properties, and WAKE_LOCK permission
  - [x] 3.1 Delete `cpp/wake_lock.cpp` and `cpp/wake_lock.h`; remove `cpp/wake_lock.cpp` from `CMakeLists.txt`.
  - [x] 3.2 In `bridges/src/asset_manager.rs`, remove the `include!("wake_lock.h")` line and the `acquire_wake_lock` / `release_wake_lock` / `is_wake_lock_acquired` decls; remove the `acquire_wake_lock_rust` / `release_wake_lock_rust` invokable decls and their `impl` bodies.
  - [x] 3.3 In `bridges/src/sutta_bridge.rs`, remove the `acquire_wake_lock_rust` / `release_wake_lock_rust` / `is_wake_lock_acquired_rust` invokable decls (~1098-1104) and their `impl` bodies (~3676-3685).
  - [x] 3.4 Remove the wake-lock qmllint stubs from `assets/qml/com/profoundlabs/simsapa/AssetManager.qml` and `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml`.
  - [x] 3.5 Remove the `property bool wake_lock_acquired` and any remaining references from `DownloadAppdataWindow.qml` (incl. the `wake_lock_acquired:` binding passed to `DownloadProgressFrame`) and `SuttaLanguagesWindow.qml`.
  - [x] 3.6 In `DownloadProgressFrame.qml`, remove the `property bool wake_lock_acquired` and the commented-out wake-lock status `Label` block.
  - [x] 3.7 In `AppSettingsWindow.qml`, remove the `property bool wake_lock_acquired`, the commented-out wake-lock debug button/label block, and the `is_wake_lock_acquired_rust()` call in `Component.onCompleted` (around line 976).
  - [x] 3.8 Remove `<uses-permission android:name="android.permission.WAKE_LOCK" />` from `android/AndroidManifest.xml`.
  - [x] 3.9 `make build -B` to confirm a clean build with all wake-lock code removed.

### Specs & dependencies for 4.0

- **Text targets (PRD FR14-16):** in `DownloadProgressFrame.qml`, the "Switching apps or suspend mode ... Tap the device periodically ..." `Label`; keep the "Please keep the app in the foreground ..." note. In `DownloadAppdataWindow.qml` Idx 2 ("Large download warning"), remove the "Open Settings > Display > Screen timeout ..." `Text` and the "Open Settings" `Button` (which calls `manager.open_display_settings()`); keep the ~700 MB / Wi-Fi quota warning.
- **Dependency:** none hard, but do after 3.0 to avoid editing the same files twice. Note: `open_display_settings()` / `open_android_display_settings()` may become unused after removing the button — leave the bridge function in place unless it has no other callers (verify; out of scope to remove if used elsewhere).

- [x] 4.0 Revise the obsolete "tap the screen" / screen-timeout UI text and buttons
  - [x] 4.1 In `DownloadProgressFrame.qml`, delete the `Label` containing "Switching apps or suspend mode (black screen) ... Tap the device periodically ...".
  - [x] 4.2 Confirm the `Label` with "Please keep the app in the foreground during the download and extract process." is **kept**.
  - [x] 4.3 In `DownloadAppdataWindow.qml` Idx 2, delete the `Text` with the "Settings > Display > Screen timeout" instruction.
  - [x] 4.4 In `DownloadAppdataWindow.qml` Idx 2 button area, delete the "Open Settings" `Button` (and its `onClicked: manager.open_display_settings()`); keep the "Continue" button and the ~700 MB / Wi-Fi warning text.
  - [x] 4.5 Note: after 4.4 the only caller of `open_display_settings()` is gone, so `open_display_settings` (`asset_manager.rs`), `open_android_display_settings` (`android_helpers.{cpp,h}`), and the `AssetManager.qml` stub become dead code. Removing them is safe and optional — do it only if it keeps the build clean; otherwise leave a code comment noting they are now unused.
  - [x] 4.6 `make build -B` to confirm the QML still loads/compiles.

### Specs & dependencies for 5.0

- **Back interception (PRD FR17-19, Technical Considerations):** `ApplicationWindow` emits `onClosing(close)` when the Android Back button requests the window to close; setting `close.accepted = false` cancels it. Use this to intercept Back and, when an operation is active, open a confirm dialog instead of closing; when idle, allow the default close.
- **"Operation active" state:**
  - `DownloadAppdataWindow.qml`: active while the progress view is showing (`views_stack.currentIndex === 3`) and not yet completed (Idx 4). Introduce a clear `property bool operation_active` driven by the download lifecycle (set true in `run_download()` / `start_redownload()`, set false in `onDownloadsCompleted` success and on retry-exhausted/error states).
  - `SuttaLanguagesWindow.qml`: already has `is_downloading`; add an analogous `is_removing` flag set true in `perform_removal()` and cleared in `onRemovalCompleted`. Treat `is_downloading || is_removing` as active.
- **Confirm dialog:** reuse the existing modal `Dialog` style; standardButtons `Yes`/`No` (or Ok/Cancel). On confirm, run the window's existing close/quit path; on cancel, keep the window.
- **Dependency:** 2.0/3.0 (windows already cleaned up). Gate Back handling so desktop is unaffected (Back/close on desktop behaves as today).

- [x] 5.0 Add the Android Back-button guard during active download/extract/import and language removal operations
  - [x] 5.1 In `DownloadAppdataWindow.qml`, add `property bool operation_active: false`; set it `true` when a download starts (`run_download()` and `start_redownload()`) and `false` on completion (Idx 4 reached) and on terminal error/retry-exhausted paths.
  - [x] 5.2 In `DownloadAppdataWindow.qml`, add `onClosing(close)`: if `root.is_mobile && root.operation_active`, set `close.accepted = false` and open a confirm dialog; otherwise allow default close.
  - [x] 5.3 Add a modal confirm `Dialog` to `DownloadAppdataWindow.qml` ("An operation is in progress. Leaving now may interrupt it. Stop anyway?") whose accept action quits/closes the window and whose reject keeps it.
  - [x] 5.4 In `SuttaLanguagesWindow.qml`, add `property bool is_removing: false`; set `true` in `perform_removal()`, clear in `onRemovalCompleted` (both success and failure branches).
  - [x] 5.5 In `SuttaLanguagesWindow.qml`, add `onClosing(close)`: if `root.is_mobile && (root.is_downloading || root.is_removing)`, set `close.accepted = false` and open a confirm dialog; otherwise allow default close.
  - [x] 5.6 Add a modal confirm `Dialog` to `SuttaLanguagesWindow.qml` with the same wording/behaviour; accept runs the existing close/cancel path, reject keeps the window.
  - [x] 5.7 Verify desktop behaviour is unchanged (guards are gated on `is_mobile`); `make build -B`.
  - [x] 5.8 After all sub-tasks above are complete, run the backend tests (`cd backend && cargo test`) to confirm the build/tests pass.
