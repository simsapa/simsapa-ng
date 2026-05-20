# Tasks: Dictionary Import UX Improvements & StarDict Resource Support

Based on `tasks/prd-dictionary-import-ux-and-stardict-resources.md`.

## Relevant Files

- `backend/src/stardict_parse.rs` - StarDict parsing/import core; `StardictImportProgress` enum, `import_stardict_as_new` (dictionaries row created at line 326; title/total available at line 349), `read_ifo_description`. New progress variant + `res/` capture hook in here.
- `backend/src/dictionary_manager_core.rs` - `import_user_zip` (holds `DICT_MGR_LOCK`), `delete_user_dictionary` (also takes `DICT_MGR_LOCK` — deadlock guard), `rename_user_dictionary`. `res/` capture + resource cleanup-on-delete go here.
- `bridges/src/dictionary_manager.rs` - CXX-Qt bridge: `importProgress`/`importFinished`/`importCancelled` signals, `import_zip`/`abort_import` (abort branch ~line 259 = empty-abort cleanup site), async label-check invokable + signal to add.
- `backend/src/app_data.rs` - `render_word_html_by_uid` (lines 389-490): the regex-rewrite branch (448-484) is the ACTUAL render path for user-dict words; where stored CSS/JS injection, `res/` image link-rewriting, and the DPD-only `assets/dpd-res/` guard go.
- `assets/qml/DictionariesWindow.qml` - Import progress UI (progress frame, `Importing "${op_label}"…` line ~413, Abort button ~458, `onImportProgress`/`onImportCancelled` handlers ~110-130). `start_import` already has `lang`.
- `assets/qml/DictionaryEditDialog.qml` - Rename dialog; per-keystroke `label_status()` call (line 36) to replace with debounced async check.
- `assets/qml/SearchBarInput.qml` - Reference debounce `Timer` idiom (`search_timer.restart()`).
- `assets/qml/com/profoundlabs/simsapa/DictionaryManager.qml` - qmllint type definition; must mirror any new invokable/signal on the bridge.
- `bridges/src/api.rs` - Localhost API; `serve_book_resources` (`api.rs:792`, `/book_resources/<book_uid>/<path..>`, mime→ContentType map) is the template for the new `/dict_resources/<dict_id>/<path..>` route.
- `backend/src/html_content.rs` - `sutta_html_page` with `css_extra`/`js_extra` — only relevant as the FALLBACK path if mw-gd definitions turn out to be bare fragments (the DPPN path); not the primary user-dict render path.
- `backend/src/db/appdata.rs` - `get_book_resource` query is the template for the new `get_dict_resource` lookup.
- `backend/src/db/dictionaries_schema.rs` - Diesel schema for the dictionaries DB; add `dict_resources` table.
- `backend/src/db/dictionaries_models.rs` - Diesel models; add `DictResource`/`NewDictResource` (template: `BookResource` in `appdata_models.rs`).
- `backend/migrations/dictionaries/` - New timestamped migration creating `dict_resources` (latest existing: `2026-04-28-120000_add_user_dict_columns`).
- `cli/src/main.rs`, `cli/src/bootstrap/mod.rs` - CLI/bootstrap `match` sites on `StardictImportProgress`; new `Identified` arm added so they stay exhaustive.
- `bridges/build.rs` - Register any new QML file in `qml_files` (none expected unless a new component is split out).

### Notes

- Per `CLAUDE.md`: new `DictionaryManager` invokables/signals MUST be mirrored in `assets/qml/com/profoundlabs/simsapa/DictionaryManager.qml` for qmllint; new QML files MUST be added to `qml_files` in `bridges/build.rs`.
- Build with `make build -B` (not direct cmake). Skip QML tests unless asked. Only run tests after all sub-tasks of a top-level task are done.
- Rust tests: `cd backend && cargo test`. Integration tests may use the real appdata DB at the path in `CLAUDE.md` (do not gate behind `#[ignore]`).
- The `dict_resources` table is not FTS-indexed, so the FTS5-rowid guidance does not apply; a plain table is fine.
- Test fixture: `bootstrap-assets-resources/stardict-imports/mw-gd.zip` (extracted `…/mw-gd/`) has a `res/` folder with a CSS file.

## Tasks

- [x] 1.0 Import progress dialog shows detailed dictionary identity
  - [x] 1.1 In `backend/src/stardict_parse.rs`, add a progress variant `StardictImportProgress::Identified { title, total }`, emitted at line ~349 (right after `stardict::no_cache` loads the dict, where the existing `Importing … total entries` log line is) carrying `ifo.bookname` and `dict.idx.items.len()`. Use the raw index count (matches the log), NOT the later `words_to_insert.len()`. Lang is not needed here — QML already has it.
  - [x] 1.2 Update `stardict_progress_to_signal` in `bridges/src/dictionary_manager.rs` to map the new variant into the `importProgress` signal. Prefer reusing the existing `(stage, done, total)` signature — format the title into `stage` (e.g. `Identified:<title>`) and pass the entry count as `total` — to avoid adding a new signal; keep the other variants working.
  - [x] 1.3 In `assets/qml/DictionariesWindow.qml`, store the title + total from `onImportProgress` (and the `lang` already passed to `start_import`) and display `Importing <title> (<lang>), <N> total entries…` in the progress frame (replacing the `Importing "${root.op_label}"…` text at ~line 413). Fall back to the bare-label text during Extracting/Parsing before the detail arrives.
  - [x] 1.4 If a new signal signature was added in 1.2 (instead of reusing `importProgress`), mirror it in `assets/qml/com/profoundlabs/simsapa/DictionaryManager.qml` for qmllint. (N/A — reused the existing `importProgress` signature, no new signal.)
  - [x] 1.5 Build with `make build -B`.

- [x] 2.0 Abort UX: immediate "Aborting…" feedback + empty-dictionary cleanup
  - [x] 2.1 In `assets/qml/DictionariesWindow.qml`, on the Abort button `onClicked` (~line 458): before/alongside calling `dict_manager.abort_import()`, immediately set an "Aborting…" UI state — set the stage text to `Aborting…`, disable the Abort button, and switch the bar to indeterminate.
  - [x] 2.2 In the abort branch of `import_zip` in `bridges/src/dictionary_manager.rs` (~line 259), when `outcome.cancelled && outcome.inserted == 0`, call `dictionary_manager_core::delete_user_dictionary(outcome.dictionary_id)` then `get_app_data().refresh_dict_source_uid_caches()`, so no 0-entry row remains. DEADLOCK GUARD: do this here (after `import_user_zip` has returned and released `DICT_MGR_LOCK`), NOT inside `import_user_zip` — `delete_user_dictionary` re-acquires the same `try_lock` and would return BUSY. The `dictionaries` row already exists (created at `stardict_parse.rs:326`), so `outcome.dictionary_id` is valid.
  - [x] 2.3 Preserve existing behavior when aborted with `inserted >= 1`: leave partial rows for the next-startup reconcile; do not delete.
  - [x] 2.4 Make the `importCancelled` signal/message distinguish the two cases — "aborted, nothing imported, removed" vs "aborted, N entries kept" — and update the abort-summary frame text in `DictionariesWindow.qml` (~lines 528-549) accordingly so it never claims a partial import was kept when 0 entries were inserted.
  - [x] 2.5 Confirm a *normal* (non-aborted) completed import with 0 entries is NOT deleted (work-in-progress dictionaries are kept).
  - [x] 2.6 Add/extend a backend test for the empty-abort cleanup path (cancel before any insert ⇒ no `dictionaries` row left), then build with `make build -B`.

- [ ] 3.0 Debounced, non-blocking rename conflict check
  - [ ] 3.1 In `bridges/src/dictionary_manager.rs`, add an async label-check invokable (e.g. `check_label_status(label)`) that runs on a worker thread and emits a new signal (e.g. `labelStatusChecked(label, status)`) carrying the same statuses as `label_status` (`invalid`/`taken_shipped`/`taken_user`/`available`), reusing `core_validate_label`, `is_label_taken_by_shipped`, and `list_dictionaries`.
  - [ ] 3.2 Mirror the new invokable + signal in `assets/qml/com/profoundlabs/simsapa/DictionaryManager.qml` for qmllint.
  - [ ] 3.3 In `assets/qml/DictionaryEditDialog.qml`, replace the synchronous `dict_manager.label_status(v)` call in `refresh_status()` with a debounce `Timer` modeled on `SearchBarInput.qml` (`restart()` on text change); on timeout, call the async `check_label_status`.
  - [ ] 3.4 Keep no-DB fast-path checks immediate in QML (empty input, unchanged label) for instant feedback; defer only the DB-backed conflict check to the debounce.
  - [ ] 3.5 Handle the `labelStatusChecked` signal in the dialog: update `label_status` (stale-guard against an outdated label if the text changed since the request) and keep the existing error labels (lines ~61-86) and OK-blocking behavior working.
  - [ ] 3.6 Build with `make build -B` and confirm typing in the label field has no perceptible lag and warnings appear shortly after pausing.

- [ ] 4.0 Capture and store StarDict `res/` resources during import
  - [ ] 4.1 Create a Diesel migration under `backend/migrations/dictionaries/` (timestamp after `2026-04-28-120000_add_user_dict_columns`) adding a `dict_resources` table mirroring `book_resources`: `id` (PK), `dictionary_id` (Integer, FK), `resource_path` (Text), `mime_type` (Nullable Text), `content_data` (Nullable Binary); index/lookup by `dictionary_id` + `resource_path`. No extra columns on `dictionaries` — CSS/JS are stored as `dict_resources` rows too (distinguished by `mime_type`).
  - [ ] 4.2 Add the `dict_resources` table to `backend/src/db/dictionaries_schema.rs` and `DictResource`/`NewDictResource` models in `backend/src/db/dictionaries_models.rs` (template: `BookResource` in `appdata_models.rs`). Add query helpers: insert resource, `get_dict_resource(dictionary_id, resource_path)` (template: `get_book_resource`, `appdata.rs:455`), list resources for a dictionary, and delete by `dictionary_id`.
  - [ ] 4.3 First confirm in code that no `res/` handling exists (it doesn't). Then in the import path (`backend/src/dictionary_manager_core.rs`, where the zip is already extracted to `extract_dir`/`unzipped_dir`, before the temp dir is dropped at line 190), detect a `res/` folder and walk it.
  - [ ] 4.4 For every file in `res/`, insert a `dict_resources` row with the relative path, `content_data`, and a detected `mime_type` (by extension). This includes CSS/JS (stored as `text/css` / `application/javascript`) and images alike — storage is uniform; the difference is only in how they're applied at render (task 5.x).
  - [ ] 4.5 Do NOT rewrite the stored `definition_html` at import time (URLs are port-dependent and rewritten at render — task 5.3). Pass the new `dictionary_id` to the resource-capture step so rows are keyed correctly.
  - [ ] 4.6 In `delete_user_dictionary` (`backend/src/dictionary_manager_core.rs`), delete the dictionary's `dict_resources` rows as part of deletion (do it within the existing lock, before/after `delete_dictionary_by_label`).
  - [ ] 4.7 Inspect a stored `definition_html` row for `mw-gd` after import (query the dictionaries DB) to confirm: (a) whether it's a full HTML doc with `<head>` or a bare fragment, and (b) the exact syntax of CSS/JS/image references to `res/`. This determines the render approach in 5.2/5.3.
  - [ ] 4.8 Add a backend test importing `mw-gd.zip`: assert the `res/` CSS file is stored as a `dict_resources` row for the new dictionary id, then build with `make build -B`.

- [ ] 5.0 Serve and apply dictionary resources at render time
  - [ ] 5.1 In `bridges/src/api.rs`, add a `GET /dict_resources/<dict_id>/<path..>` route modeled on `serve_book_resources` (`api.rs:792`): call `get_dict_resource(dict_id, path)` and return the blob with the stored `mime_type` (reuse the existing mime→`ContentType` mapping; 404 when missing). Register the route in the routes list.
  - [ ] 5.2 In `backend/src/app_data.rs::render_word_html_by_uid`, in the regex-rewrite branch (lines 448-484 — the ACTUAL user-dict path, not `sutta_html_page`), look up the word's dictionary CSS/JS rows (by `word.dictionary_id`) and inject their contents into the existing `</head>` `<style>`/`<script>` injection alongside `DICTIONARY_CSS`. Neutralise the original `<link href="res/…css">` / `<script src="res/…js">` so they aren't double-loaded.
  - [ ] 5.3 In the same branch, rewrite non-CSS/JS `res/…` references (e.g. `src="res/foo.png"`) to `<self.api_url>/dict_resources/<word.dictionary_id>/…` at render time (never baked at import). 
  - [ ] 5.4 Make the existing unconditional `<link href>` → `{api_url}/assets/dpd-res/` rewrite (`app_data.rs:473-479`) DPD-only (gate on `word.dict_label` being dpd), so user-dict links are no longer misrouted to the DPD resource folder.
  - [ ] 5.5 If task 4.7 found that `mw-gd` definitions are bare fragments (no `<head>`), instead route them through `sutta_html_page` with the dictionary's CSS/JS passed via `css_extra`/`js_extra` (the DPPN path), applying 5.3's image rewriting to the fragment first.
  - [ ] 5.6 Verify a rewritten `/dict_resources/<id>/…` request returns HTTP 200 with the correct content type, and that an imported `mw-gd` definition renders with its bundled CSS applied. Build with `make build -B`.
  - [ ] 5.7 Run `cd backend && cargo test` for the full backend suite once 4.0 and 5.0 are complete; confirm a clean build (ignore unrelated pre-existing failures).
