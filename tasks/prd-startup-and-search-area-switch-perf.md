# PRD — Startup and Search-Area Switch Performance

## 1. Background and motivation

Two user-visible delays in the current build:

1. **Startup delay before the theme is applied.** On mobile (Android) the
   pre-`apply_theme()` phase takes several seconds. During this time the user
   sees the un-themed default palette of the main window.
2. **Switching to the Dictionary search area is always slow.** Tapping the
   `D` button in `SearchBarInput.qml` introduces a noticeable hitch that is
   not present when switching to `S` (Suttas) or `L` (Library).

Both delays trace back to the same family of root causes: synchronous
`SELECT DISTINCT` scans on `dict_words` (≈185 k rows, no index on
`language`), and a small number of other cold-path queries that run on the
GUI thread during window construction.

## 2. Findings (already verified against the local DB)

### 2.1 Dictionary search-area switch (`S → D`)

The QML side of the switch calls (`assets/qml/SearchBarInput.qml:60`):

```qml
function load_language_labels_for_area(area: string) {
    if (area === "Suttas")   { lang_labels = SuttaBridge.get_sutta_language_labels(); }
    else if (area === "Library") { lang_labels = SuttaBridge.get_library_language_labels(); }
    else { /* Dictionary */ lang_labels = SuttaBridge.get_dict_language_labels(); }
}
```

This is invoked synchronously from `language_filter_dropdown.restore_for_current_area()`
each time `search_area` changes. The three bridge calls map to:

| Area       | Backend query                                                                                                              | Row source                                        |
|------------|----------------------------------------------------------------------------------------------------------------------------|---------------------------------------------------|
| Suttas     | `SELECT DISTINCT language FROM suttas`                                                                                     | `suttas` (covering index `idx_suttas_language`)   |
| Dictionary | `SELECT DISTINCT language FROM dict_words WHERE language IS NOT NULL`                                                      | `dict_words` (**no index on `language`**)         |
| Library    | `SELECT (spine_item.language, book.language) FROM book_spine_items JOIN books ...` (Rust-side fallback to `"en"`)          | `book_spine_items`, `books` (small)               |

EXPLAIN QUERY PLAN (local DB):

```
-- dict_words: full table scan + temp B-tree
SCAN dict_words
USE TEMP B-TREE FOR DISTINCT
Run Time: ~107 ms warm on desktop

-- suttas: covering index, no scan, no B-tree
SCAN suttas USING COVERING INDEX idx_suttas_language
Run Time: ~9 ms
```

On mobile with cold SQLite pages, the dict variant is significantly worse —
this is the entire source of the `S → D` hitch.

### 2.2 Startup phase before `apply_theme()`

`apply_theme()` is the first statement of `SuttaSearchWindow.qml::Component.onCompleted`
(line 1181). Everything before that runs in `cpp/gui.cpp::start()`:

1. `dotenv_c`, `find_port_set_env_c`, `init_app_globals`, `remove_download_temp_folder`,
   `ensure_no_empty_db_files`, `check_delete_files_for_upgrade`, desktop file refresh.
2. `QApplication` construction, system tray, icon load.
3. `init_app_data()` → `AppData::new()` (`backend/src/app_data.rs:35`):
   - Opens three SQLite DBs (appdata, dictionaries, dpd) and runs Diesel migrations
     check on each.
   - Reads `AppSettings` row into `app_settings_cache`.
   - **If `cached_shipped_source_uids` or `cached_commentary_definitions_source_uids`
     is empty**, calls `refresh_dict_source_uid_caches()` which runs:
     - `list_shipped_source_uids` — `SELECT DISTINCT dict_label FROM dict_words JOIN dictionaries ON ... WHERE NOT is_user_imported` (≈115 ms warm).
     - `list_distinct_bold_def_ref_codes` — DPD scan.
   - `init_fulltext_searcher()` opens the Tantivy indexes (cold mmap).
4. `import_user_data_after_upgrade`, `cleanup_stale_legacy_userdata`,
   `check_and_configure_for_first_start`.
5. `reconcile_dict_indexes_blocking_c` (only when needed; can take seconds).
6. `WindowManager::create_sutta_search_window` — parses
   **`SuttaSearchWindow.qml` (156 KB / 3414 lines)** plus every component it
   directly instantiates (GlossTab.qml 80 KB, PromptsTab.qml 43 KB,
   AppSettingsWindow.qml 42 KB, UpdateNotificationDialog.qml 35 KB,
   ChantingPracticeReviewWindow…, etc.). QML parse + JS compile of all of
   these happens before `Component.onCompleted` fires.
7. `Component.onCompleted` → `apply_theme()`.

### 2.3 Tantivy vs SQLite index for distinct languages

Evaluated for fairness: should all three areas pull languages from
Tantivy instead of SQLite?

| Aspect                          | SQLite covering index                                  | Tantivy enumeration                                |
|---------------------------------|--------------------------------------------------------|----------------------------------------------------|
| Cost of DISTINCT enumeration    | O(distinct values) via index leaf walk                 | No native DISTINCT — must enumerate term dict or scan docs |
| Implementation in our code      | 1-line `CREATE INDEX … (language)`                     | New term-iterator code per index, per area         |
| Schema requirement              | None beyond the index                                  | Requires `language` as an indexed (not text-only) field |
| Liveness after writes           | Always up-to-date                                      | Indexes are rebuilt on reconcile only — can lag (e.g. immediately after a user-dict import the lang is in SQLite but not in Tantivy yet) |
| Fits existing project pattern   | Yes — `idx_suttas_language` already exists             | No — Tantivy is only consulted for fulltext search |
| Risk                            | Negligible (one extra index)                           | Larger surface area; correctness depends on indexer being current |

**Conclusion.** SQLite covering indexes are the right substrate. Tantivy is
a fulltext store, not a column store — it does not improve distinct-value
enumeration in any meaningful way for the table sizes we have, and would
introduce a staleness window after user-dict mutations.

The runtime cost is then handled by an `AppSettings` cache layer on top
of those (now-fast) queries, mirroring `cached_shipped_source_uids`.

## 3. Goals

- `S → D` switch is indistinguishable from `S → L` and `S → S` on mobile.
- First app launch after install/upgrade reaches `apply_theme()` without
  blocking on any DB DISTINCT scan.
- Subsequent launches reach `apply_theme()` within the time budget of QML
  parse + theme JSON read — no DB-scan work on the critical path.
- User-visible language filter dropdowns stay correct after sutta language
  download/removal and after user-dict import/delete/rename.

## 4. Non-goals

- No change to the Tantivy fulltext schema or the indexer.
- No change to the FTS5 trigram indexes or the dict_words rowid convention.
- No change to the search query semantics or paging.
- No theme system rewrite. `apply_theme()` is unchanged; we only move work
  out of its way.

## 5. Design

### 5.1 Dictionary SQLite index (re-bootstrap fix)

Add a covering index in
`backend/migrations/dictionaries/2025-05-03-143320_create-tables/up.sql`
(this DB is re-bootstrapped, no incremental migration needed):

```sql
-- Speeds up SELECT DISTINCT language FROM dict_words (search-bar lang filter)
-- and any future language-scoped lookup. Covering: SQLite serves DISTINCT
-- entirely from the index leaf.
CREATE INDEX dict_words_language_idx ON dict_words(language);
```

Corresponding `down.sql` adds `DROP INDEX IF EXISTS dict_words_language_idx;`.

**No equivalent index is needed for the other areas:** `suttas.language`
already has `idx_suttas_language`, and `book_spine_items.language` /
`books.language` already have `idx_book_spine_items_language` and
`idx_books_language`. The two `*-language` indexes plus the new dict index
cover all three search areas.

### 5.2 AppSettings cache for language labels (all three areas)

Extend `AppSettings` (`backend/src/app_settings.rs`) with three new fields,
populated and refreshed in the same way `cached_shipped_source_uids`
already is:

```rust
pub cached_sutta_languages: Vec<String>,
pub cached_dict_languages: Vec<String>,
pub cached_library_languages: Vec<String>,
```

Defaults in `AppSettings::default()`: empty `Vec`.

Add to `AppData` (`backend/src/app_data.rs`):

```rust
pub fn refresh_language_caches(&self) { /* writes all three */ }

pub fn get_cached_sutta_languages(&self) -> Vec<String>;
pub fn get_cached_dict_languages(&self) -> Vec<String>;
pub fn get_cached_library_languages(&self) -> Vec<String>;
```

Each `get_…` reads from `app_settings_cache` without touching SQLite.
`refresh_language_caches` writes through to the appdata row using the same
`persist_app_settings` path as the existing UID caches.

Bridge layer (`bridges/src/sutta_bridge.rs`): `get_sutta_language_labels`,
`get_library_language_labels`, `get_dict_language_labels` switch to
reading the cached vectors via `get_cached_*`. The current
`SELECT DISTINCT` code paths in `backend/src/db/appdata.rs`,
`backend/src/db/dictionaries.rs`, and `search/indexer.rs::get_library_languages`
stay (they're the source of truth used by the refresh job), but the GUI
thread no longer calls them directly.

### 5.3 Cache lifecycle

**Write at bootstrap (cli/src/bootstrap).**
At the end of bootstrap, after all dict/sutta/library DBs are populated
and FTS5 indexes are created, run a one-shot computation of all five
caches:

- `cached_shipped_source_uids`
- `cached_commentary_definitions_source_uids`
- `cached_sutta_languages`
- `cached_dict_languages`
- `cached_library_languages`

and persist them into the `app_settings` JSON row of `appdata.sqlite3`.
This means a shipped DB has the caches pre-warmed; first launch never
needs to compute them. The helper lives in `backend/src/app_data.rs`
as `warm_caches_into_appdata()` (small enough that a dedicated
`cli/src/bootstrap/cache_warm.rs` module was not warranted); bootstrap
calls it from `cli/src/bootstrap/mod.rs`.

**Read at startup (backend/src/lib.rs).**
`AppData::new()` stops calling `refresh_dict_source_uid_caches()`
synchronously. The empty-cache detection and background spawn move into
`init_app_data()` *after* `APP_DATA.set(app_data)` so the worker can
reach `get_app_data()` — `APP_DATA` is `OnceLock<AppData>` (value-typed,
not `Arc`), so the spawn cannot capture a handle from inside
`AppData::new()` and cannot reach the still-uninitialised
`OnceLock` either.

```rust
// backend/src/lib.rs::init_app_data
pub extern "C" fn init_app_data() {
    if APP_DATA.get().is_none() {
        let app_data = AppData::new();             // no inline warm
        APP_DATA.set(app_data).expect("Can't set AppData");

        let needs_warm = {
            let s = get_app_data().app_settings_cache.read().unwrap();
            s.cached_shipped_source_uids.is_empty()
                || s.cached_commentary_definitions_source_uids.is_empty()
                || s.cached_sutta_languages.is_empty()
                || s.cached_dict_languages.is_empty()
                || s.cached_library_languages.is_empty()
        };
        if needs_warm {
            std::thread::spawn(|| {
                let app_data = get_app_data();     // OnceLock is set now
                app_data.refresh_dict_source_uid_caches();
                app_data.refresh_language_caches();
            });
        }
    }
    // ... fulltext searcher init (see §5.4)
}
```

Until the background job finishes, `get_cached_*` returns whatever is in
the cache (possibly empty). Empty cache → dropdown shows just the
sentinel ("Language" / "Lang"); the dropdown will reload on the next area
change once the background job has populated the cache. Acceptable
degradation for the first-launch-of-legacy-DB edge case.

**Refresh after mutations (bridges).**
The existing `refresh_dict_source_uid_caches()` call sites in
`bridges/src/dictionary_manager.rs` (8 sites: import / delete / rename
flows, lines 343–563) are extended to also call `refresh_language_caches()`.
A combined helper `refresh_all_dict_caches()` keeps the call sites
single-line; it dispatches the work to a background thread so the
mutation bridge call returns promptly:

```rust
fn refresh_all_dict_caches() {
    std::thread::spawn(|| {
        let app_data = get_app_data();
        app_data.refresh_dict_source_uid_caches();
        app_data.refresh_language_caches();
    });
}
```

This keeps the GUI thread off the source-of-truth DISTINCT scans even
for runtime mutations; the dropdown reflects the new state on the next
area switch after the refresh thread finishes (typically <100 ms).

Sutta language download/removal (`bridges/src/asset_manager.rs` →
`import_suttas_lang_to_appdata`, and `backend/src/db/appdata.rs` →
`remove_sutta_languages`) gets one new call that spawns
`refresh_language_caches()` on a background thread on completion (same
pattern — never block the calling thread).

Library mutations: there is **no runtime book-import bridge** today
(`cli/src/bootstrap/library_imports.rs` runs at bootstrap time, not from
the running app). The library cache is therefore covered by the
bootstrap warm step; no runtime refresh hook is needed. If a runtime
book-import path is added later, that PR adds the matching
`refresh_language_caches()` call.

### 5.4 Startup: move remaining synchronous work off the critical path

After 5.2/5.3 the dict-UID and language caches no longer block startup.
Remaining items to consider in the same pass:

1. **`init_fulltext_searcher()`** runs inline today (`backend/src/lib.rs:111`,
   called from `init_app_data`). The searcher is not needed until the
   user fires a query. Move to a `std::thread::spawn` so
   `SuttaSearchWindow.qml` can render. `FulltextSearcher` access already
   goes through `FULLTEXT_SEARCHER: RwLock<Option<…>>`; callers must
   treat `None` as "searcher not ready yet, retry/skip" (today `None`
   only occurs on open failure — we extend that to "still warming").

   `SuttaBridge` already exposes `#[qproperty(bool, db_loaded)]`
   (sutta_bridge.rs:572) with the property write at sutta_bridge.rs:1375.
   Mirror that pattern: add `#[qproperty(bool, searcher_ready)]`, set it
   from the background thread once `FULLTEXT_SEARCHER` is installed. QML
   binds `enabled: SuttaBridge.db_loaded && SuttaBridge.searcher_ready`
   on the search button and gates `handle_query` on the same.

   The three other `reinit_fulltext_searcher()` call sites
   (`reconcile_dict_indexes_blocking_c` at lib.rs:241, and three sites
   in `sutta_bridge.rs` around lines 2790 / 2875 / 2993 driven by dict
   reconcile / upgrade flows) stay synchronous — they run after the
   cold-start path with their own progress UI, so blocking the caller
   is intentional. The async refactor only applies to the *first*
   `init_fulltext_searcher()` from `init_app_data`.

2. **`refresh_dict_source_uid_caches` legacy fallback** — covered above
   (background thread).

3. **`reconcile_dict_indexes_blocking_c`** — already runs in its own QML
   window with a progress bar; no change needed.

These are bounded, surgical changes. We do **not** restructure
`init_app_data` or `start()` ordering beyond moving the searcher.

### 5.5 QML cold-path Loader split

Goal: shrink the QML parse cost incurred before `Component.onCompleted`
fires, since on mobile QML parse dominates after the DB fixes above.

Audit of top-level children of `SuttaSearchWindow.qml` (file sizes shown):

Classify by **actual root element** of the .qml file — `ApplicationWindow`
roots cannot be hosted by `Loader` (see §5.5.2).

| Component                          | File size | Root element        | Visible at startup? | Plan                                  |
|------------------------------------|-----------|---------------------|---------------------|---------------------------------------|
| `SearchBarInput`                   | ~16 KB    | `Item`              | Yes                 | Keep inline                           |
| `DrawerMenu`                       | ~3 KB     | `Drawer`            | Mobile only         | Keep inline (small, used early)       |
| `FulltextResults`                  | ~?        | `Item`              | Sidebar tab 0       | Keep inline (first sidebar tab)       |
| `DictionaryTab`                    | ~?        | `ColumnLayout`      | Sidebar tab 1       | `Loader { active: false }`            |
| `GlossTab`                         | **80 KB** | `ColumnLayout`      | Sidebar tab 2       | **`Loader { active: false }`** (biggest single win) |
| `PromptsTab`                       | **43 KB** | `ColumnLayout`      | Sidebar tab 3       | **`Loader { active: false }`**        |
| `TocTab`                           | ~?        | `ColumnLayout`      | Sidebar tab 4       | `Loader { active: false }`            |
| `AboutDialog`                      | small     | `ApplicationWindow` | On demand           | `Component` + `createObject(null)`    |
| `SystemPromptsDialog`              | medium    | `ApplicationWindow` | On demand           | `Component` + `createObject(null)`    |
| `ModelsDialog`                     | medium    | `ApplicationWindow` | On demand           | `Component` + `createObject(null)`    |
| `AnkiExportDialog`                 | medium    | `ApplicationWindow` | On demand           | `Component` + `createObject(null)`    |
| `DatabaseValidationDialog`         | small     | `ApplicationWindow` | On demand           | `Component` + `createObject(null)`    |
| `DhammaTextSourcesDialog`          | small     | `ApplicationWindow` | On demand           | `Component` + `createObject(null)`    |
| `UpdateNotificationDialog`         | **35 KB** | `ApplicationWindow` | On demand           | **`Component` + `createObject(null)`** |
| `AppSettingsWindow`                | **42 KB** | `ApplicationWindow` | On demand           | **`Component` + `createObject(null)`** |
| `TabListDialog`                    | medium    | `Dialog`            | On demand           | `Loader`                              |
| `ColorThemeDialog`                 | small     | `Dialog`            | On demand           | `Loader`                              |

**Highest-value first**: GlossTab, PromptsTab, AppSettingsWindow,
UpdateNotificationDialog, DictionaryTab. These five alone account for the
bulk of QML bytes parsed at startup.

#### 5.5.0 Pre-flight: route eager bindings off deferred components

Several eager bindings in `SuttaSearchWindow.qml` reach *into* components
that will become deferred-load. Each must be re-routed *before* any
wrapping, or the deferred component will be referenced as `null` and
crash on first paint.

1. **`webview_visible` (line 69) fans out across nearly every dialog.**
   It currently reads `.visible` on `about_dialog`, `models_dialog`,
   `anki_export_dialog`, `gloss_tab.commonWordsDialog`, `tab_list_dialog`,
   `database_validation_dialog`, `app_settings_window`, `info_dialog`,
   `mobile_menu`. Rewrite the binding once, up front, to use the
   `?.item?.visible ?? false` form (or a `dialog_visible` proxy on each
   loader/component holder). After this rewrite each individual wrap is
   a one-liner with no further binding churn.

2. **`app_settings_window.search_as_you_type` is read eagerly from three
   sites in `SuttaSearchWindow.qml`** (lines 428, 2285, 3183) — before
   the user ever opens settings. `SuttaBridge.get_search_as_you_type()`
   already exists (used by AppSettingsWindow.qml:983) and is the
   authoritative source. Re-route the three sites to the bridge call,
   or to a root-level `property bool search_as_you_type:
   SuttaBridge.get_search_as_you_type()` mirror updated on
   `set_search_as_you_type`. AppSettingsWindow becomes a writer-only
   for this value; the root no longer depends on the window object
   existing.

3. **`app_settings_window.open_find_in_sutta_results` is read eagerly**
   at SuttaSearchWindow.qml:1004 — same class of bug as (2). Re-route
   via a bridge getter / root property mirror so the read does not
   require `AppSettingsWindow` to exist yet.

4. **`gloss_tab.commonWordsDialog` is referenced from the root** at
   line 69 (`webview_visible`) and line 1978 (`onTriggered:
   gloss_tab.commonWordsDialog.open()`). Lifting `commonWordsDialog`
   out of `GlossTab` to the SuttaSearchWindow root is the simplest fix
   — it's a `Dialog` that doesn't need `GlossTab` to exist. Alternative:
   keep it inside GlossTab and replace the two root references with
   `gloss_tab_loader.item?.commonWordsDialog?.visible ?? false` and a
   `Connections` handler.

5. **`models_dialog.auto_retry.checked` is read across deferred
   components** at SuttaSearchWindow.qml:3334 (from `GlossTab`-area
   wiring) and :3351 (from `PromptsTab`-area wiring). This is a
   *cross-deferred-component* binding: both ModelsDialog and the
   referencing tab become deferred, so the read evaluates to `null`
   until the user opens ModelsDialog at least once. Route through a
   new `SuttaBridge.get_models_auto_retry()` getter, or a root mirror
   `property bool models_auto_retry` initialised from settings and
   updated by ModelsDialog when it eventually loads.

6. **`update_notification_dialog` method calls fire on signals that
   may arrive before any user interaction** —
   `show_app_update` (2236), `show_db_update` (2241),
   `show_obsolete_warning` (2249), `show_no_updates` (2253). These run
   from `Connections` handlers on update-check signals, so the proxy
   wrapper for `UpdateNotificationDialog` must forward all four call
   forms (not just `show()`): each call activates the
   `createObject(null)` instance and invokes the matching method on it.

7. **Menu `onTriggered` handlers** that call `.show()` on
   to-be-deferred ApplicationWindow components:
   `app_settings_window.show()` (1490), `anki_export_dialog.show()`
   (1986), `models_dialog.show()` (1999),
   `system_prompts_dialog.show()` (2007),
   `dhamma_text_sources_dialog.show()` (2028),
   `about_dialog.show()` (2035). Each becomes a call to the matching
   `open_*_window()` proxy that lazily `createObject(null)`s on first
   use.

Pre-flight is its own task — no Loader wrapping yet, the app should
behave identically after this step.

#### 5.5.1 Pattern: `Dialog`/`Popup` roots — `Loader`

Pattern for tabs in the right-side `StackLayout`:

```qml
Loader {
    id: gloss_tab_loader
    active: false                 // becomes true on first tab activation
    sourceComponent: gloss_tab_component
    // Forward the public properties the rest of the window touches, or
    // route accesses through the loader.
}

Component {
    id: gloss_tab_component
    GlossTab { id: gloss_tab; /* … */ }
}
```

`rightside_tabs.onCurrentIndexChanged` (or each tab button's `onClicked`)
flips `gloss_tab_loader.active = true`. Once `true`, it stays true for
the session.

Pattern for `Dialog` / `Popup` roots (e.g. `info_dialog`,
`database_validation_dialog`, `tab_list_dialog`, `related_sutta_not_found_dialog`):

```qml
Loader {
    id: about_dialog_loader
    active: false
    sourceComponent: about_dialog_component
    readonly property bool dialog_visible: (item && item.visible) || false
    function open() { active = true; if (item) item.open(); }
}

Component {
    id: about_dialog_component
    AboutDialog { /* … */ }
}
```

Any call site that currently does `about_dialog.open()` becomes
`about_dialog_loader.open()`; references to `.visible` go through the
`dialog_visible` proxy.

#### 5.5.2 Pattern: `ApplicationWindow` roots — `Component` + `createObject`

`AppSettingsWindow.qml`, `AboutDialog.qml` (despite its name), and
`UpdateNotificationDialog.qml` all have **`ApplicationWindow`** roots,
not `Dialog`. `Loader` instantiates an `Item` child and is not a
suitable host for a top-level window. Use `Component` +
`createObject(null)` (null parent → top-level window):

```qml
Component {
    id: app_settings_window_component
    AppSettingsWindow { /* … */ }
}

property var app_settings_window_instance: null
readonly property bool app_settings_window_visible:
    (app_settings_window_instance && app_settings_window_instance.visible) || false

function open_app_settings_window() {
    if (!app_settings_window_instance) {
        app_settings_window_instance = app_settings_window_component.createObject(null);
    }
    app_settings_window_instance.show();
}
```

The instance lives for the rest of the session after first open (no
`destroy()` on close — re-show is cheap). The root's `webview_visible`
binding reads `app_settings_window_visible` (the proxy property), not
the instance directly.

**Multi-method forwarding.** When a component is invoked via more than
one entry point (`UpdateNotificationDialog` is called as
`show_app_update()`, `show_db_update()`, `show_obsolete_warning()`,
`show_no_updates()` from update-check signal handlers), the proxy
exposes one function per call form, each lazily creating the instance
before forwarding:

```qml
function ensure_update_notification_window() {
    if (!update_notification_window_instance) {
        update_notification_window_instance =
            update_notification_window_component.createObject(null);
    }
    return update_notification_window_instance;
}
function show_app_update(info)   { ensure_update_notification_window().show_app_update(info); }
function show_db_update(info)    { ensure_update_notification_window().show_db_update(info); }
function show_obsolete_warning() { ensure_update_notification_window().show_obsolete_warning(); }
function show_no_updates()       { ensure_update_notification_window().show_no_updates(); }
```

**Out of scope for this PRD**: refactoring `SuttaSearchWindow.qml`
internals beyond Loader-wrapping. The window's own size, the search bar,
the sidebar host `StackLayout`, and the webview host stay inline so the
first paint is unchanged.

## 6. Acceptance criteria

1. `EXPLAIN QUERY PLAN SELECT DISTINCT language FROM dict_words` shows
   `SCAN dict_words USING COVERING INDEX dict_words_language_idx` after a
   fresh re-bootstrap.
2. Switching to Dictionary in the search bar takes the same wall-clock
   time as switching to Suttas (within noise) on mobile.
3. On a freshly bootstrapped DB the appdata `app_settings` JSON contains
   non-empty values for all five cache fields (UID × 2, language × 3).
4. `AppData::new()` does **not** call any of the five cache-computing
   queries on the foreground thread when the caches are non-empty.
5. `SuttaBridge::get_*_language_labels` reads from the in-memory cache;
   no SQLite call appears in a trace from `S/D/L` button presses.
6. After a user-dict import / delete / rename, the dict language
   dropdown reflects the new state on the next area switch.
7. After a sutta language download / removal, the sutta language
   dropdown reflects the new state on the next area switch.
8. `SuttaSearchWindow.qml` measures (instrumentation via `console.time`
   wrapping `Component.onCompleted`) at least a 30 % reduction in
   pre-`apply_theme` parse cost on a representative Android device.
9. All existing functionality of cold-loaded components remains
   reachable from the menus and shortcuts that drive them today.

## 7. Implementation plan (suggested ordering)

1. **DB index.** Add `dict_words_language_idx` to dictionaries migration
   (up + down). Re-bootstrap; verify EXPLAIN. (Bootstrap is driven by
   the `cli` binary — see `cli/src/main.rs` for the exact subcommand.)
2. **AppSettings fields.** Add three `cached_*_languages` fields with
   defaults. Migrate is a no-op because `AppSettings` is JSON-blob in a
   single row — old rows deserialize with empty `Vec`s via
   `#[serde(default)]`. Verify the `#[serde(default)]` is per-field (or
   on the struct) so a missing key round-trips cleanly.
3. **Backend cache layer.** Add `refresh_language_caches`,
   `get_cached_sutta_languages`, `get_cached_dict_languages`,
   `get_cached_library_languages` to `AppData`. Add an umbrella
   `refresh_all_dict_caches()` that calls both
   `refresh_dict_source_uid_caches` and `refresh_language_caches`.
   The underlying source-of-truth methods stay where they are —
   `DbManager::get_sutta_languages` (db/mod.rs:167 → db/appdata.rs:18),
   `Dictionaries::get_distinct_languages` (db/dictionaries.rs:36),
   `indexer::get_library_languages` (search/indexer.rs). They are
   called *only* by the refresh job; bridges no longer call them
   directly.
4. **Bridge switchover.** Point `SuttaBridge::get_*_language_labels`
   (sutta_bridge.rs:3528 / 3539 / 3558) at the cache.
5. **Bootstrap cache warming.** Add `warm_caches_into_appdata(...)`
   step at the end of `cli/src/bootstrap/mod.rs`. The helper opens
   `appdata.sqlite3` via the existing Diesel models, deserialises the
   `app_settings` JSON row into `AppSettings`, populates the five
   `cached_*` fields, re-serialises and writes back — same shape as
   the runtime `persist_app_settings` path. Re-bootstrap and verify
   the JSON round-trips cleanly through `AppSettings`.
6. **Background fallback warming.** Add the empty-cache check + thread
   spawn to `init_app_data()` (lib.rs) **after `APP_DATA.set(app_data)`**
   — *not* inside `AppData::new()`, which would deadlock against an
   uninitialised `OnceLock`. Remove the inline
   `refresh_dict_source_uid_caches()` call from `AppData::new()`.
7. **Sutta refresh hooks.** At the end of the success branch of
   `import_suttas_lang_to_appdata` (asset_manager.rs, after the call
   at line 551) and at the end of `db::appdata::remove_sutta_languages`
   (around line 1565), spawn a thread that calls
   `refresh_language_caches()` — never block the calling thread on the
   DISTINCT scan. Replace the eight `refresh_dict_source_uid_caches()`
   call sites in `dictionary_manager.rs` (lines 343, 348, 360, 415,
   418, 430, 517, 563) with `refresh_all_dict_caches()`, which itself
   dispatches to a background thread. No runtime library-import path
   exists; the library cache is covered by bootstrap.
8. **Async fulltext searcher.** Move the `init_fulltext_searcher()`
   call out of the synchronous startup path (currently lib.rs:111) into
   a `std::thread::spawn`. Add `#[qproperty(bool, searcher_ready)]` to
   `SuttaBridge` mirroring the existing `db_loaded` qproperty pattern
   (sutta_bridge.rs:572, 1375); set it from the background thread once
   `FULLTEXT_SEARCHER` is installed. Backend safety is already covered
   — every consumer goes through `with_fulltext_searcher() ->
   Option<R>` (lib.rs:208) and handles `None`. Enumerate every
   `handle_query` entry point in QML (button click, keyboard shortcut,
   drawer menu, programmatic triggers) and gate each on
   `db_loaded && searcher_ready` so the user never sees a silent
   no-op. Leave the three `reinit_fulltext_searcher()` call sites in
   `sutta_bridge.rs` (~2790, 2875, 2993) and the post-reconcile reinit
   at lib.rs:241 synchronous — they run with their own progress UI.
9. **QML pre-flight (no Loader wrapping yet).** Per §5.5.0:
   (a) rewrite `webview_visible` (SuttaSearchWindow.qml:69) to use
   `?.item?.visible ?? false` / `dialog_visible` proxies for every
   referenced dialog; (b) re-route the three
   `app_settings_window.search_as_you_type` reads to
   `SuttaBridge.get_search_as_you_type()` or a root mirror;
   (c) lift `commonWordsDialog` out of `GlossTab` to the root, or
   adapt the two references. Verify the app still behaves identically.
10. **QML Loader/Component split — phase A (highest impact).** GlossTab,
    PromptsTab, DictionaryTab via Loader (Item-rooted tabs).
    AppSettingsWindow, UpdateNotificationDialog via
    `Component { ApplicationWindow {} }` + `createObject(null)` (§5.5.2).
    Add `Component.onCompleted` instrumentation (Logger.info with
    `Date.now()` deltas) so phase B can be measured.
11. **QML Loader/Component split — phase B.** AboutDialog
    (`ApplicationWindow` → `Component` + `createObject`).
    SystemPromptsDialog, ModelsDialog, AnkiExportDialog,
    DatabaseValidationDialog, DhammaTextSourcesDialog, TabListDialog,
    ColorThemeDialog (Dialog/Popup → Loader). TocTab (tab Loader).
12. **Manual mobile verification.** Run on Android, confirm visible
    delays are gone and no regression in dialog-open latency. Trace
    SQLite during `S`/`D`/`L` button presses to confirm zero
    `SELECT DISTINCT language` calls (acceptance criterion §6.5).

## 8. Risks and mitigations

- **Stale cache after schema-touching bootstrap.** Always warm at bootstrap;
  background-refresh on empty at startup is the safety net.
- **Loader/Component-wrapped components break property access from the
  window root.** The known eager-binding sites in `SuttaSearchWindow.qml`
  are catalogued in §5.5.0 (items 1–7): `webview_visible` at line 69;
  `app_settings_window.search_as_you_type` at lines 428, 2285, 3183;
  `app_settings_window.open_find_in_sutta_results` at line 1004;
  `gloss_tab.commonWordsDialog` at lines 69 and 1978;
  `models_dialog.auto_retry.checked` at lines 3334 and 3351 (the most
  dangerous: a *cross-deferred-component* binding);
  `update_notification_dialog.show_*` method calls at lines 2236, 2241,
  2249, 2253; menu `.show()` triggers at lines 1490, 1986, 1999, 2007,
  2028, 2035; and `tab_list_dialog.visible` at lines 1166, 1172, 1296,
  1774, 1844, 1861, 1942, 1959. The §5.5.0 pre-flight step handles all
  of these *before* any wrapping. Each wrapped component then gets
  either a `dialog_visible` proxy (Dialog/Popup via Loader) or a
  top-level `*_window_visible` mirror plus per-method proxy functions
  (ApplicationWindow via Component + createObject).
- **`ApplicationWindow` ≠ `Dialog`.** `AppSettingsWindow`, `AboutDialog`,
  and `UpdateNotificationDialog` are `ApplicationWindow` roots. `Loader`
  hosts `Item` children — it is the wrong wrapper. Use
  `Component { ApplicationWindow {} }` + `createObject(null)` for these
  three; reserve `Loader` for `Dialog` / `Popup` roots.
- **Background searcher init: first query hits an un-ready searcher.**
  Gate `handle_query` on `db_loaded && searcher_ready`; the search button
  is already disabled until db_loaded — extend the disable to cover
  searcher_ready. Time-to-ready is a small fraction of QML parse time
  so the user will not perceive this. **Enumerate every `handle_query`
  call site** (button click, keyboard shortcut, programmatic search
  triggers, drawer menu) and gate each — disabling the button alone is
  not sufficient. Callers into `with_fulltext_searcher` already get
  `Option<R>` so the backend is safe regardless; the gating is a UX
  concern (no silent no-op queries).
- **JSON-blob backwards compatibility.** New fields default to empty Vec
  via `#[serde(default)]`. Old `app_settings` rows continue to load.

## 9. Out of scope (parking lot for later)

- Splitting `SuttaSearchWindow.qml` itself into smaller files.
- Pre-compiling QML to .qmlc cache files in the build.
- Async open of the three SQLite handles in `DbManager::new()`.
- Replacing `SELECT DISTINCT` calls in the indexer (used at bootstrap
  / reconcile) with the cached values — not on a hot path.
