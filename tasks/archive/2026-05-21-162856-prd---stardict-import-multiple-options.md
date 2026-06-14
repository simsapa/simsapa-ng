# PRD: Expanded StarDict Import Options (single/multiple, zip/extracted)

## 1. Introduction/Overview

Today the "Import StarDict..." button in `DictionariesWindow.qml` only handles a
**single dictionary `.zip` archive**. Users frequently have several dictionaries
to import at once (e.g. the whole
[other-dictionaries](https://github.com/digitalpalidictionary/other-dictionaries/releases/)
release), or have already-extracted StarDict folders.

This feature expands the import entry point to offer four source modes, lets the
user import many dictionaries in one operation, and remodels
`DictionaryImportDialog.qml` into a checklist UI (one row per discovered
dictionary) so the user reviews title / word count / detected language / label
and chooses what to import before committing.

Separately, it fixes a data-quality gap: the built-in **DPD** and
**bold_definitions** dictionaries (and their `dict_words`) are missing a
`language` value, and **DPPN** `dict_words` should carry `en`. This is corrected
in the CLI bootstrap.

## 2. Goals

1. Let the user choose among four import sources via radio options:
   single `.zip`, single extracted folder, folder of `.zip` archives, folder of
   extracted dictionary folders.
2. Discover all valid StarDict dictionaries in the chosen source, skipping
   non-StarDict files/folders without error.
3. Present a single checklist dialog (used for 1 item or many) showing per-item
   title, word count, detected language (editable), and label (editable), with
   inline conflict validation and Select-All / Clear-Selection controls.
4. Import all selected dictionaries sequentially, reusing the existing
   import-progress frame with an added "Importing N of M" indication.
5. Remodel `DictionaryImportDialog.qml` to an `ApplicationWindow`-based
   structure consistent with `DictionariesWindow.qml`.
6. Assign `language` to the built-in DPD (`pli`), bold_definitions (`pli`), and
   DPPN (`en`) dictionaries and their `dict_words` rows during bootstrap.

## 3. User Stories

- As a user with a folder of downloaded `.zip` dictionaries, I want to point the
  app at the folder and pick which ones to import, so I don't have to import
  each one by hand.
- As a user who already extracted a dictionary, I want to import it from its
  folder without re-zipping it.
- As a user, I want to see each dictionary's title, entry count, and language
  before importing, and correct the label or language if needed, so I import the
  right data under sensible names.
- As a user, I want colliding labels flagged clearly so I can fix them before the
  import runs, rather than failing midway.
- As a maintainer, I want the built-in dictionaries to carry correct language
  values so language-scoped search/indexing behaves consistently.

## 4. Functional Requirements

### 4.1 Source-selection dialog

1. The "Import StarDict..." button must open a modal dialog presenting four
   radio options (default = first):
   1. Single dictionary `.zip` archive
   2. Single folder of an already-extracted dictionary
   3. Folder of multiple dictionary `.zip` archives
   4. Folder of multiple extracted dictionary folders
2. The dialog must have OK and Cancel. OK opens the appropriate native picker:
   - Option 1 → `FileDialog` filtered to `*.zip`.
   - Options 2, 3, 4 → `FolderDialog` (single folder selection).
3. Cancel (in either the radio dialog or the picker) returns to the list with no
   change.

### 4.2 Discovery & suitability test

4. After the picker returns, the backend must enumerate candidate dictionaries
   and test each for StarDict suitability (presence of a `*.ifo` file, matching
   the existing `find_ifo_stem_in` logic, including one-level-deep lookup for the
   zip/extracted-folder case):
   - Option 1: the single `.zip` (one candidate).
   - Option 2: the single extracted folder (one candidate).
   - Option 3: every `*.zip` file directly in the chosen folder (non-recursive);
     each is probed for a `.ifo` entry inside.
   - Option 4: every **direct** sub-folder of the chosen folder (non-recursive,
     no exhaustive descent); each probed for a `.ifo`.
5. Non-StarDict files and folders must be silently skipped (not errors). Other
   `.zip` files / folders may legitimately be present.
6. For each valid candidate the backend must return enough metadata to populate
   the checklist **without** importing: dictionary title (`.ifo` bookname), raw
   entry/word count, a suggested label (sanitised from the zip/folder name via
   the existing `suggested_label_for_zip` rules), and the source path. This probe
   must be reasonably cheap (parse `.ifo` + index count; avoid parsing every
   definition). The probe re-parses on each invocation — no caching of the parsed
   `.ifo`/index is required (see Open Questions, resolved). Discovery runs on a
   worker thread with an indeterminate "Scanning…" progress indication if it is
   not instant.
7. Language default: each candidate defaults to `pli`, matching the existing
   single-zip behaviour (`DictionaryImportDialog.qml` sets `lang_input_text =
   "pli"`). No `.ifo` language detection is performed. The value is editable per
   row.
8. If discovery finds zero valid dictionaries, show an informative message
   (reusing the error/summary frame) and return to the list.

### 4.3 Checklist dialog (remodeled `DictionaryImportDialog.qml`)

9. `DictionaryImportDialog.qml` must be remodeled to an `ApplicationWindow`-based
   structure (mirroring `DictionariesWindow.qml`'s conventions: `is_mobile`,
   `pointSize`, `ThemeHelper`, top-bar margin), replacing the current `Item` +
   `Dialog` structure.
10. The dialog must show one row per discovered dictionary. **All four source
    modes funnel through this same checklist** — a single dictionary shows one
    pre-checked row (no separate single-item layout).
11. Each row must display and allow editing of:
    - A checkbox (selected = will be imported). Single-item: pre-checked.
    - Title (read-only) and word/entry count (read-only).
    - Editable label field (defaults to the suggested label).
    - Editable language field (defaults to `pli`), with the same "unknown
      tokenizer language" warning as today (`is_known_tokenizer_lang`).
12. Inline conflict validation per row, reusing the existing `label_status`
    semantics (`invalid` / `taken_shipped` / `taken_user` / `available`):
    - `invalid` and `taken_shipped` → blocking error (red), as today.
    - `taken_user` → **blocking** in batch mode (amber→treated as blocking): the
      user must edit the label or uncheck the row. There is **no** silent replace
      in this flow. (Single-item replace is out of scope for this batch UI; see
      Non-Goals.)
    - Duplicate labels **within** the current selection must also be flagged as
      blocking.
13. The dialog must have "Select All" and "Clear Selection" buttons affecting all
    rows' checkboxes.
14. OK (import) must be disabled while any **checked** row has a blocking
    conflict, or while zero rows are checked. Unchecked rows with conflicts do
    not block.
15. OK emits an import request carrying the ordered list of selected items
    (each: source path, source kind = zip|dir, label, language). Cancel returns
    to the list.

### 4.4 Sequential batch import

16. On OK, `DictionariesWindow.qml` must import the selected items **one at a
    time**, reusing the existing import-progress frame (Idx 2). The frame must
    additionally show "Importing N of M" (current item index / total selected).
17. Each item reuses the existing per-dictionary progress signals
    (`importProgress`/`importFinished`/`importFailed`/`importCancelled`).
18. **Abort** during the batch stops the current item per existing semantics and
    cancels the remaining queue (does not start further items).
19. Per-item failure: the batch records the failure and continues to the next
    item (a single bad dictionary must not abort the whole batch). A final
    summary reports how many succeeded / failed / were skipped.
20. The final summary frame (Idx 4) must report the batch outcome, e.g.
    "Imported K of M dictionaries (… entries total)", listing any failures.
    The existing "Back to Dictionaries" / "Quit" buttons and the next-startup
    re-index note are retained.

### 4.5 Backend: import from an extracted directory

21. Add a core import path that imports directly from an **already-extracted**
    StarDict directory (Options 2 and 4), skipping the unzip step and reusing
    `import_stardict_as_new` (via `locate_stardict_dir` + `read_ifo_description`).
    This is a new function alongside `import_user_zip` in
    `dictionary_manager_core.rs`, and a corresponding `DictionaryManager`
    invokable (e.g. `import_dir`). The shared serialisation lock
    (`DICT_MGR_LOCK`), abort flag, and resource (`res/`) capture behaviour must
    match the zip path.
22. Add backend discovery/probe invokables on `DictionaryManager` that, given a
    source kind + path, return the candidate metadata JSON described in req. 6
    (title, count, suggested label, detected language, source path, source kind).
    These must not mutate the DB.

### 4.6 Bootstrap language assignment

23. The CLI bootstrap must set `language` on the built-in dictionary rows **and**
    their `dict_words`:
    - `dpd` dictionary `language = "pli"`; its `dict_words.language = "pli"`.
    - `bold_definitions` dictionary `language = "pli"` — **already set** by
      `ensure_bold_definitions_parent_dictionary` (dictionaries.rs:371); no change
      needed, and it has no `dict_words` of its own (it is a category backed by
      `dpd.sqlite3::bold_definitions`).
    - `dppn` dictionary `language = "en"` (already set); ensure its
      `dict_words.language = "en"`.
24. Fix `find_or_create_dpd_dictionary` (`backend/src/db/dictionaries.rs:382`) so
    the created `dpd` row carries `language: Some("pli")` instead of the current
    `..Default::default()` (NULL).
25. The CLI DPD stardict import (`cli/src/main.rs:106`,
    `import_stardict_dictionary`) currently passes `is_user_imported=false`, and
    `import_stardict_as_new` only stores `Dictionary.language` when
    `is_user_imported` is true. Adjust so built-in DPD/bold get `pli` on the
    dictionary row without otherwise changing the `is_user_imported` semantics
    (e.g. always set `language` from the passed `lang`, regardless of the
    `is_user_imported` flag). `dict_words.language` is already set from `lang`
    in `db_entries`, so verify rather than re-implement.
26. For DPPN, ensure `dict_words.language` is `"en"` even when the source
    `dict_words.language` is NULL (`cli/src/bootstrap/dppn.rs` copies
    `w.language.clone()` — coalesce to `"en"`).
27. These changes require a **manual re-bootstrap** of the affected DBs to take
    effect (consistent with the project's no-migration-for-built-in-data
    convention). Document this in the task notes.

## 5. Non-Goals (Out of Scope)

- Recursive/exhaustive descent into nested folders (Option 4 is direct
  sub-folders only).
- Per-item "replace existing user dictionary" within the batch flow. Colliding
  user labels are blocked and must be renamed/unchecked. (The existing
  single-zip replace flow is being superseded by the checklist; replace is not
  carried into batch mode.)
- Importing non-StarDict formats.
- Parallel/concurrent imports — imports remain serialised by `DICT_MGR_LOCK`.
- Changing the startup re-indexing/reconcile pass (it already picks up
  newly-imported dictionaries with `indexed_at = NULL`).

## 6. Design Considerations

- Mirror `DictionariesWindow.qml` conventions in the remodeled dialog:
  `ApplicationWindow`, `flags: Qt.Dialog`, `ThemeHelper`, `is_mobile`/`pointSize`
  sizing, top-bar margin.
- Reuse `DictionaryListItem.qml` styling cues where practical for the checklist
  rows (title + label + language + count), adding a checkbox and editable
  label/language fields.
- New QML files (e.g. a source-selection dialog, a checklist row component) must
  be added to the `qml_files` list in `bridges/build.rs` per project convention.
- New `DictionaryManager` invokables need matching stubs in
  `assets/qml/com/profoundlabs/simsapa/DictionaryManager.qml` for `qmllint`.
- Reuse the existing debounce idiom for per-row label conflict checks; avoid
  firing a DB check per keystroke across many rows simultaneously (debounce per
  row, or check on focus-out).

## 7. Technical Considerations

- Existing single-zip path: `DictionaryImportDialog.qml` → `FileDialog` → details
  `Dialog` → `import_requested` → `DictionariesWindow.start_import` →
  `dict_manager.import_zip` → `dictionary_manager_core::import_user_zip` →
  `import_stardict_as_new`. The new design replaces the per-item details dialog
  with the checklist and adds a discovery step before it.
- Suitability test: presence of `*.ifo` via the existing `find_ifo_stem_in` /
  `locate_stardict_dir` helpers in `dictionary_manager_core.rs`.
- The discovery probe should parse the `.ifo` (`Ifo::new`) and read the index
  item count (`stardict::no_cache(...).idx.items.len()`), matching the
  `Identified` progress data, without iterating definitions.
- Keep all DB mutations on worker threads and serialised by `DICT_MGR_LOCK`; the
  batch loop in QML drives one bridge call per item and waits for its
  finished/failed signal before starting the next.
- Bootstrap edits touch: `backend/src/db/dictionaries.rs:382`
  (`find_or_create_dpd_dictionary` — add `language: Some("pli")` to the DPD row),
  `backend/src/stardict_parse.rs:340` (store `Dictionary.language` from `lang`
  regardless of `is_user_imported`), and `cli/src/bootstrap/dppn.rs` (coalesce
  DPPN `dict_words.language` to `"en"`). The `bold_definitions` dictionary row
  already sets `language: Some("pli")` in
  `ensure_bold_definitions_parent_dictionary` (dictionaries.rs:371) — no change.

## 7a. Implementation Stages (DictionaryImportDialog refactor)

Rewriting `DictionaryImportDialog.qml` from an `Item` + `Dialog` into an
`ApplicationWindow` is a significant refactor. This section records the
decisions and staging so the work can be picked up incrementally.

### 7a.1 Processing logic that MUST be preserved

The current dialog wraps a zip `FileDialog`, a details `Dialog`, and a replace
`MessageDialog`. The reusable logic to carry over (file/line refs are the
current `DictionaryImportDialog.qml`):

| Logic | Current location | Preserved as |
|---|---|---|
| `start()` entry point | :48 | Entry that opens the **source-selection** frame (not the zip picker directly) |
| `file://` scheme stripping | :58–62 | Same, applied to both `FileDialog` and `FolderDialog` URLs |
| Default label = `suggested_label_for_zip(path)` | :64 | Per-row default label, from the backend probe metadata |
| Default lang = `"pli"` | :65 | Per-row default (no `.ifo` detection) |
| `valid_label_re` fast-path (`/^[A-Za-z0-9_-]+$/`) | :46 | Per-row instant validation before the debounced DB check |
| Debounced `check_label_status` (400 ms) | :36–41 | **Per-row** debounce — must not fire one DB check per keystroke × N rows |
| Stale-guard on `labelStatusChecked` (label === current input) | :26–30 | Per-row guard, keyed so the right row receives its result |
| `refresh_status()` empty/invalid → immediate `invalid`, else debounce | :89–99 | Per-row |
| `refresh_lang_warning()` via `is_known_tokenizer_lang` | :101–104 | Per-row lang warning |
| Accept gating: `invalid`/`taken_shipped` block; `available` proceed | :179–192 | OK-disabled logic across all *checked* rows |
| `taken_user` → replace confirm → `replace_requested` | :185–231 | **Removed** (see §7a.2) |

### 7a.2 Integration-contract changes with `DictionariesWindow.qml`

Current contract (`DictionariesWindow.qml:229–258`): calls `import_dialog.start()`;
consumes `onImport_requested(zip_path, label, lang)` → `start_import(...)`;
consumes `onReplace_requested(...)` → delete-then-import chain (:237–253);
`onCanceled` → no-op.

Changes:

1. **`replace_requested` and the delete-then-import chain are removed.** Since all
   four modes (including single zip) funnel through the checklist and `taken_user`
   is a **blocking** state with no batch replace (Non-Goals), the entire replace
   path goes away in both files. The `replace_pending` / `pending_*` state in
   `DictionariesWindow.qml` (:56–60, :169–175, :241–252) is deleted.
   **User-visible regression — confirm before implementing:** importing over an
   existing user dictionary now requires renaming or deleting it first.
2. **New signal** replaces `import_requested`/`replace_requested`: a single
   `import_batch_requested(items_json)` carrying the ordered list of
   `{path, kind: "zip"|"dir", label, lang}` for *checked* rows.
3. DictionariesWindow's existing **Idx 2 progress frame and Idx 4 summary frame
   are reused** by a new sequential batch driver; only the "Importing N of M"
   text and the batch-summary aggregation are added.

### 7a.3 Structural decision

`DictionaryImportDialog.qml` becomes a **standalone `ApplicationWindow`** (own
`flags: Qt.Dialog`, `ThemeHelper`, `is_mobile`/`pointSize`, top-bar margin),
internally a `StackLayout` of frames:

- **Idx 0 — Source selection:** four radio options + OK/Cancel.
- **Idx 1 — Scanning:** indeterminate progress while the discovery probe runs on
  a worker thread.
- **Idx 2 — Checklist:** `ScrollView` + `Repeater` of a new
  `DictionaryImportRow.qml`, plus Select-All / Clear-Selection + OK/Cancel.

The pickers (`FileDialog` for zip, `FolderDialog` for the other three) stay as
child dialogs opened from Idx 0. **Import progress/summary stays in
`DictionariesWindow.qml`** (its existing frames) — the import window closes once
items are chosen and emits `import_batch_requested`.

### 7a.4 Stages

- **Stage 0 — Backend prerequisites (unblocks the QML).** Add `DictionaryManager`
  invokables: a discovery probe (`scan_source(kind, path) -> items_json`,
  worker-thread, no DB mutation) and `import_dir(path, label, lang)` alongside
  `import_zip`. Add qmllint stubs in
  `assets/qml/com/profoundlabs/simsapa/DictionaryManager.qml`. Independent of the
  QML rewrite; can land first.
- **Stage 1 — New row component.** Create `DictionaryImportRow.qml` (checkbox +
  read-only title/count + editable label/lang fields + the four conflict labels +
  lang warning), encapsulating the per-row debounce timer, `valid_label_re`,
  `refresh_status()`, and the stale-guarded `labelStatusChecked`. Register in
  `bridges/build.rs` `qml_files`. Expose per-row `blocking` bool plus
  `checked`/`label`/`lang` for the parent to aggregate.
- **Stage 2 — Rewrite `DictionaryImportDialog.qml`** as the ApplicationWindow with
  the three-frame StackLayout, source pickers, the scan call, and the OK
  aggregation that builds `items_json` and emits `import_batch_requested`.
  Preserve `file://` stripping and `start()` as the entry point. Update
  `bridges/build.rs` if registration changes.
- **Stage 3 — Rewire `DictionariesWindow.qml`.** Replace
  `onImport_requested`/`onReplace_requested` with `onImport_batch_requested`; add
  a sequential batch driver (queue of items, advance on
  `importFinished`/`importFailed`, "Importing N of M", abort cancels the queue,
  per-item failure continues, aggregate into the Idx 4 summary). Delete the dead
  `replace_pending`/`pending_*` machinery.
- **Stage 4 — Bootstrap language fix** (independent; the three edits pinned in §7).

### 7a.5 Gotchas

- **N concurrent debounced DB checks:** debounce *per row* (or on focus-out) and
  stale-guard per row. The current single guard keyed on one `label_input.text`
  cannot disambiguate which row a `labelStatusChecked` belongs to; the row
  component must match on its own current label.
- **Intra-batch duplicate labels:** two checked rows resolving to the same label
  must be flagged client-side — the backend `label_status` only knows about
  already-stored dictionaries (see req. 12).
- **`onClosing` guard:** the import window runs no long write itself (writes
  happen in DictionariesWindow), so it can close once items are emitted; the
  existing DictionariesWindow `onClosing` guard (Idx 1/2/3) still protects the
  actual import.
- **Single-zip replace removal** is a user-visible behavior change — confirm with
  the user before implementing Stage 3.

## 8. Success Metrics

- A user can import N valid dictionaries from a folder in one operation, with
  non-StarDict entries skipped and no errors.
- The checklist correctly blocks OK on colliding/invalid labels among checked
  rows and on zero selection.
- After re-bootstrap, `SELECT language FROM dictionaries WHERE label IN
  ('dpd','bold_definitions','dppn')` returns `pli`/`pli`/`en`, and the
  corresponding `dict_words.language` values match (`pli` for dpd, `en` for
  dppn).
- The remodeled dialog renders correctly on desktop and mobile sizing.

## 9. Open Questions (resolved)

1. **Where is the `bold_definitions` dictionary row created, and does it need a
   language fix?** — Resolved: created by
   `ensure_bold_definitions_parent_dictionary` (dictionaries.rs:352), which
   already sets `language: Some("pli")`. No change needed there; only
   `find_or_create_dpd_dictionary` (the DPD row) needs the `pli` fix.
2. **What `.ifo` field drives language detection?** — Resolved: no `.ifo`
   detection. Default every candidate to `pli` (matching the existing single-zip
   behaviour) and rely on per-row editing.
3. **Cache the parsed `.ifo`/index across discovery → import?** — Resolved: no
   caching. Re-parse on each import.
