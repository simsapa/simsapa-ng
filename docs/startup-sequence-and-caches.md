# Startup Sequence, Caches, and Cold-Path Design

This document describes the intended startup sequence of the Simsapa app
— what runs synchronously, what is deferred to background threads, and
the design reasons behind each split. It complements the PRD
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

## 5. QML cold-path (not pursued)

Deferring heavy child components of `SuttaSearchWindow.qml` (GlossTab,
PromptsTab, AppSettingsWindow, UpdateNotificationDialog, …) via
`Loader { active: false }` and `Component { … } + createObject(null)`
was prototyped but **did not measurably reduce time-to-`apply_theme()`**
in practice and was rolled back. The QML parse work that runs before
`Component.onCompleted` is dominated by `SuttaSearchWindow.qml` itself,
not by its directly-instantiated children, so wrapping the children did
not move the critical-path number.

The DB-cache and async-fulltext-searcher work in §3 and §4 stands; the
QML splitting work does not. If this is revisited later, the eager
bindings into would-be-deferred components (`webview_visible` fan-out,
`app_settings_window.search_as_you_type` reads, `gloss_tab.commonWordsDialog`
reads, `models_dialog.auto_retry.checked` cross-component reads, the
`update_notification_dialog.show_*` signal-driven calls, and the menu
`.show()` triggers) are the gotchas to handle first — see the original
PRD §5.5.0 for the full catalogue.

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

4. **`SuttaBridge` qproperties are the right pattern for "ready"
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

5. **The critical path ends at `apply_theme()`.** When you add startup
   work, decide whether it has to happen before that point. If it
   doesn't, it goes on a background thread spawned from
   `init_app_data()`.

## 7. References

- PRD: [`tasks/prd-startup-and-search-area-switch-perf.md`](../tasks/prd-startup-and-search-area-switch-perf.md)
- Task list: [`tasks/tasks-prd-startup-and-search-area-switch-perf.md`](../tasks/tasks-prd-startup-and-search-area-switch-perf.md)
- Existing language-filter query logic: [`docs/language-filter-query-logic.md`](./language-filter-query-logic.md)
- Related: `backend/src/app_data.rs` (`refresh_dict_source_uid_caches`,
  `refresh_language_caches`, `refresh_all_dict_caches`),
  `backend/src/lib.rs` (`init_app_data`, `init_fulltext_searcher`),
  `bridges/src/sutta_bridge.rs` (`db_loaded`, `searcher_ready`
  qproperties), `backend/src/app_data.rs`
  (`warm_caches_into_appdata` — small helper inlined here rather than
  in its own bootstrap module).
