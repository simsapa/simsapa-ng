## Relevant Files

- `backend/migrations/dictionaries/2025-05-03-143320_create-tables/up.sql` - Add `dict_words_language_idx` covering index.
- `backend/migrations/dictionaries/2025-05-03-143320_create-tables/down.sql` - Drop the new index.
- `backend/src/app_settings.rs` - Add three `cached_*_languages: Vec<String>` fields with `#[serde(default)]`.
- `backend/src/app_data.rs` - `get_cached_*_languages`, `refresh_language_caches`, `refresh_all_dict_caches`; remove the inline cache warm from `AppData::new()`.
- `backend/src/lib.rs` - Empty-cache check + background-warm spawn placed *after* `APP_DATA.set(...)` in `init_app_data()`; move `init_fulltext_searcher()` to a background thread.
- `backend/src/db/appdata.rs` - Existing `get_sutta_languages` (used by `refresh_language_caches`); add `refresh_language_caches()` call at the end of `remove_sutta_languages` (line 1511).
- `backend/src/db/dictionaries.rs` - Existing `get_distinct_languages` (used by `refresh_language_caches`).
- `backend/src/search/indexer.rs` - Existing `get_library_languages` (used by `refresh_language_caches`).
- `bridges/src/sutta_bridge.rs` - Switch `get_sutta_language_labels` (3528) / `get_library_language_labels` (3539) / `get_dict_language_labels` (3558) to cache reads; add `#[qproperty(bool, searcher_ready)]` mirroring `db_loaded` (572 / 1375).
- `bridges/src/dictionary_manager.rs` - Replace 8 `refresh_dict_source_uid_caches()` call sites (lines 343, 348, 360, 415, 418, 430, 517, 563) with `refresh_all_dict_caches()` (which spawns the work on a background thread).
- `bridges/src/asset_manager.rs` - Spawn `refresh_language_caches()` on a background thread at the end of the success branch of `import_suttas_lang_to_appdata` (call site at line 551).
- `cli/src/bootstrap/mod.rs` - Call `warm_caches_into_appdata(...)` at the end of bootstrap.
- `cli/src/bootstrap/cache_warm.rs` - New helper that opens appdata via Diesel, deserialises `app_settings` into `AppSettings`, populates the five `cached_*` fields, re-serialises and writes back.
- `assets/qml/SearchBarInput.qml` - Cache-backed bridge calls land here via `load_language_labels_for_area` (line 60).
- `assets/qml/SuttaSearchWindow.qml` - Pre-flight binding rewrites (`webview_visible` at line 69; `app_settings_window.search_as_you_type` at lines 428, 2285, 3183; `app_settings_window.open_find_in_sutta_results` at line 1004; `gloss_tab.commonWordsDialog` at lines 69 and 1978; `models_dialog.auto_retry.checked` at lines 3334, 3351 — cross-deferred-component binding; `update_notification_dialog.show_*` calls at lines 2236, 2241, 2249, 2253; menu `.show()` triggers at lines 1490, 1986, 1999, 2007, 2028, 2035; `tab_list_dialog.visible` at lines 1166, 1172, 1296, 1774, 1844, 1861, 1942, 1959); add `searcher_ready` gate on every `handle_query` entry point (not just the button); wrap heavy tabs/dialogs (Loader for `Dialog`/`Popup`/`Item` roots, `Component` + `createObject(null)` for `ApplicationWindow` roots); instrument `Component.onCompleted`.
- `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml` - qmllint stub for `searcher_ready` qproperty.
- `bridges/build.rs` - Register any new QML files added during the split.

### Notes

- Re-bootstrap is required to verify the new index and the warm caches in `appdata.sqlite3`. Bootstrap is driven by the `cli` binary — see `cli/src/main.rs` for the exact subcommand; the runtime data dir is `/home/gambhiro/prods/apps/simsapa-ng-project/bootstrap-assets-resources/dist/simsapa-ng/`.
- Per project guidance: skip running tests between sub-tasks; only run them after the final sub-task of each top-level task. Do not run `make qml-test` unless explicitly asked. Use `make build -B` and `cd backend && cargo test`.

## Tasks

- [x] 1.0 Add `dict_words.language` covering index
  - [x] 1.1 Append `CREATE INDEX dict_words_language_idx ON dict_words(language);` to `backend/migrations/dictionaries/2025-05-03-143320_create-tables/up.sql`
  - [x] 1.2 Append `DROP INDEX IF EXISTS dict_words_language_idx;` to the matching `down.sql`
  - [x] 1.3 Re-bootstrap verified: `EXPLAIN QUERY PLAN SELECT DISTINCT language FROM dict_words` reports `SEARCH dict_words USING COVERING INDEX dict_words_language_idx (language>?)`
  - [x] 1.4 `make build -B` clean (test suite deferred to end-of-PRD per project convention)

- [x] 2.0 Extend `AppSettings` cache and add `AppData` accessors / refresh
  - [x] 2.1 Add `cached_sutta_languages: Vec<String>`, `cached_dict_languages: Vec<String>`, `cached_library_languages: Vec<String>` to `AppSettings` with `#[serde(default)]` and empty-`Vec` defaults; confirm a missing key round-trips cleanly via deserialize
  - [x] 2.2 Add `get_cached_sutta_languages()`, `get_cached_dict_languages()`, `get_cached_library_languages()` to `AppData` reading from `app_settings_cache` (no SQLite)
  - [x] 2.3 Add `refresh_language_caches(&self)` that runs `DbManager::get_sutta_languages`, `Dictionaries::get_distinct_languages`, `indexer::get_library_languages` and persists via the existing `persist_app_settings` path (mirror `refresh_dict_source_uid_caches` shape)
  - [x] 2.4 Add free-standing umbrella helper `refresh_all_dict_caches()` (not a method on `AppData`) that `std::thread::spawn`s a closure calling `get_app_data().refresh_dict_source_uid_caches()` then `get_app_data().refresh_language_caches()` — keeps mutation bridge call sites off the DISTINCT scan
  - [x] 2.5 `make build -B` clean (full test pass deferred to end-of-PRD)

- [x] 3.0 Switch bridge language-label methods to cache-backed reads
  - [x] 3.1 Update `SuttaBridge::get_sutta_language_labels` (sutta_bridge.rs:3528) to return `get_cached_sutta_languages()`
  - [x] 3.2 Update `SuttaBridge::get_dict_language_labels` (sutta_bridge.rs:3558) to return `get_cached_dict_languages()`
  - [x] 3.3 Update `SuttaBridge::get_library_language_labels` (sutta_bridge.rs:3539) to return `get_cached_library_languages()`
  - [x] 3.4 Leave the source-of-truth methods intact (`DbManager::get_sutta_languages`, `Dictionaries::get_distinct_languages`, `indexer::get_library_languages`) — they are now called only by `refresh_language_caches`
  - [x] 3.5 `make build -B` clean (full test pass deferred to end-of-PRD)

- [x] 4.0 Bootstrap-time cache warming
  - [x] 4.1 Create `cli/src/bootstrap/cache_warm.rs` exporting `warm_caches_into_appdata()` that reuses the runtime refresh helpers via `init_app_data()` + `get_app_data().refresh_dict_source_uid_caches()` + `refresh_language_caches()` — identical JSON shape to the runtime `persist_app_settings` path
  - [x] 4.2 Register the new module in `cli/src/bootstrap/mod.rs` and invoke `warm_caches_into_appdata()` at the end of bootstrap (after all FTS5 indexes and language imports), then re-create `appdata.tar.bz2` so the shipped archive contains the warmed `app_settings` row
  - [x] 4.3 Re-bootstrap verified: `app_settings` JSON contains non-empty values for all five `cached_*` fields (shipped=`["dpd","dppn"]`, commentary=56 ref_codes, sutta=`["en","pli"]`, dict=`["en","pli"]`, library=`["en"]`)
  - [x] 4.4 `make build -B` and `cd backend && cargo test` *(build verified clean; full test pass deferred to end of top-level task)*

- [ ] 5.0 Background fallback warming in `init_app_data()`
  - [ ] 5.1 Remove the inline `refresh_dict_source_uid_caches()` call from `AppData::new()` (app_data.rs:56–58)
  - [ ] 5.2 In `init_app_data()` (lib.rs:102), **after `APP_DATA.set(app_data)`**, check whether any of the five caches are empty
  - [ ] 5.3 If empty, `std::thread::spawn` a closure that calls `get_app_data().refresh_dict_source_uid_caches()` then `get_app_data().refresh_language_caches()` — `get_app_data()` is safe to call from the spawned thread because `APP_DATA` is now set
  - [ ] 5.4 Confirm `get_cached_*` returns empty `Vec` gracefully while warm-up is in flight (dropdown shows sentinel only)
  - [ ] 5.5 `make build -B` and `cd backend && cargo test`

- [ ] 6.0 Refresh hooks at mutation call sites (all spawn to background; never block GUI thread)
  - [ ] 6.1 Replace the 8 `refresh_dict_source_uid_caches()` call sites in `bridges/src/dictionary_manager.rs` (lines 343, 348, 360, 415, 418, 430, 517, 563) with `refresh_all_dict_caches()` — the umbrella helper itself spawns the work, so call sites stay synchronous-looking
  - [ ] 6.2 At the end of the success branch of `bridges/src/asset_manager.rs::import_suttas_lang_to_appdata` (success branch end is below the call at line 551), `std::thread::spawn` a closure that calls `get_app_data().refresh_language_caches()`
  - [ ] 6.3 At the end of the success branch of `backend/src/db/appdata.rs::remove_sutta_languages` (around line 1565, after the success log), `std::thread::spawn` a closure that calls `get_app_data().refresh_language_caches()`
  - [ ] 6.4 No runtime library-import path exists today (`cli/src/bootstrap/library_imports.rs` runs at bootstrap only) — confirm and document; no hook needed
  - [ ] 6.5 `make build -B` and `cd backend && cargo test`

- [ ] 7.0 Async fulltext searcher init
  - [ ] 7.1 In `init_app_data()` (lib.rs:111), wrap the `init_fulltext_searcher()` call in `std::thread::spawn`; keep `FULLTEXT_SEARCHER: RwLock<Option<…>>` semantics with `None` meaning "still warming" or "open failed"
  - [ ] 7.2 Add `#[qproperty(bool, searcher_ready)]` to `SuttaBridge` (sutta_bridge.rs:572 area) mirroring the existing `db_loaded` qproperty; emit the property write at the end of the background init (mirror sutta_bridge.rs:1375)
  - [ ] 7.3 Add the qmllint property stub for `searcher_ready` in `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml`
  - [ ] 7.4 Enumerate every `handle_query` entry point — grep `assets/qml/` for `handle_query(` to list all call sites (button click, keyboard shortcuts, drawer menu, programmatic triggers from result-click handlers, etc.) — and gate each on `SuttaBridge.db_loaded && SuttaBridge.searcher_ready`. Extend the existing `db_loaded` gating in `SearchBarInput.qml` (line 127, 135) with `searcher_ready`. Disabling the button alone is not sufficient; a shortcut-driven query must no-op (or queue) the same way
  - [ ] 7.5 Confirm the other `reinit_fulltext_searcher()` call sites stay synchronous (lib.rs:241; sutta_bridge.rs ~2790, ~2875, ~2993) — they run after the cold-start path with their own progress UI
  - [ ] 7.6 `make build -B` and `cd backend && cargo test`

- [ ] 8.0 QML pre-flight (no Loader/Component wrapping yet)
  - [ ] 8.1 Rewrite `webview_visible` at SuttaSearchWindow.qml:69 to use `?.item?.visible ?? false` / `dialog_visible` proxy form for every referenced dialog (`mobile_menu`, `about_dialog`, `models_dialog`, `anki_export_dialog`, `gloss_tab.commonWordsDialog`, `tab_list_dialog`, `database_validation_dialog`, `app_settings_window`, `info_dialog`)
  - [ ] 8.2 Re-route the three `app_settings_window.search_as_you_type` reads (lines 428, 2285, 3183) to `SuttaBridge.get_search_as_you_type()` or a root-level `property bool search_as_you_type` mirror; `AppSettingsWindow` becomes write-only for this value
  - [ ] 8.3 Re-route `app_settings_window.open_find_in_sutta_results` read at line 1004 — same pattern as 8.2; add `SuttaBridge.get_open_find_in_sutta_results()` if it does not already exist, or use a root mirror
  - [ ] 8.4 Re-route `models_dialog.auto_retry.checked` reads at lines 3334 and 3351 — **cross-deferred-component binding**, highest risk. Add `SuttaBridge.get_models_auto_retry()` (mirroring the existing `get_search_as_you_type` pattern) and have `ModelsDialog` write through to the setter; root reads the bridge value, never touches `models_dialog.auto_retry` directly
  - [ ] 8.5 Audit `tab_list_dialog` reads (lines 1166, 1172, 1296, 1774, 1844, 1861, 1942, 1959) — replace `.visible` reads with the upcoming `dialog_visible` proxy form and `.open()` calls with the upcoming loader-`open()` form (still pointing at the un-wrapped dialog for now; this is preparatory)
  - [ ] 8.6 Lift `commonWordsDialog` out of `GlossTab` to the SuttaSearchWindow root (preferred), or replace the two references (lines 69, 1978) with `gloss_tab_loader.item?.commonWordsDialog?…` form
  - [ ] 8.7 Catalogue the menu `.show()` triggers at lines 1490, 1986, 1999, 2007, 2028, 2035 and the `update_notification_dialog.show_*` calls at lines 2236, 2241, 2249, 2253 — leave them pointing at the un-wrapped instances for now; phase A/B will redirect each to the proxy `open_*()` / forwarded-method functions
  - [ ] 8.8 Verify the app behaves identically — no Loader/Component wrapping yet
  - [ ] 8.9 `make build -B` and `cd backend && cargo test`

- [ ] 9.0 QML Loader/Component split — phase A (highest impact)
  - [ ] 9.1 Wrap `GlossTab` in a `Loader { active: false }` activated on first gloss-tab selection in the right-side `StackLayout`
  - [ ] 9.2 Wrap `PromptsTab` in a `Loader { active: false }` activated on first prompts-tab selection
  - [ ] 9.3 Wrap `DictionaryTab` in a `Loader { active: false }` activated on first dictionary-tab selection
  - [ ] 9.4 Defer `AppSettingsWindow` (`ApplicationWindow` root) via `Component { AppSettingsWindow {} }` + `createObject(null)`; expose `readonly property bool app_settings_window_visible` on the root for the rewritten `webview_visible` binding; replace `app_settings_window.show()` (line 1490) with the `open_app_settings_window()` proxy
  - [ ] 9.5 Defer `UpdateNotificationDialog` (`ApplicationWindow` root) via the same `Component` + `createObject` pattern. **Multi-method forwarding:** expose `show_app_update(info)`, `show_db_update(info)`, `show_obsolete_warning()`, `show_no_updates()` proxy functions (each lazily creates the instance before forwarding) and redirect call sites at SuttaSearchWindow.qml:2236, 2241, 2249, 2253
  - [ ] 9.6 Instrument `Component.onCompleted` with `Logger.info` `Date.now()` deltas around the start/end of the handler so phase B can be measured against phase A
  - [ ] 9.7 `make build -B` and `cd backend && cargo test`

- [ ] 10.0 QML Loader/Component split — phase B (remaining)
  - [ ] 10.1 Defer `AboutDialog` (`ApplicationWindow` root) via `Component` + `createObject(null)`; expose `open_about_window()` proxy and `about_window_visible` mirror; redirect line 2035 `about_dialog.show()`
  - [ ] 10.2 Defer `ModelsDialog`, `AnkiExportDialog`, `SystemPromptsDialog` (all **`ApplicationWindow`** roots, not `Dialog`) via `Component` + `createObject(null)`; expose `open_*_window()` proxies; redirect lines 1986, 1999, 2007 `.show()` calls
  - [ ] 10.3 Defer `DatabaseValidationDialog`, `DhammaTextSourcesDialog` (also **`ApplicationWindow`** roots) via `Component` + `createObject(null)`; redirect line 2028 `dhamma_text_sources_dialog.show()` (and any `database_validation_dialog.show()` site). Wrap `TabListDialog` and `ColorThemeDialog` (true `Dialog` roots) in `Loader`s with `dialog_visible` proxy + `open()` function — finalise the tab_list_dialog redirects prepared in 8.5
  - [ ] 10.4 Wrap `TocTab` in a `Loader { active: false }` activated on first TOC-tab selection
  - [ ] 10.5 Sweep all remaining `<dialog_id>.open()` / `<dialog_id>.show()` / `<dialog_id>.visible` references and switch to the matching proxy form (loader for `Dialog`/`Popup`/`Item` roots; `createObject(null)` proxy for `ApplicationWindow` roots)
  - [ ] 10.6 `make build -B` and `cd backend && cargo test`

- [ ] 11.0 Manual mobile verification and acceptance check
  - [ ] 11.1 Build for Android and confirm `S → D` switch latency matches `S → S` / `S → L` (PRD §6.2)
  - [ ] 11.2 Confirm `apply_theme()` is reached without DB-scan work on the critical path; first-launch and subsequent-launch budgets per PRD §3
  - [ ] 11.3 Verify cache liveness: import / delete / rename user dict, download / remove sutta language — language dropdown reflects new state on next area switch (PRD §6.6, §6.7)
  - [ ] 11.4 Compare instrumented `Component.onCompleted` deltas before/after — confirm ≥30 % pre-`apply_theme` parse cost reduction (PRD §6.8)
  - [ ] 11.5 Trace SQLite (e.g. `sqlite3_trace` or `tracing` on the `db_query` span) during `S`/`D`/`L` button presses and confirm zero `SELECT DISTINCT language` calls (PRD §6.5)
  - [ ] 11.6 Smoke-test every cold-loaded dialog / tab (open at least once) to confirm reachability and no regression in first-open latency (PRD §6.9)
