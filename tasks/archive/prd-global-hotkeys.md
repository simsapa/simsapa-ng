# PRD: Global Hotkeys for Dictionary Lookup

## 1. Introduction / Overview

Simsapa is currently only reachable via its own UI: a user reading text in another
application (a browser, a PDF viewer, an editor) cannot trigger a Pāli dictionary
lookup without manually switching to Simsapa, focusing the search bar, pasting the
word, and selecting the dictionary search area.

This feature adds **OS-level global hotkeys** so that a user anywhere on the desktop
can press a configured key sequence (default: `Ctrl+C+C`, matching the well-known
Goldendict convention) and have Simsapa raise itself to the foreground and run a
dictionary lookup against the currently selected text. The selection is acquired by
synthesizing a `Ctrl+C` (or `⌘C` on macOS) and reading the system clipboard, so the
feature works regardless of whether the source application supports the X11 PRIMARY
selection.

Goal: make Simsapa's dictionary usable as an in-place lookup tool from any other
desktop application on Linux (X11), Windows, and macOS.

## 2. Goals

1. A user can configure a global hotkey for "dictionary lookup" in the
   `AppSettingsWindow` and toggle it on/off with a checkbox.
2. Pressing that hotkey from **any** focused application (Simsapa not focused) on
   Linux X11, Windows, and macOS:
   - raises the Simsapa main window to the foreground,
   - ensures the sidebar is shown,
   - activates the **Results** tab,
   - sets the search area to **Dictionary**,
   - executes a dictionary query using the user's current text selection.
3. The hotkey supports both single chord sequences (e.g. `Ctrl+Shift+D`) and
   double-tap chord sequences (e.g. `Ctrl+C+C`), the latter being the default.
4. Failures to register the hotkey at the OS level surface as a one-time error
   dialog and do not crash the application.
5. The feature is implemented as Simsapa's own C++ class (with attribution, but
   not a copy of Goldendict source files) wired into the existing CXX-Qt bridge
   architecture.

## 3. User Stories

- As a Pāli student reading a PDF, I want to highlight a word and press
  `Ctrl+C+C` so that Simsapa pops to the foreground with the dictionary entry
  for that word already shown, without me having to alt-tab and paste.
- As an advanced user, I want to remap the global hotkey to something other
  than `Ctrl+C+C` (e.g. `Ctrl+Alt+L`) because `Ctrl+C+C` interferes with my
  workflow in some apps.
- As a user on a system where global key grabbing fails (e.g. unusual desktop
  environment, missing X11 RECORD extension), I want a clear error message
  instead of a silent failure or crash.
- As a Wayland or Android user, I want the rest of Simsapa to keep working
  even though global hotkeys are not available on my platform.

## 4. Functional Requirements

### 4.1 Settings UI (`AppSettingsWindow.qml`, "Keybindings" tab)

1. The "Keybindings" tab must display a new sub-section titled **"Global
   Hotkeys"** rendered **above** the existing general (in-app) keybindings list.
2. The Global Hotkeys sub-section must contain:
   1. An "Enable global hotkeys" checkbox.
   2. A list of global hotkey rows. For this version, exactly one row exists:
      `dictionary_lookup` (display label: "Dictionary lookup").
3. Each row must show the action label and the currently bound key sequence,
   and an Edit button that opens `KeybindingCaptureDialog.qml`.
4. The Enable checkbox controls **only the OS-level registration**. When
   unchecked, the hotkey is unregistered with the OS, but the row remains
   editable so the user can change the binding before re-enabling.
5. Toggling the checkbox or saving a new sequence must immediately call the
   Rust bridge to (un)register the hotkey; no app restart should be required.
6. The default value for `dictionary_lookup` is `Ctrl+C+C`. The default for the
   enable checkbox is **off** (the user opts in).

### 4.2 Storage (Rust bridge)

7. Global hotkeys must be persisted in a **separate** store from the existing
   in-app keybindings — they are not interchangeable and have an extra
   `enabled` flag.
8. The `SuttaBridge` must expose at minimum:
   - `get_global_hotkeys_json() -> String` — returns JSON
     `{ "enabled": bool, "bindings": { "dictionary_lookup": "Ctrl+C+C" } }`.
   - `set_global_hotkey(action_id: String, sequence: String)` — persists and
     re-registers.
   - `set_global_hotkeys_enabled(enabled: bool)` — persists and (un)registers.
   - `get_default_global_hotkeys_json() -> String`.
9. Corresponding stub functions must be added to
   `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml` per the project's
   `qmllint` convention documented in `CLAUDE.md`.

### 4.3 Capture dialog (`KeybindingCaptureDialog.qml`)

10. The capture dialog must be extended so that, when invoked for a **global
    hotkey** action, it can capture **double-tap chord** sequences of the form
    `Modifier+Key+Key` (e.g. `Ctrl+C+C`) in addition to ordinary single chords.
11. The dialog must accept the second key only if it arrives within a short
    timeout (~500 ms) after the first chord, matching Goldendict's behaviour.
12. The dialog must visually indicate when it is "waiting for the second key"
    so the user understands the double-tap UX.
13. When invoked for a regular in-app keybinding, the dialog must continue to
    behave as today (single chord only).

### 4.4 C++ hotkey implementation

14. A new C++ class `GlobalHotkeyManager` must be added under the existing C++
    source tree. It must **not** be a copy of Goldendict source files; it must
    be Simsapa's own implementation, with comments documenting the approach
    and a header note crediting Goldendict-ng (GPLv3) as the reference.
15. `GlobalHotkeyManager` must expose a stable cross-platform API:
    - `bool registerHotkey(const QKeySequence &seq, int handle)`
    - `void unregisterAll()`
    - signal `hotkeyActivated(int handle)`
16. The platform implementations live behind `#ifdef` blocks in separate
    translation units:
    - **Linux X11**: use the `XRecord` extension to observe key presses
      without interfering with focus, mirroring Goldendict's approach. The
      thread that runs `XRecord` must marshal key events back to the main
      thread via a queued signal.
    - **Windows**: use `RegisterHotKey` for the first chord. Because
      `RegisterHotKey` *consumes* the keystroke, the manager must re-emit it
      via `SendInput` so the foreground application still performs its own
      copy when the user types `Ctrl+C` as part of `Ctrl+C+C`. While
      re-emitting, the manager must briefly `UnregisterHotKey` and
      re-register to avoid re-triggering itself. The 500 ms `state2` timer
      then waits for the second `Ctrl+C` (also caught by `RegisterHotKey`
      and re-emitted in the same way). This mirrors Goldendict-ng's proven
      Windows approach (`winhotkeywrapper.cc`).
    - **macOS**: use Carbon `RegisterEventHotKey` (still supported and the
      community-recommended path despite Carbon being mostly deprecated —
      see Goldendict-ng's `machotkeywrapper.mm` and the `KeyboardShortcuts`
      Swift library README cited there). For double-tap chords, register the
      first chord and apply the same two-stage `state2` timer pattern.
    - **macOS — selected-text acquisition**: when the first `⌘C` fires,
      attempt to read the selected text directly via the Accessibility API
      (`AXUIElementCopyAttributeValue` with `kAXSelectedTextAttribute` on
      the system-wide focused UI element) before falling back to
      synthesizing `⌘C`. The AX path is preferred because it is instant,
      does not pollute the clipboard, and works in apps that don't honour
      synthesized key events. The fallback `⌘C` path remains required for
      apps that don't expose AX text (e.g. some Electron apps and games).
    - **macOS — Accessibility permission**: posting a synthesized `⌘C` and
      using the AX selected-text API both require the **Accessibility**
      permission on modern macOS. On first activation of the hotkey the
      manager must check `AXIsProcessTrusted()`; if false, present a dialog
      explaining the requirement and offering a button that opens
      `x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility`
      directly (matching Goldendict-ng's UX).
17. On Linux, if `QGuiApplication::platformName() != "xcb"` (i.e. Wayland), the
    manager must report success-with-no-op and emit nothing — the rest of
    Simsapa must continue to function. See §4.8 for the Wayland workaround
    that the UI must surface to the user.
18. If initialization throws (e.g. missing X11 RECORD extension, OS API
    failure), the manager must propagate the failure as a return value to the
    Rust bridge. The bridge must then surface a one-time error dialog from
    QML.

### 4.5 Selection / clipboard acquisition

19. When the hotkey fires, Simsapa must obtain the lookup text by **simulating
    a copy keystroke in the foreground application and then reading the
    system clipboard**, mirroring Goldendict's `Ctrl+C+C` strategy:
    - **macOS**: first try the Accessibility API
      (`AXUIElementCopyAttributeValue` / `kAXSelectedTextAttribute`) to read
      the selected text directly. If that returns empty, fall back to
      posting `⌘C` via `CGEventCreateKeyboardEvent` and reading
      `QApplication::clipboard()->text(QClipboard::Clipboard)`. While
      synthesizing `⌘C`, the manager must `suspendHotkeys()` /
      `resumeHotkeys()` around the `CGEventPost` call to avoid recursive
      activation.
    - **Windows**: the user's own second `Ctrl+C` press in `Ctrl+C+C` is
      re-emitted via `SendInput` (see §4.4) so the foreground app performs
      the copy; Simsapa then reads the clipboard. For single-chord global
      hotkeys (e.g. a custom `Ctrl+Alt+L`), `SendInput` synthesizes a
      `Ctrl+C` press explicitly before reading.
    - **Linux X11**: this happens implicitly — the second `C` press in
      `Ctrl+C+C` is the foreground app's own copy command; Simsapa simply
      reads the clipboard after a small delay (~10 ms) to give the foreground
      app time to populate it.
20. If the clipboard text is empty or contains only whitespace after
    sanitisation, the lookup must be silently cancelled (no error dialog, no
    raised window).
21. Sanitisation must trim whitespace and cap the query length at a sensible
    bound (e.g. 200 characters) to avoid pasting an entire paragraph into the
    search field.

### 4.8 Wayland fallback (localhost API)

W1. On Wayland, where OS-level global hotkey registration is not feasible from
    a sandboxed application, the "Global Hotkeys" sub-section must display an
    informational note in place of (or alongside) the disabled controls,
    explaining that the user can instead bind a desktop-environment shortcut
    (e.g. via GNOME Settings → Keyboard → Custom Shortcuts, or KDE's
    equivalent) that calls Simsapa's localhost API to trigger a lookup.
W2. The note must include the two concrete invocations:
    - `GET  http://127.0.0.1:<port>/lookup_window_query?q=<text>`
    - `POST http://127.0.0.1:<port>/lookup_window_query` with body `{"q": "<text>"}`
    and a copy-to-clipboard button for an example `curl` command, e.g.:
    `curl -G --data-urlencode "q=$(xclip -selection clipboard -o)" http://127.0.0.1:<port>/lookup_window_query`
    (or the Wayland-appropriate equivalent, e.g. `wl-paste`).
W3. The existing API server (`bridges/src/api.rs`) must expose a
    `/lookup_window_query` endpoint accepting both `GET` (query string `q`)
    and `POST` (JSON `{"q": "..."}`) that performs the same activation
    sequence as §4.6 (raise main window, sidebar on, Results tab, Dictionary
    area, run query). The endpoint must be enabled regardless of platform so
    that the same workaround works for advanced users on X11/Win/macOS too.
W4. The note must show the actual port the API is listening on at runtime,
    not a hard-coded value, so users can copy a working command directly.

### 4.6 Window-raising and lookup behaviour

22. On a successful lookup activation, Simsapa must:
    1. Raise and focus the **main window** (the first opened
       `SuttaSearchWindow`) — not the most recently focused window, and not a
       new window.
    2. If the main window is on a different workspace/desktop, bring it to
       the current one (using Qt's `raise()` / `activateWindow()` and any
       platform-specific helpers required for reliable focus stealing).
    3. Show the sidebar if currently hidden.
    4. Activate the Results tab in the sidebar.
    5. Set the search area selector to "Dictionary".
    6. Populate the search input with the sanitised query and execute the
       search as if the user had pressed Enter.
23. If no Simsapa window is open at all, the activation must open the main
    window first and then perform the steps in §4.6.22.

### 4.7 Lifecycle

24. The `GlobalHotkeyManager` must be created at app startup if global
    hotkeys are enabled in settings, and destroyed cleanly on app shutdown
    (unregistering all OS-level grabs).
25. Changes from settings must reach the manager without a restart: setting a
    new sequence unregisters the old one and registers the new one.

## 5. Non-Goals (Out of Scope)

- A floating "scan popup" window like Goldendict's `ScanPopup` — the lookup
  always happens in the existing main window.
- Mouse-hover scan / "scan flag" features.
- Wayland support (best-effort no-op only; not a target).
- Android / iOS / mobile global hotkeys.
- Multiple distinct global hotkeys (the architecture supports it, but only
  `dictionary_lookup` is exposed in this version).
- A separate "lookup history" or recent-lookups panel triggered by the hotkey.
- Auto-detecting the source-text language and switching between dictionary and
  sutta search modes (always dictionary in this version).

## 6. Design Considerations

- The new "Global Hotkeys" sub-section in `AppSettingsWindow.qml` should
  visually match the existing keybindings list (same row layout, same Edit
  button style) so the two sections feel like one cohesive settings page.
- `KeybindingCaptureDialog.qml` should switch between "single chord" and
  "double tap" modes based on a `is_global` (or similarly named) property
  passed in by the caller, and show clear instructional text in both modes
  ("Press the key combination" vs. "Press the modifier+key, then press the
  second key within a moment").
- The error dialog for failed hotkey registration should be a standard Qt
  message box invoked from QML, with a translatable message that mentions the
  likely causes per platform (e.g. on Linux: "Make sure your X server has the
  RECORD extension enabled.").

## 7. Technical Considerations

- **Architecture split** (confirmed):
  - C++ owns `GlobalHotkeyManager` and the OS-level key grabbing.
  - The Rust `SuttaBridge` exposes the JSON-based settings API and a Qt
    signal `globalDictionaryLookupRequested(QString query)` (or equivalent)
    that QML connects to.
  - QML reacts to that signal by performing the window-raising and search
    sequence (§4.6).
- The existing `clipboard_manager.rs` may be extended or referenced for the
  clipboard read, but the synthesized copy keystroke must live in C++ since
  it is OS-API-dependent.
- New CXX-Qt bridge files must be registered per the procedure documented in
  `CLAUDE.md` (`bridges/build.rs` `rust_files` list, `qmldir`, QML stub for
  `qmllint`).
- New QML files (e.g. an extracted "Global Hotkeys" section component, if
  factored out) must be appended to `qml_files` in `bridges/build.rs`.
- The Linux X11 implementation uses `QThread` + `XRecord` and must use a
  queued connection back to the main thread to avoid races.
- File-existence checks on the Linux side must use `try_exists()` per the
  project's Android-safety rule (relevant if any settings file probing is
  added).
- Goldendict's `hotkeywrapper` headers and the `ScanPopup::translateWord*`
  flow are the reference implementations and should be cited in source
  comments. No source files are to be copied verbatim.

## 8. Success Metrics

- A user can, on Linux X11, Windows, and macOS, configure `Ctrl+C+C` (or any
  custom sequence), select a Pāli word in any other application, press the
  hotkey, and see the correct dictionary entry in Simsapa's main window
  within ~500 ms of the keypress.
- Disabling the checkbox releases the OS-level hotkey within the same
  session (verifiable by re-pressing the sequence in another app and seeing
  no Simsapa response).
- No crashes or hangs are introduced when global hotkeys are toggled, when
  the bound sequence is changed at runtime, or when the OS denies
  registration.
- Wayland users and Android users can install and use Simsapa with no
  regressions: the rest of the app behaves as before, and the Global Hotkeys
  section either reports "not supported on this platform" or remains
  toggleable but non-functional with a clear message.

## 9. Resolved Design Decisions

The questions previously listed here have been resolved by referring to
Goldendict-ng's production implementation (which has many Windows and macOS
users) and to project conventions:

1. **Windows hotkey strategy**: use `RegisterHotKey` + `SendInput` re-emit +
   500 ms `state2` window for the second key, exactly as Goldendict-ng does
   in `winhotkeywrapper.cc`. No low-level keyboard hook (`WH_KEYBOARD_LL`)
   to avoid antivirus false-positives.
2. **macOS strategy**: Carbon `RegisterEventHotKey` for grabbing, AX API
   first for selected-text acquisition, synthesized `⌘C` as fallback. On
   first activation, prompt for Accessibility permission with a one-click
   button that opens the relevant System Preferences pane (Goldendict-ng's
   `machotkeywrapper.mm` pattern).
3. **i18n**: Simsapa does not currently store UI translation strings, so
   global-hotkey UI text and error messages are written as plain strings
   without translation wrappers, consistent with the rest of the codebase.
4. **Wayland UI**: hide the global-hotkey controls entirely on Wayland and
   show only the informational note describing the localhost-API workaround
   (§4.8). Settings persist regardless of platform so they reactivate if
   the user later runs Simsapa under X11.
5. **Query length cap**: 200 characters, hard-coded. Not exposed as a
   user-tunable setting in this version.
