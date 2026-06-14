# PRD: Keep Screen On During Asset Install (replace wake lock with FLAG_KEEP_SCREEN_ON)

## 1. Introduction / Overview

When the app first launches it must download and extract ~700 MB of database
assets, and the **Sutta Languages** window lets the user download/import or
remove additional sutta language databases later. On Android these operations
can take several minutes.

The current code tries to prevent the device from suspending (black screen,
which interrupts the download/extract) by acquiring an Android **`PowerManager`
`PARTIAL_WAKE_LOCK`** (`acquire_wake_lock_rust()` in `DownloadAppdataWindow.qml`
and `SuttaLanguagesWindow.qml`). **This technique does not work** — the device
still goes into suspend even while the wake lock is held, and the UI even tells
users to "tap the device periodically" to keep it awake.

This feature replaces the wake lock with the **`FLAG_KEEP_SCREEN_ON` window
flag** technique (the approach recommended by Android's official guidance and
used by the Fossify Gallery app). Setting this flag on the activity window keeps
the screen on and at full brightness for as long as the window is visible, needs
no permission, and cannot leak. See
[`tasks/keep-screen-on-qt.md`](./keep-screen-on-qt.md) for the full technical
write-up and reference links.

Because keeping the screen on does **not** stop a user from manually pressing
Back (which can interrupt download/extract/import or removal), this feature also
adds a **Back-button guard** during active operations. The misleading "tap the
screen" instructions are removed.

## 2. Goals

1. Prevent the Android screen from dimming/suspending while the asset
   download/extract/import or language removal windows are open, using
   `FLAG_KEEP_SCREEN_ON` instead of a `WakeLock`.
2. Completely remove the old `PowerManager` wake-lock implementation and its
   `WAKE_LOCK` permission.
3. Remove the now-incorrect UI text that tells users to tap the screen or raise
   the system screen-timeout setting.
4. Reduce accidental interruptions of an in-progress operation by guarding the
   Back button while a download/extract/import or removal is running.

## 3. User Stories

- **As a new Android user setting up the app**, I want the screen to stay on by
  itself while the 700 MB download and extraction runs, so the install
  completes without me having to keep tapping the screen.
- **As a user adding extra languages** in the Sutta Languages window, I want the
  screen to stay on during the download and import so the process isn't
  interrupted.
- **As a user who accidentally pressed Back** during a download or a language
  removal, I want to be warned before the window closes, so I don't abort the
  operation by mistake.
- **As a returning user**, I don't want the app to keep my screen on or hold any
  wake resource once the install windows are closed, so my battery and normal
  screen-timeout behaviour are unaffected.

## 4. Functional Requirements

### Keep-screen-on mechanism

1. The system must provide a single cross-platform interface
   `keep_screen_on(bool on)` (C++ helper) exposed to QML/Rust through an
   invokable such as `set_keep_screen_on(bool)`.
2. On Android, `keep_screen_on(true)` must add
   `WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON` (value `0x80`) to the
   activity's window, and `keep_screen_on(false)` must clear it.
3. The window-flag change must be executed on the Android UI (main) thread
   (e.g. via `QtAndroidPrivate::runOnAndroidMainThread`), reaching the activity
   through `QJniObject` / `QNativeInterface::QAndroidApplication::context()`.
4. On non-Android platforms (Linux, Windows, macOS) the interface must be a
   safe no-op for now (guarded by `#ifdef Q_OS_ANDROID` in C++ and/or
   `#[cfg(target_os = "android")]` in Rust). No desktop screen-saver inhibitor
   is implemented in this feature.
5. The C++ helper must be registered in the build the same way the existing
   helpers are: added to the `cpp/` source list in `CMakeLists.txt` and exposed
   to the CXX-Qt bridge (declared in `asset_manager.rs`'s `extern "C++"` block,
   following the existing `wake_lock.h` / `android_helpers.h` pattern). The
   `qmllint` type stub (`AssetManager.qml`, and `SuttaBridge.qml` if exposed
   there) must declare the new invokable.

### Wiring to window lifecycle (whole-window scope)

6. `DownloadAppdataWindow.qml` must call `set_keep_screen_on(true)` when the
   window is created (`Component.onCompleted`) on mobile, and
   `set_keep_screen_on(false)` when the window is destroyed
   (`Component.onDestruction`) — replacing the current
   `acquire_wake_lock_rust()` / `release_wake_lock_rust()` calls.
7. `SuttaLanguagesWindow.qml` must do the same: enable on
   `Component.onCompleted`, disable on `Component.onDestruction`.
8. The flag is set for the **whole lifetime of these two windows** (not gated to
   only the active download), matching the existing wake-lock wiring. (No new
   user setting is introduced.)

### Removal of the old wake-lock implementation

9. Delete the wake-lock C++ implementation: `cpp/wake_lock.cpp` and
   `cpp/wake_lock.h`, and remove `cpp/wake_lock.cpp` from `CMakeLists.txt`.
10. Remove the wake-lock declarations and invokables from the bridges:
    `acquire_wake_lock` / `release_wake_lock` / `is_wake_lock_acquired` and the
    `*_rust` wrappers in `bridges/src/asset_manager.rs` and
    `bridges/src/sutta_bridge.rs`.
11. Remove the corresponding `qmllint` stub functions from
    `assets/qml/com/profoundlabs/simsapa/AssetManager.qml` and
    `SuttaBridge.qml`.
12. Remove the `wake_lock_acquired` properties and all references to them from
    `DownloadAppdataWindow.qml`, `SuttaLanguagesWindow.qml`,
    `DownloadProgressFrame.qml`, and `AppSettingsWindow.qml` (including the
    commented-out wake-lock debug UI block in `AppSettingsWindow.qml`).
13. Remove the `<uses-permission android:name="android.permission.WAKE_LOCK" />`
    line from `android/AndroidManifest.xml`.

### UI text revisions

14. In `DownloadProgressFrame.qml`, remove the message that instructs the user
    to "Tap the device periodically to keep it awake and prevent screen timeout
    suspend." (the obsolete tap-to-stay-awake warning).
15. In `DownloadAppdataWindow.qml` (Idx 2 "Large download warning"), remove the
    instruction to open **Settings > Display > Screen timeout** and increase it,
    and remove the **"Open Settings"** button and its `open_display_settings()`
    wiring on that screen (the screen-timeout workaround is no longer needed
    because the screen now stays on automatically). The Wi-Fi / mobile-data
    quota warning text must be kept.
16. The note **"Please keep the app in the foreground during the download and
    extract process."** in `DownloadProgressFrame.qml` must be **kept** (it is
    still accurate — keep-screen-on does not prevent manual app switching).

### Back-button guard (Android)

17. While a download/extract/import operation (in `DownloadAppdataWindow` and in
    `SuttaLanguagesWindow`) **or** a language removal operation (in
    `SuttaLanguagesWindow`) is **actively running**, pressing the Android Back
    button must **not** immediately close the window. Instead it must show a
    warning dialog (e.g. "An operation is in progress. Leaving now may interrupt
    it. Are you sure you want to stop?") with confirm / cancel actions.
18. If the user confirms, the existing close/cancel behaviour runs (for the
    download window, `Qt.quit()` / close; for the languages window, the existing
    close / cancel path). If the user cancels, the window stays and the
    operation continues.
19. When **no** operation is running, Back behaves normally (closes the window /
    quits as it does today).

### Cross-platform safety

20. None of the new behaviour (Back guard, keep-screen-on) may break or change
    behaviour on desktop builds. The Android-specific guards must be gated on
    `is_mobile` / `Q_OS_ANDROID` as appropriate.

## 5. Non-Goals (Out of Scope)

- Implementing desktop screen-saver/suspend inhibitors (Linux D-Bus, Windows
  `SetThreadExecutionState`, macOS `IOPMAssertion`). The interface is a no-op on
  desktop for now.
- Intercepting the Android **Home** or **Overview/Recents** buttons — this is
  not possible for an app on Android; only Back can be intercepted.
- Detecting that the app was backgrounded and warning the user on return.
- Any resume/restart handling of interrupted downloads, extraction, or imports
  (no recovery logic, no HTTP `Range`/`.part` resume, no persistence of an
  interrupted-install state). The existing per-URL retry button behaviour is
  left exactly as it is.
- Adding a user-facing setting to toggle keep-screen-on; it is always on while
  the two install windows are open.
- Changing the storage-selection flow, the bundle/language selection UI, or the
  download URL/versioning logic, except for the specific text/button removals
  listed above.

## 6. Design Considerations

- Follow the existing pattern: a small C++ helper (mirroring
  `cpp/android_helpers.cpp` / `cpp/wake_lock.cpp`) exposed through
  `asset_manager.rs`. The reference C++ body is in
  [`tasks/keep-screen-on-qt.md`](./keep-screen-on-qt.md) (Option A / Option B).
- Reuse the existing `error_dialog` / `Dialog` style already present in
  `DownloadAppdataWindow.qml` and `SuttaLanguagesWindow.qml` for the new
  Back-guard warning dialog, so the look and feel is consistent (point sizes via
  `root.pointSize`, modal dialogs, word wrap).
- QML logging must use the `Logger { id: logger }` module, not `console.*`
  (except in the `com/profoundlabs/simsapa/` type-stub files).
- The "active operation" state already exists in part: `SuttaLanguagesWindow`
  has `is_downloading` (and should treat an in-progress removal as active too);
  `DownloadAppdataWindow` drives `views_stack.currentIndex === 3` for the
  progress view. The Back guard should key off a clear "operation in progress"
  boolean in each window.

## 7. Technical Considerations

- **Build registration:** new `cpp/screen.cpp` (or similarly named) must be
  added to `CMakeLists.txt`'s `cpp_files` and declared in `asset_manager.rs`'s
  `extern "C++"` block. New invokables need matching `qmllint` stubs (see
  `CLAUDE.md` rules for bridge functions and QML type stubs).
- **Android UI thread:** window-flag changes must run on the Android main
  thread; do not call `addFlags`/`clearFlags` directly from a worker thread.
- **File existence checks:** any new Rust file/dir checks must use
  `try_exists()` (Android-safe), per project rules.
- **Back-button capture in QML:** on Android the Back key arrives as a
  `Qt.Key_Back` key event / the window's close request; the window must
  intercept it (e.g. `onClosing` with `close.accepted = false`, or a `Keys`
  handler / `Shortcut`) and route to the guard dialog when an operation is
  active, otherwise allow the default close.
- **Removal operation state:** `SuttaLanguagesWindow` currently sets
  `is_downloading` for downloads but the removal path
  (`perform_removal()` → `remove_sutta_languages()`) does not have an analogous
  "active" flag; an equivalent in-progress flag for removal is needed so the
  Back guard covers it.

## 8. Success Metrics

1. On a physical Android device, starting the initial 700 MB install and not
   touching the device, the screen stays on and the install completes without
   suspend interrupting it.
2. The `WAKE_LOCK` permission no longer appears in the built APK's manifest, and
   no `wake_lock` symbols remain in the codebase.
3. Pressing Back during an active download or language removal shows the
   confirmation dialog rather than silently aborting; pressing Back when idle
   closes the window as before.
4. Desktop builds compile and behave exactly as before (keep-screen-on is a
   no-op; no Back guard dialog).

## 9. Open Questions

1. Confirm the exact place the keep-screen-on invokable should live — on
   `AssetManager` (used by both windows) vs. a new dedicated `ScreenController`
   bridge as suggested in `keep-screen-on-qt.md`.
2. Confirm the cleanest QML mechanism to intercept Back on Android for an
   `ApplicationWindow` with `flags: Qt.Dialog` (`onClosing` vs. a `Keys`/
   `Shortcut` handler), so the guard works consistently for both windows.
