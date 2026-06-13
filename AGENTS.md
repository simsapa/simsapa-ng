# Agent Guidelines for Simsapa App

Simsapa is a sutta reader app for reading the Theravāda Tipitaka in Pāli and translated languages, providing Pāli language tools to analyse passages.

## Architecture

This is a Qt app with QML window layouts, connecting to a Rust back-end using bridge elements with the CXX-Qt library.

- Multi-platform Qt6 app
- C++ and Rust using the CXX-Qt library with QML for window layouts and UI widgets
- Rust backend uses SQLite with Diesel ORM
- Rust CXX-Qt bridges define backend functions used in QML elements

## Project Structure

For detailed information about the codebase organization, folder structure, and locations of essential functions, read [PROJECT_MAP.md](./PROJECT_MAP.md).

Keep [PROJECT_MAP.md](./PROJECT_MAP.md) updated as changes are made.

When working on features, the PRD (Product Requirements Document) files are in
the `tasks/` folder. They often contain the reasoning and logic for existing
features.

Documentation is in the `docs/` folder. Keep it updated for relevant features.

Notable feature docs:
- [Language filter query logic](./docs/language-filter-query-logic.md) — how the
  search bar's per-area language filter is persisted, how the dropdown options
  are loaded from distinct DB values, and where the filter is applied in the
  Suttas / Dictionary / Library query paths (including the "no filter" gate and
  the DPD `language = "pli"` gotcha).
- [Startup sequence and caches](./docs/startup-sequence-and-caches.md) — what
  runs synchronously vs. on a background thread vs. deferred to first QML use,
  the five `AppSettings` caches and their refresh hooks, why background warming
  lives in `init_app_data()` (not `AppData::new()`), the
  `Loader` vs. `Component + createObject` rule for QML wrapping based on root
  element type (`Dialog`/`Popup` vs `ApplicationWindow`), and the eager-binding
  pre-flight required before deferring components.
- [User data imports and SQLite `ANALYZE`](./docs/user-data-and-sqlite-analyze.md) —
  every code path that grows a shipped DB at runtime (StarDict zip/dir,
  EPUB/PDF/HTML books, sutta language downloads) and where the matching
  post-write `ANALYZE` lives. Shipped DBs are `ANALYZE`d at bootstrap;
  `DatabaseHandle::analyze` is the runtime hook. **If you add a new import
  path, add an `ANALYZE` call and update the table in that doc.** Background:
  missing `sqlite_stat1` made the Headword Match query 170 s instead of 17 ms
  (see `tasks/prd-fixing-headword-match-slow-query.md`).
- [Windows portable install](./docs/windows-portable-install.md) — the Standard
  vs Portable installer modes, the relocatable folder layout, how the portable
  `config.txt` sets a relative `SIMSAPA_DIR` resolved against the **exe
  directory** (`exe_dir()` / `resolve_simsapa_dir()` / `normalize_lexically()`
  in `backend/src/lib.rs`, not `canonicalize()`), the `.lnk` vs `.cmd` launcher
  choice, and USB drive-letter robustness.
- [Android / ChromeOS soft keyboard](./docs/android-soft-keyboard.md) — why a
  Qt `TextField`/`TextArea` needs two taps (or never raises) the on-screen
  keyboard on Android/ChromeOS, the reusable `MobileKeyboardHelper.qml`
  (focus-in + tap + retry `Timer` until `Qt.inputMethod.visible`), the
  `EnterKey.type` rules (`EnterKeySearch` for search fields needs a matching
  `onAccepted`; `EnterKeyDone` for form fields; omit for multi-line), and the
  `focus: root.is_desktop` gate for pre-focused persistent fields. **Apply this
  technique to every new text input.**
- [Mobile rendering troubleshooting](./docs/mobile-rendering-troubleshooting.md) —
  the three mobile-only **Settings → Rendering** tab toggles that work around
  GPU framebuffer / scene-graph corruption on flaky Android drivers (flat result
  backgrounds, disable list clip, `QSG_RENDER_LOOP=basic`; `QSG_RHI_BACKEND=vulkan`
  and `QT_QUICK_BACKEND=software` toggles were removed — the first crashed the
  app, the second produced an unusable UI). Explains why the `render_loop_basic`
  env-var toggle is read from the DB in `gui.cpp` before `QApplication`
  (standalone `db::get_app_settings()` + `render_loop_basic_c()` FFI, cached,
  restart-only) vs. the two QML toggles passed down to `FulltextResults.qml`.

## Specific coding procedures

### Android compatibility: File existence checks

**IMPORTANT:** Always use `try_exists()` instead of `.exists()` when checking if files or directories exist. The `.exists()` method can cause permission crashes on Android.

Example:
```rust
// ❌ BAD - can crash on Android
if log_file.exists() {
    // ...
}

// ✅ GOOD - safe on all platforms including Android
match log_file.try_exists() {
    Ok(true) => {
        // File exists
    }
    Ok(false) => {
        // File doesn't exist
    }
    Err(_) => {
        // Permission error or other issue
    }
}
```

See `backend/src/logger.rs` for examples of this pattern in practice.

### Android "isn't 16 KB compatible" warning (test-deploy only)

If the Android app shows the dialog **"This app isn't 16 KB compatible"** listing
libraries with "Unknown error", **do not assume the build is broken.** This
warning is gated on the **install path**, not the APK contents. The dialog text
says it outright: *"because this is a debuggable app which is currently being
tested."*

- Installing via **Qt Creator deploy** (`adb install` / `pm install` marks it a
  test deployment) → the dialog appears.
- **Sideloading the byte-identical APK** (copy to phone, tap to install in a
  file manager) → no dialog.

Confirmed empirically: APK sideload → no warning; Qt Creator deploy of the same
build → warning; sideload again → no warning. End users (and any normal
sideload/Play install) never see it.

"Unknown error" next to a library does **not** mean it is misaligned — it means
the on-device checker couldn't verify a library that is stored **compressed**
(`Defl:N`, `extractNativeLibs=true`) in the APK. To prove a given build is fine,
verify the libs directly instead of trusting the dialog:

```sh
# ELF LOAD-segment alignment of a flagged lib (want 0x4000 = 16 KB)
unzip -p app.apk lib/arm64-v8a/libQt6Widgets_arm64-v8a.so > /tmp/x.so
readelf -lW /tmp/x.so | grep LOAD          # last column is p_align
# APK-level 16 KB page alignment
$ANDROID_SDK/build-tools/<ver>/zipalign -c -P 16 4 app.apk && echo PASS
```

(June 2026 forensics: a warning build and a known-good no-warning build were
identical on every 16 KB axis — lib md5s, ELF p_align (Qt libs 0x4000), all 148
libs `Defl:N`, `zipalign -P16` PASS, compileSdk 36 / targetSdk 35 / debuggable.
The only variable was the install path.)

**Separate, real gap (not this dialog):** the app is not *truly* 16 KB-compatible
because Qt's 5 bundled FFmpeg prebuilts (`libavcodec`, `libavformat`,
`libavutil`, `libswresample`, `libswscale`) are 4 KB-aligned (0x1000). Harmless
for dev/sideload, but it will fail a Play Store submission targeting API 35+. Fix
path: swap Qt's FFmpeg multimedia backend for the Android MediaCodec backend, or
ship 16 KB-aligned FFmpeg builds. The FFmpeg backend is pulled in for recording
(`RecordingPlaybackItem`, chanting practice).

### New QML components

When you create a new QML component such as `SearchBarInput.qml`, the file has to be added to the `qml_files` list in `bridges/build.rs`.

``` rust
qml_files.push("../assets/qml/SearchBarInput.qml");
```

### Logging in QML (no console API)

In the QML files under `assets/qml/`, do **not** use the `console` API
(`console.log()`, `console.error()`, etc.). Use the `Logger { id: logger }`
module's functions for logging instead.

The one exception is the folder
`assets/qml/com/profoundlabs/simsapa/`: the `console` API is allowed there
because those files are type stubs for `qmllint`.

#### Using the Logger module

`Logger.qml` lives in `assets/qml/`, so it is automatically available to any
other component in that directory — no `import` statement is needed. Declare an
instance once in the component's root element, conventionally with `id: logger`:

``` qml
Item {
    id: root

    Logger { id: logger }

    // ...
}
```

The available functions map to log levels: `logger.debug(message)`,
`logger.info(message)`, `logger.warn(message)`, `logger.error(message)`.

**IMPORTANT — each function takes a single `message` argument, unlike the
variadic `console` API.** Do not pass multiple comma-separated arguments;
build one string with concatenation instead:

``` qml
// ❌ BAD - console-style multiple arguments; extra args are dropped/ignored
logger.error("Failed to parse data_json:", e, "data_json:", root.data_json);

// ✅ GOOD - a single concatenated string
logger.error("Failed to parse data_json: " + e + " data_json: " + root.data_json);
```

When migrating from `console`, map the methods by severity rather than
mechanically: `console.error` → `logger.error`, `console.warn` → `logger.warn`,
and `console.log` → `logger.info` (or `logger.error` when the message actually
reports a failure).

### New functions on Rust bridge QML components

When adding new functions to Rust bridge QML components such as SuttaBridge, add a corresponding function in the `qmllint` type definition, e.g. SuttaBridge.qml

For example, when implementing the `get_api_key()` method in `sutta_bridge.rs`, add a corresponding function in `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml` with the correct function signature and a simple return value. The internal logic doesn't have to be repeated, because this is only for the benefit of `qmllint`.

``` qml
function get_api_key(key_name: string): string {
    return 'key_value';
}
```

### New Rust bridges

When you create a new Rust bridge such as `bridges/src/prompt_manager.rs`, it has to be registered as a QmlModule and the Rust file name has to be added to the `rust_files` list in `bridges/build.rs`:

``` rust
.qml_module(QmlModule {
        uri: "com.profoundlabs.simsapa",
        rust_files: &[
                "src/sutta_bridge.rs",
                "src/asset_manager.rs",
                "src/storage_manager.rs",
                "src/prompt_manager.rs",
                "src/api.rs",
        ],
        qml_files: &qml_files,
        ..Default::default()
})
```

`qmllint` requires that the corresponding QML type definition for the Rust bridge has to be created and it should be declared in the `qmldir` file.

```
assets/qml/com/profoundlabs/simsapa/PromptManager.qml
assets/qml/com/profoundlabs/simsapa/qmldir
```

### FTS5 fulltext search tables (scripts in `scripts/`)

The fulltext search tables are FTS5 virtual tables created by the SQL scripts in
`scripts/` (`appdata-fts5-indexes.sql`, `books-fts5-indexes.sql`,
`dictionaries-fts5-indexes.sql`, `dpd-fts5-indexes.sql`,
`dpd-bold-definitions-fts5-indexes.sql`). There is **no Diesel migration** for
these — the scripts drop and recreate the FTS table + sync triggers, so any
schema change requires a **manual re-bootstrap** of the affected DB (run the
script again). The scripts are run from the bootstrap code in `cli/src/`.

**IMPORTANT — store the source row id as the FTS5 `rowid`, never as an
`UNINDEXED` column.** FTS5 has no secondary indexes, so a lookup like
`WHERE dict_word_id = ?` against an `UNINDEXED` id column is a **full table
scan**. The `AFTER DELETE` / `AFTER UPDATE` sync triggers run exactly that
lookup once per affected source row, so a cascade delete of an N-row dictionary
became N full scans of the entire FTS table — deleting a 2000-entry dictionary
took ~3 minutes (measured 168 s) against a 198k-row FTS, and ~8 minutes in-app.

The fix (applied to all FTS scripts) is to carry the source `id` as the FTS5
`rowid`, which makes the trigger lookups O(log n):

```sql
-- ✅ GOOD: id is the FTS5 rowid (no UNINDEXED id column)
CREATE VIRTUAL TABLE dict_words_fts USING fts5(
    language UNINDEXED, dict_label UNINDEXED, word, definition_plain,
    tokenize='trigram', detail='none'
);
INSERT INTO dict_words_fts (rowid, language, dict_label, word, definition_plain)
SELECT id, language, dict_label, word, definition_plain FROM dict_words ...;

CREATE TRIGGER dict_words_fts_delete AFTER DELETE ON dict_words
BEGIN
    DELETE FROM dict_words_fts WHERE rowid = OLD.id;  -- O(log n), not a full scan
END;
```

Consequences for query code (`backend/src/query_task.rs`): join on the rowid
(`JOIN dict_words_fts f ON f.rowid = dict_words.id`) and project it with an
alias when needed (`SELECT rowid AS headword_id`). When adding a new FTS5 table
or query, follow this convention — do not reintroduce an `UNINDEXED` id column.

## Testing with the Database

**SIMSAPA_DIR** (the runtime data directory) is at:
```
/home/gambhiro/prods/apps/simsapa-ng-project/bootstrap-assets-resources/dist/simsapa-ng
```

The SQLite database is at:
```
/home/gambhiro/prods/apps/simsapa-ng-project/bootstrap-assets-resources/dist/simsapa-ng/app-assets/appdata.sqlite3
```

Use this path for any tests or experimental scripts that need to query the actual database or access runtime assets.

## Build/Test Commands

### Development Build
- **Build:** `make build -B` (CMake + Qt6) or `cmake -S . -B ./build/simsapadhammareader/ && cmake --build ./build/simsapadhammareader/`
- **Run:** `make run` or `./build/simsapadhammareader/simsapadhammareader`
- **TypeScript:** `npx webpack` (builds src-ts/ → assets/js/simsapa.min.js)
- **Sass:** `make sass` or `sass --no-source-map './assets/sass/:./assets/css/'`

### Distribution Packages
- **Linux AppImage:** `make appimage -B` (creates Simsapa-*.AppImage)
  - Clean rebuild: `make appimage-rebuild`
  - Clean only: `make appimage-clean`
- **macOS Bundle & DMG:** `make macos -B` (creates .app and .dmg for macOS)
  - App bundle only: `make macos-app` (skips DMG creation)
  - Clean only: `make macos-clean`
  - Clean rebuild: `make macos-rebuild`
- **Android APK:** Build with Qt Creator

### Testing
- **QML Tests:** `make qml-test` (runs all QML tests with offscreen platform)
- **Rust Tests:** `cd backend && cargo test` (runs all backend tests)
- **Single Test:** `cd backend && cargo test test_name` (replace test_name with specific test function)
- **All Tests:** `make test` (runs Rust, QML, and JavaScript tests)

### GUI Testing for Agents

**⚠️ Avoid GUI Testing:** As an AI agent, avoid running the GUI application for testing purposes. The WebEngine components require proper process cleanup that may interfere with your terminal session.

If you must test GUI functionality:
- Use `make build -B` to verify compilation only
- Test individual Rust components with `cd backend && cargo test`
- GUI functionality should be tested manually by the user

The command `export QT_QPA_PLATFORM=offscreen && timeout 10 make run` may leave hanging processes that require manual cleanup, which is not suitable for automated agent testing.

## Code Style

Use lowercase snake_case for new functions, variables and id names, E.g:
- `id: next_message, id: message_item, property bool is_collapsed`
- `function export_dialog_accepted()`

- **Rust:** snake_case, standard rustfmt, use `anyhow::Result` for error handling, prefer `tracing` over `println!`

- **TypeScript:** 2-space indents, import * as alias style, use webpack for bundling

- **C++:** lowercase snake_case functions, PascalCase classes, include proper error handling with custom exceptions

- **QML:** PascalCase components, camelCase properties, follow Qt conventions

- **Naming:** Descriptive names, avoid abbreviations, use domain-specific terms (sutta, pali, dhamma)

- **Errors:** Use Result types in Rust, exceptions in C++, proper error propagation throughout stack

