# Tasks: History feature for the Gloss and Prompts tabs

Source PRD: `2026-06-27-131935-prd---gloss-prompts-history.md`

## Component analysis (what blocks what)

- **DB layer** (migration + Diesel schema/model + CRUD helpers) — foundation;
  blocks the bridge.
- **Rust bridge** (shared `item_type` async save + completion signals, sync flush
  for close, list/delete/clear; replace stubs; qmllint stubs) — depends on DB
  layer; blocks both tabs.
- **Shared QML** (`HistoryListItem.qml` + shared truncate/single-line helper,
  `build.rs` registration) — depends on nothing in this feature; blocks the
  history list UI in both tabs.
- **GlossTab integration** — depends on bridge + shared QML.
- **PromptsTab integration** — depends on bridge + shared QML.
- **App-close flush** (SuttaSearchWindow `onClosing`) — depends on both tabs +
  the bridge sync flush.
- **Tests + docs** — final, depends on everything.

Each top-level task is sequenced so the app compiles and relevant tests pass at
its boundary.

## Relevant Files

- `backend/migrations/appdata/2026-06-27-131935_create_gloss_prompts_history/up.sql` —
  New migration creating the `gloss_prompts_history` table + index (mirror
  `2026-04-02-120000_create_bookmarks`).
- `backend/migrations/appdata/2026-06-27-131935_create_gloss_prompts_history/down.sql` —
  Drop table (rollback).
- `backend/src/db/appdata_schema.rs` — Add the `gloss_prompts_history`
  `diesel::table!` block.
- `backend/src/db/appdata_models.rs` — Add `GlossPromptsHistory` (Queryable) and
  `NewGlossPromptsHistory` (Insertable) structs.
- `backend/src/db/appdata.rs` — Add CRUD helpers (`get_history_for_type`,
  `save_new_history`, `update_history`, `delete_history_item`, `clear_history`)
  using the `do_read`/`do_write` pattern. No per-save `ANALYZE` (see task 1.7).
- `bridges/src/sutta_bridge.rs` — Replace the gloss stubs (`:2515`) with real
  shared `item_type` functions; add async background save + `#[qsignal]`
  completion signals, a synchronous flush for close, and list/delete/clear.
- `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml` — Add/repair qmllint
  stubs for every new/changed bridge function and signal.
- `assets/qml/HistoryListItem.qml` — New shared list-item component (Open/Delete
  buttons, single-line truncated label, click-to-select), mirroring
  `BookmarkListItem.qml`.
- `assets/qml/HistoryUtils.qml` (or a JS helper) — Shared `single_line_truncate`
  / formatting helper usable by both tabs.
- `bridges/build.rs` — Register `HistoryListItem.qml` (and any shared helper qml)
  in `qml_files`.
- `assets/qml/GlossTab.qml` — Extend session lifecycle + history UI (existing
  `current_session_id`/`history_model`/`load_history`/`save_session`/`load_session`).
- `assets/qml/PromptsTab.qml` — Add the parallel session lifecycle + history UI +
  `messages_model` serialize/restore.
- `assets/qml/SuttaSearchWindow.qml` — Hook the blocking app-close flush into the
  existing `onClosing` (`:19`); hosts `gloss_tab` (`:3559`) and `prompts_tab`
  (`:3576`).
- `backend/src/db/appdata.rs` (tests) — Rust unit/integration tests for the new
  CRUD helpers against the real appdata DB.
- `PROJECT_MAP.md`, `docs/user-data-and-sqlite-analyze.md`,
  `docs/startup-sequence-and-caches.md` — Documentation updates.

### Notes

- Async pattern to mirror: `thread::spawn(move || { …; qt_thread.queue(move |mut
  qo| { qo.as_mut().emit_signal(...) }) })` as in `load_db` / `load_searcher`
  (`sutta_bridge.rs:1437`), with `#[qsignal]` + `#[cxx_name]` declarations
  (`:597`).
- CRUD pattern to mirror: `create_bookmark_item` / `delete_bookmark_item` in
  `backend/src/db/appdata.rs:1698`/`:1788` (`do_read`/`do_write` closures).
- Per repo rules: build with `make build -B`; run backend tests with
  `cd backend && cargo test`; do **not** run `make qml-test` unless asked; only
  run tests after all sub-tasks of a top-level task are done; skip tests for
  docs-only changes. `CLAUDE.md` is a symlink — edit `AGENTS.md`.
- FTS note does not apply (this is a plain table, not an FTS5 virtual table).

## Instructions for Completing Tasks

As each sub-task is completed, change `- [ ]` to `- [x]` in this file, updating
after each sub-task (not just each parent task).

## Tasks

### 1.0 Backend DB layer (PRD reqs 1–4, 6) ✅

**Spec:** table `gloss_prompts_history` with `id` INTEGER PK, `item_type` VARCHAR
NOT NULL (`"gloss"`|`"prompts"`), `data_json` TEXT NOT NULL, `created_at`
DATETIME DEFAULT CURRENT_TIMESTAMP, `updated_at` DATETIME. Composite index on
`(item_type, updated_at)` for `WHERE item_type = ? ORDER BY updated_at DESC`. No
retention cap. `data_json` is opaque text to the backend.
**Depends on:** existing appdata DB infra (`do_read`/`do_write`, bookmarks
precedent).

- [x] 1.1 Create the migration folder
      `backend/migrations/appdata/2026-06-27-131935_create_gloss_prompts_history/`
      with `up.sql` (CREATE TABLE + `CREATE INDEX IF NOT EXISTS
      idx_gloss_prompts_history_type_updated ON gloss_prompts_history(item_type,
      updated_at)`) and `down.sql` (DROP TABLE), following the bookmarks migration.
- [x] 1.2 Add the `gloss_prompts_history` `diesel::table!` block to
      `backend/src/db/appdata_schema.rs`.
- [x] 1.3 Add `GlossPromptsHistory` (Queryable/Selectable) and
      `NewGlossPromptsHistory<'a>` (Insertable) structs to
      `backend/src/db/appdata_models.rs`.
- [x] 1.4 Add `get_history_for_type(item_type) -> Vec<GlossPromptsHistory>`
      (ordered `updated_at DESC`) to `backend/src/db/appdata.rs`.
- [x] 1.5 Add `save_new_history(item_type, data_json) -> Result<i32>` (INSERT,
      sets `created_at`/`updated_at`, returns new id) and
      `update_history(id, data_json) -> Result<()>` (UPDATE `data_json` +
      `updated_at`).
- [x] 1.6 Add `delete_history_item(id) -> Result<()>` and
      `clear_history(item_type) -> Result<()>`.
- [x] 1.7 **Do NOT add a per-save `ANALYZE`.** `DatabaseHandle::analyze`
      (`backend/src/db/mod.rs:116`) runs a full-DB `ANALYZE;` over all appdata
      tables — wasteful per 60 s save and unnecessary here: the only query is a
      single-table equality + order served by the `(item_type, updated_at)` index
      (the slow-query bug in `docs/user-data-and-sqlite-analyze.md` was a
      multi-table join, not this shape). Document the decision (code comment +
      docs in 7.5) rather than calling `analyze`.
- [x] 1.8 Apply the migration to the dev DB and confirm the backend compiles
      (`cd backend && cargo build`); add CRUD tests in task 7.0.

### 2.0 Rust bridge: shared `item_type` history functions (PRD reqs 5, 10a, 17-backend) ✅

**Spec / API surface** (all `item_type`-parameterised; reuse for both tabs):
- `get_history_json_background(item_type)` → emits `historyListReady(item_type,
  json)` — `json` is `[{id, modified, data}]` newest-first (matches GlossTab
  `load_history` shape at `:540`).
- `save_history_session_background(item_type, session_id, data_json)` — INSERT if
  `session_id` empty else UPDATE; emits `historySaved(item_type, session_id)`
  with the resolved id (new id on INSERT) so the tab can store
  `current_session_id`.
- `save_history_session_blocking(item_type, session_id, data_json) -> QString` —
  synchronous variant returning the resolved id, **for the app-close flush only**
  (req 17).
- `delete_history_item(item_type, id)` and `clear_history(item_type)` — run off
  the UI thread; emit `historyChanged(item_type)` so the tab reloads its list.
**Depends on:** task 1.0 CRUD helpers; the `qt_thread`/`#[qsignal]` async pattern.

- [x] 2.1 In the `extern` block of `bridges/src/sutta_bridge.rs`, declare the new
      `#[qsignal]`s (`historyListReady`, `historySaved`, `historyChanged`, each
      with `item_type` + payload) with `#[cxx_name]` camelCase names, mirroring
      `:597`.
- [x] 2.2 Declare the new functions in the `extern` block (the four background
      fns + the blocking flush), removing/repurposing the three gloss stubs
      (`get_gloss_history_json`, `save_new_gloss_session`, `update_gloss_session`).
- [x] 2.3 Implement `get_history_json_background` using `thread::spawn` +
      `qt_thread.queue` → `emit_history_list_ready`, building the JSON array from
      `get_history_for_type` (id, `updated_at` as `modified`, `data_json` as
      `data`).
- [x] 2.4 Implement `save_history_session_background`: skip empty `data_json`
      sessions (guard mirrors req 15, though the QML also guards); INSERT vs
      UPDATE by `session_id`; emit `historySaved(item_type, resolved_id)`.
      **ID-type contract:** `session_id` is a `QString` (`""` = INSERT, otherwise
      UPDATE by parsed `i32`), and the emitted `resolved_id` is a `QString` too,
      so QML keeps `current_session_id` as a `string` end-to-end (GlossTab's
      `current_session_id` is `property string`, `:219`). Parse to `i32` only
      inside Rust.
- [x] 2.5 Implement `save_history_session_blocking(item_type, session_id,
      data_json) -> QString` (synchronous INSERT/UPDATE, returns the resolved id
      as a `QString`) for the close path; same skip-empty + ID-type contract.
- [x] 2.6 Implement `delete_history_item` and `clear_history` as background ops
      emitting `historyChanged(item_type)`.
- [x] 2.7 Update the GlossTab callers that referenced the old stub names
      (`get_gloss_history_json`/`save_new_gloss_session`/`update_gloss_session` at
      GlossTab `:536`/`:579`/`:581`) so the project still compiles after the
      rename — full rewiring happens in task 4.0, but keep the build green here.
- [x] 2.8 Add matching `qmllint` stubs for every new function and signal to
      `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml` (signals as
      functions/properties per existing convention); remove the obsolete
      `get_gloss_history_json` stub (`:425`).
- [x] 2.9 Confirm the project builds (`make build -B`).

### 3.0 Shared QML: list-item component + helper (PRD reqs 21–23, 25, 26) ✅

**Spec:** `HistoryListItem.qml` mirrors `BookmarkListItem.qml` layout but
simplified — no drag/checkbox/badge. Shows a single-line, ~80-char-truncated
label derived from the session's input text; an **Open** button (text) and a
**Delete** icon button (`ion--trash-outline.png`); row MouseArea emits a
`select` signal (highlight only, no load). Signals: `open_clicked(item_data)`,
`delete_clicked(id)`, `select_clicked(id)`. Selected/hover highlight via a
background `Rectangle`.
**Depends on:** nothing in this feature (pure QML).

- [x] 3.1 Create `assets/qml/HistoryListItem.qml` with `required property var
      item_data`, the truncated single-line label, Open/Delete buttons, and the
      row-select MouseArea + selection highlight; use `Logger { id: logger }` (no
      `console`).
- [x] 3.2 Create the shared single-line/truncate helper (e.g.
      `assets/qml/HistoryUtils.qml` exposing
      `single_line_truncate(text, max_len)` that collapses
      newlines/whitespace runs to single spaces and truncates to ~80 chars with
      an ellipsis) so both tabs and the delegate reuse one implementation.
- [x] 3.3 Register `HistoryListItem.qml` (and the helper qml, if a Component) in
      the `qml_files` list in `bridges/build.rs` (near `BookmarkListItem.qml`,
      `:74`).
- [x] 3.4 Confirm the project builds (`make build -B`).

### 4.0 GlossTab integration (PRD reqs 7–16, 18-gloss, 19–24) ✅

**Spec — session lifecycle:** add `property bool session_needs_saving: false`;
change events set it true (do **not** save synchronously); a `Timer`
(`interval: 60000`, `repeat: true`) flushes via the async bridge when dirty and
no write is in flight; clear the flag on the `historySaved` signal. **Top
toolbar:** New Session (with confirm dialog) + Save (disabled when clean) + a
save-state indicator Label bound to `session_needs_saving`. **Full restore (req
18):** extend `save_session()`/`load_session()` to round-trip
`translations_json`, `selected_ai_tab`, `no_duplicates_globally`, `skip_common`,
and the global dedup/unrecognized state. **History UI:** real `ListView` of
`HistoryListItem` with Clear (confirm dialog) at top, click-to-select, Open
(flush-then-load), Delete; refresh list only on tab-show / Save / New Session /
Open / Delete / Clear (not autosave).
**Depends on:** tasks 2.0 (bridge) and 3.0 (shared QML).

- [x] 4.1 Add `session_needs_saving` and a `selected_history_id` property; add the
      60 s autosave `Timer` that calls the async save when dirty (guard against
      overlapping in-flight writes) and a `Connections` to `historySaved`
      (filter `item_type === "gloss"`) that sets `current_session_id` and clears
      the flag.
- [x] 4.2 Refactor the eager-save change points to mark-dirty only:
      `update_paragraph_text` (`:931`), `update_word_selection` (`:909`), and the
      post-gloss handlers (`:763`, `:795`) set `session_needs_saving = true`
      instead of calling `save_session()`.
- [x] 4.3 Rework `save_session()` to (a) build the **full** `gloss_data`
      including per-paragraph `translations_json` + `selected_ai_tab` + options +
      global state, (b) call the async (or, on close, blocking) bridge save, and
      (c) **not** call `load_history()` (remove `:584`). Skip empty sessions
      (req 15).
- [x] 4.4 Rework `load_session()` to restore the full set — stop hard-coding
      `translations_json: "[]"` (`:895`); repopulate translations, selected AI
      tab, options, and global state so the UI matches the saved state.
- [x] 4.5 Add a `new_session()` function (flush-if-dirty, reset
      `current_session_id`, clear input/`paragraph_model`/global state, clear
      flag) and a **New Session** button with a confirmation dialog (req 14).
- [x] 4.6 Add a **Save** button (kicks off the async save immediately; disabled
      when `!session_needs_saving`) and a save-state indicator Label bound to
      `session_needs_saving`, placed in a top toolbar row.
- [x] 4.7 Replace the History sub-tab placeholder (`:1538`) with a real
      `ListView` (`model: history_model`, `delegate: HistoryListItem`) and a
      **Clear** button (with confirm dialog) above it; wire `select_clicked` →
      set `selected_history_id`, `open_clicked` → flush-then-`load_session`,
      `delete_clicked` → bridge delete.
- [x] 4.8 Update `load_history()` to use the async list bridge: trigger
      `get_history_json_background("gloss")` and populate `history_model` from the
      `historyListReady` signal (filter `item_type === "gloss"`); refresh after
      Save/New Session/Open/Delete/Clear (via `historyChanged`), not on autosave.
      **Add the missing activation trigger:** the `tabBar` has no
      `onCurrentIndexChanged` today (`:1378`), so add one (or the History page's
      `onVisibleChanged`) to reload the list when the History sub-tab becomes
      visible — otherwise it only updates after a write and looks stale.
- [x] 4.9 Confirm the project builds (`make build -B`) and the Gloss flow works
      (manual check by user if needed per repo GUI-testing guidance).

### 5.0 PromptsTab integration (PRD reqs 7–16, 18-prompts, 19–24) ✅

**Spec:** the same lifecycle + history UI as GlossTab, parameterised with
`item_type = "prompts"`. **Serialize:** `save_session()` builds
`{ messages: [ { role, content, content_html, responses_json, selected_ai_tab }
… ] }` from `messages_model` (`:239`). **Restore:** `load_session()` clears and
rebuilds `messages_model` from that array (reuse the `init_messages` pattern at
`:265`). Transient UI state (scroll, in-flight, `is_collapsed`) is not persisted.
Change points that mark dirty: appending a user message, receiving responses
(`:100`, `:182`), editing a user/system message, regenerate/branch (`:920`).
**Depends on:** tasks 2.0 and 3.0; reuse the shared helper + `HistoryListItem`.

**IMPORTANT — replicate the GlossTab session-lifecycle gotchas (PRD §10).** The
GlossTab build hardened the shared lifecycle; PromptsTab must mirror **all** of
it, not just the happy path. Concretely (see PRD §10 for the why):
- State machine: `session_needs_saving`, `save_in_flight`, `save_again_pending`,
  `refresh_list_on_save`, `current_session_id` (string, `""` = no row yet).
- `save_session(blocking)` is **single-writer**: async path returns early setting
  `save_again_pending = true` when `save_in_flight`; the `blocking === true`
  branch is the synchronous close/flush variant (PRD §10.3, §10.4).
- `onHistorySaved` (filtered `item_type === "prompts"`): reset `save_in_flight`;
  **empty `session_id` = failure** → keep dirty, don't clobber id, return;
  otherwise set id, clear dirty, refresh list iff `refresh_list_on_save`, then run
  the coalesced `save_again_pending` save (PRD §10.2, §10.3).
- Reset `current_session_id = ""` on **Clear**, and on **Delete** of the active
  session (PRD §10.1 — backend INSERT-fallback already backs this up).
- `load_session()` sets `session_needs_saving = false` **last** (after rebuilding
  `messages_model`); guard any delegate change handlers so they don't mark dirty
  during restore (PRD §10.5).
- Use the **blocking** flush for New Session / Open (PRD §10.4).
The cleanest path is to copy GlossTab's `save_session` / `save_session_now` /
`flush_if_needed` / `onHistorySaved` shape verbatim and swap the serialize /
restore bodies + `"gloss"` → `"prompts"`.

- [x] 5.1 Add `current_session_id`, `session_needs_saving`, `save_in_flight`,
      `save_again_pending`, `refresh_list_on_save`, `selected_history_id`,
      `ListModel { id: history_model }`, the 60 s autosave `Timer` (tick gated on
      `session_needs_saving && !save_in_flight`), and a `Connections` to
      `historySaved`/`historyChanged`/`historyListReady` filtered on
      `item_type === "prompts"` (with the empty-id failure handling per PRD §10.2).
- [x] 5.2 Implement `save_session(blocking)` (single-writer guard + coalesce per
      PRD §10.3; `blocking` close/flush variant per §10.4; `is_session_empty()`
      skip; no list reload on autosave), `save_session_now()` (sets
      `refresh_list_on_save`), `flush_if_needed()` (blocking), and `load_session()`
      (rebuild `messages_model`, set `session_needs_saving = false` last per
      §10.5); add a `session_data_json()` helper. **Normalize in-flight responses
      on serialize:** responses arrive asynchronously via
      `onPromptResponseForMessages` (`:38`) and `responses_json` carries a per-
      response `status` (`pending`/`completed`/`error`); since in-flight state is
      not persisted, any non-terminal (`pending`) response must be dropped or
      marked interrupted at save time so a restored conversation has no "zombie
      spinner" (PRD req 18).
- [x] 5.3 Mark dirty at the conversation change points (message append, responses
      received, message edit, regenerate/branch) instead of any direct save —
      including edits to the user/system message text fields (PRD §10.6, the
      Prompts analogue of GlossTab's main-input `onTextChanged`).
- [x] 5.4 Add a `new_session()` (flush-if-dirty via the **blocking** flush, clear
      conversation back to the system + empty user message, reset
      id/flag/`selected_history_id`, then `load_history()`) and a **New Session**
      button with a confirmation dialog. **External entry (PRD §10.7):** route
      `SuttaSearchWindow.new_prompt` → `PromptsTab.new_prompt` through the same
      confirm/new-session path when a non-empty conversation is in progress, and
      detach `current_session_id` when starting fresh from an empty-but-loaded
      session, so it never overwrites the previously opened session's row.
- [x] 5.5 Add the **Save** button + save-state indicator (bound to
      `session_needs_saving`) in a top toolbar row, matching GlossTab.
- [x] 5.6 Replace the History sub-tab placeholder (`:617`) with the real
      `ListView` of `HistoryListItem` + **Clear** button (confirm dialog); wire
      select / open (flush-then-load) / delete. **Reset `current_session_id = ""`
      on Clear, and on Delete of the active session** (PRD §10.1).
- [x] 5.7 Implement `load_history()` via `get_history_json_background("prompts")`
      + `historyListReady` (filter `item_type === "prompts"`); refresh after
      Save/New Session/Open/Delete/Clear. **Add the activation trigger:** the
      `tabBar` has no `onCurrentIndexChanged` today (`:540`), so add one (or the
      History page's `onVisibleChanged`) to reload when the History sub-tab
      becomes visible.
- [x] 5.8 Confirm the project builds (`make build -B`).

### 6.0 App-close flush (PRD req 17) ✅

**Spec:** on app/tab close, for each tab with `session_needs_saving === true` and
a non-empty session, perform a **blocking** save (`save_history_session_blocking`)
so it completes before the process exits. Hook into the existing
`SuttaSearchWindow.qml` `onClosing` (`:19`), which already hosts `gloss_tab`
(`:3559`) and `prompts_tab` (`:3576`).
**Depends on:** tasks 4.0, 5.0 (tabs expose a flush function) and 2.5 (blocking
bridge fn).

- [x] 6.1 Ensure `flush_if_needed()` exists on both tabs: when
      `session_needs_saving` and the session is non-empty, it calls
      `save_session(true)` (the **blocking** variant, which clears the flag and
      `save_again_pending`). It must be idempotent (safe to call twice / when
      already clean). **Already implemented on GlossTab in task 4.x**; PromptsTab
      gets it in task 5.2. Do not duplicate the blocking bridge call at the call
      sites — go through `flush_if_needed()` (PRD §10.4).
- [x] 6.2 Call `gloss_tab.flush_if_needed()` and `prompts_tab.flush_if_needed()`
      from `SuttaSearchWindow.qml` `onClosing` (`:19`). **Mobile caveat:** the
      handler sets `close.accepted = false` on mobile (`:20-23`), so there is no
      guaranteed real-exit hook — run the flush on **both** branches (a redundant
      save is harmless) and add `Component.onDestruction` flushes on `gloss_tab`
      and `prompts_tab` as a backstop.
- [x] 6.3 Confirm the project builds and that quitting right after an edit
      preserves the session (manual verification).

### 7.0 Tests, verification, and documentation

- [ ] 7.1 Add Rust tests in `backend/src/db/appdata.rs` (or the crate's test
      module) for the CRUD helpers against the real appdata DB: save-new returns
      id, update changes `data_json`/`updated_at`, list is newest-first and
      `item_type`-scoped, delete-one and clear-all, and that `"gloss"` vs
      `"prompts"` rows don't cross-contaminate. **Also cover the stale-row contract
      (PRD §10.1):** `update_history` returns `0` for a deleted/non-existent id
      (so the bridge's INSERT-fallback triggers), and a save against an id removed
      by `clear_history` re-creates a row rather than vanishing.
- [ ] 7.2 Run `cd backend && cargo test` and confirm the new tests pass (ignore
      unrelated pre-existing failures per repo guidance).
- [ ] 7.3 Run `make build -B` for a clean full build.
- [ ] 7.4 Update `PROJECT_MAP.md` with the new table, bridge functions, shared
      QML component, and the session-lifecycle additions to both tabs.
- [ ] 7.5 Update `docs/user-data-and-sqlite-analyze.md` (new runtime-growing
      table + the **no per-save `ANALYZE`** decision from task 1.7) and note the
      history feature in `docs/startup-sequence-and-caches.md` if relevant. **Add a
      short feature doc** (e.g. `docs/gloss-prompts-history.md`) capturing the
      shared session-lifecycle state machine and the gotchas/fixes from **PRD
      §10** (single-writer + coalesce, empty-id failure contract, stale-row
      INSERT-fallback, blocking flush rationale, spurious-dirty-on-load), since the
      PRD/tasks files are transient and the gotchas are load-bearing for future
      edits to either tab. Link it from `CLAUDE.md`/`AGENTS.md` notable-docs list.
