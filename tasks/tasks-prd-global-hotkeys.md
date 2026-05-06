# Tasks: Global Hotkeys for Dictionary Lookup

Source PRD: [`prd-global-hotkeys.md`](./prd-global-hotkeys.md)

## Relevant Files

### New files

- `cpp/global_hotkey_manager.h` — Cross-platform `GlobalHotkeyManager` Qt class declaration: `registerHotkey(QKeySequence, int)`, `unregisterAll()`, signal `hotkeyActivated(int)`, shared `state2` double-tap state.
- `cpp/global_hotkey_manager.cpp` — Cross-platform shared logic (parsing `QKeySequence`, double-tap state machine, dispatch to platform backend, header crediting Goldendict-ng GPLv3 as reference).
- `cpp/global_hotkey_x11.cpp` — Linux X11 backend using the `XRecord` extension on a `QThread`, queued signal back to main thread. Built only when `WITH_X11` is defined and `QGuiApplication::platformName() == "xcb"`.
- `cpp/global_hotkey_win.cpp` — Windows backend using `RegisterHotKey`/`UnregisterHotKey` plus `SendInput` re-emit, with brief unregister/re-register around the synthesized event.
- `cpp/global_hotkey_mac.mm` — macOS backend using Carbon `RegisterEventHotKey`, AX selected-text via `AXUIElementCopyAttributeValue` / `kAXSelectedTextAttribute`, fallback `CGEventCreateKeyboardEvent` for `⌘C`, and `AXIsProcessTrusted()` permission prompt.
- `bridges/src/global_hotkey_manager.rs` — Rust CXX-Qt bridge that owns the C++ `GlobalHotkeyManager`, exposes settings JSON API, emits an activation signal, and triggers the lookup pipeline.
- `assets/qml/com/profoundlabs/simsapa/GlobalHotkeyManager.qml` — `qmllint` QML stub for the new bridge type (per `CLAUDE.md`).
- `assets/qml/GlobalHotkeysSection.qml` — Extracted QML component for the "Global Hotkeys" sub-section, rendered above the existing keybindings list in `AppSettingsWindow.qml`.
- `backend/src/global_hotkeys.rs` — Persistence layer: load/save the global-hotkey settings to `SIMSAPA_DIR/global_hotkeys.json`, with defaults `{ "enabled": false, "bindings": { "dictionary_lookup": "Ctrl+C+C" } }`.

### Modified files

- `bridges/src/lib.rs` — Register the new `global_hotkey_manager` module.
- `bridges/build.rs` — Add `src/global_hotkey_manager.rs` to `rust_files`; add `../assets/qml/GlobalHotkeysSection.qml` to `qml_files`.
- `assets/qml/com/profoundlabs/simsapa/qmldir` — Declare `GlobalHotkeyManager` QML type.
- `CMakeLists.txt` — Add `cpp/global_hotkey_manager.cpp` and the platform-conditional `cpp/global_hotkey_x11.cpp` / `cpp/global_hotkey_win.cpp` / `cpp/global_hotkey_mac.mm` to the `app_files` list; add platform link libs (`X11`, `Xtst` for X11 RECORD extension; `Carbon`, `AppKit`, `ApplicationServices` frameworks on macOS).
- `cpp/gui.h`, `cpp/gui.cpp` — Add `callback_global_hotkey_activated()` (or extend `callback_run_lookup_query`) used by the bridge to drive the raise + lookup pipeline. Reuse existing `callback_run_lookup_query`.
- `cpp/main.cpp` — Construct the `GlobalHotkeyManager` early (or via the bridge) so it lives for the app lifetime.
- `cpp/window_manager.cpp` / `cpp/window_manager.h` — Add a "raise main window" helper if the existing one is not sufficient for cross-workspace activation.
- `assets/qml/AppSettingsWindow.qml` — Insert `GlobalHotkeysSection` above the existing keybindings list; hide it on Wayland and show the localhost-API note instead.
- `assets/qml/KeybindingCaptureDialog.qml` — Add `is_global` (or `allow_double_tap`) property, double-tap capture state with ~500 ms timeout, and instructional text variants.
- `backend/src/app_data.rs` / `backend/src/lib.rs` — Wire `global_hotkeys.rs` into `AppData` and ensure the JSON file path uses `try_exists()` per the project's Android-safety rule.
- `bridges/src/api.rs` — Already has `/lookup_window_query` GET and POST; verify they are reachable on all platforms (no platform gating needed for Wayland workaround).

### Test files

- `backend/src/global_hotkeys.rs` (`#[cfg(test)]` module) — Unit tests for default JSON, round-trip serialise/deserialise, and missing-file fallback.
- `bridges/src/global_hotkey_manager.rs` (`#[cfg(test)]` module) — Unit tests for sequence parsing/validation and the JSON shape returned by `get_global_hotkeys_json()`.
- `tests/qml/tst_KeybindingCaptureDialog.qml` (new) — QML test verifying the double-tap capture mode produces a `Ctrl+C+C`-style sequence string.

### Notes

- Per `CLAUDE.md`: each new QML file must be appended to `qml_files` in `bridges/build.rs`; each new bridge `.rs` must be in the `rust_files` list and have a corresponding `qmllint` stub under `assets/qml/com/profoundlabs/simsapa/` declared in `qmldir`.
- Per `CLAUDE.md`: use `try_exists()` instead of `.exists()` for any filesystem checks.
- Per user feedback in MEMORY.md: do not run `make qml-test` unless explicitly asked; only run tests after **all** sub-tasks of a top-level task are done; use `make build -B` (not raw `cmake`).
- New C++ source files do not need test files; behaviour is exercised end-to-end via manual GUI testing by the user (per `CLAUDE.md`'s "Avoid GUI Testing" guidance for agents).

## Tasks

- [ ] 1.0 Add a separate persistence layer and Rust bridge for global hotkey settings (enabled flag + per-action sequences), independent from the existing in-app keybindings store.
  - [ ] 1.1 Create `backend/src/global_hotkeys.rs` defining a `GlobalHotkeysConfig` struct (`enabled: bool`, `bindings: HashMap<String, String>`) with serde derives.
  - [ ] 1.2 Implement `load(simsapa_dir: &Path) -> GlobalHotkeysConfig` that reads `<simsapa_dir>/global_hotkeys.json`, returning the default config (`enabled: false`, `dictionary_lookup: "Ctrl+C+C"`) when the file is missing. Use `try_exists()` for the file check.
  - [ ] 1.3 Implement `save(&self, simsapa_dir: &Path) -> Result<()>` that writes the config atomically (tempfile + rename).
  - [ ] 1.4 Add helper methods `set_enabled(&mut self, bool)`, `set_binding(&mut self, action_id: &str, sequence: &str)`, `get_binding(&self, action_id: &str) -> Option<&str>`.
  - [ ] 1.5 Wire `GlobalHotkeysConfig` into `AppData` (load on construction, expose accessor methods; this is independent from `keybindings.json`).
  - [ ] 1.6 Create `bridges/src/global_hotkey_manager.rs` with a CXX-Qt bridge `GlobalHotkeyManager` (Rust-side QObject) exposing:
    - `get_global_hotkeys_json() -> QString`
    - `get_default_global_hotkeys_json() -> QString`
    - `set_global_hotkey(action_id: &QString, sequence: &QString)`
    - `set_global_hotkeys_enabled(enabled: bool)`
    - signal `globalHotkeysChanged()` (so QML reloads after a change)
  - [ ] 1.7 Register the new module in `bridges/src/lib.rs` and add `src/global_hotkey_manager.rs` to the `rust_files` list in `bridges/build.rs`.
  - [ ] 1.8 Create the `qmllint` stub `assets/qml/com/profoundlabs/simsapa/GlobalHotkeyManager.qml` with the four function signatures returning placeholder values, and declare it in `qmldir`.
  - [ ] 1.9 Add `#[cfg(test)]` tests in `backend/src/global_hotkeys.rs` for: default config, missing-file fallback, save/load round-trip, and binding mutation.
  - [ ] 1.10 Run `make build -B` and `cd backend && cargo test global_hotkeys` to confirm a clean build.

- [ ] 2.0 Add the "Global Hotkeys" sub-section to `AppSettingsWindow.qml` (placed above the general keybindings list) with the enable checkbox, the `dictionary_lookup` row, and the Wayland-only informational note describing the localhost `/lookup_window_query` workaround.
  - [ ] 2.1 Create `assets/qml/GlobalHotkeysSection.qml` with: a section header "Global Hotkeys", an "Enable global hotkeys" `CheckBox`, and a `ListView` of one row showing the `dictionary_lookup` action label and current sequence with an Edit button.
  - [ ] 2.2 Wire the section to `GlobalHotkeyManager` (load via `get_global_hotkeys_json()`, save via `set_global_hotkey()` / `set_global_hotkeys_enabled()`).
  - [ ] 2.3 The Edit button must open `KeybindingCaptureDialog` with the new double-tap mode enabled (see task 3.0).
  - [ ] 2.4 The enable checkbox must remain editable independently of the row; unchecking only disables OS registration, the row stays editable.
  - [ ] 2.5 In `AppSettingsWindow.qml`, detect Wayland via a property exposed by an existing bridge (or add a small `is_wayland` getter on `SuttaBridge` reading `QGuiApplication::platformName()`).
  - [ ] 2.6 In the "Keybindings" tab, render `GlobalHotkeysSection` above the existing keybindings list when not on Wayland; on Wayland, render only an informational panel describing the localhost `/lookup_window_query` workaround.
  - [ ] 2.7 The Wayland panel must include the actual API port (read from existing API server config), and provide the GET/POST URL forms plus a copy-to-clipboard button for an example `curl` command using `xclip`/`wl-paste`.
  - [ ] 2.8 Append `../assets/qml/GlobalHotkeysSection.qml` to `qml_files` in `bridges/build.rs`.
  - [ ] 2.9 Run `make build -B` to confirm a clean build (do not run `make qml-test` per project preference).

- [ ] 3.0 Extend `KeybindingCaptureDialog.qml` with a double-tap (chord-then-key, e.g. `Ctrl+C+C`) capture mode, toggled by a property on the dialog so existing in-app keybinding capture is unchanged.
  - [ ] 3.1 Add a `bool allow_double_tap: false` property on the dialog.
  - [ ] 3.2 When `allow_double_tap` is true, after capturing the first chord, enter a "waiting for second key" state with a ~500 ms `Timer`; if a second key (without modifiers) arrives in time, append it to the sequence as `Ctrl+C+C`.
  - [ ] 3.3 Update the dialog instruction text to switch between "Press the key combination" (single chord) and "Press the modifier+key, then press the second key" (double-tap), with a visible state indicator while waiting.
  - [ ] 3.4 Validate that the captured sequence parses back as a `QKeySequence`-compatible string (`Modifier+Key+Key`); reject invalid combinations with a clear inline error.
  - [ ] 3.5 Confirm regular in-app keybinding capture (`allow_double_tap` defaulting to false) still produces single-chord output unchanged.
  - [ ] 3.6 Run `make build -B` to confirm a clean build.

- [ ] 4.0 Implement the cross-platform C++ `GlobalHotkeyManager` skeleton (header, Qt signal `hotkeyActivated(int)`, `state2` double-tap state machine in shared code) plus the **Linux X11** backend using the `XRecord` extension in a worker thread.
  - [ ] 4.1 Create `cpp/global_hotkey_manager.h` declaring class `GlobalHotkeyManager : public QObject` with: `bool registerHotkey(const QKeySequence&, int handle)`, `void unregisterAll()`, signal `void hotkeyActivated(int handle)`, and a private `HotkeyEntry` struct (key, key2, modifier, handle, id). Add a one-paragraph header comment crediting Goldendict-ng (GPLv3) as the design reference.
  - [ ] 4.2 Create `cpp/global_hotkey_manager.cpp` containing the cross-platform `state2` double-tap state machine (`waitKey2()` slot + 500 ms `QTimer::singleShot`) and a `parseSequence()` helper that splits `QKeySequence` strings like `Ctrl+C+C` into `(modifier, key, key2)`.
  - [ ] 4.3 Add a private virtual `init()` and `unregister()` interface so platform-specific code lives in the per-platform `.cpp`/`.mm` file under `#ifdef` guards.
  - [ ] 4.4 Create `cpp/global_hotkey_x11.cpp` (compiled only when `WITH_X11` is defined). Open a second `Display` for `XRecord`, allocate an `XRecordRange` covering `KeyPress`/`KeyRelease`, create a `XRecordContext`, and run `XRecordEnableContext` on a `QThread`.
  - [ ] 4.5 In the record callback, translate keycodes + current modifiers to a `(vk, mod)` pair and emit a `keyRecorded(quint32, quint32)` signal queued back to the main thread.
  - [ ] 4.6 In the main-thread slot, run the shared `state2` matcher; when a hotkey matches, emit `hotkeyActivated(handle)`.
  - [ ] 4.7 At runtime, if `QGuiApplication::platformName() != "xcb"`, `init()` must short-circuit and return success-as-no-op so Wayland users don't see errors.
  - [ ] 4.8 Add `cpp/global_hotkey_manager.cpp` and (conditionally) `cpp/global_hotkey_x11.cpp` to `app_files` in `CMakeLists.txt`; link `X11` and `Xtst` on Linux.
  - [ ] 4.9 Wire the `hotkeyActivated` signal into `bridges/src/global_hotkey_manager.rs` (the bridge holds a `cxx_qt::QObject` reference to the C++ manager and forwards activations).
  - [ ] 4.10 Run `make build -B` to confirm a clean build on Linux.

- [ ] 5.0 Implement the **Windows** backend of `GlobalHotkeyManager` (`RegisterHotKey` + `SendInput` re-emit + transient unregister/register around the synthesized event, mirroring Goldendict-ng's proven approach).
  - [ ] 5.1 Create `cpp/global_hotkey_win.cpp` (compiled only on `Q_OS_WIN`). Implement `init()` to obtain an `HWND` (use a hidden helper window or the main window's `winId()`).
  - [ ] 5.2 Implement `registerHotkey()` to call `RegisterHotKey(hwnd, id, modifier, vk)` for the first chord; persist the second key separately in the `HotkeyEntry` for the `state2` matcher.
  - [ ] 5.3 Install a Windows native event filter (`QAbstractNativeEventFilter::nativeEventFilter`) to intercept `WM_HOTKEY` messages and feed them into the shared `state2` matcher.
  - [ ] 5.4 In the matcher, when a hotkey first chord fires and the user's intent is to copy (i.e. it's `Ctrl+C` or any sequence whose first chord is a copy combo), re-emit the keystroke via `SendInput` so the foreground application performs its copy — temporarily `UnregisterHotKey`/`RegisterHotKey` around the `SendInput` call to avoid retriggering ourselves.
  - [ ] 5.5 Track the modifier-key state to avoid emitting modifier presses that are already physically held (port the `GetAsyncKeyState` checks from Goldendict-ng's `winhotkeywrapper.cc`).
  - [ ] 5.6 Implement `unregister()` to `UnregisterHotKey` everything and reset the id counter.
  - [ ] 5.7 Add `cpp/global_hotkey_win.cpp` to `app_files` in `CMakeLists.txt` with appropriate `if(WIN32)` guards.
  - [ ] 5.8 Run `make build -B` (Linux build remains green; the Windows path is compile-checked locally via the `#ifdef` guards and will be exercised on a Windows build by the user).

- [ ] 6.0 Implement the **macOS** backend of `GlobalHotkeyManager` (Carbon `RegisterEventHotKey`, AX selected-text via `AXUIElementCopyAttributeValue`/`kAXSelectedTextAttribute` with synthesized `⌘C` fallback wrapped in `suspendHotkeys()`/`resumeHotkeys()`, and the first-use Accessibility permission dialog that opens System Preferences).
  - [ ] 6.1 Create `cpp/global_hotkey_mac.mm` (compiled only on `Q_OS_MACOS`). Install an `EventHotKey` handler via `InstallApplicationEventHandler` listening for `kEventClassKeyboard` / `kEventHotKeyPressed`.
  - [ ] 6.2 Build a Qt-key → native macOS keycode map at startup using `TISCopyCurrentKeyboardLayoutInputSource` + `UCKeyTranslate` (port the `MacKeyMapping` helper from Goldendict-ng with a fresh implementation).
  - [ ] 6.3 Implement `registerHotkey()` to call `RegisterEventHotKey()` for the first chord (and the second chord with `id+1` to support double-tap matching).
  - [ ] 6.4 In the event handler, route the activated id into the shared `state2` matcher.
  - [ ] 6.5 Implement `getSelectedTextViaAXAPI()` using `AXUIElementCreateSystemWide()` + `AXUIElementCopyAttributeValue(focused, kAXSelectedTextAttribute)`. Return an empty `QString` if anything fails.
  - [ ] 6.6 Implement `sendCmdC()` using `CGEventCreateKeyboardEvent` + `CGEventPost(kCGAnnotatedSessionEventTap, …)` for both keyDown and keyUp, with `kCGEventFlagMaskCommand` set.
  - [ ] 6.7 Implement `suspendHotkeys()` / `resumeHotkeys()` that `UnregisterEventHotKey` and re-register every entry, used to bracket the synthesized `⌘C`.
  - [ ] 6.8 On hotkey activation: call `getSelectedTextViaAXAPI()` first; if empty, `suspendHotkeys()` → `sendCmdC()` → `resumeHotkeys()`, then read `QGuiApplication::clipboard()->text()` after a short delay.
  - [ ] 6.9 Implement `checkAndRequestAccessibilityPermission()`: on first activation, if `AXIsProcessTrusted()` returns false, show a Qt message box explaining the requirement with a button that opens `x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility` via `[NSWorkspace openURL:]`.
  - [ ] 6.10 Add `cpp/global_hotkey_mac.mm` to `app_files` in `CMakeLists.txt` with `if(APPLE)` guards; link `Carbon`, `AppKit`, `ApplicationServices` frameworks.
  - [ ] 6.11 Run `make build -B` to confirm the cross-platform skeleton still compiles on Linux (the macOS path is `#ifdef`-guarded).

- [ ] 7.0 Wire the activation pipeline end-to-end: on `hotkeyActivated`, read the clipboard, sanitize/cap the query at 200 chars, raise the **main** `SuttaSearchWindow`, ensure sidebar visible / Results tab active / search area = Dictionary, and run the dictionary query (reusing existing `run_lookup_query()` and `callback_run_lookup_query`).
  - [ ] 7.1 In `bridges/src/global_hotkey_manager.rs`, connect to the C++ `hotkeyActivated(int)` signal. On activation: read the clipboard via the existing `clipboard_manager` (or `cpp/clipboard_manager`) and obtain the selected text. On macOS the AX path may have already populated the clipboard; on X11 it is the user's own `Ctrl+C` second press; on Windows it is the synthesized `Ctrl+C`.
  - [ ] 7.2 Add a small delay (~10 ms on Linux) before reading the clipboard to give the foreground app time to populate it, mirroring Goldendict-ng's note in `mainwindow.cc`.
  - [ ] 7.3 Sanitize the query: trim whitespace, collapse internal newlines to spaces, and cap the length at 200 characters.
  - [ ] 7.4 If the sanitized query is empty, abort silently — do not raise the window, do not show an error.
  - [ ] 7.5 Otherwise, call `callback_run_lookup_query(query)` which already exists in `cpp/gui.cpp` and triggers `SuttaSearchWindow.run_lookup_query()` (which already shows the sidebar and sets the search area to "Dictionary").
  - [ ] 7.6 Add a "raise main window" step before invoking `callback_run_lookup_query`: identify the first-opened `SuttaSearchWindow` via the existing `window_manager`, then call `raise()` + `activateWindow()` and any platform-specific helper required for cross-workspace activation (Win: `SetForegroundWindow` workaround; macOS: `[NSApp activateIgnoringOtherApps:YES]`; X11: `_NET_ACTIVE_WINDOW` request).
  - [ ] 7.7 Verify in `SuttaSearchWindow.qml` that `run_lookup_query()` already activates the **Results** tab in the sidebar; if not, extend it to do so explicitly.
  - [ ] 7.8 If no `SuttaSearchWindow` is currently open, open the main window first, then invoke the lookup once it is ready.
  - [ ] 7.9 Confirm the existing `bridges/src/api.rs` `/lookup_window_query` GET and POST endpoints still hit the same final pipeline (they should — this is the Wayland workaround path).
  - [ ] 7.10 Run `make build -B` to confirm a clean build.

- [ ] 8.0 Lifecycle, failure handling, and platform gating: register the manager at app startup based on settings, re-register on settings change, unregister cleanly on shutdown, surface a one-time error dialog on registration failure, and hide the global-hotkey controls entirely when running on Wayland.
  - [ ] 8.1 In `cpp/main.cpp` (or wherever `SuttaBridge` is instantiated), construct the `GlobalHotkeyManager` once, parented to the application, so it lives for the app lifetime.
  - [ ] 8.2 At startup, if `GlobalHotkeysConfig.enabled` is true, register the configured `dictionary_lookup` sequence with handle `0`. Otherwise leave the manager idle.
  - [ ] 8.3 On `globalHotkeysChanged` (emitted by the bridge after `set_global_hotkey` or `set_global_hotkeys_enabled`), call `unregisterAll()` then re-register from current config — no app restart required.
  - [ ] 8.4 On application shutdown (`QCoreApplication::aboutToQuit`), call `unregisterAll()` to release OS-level grabs cleanly.
  - [ ] 8.5 If `registerHotkey()` returns false at any point (X11 RECORD missing, Windows hotkey conflict, macOS Carbon failure), surface a one-time `QMessageBox::critical` from QML with a platform-specific message; record that the dialog has been shown for this session so it does not spam.
  - [ ] 8.6 Reset the "already shown" flag whenever the user changes the configured sequence, so a new conflict produces a fresh dialog.
  - [ ] 8.7 In `assets/qml/AppSettingsWindow.qml`, hide `GlobalHotkeysSection` entirely when `is_wayland` is true and show only the localhost-API note panel from task 2.7.
  - [ ] 8.8 Add a small note in `PROJECT_MAP.md` describing where global-hotkey code lives (per `CLAUDE.md`'s instruction to keep `PROJECT_MAP.md` current).
  - [ ] 8.9 Add a section in `docs/` documenting the feature for end users (settings location, default hotkey, Wayland workaround, macOS Accessibility prompt).
  - [ ] 8.10 Run `make build -B` and `cd backend && cargo test` to confirm everything passes.
