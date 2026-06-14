# PRD: Dictionary Import UX Improvements & StarDict Resource Support

## 1. Introduction/Overview

The user-imported StarDict dictionary feature (import / rename / delete) works
functionally, but the UX has rough edges and one capability gap:

1. The **import progress dialog** shows only the bare label (e.g. `Importing "mw-gd"…`)
   even though the backend already knows the dictionary's full title, language,
   and entry count by the time inserting begins.
2. **Aborting an import** gives no immediate visual feedback — the user clicks
   "Abort" and the UI looks unchanged until the worker notices the cancel flag.
3. If an import is **aborted before any entries are inserted**, the (empty)
   `dictionaries` row is left behind, so the dictionary list shows an imported
   dictionary with 0 entries.
4. **Renaming a dictionary** runs a synchronous conflict check on every keystroke
   (`label_status()`), making the label field feel sluggish.
5. **StarDict `res/` resources** (images, CSS, etc. bundled in the zip) are not
   handled at all on general imports. DPD gets special migration treatment at
   bootstrap (its `res/` is copied to `assets/dpd-res/`), but general StarDict
   imports silently drop their resources, so definitions that reference those
   files render broken.

This feature improves the import/rename UX and adds general StarDict resource
support, storing resources in the database and serving them over the existing
localhost API.

## 2. Goals

1. Show the dictionary's full descriptive identity (title, language, entry count)
   in the import progress dialog as soon as it is known.
2. Give the user immediate visual feedback ("Aborting…") when they click Abort.
3. Never leave a 0-entry dictionary row behind when an import is aborted before
   any entries were inserted.
4. Make the rename-dialog conflict check debounced and non-blocking, modeled on
   the search-input debounce in `SearchBarInput.qml`, so typing stays smooth.
5. Support StarDict `res/` resources (when present in the zip) so dictionary
   definitions render correctly — without adding files to the `assets/` folder.
   CSS/JS bundled in `res/` is injected at render time via the existing
   `css_extra`/`js_extra` hooks; other resources (images, etc.) are stored in the
   database and served via the localhost API.

## 3. User Stories

- **As a user importing a large dictionary**, I want to see which dictionary is
  being imported and how many entries it has, so I know the import is working on
  the right file and roughly how long it will take.
- **As a user who started the wrong import**, I want clicking "Abort" to visibly
  acknowledge my action immediately, so I'm not left wondering if it registered.
- **As a user who aborts an import early**, I don't want a useless empty
  dictionary left in my list afterward.
- **As a user renaming a dictionary**, I want the label field to respond
  instantly as I type, with conflict warnings appearing shortly after I pause,
  rather than the field stuttering on every keystroke.
- **As a user importing a StarDict dictionary with bundled resources** (e.g.
  `mw-gd.zip` with a `res/` folder containing CSS), I want definitions to render
  with their intended styling and images.

## 4. Functional Requirements

### 4.1 Detailed import progress label

1. The backend MUST emit the dictionary's full descriptive identity to QML as
   early as it is known — i.e. after the `.ifo` is parsed and the entry count is
   read, before/at the start of the inserting-words stage.
2. The displayed text MUST follow the form used in the existing log line:
   `Importing <title> (<lang>), <total> total entries …`
   (e.g. `Importing Monier-Williams Sanskrit-English Dictionary, 1899 (sa-en),
   194070 total entries…`).
3. Until the detail is available (during Extracting/Parsing), the dialog MAY fall
   back to the existing `Importing "<label>"…` text.
4. The `<title>` and `<total>` come from the backend: `ifo.bookname` and the raw
   index item count `dict.idx.items.len()` — both available at
   `stardict_parse.rs:349` (after `stardict::no_cache` loads the dict, where the
   existing `Importing … total entries` log line is emitted). Note this index
   count is the correct value to display (it matches the log) and differs from the
   later inserted count `words_to_insert.len()` used by the inserting-words bar.
5. `<lang>` does NOT need to travel through the signal: QML already passed `lang`
   into `start_import`/`import_zip`, so it can compose the `(<lang>)` part itself.
   The new progress data therefore only needs to carry `title` and `total`.
6. Implementation note: add a new progress variant (e.g.
   `StardictImportProgress::Identified { title, total }`) surfaced through the
   `importProgress` signal path (`bridges/src/dictionary_manager.rs`) and handled
   in `onImportProgress` in `DictionariesWindow.qml`.

### 4.2 Immediate "Aborting…" feedback

6. Clicking "Abort" during an import MUST immediately change the UI to an
   "Aborting…" state (e.g. progress text becomes `Aborting…`, the Abort button is
   disabled, the bar may switch to indeterminate).
7. This state change MUST be driven on the QML side at click time (in the
   `onClicked` handler that calls `dict_manager.abort_import()`), not deferred
   until a backend signal arrives.
8. When the backend subsequently emits `importCancelled`, the UI proceeds to the
   existing abort-summary frame as it does today.

### 4.3 Remove empty dictionary row on early abort

9. When an import is aborted and **0 entries were inserted**, the backend MUST
   delete the `dictionaries` row that was created for it (and any associated FTS
   / index artifacts and resources), so no 0-entry dictionary remains. The
   `dictionaries` row is created before any word insertion
   (`stardict_parse.rs:326`), so `outcome.dictionary_id` is valid even on a
   0-entry abort. This cleanup MUST run in the **bridge** abort branch
   (`dictionary_manager.rs:259`) after `import_user_zip` has returned (the
   `DICT_MGR_LOCK` is released by then) — calling `delete_user_dictionary`
   from inside `import_user_zip` would self-deadlock on that same `try_lock`.
10. When an import is aborted **after** ≥1 entry was inserted, the current
    behavior is preserved: partial rows remain and are reconciled on next startup
    (do not change this).
11. The abort-summary message and `importCancelled` signal MUST remain accurate:
    for the 0-entry case the message MUST NOT claim a partial import was kept.
    (The QML summary at `DictionariesWindow.qml` should distinguish "aborted,
    nothing imported, removed" from "aborted, N entries kept".)

### 4.4 Debounced, non-blocking rename conflict check

12. The label-conflict check in `DictionaryEditDialog.qml` MUST NOT run
    synchronously on every keystroke. It MUST be debounced using a `Timer`,
    mirroring the debounce pattern in `SearchBarInput.qml`.
13. The actual conflict check MUST run as a background task that does not block
    or slow QML rendering. The backend MUST expose an asynchronous variant of the
    label-status check that returns its result via a signal (analogous to the
    other `*Finished`/`*Failed` signals in `DictionaryManager`), rather than the
    current synchronous `label_status()` invokable.
14. When the async check reports a problem (`invalid`, `taken_shipped`,
    `taken_user`), the dialog MUST show the corresponding error message (the
    existing labels at `DictionaryEditDialog.qml:61-86`) and keep the OK action
    disabled/blocked, exactly as the synchronous version does today.
15. Local fast-path validation that needs no DB lookup (empty input, unchanged
    label, obviously-invalid characters) MAY still be evaluated immediately on the
    QML side to give instant feedback before the debounced backend check returns.
16. The synchronous `label_status()` may be retained for non-typing callers, but
    the per-keystroke path in the edit dialog MUST use the debounced async path.

### 4.5 StarDict resource handling

The standard StarDict bundled-resource location is a `res/` folder inside the
zip. All such files are stored uniformly in the database; they differ only in how
they are applied at render time (CSS/JS injected inline; images and other files
served over the API).

**Storage (at import time):**

17. During StarDict import, the importer MUST detect a `res/` folder in the
    extracted zip, if present. (Confirm in code that resource handling is
    genuinely absent first — it currently is: `import_user_zip` /
    `import_stardict_as_new` do not read `res/`.)
18. If a `res/` folder is present, **every** file in it MUST be stored as a row in
    a new `dict_resources` table, keyed by the dictionary **`id`** plus the
    relative resource path, storing `content_data` (blob) and `mime_type`,
    mirroring the existing `book_resources` table (`appdata_schema.rs:122`,
    model `BookResource`, query `get_book_resource`). The key MUST be the
    dictionary `id`, not the label, because the label can change on rename.
19. Resources MUST NOT be written to the `assets/` folder. (DPD's
    `assets/dpd-res/` bootstrap copy is a pre-existing special case and is out of
    scope for this change.)
20. The stored definition HTML MUST NOT be rewritten at import time. URL rewriting
    is deferred to render time (req 24) because the API port can change between
    runs, so absolute URLs cannot be baked in.

**Serving (API route):**

21. A new localhost API route MUST serve these resources, modeled on
    `serve_book_resources` (`bridges/src/api.rs:792`), keyed by id:
    `GET /dict_resources/<dict_id>/<path..>`, returning the stored blob with the
    correct content type (reuse the same `mime_type` → `ContentType` mapping).

**Applying at render time (`render_word_html_by_uid`, `app_data.rs:389`):**

22. User-imported dictionary words render through the regex-rewrite branch of
    `render_word_html_by_uid` (`app_data.rs:448-484`) — **not** through
    `sutta_html_page`/`css_extra`. Per-dictionary CSS and JS captured from `res/`
    (`mime_type` `text/css` and `application/javascript`/`text/javascript`) MUST
    be injected into that branch's existing `</head>` `<style>`/`<script>`
    injection (alongside `DICTIONARY_CSS`), looked up by `word.dictionary_id`.
23. The original `<link href="res/…css">` / `<script src="res/…js">` references in
    the definition HTML MUST be neutralised (removed or rewritten) so the injected
    copies are not also requested and 404 — CSS/JS are injected, not served.
24. References to non-CSS/JS resources (images, fonts, etc.) inside the definition
    HTML MUST be rewritten **at render time** to the id-keyed serving route, using
    the live `self.api_url` (e.g. `src="res/foo.png"` →
    `<api_url>/dict_resources/<id>/foo.png`). Because the URL is keyed by the
    stable `id`, rename does not require any rewriting.
25. The existing unconditional `<link href>` → `{api_url}/assets/dpd-res/`
    rewrite (`app_data.rs:473-479`) MUST be made **DPD-only**. It currently
    rewrites every dict word's links to the DPD resource folder, which is wrong
    for user-imported dictionaries (a latent bug this feature must guard against).
26. If a dictionary's `definition_html` is a bare fragment (no `<head>`/`<body>`
    for the regex injection to operate on), the renderer MUST instead wrap it via
    `sutta_html_page` with the per-dictionary CSS/JS passed through
    `css_extra`/`js_extra` (the same mechanism DPPN uses). The implementer MUST
    inspect the actual `definition_html` shape produced for `mw-gd` and choose the
    correct path; the full-doc path (req 22) is expected for goldendict-style
    bundles like DPD/mw-gd.

**Cleanup:**

27. When a dictionary is deleted (`delete_user_dictionary`), its `dict_resources`
    rows MUST be removed as well.

## 5. Non-Goals (Out of Scope)

- Changing DPD's special bootstrap migration or its `assets/dpd-res/` mechanism.
- Removing a 0-entry dictionary on a *normal* (non-aborted) import. A
  successfully-completed import is kept even with 0 entries, because the user may
  be testing a work-in-progress dictionary. Only the **abort-with-0-entries** case
  removes the row.
- Reworking the startup reconcile flow beyond what requirement 9 implies.
- Supporting resource formats StarDict itself doesn't bundle, or fetching remote
  resources.
- Changing the rename re-indexing model (still takes effect after restart).

## 6. Design Considerations

- The import progress dialog lives in `DictionariesWindow.qml` (progress frame
  ~lines 405-460; `Importing "${root.op_label}"…` at line 413, Abort button at
  line 458).
- The rename dialog is `DictionaryEditDialog.qml`; its status labels already exist
  for the three problem states. Reuse the `SearchBarInput.qml` debounce `Timer`
  idiom for consistency.
- Per the project conventions in `AGENTS.md`/`CLAUDE.md`: any new `DictionaryManager`
  invokable/signal must be added to the qmllint type definition
  `assets/qml/com/profoundlabs/simsapa/DictionaryManager.qml`; new QML files (if
  any) must be added to `qml_files` in `bridges/build.rs`.

## 7. Technical Considerations

- **New progress data**: add a variant to `StardictImportProgress` in
  `backend/src/stardict_parse.rs` carrying `title`/`total`, emitted at
  `stardict_parse.rs:349`; map it in `stardict_progress_to_signal` and surface it
  on the `importProgress` signal in `bridges/src/dictionary_manager.rs`. `lang` is
  already known to QML and need not travel through the signal.
- **Async label check**: add a worker-thread invokable + `labelStatusChecked`-style
  signal to `DictionaryManager`, reusing `core_validate_label` /
  `is_label_taken_by_shipped` / `list_dictionaries`.
- **Empty-abort cleanup (deadlock guard)**: do this in the **bridge** abort branch
  (`dictionary_manager.rs:259`), AFTER `import_user_zip` returns, when
  `outcome.cancelled && outcome.inserted == 0` — call
  `dictionary_manager_core::delete_user_dictionary(outcome.dictionary_id)` then
  `refresh_dict_source_uid_caches()`. Do NOT call it from inside `import_user_zip`:
  both `import_user_zip` and `delete_user_dictionary` acquire the same
  `DICT_MGR_LOCK` via `try_lock`, so a nested call returns `BUSY` / cannot clean up.
- **`dict_resources` table**: requires a Diesel migration under
  `backend/migrations/dictionaries/` and a model. Template directly off
  `book_resources` (`appdata_schema.rs:122`, model `BookResource`,
  `get_book_resource` query, `serve_book_resources` route). Not FTS-indexed, so the
  FTS5-rowid guidance in `CLAUDE.md` does not apply — a plain table. Store all
  `res/` files here (CSS/JS and images alike); the distinction is only in how they
  are applied at render.
- **Render path (the important correction)**: user-imported dictionary words are
  NOT rendered via `sutta_html_page`. They go through the regex-rewrite branch of
  `render_word_html_by_uid` (`backend/src/app_data.rs:448-484`), which injects
  `DICTIONARY_CSS` + JS by `word_html.replace("</head>", …)` and (currently,
  unconditionally) rewrites every `<link href>` to `{api_url}/assets/dpd-res/`.
  This feature must:
  1. inject the dictionary's stored CSS/JS (looked up by `word.dictionary_id`)
     into that same `</head>` injection;
  2. neutralise the original `<link>`/`<script src>` to `res/` CSS/JS so they are
     not double-loaded / 404'd;
  3. rewrite non-CSS/JS `res/…` references to `<self.api_url>/dict_resources/<id>/…`
     at render time (port-independent — never bake URLs at import);
  4. make the existing `assets/dpd-res/` rewrite **DPD-only** (latent bug).
  If `mw-gd`'s `definition_html` turns out to be a bare fragment without
  `<head>`, route it through `sutta_html_page` with `css_extra`/`js_extra` instead
  (the DPPN path) — inspect the actual stored shape first.
- **mw-gd test fixture**: `bootstrap-assets-resources/stardict-imports/mw-gd.zip`
  (extracted to `…/mw-gd/`) contains a `res/` folder with a CSS file — use it as
  the integration test case. Inspect a stored `definition_html` row after import to
  confirm the full-doc-vs-fragment shape and the exact resource reference syntax.

## 8. Success Metrics

- Importing `mw-gd.zip` shows `Importing Monier-Williams Sanskrit-English
  Dictionary, 1899 (sa-en), 194070 total entries…` in the progress dialog.
- Clicking Abort shows `Aborting…` within one frame.
- Aborting `mw-gd` before any insert leaves **no** mw-gd row in the dictionary
  list (verified via `list_user_dictionaries`).
- Typing a new label in the rename dialog produces no perceptible input lag;
  conflict warnings appear shortly after the user stops typing.
- After importing `mw-gd`, its definitions render with the bundled `res/` CSS
  applied (injected inline), and any image/other resource requests resolve via
  `<api_url>/dict_resources/<id>/…` (HTTP 200, correct content type).
- A user dictionary's links are no longer misrouted to `assets/dpd-res/`.
- Deleting `mw-gd` removes its `dict_resources` rows.

## 9. Open Questions

None outstanding.

### Resolved

- **Normal 0-entry import:** keep the dictionary (user may be testing a
  work-in-progress dictionary). Only abort-with-0-entries removes the row. (§4.3,
  Non-Goals)
- **CSS/JS application:** inject `res/` CSS via `css_extra` and `res/` JS via
  `js_extra` at render time — no API URL or link-rewriting needed for these.
  Only images/other resources go through the `dict_resources` table + API. (§4.5)
- **Resource location:** `res/` is the standard folder; no other conventions
  need detecting. (§4.5)
- **Image/resource serving URL:** keyed by dictionary `id` (not label), so the
  baked-in `/dict_resources/<id>/…` URLs survive rename without rewriting. (§4.5)
