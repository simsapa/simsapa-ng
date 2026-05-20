# Tasks: Dictionary Import UX Improvements & StarDict Resource Support

Based on `tasks/prd-dictionary-import-ux-and-stardict-resources.md`.

## Relevant Files

- `backend/src/stardict_parse.rs` - StarDict parsing/import core; `StardictImportProgress` enum, `import_stardict_as_new` (dictionaries row created at line 326; title/total available at line 349), `read_ifo_description`. New progress variant + `res/` capture hook in here.
- `backend/src/dictionary_manager_core.rs` - `import_user_zip` (holds `DICT_MGR_LOCK`), `delete_user_dictionary` (also takes `DICT_MGR_LOCK` — deadlock guard), `rename_user_dictionary`. `res/` capture + resource cleanup-on-delete go here.
- `bridges/src/dictionary_manager.rs` - CXX-Qt bridge: `importProgress`/`importFinished`/`importCancelled` signals, `import_zip`/`abort_import` (abort branch ~line 259 = empty-abort cleanup site), async label-check invokable + signal to add.
- `backend/src/app_data.rs` - `render_word_html_by_uid` (lines 389-490): the regex-rewrite branch (448-484) is the ACTUAL render path for user-dict words; where stored CSS/JS injection, `res/` image link-rewriting, and the DPD-only `assets/dpd-res/` guard go.
- `assets/qml/DictionariesWindow.qml` - Import progress UI (progress frame, `Importing "${op_label}"…` line ~413, Abort button ~458, `onImportProgress`/`onImportCancelled` handlers ~110-130). `start_import` already has `lang`.
- `assets/qml/DictionaryEditDialog.qml` - Rename dialog; per-keystroke `label_status()` call replaced with debounced async `check_label_status` + `labelStatusChecked` handler.
- `assets/qml/DictionaryImportDialog.qml` - Import dialog; had the same per-keystroke synchronous `label_status()` anti-pattern (not flagged in the PRD); given the identical debounced async treatment to fix typing lag in the label field.
- `assets/qml/SearchBarInput.qml` - Reference debounce `Timer` idiom (`search_timer.restart()`).
- `assets/qml/com/profoundlabs/simsapa/DictionaryManager.qml` - qmllint type definition; must mirror any new invokable/signal on the bridge.
- `bridges/src/api.rs` - Localhost API; `serve_book_resources` (`api.rs:792`, `/book_resources/<book_uid>/<path..>`, mime→ContentType map) is the template for the new `/dict_resources/<dict_id>/<path..>` route.
- `backend/src/html_content.rs` - `sutta_html_page` with `css_extra`/`js_extra` — only relevant as the FALLBACK path if mw-gd definitions turn out to be bare fragments (the DPPN path); not the primary user-dict render path.
- `backend/src/db/appdata.rs` - `get_book_resource` query is the template for the new `get_dict_resource` lookup.
- `backend/src/db/dictionaries_schema.rs` - Diesel schema for the dictionaries DB; add `dict_resources` table.
- `backend/src/db/dictionaries_models.rs` - Diesel models; add `DictResource`/`NewDictResource` (template: `BookResource` in `appdata_models.rs`).
- `backend/migrations/dictionaries/2026-05-20-120000_add_dict_resources/` - New migration creating the `dict_resources` table (FK `ON DELETE CASCADE`, index on `dictionary_id, resource_path`).
- `backend/src/db/mod.rs` - `DbManager::new` now runs pending dictionaries migrations on an EXISTING dict DB too (not only on first creation), so schema additions like `dict_resources` reach already-shipped DBs.
- `backend/src/db/dictionaries.rs` - Added `create_dict_resource`, `get_dict_resource`, `list_dict_resources`, `delete_dict_resources` query helpers.
- `backend/tests/stardict_import_resources.rs` - Integration tests: (1) synthetic StarDict with a `res/` folder — resources captured (CSS + nested image) and removed on delete; (2) render test — CSS injected inline, `<link>` neutralised, image `src` rewritten to the `/dict_resources/…` route, no `dpd-res` misrouting.
- `bridges/src/api.rs` - Added `serve_dict_resources` (`GET /dict_resources/<dict_id>/<path..>`) and registered it; serves user-dict resource blobs by dictionary id.
- `backend/src/app_data.rs` - `render_word_html_by_uid` regex branch now injects per-dictionary CSS/JS from `dict_resources`, neutralises the original res CSS/JS links, rewrites servable res references to the id-keyed API route, and gates the `assets/dpd-res/` link rewrite to DPD only.
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

- [x] 3.0 Debounced, non-blocking rename conflict check
  - [x] 3.1 In `bridges/src/dictionary_manager.rs`, add an async label-check invokable (e.g. `check_label_status(label)`) that runs on a worker thread and emits a new signal (e.g. `labelStatusChecked(label, status)`) carrying the same statuses as `label_status` (`invalid`/`taken_shipped`/`taken_user`/`available`), reusing `core_validate_label`, `is_label_taken_by_shipped`, and `list_dictionaries`.
  - [x] 3.2 Mirror the new invokable + signal in `assets/qml/com/profoundlabs/simsapa/DictionaryManager.qml` for qmllint.
  - [x] 3.3 In `assets/qml/DictionaryEditDialog.qml`, replace the synchronous `dict_manager.label_status(v)` call in `refresh_status()` with a debounce `Timer` modeled on `SearchBarInput.qml` (`restart()` on text change); on timeout, call the async `check_label_status`.
  - [x] 3.4 Keep no-DB fast-path checks immediate in QML (empty input, unchanged label) for instant feedback; defer only the DB-backed conflict check to the debounce.
  - [x] 3.5 Handle the `labelStatusChecked` signal in the dialog: update `label_status` (stale-guard against an outdated label if the text changed since the request) and keep the existing error labels (lines ~61-86) and OK-blocking behavior working.
  - [x] 3.6 Build with `make build -B` and confirm typing in the label field has no perceptible lag and warnings appear shortly after pausing.

- [x] 4.0 Capture and store StarDict `res/` resources during import
  - [x] 4.1 Create a Diesel migration under `backend/migrations/dictionaries/` (timestamp after `2026-04-28-120000_add_user_dict_columns`) adding a `dict_resources` table mirroring `book_resources`: `id` (PK), `dictionary_id` (Integer, FK), `resource_path` (Text), `mime_type` (Nullable Text), `content_data` (Nullable Binary); index/lookup by `dictionary_id` + `resource_path`. No extra columns on `dictionaries` — CSS/JS are stored as `dict_resources` rows too (distinguished by `mime_type`). (Migration `2026-05-20-120000_add_dict_resources`; FK is `ON DELETE CASCADE`.)
  - [x] 4.2 Add the `dict_resources` table to `backend/src/db/dictionaries_schema.rs` and `DictResource`/`NewDictResource` models in `backend/src/db/dictionaries_models.rs` (template: `BookResource` in `appdata_models.rs`). Add query helpers: insert resource, `get_dict_resource(dictionary_id, resource_path)` (template: `get_book_resource`, `appdata.rs:455`), `list_dict_resources`, and `delete_dict_resources` by `dictionary_id`.
  - [x] 4.3 First confirm in code that no `res/` handling exists (confirmed — no `res/` references in `dictionary_manager_core.rs`/`stardict_parse.rs`). Then in the import path (`backend/src/dictionary_manager_core.rs`, after a successful import, before the temp dir is dropped), detect a `res/` folder and walk it (`capture_stardict_resources`).
  - [x] 4.4 For every file in `res/`, insert a `dict_resources` row with the relative path, `content_data`, and a detected `mime_type` (by extension, `guess_resource_mime_type`). CSS/JS (`text/css` / `application/javascript`) and images alike — storage is uniform.
  - [x] 4.5 Do NOT rewrite the stored `definition_html` at import time. The new `dictionary_id` (from `outcome.dictionary_id`) is passed to `capture_stardict_resources` so rows are keyed correctly.
  - [x] 4.6 In `delete_user_dictionary`, delete the dictionary's `dict_resources` rows as part of deletion (within the existing lock, before `delete_dictionary_by_label`).
  - [x] 4.7 Inspected `mw-gd` definition data: it is a **full HTML document** (`<!DOCTYPE html><html><head>…<link href="mw.css" rel="stylesheet"></head><body>…`) → the regex-rewrite branch (task 5.2) is the correct render path, NOT the bare-fragment fallback. The only resource reference is `<link href="mw.css">` (a **bare filename, no `res/` prefix**) while the file lives at `res/mw.css`; so `resource_path` stored relative to `res/` = `mw.css`, matching the href. No `<script>` tags and no `res/`-prefixed image references in the sampled data.
  - [x] 4.8 Added backend test `tests/stardict_import_resources.rs` (synthetic StarDict with a `res/` folder — fast/deterministic, mirrors `mw-gd.zip`'s `res/` layout) asserting the CSS is stored as a `dict_resources` row (`text/css`) plus a nested image, and removed on delete. Build clean with `make build -B`; test passes.

- [x] 5.0 Serve and apply dictionary resources at render time
  - [x] 5.1 In `bridges/src/api.rs`, added `GET /dict_resources/<dict_id>/<path..>` route (`serve_dict_resources`) modeled on `serve_book_resources`: calls `get_dict_resource(dict_id, path)` and returns the blob with the stored `mime_type` (same mime→`ContentType` map; 404 when missing). Registered in the routes list.
  - [x] 5.2 In `render_word_html_by_uid`, the regex-rewrite branch now looks up the word's `dict_resources` (by `word.dictionary_id`), injects CSS (`text/css`) and JS (`application/javascript`/`text/javascript`) contents into the `</head>` `<style>`/`<script>` injection alongside `DICTIONARY_CSS`/`js_extra`, and neutralises the original `<link>`/`<script src>` referencing those resources so they aren't double-loaded.
  - [x] 5.3 In the same branch, non-CSS/JS resource references (`src="…"`, or `<link href>` to a servable resource) are rewritten to `<self.api_url>/dict_resources/<word.dictionary_id>/…` at render time. A `normalize_res` helper strips a leading `./` and optional `res/` prefix so bare hrefs (mw-gd's `mw.css`) and `res/…`-prefixed refs both resolve.
  - [x] 5.4 The `<link href>` → `{api_url}/assets/dpd-res/` rewrite is now gated on `word.dict_label == "dpd"`, so user-dict links are no longer misrouted to the DPD resource folder.
  - [x] 5.5 N/A — task 4.7 confirmed `mw-gd` definitions are full HTML docs with `<head>`, so the regex-rewrite path (5.2/5.3) applies; the bare-fragment `sutta_html_page` fallback is not needed.
  - [x] 5.6 Added render test `render_applies_dict_resources` in `tests/stardict_import_resources.rs`: asserts the CSS is injected inline, the original `<link>` is neutralised, the image `src` is rewritten to `/dict_resources/<id>/…`, and no `assets/dpd-res/` misrouting occurs. The HTTP route is a structural copy of the proven `serve_book_resources` and shares the `get_dict_resource` data path covered by 4.8. Build clean with `make build -B`.
  - [x] 5.7 Ran `cd backend && cargo test` (full suite): all dictionary import/resource/render tests pass; build clean. Two pre-existing fixture-drift failures remain (`test_dpd_lookup_generate_json`, `test_sutta_search_contains_match`) — both compare stored JSON against live-DB output and are unrelated to this feature (neither touches import, resources, the API, or `render_word_html_by_uid`).
