# Tasks: Expanded StarDict Import Options

Derived from [prd-stardict-import-multiple-options.md](./prd-stardict-import-multiple-options.md).
Stage numbering follows the PRD §7a.4 implementation stages. Each top-level task
leaves the app compiling and the relevant tests passing.

## Relevant Files

- `backend/src/dictionary_manager_core.rs` - Core import orchestration; add a directory-import path (`import_user_dir`) alongside `import_user_zip`, plus a discovery/probe helper that enumerates candidates per source kind. Houses `locate_stardict_dir` / `find_ifo_stem_in` reused by both.
- `backend/src/stardict_parse.rs` - `import_stardict_as_new` (shared by zip and dir paths); also edited in Stage 4 to store `Dictionary.language` regardless of `is_user_imported`.
- `bridges/src/dictionary_manager.rs` - Add `import_dir` and `scan_source` invokables (worker-threaded, signal/return JSON); reuse existing import progress/finished/failed/cancelled signals.
- `assets/qml/com/profoundlabs/simsapa/DictionaryManager.qml` - qmllint stubs for the new invokables (`import_dir`, `scan_source`).
- `assets/qml/DictionaryImportRow.qml` - **New** checklist-row component (checkbox, title/count, editable label/lang, conflict + lang warnings, per-row debounce + stale-guard).
- `assets/qml/DictionaryImportDialog.qml` - Rewritten as an `ApplicationWindow` with a 3-frame `StackLayout` (source selection / scanning / checklist); pickers and aggregation that emits `import_batch_requested`.
- `assets/qml/DictionariesWindow.qml` - Rewire to consume `import_batch_requested`; add the sequential batch driver and "Importing N of M"; remove the `replace_*` machinery.
- `bridges/build.rs` - Register the new QML files (`DictionaryImportRow.qml`) in `qml_files`.
- `cli/src/main.rs` - DPD stardict import (`import_stardict_dictionary`, line ~106) — ensure `pli` lands on the DPD dictionary row.
- `backend/src/db/dictionaries.rs` - `find_or_create_dpd_dictionary` (line ~382) — add `language: Some("pli")`.
- `cli/src/bootstrap/dppn.rs` - Coalesce DPPN `dict_words.language` to `"en"`.
- `backend/tests/` - Rust tests for directory import, discovery probe, and bootstrap language (e.g. `test_dictionary_import.rs`).

### Notes

- Per project memory: build with `make build -B`; run Rust tests with `cd backend && cargo test`; **do not** run `make qml-test` unless asked; only run tests after all sub-tasks of a top-level task are done.
- New QML components must be added to `qml_files` in `bridges/build.rs`.
- New bridge invokables need matching qmllint stubs in `assets/qml/com/profoundlabs/simsapa/DictionaryManager.qml`.
- Suitability test = presence of a `*.ifo` (existing `find_ifo_stem_in` / `locate_stardict_dir`); folder scans are non-recursive (direct children only).
- Discovery re-parses on each import (no caching). Language defaults to `pli`, editable; no `.ifo` language detection.
- Stage 4 changes require a manual re-bootstrap of the affected DBs to take effect.

## Tasks

- [ ] 1.0 Backend: directory-import core path, discovery probe, and `DictionaryManager` invokables (PRD Stage 0; §4.2, §4.5, req. 21–22)
  - [ ] 1.1 Add `import_user_dir(dir, label, lang, on_progress, cancel)` in `dictionary_manager_core.rs` that imports directly from an already-extracted StarDict directory: skip the unzip step, run `locate_stardict_dir` + `read_ifo_description` + `import_stardict_as_new`, and reuse the `DICT_MGR_LOCK`, abort flag, and `res/` capture (`capture_stardict_resources`) exactly as `import_user_zip`. Factor the shared tail (locate → describe → import → capture) so zip and dir paths don't diverge.
  - [ ] 1.2 Add a discovery/probe function (e.g. `scan_source(kind, path) -> Vec<CandidateMeta>`) in `dictionary_manager_core.rs` covering the four kinds: single zip, single dir, folder-of-zips (direct `*.zip` children), folder-of-dirs (direct sub-folders). Probe each candidate for a `.ifo` (skip non-StarDict silently) and return `{ title, entry_count, suggested_label, source_path, source_kind }`. Parse `.ifo` (`Ifo::new`) + index count (`stardict::no_cache(...).idx.items.len()`) only; do not iterate definitions. For zip candidates, probe without full extraction where feasible, else extract to a temp dir for the probe.
  - [ ] 1.3 Define a `CandidateMeta` struct (serde `Serialize`) and a JSON shape consumed by QML; default language is omitted (QML defaults to `pli`). Suggested label uses the existing `suggested_label_for_zip` rules (generalize to also accept a directory name).
  - [ ] 1.4 Add the `import_dir` invokable to `bridges/src/dictionary_manager.rs`, mirroring `import_zip` (worker thread, same `importProgress`/`importFinished`/`importFailed`/`importCancelled` signals, same quick-fail-returns-`"ok"` contract).
  - [ ] 1.5 Add the `scan_source` invokable to `bridges/src/dictionary_manager.rs`: run discovery on a worker thread and report via a new `scanFinished(items_json)` signal (and `scanFailed(message)`), or return JSON synchronously if the probe is cheap enough — choose worker-thread + signal to keep the UI responsive for large folders.
  - [ ] 1.6 Add qmllint stubs for `import_dir` and `scan_source` (and any new signals) to `assets/qml/com/profoundlabs/simsapa/DictionaryManager.qml`.
  - [ ] 1.7 Add Rust tests: directory import of an extracted StarDict dir produces the same rows as the equivalent zip; discovery over a mixed folder returns only valid candidates and skips non-StarDict entries; suggested-label sanitisation for a directory name.
  - [ ] 1.8 `make build -B` and `cd backend && cargo test` clean.

- [ ] 2.0 QML: `DictionaryImportRow.qml` checklist-row component with per-row label/lang validation (PRD Stage 1; §4.3 req. 11–12)
  - [ ] 2.1 Create `assets/qml/DictionaryImportRow.qml`: checkbox (`checked`), read-only title + entry count, editable label `TextField`, editable language `TextField`. Public props: `source_path`, `source_kind`, `title_text`, `entry_count`, `label` (alias), `lang` (alias), `checked` (alias), and a read-only `blocking` bool. Style consistent with `DictionaryListItem.qml`.
  - [ ] 2.2 Port the per-row validation: `valid_label_re` fast-path, a per-row debounce `Timer` (400 ms) calling `dict_manager.check_label_status`, and a `refresh_status()` that resolves empty/invalid immediately and defers the DB check otherwise.
  - [ ] 2.3 Port the lang warning: `refresh_lang_warning()` via `is_known_tokenizer_lang`, shown as the existing amber note.
  - [ ] 2.4 Stale-guard `labelStatusChecked` **per row**: the row only applies the result when the queried label equals this row's current label text (so concurrent rows don't cross-apply).
  - [ ] 2.5 Render the four conflict states (`invalid`, `taken_shipped`, `taken_user`, `available`); set `blocking = true` for `invalid`/`taken_shipped`/`taken_user` (req. 12 makes `taken_user` blocking in batch mode).
  - [ ] 2.6 Register `../assets/qml/DictionaryImportRow.qml` in `bridges/build.rs` `qml_files`.

- [ ] 3.0 QML: rewrite `DictionaryImportDialog.qml` as an `ApplicationWindow` (source selection → scan → checklist) (PRD Stage 2; §4.1, §4.2, §4.3)
  - [ ] 3.1 Convert the root from `Item` + `Dialog` to an `ApplicationWindow` mirroring `DictionariesWindow.qml` conventions (`flags: Qt.Dialog`, `ThemeHelper`, `is_mobile`/`pointSize`/`largePointSize`, top-bar margin, `DictionaryManager`). Keep `start()` as the public entry point (now shows the window at Idx 0).
  - [ ] 3.2 Idx 0 — Source selection frame: four radio options (default = single zip) + OK/Cancel. OK opens `FileDialog` (`*.zip`) for option 1, else `FolderDialog` for options 2–4. Preserve `file://` scheme stripping for both dialog URL types.
  - [ ] 3.3 On picker accept, switch to Idx 1 — Scanning (indeterminate progress) and call `dict_manager.scan_source(kind, path)`; on `scanFinished` populate the checklist model and switch to Idx 2; on empty result or `scanFailed`, surface an informative message and return to the list (reuse parent error/summary or a local message).
  - [ ] 3.4 Idx 2 — Checklist frame: `ScrollView` + `Repeater` over the scanned items using `DictionaryImportRow`. Single-item result shows one pre-checked row. Add "Select All" / "Clear Selection" buttons toggling every row's `checked`.
  - [ ] 3.5 Compute intra-batch duplicate-label conflicts client-side and surface them on the offending rows (extend `DictionaryImportRow.blocking` or an external flag set by the parent).
  - [ ] 3.6 OK enablement: disabled when zero rows checked, or any *checked* row is `blocking` (incl. intra-batch duplicates). OK builds an ordered `items_json` of checked rows (`{path, kind, label, lang}`) and emits the new `import_batch_requested(items_json)` signal, then closes/hides. Cancel returns to the list and emits `canceled()`.
  - [ ] 3.7 Replace the old `import_requested`/`replace_requested` signals with `import_batch_requested`; remove the details `Dialog` and the replace `MessageDialog`.
  - [ ] 3.8 If the QML element registration/name changes, update `bridges/build.rs` accordingly. `make build -B` clean.

- [ ] 4.0 QML: rewire `DictionariesWindow.qml` for sequential batch import and remove the replace path (PRD Stage 3; §4.4)
  - [ ] 4.1 Replace `onImport_requested`/`onReplace_requested` handlers with `onImport_batch_requested(items_json)`: parse the list into a queue and start the first item.
  - [ ] 4.2 Add a sequential batch driver: maintain `batch_queue`, `batch_index`, `batch_total`, plus accumulators (`batch_succeeded`, `batch_failed[]`, `batch_entries_total`). Start each item via `import_zip` or `import_dir` per its `kind`; advance to the next on `importFinished`/`importFailed`/`importCancelled`.
  - [ ] 4.3 Extend the Idx 2 import-progress frame to show "Importing N of M" alongside the existing per-dictionary progress.
  - [ ] 4.4 Per-item failure does not abort the batch — record it and continue (req. 19). Abort stops the current item and cancels the remaining queue (req. 18).
  - [ ] 4.5 On queue completion, route to the Idx 4 summary frame with a batch summary ("Imported K of M dictionaries — … entries total", listing failures). Keep the existing "Back to Dictionaries"/"Quit" buttons and the next-startup re-index note.
  - [ ] 4.6 Delete the dead replace machinery: `replace_pending`, `pending_zip_path`/`pending_label`/`pending_lang`, and the `onDeleteFinished` replace-chain branch (DictionariesWindow.qml ~:56–60, :169–175). Delete is now only triggered from the list, not from import.
  - [ ] 4.7 Confirm the user-visible regression (single-zip "replace existing" is gone) is acceptable before finalizing; `make build -B` clean.

- [ ] 5.0 Bootstrap: assign `language` to built-in DPD/bold/DPPN dictionaries and dict_words (PRD Stage 4; §4.6)
  - [ ] 5.1 In `backend/src/db/dictionaries.rs`, `find_or_create_dpd_dictionary` (~:382): add `language: Some("pli")` to the created `dpd` row.
  - [ ] 5.2 In `backend/src/stardict_parse.rs` (~:340), store `Dictionary.language` from the passed `lang` regardless of `is_user_imported`, so the built-in DPD stardict import gets `pli` on the dictionary row. Verify `dict_words.language` is already set from `lang` in `db_entries` (no re-implementation).
  - [ ] 5.3 In `cli/src/bootstrap/dppn.rs`, coalesce DPPN `dict_words.language` to `"en"` when the source value is NULL (the dictionary row already sets `Some("en")`).
  - [ ] 5.4 Confirm `bold_definitions` needs no change (`ensure_bold_definitions_parent_dictionary` already sets `Some("pli")`; it has no own `dict_words`).
  - [ ] 5.5 Add/adjust a test or bootstrap assertion verifying `language` for `dpd`/`bold_definitions`/`dppn` rows is `pli`/`pli`/`en` and the corresponding `dict_words.language` matches. Document the manual re-bootstrap requirement in the task notes / commit message.
  - [ ] 5.6 `make build -B` and `cd backend && cargo test` clean.
