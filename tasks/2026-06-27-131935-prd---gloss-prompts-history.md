# PRD: History feature for the Gloss and Prompts tabs

## 1. Introduction/Overview

The **Gloss** tab (`assets/qml/GlossTab.qml`) and the **Prompts** tab
(`assets/qml/PromptsTab.qml`) both already have a "History" sub-tab containing a
placeholder (a commented-out `ListView` / `history_model`). This feature
implements that history: a persisted list of the user's recent sessions for each
tab, so a user can return to earlier work — re-open a previous gloss session or a
previous AI chat conversation — instead of losing it when they start something
new.

The two histories are almost identical in behaviour, so the implementation must
**share** the database table, the Rust helper logic, and the QML list/item UI as
much as possible, distinguishing the two only by an `item_type` field.

The goal is a working, persistent, shared history for both tabs that records full
sessions automatically, lists them most-recent-first, and lets the user open or
delete individual sessions or clear all of them.

## 2. Goals

1. Persist each tab's sessions in a single shared database table, distinguished
   by an `item_type` field (`"gloss"` vs `"prompts"`).
2. Automatically capture the **full session/conversation** state (not just the
   input text), with **no data loss** — the saved data must be sufficient to
   restore the UI to the exact state the user left it.
3. **Never lose unsaved work on quit** — sessions with pending changes are saved
   when the app/tab closes.
4. Show, per tab, a list of saved sessions sorted **most-recently-modified
   first**, each rendered as a single truncated line derived from the input
   text.
5. Let the user **Open** a session (load it back into the tab), **Delete** a
   single session, **Clear** all sessions for that tab, and **select** an item by
   clicking it (selection only — no load).
6. Add **"New Session"** and **"Save"** controls plus a save-state indicator so
   the user can explicitly start fresh, force a save, and always see whether the
   current session has unsaved changes (`session_needs_saving`).
7. Keep saving **off the typing-latency path** — change events only mark the
   session dirty; writes are debounced — so editing never causes UI lag.
8. Maximise code reuse between the two tabs (one DB table, one set of Rust bridge
   functions, one shared QML list-item component and helper utilities).

## 3. User Stories

- **As a user of the Gloss tab**, I want my glossing sessions saved
  automatically, so that when I come back later I can re-open a paragraph set I
  glossed before, with its words still resolved.
- **As a user of the Prompts tab**, I want my past AI conversations saved, so I
  can re-open an earlier chat and continue or review it.
- **As a user**, I want to click "New Session" to deliberately start fresh
  without overwriting my previous session.
- **As a user**, I want a "Save" button and a visible "saved / unsaved changes"
  marker, so I know my work is persisted and can force a save when I want.
- **As a user editing a glossed paragraph**, I don't want the app to stutter on
  every keystroke, so saving should happen in the background, not as I type.
- **As a user**, I want to see my recent sessions as short, scannable one-line
  summaries, ordered with the most recent at the top.
- **As a user**, I want to delete a single old session, or clear the whole
  history, to keep the list tidy.
- **As a user**, I want clicking an item to just highlight/select it (so I can
  see which one I mean) without immediately replacing my current work — I open it
  explicitly with the Open button.

## 4. Functional Requirements

### Data model & persistence

1. The system **must** create one shared appdata table (working name:
   `gloss_prompts_history`) via a new Diesel migration under
   `backend/migrations/appdata/`, following the existing bookmarks migration
   pattern (`2026-04-02-120000_create_bookmarks`).
2. The table **must** include at least: `id` (PK), `item_type` VARCHAR NOT NULL
   (`"gloss"` | `"prompts"`), `data_json` TEXT NOT NULL (the full serialized
   session/conversation), `created_at` DATETIME, and `updated_at` DATETIME.
3. The table **must** be indexed for the primary query pattern
   (`WHERE item_type = ? ORDER BY updated_at DESC`), e.g. a composite index on
   `(item_type, updated_at)`.
4. A corresponding Diesel model and schema entry **must** be added in
   `backend/src/db/appdata_models.rs` and `appdata_schema.rs`, and query helpers
   in `backend/src/db/appdata.rs`.
5. The system **must** expose shared, `item_type`-parameterised functions on the
   Rust bridge (`bridges/src/sutta_bridge.rs`) and replace the existing gloss
   stubs (`get_gloss_history_json`, `save_new_gloss_session`,
   `update_gloss_session` currently return placeholders). The set **must** cover:
   list history (returns JSON array, newest first), save new session (returns new
   id/uid), update existing session by id, delete one by id, and clear all for an
   item_type. Each bridge function declared in the `extern` block **must** also
   get a matching stub in `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml`
   for `qmllint`.
6. There **must be no retention cap** — sessions persist until removed by Delete
   or Clear.

### Saving / session lifecycle

7. Saving is **driven by the current session**, not by run/send. Each tab tracks
   a `current_session_id` (already present in GlossTab as
   `root.current_session_id`).
8. Each tab **must** track a boolean property named **`session_needs_saving`**
   (initialised `false`) that represents whether the current session has unsaved
   changes. This is the single source of truth for both the autosave decision and
   the save-state indicator (requirement 13).
9. **Change events must only mark the session dirty, not write immediately.**
   Every place that currently calls `save_session()` directly on a change
   (GlossTab `update_paragraph_text` on every keystroke at `:931`,
   `update_word_selection` at `:909`, and the post-gloss handlers at `:763`,
   `:795`) — and the equivalent change points added to PromptsTab — **must
   instead set `session_needs_saving = true`** and let the debounced timer (next
   requirement) perform the actual write. This removes the per-keystroke
   serialize + synchronous SQLite round-trips from the typing-latency path.
10. The system **must** debounce autosave with a **`Timer`** (interval **60000
    ms**, repeating). On each tick, **if `session_needs_saving` is true** it
    performs one `save_session()` write (UPDATE existing row, or INSERT if no
    `current_session_id` yet) and then sets `session_needs_saving = false`. If the
    flag is false the tick is a no-op (no DB access).
10a. **All `save_session()` DB writes (and the history list/delete/clear reads
    and writes) must run as background operations off the UI thread**, never as
    synchronous blocking calls on the QML/render thread. The Rust bridge save
    functions must offload the SQLite work to a background thread/worker and
    signal completion back to QML (mirroring the existing background pattern used
    by `process_all_paragraphs_background` / `process_paragraph_background`), so
    no save ever blocks rendering even on a large session. `current_session_id`
    for a freshly INSERTed session is returned via the completion signal/callback,
    not a synchronous return value. The autosave flag should only be cleared on
    successful completion; if a write is still in flight when the next tick fires,
    do not start an overlapping write.
11. `save_session()` **must not** call `load_history()` (the current
    save-then-reload-the-whole-list behaviour at GlossTab `:584`). The history
    `ListModel` **must** be refreshed only when the History sub-tab becomes
    visible and after New Session / Save / Open / Delete / Clear — never on the
    autosave write path.
12. The system **must** provide an explicit **"Save"** button at the top of the
    Gloss and Prompts working area. Clicking it **immediately triggers** the
    background write (same async path as the timer, requirement 10a); on
    completion `session_needs_saving` is set to `false` and the history list is
    refreshed. The button **should** be disabled (or visually inert) when
    `session_needs_saving` is false. "Immediately" means the write is *kicked off*
    at once (not deferred to the next tick); it still runs off the UI thread.
13. The system **must** show a **save-state indicator at the top** of each tab's
    working area, driven by `session_needs_saving`: a clear "saved" state when
    `false` (e.g. "Saved" / a check) and a "needs saving" / "unsaved changes"
    state when `true`. The indicator must update reactively as the flag changes.
14. The system **must** provide a **"New Session"** button at the top of the
    Gloss and Prompts working area. Clicking it **must first warn the user with a
    confirmation dialog** (e.g. "Start a new session? The current session will be
    saved to history."). On confirm, it **flushes** the current session if
    `session_needs_saving` is true (so nothing is lost), then starts a fresh
    session: `current_session_id` reset to empty, input/paragraphs (Gloss) or
    conversation (Prompts) cleared, and `session_needs_saving` reset to `false`.
    The next change marks it dirty again, and the next save INSERTs a new row.
15. The autosave/Save write **must skip genuinely empty sessions** (no input
    text / whitespace-only, no paragraphs or conversation content) so the history
    is not polluted with blank rows; an empty session leaves `current_session_id`
    unset.
16. Opening a history item (requirement 22) **must first flush** any pending
    changes to the *current* session (if `session_needs_saving`) before loading
    the selected session, so switching sessions never drops unsaved work.
17. **On application/tab close, any unsaved session must be saved.** When the app
    is closing (or the tab is being torn down), if `session_needs_saving` is true
    the system **must** perform a final flush so the user never loses work by
    quitting before the 60 s timer fires. This must hook into the existing
    app-shutdown path (see Technical Considerations) and skip empty sessions per
    requirement 15. Normal saves are background/async (requirement 10a); the
    close path is the **one** place where the flush must be guaranteed to
    **complete before the process exits** — implement this by running the
    close-time write to completion on the shutdown path (a short blocking wait at
    quit is acceptable since the app is already terminating), rather than firing
    an async write that the exiting process may abandon.
18. **Session restore must not lose data — the saved `data_json` must be
    sufficient to restore the UI to the exact state the user left it.** Opening a
    session (or restoring on next launch) must reproduce the working area as it
    was at save time, not a degraded approximation. Concretely the `data_json`
    **must** persist *all* state that affects what the user sees and can continue
    editing:
    - **Gloss:** input text; every paragraph's text; the glossed words per
      paragraph (lookup results **and** the user's selected result/stem per word);
      the per-paragraph **AI translations** (`translations_json`) and the
      per-paragraph selected AI tab (`selected_ai_tab`); and the session options
      that change behaviour/appearance (`no_duplicates_globally`, `skip_common`,
      and the global dedup/unrecognized-word state such as `global_shown_stems` /
      `global_unrecognized_words` / `paragraph_unrecognized_words` to the extent
      they affect re-display). The current `save_session()` shape is a *subset*
      and **must be extended**: it omits `translations_json`/`selected_ai_tab` and
      the global state, and `load_session()` hard-codes `translations_json: "[]"`
      (`:895`) — both must round-trip the full set so nothing is lost.
    - **Prompts:** the full conversation needed to rebuild `messages_model`
      exactly — every message's `{ role, content, content_html, responses_json,
      selected_ai_tab }` in order, including the system prompt and each model's
      responses (see PromptsTab `:239`). These fields are confirmed sufficient
      (message delegate `:636-640`); transient UI state — scroll position,
      in-flight request state, the local `is_collapsed` toggle — is intentionally
      **not** persisted. **In-flight responses must be normalized on save:**
      `responses_json` carries a per-response `status`
      (`pending`/`completed`/`error`); since in-flight state is not persisted, any
      non-terminal (`pending`) response **must** be normalized at serialize time
      (dropped or marked interrupted) so a restored conversation never shows a
      request that will never complete (no "zombie spinner").
    - If any field needed for faithful restore is discovered during
      implementation that is not in the lists above, it **must** be added to
      `data_json` rather than dropped — "no data loss" is the governing rule.

### History list UI (shared)

19. Each tab's "History" sub-tab **must** replace its placeholder with a real
    `ListView` bound to the tab's `history_model`, populated from the list bridge
    function.
20. At the **top of the list** there **must** be a **"Clear"** button that
    removes all history items for that tab's `item_type` (after a confirmation
    prompt), then refreshes the list.
21. Each list item **must** display the session's input text as a **single
    line** — newlines and runs of whitespace collapsed to single spaces — and
    **truncated to ~80 characters** with an ellipsis.
22. Each list item **must** show two action buttons, styled consistently with
    `BookmarkListItem.qml`:
    - **Open** — loads/restores that session into the tab (sets
      `current_session_id`, repopulates input/paragraphs/conversation).
    - **Delete** — removes that single session (then refreshes the list).
    - *(No Edit button — editing a session is done by opening it and changing the
      loaded text in the UI.)*
23. **Clicking a list item (the row itself)** **must only select/highlight** it —
    it must **not** load the session into the tab. Loading happens only via the
    Open button. (Open is gated by the flush in requirement 16.)
24. The list **must** be ordered most-recently-modified first.
25. A shared QML list-item component (working name: `HistoryListItem.qml`)
    **must** be created and used by both tabs, and registered in the `qml_files`
    list in `bridges/build.rs`. Any new shared helper functions for
    truncation/single-line formatting should live where both tabs can reuse them
    (avoid duplicating the logic in each tab).

### Code-reuse requirement

26. The DB table, Rust bridge functions, and QML list/item UI **must** be shared
    between the two tabs, parameterised by `item_type`. New tab-specific code
    should be limited to serializing/deserializing that tab's session shape and
    wiring its input fields.

## 5. Non-Goals (Out of Scope)

- No per-item **Edit** button / inline text editing of a stored session.
- No retention cap, auto-pruning, or "history size limit" setting.
- No search, filtering, tagging, or folders within history.
- No drag-and-drop reordering (unlike bookmarks; order is by recency only).
- No cross-device sync; storage is the local appdata DB only.
- No multi-select / bulk delete beyond the single "Clear all" button.
- No export/import of history.

## 6. Design Considerations

- Mirror `assets/qml/BookmarkListItem.qml` for the item layout and the
  Open/Delete button styling (icon buttons: `fa_pen-to-square-solid.png` is the
  edit icon — not used here; `ion--trash-outline.png` for delete; a text "Open"
  button). Use selection highlighting on the row for the click-to-select state.
- The "New Session", "Save", and "Clear" buttons should match existing button
  styling in the respective tabs.
- The save-state indicator at the top should be lightweight and reactive to
  `session_needs_saving` — e.g. a small Label that reads "Saved" vs "Unsaved
  changes" (optionally with an icon/colour). Keep it unobtrusive; it sits next to
  the New Session / Save controls.
- Follow project QML conventions: `Logger { id: logger }` for logging (no
  `console` API), single-string log messages, snake_case ids/properties.
- The "History" sub-tab already exists in both files (GlossTab around line
  1538–1545, PromptsTab around line 617–626); replace the placeholder there.

## 7. Technical Considerations

### Groundwork inventory — what exists vs. what is missing

From a review of the current code, here is the precise gap to close.

**GlossTab (`assets/qml/GlossTab.qml`) — exists today (extend, don't rewrite):**
`current_session_id` (`:219`), `history_model` (`:233`), `load_history()`
(`:534`), `save_session()` (`:553`), `load_session()` — the Open/restore path
(`:877`); plus the (stub) bridge calls. **Missing and to be added in GlossTab:**

- `session_needs_saving` property + the change-event → mark-dirty refactor
  (req 9).
- A 60 s autosave `Timer` (req 10).
- `new_session()` function (reset id/input/paragraphs/flag) + a **New Session**
  button (req 14).
- An explicit flush/`save_session()`-now path bound to a **Save** button
  (req 12).
- A save-state indicator bound to `session_needs_saving` (req 13).
- `delete_history_item(id)` and `clear_history()` functions + the **Delete** /
  **Clear** UI (reqs 20, 22).
- A `selected_history_id` (or equivalent) for click-to-select (req 23).
- The real History `ListView` + delegate replacing the placeholder (`:1538`).
- Extend `save_session()`/`load_session()` to round-trip the **full** restore set
  (req 18: translations, selected_ai_tab, options, global state).
- Remove `load_history()` from `save_session()` (`:584`).

**PromptsTab (`assets/qml/PromptsTab.qml`) — almost all missing (build new):**
only the serializable `messages_model` shape exists (`:239`) and
`new_prompt()`/`init_messages()` (`:265`) show how to rebuild a conversation.
**Add:** `current_session_id`, `session_needs_saving`, `history_model`,
`load_history()`, `save_session()` (serialize `messages_model`), `load_session()`
(rebuild `messages_model`), `new_session()`, the autosave `Timer`, the
Save/New Session/Clear/Delete UI + indicator, and the real History `ListView`
(placeholder at `:617`).

**Rust bridge (`bridges/src/sutta_bridge.rs:2515`) — all stubs / missing:**
`get_gloss_history_json` → `"[]"`, `update_gloss_session` → no-op,
`save_new_gloss_session` → `"session-uid"`. These must become real,
`item_type`-parameterised functions, and the set must be completed with the
operations the UI needs but which **do not exist at all today**: list, save-new
(returns id), update, **delete-one**, **clear-all** — each usable by both Gloss
and Prompts. Every `extern`-block function also needs a `qmllint` stub in
`assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml`.

**Backend DB — missing entirely:** the table, the Diesel
schema/model, and the CRUD helpers do not exist yet (see migration/Diesel bullets
below).

**Shared QML — missing entirely:** `HistoryListItem.qml` and the shared
truncate/single-line helper do not exist yet.

### Notes

- **Existing stubs to replace:** `bridges/src/sutta_bridge.rs:2515` onward —
  `get_gloss_history_json` / `update_gloss_session` / `save_new_gloss_session`
  currently return placeholders. Generalize these to take an `item_type` (or add
  prompts equivalents that share the same backend implementation).
- **App-close flush (req 17):** wire the final flush into the existing
  `SuttaSearchWindow.qml` `onClosing` handler (`:19`), which hosts both
  `gloss_tab` (`:3559`) and `prompts_tab` (`:3576`). The autosave `Timer` does
  not fire on quit, so the close hook must call a **blocking** flush for any tab
  whose `session_needs_saving` is true; the write must complete before the
  process exits. **Mobile caveat:** the existing `onClosing` *cancels* the close
  on mobile (`close.accepted = false`, opens the tab-list dialog, `:20-23`), so
  there is no guaranteed "real exit" hook on mobile. Run the flush on both
  branches anyway (a redundant save is harmless), and add
  `Component.onDestruction` flushes on the tabs as a backstop. The double-write
  race with an in-flight async save is bounded — the blocking flush only runs
  while `session_needs_saving` is still true, so the worst case is one redundant
  UPDATE, never data loss.
- **Debounce refactor (important — current code saves eagerly on the typing
  path).** Today GlossTab calls `save_session()` *synchronously on every change*:
  per-keystroke inline paragraph edits (`update_paragraph_text`, `:931` ←
  `TextArea.onTextChanged` `:1613`), word-selection changes
  (`update_word_selection`, `:909`), and post-gloss handlers (`:763`, `:795`).
  Each `save_session()` does `JSON.stringify` of the whole session (heavy:
  per-word lookup-result arrays) + a synchronous SQLite write + `load_history()`
  (a second synchronous SQLite read that rebuilds the whole `ListModel`). With the
  stub bridge this is free, but the real DB-backed version would block the UI
  thread on every keystroke. **Replace these direct calls with
  `session_needs_saving = true`** and let the 60 s `Timer` flush. Also remove the
  `load_history()` call from `save_session()` (`:584`).
- **Translations are currently lost on round-trip** — `save_session()` does not
  serialize each paragraph's `translations_json` and `load_session()` hard-codes
  `translations_json: "[]"` (`:895`). The "full restore / no data loss"
  requirement (18) means both must be fixed, along with the other state listed
  there.
- **Migration + bootstrap:** new appdata migration following the bookmarks
  pattern; the appdata DB is a shipped/user DB. **`ANALYZE` is intentionally NOT
  called per-save here.** `DatabaseHandle::analyze` (`backend/src/db/mod.rs:116`)
  runs a full-DB `ANALYZE;` over *all* appdata tables, so calling it on every
  60 s save would be wasteful. It is also unnecessary for this table: the
  catastrophic case in `docs/user-data-and-sqlite-analyze.md` /
  `tasks/prd-fixing-headword-match-slow-query.md` was a **multi-table join**,
  whereas the only query here is a single-table equality + order
  (`WHERE item_type = ? ORDER BY updated_at DESC`) fully served by the
  `(item_type, updated_at)` index, which SQLite plans correctly without stats.
  Document this decision rather than adding an ANALYZE hook.
- **Diesel:** add table to `appdata_schema.rs`, model struct(s) to
  `appdata_models.rs`, CRUD helpers to `appdata.rs`.
- **build.rs:** register the new `HistoryListItem.qml` in `qml_files`; no new
  Rust bridge file is needed (reuse `SuttaBridge`).
- **Background/async saves (req 10a):** the save/list/delete/clear bridge
  functions must do their SQLite work off the UI thread and report back via a
  signal/callback, following the existing background pattern
  (`process_all_paragraphs_background` / `process_paragraph_background` in
  GlossTab + `sutta_bridge.rs`). A new-session INSERT returns its
  `current_session_id` through the completion signal, not a synchronous return.
  Guard against overlapping in-flight writes (skip a tick if one is still
  running). The **only** exception is the app-close flush (req 17), which must run
  to completion before exit.
- **Timer:** one QML `Timer` (interval 60000 ms, `repeat: true`) per tab drives
  autosave; the tick is a no-op unless `session_needs_saving` is true, so an idle
  session causes zero DB access. The "Save" button and New Session/Open flush
  share the same write code path the timer uses.
- Persisted `data_json` is opaque to the backend (stored/returned as text); the
  QML side owns its schema per `item_type`.

## 8. Success Metrics

- Glossing a paragraph set, waiting for autosave (or clicking New Session),
  reveals the session in the Gloss History list as a truncated single line;
  Open fully restores text, paragraphs, and glossed words.
- Sending prompts produces a session in the Prompts History list; Open restores
  the full conversation.
- Both lists sort most-recent-first; Delete removes one item; Clear empties the
  list; clicking a row only highlights it.
- History survives an app restart (persisted in appdata DB).
- The save-state indicator shows "unsaved changes" after an edit and flips to
  "saved" after the timer tick or a Save-button click; the Save button forces an
  immediate write.
- Editing a glossed paragraph inline produces **no per-keystroke DB write and no
  visible input lag** (writes are debounced through the 60 s timer).
- **Restore fidelity:** opening a saved session reproduces the working area
  exactly as left — Gloss round-trips paragraph text, per-word selections, AI
  translations, selected AI tab, and options; Prompts round-trips the full
  conversation. No field visible/editable at save time is lost on restore.
- **No loss on quit:** making a change and immediately closing the app (before
  the 60 s timer fires), then reopening, shows the session preserved in history.
- **Non-blocking saves:** saving a large session (many paragraphs/words) via the
  timer or the Save button does not freeze the UI — the write runs in the
  background and the indicator flips to "saved" on completion.
- New Session and Clear both prompt the user for confirmation before proceeding.
- The same DB table, bridge functions, and list-item component serve both tabs
  (verified by code review — no duplicated history table or duplicated list-item
  QML).

## 9. Open Questions

1. *(Resolved → requirement 15)* Empty/whitespace-only sessions are skipped so
   the history isn't polluted with blank rows.
2. *(Resolved → requirements 12, 14, 16)* Pending changes are flushed immediately
   on Save, New Session, and Open; the 60 s timer covers the idle case.
3. *(Resolved → requirement 18)* The Prompts per-message fields `{ role, content,
   content_html, responses_json, selected_ai_tab }` are **sufficient** to restore
   the conversation UI — confirmed against the message delegate (`:636-640`),
   whose only other state is the local `is_collapsed` collapse toggle. Scroll
   position, in-flight request state, and other transient UI state are explicitly
   **not** persisted.
4. *(Resolved)* Naming confirmed: `item_type` literals `"gloss"` and `"prompts"`,
   table `gloss_prompts_history`.
5. *(Resolved → requirements 14, 20)* A confirmation dialog is shown before
   **Clear** (wipes all history) **and** before **New Session** (current session
   will be saved to history). Both warn the user before proceeding.
6. *(Resolved → requirement 17)* A final flush **must** run on app/tab close for
   any session with unsaved changes; the open implementation detail is *which*
   existing shutdown hook to attach it to (see Technical Considerations).
7. *(Resolved → requirement 10a)* Saves run as background operations off the UI
   thread; the only synchronous case is the app-close flush (requirement 17).
