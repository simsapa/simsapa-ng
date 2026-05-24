# Startup Sequence, Caches, and Cold-Path Design

This document describes the intended startup sequence of the Simsapa app
— what runs synchronously, what is deferred to background threads, what
is deferred to first-use QML instantiation, and the design reasons
behind each split. It complements the PRD
[`tasks/prd-startup-and-search-area-switch-perf.md`](../tasks/prd-startup-and-search-area-switch-perf.md)
and is the source of truth for "where should this work go" decisions
when adding new startup-time code.

## 1. Time budget on the critical path

The user-perceived startup window is from process launch to the first
themed paint of `SuttaSearchWindow.qml`. The theme is applied by
`apply_theme()`, which is the **first statement** of
`SuttaSearchWindow.qml::Component.onCompleted`. Everything between
process start and that point runs against the system default palette,
so any synchronous work there is visible to the user as an "un-themed
flash."

The design goal is: **nothing on the critical path before
`apply_theme()` should touch SQLite beyond the three handle opens, and
nothing should run a `SELECT DISTINCT` scan, a Tantivy mmap, or any
other O(rows) work.**

## 2. The startup phases

```
process start
│
├─ cpp/main.cpp::main()
│   ├─ dotenv_c, find_port_set_env_c, init_app_globals
│   ├─ remove_download_temp_folder, ensure_no_empty_db_files
│   ├─ check_delete_files_for_upgrade, desktop file refresh
│   ├─ QApplication construction, system tray, icon load
│   │
│   ├─ init_app_data()                        backend/src/lib.rs
│   │   ├─ AppData::new()                     [SYNC, fast]
│   │   │   ├─ DbManager::new()  — opens 3 SQLite handles + Diesel migrations check
│   │   │   └─ read app_settings row into in-memory cache
│   │   │
│   │   ├─ APP_DATA.set(app_data)
│   │   │
│   │   ├─ if any of the 5 caches are empty:   [BG THREAD, see §3]
│   │   │     spawn → refresh_dict_source_uid_caches + refresh_language_caches
│   │   │
│   │   └─ spawn init_fulltext_searcher       [BG THREAD, see §4]
│   │
│   ├─ import_user_data_after_upgrade, cleanup_stale_legacy_userdata
│   ├─ check_and_configure_for_first_start
│   ├─ reconcile_dict_indexes_blocking_c       [shows its own progress window if work needed]
│   │
│   └─ WindowManager::create_sutta_search_window
│       └─ QML parse of SuttaSearchWindow.qml + its directly-instantiated children
│           (heavy children are wrapped — see §5)
│
└─ SuttaSearchWindow.qml::Component.onCompleted
    └─ apply_theme()  ← critical-path target reached
```

The two-window orchestration (`DictionaryIndexProgressWindow` →
`SuttaSearchWindow`) means the user *sees* a themed window during
reconcile; only the path from "reconcile finished" (or "no reconcile
needed") to `apply_theme()` is the un-themed window. That path is what
this design protects.

## 3. The five `AppSettings` caches

Five queries used to run on the GUI thread during startup or during
search-bar interaction. All five now live as cached `Vec<String>` /
`Vec<String>` fields inside the single-row JSON `app_settings` table:

| Cache field                                        | Source-of-truth query                                                                                 | Used by                                            |
|----------------------------------------------------|-------------------------------------------------------------------------------------------------------|----------------------------------------------------|
| `cached_shipped_source_uids`                       | `dict_words ⨝ dictionaries` filtered by `NOT is_user_imported`                                        | Dictionary search inclusion-set filtering          |
| `cached_commentary_definitions_source_uids`        | DPD `bold_definitions.ref_code` distinct                                                              | Commentary-definition source-uid set               |
| `cached_sutta_languages`                           | `SELECT DISTINCT language FROM suttas` (covering index `idx_suttas_language`)                          | Search-bar language filter, Suttas area            |
| `cached_dict_languages`                            | `SELECT DISTINCT language FROM dict_words` (covering index `dict_words_language_idx`)                  | Search-bar language filter, Dictionary area        |
| `cached_library_languages`                         | `book_spine_items ∪ books` distinct languages (Rust-side merge)                                       | Search-bar language filter, Library area           |

### Why a JSON blob in `app_settings`, not a dedicated table

`AppSettings` is already a single-row JSON-serialised blob; adding three
`Vec<String>` fields with `#[serde(default)]` is a no-op migration that
deserialises cleanly from old rows. There is no separate table to
provision, no schema bump, and the existing `persist_app_settings`
write path is reused unchanged.

### Cache lifecycle

**Write at bootstrap.** At the end of `cli/src/bootstrap/mod.rs`,
`warm_caches_into_appdata(appdata_path, dict_path, dpd_path)` computes
all five values and writes them into the shipped `appdata.sqlite3`. A
freshly downloaded DB therefore arrives with the caches pre-warmed —
**first launch never needs to compute them**.

**Read at startup.** `AppData::new()` only reads the in-memory
`app_settings_cache`. The empty-cache fallback check and refresh spawn
sit in `init_app_data()` (lib.rs) *after* `APP_DATA.set(app_data)`,
because:

- `APP_DATA` is `OnceLock<AppData>` (value-typed, not `Arc`).
- A thread spawned from inside `AppData::new()` cannot capture an
  `Arc` handle (there is none), and cannot reach `get_app_data()`
  because the `OnceLock` is not yet populated.

Placing the spawn after the `set()` means the worker can call
`get_app_data()` safely. Until the worker finishes, `get_cached_*()`
returns an empty `Vec`; the search-bar language dropdown shows just
the sentinel ("Language" / "Lang"). This is acceptable degraded
behaviour for legacy or mid-development DBs that pre-date the warming
step.

**Refresh after mutations.** The two sutta write paths spawn
`refresh_language_caches()` on a background thread at the end of the
success branch — the calling thread (GUI or bridge worker) never
blocks on the DISTINCT scan:

- `bridges/src/asset_manager.rs::import_suttas_lang_to_appdata` (sutta language download)
- `backend/src/db/appdata.rs::remove_sutta_languages` (sutta language removal)

The eight `refresh_dict_source_uid_caches()` call sites in
`bridges/src/dictionary_manager.rs` (user-dict import / delete / rename)
become `refresh_all_dict_caches()`, the umbrella helper that *itself*
spawns a background thread to run the UID + language refresh
sequentially. Call sites remain single-line and synchronous-looking;
no DB-scan work runs on the GUI thread. There is no runtime
library-import path today; the library cache is covered by bootstrap.

The dropdown reflects the new state on the next area switch after the
refresh thread finishes — typically <100 ms post-mutation, so the user
never sees a stale dropdown in practice.

### Why not Tantivy for distinct values

Tantivy is a fulltext store, not a column store. `SELECT DISTINCT` over
a small cardinality (~10 languages, ~50 dict labels) walks one
secondary index leaf in SQLite; the equivalent in Tantivy requires
enumerating the term dictionary of a field that may not even be
indexed as a term field. SQLite covering indexes are strictly cheaper
for this access pattern, and they are always up to date with writes
(Tantivy indexes lag by a reconcile pass).

The `dict_words.language` covering index
(`dict_words_language_idx`, defined in the dictionaries migration
`2025-05-03-143320_create-tables/up.sql`) is what makes the source-of-
truth query fast enough that even the legacy-DB background refresh is
not user-visible.

## 4. The fulltext searcher

`init_fulltext_searcher()` opens the Tantivy indexes via mmap. On cold
mobile storage this is the slowest single thing in startup after QML
parse. The searcher is **not** needed before the user fires their first
query, so it runs on a background thread spawned from
`init_app_data()`.

`SuttaBridge` exposes the ready state as `#[qproperty(bool,
searcher_ready)]`, mirroring the existing `#[qproperty(bool,
db_loaded)]` pattern. The background thread flips the qproperty once
`FULLTEXT_SEARCHER` is installed. The search button and `handle_query`
gate on `SuttaBridge.db_loaded && SuttaBridge.searcher_ready`, so the
race ("user typed before searcher was ready") is handled by disabling
the search affordance, not by polling or retrying.

The three other `reinit_fulltext_searcher()` call sites
(`reconcile_dict_indexes_blocking_c` in lib.rs and three sites in
`sutta_bridge.rs` around the dict reconcile / upgrade flows) stay
synchronous. They run after the cold-start path is done, with their
own progress UI; blocking the caller there is intentional.

## 5. QML cold-path: Loader vs. Component + createObject

After the DB and searcher fixes, **QML parse + JS compile is the
dominant cost on the critical path**. The QML engine parses every
component that `SuttaSearchWindow.qml` directly instantiates before
`Component.onCompleted` fires.

`SuttaSearchWindow.qml` is 156 KB on its own. The big children are
GlossTab (80 KB), PromptsTab (43 KB), AppSettingsWindow (42 KB),
UpdateNotificationDialog (35 KB). These are deferred to first use.

### Two patterns, two reasons

QML offers `Loader` and `Component`; pick by **root element type**:

- **`Dialog` / `Popup` / `Item` roots** → `Loader { active: false }`.
  Loader hosts an `Item` child; this is the natural shape.
  `tab_list_dialog`, `info_dialog`, `database_validation_dialog`, etc.
  follow this pattern. The loader carries a `dialog_visible` proxy
  property and an `open()` function so call sites don't need to know
  the dialog is wrapped.

- **`ApplicationWindow` roots** → `Component { … }` +
  `createObject(null)`. `Loader` cannot host a top-level window;
  attempting it parents a `Window` inside an `Item` which silently
  breaks. Every "dialog" in the codebase that opens as its own OS
  window is `ApplicationWindow`-rooted — verified for
  `AppSettingsWindow`, `AboutDialog`, `UpdateNotificationDialog`,
  `ModelsDialog`, `AnkiExportDialog`, `SystemPromptsDialog`,
  `DatabaseValidationDialog`, `DhammaTextSourcesDialog`. Despite the
  `*Dialog.qml` filenames, all eight use the `Component` +
  `createObject(null)` pattern. Only `TabListDialog` and
  `ColorThemeDialog` are true `Dialog` roots that fit `Loader`.
  **Classify by reading the .qml root element, not by the filename.**
  The instance is created once on first show and retained for the
  session.

  When a component is invoked via more than one entry point —
  `UpdateNotificationDialog` is called as `show_app_update`,
  `show_db_update`, `show_obsolete_warning`, `show_no_updates` from
  update-check signal handlers — the proxy exposes one function per
  call form, each calling a small `ensure_*_window()` helper that
  lazily `createObject(null)`s before forwarding.

The right-side `StackLayout` tabs (GlossTab, PromptsTab, DictionaryTab,
TocTab) are `Item`-rooted and wrapped in `Loader { active: false }`
flipped to `true` on first activation of that tab.

### Pre-flight: kill eager bindings before wrapping

`SuttaSearchWindow.qml` has bindings that reach *into* components that
become deferred-load. Each one must be re-routed before wrapping, or
the deferred component will be `null` and crash on first paint. The
known sites are:

| Site                                                | Problem                                                                                  | Fix                                                                                                |
|-----------------------------------------------------|------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------|
| `webview_visible` (line 69)                         | Fans out to `.visible` on nearly every dialog                                            | Rewrite to `?.item?.visible ?? false` / `dialog_visible` proxy form for every referenced dialog    |
| `app_settings_window.search_as_you_type` (×3)       | Read before user opens settings; `null.search_as_you_type` after wrap                    | Re-route to `SuttaBridge.get_search_as_you_type()` (already exists) or a root property mirror      |
| `app_settings_window.open_find_in_sutta_results`    | Read before user opens settings (line 1004)                                              | Re-route to bridge getter / root mirror, same pattern                                              |
| `gloss_tab.commonWordsDialog` (lines 69, 1978)      | Root reads into GlossTab, which is itself deferred                                       | Lift `commonWordsDialog` out of `GlossTab` to the root (preferred), or adapt with `item?.…`        |
| `models_dialog.auto_retry.checked` (lines 3334, 3351) | **Cross-deferred-component binding** — both ModelsDialog *and* the referencing tab are deferred; read is `null.checked` until first ModelsDialog open | Add `SuttaBridge.get_models_auto_retry()` setter/getter pair; ModelsDialog becomes write-only      |
| `update_notification_dialog.show_*` (lines 2236, 2241, 2249, 2253) | Method calls in `Connections` handlers fire on signals before any user click | Proxy exposes one function per call form, each `ensure_*_window()` + forwards (see §5)             |
| Menu `.show()` triggers (lines 1490, 1986, 1999, 2007, 2028, 2035) | Direct `.show()` on `ApplicationWindow` instances that are about to be deferred | Replace with `open_*_window()` proxy that lazily `createObject(null)`s on first use                |
| `tab_list_dialog.visible` / `.open()` (×8)          | Eager `.visible` reads and `.open()` calls                                               | `dialog_visible` proxy property + `open()` function on the loader                                  |

The pre-flight is a discrete step (separate task in the implementation
plan): the app behaves identically after it, with no wrapping in
place. This makes the wrapping that follows a series of one-line
flips of `active`, instead of a thicket of binding edits.

## 6. Design implications (rules to add new code by)

1. **Anything that runs an O(rows) DB scan at startup needs a cached
   value in `app_settings`.** Source-of-truth queries stay in the DB
   layer; the cache is refreshed from the same mutation hooks that
   change the source rows.

2. **Background warming uses `init_app_data()`, not `AppData::new()`.**
   The `OnceLock<AppData>` constraint dictates this. If you add a new
   cache, follow the same pattern: empty check + `thread::spawn` after
   `APP_DATA.set(...)`, with `get_app_data()` inside the closure.

3. **Refresh on mutation, not on read.** The five caches are
   never-stale because every code path that *changes* the source rows
   calls the matching refresh helper. Read paths (`get_cached_*`)
   never go to SQLite.

4. **QML wrapping pattern follows the root element type.**
   - `ApplicationWindow` root → `Component` + `createObject(null)`.
   - `Dialog` / `Popup` / `Item` root → `Loader`.
   Do not Loader-wrap a `Window` — Loader hosts an `Item` child.

5. **Eager bindings into deferred components are a class of bug.**
   When adding a new binding that reads `<some_id>.<prop>`, check
   whether `<some_id>` is (or will be) wrapped. If so, use a proxy
   property on the wrapper, not a direct read.

6. **`SuttaBridge` qproperties are the right pattern for "ready"
   gates.** `db_loaded`, `searcher_ready`, and any future readiness
   signal use `#[qproperty(bool, …)]` mirrored to QML; the QML side
   binds `enabled:` on the affordance. No polling, no
   `Connections { onReady: … }` workarounds. **Gate every entry
   point**, not just the visible button — keyboard shortcuts, drawer
   menu items, and programmatic triggers into `handle_query` must each
   check the gate, or a shortcut-driven query will fire against a
   not-yet-ready searcher and silently no-op (the backend's
   `with_fulltext_searcher() -> Option<R>` makes this safe but the UX
   is bad).

7. **The critical path ends at `apply_theme()`.** When you add startup
   work, decide whether it has to happen before that point. If it
   doesn't, it goes on a background thread spawned from
   `init_app_data()` (or deferred to first QML use).

## 7. References

- PRD: [`tasks/prd-startup-and-search-area-switch-perf.md`](../tasks/prd-startup-and-search-area-switch-perf.md)
- Task list: [`tasks/tasks-prd-startup-and-search-area-switch-perf.md`](../tasks/tasks-prd-startup-and-search-area-switch-perf.md)
- Existing language-filter query logic: [`docs/language-filter-query-logic.md`](./language-filter-query-logic.md)
- Related: `backend/src/app_data.rs` (`refresh_dict_source_uid_caches`,
  `refresh_language_caches`, `refresh_all_dict_caches`),
  `backend/src/lib.rs` (`init_app_data`, `init_fulltext_searcher`),
  `bridges/src/sutta_bridge.rs` (`db_loaded`, `searcher_ready`
  qproperties), `cli/src/bootstrap/cache_warm.rs`
  (`warm_caches_into_appdata`).
