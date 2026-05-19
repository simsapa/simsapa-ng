# Tasks: Progress Frames for DictionariesWindow

Source PRD: [prd-dictionaries-window-progress-frames.md](./prd-dictionaries-window-progress-frames.md)

## Relevant Files

### Backend (Rust)

- `backend/src/db/dictionaries.rs` — no code changes for delete; the existing `delete_dictionary_by_label` (and the `ON DELETE CASCADE` FK) is the chosen path. The note at the head of the function documents the cascade reliance. Keep `count_words_for_dictionary` so the worker can report `removed_count`.
- `backend/src/dictionary_manager_core.rs` — keep `delete_user_dictionary` as-is (single-statement cascade delete); plumb `cancel_flag` through `import_user_zip`; keep `BUSY_MSG` / `DICT_MGR_LOCK`.
- `backend/src/stardict_parse.rs` — switch the SQL insert to per-chunk transactions; reduce `chunk_size` from 5000 → 1000; add cancel check between chunks; add `StardictImportProgress::Aborted { inserted }`. Do NOT call `delete_dictionary_by_label` on abort.
- `backend/src/dict_index_reconcile.rs` — verify the existing per-1000-word cadence in `index_dict_words_into_dict_index`; no code change expected, only verification.
- `backend/src/search/indexer.rs` — referenced for verification only (already chunks at 1000, `:688,715`).
- `backend/tests/stardict_import_per_chunk_commit.rs` *(new)* — integration test confirming partial inserts survive abort.

### Bridge (CXX-Qt)

- `bridges/src/dictionary_manager.rs` — store one `Arc<AtomicBool>` import-cancel flag on `DictionaryManagerRust`; add new invokable `abort_import`; add new signals `deleteFinished`, `deleteFailed`, `importCancelled`, `renameFinished`, `renameFailed`; extend `importFinished` signature with `inserted_count` + `elapsed_ms`; move `delete_dictionary` and `rename_label` to worker threads; update `reconcile_progress_to_signal` formatter. Delete has no progress / cancel signals.
- `bridges/build.rs` — no change expected (file is already registered).

### QML (UI)

- `assets/qml/DictionariesWindow.qml` — wholesale restructure: `StackLayout` with frames Idx 0–5; remove `restart_dialog`, `error_dialog`, inline progress strip; add `Connections` for new signals; add `onClosing` interception while a long op is in progress.
- `assets/qml/DictionaryIndexProgressWindow.qml` — verify the new bridge-side `stage` string format renders correctly. Likely no code change.
- `assets/qml/com/profoundlabs/simsapa/DictionaryManager.qml` — `qmllint` type stub: declare new invokables and signals, update `importFinished` signature.
- `assets/qml/com/profoundlabs/simsapa/qmldir` — verify no edits needed (no new files).

### C++

- `cpp/gui.cpp` — referenced for verification only (lines 428–454 already launch `DictionaryIndexProgressWindow.qml` correctly).

### Notes

- Per CLAUDE.md: always use `make build -B` for builds (not direct cmake commands). Rust tests via `cd backend && cargo test`. Avoid `make qml-test` and avoid running the GUI as an agent.
- Per CLAUDE.md: when adding new functions to `DictionaryManager` (Rust bridge), the corresponding `DictionaryManager.qml` stub MUST be updated with matching function signatures for `qmllint`.
- Per CLAUDE.md: always use `try_exists()` (not `.exists()`) for file checks (Android-safe).
- After each top-level task, run `make build -B` and (when a top-level task is complete) `cd backend && cargo test`. Skip QML tests unless asked.
- Keep [PROJECT_MAP.md](../PROJECT_MAP.md) updated as new functions / files are added.

## Tasks

- [x] 1.0 Pre-implementation verifications
  - [x] 1.1 Verified: bundled libsqlite3-sys does NOT support `DELETE … LIMIT` (errors with `near "LIMIT": syntax error`). Portable fallback `DELETE … WHERE id IN (SELECT id … LIMIT ?)` works. Probe test: `backend/tests/test_delete_limit_support.rs`. **No longer load-bearing for this PRD** — delete now relies on the FK cascade, not on batched `LIMIT` deletes. Note retained near `delete_dictionary_by_label` for future reference.
  - [x] 1.2 Verified: `backend/migrations/dictionaries/2025-05-03-143320_create-tables/up.sql:42` declares `FOREIGN KEY(dictionary_id) REFERENCES dictionaries (id) ON DELETE CASCADE`. This is the **chosen path** for user-dictionary delete: a single `DELETE FROM dictionaries WHERE id = ?` wipes the parent row and all child `dict_words`. No batched delete needed.
  - [x] 1.3 Estimate: bundled SQLite in WAL mode commits a 1000-row chunk in ~5–20 ms on desktop SSD; ~100 chunks for a 100k-entry dictionary ⇒ ~1–2 s of commit overhead total. Acceptable for import per-chunk commits; no commit-coalescing required.
  - [x] 1.4 `importFinished` callers (excluding `bridges/target/` build artefacts):
        - `assets/qml/DictionariesWindow.qml:72` — main caller (updated in §5.6).
        - `assets/qml/DictionarySearchDictionariesPanel.qml:98` — **additional caller not mentioned in the PRD**; §5.6 must also update its handler signature (`+ inserted_count: int, elapsed_ms: int`).
        - `assets/qml/com/profoundlabs/simsapa/DictionaryManager.qml:92` — qmllint stub (§8.1).
        - `bridges/src/dictionary_manager.rs:117` — bridge declaration (§5.1).

- [ ] 2.0 Bridge: route user-dictionary delete through a worker thread (FK cascade)
  - Decision (2026-05-19): use the existing `ON DELETE CASCADE` FK instead of batched deletes. The delete frame uses an indeterminate progress bar and has no Abort button. This subsection replaces the previous "batched cancellable delete" plan.
  - [ ] 2.1 In `bridges/src/dictionary_manager.rs`, add `#[qsignal]`s: `deleteFinished(dictionary_id: i32, label: QString, removed_count: i32, elapsed_ms: i32)` and `deleteFailed(message: QString)`. No `deleteProgress`, no `deleteCancelled`, no `abort_delete` invokable.
  - [ ] 2.2 Rewrite `fn delete_dictionary(&self, …) -> QString` to spawn a worker thread (mirroring `import_zip`). The worker:
        1. acquires `DICT_MGR_LOCK`;
        2. calls `count_words_for_dictionary(dict_id)` to capture `removed_count`;
        3. captures `Instant::now()`;
        4. calls the existing `delete_user_dictionary` (single `DELETE FROM dictionaries WHERE id = ?` — FK cascade wipes `dict_words`);
        5. on success queues `deleteFinished(dict_id, label, removed_count, elapsed_ms)`;
        6. on error queues `deleteFailed(message)`.
        The synchronous return is `"ok"` for kickoff or a busy-lock message for quick-fail.
  - [ ] 2.3 No changes to `backend/src/db/dictionaries.rs` for delete (the explanatory note at the head of `delete_dictionary_by_label` already documents the cascade reliance).
  - [ ] 2.4 No new backend tests for delete (the cascade is exercised by existing import + manual delete flow; the worker is thin glue covered by manual smoke).
  - [ ] 2.5 Run `make build -B`.

- [ ] 3.0 QML: migrate `DictionariesWindow.qml` to a StackLayout and wire delete through it
  - [ ] 3.1 In `bridges/src/dictionary_manager.rs`, change `struct DictionaryManagerRust` from `#[derive(Default)]` to a struct with `import_cancel: Arc<AtomicBool>`. Implement `Default` manually with a fresh flag. (Delete does not need a cancel flag.)
  - [ ] 3.2 Restructure `assets/qml/DictionariesWindow.qml`: wrap the current `ColumnLayout` content into a new top-level `StackLayout { id: views_stack }`. Slot the existing header + import button + list ScrollView + close button into a `Frame` as Idx 0 (the list). Remove the inline progress `Rectangle` (current lines 228–256). Remove `Dialog { id: restart_dialog }` and `MessageDialog { id: error_dialog }`.
  - [ ] 3.3 Add Idx 1 (delete progress frame): title `Deleting dictionary "<label>"…`, status line `Removing entries…`, indeterminate progress bar, no buttons. Follow the styling of `DownloadAppdataWindow.qml`.
  - [ ] 3.4 Add Idx 4 (summary frame, shared) and Idx 5 (error frame, shared). Idx 4 has a `Quit` button → `Qt.quit()`. Idx 5 has an `OK` button → returns to Idx 0 and calls `refresh_list()`.
  - [ ] 3.5 Add `Connections { target: dict_manager }` handlers for `onDeleteFinished` and `onDeleteFailed`. On finished: switch to Idx 4 with the summary text from PRD §4.2.5. On failed: switch to Idx 5.
  - [ ] 3.6 Replace `confirm_delete_dialog.onButtonClicked` to: switch to Idx 1, call `dict_manager.delete_dictionary(id)`. Remove the old synchronous result handling.
  - [ ] 3.7 Add `onClosing` to `ApplicationWindow`: `if (views_stack.currentIndex === 1 || 2 || 3) close.accepted = false;`. (Idx 2 and 3 will be added in later tasks; the guard is forward-compatible.)
  - [ ] 3.8 Keep import and rename still using the old paths for now (do NOT yet remove the old `restart_dialog` references from `onImport_requested` / `onRenamed`; instead, route those to the new Idx 4/5 frames temporarily by creating a small helper, OR keep a minimal legacy `Dialog` for import/rename until §5/§6 cut them over). Choose whichever is simpler; document the choice in a TODO inside the QML file.
  - [ ] 3.9 Run `make build -B` → confirm successful build. Manually verify (or document inability to verify): selecting a small test dictionary and deleting it shows the new delete frame.

- [ ] 4.0 Backend: per-chunk import commits and cancellation
  - [ ] 4.1 In `backend/src/stardict_parse.rs`, change `chunk_size` from 5000 to 1000 (line 346).
  - [ ] 4.2 Restructure the insert loop: replace the single outer `db_conn.transaction::<…>` (line 348) with a loop where each 1000-row chunk runs in its OWN transaction (`db_conn.transaction(|tx| { create_dict_words_batch(tx, chunk) })`). After each successful chunk transaction, call `progress(StardictImportProgress::InsertingWords { done, total })`.
  - [ ] 4.3 Add `StardictImportProgress::Aborted { inserted: usize }` variant. Update `match` arms in `bridges/src/dictionary_manager.rs::stardict_progress_to_signal` to map it (e.g. `("Aborted", inserted as i32, 0)` — though the bridge will handle abort via a separate signal; see §5).
  - [ ] 4.4 Add a `cancel: &AtomicBool` parameter to `import_stardict_as_new` (`stardict_parse.rs:251`). Between chunks, check `cancel.load(Ordering::Relaxed)`; on `true`, emit `Aborted { inserted }` and return early (Ok) WITHOUT calling `delete_dictionary_by_label`. Crucially: existing `Failed`-path calls to `delete_dictionary_by_label` (lines 271, 324, 370) remain — those are not abort, they are unrecoverable errors.
  - [ ] 4.5 Add a `cancel: &AtomicBool` parameter to `import_user_zip` (`backend/src/dictionary_manager_core.rs:82`). Pass it through to `import_stardict_as_new`.
  - [ ] 4.6 Return type of `import_user_zip` / `import_stardict_as_new` needs to carry the abort vs success distinction. Either: change return to `Result<ImportOutcome, String>` where `ImportOutcome { dictionary_id, inserted, cancelled }`; OR rely on the `Aborted` progress event and return `Ok(dictionary_id)` in both cases. Pick option A for explicitness — update the function signatures and call sites in `dictionary_manager_core.rs::import_user_zip` and `bridges/src/dictionary_manager.rs::import_zip`.
  - [ ] 4.7 Add `backend/tests/stardict_import_per_chunk_commit.rs`: feed a small synthetic StarDict, set the cancel flag after the second chunk, assert that ~2000 rows persist in `dict_words` after the worker returns and the `dictionaries` row is also present (so reconcile will pick it up at next start).
  - [ ] 4.8 Run `cd backend && cargo test`.

- [ ] 5.0 Bridge + QML: import path migration
  - [ ] 5.1 In `bridges/src/dictionary_manager.rs`, change `importFinished` signal signature to `(dictionary_id: i32, label: QString, inserted_count: i32, elapsed_ms: i32)`.
  - [ ] 5.2 Add new signal `importCancelled(message: QString, inserted_count: i32)`. Add invokable `abort_import(self: Pin<&mut DictionaryManager>)` that flips `import_cancel`.
  - [ ] 5.3 Update `fn import_zip` worker: capture `Instant::now()` at start; reset `import_cancel`; pass it down to `import_user_zip`; on `Ok(ImportOutcome { cancelled: true, inserted, .. })` queue `importCancelled(message, inserted)`; on `Ok(ImportOutcome { cancelled: false, dictionary_id, inserted, .. })` queue `importFinished(dictionary_id, label, inserted, elapsed_ms)`.
  - [ ] 5.4 In `DictionariesWindow.qml`, add Idx 2 (import progress frame): title `Importing "<label>"…`, stage line, counts line `Inserting words: <done> / <total>`, progress bar (indeterminate for Extracting/Parsing, determinate for InsertingWords), single right-aligned `Abort` button calling `dict_manager.abort_import()`.
  - [ ] 5.5 Update the `onImportProgress` handler in QML to interpret `stage` and drive both the stage line and counts line; keep determinate-vs-indeterminate logic.
  - [ ] 5.6 Update `onImportFinished` to use the new 4-arg signature and switch to Idx 4 (summary) with the PRD §4.3.5 text. Add `onImportCancelled` that switches to Idx 4 with the PRD §4.3.4 partial-import text.
  - [ ] 5.7 Update `onImport_requested` and `onReplace_requested` handlers in `DictionaryImportDialog` connections: replace `root.busy = true` + progress text assignments with `views_stack.currentIndex = 2`, then call `dict_manager.import_zip(...)`.
  - [ ] 5.8 Drop any temporary legacy import paths introduced in §3.11.
  - [ ] 5.9 Update the `DictionaryManager.qml` qmllint stub for the new `importFinished` signature and the new signals/functions. (Final pass on the stub is §8.1.)
  - [ ] 5.10 Run `make build -B`.

- [ ] 6.0 Bridge + QML: rename path migration
  - [ ] 6.1 In `bridges/src/dictionary_manager.rs`, add `#[qsignal]`s `renameFinished(dictionary_id: i32, old_label: QString, new_label: QString, elapsed_ms: i32)` and `renameFailed(message: QString)`.
  - [ ] 6.2 Rewrite `fn rename_label` to spawn a worker thread (mirroring `import_zip`). The synchronous return remains `"ok"` for kickoff or a busy/validation error string for quick-fail. On success the worker queues `renameFinished`; on error it queues `renameFailed`.
  - [ ] 6.3 In `DictionariesWindow.qml`, add Idx 3 (rename progress frame): title `Renaming "<old_label>" → "<new_label>"…`, indeterminate progress bar, no buttons.
  - [ ] 6.4 Replace `DictionaryEditDialog` `onRenamed`/`onFailed` handlers: the dialog should no longer call into the bridge directly. Add a new signal on `DictionaryEditDialog` (`rename_requested(dictionary_id, new_label)`) or have it call `dict_manager.rename_label` then switch to Idx 3. The simplest path: keep the dialog's call site, but switch the parent window to Idx 3 before the call, and wire `onRenameFinished` / `onRenameFailed` `Connections` on `dict_manager` to switch to Idx 4 / Idx 5.
  - [ ] 6.5 Inspect `DictionaryEditDialog.qml` to confirm where the bridge call is currently made; adjust either the dialog or the window accordingly so that the call happens *before* the frame switch and the result is delivered via signals, not the dialog's `onRenamed`.
  - [ ] 6.6 Summary text on Idx 4 for rename: `Dictionary renamed to "<new_label>". Simsapa will now exit. Start the application again so that the dictionary entries can be re-indexed for fulltext search.` Button: `Quit`.
  - [ ] 6.7 Remove `MessageDialog { id: error_dialog }` and `Dialog { id: restart_dialog }` if any temporary references survived. All error and completion paths now route through Idx 4 / Idx 5.
  - [ ] 6.8 Run `make build -B`.

- [ ] 7.0 Reconcile label reformat
  - [ ] 7.1 In `bridges/src/dictionary_manager.rs::reconcile_progress_to_signal`, change the `IndexingDictionary` arm to format the stage as `"Indexing: <dict_index>/<dict_total> <label>, <done>/<total> words"` (a single concatenated line as accepted by PRD §4.7.2). Keep `done` and `total` as the integer fields for the progress bar.
  - [ ] 7.2 Open `assets/qml/DictionaryIndexProgressWindow.qml`, run a manual smoke (or read-through) to confirm the new long string wraps reasonably (`Text.Wrap` is already set on `stage_label`).
  - [ ] 7.3 Verify (no code change expected) that `backend/src/search/indexer.rs:688,715` still emits at every 1000 words and that `backend/src/dict_index_reconcile.rs:160-178` forwards every emit. If a small dictionary (<1000 words) only emits at completion, that is acceptable per PRD §4.8.

- [ ] 8.0 `DictionaryManager.qml` qmllint stub finalisation and final QA
  - [ ] 8.1 Update `assets/qml/com/profoundlabs/simsapa/DictionaryManager.qml` to declare all new invokables and signals introduced in §2, §5, §6: `abort_import()`, signals `deleteFinished`, `deleteFailed`, `importCancelled`, `renameFinished`, `renameFailed`, and the updated `importFinished(dictionary_id: int, label: string, inserted_count: int, elapsed_ms: int)` signature. Per CLAUDE.md, function bodies in the stub return simple placeholder values.
  - [ ] 8.2 Confirm `assets/qml/com/profoundlabs/simsapa/qmldir` needs no changes (no new files).
  - [ ] 8.3 Update [PROJECT_MAP.md](../PROJECT_MAP.md) to reflect the new bridge functions, signals, and the StackLayout structure of `DictionariesWindow.qml`.
  - [ ] 8.4 Run `make build -B` to confirm a fully clean build. Address any `qmllint` warnings about missing types in the stub.
  - [ ] 8.5 Run `cd backend && cargo test`. Per CLAUDE.md memory: don't gate the new integration tests behind `#[ignore]`; pre-existing unrelated failures may be ignored after confirming the build is clean.
  - [ ] 8.6 Manual smoke checklist (to be exercised by the user, not the agent — per CLAUDE.md GUI-testing guidance):
    - Delete a small (~100-entry) dictionary; confirm Idx 1 → Idx 4 → Quit.
    - Delete a large (~10k-entry) dictionary; confirm Idx 1 shows the indeterminate bar for the duration of the cascade DELETE, then Idx 4 → Quit; restart app and confirm reconcile cleans up any Tantivy orphans for the deleted label via the existing `DroppingOrphans` pass.
    - Import a small StarDict; confirm Idx 2 → Idx 4 → Quit.
    - Import a large StarDict; confirm progress ticks every 1000 inserts; click Abort mid-way; restart app; confirm reconcile picks up the partial dictionary and indexes the rows already present.
    - Rename a dictionary; confirm Idx 3 → Idx 4 → Quit.
    - Confirm the OS close (×) button is ignored during Idx 1/2/3 and works in Idx 0/4/5.
    - Confirm the startup reconcile window now shows `Indexing: 1/N <label>, M/T words`.
