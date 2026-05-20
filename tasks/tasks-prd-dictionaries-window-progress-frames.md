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

- [x] 2.0 Bridge: route user-dictionary delete through a worker thread (FK cascade)
  - Decision (2026-05-19): use the existing `ON DELETE CASCADE` FK instead of batched deletes. The delete frame uses an indeterminate progress bar and has no Abort button. This subsection replaces the previous "batched cancellable delete" plan.
  - [x] 2.1 In `bridges/src/dictionary_manager.rs`, add `#[qsignal]`s: `deleteFinished(dictionary_id: i32, label: QString, removed_count: i32, elapsed_ms: i32)` and `deleteFailed(message: QString)`. No `deleteProgress`, no `deleteCancelled`, no `abort_delete` invokable.
  - [x] 2.2 Rewrite `fn delete_dictionary(&self, …) -> QString` to spawn a worker thread (mirroring `import_zip`). Implementation note: lookup of label + entry count happens on the calling (UI) thread BEFORE the spawn so that bogus ids fail fast with a synchronous error string; the worker then calls `delete_user_dictionary` (which acquires `DICT_MGR_LOCK` internally), captures `Instant::now()` around the cascade DELETE, and queues `deleteFinished` / `deleteFailed`. The synchronous return is `"ok"` on kickoff or an error message on quick-fail (bogus id / list failure).
  - [x] 2.3 No changes to `backend/src/db/dictionaries.rs` for delete (the explanatory note at the head of `delete_dictionary_by_label` already documents the cascade reliance).
  - [x] 2.4 No new backend tests for delete (the cascade is exercised by existing import + manual delete flow; the worker is thin glue covered by manual smoke).
  - [x] 2.5 Run `make build -B`.

- [x] 3.0 QML: migrate `DictionariesWindow.qml` to a StackLayout and wire delete through it
  - [x] 3.1 In `bridges/src/dictionary_manager.rs`, change `struct DictionaryManagerRust` from `#[derive(Default)]` to a struct with `import_cancel: Arc<AtomicBool>`. Implement `Default` manually with a fresh flag. (Delete does not need a cancel flag.)
  - [x] 3.2 Restructure `assets/qml/DictionariesWindow.qml`: wrap the current `ColumnLayout` content into a new top-level `StackLayout { id: views_stack }`. Slot the existing header + import button + list ScrollView + close button into a `Frame` as Idx 0 (the list). Removed the inline progress `Rectangle`, `Dialog { id: restart_dialog }`, and `MessageDialog { id: error_dialog }`.
  - [x] 3.3 Add Idx 1 (delete progress frame): title `Deleting dictionary "<label>"…`, status line `Removing entries…`, indeterminate progress bar, no buttons.
  - [x] 3.4 Add Idx 4 (summary frame, shared) and Idx 5 (error frame, shared). Idx 4 has a `Quit` button → `Qt.quit()`. Idx 5 has an `OK` button → returns to Idx 0 and calls `refresh_list()`.
  - [x] 3.5 Added `Connections { target: dict_manager }` handlers for `onDeleteFinished` and `onDeleteFailed`. On finished: switch to Idx 4 with the summary text from PRD §4.2.5. On failed: switch to Idx 5.
  - [x] 3.6 `confirm_delete_dialog.onButtonClicked` now switches to Idx 1 and calls `dict_manager.delete_dictionary(id)`. Synchronous quick-fail routes directly to Idx 5.
  - [x] 3.7 Added `onClosing` to `ApplicationWindow`: ignores close while `views_stack.currentIndex` is 1, 2, or 3 (forward-compatible with the import/rename frames).
  - [x] 3.8 Import and rename are kept on their legacy synchronous paths, but their result handlers now route to the new Idx 4 / Idx 5 frames instead of the deleted `restart_dialog` / `error_dialog`. TODO comments in the QML mark the §5/§6 cutovers. Also added placeholder Idx 2 (import progress) and Idx 3 (rename progress) frames so the StackLayout indices line up with the PRD now and §5/§6 can fill in the details.
  - [x] 3.9 Run `make build -B` → successful build. (Manual delete-flow verification is for the user, per CLAUDE.md GUI-testing guidance.)

- [x] 4.0 Backend: per-chunk import commits and cancellation
  - [x] 4.1 `chunk_size` reduced from 5000 → 1000 in `backend/src/stardict_parse.rs`.
  - [x] 4.2 Insert loop restructured: each 1000-row chunk now commits in its own `db_conn.transaction(...)`. Progress emitted after each successful chunk.
  - [x] 4.3 Added `StardictImportProgress::Aborted { inserted: usize }`. Mapped to `("Aborted", inserted as i32, 0)` in `stardict_progress_to_signal`; §5 will route to a dedicated `importCancelled` signal.
  - [x] 4.4 `import_stardict_as_new` now takes `cancel: &AtomicBool`. Cancel is checked between chunks; on abort it emits `Aborted` and returns `Ok(ImportOutcome { cancelled: true, .. })` WITHOUT calling `delete_dictionary_by_label`. The existing `Failed`-path cleanup calls are preserved.
  - [x] 4.5 `import_user_zip` now takes `cancel: &AtomicBool` and forwards it.
  - [x] 4.6 Option A picked: return type is `Result<ImportOutcome, String>` with `ImportOutcome { dictionary_id, inserted, cancelled }`. All call sites updated (`bridges/src/dictionary_manager.rs::import_zip`, `cli/src/bootstrap/mod.rs`, `cli/src/main.rs`).
  - [x] 4.7 Added `backend/tests/stardict_import_per_chunk_commit.rs`: builds a 2500-entry synthetic StarDict, flips the cancel flag after the 2nd committed chunk, asserts (a) `outcome.cancelled == true`, (b) `outcome.inserted == 2000`, (c) the `Aborted` progress event reports 2000, (d) 2000 `dict_words` rows persist, and (e) the parent `dictionaries` row persists so the next reconcile picks the partial import up. **Important:** the test uses a unique timestamped label per run and intentionally does NOT clean up at the end, because `dict_word_id` is UNINDEXED in `dict_words_fts` and the per-row FTS5 delete trigger turns each child delete into a full FTS scan (cleanup of 2000 rows ⇒ multiple minutes). Leaving the partial dictionary in place matches production abort behavior; manual cleanup is occasional.
  - [x] 4.8 Run `cd backend && cargo test --test stardict_import_per_chunk_commit` → passes in ~0.7s.

- [x] 5.0 Bridge + QML: import path migration
  - [x] 5.1 In `bridges/src/dictionary_manager.rs`, changed `importFinished` signal signature to `(dictionary_id: i32, label: QString, inserted_count: i32, elapsed_ms: i32)`.
  - [x] 5.2 Added new signal `importCancelled(message: QString, inserted_count: i32)` and invokable `abort_import(self: Pin<&mut DictionaryManager>)` that flips `import_cancel`.
  - [x] 5.3 Updated `fn import_zip` worker: captures `Instant::now()` at start; resets `import_cancel`; passes it to `import_user_zip`; on `Ok(ImportOutcome { cancelled: true, inserted, .. })` queues `importCancelled(message, inserted)`; on `cancelled: false` queues `importFinished(dictionary_id, label, inserted, elapsed_ms)`.
  - [x] 5.4 In `DictionariesWindow.qml`, filled in Idx 2 (import progress frame): title, stage line, counts line `Inserting words: <done> / <total>`, determinate/indeterminate progress bar, right-aligned `Abort` button calling `dict_manager.abort_import()`.
  - [x] 5.5 `onImportProgress` now drives `import_stage` / `import_done` / `import_total` and sets `import_indeterminate` from `total <= 0`.
  - [x] 5.6 `onImportFinished` uses the new 4-arg signature, switches to Idx 4 with the §4.3.5 text (now including count + elapsed). Added `onImportCancelled` → Idx 4 with the §4.3.4 partial-import text (`op_kind = "import_aborted"`).
  - [x] 5.7 `onImport_requested` now routes through a new `start_import()` helper (resets progress, switches to Idx 2, calls `import_zip`). **`onReplace_requested` required more than the task assumed:** `import_user_zip` rejects a label collision, so replace must delete-then-import. It switches to Idx 1, calls `delete_dictionary`, sets `replace_pending` + pending zip/label/lang; `onDeleteFinished` detects `replace_pending` and chains into `start_import` instead of showing the delete summary.
  - [x] 5.8 Dropped the temporary legacy import paths (TODO §5 comments removed; old `op_kind`-only finished handler replaced).
  - [x] 5.9 Updated the `DictionaryManager.qml` qmllint stub: new `importFinished` signature, `importCancelled`, `deleteFinished`, `deleteFailed` signals, and `abort_import()`. Also updated the second `onImportFinished` consumer at `DictionarySearchDictionariesPanel.qml:98` (per §1.4).
  - [x] 5.10 Run `make build -B` → successful build.

- [x] 6.0 Bridge + QML: rename path migration
  - [x] 6.1 Added `#[qsignal]`s `renameFinished(dictionary_id, old_label, new_label, elapsed_ms)` and `renameFailed(message)` in `bridges/src/dictionary_manager.rs`.
  - [x] 6.2 Rewrote `fn rename_label` (now `Pin<&mut Self>`) to spawn a worker thread mirroring `delete_dictionary`: looks up `old_label` + quick-fails a bogus id synchronously, then the worker calls `rename_user_dictionary` (which validates label collisions / busy-lock internally and routes those through `renameFailed`), captures `Instant::now()`, and queues `renameFinished` / `renameFailed`. Synchronous return is `"ok"` on kickoff.
  - [x] 6.3 Idx 3 rename progress frame was already in place from §3 (title `Renaming "<old_label>" → "<new_label>"…`, indeterminate bar, no buttons); now driven by `root.old_label` / `root.new_label`.
  - [x] 6.4/6.5 `DictionaryEditDialog` no longer calls the bridge directly. Replaced its `renamed`/`failed` signals with a single `rename_requested(dictionary_id, old_label, new_label)` emitted from `onAccepted` after validation. The window's `onRename_requested` sets `old_label`/`new_label`, switches to Idx 3, calls `dict_manager.rename_label`, and quick-fails to Idx 5. Results arrive via `onRenameFinished` / `onRenameFailed` `Connections`.
  - [x] 6.6 Idx 4 rename summary text already matches §4.4.5 (`Dictionary renamed to "<new_label>". …`); `op_label` is set to `new_label` in `onRenameFinished`. Button: `Quit`.
  - [x] 6.7 Confirmed no `error_dialog` / `restart_dialog` / `onRenamed` / `onFailed` references survive in `DictionariesWindow.qml`.
  - [x] 6.8 Run `make build -B` → successful build.

- [x] 7.0 Reconcile label reformat
  - [x] 7.1 In `reconcile_progress_to_signal`, the `IndexingDictionary` arm now formats `"Indexing: <dict_index>/<dict_total> <label>, <done>/<total> words"` (single concatenated line per §4.7.2). `done`/`total` are still passed as the integer fields driving the bar.
  - [x] 7.2 Read-through of `DictionaryIndexProgressWindow.qml`: `stage_label` already has `wrapMode: Text.Wrap` + `Layout.fillWidth: true`, so the longer string wraps on narrow widths. No QML change needed.
  - [x] 7.3 Verified: `index_dict_words_into_dict_index` (`indexer.rs:715`) emits every 1000 words plus a final `on_progress(total, total)` at `:723`; `reconcile_dict_indexes` (`dict_index_reconcile.rs:160-179`) forwards every emit plus an initial `done: 0`. Dictionaries <1000 words emit only at 0 and completion — acceptable per §4.8. No code change.

- [ ] 8.0 `DictionaryManager.qml` qmllint stub finalisation and final QA
  - [x] 8.1 `DictionaryManager.qml` stub now declares `abort_import()` and signals `deleteFinished`, `deleteFailed`, `importCancelled`, `renameFinished`, `renameFailed`, plus the updated `importFinished(dictionary_id, label, inserted_count, elapsed_ms)` signature. (`rename_label`'s QML-facing signature is unchanged.)
  - [x] 8.2 Confirmed `qmldir` needs no changes — no new QML files were added; `DictionaryManager 1.0 DictionaryManager.qml` already declared.
  - [x] 8.3 Updated [PROJECT_MAP.md](../PROJECT_MAP.md): added a "User Dictionary Management (StarDict import / delete / rename)" section documenting the StackLayout frames, worker-thread signals, import abort, FK-cascade delete, replace=delete-then-import chaining, rename, and the reconcile label format.
  - [x] 8.4 `make build -B` → fully clean build; no `qmllint` type warnings (only unrelated Qt-header SFINAE warnings from cxx-qt generated code).
  - [x] 8.5 `cargo test --no-fail-fast`: the new `stardict_import_per_chunk_commit` and `test_cascade_delete` pass. Three failures (`test_dpd_lookup_generate_json`, `test_dict_word_search_contains_match`, `test_sutta_search_contains_match`) are pre-existing DPD/search result-ordering content tests — `git diff` confirms this session changed only QML + the bridge + docs (no `backend/src` changes), so they are unrelated.
  - [ ] 8.6 Manual smoke checklist (to be exercised by the user, not the agent — per CLAUDE.md GUI-testing guidance):
    - Delete a small (~100-entry) dictionary; confirm Idx 1 → Idx 4 → Quit.
    - Delete a large (~10k-entry) dictionary; confirm Idx 1 shows the indeterminate bar for the duration of the cascade DELETE, then Idx 4 → Quit; restart app and confirm reconcile cleans up any Tantivy orphans for the deleted label via the existing `DroppingOrphans` pass.
    - Import a small StarDict; confirm Idx 2 → Idx 4 → Quit.
    - Import a large StarDict; confirm progress ticks every 1000 inserts; click Abort mid-way; restart app; confirm reconcile picks up the partial dictionary and indexes the rows already present.
    - Rename a dictionary; confirm Idx 3 → Idx 4 → Quit.
    - Confirm the OS close (×) button is ignored during Idx 1/2/3 and works in Idx 0/4/5.
    - Confirm the startup reconcile window now shows `Indexing: 1/N <label>, M/T words`.
