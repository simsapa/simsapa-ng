# PRD: Progress Frames for DictionariesWindow

## 1. Introduction / Overview

`DictionariesWindow.qml` currently performs long-running operations (delete, import,
indexing) with minimal in-window feedback: a small inline progress strip above the
dictionary list. The list and its action buttons remain visible during the operation,
which invites users to click again, close the window, or wonder whether the app has
frozen. Delete in particular runs synchronously on the UI thread and gives **no**
visual feedback at all between the confirm dialog and the restart prompt.

This feature restructures `DictionariesWindow.qml` to use a `StackLayout` of full-window
`Frame`s — the same pattern as `DownloadAppdataWindow.qml` — so that long-running
operations replace the list view with a dedicated progress frame. The normal window
controls are hidden until the operation finishes, and the user is shown clear status
text, a batched count of work done, and only the buttons that are valid for the
current state (Cancel during progress; OK on completion/error).

The goal is to make dictionary operations feel honest and responsive across desktop
and mobile, without changing what the operations themselves do.

## 2. Goals

1. Replace the inline progress strip with full-frame progress views modeled on
   `DownloadAppdataWindow.qml`.
2. Provide visible batched progress (every 1000 entries) for import and the
   existing startup re-indexing pass. Delete runs as a single SQL statement
   (relying on the `dict_words.dictionary_id` FK `ON DELETE CASCADE`) and is
   shown with an indeterminate progress bar only.
3. Block list interactions while an operation is in progress, exposing only a
   Cancel button where the operation supports cancellation (import only).
4. Show a summary frame on completion with counts and elapsed time, plus an OK
   button back to the list.
5. Show an error frame with a message and an OK button when an operation fails — no
   retry button; the user decides what to do next.
6. Support `Abort` for import (large StarDicts can take a long time and users may
   not realize the duration): partial inserts remain in the DB and are picked up
   by the next startup reconcile pass. Delete is NOT cancellable (single
   statement, no useful interruption point).
7. Adapt the existing `DictionaryIndexProgressWindow.qml` so the indexing label
   matches the user-requested format
   (e.g. `Indexing: 1/3 Whitney's Roots, 1000/12000 words`).

## 3. User Stories

- **As a user who clicks "Yes" on the delete confirmation**, I want the window to
  visibly switch into a "Deleting…" state, so I know the app is working and that
  I should wait.
- **As a user importing a large StarDict**, I want to see entries being inserted in
  visible batches and know roughly how many remain, instead of staring at a thin
  progress bar above a still-clickable list.
- **As a user starting the app after importing a new dictionary**, I want the
  re-indexing window to tell me both how many dictionaries are being indexed and
  how many words have been processed in the current dictionary (e.g.
  `Indexing: 1/3 Whitney's Roots, 1000/12000 words`).
- **As a user whose import or delete failed**, I want a clear error frame with the
  reason and a single OK button to return to the list, so I can decide what to do.
- **As a user who started an operation by mistake**, I want a Cancel button that
  safely aborts when the operation supports it.

## 4. Functional Requirements

### 4.1 Window structure

1. `DictionariesWindow.qml` MUST use a `StackLayout` (`id: views_stack`) as its
   top-level content, mirroring `DownloadAppdataWindow.qml`.
2. The stack MUST contain the following frames, in this order:
   - **Idx 0 — List frame (default):** the current dictionary list, header
     ("Imported Dictionaries"), Import button, scroll area, and Close button.
   - **Idx 1 — Delete progress frame.**
   - **Idx 2 — Import progress frame.**
   - **Idx 3 — Rename progress frame.**
   - **Idx 4 — Completion/summary frame** (shared by delete, import, and rename).
   - **Idx 5 — Error frame** (shared by delete, import, and rename).
3. The inline progress `Rectangle`/`ProgressBar` currently shown above the list
   (lines 228–256 of the current file) MUST be removed.
4. While `views_stack.currentIndex != 0`, no part of the list, Import button, or
   Close button is visible.
5. The window's `Close` (×) button and the OS close action MUST be disabled while a
   delete or import is in progress, to prevent the user closing the window mid-write.

### 4.2 Delete progress frame (Idx 1)

1. Activated when the user clicks `Yes` on `confirm_delete_dialog`.
2. The frame MUST show:
   - A title: `Deleting dictionary "<label>"…`
   - A status line: `Removing entries…` (no running count — the delete is a
     single SQL statement with no intermediate observable state).
   - An indeterminate progress bar for the duration of the operation.
   - No buttons. Delete is not cancellable.
3. Backend changes (see §7):
   - Delete MUST run on a worker thread so the UI stays responsive.
   - A new `deleteFinished(dictionary_id: int, label: string, removed_count: int,
     elapsed_ms: int)` signal MUST be emitted on success. `removed_count` is
     obtained via `count_words_for_dictionary` BEFORE issuing the cascading
     `DELETE FROM dictionaries WHERE id = ?` (so the count reflects what was
     wiped by the cascade).
   - A `deleteFailed(message: string)` signal MUST be emitted on error.
   - Delete relies on the `dict_words.dictionary_id` FK `ON DELETE CASCADE`
     (see `backend/migrations/dictionaries/2025-05-03-143320_create-tables/up.sql:42`):
     a single `DELETE FROM dictionaries WHERE id = ?` wipes the parent row and
     all child `dict_words` in one statement. Tantivy orphans are reconciled
     on next startup by the existing `DroppingOrphans` pass.
4. No abort. The delete is a single uninterruptible SQL statement; offering a
   cancel button would be misleading.
5. On `deleteFinished`, the stack MUST switch to Idx 4 (summary) with text:
   `Deleted "<label>" — removed <removed_count> entries in <elapsed_s>s.` followed
   by `Simsapa will now exit. Start the application again so that the fulltext
   search index can be updated.` (mirroring the wording of the current
   `restart_dialog`). The summary frame's button is `Quit` (matching
   `DownloadAppdataWindow.qml`'s completion screen) and MUST call `Qt.quit()`.

### 4.3 Import progress frame (Idx 2)

1. Activated when `DictionaryImportDialog` emits `import_requested` or
   `replace_requested`.
2. The frame MUST show:
   - A title: `Importing "<label>"…`
   - A stage line for the current backend stage
     (`Extracting`, `Parsing`, `Inserting words`).
   - A counts line for the inserting-words stage: `Inserting words: <done> / <total>`.
   - A determinate progress bar driven by `done / total`; indeterminate during
     Extracting/Parsing.
   - An `Abort` button.
3. Backend granularity and batching: import MUST run in batches such that the
   `InsertingWords` progress callback fires at least every **1000** inserted rows.
   The current chunk size in `backend/src/stardict_parse.rs:346` is 5000 and MUST
   be reduced to **1000** so that each committed chunk is also a progress tick
   *and* an abort checkpoint. Each chunk MUST be committed in its own transaction
   (or in a way that ensures already-inserted entries survive abort) so that on
   abort the rows already written remain in the database.
4. Abort:
   - Same model as delete: cooperative `AtomicBool` checked between batches.
   - On abort, the partial dictionary MUST be **left in the database**. The
     `dictionaries` row MUST also be left, with the entries already inserted
     associated to it, so that the next startup reconcile pass indexes them
     normally. (This intentionally differs from the existing `Failed` path in
     `stardict_parse.rs`, which calls `delete_dictionary_by_label` to roll back —
     the abort path MUST NOT call that cleanup.)
   - An `importCancelled(message, inserted_count)` signal transitions to the
     summary frame (Idx 4) with text: `Import aborted — "<label>" was partially
     imported (<inserted_count> entries). The remaining entries can be added by
     re-running the import; already-imported entries will be indexed on next
     start. Simsapa will now exit.` Button: `Quit` → `Qt.quit()`.
   - Rationale: a partial dictionary is more useful than no dictionary, and the
     user can re-import (which replaces by label) to complete the work.
5. On `importFinished`, the stack MUST switch to Idx 4 with text:
   `Imported "<label>" — <inserted_count> entries in <elapsed_s>s.` followed by
   `Simsapa will now exit. Start the application again so that the dictionary
   can be indexed for fulltext search.` Button: `Quit` → `Qt.quit()`.

### 4.4 Rename progress frame (Idx 3)

1. Activated when `DictionaryEditDialog` confirms a rename.
2. Rename is fast (a single SQL UPDATE plus an `indexed_at = NULL`) but for
   consistency with delete and import it MUST route through the same progress /
   summary / error frames rather than the inline path it uses today.
3. The frame MUST show:
   - A title: `Renaming "<old_label>" → "<new_label>"…`
   - An indeterminate progress bar.
   - No buttons (no Abort — the operation is a single SQL statement and
     completes in milliseconds; an abort button would never be reachable).
4. Backend changes:
   - `bridges/src/dictionary_manager.rs::rename_label` MUST move to a worker
     thread (mirror the import worker pattern) and emit:
     - `renameFinished(dictionary_id: i32, old_label: QString, new_label: QString, elapsed_ms: i32)`
     - `renameFailed(message: QString)`
   - The existing synchronous return value remains as a quick-fail channel (e.g.
     busy-lock, validation failures) that maps directly to the error frame
     without entering the progress frame.
5. On `renameFinished`, the stack MUST switch to Idx 4 (summary) with text:
   `Dictionary renamed to "<new_label>".` followed by `Simsapa will now exit.
   Start the application again so that the dictionary entries can be re-indexed
   for fulltext search.` Button: `Quit` → `Qt.quit()`. (Matches the current
   `restart_dialog` wording for rename.)
6. On `renameFailed`, route to Idx 5 (error frame).

### 4.5 Completion / summary frame (Idx 4)

1. Shared by delete (success), import (success), import (aborted), and rename
   (success). Delete has no cancellation path, so it never routes here via abort.
2. MUST show:
   - A bold title — `Deleted`, `Imported`, `Import aborted`, or `Renamed`.
   - A summary line with counts and elapsed time as specified above.
   - A note explaining that the app will exit so the search index can be updated
     (text varies per operation; see §4.2 / §4.3 / §4.4).
   - A single right-aligned `Quit` button (matching `DownloadAppdataWindow.qml`'s
     completion screen). Clicking it calls `Qt.quit()`.
3. The existing `restart_dialog` MUST be removed; its message text moves into the
   summary frame.

### 4.6 Error frame (Idx 5)

1. Shared by delete, import, import-replace (delete-then-import), and rename
   failures.
2. MUST show:
   - A bold title: `Error`.
   - The error message returned by the backend (wrapping, selectable text).
   - A single right-aligned `OK` button.
3. Clicking `OK` MUST return the user to Idx 0 (list frame) and refresh the list.
   No retry button. No automatic retry.
4. The existing `MessageDialog { id: error_dialog }` MUST be removed; all error
   surfaces in the window MUST route through this frame.

### 4.7 Startup re-indexing frame — `DictionaryIndexProgressWindow.qml`

1. The existing window MUST continue to be the entry point used by C++ before
   `SuttaSearchWindow`, and MUST continue to drive itself from the existing
   `reconcileProgress` / `reconcileFinished` signals on `DictionaryManager`.
2. The QML `Connections` handler MUST format the
   `ReconcileProgress::IndexingDictionary { label, done, total, dict_index, dict_total }`
   case as a two-line display:
   - Line 1: `Indexing: <dict_index>/<dict_total> <label>`
   - Line 2: `<done> / <total> words`
   - Combined example: `Indexing: 1/3 Whitney's Roots, 1000/12000 words`
     (a single concatenated line is acceptable on narrow widths).
3. The backend MUST emit `IndexingDictionary` progress at least every **1000**
   words within a dictionary. (Current cadence in
   `backend/src/dict_index_reconcile.rs` MUST be verified and tuned if needed.)
4. The `DroppingOrphans` stage MAY continue to display as today (single label),
   since it operates per-dictionary, not per-word.
5. The reconcile window MUST NOT be made cancellable in this PRD — Abort applies
   to user-initiated delete/import only.

### 4.8 Progress cadence (general)

1. UI-visible progress for delete, import, and indexing MUST advance at least once
   per 1000 entries processed (whichever is sooner: 1000 entries, or end of stage).
2. Backend signals MAY be emitted more often; the UI MAY coalesce, but MUST NOT
   skip the final `done == total` emission for each stage.

### 4.9 Behavior summary table

| State | Stack idx | Visible buttons | Window-close enabled |
| --- | --- | --- | --- |
| Idle / list | 0 | Import, Close, per-row Edit/Delete | Yes |
| Deleting | 1 | (none) | No |
| Importing | 2 | Abort | No |
| Renaming | 3 | (none) | No |
| Completed (delete/import/rename) | 4 | Quit (→ Qt.quit) | Yes |
| Import aborted (partial kept) | 4 | Quit (→ Qt.quit) | Yes |
| Error | 5 | OK (→ list) | Yes |

## 5. Non-Goals (Out of Scope)

1. Reworking the dictionary list itself, `DictionaryListItem`, `DictionaryEditDialog`,
   or `DictionaryImportDialog`.
2. Backgrounding operations / allowing the user to navigate elsewhere in the app
   while a delete or import runs (the choice was: Abort only for import, no
   Hide/Background).
3. Adding an Abort button to delete or to the startup re-indexing flow.
4. Adding a Retry button to the error frame.
5. Changing what triggers a full app restart after delete/import/rename (still
   required for the search-index pass).
6. Reworking `DownloadAppdataWindow.qml` or `DownloadProgressFrame.qml`.

## 6. Design Considerations

- Visual style MUST follow `DownloadAppdataWindow.qml`:
  - Each state is a top-level `Frame { Layout.fillWidth; Layout.fillHeight }` inside
    a `StackLayout`.
  - Content uses a centered `ColumnLayout` for the message area and a
    `RowLayout`/`ColumnLayout` at the bottom for buttons.
  - Mobile uses vertical button stacks with `Layout.bottomMargin: 60` to clear the
    Android nav bar (see `DownloadAppdataWindow.qml` lines 564–599); desktop uses
    horizontal rows.
  - `pointSize` / `largePointSize` / `top_bar_margin` are already defined on
    `DictionariesWindow.qml` — reuse them.
- Status text colors: use `palette.text` for body, `palette.mid` for secondary
  counts; do not introduce new colors.
- The progress bar MUST be `indeterminate: true` until the backend supplies a
  `total > 0`.
- Logging: progress frames SHOULD log stage transitions via the existing `Logger`
  component, matching the level used by `DownloadAppdataWindow.qml`.

## 7. Technical Considerations

### 7.1 Existing signals to reuse

- `DictionaryManager.importProgress(stage, done, total)` — exists; reuse.
- `DictionaryManager.importFinished(dictionary_id, label)` — exists; **add**
  `inserted_count` and `elapsed_ms` parameters (signal signature change; QML and
  Rust must update together).
- `DictionaryManager.importFailed(message)` — exists; reuse.
- `DictionaryManager.reconcileProgress(stage, done, total)` — exists; reuse, but
  the stage string from
  `bridges/src/dictionary_manager.rs::reconcile_progress_to_signal`
  MUST be updated to the format described in §4.7 (or, preferably, the QML side
  reformats from a structured stage; simplest path is the formatter in the bridge).
- `DictionaryManager.reconcileFinished()` — exists; reuse.

### 7.2 New signals required

- `deleteFinished(dictionary_id: i32, label: QString, removed_count: i32, elapsed_ms: i32)`
- `deleteFailed(message: QString)`
- `importCancelled(message: QString, inserted_count: i32)` — import-aborted path;
  routes to summary frame, partial entries kept.
- `renameFinished(dictionary_id: i32, old_label: QString, new_label: QString, elapsed_ms: i32)`
- `renameFailed(message: QString)`

No `deleteProgress` / `deleteCancelled` signal: delete is a single SQL
statement with an indeterminate progress bar and no cancellation path.

### 7.3 Backend changes

1. `bridges/src/dictionary_manager.rs::delete_dictionary` MUST move to a worker
   thread (mirror the `import_zip` thread/`qt_thread.queue` pattern) so the UI
   thread stays responsive while the cascading DELETE runs.
2. `backend/src/dictionary_manager_core.rs::delete_user_dictionary` keeps its
   single-statement behaviour (a `DELETE FROM dictionaries WHERE id = ?` that
   relies on the `ON DELETE CASCADE` FK to wipe child `dict_words`). It MUST
   call `count_words_for_dictionary` BEFORE the delete so the worker can report
   the removed count in `deleteFinished`. No per-row progress callback, no
   cancellation flag, no batching.
3. `backend/src/stardict_parse.rs` MUST set `chunk_size = 1000` so each committed
   chunk is both a progress tick and an abort checkpoint. Each chunk MUST commit
   in its own transaction (or equivalent) so that aborted imports keep the rows
   already inserted. Verify import throughput remains acceptable; if a single
   1000-row transaction is too slow, an inner `chunk_size_inner` for the SQL
   insert MAY be smaller while the progress/abort granularity remains 1000.
4. `backend/src/dict_index_reconcile.rs` MUST emit `IndexingDictionary` at least
   every 1000 words. Verify current cadence.
5. Cancellation: import MUST hold a shared `Arc<AtomicBool>` accessible from
   the bridge so a QML `abort_import()` invokable can flip it; the worker
   checks it between insert chunks. Delete has no cancellation.
6. Rename: `bridges/src/dictionary_manager.rs::rename_label` MUST move to a worker
   thread and emit `renameFinished` / `renameFailed`. No abort flag is required
   (operation is a single SQL statement).

### 7.4 Removed UI elements

- The inline `Rectangle` progress strip in `DictionariesWindow.qml`.
- `Dialog { id: restart_dialog }` — its content moves into the summary frame.
- `MessageDialog { id: error_dialog }` — replaced by the error frame.
- `MessageDialog { id: confirm_delete_dialog }` stays (it is a *pre-operation*
  confirmation, not progress feedback).

### 7.5 QML conventions

- Follow CLAUDE.md: snake_case ids/properties; QML PascalCase components.
- No new bridge files needed; signal additions go into the existing
  `bridges/src/dictionary_manager.rs`. The `qmllint` type definition at
  `assets/qml/com/profoundlabs/simsapa/DictionaryManager.qml` MUST be updated to
  declare the new signals (matching the project's existing pattern documented in
  CLAUDE.md).

## 8. Success Metrics

1. After clicking `Yes` on the delete confirmation for a dictionary with ≥10,000
   entries, the user sees a non-blank, updating progress frame within 500 ms and
   the running count advances at least once per second on a typical desktop SSD.
2. During import of a ≥10,000-entry StarDict, the `Inserting words` count
   advances at least every 1000 entries on both desktop and Android.
3. The startup reconcile window's label shows the dictionary index, label, and
   word counts in the format
   `Indexing: <i>/<n> <label>, <done>/<total> words`.
4. No code path in `DictionariesWindow.qml` leaves the user looking at the list
   view while a delete or import is in progress (manual QA checklist).
5. Cancellation, when triggered, leaves the database in a consistent state
   verifiable by a follow-up startup reconcile (no permanent FTS orphans beyond a
   single reconcile pass).

## 9. Open Questions

(All previously-open questions have been resolved:
- Summary → `Quit` button, calls `Qt.quit()`, matching `DownloadAppdataWindow.qml`.
- Import abort keeps partial entries for next-start indexing.
- Sub-1000-entry tick cadence: acceptable as-is.
- Rename routes through the same progress / summary / error frames for
  consistency and code-deduplication.)

None remaining.

## 10. Code Review: existing code vs. PRD

This section maps the PRD requirements against the current codebase and
identifies what can be reused, what needs to be adapted, and what is genuinely
new. Captured during the review pass on 2026-05-19.

### 10.1 Reuse as-is (no changes)

| PRD requirement | Existing code |
|---|---|
| Import worker thread + progress pipeline | `bridges/src/dictionary_manager.rs:190-225` (`import_zip`) already spawns a thread, queues via `qt_thread.queue`, emits `importProgress`/`importFinished`/`importFailed`. |
| Reconcile worker thread + progress pipeline | `bridges/src/dictionary_manager.rs:435-454` (`start_reconcile`) and `backend/src/dict_index_reconcile.rs`. |
| Reconcile structured progress with `dict_index/dict_total` and per-word `done/total` | `ReconcileProgress::IndexingDictionary { label, done, total, dict_index, dict_total }` (`dict_index_reconcile.rs:38-48`) — exactly what §4.7 needs. |
| Reconcile word-cadence ≥ every 1000 | `index_dict_words_into_dict_index` already chunks at 1000 (`backend/src/search/indexer.rs:688,715`). **PRD §4.7 / §7.3.4 is already satisfied** — no change needed beyond verification. |
| Reconcile window QML + C++ launch | `assets/qml/DictionaryIndexProgressWindow.qml` + `cpp/gui.cpp:428-454`. |
| `count_words_for_dictionary` for delete `total` | `backend/src/db/dictionaries.rs:142-151`. |
| Global serialisation lock | `DICT_MGR_LOCK` / `BUSY_MSG` in `dictionary_manager_core.rs:23-26`. |
| Confirm-delete dialog (pre-operation) | `MessageDialog { id: confirm_delete_dialog }` in `DictionariesWindow.qml:119-140`. |
| `DownloadAppdataWindow` Frame/StackLayout pattern, Quit-button completion screen | `assets/qml/DownloadAppdataWindow.qml` — directly applicable template. |

### 10.2 Adapt (modify existing)

| Change | Where | Notes |
|---|---|---|
| Stardict insert chunk size 5000 → 1000 | `backend/src/stardict_parse.rs:346` | Single-line change; gives both 1000-row progress cadence AND abort checkpoints. **Important:** the surrounding `db_conn.transaction::<…>` (line 348) wraps *all chunks* in one transaction, so a rollback on abort would discard everything inserted. To satisfy "abort keeps partial entries" (§4.3.4) the per-chunk commit MUST be moved outside the outer transaction — i.e. commit each 1000-row chunk independently. This is a non-trivial change to that function's transaction boundaries; flag in tasks. |
| Reconcile stage string format | `bridges/src/dictionary_manager.rs:180-184` (`reconcile_progress_to_signal`) | Currently formats `"Indexing {label} ({i}/{n})"`. Change to produce something the QML can split into two lines, or pass structured fields. Simplest: emit `"Indexing: 1/3 Whitney's Roots, 1000/12000 words"` directly in the bridge and reuse the existing single `stage` QString. |
| `DictionaryIndexProgressWindow.qml` label | `assets/qml/DictionaryIndexProgressWindow.qml:35-43` | Just relays whatever `stage` arrives; if the bridge formats the line as above, no QML change needed. |
| `importFinished` signature | `bridges/src/dictionary_manager.rs:118`, `:211` | Add `inserted_count: i32, elapsed_ms: i32`. Requires updating QML `Connections.onImportFinished` in `DictionariesWindow.qml:72-80` and the `DictionaryManager.qml` stub. |
| `rename_label` → worker thread | `bridges/src/dictionary_manager.rs:237-245` and `dictionary_manager_core.rs:254-300` | Currently synchronous. Move to worker thread following the `import_zip` pattern. Existing `Result<(), String>` return becomes the quick-fail path (busy/validation); a successful kickoff returns `"ok"` and emits `renameFinished`/`renameFailed` from the worker. |
| `delete_dictionary` → worker thread | `bridges/src/dictionary_manager.rs:227-235` and `dictionary_manager_core.rs:227-248` | Move to a thread following the `import_zip` pattern. Backend `delete_user_dictionary` keeps its single-statement cascade behaviour; the worker calls `count_words_for_dictionary` first so it can report `removed_count` in `deleteFinished`. No progress callback, no cancellation flag. |
| Keep SQL cascade for delete | `backend/src/db/dictionaries.rs:109-115` (`delete_dictionary_by_label`) | Unchanged: a single `DELETE FROM dictionaries WHERE label = ?` relying on the `ON DELETE CASCADE` FK. Continues to be reused by the `stardict_parse.rs` failure-cleanup callsites; the new delete worker uses the same helper (or the `id`-based equivalent). |
| Remove inline progress strip, `restart_dialog`, `error_dialog` | `DictionariesWindow.qml:92-117, 228-256` | Wholesale replacement with `StackLayout` of frames. |
| `DictionaryManager.qml` qmllint stub | `assets/qml/com/profoundlabs/simsapa/DictionaryManager.qml` | Add signal declarations + new function stubs for: `abort_import`, signals `deleteFinished`, `deleteFailed`, `importCancelled`, `renameFinished`, `renameFailed`, and the updated `importFinished` signature. |

### 10.3 New (does not yet exist)

#### Bridge (`bridges/src/dictionary_manager.rs`)

- `#[qinvokable] fn abort_import(self: Pin<&mut Self>)` — flips `import_cancel_flag`.
- Stored state on `DictionaryManagerRust`: one `Arc<AtomicBool>` field for the
  in-flight import. (Currently struct is `#[derive(Default)]` with no fields.)
- New signals:
  `deleteFinished(dictionary_id, label, removed_count, elapsed_ms)`,
  `deleteFailed(message)`,
  `importCancelled(message, inserted_count)`,
  `renameFinished(dictionary_id, old_label, new_label, elapsed_ms)`,
  `renameFailed(message)`.
- New formatter or QML-side handling for the reconcile two-line label.

#### Backend (`backend/src/dictionary_manager_core.rs`)

- The existing `delete_user_dictionary` remains a single-statement cascade
  delete; the bridge worker calls `count_words_for_dictionary` immediately
  before it so the resulting `removed_count` can be reported in
  `deleteFinished`. No new with-progress / with-cancel variant.
- Plumb `cancel_flag: &AtomicBool` through `import_user_zip` and
  `import_stardict_as_new` (`stardict_parse.rs:251` signature change) so abort
  can be checked between chunks. Important: on abort, do NOT call
  `delete_dictionary_by_label` — let partial rows survive (this is the inverse
  of the existing `Failed` cleanup at `stardict_parse.rs:271,324,370`).
- New `StardictImportProgress::Aborted { inserted: usize }` variant, or a
  separate return-channel for the inserted count.

#### Backend (`backend/src/db/dictionaries.rs`)

- No new helpers needed for delete. The existing `delete_dictionary_by_label`
  (and a sibling `id`-based variant if convenient) drives the single-statement
  cascade. The clarifying note at the head of `delete_dictionary_by_label`
  documents the cascade reliance and the (intentionally unused) `LIMIT`
  fallback shape for any future batched scenarios.

#### Backend (`backend/src/stardict_parse.rs`)

- Per-chunk commit boundary (move `db_conn.transaction(...)` inside the
  `for chunk in …` loop, or use separate transactions per chunk) so aborted
  partial inserts persist.
- Cancel check between chunks.
- Emit `Aborted { inserted }` on cancel; do **not** call
  `delete_dictionary_by_label`.

#### QML

- `DictionariesWindow.qml`: rewrite top-level content as `StackLayout` with 6
  frames (Idx 0–5). Reuse the existing list section (header + scroll +
  delegates) inside Idx 0.
- New `Connections` handlers for the new signals.
- Window-close intercept: `onClosing` handler that ignores close while
  `views_stack.currentIndex` is 1, 2, or 3 (§4.1.5).

### 10.4 Things the PRD got right that the code already supports

1. Reconcile already emits the structured `dict_index/dict_total/done/total` —
   only the formatter string differs.
2. Indexer already ticks at 1000 — no change.
3. Worker-thread / `qt_thread.queue` plumbing is established and copyable for
   delete/rename.
4. `DownloadAppdataWindow.qml` is a faithful template for the
   Frame+StackLayout+Quit-button pattern.

### 10.5 Risks / verifications required before implementation

1. **Foreign-key cascade direction.** Verified (see §10.2): the migration at
   `backend/migrations/dictionaries/2025-05-03-143320_create-tables/up.sql:42`
   declares `dict_words.dictionary_id` as `ON DELETE CASCADE`, so a single
   `DELETE FROM dictionaries WHERE id = ?` is sufficient to wipe a user
   dictionary. The simpler implementation (no batching, no per-row progress,
   no cancellation for delete) is preferred and is the chosen path.
2. **`import_stardict_as_new` transaction boundary.** The current single outer
   transaction at `stardict_parse.rs:348` is intentional for atomicity.
   Splitting it into per-chunk transactions is the only way to satisfy
   "abort keeps partial entries", and is a real semantic change — confirm
   acceptable for the PRD's stated goal.
4. **`importFinished` signature change** ripples to every caller of the QML
   signal. Currently only `DictionariesWindow.qml:72` consumes it — small blast
   radius.
5. **Rename + reconcile semantics.** Rename sets `indexed_at = NULL` so the
   next reconcile re-indexes. The PRD's rename-summary text matches this.
   No conflict.
6. **No existing `Arc<AtomicBool>` cancellation pattern** in the codebase —
   import establishes the convention. Delete does not need one.

### 10.6 Suggested implementation order

1. Bridge: move `delete_dictionary` to a worker thread; add `deleteFinished` /
   `deleteFailed` signals; QML rewrite of `DictionariesWindow.qml` (delete
   path first, indeterminate progress only; import/rename still using old path
   so the UI can be incrementally tested).
2. Backend: per-chunk commits in `stardict_parse.rs` + cancel check; new
   `Aborted` variant.
3. Bridge: extend `importFinished` signature, add `importCancelled`, add
   `abort_import`.
4. Bridge: worker-thread `rename_label` + signals.
5. Bridge: reconcile stage string reformat (one-liner).
6. QML stub updates (`DictionaryManager.qml`) — last, since signatures stabilise
   by then.
7. `make build -B`, then `cargo test`, then manual smoke.
