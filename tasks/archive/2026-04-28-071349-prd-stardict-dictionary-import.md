# PRD: User-Imported StarDict Dictionaries

## 1. Introduction / Overview

Users currently can only search the dictionaries that ship bundled with Simsapa. This feature lets a user import their own StarDict-format dictionaries (as `.zip` archives) at runtime, manage them from a dedicated window, and include or exclude them per-search from the dictionary search UI.

Imported dictionaries are stored in the existing `dictionaries.sqlite3` (in the same `dict_words` table used by shipped dictionaries) and are indexed into the existing FTS5 and Tantivy indexes so they can be queried by the same code paths as shipped dictionaries.

## 2. Goals

1. Let a user import a StarDict `.zip` archive and have its words searchable immediately.
2. Provide a `Windows > Dictionaries…` menu and a `DictionariesWindow.qml` for managing imported dictionaries (list / edit label / delete).
3. Reuse the existing dict_words storage and the existing dictionary Tantivy schema + FTS5 index so query code does not branch on user vs shipped data.
4. Reorganize the dictionary search UI: collapse the existing advanced filter inputs under a "Filters" section, and add a "Dictionaries" section listing user-imported dictionaries with per-dictionary enable/lock controls.
5. Persist the user's per-dictionary enable selection across app restarts.
6. Migrate existing user databases with a Diesel migration that adds the new schema column (no data movement).

## 3. User Stories

- As a user, I want to import a StarDict `.zip` archive so that I can search its entries alongside the bundled dictionaries.
- As a user, I want to assign a short label (e.g. `mydict`) when importing so that lookup uids follow the `{word}/{label}` convention used by the rest of the app.
- As a user, I want to see a list of my imported dictionaries with their title, label, language, and entry count so that I know what is installed.
- As a user, I want to rename a dictionary's label and, after restarting the app for the re-indexing pass to run, have all of its uids and indexes consistently updated so my search results stay consistent.
- As a user, I want to remove an imported dictionary (with confirmation) so that its entries are removed from search.
- As a user, I want to choose which of my imported dictionaries are part of the search by toggling checkboxes in a "Dictionaries" panel in the dictionary search area.
- As a user, I want a quick "lock" (solo) toggle on a dictionary to temporarily search only that one without losing my other checkbox selections.
- As a user, I want the existing advanced filter controls grouped under a collapsible "Filters" section so the search area can be tidied when I don't need them.

## 4. Functional Requirements

### 4.1 Menu and Window

1. `SuttaSearchWindow.qml` must add a `Windows > Dictionaries…` menu item that opens `DictionariesWindow.qml`.
2. `DictionariesWindow.qml` must be an `ApplicationWindow`, structured similarly to `SuttaLanguagesWindow.qml`.
3. The window must contain:
   1. An "Import StarDict…" button.
   2. A list of currently imported dictionaries.

### 4.2 Import Flow

4. Clicking "Import StarDict…" must open a file picker restricted to `.zip` files.
5. After the file is chosen, the user must be prompted in a small dialog for:
   1. **Label** — Allowed characters: ASCII alphanumeric and `_-` only. Must be globally unique across all dictionary `source_uid` values used in `dict_words` (shipped and user — see below).
      - The dialog must pre-fill the label field with a sanitised version of the chosen `.zip` file's stem (e.g. `dppn.zip` → `dppn`, `My Dict v2.zip` → `My_Dict_v2`). Sanitisation rule: replace any character outside ASCII alnum / `_-` with `_`, collapse runs of `_`, and trim leading/trailing `_-`. The user may freely edit the pre-filled value before submitting.
      - **Built-in label set** = the set of distinct `source_uid` values appearing on any `dict_words` row whose owning `dictionaries` row has `is_user_imported = false`. This is computed at validation time (not from the `dictionaries.label` column) because some shipped sources (notably bold-definitions, which uses `ref_code` as `source_uid`) do not match the parent `dictionaries.label`. If the user-typed label collides with this set, the import must be **rejected with an error**. Replace is never offered for built-in labels.
      - If the label collides with another **user-imported** dictionary, the dialog must offer to **Replace** the existing user dictionary (delete old rows + indexes, then import new) or **Cancel**.
   2. **Language code** — a short text input (e.g. `pli`, `en`), default pre-filled `pli`. Any non-empty ASCII value is accepted; if the value is not a recognised tokenizer language for Tantivy (see `backend/src/search/tokenizer.rs::register_tokenizers`), the dialog must show a warning that the dictionary will be indexed with the fallback tokenizer, but the user may proceed.
6. The import (inside the running app) must:
   1. Extract the `.zip` to a temporary directory only (no permanent archive of the source `.zip` is kept; SQL is the source of truth).
   2. Call the same import code path used in the bootstrap (a runtime variant of `import_stardict_as_new()`), passing the user-chosen label.
   3. Insert rows into `dictionaries` and `dict_words` in `dictionaries.sqlite3`, using uids of the form `{word}/{label}`. Set `is_user_imported = true`, `language = <chosen>`, and leave `indexed_at = NULL` (this signals the startup pass to index).
   4. **Do not** touch the FTS5 or Tantivy indexes from inside the running app — those are updated by the startup re-indexing window (see §4.9). This avoids contention with the live `SuttaSearchWindow`'s Tantivy searcher / FTS5 readers.
   5. After the SQL insert succeeds, show a `MessageDialog`: "Dictionary `<label>` was imported. Please close and re-open Simsapa so it can be indexed for searching." The dictionary will not appear in dictionary search results until the next startup completes its indexing pass.
   6. Delete the temp extraction directory after import (success or failure).
7. The window must show a non-blocking inline progress bar during the SQL insert phase, modeled on the progress bar mechanism used by `DownloadAppdataWindow.qml`, with stage labels (Extracting / Inserting words) and a numeric percentage where available. There is no in-app indexing stage (see req. 6.4).
8. On SQL insert error, the partial dictionary row and its `dict_words` rows must be rolled back so the database remains clean.
9. **Concurrency / serialisation:** imports, renames, and deletes must be serialised by a single bridge-level mutex / busy flag — at most one of these operations may run at a time. While one is in progress, the bridge must reject the others with a clear "busy" error so the UI can disable buttons. (Single-user app; no inter-process contention is required.)
10. **Recovery from interrupted indexing** is handled by the `indexed_at IS NULL` check at the next startup (see §4.9). No additional in-app recovery path is required.

### 4.3 Distinguishing User vs Shipped Dictionaries

11. The `dictionaries` table must gain via Diesel migration:
    1. `is_user_imported BOOLEAN NOT NULL DEFAULT 0`
    2. `language TEXT NULL` — stores the language code chosen at import time. Existing shipped rows pick up `NULL` (their per-word language is on `dict_words` and remains the source of truth for them).
    3. `indexed_at TIMESTAMP NULL` — set after FTS5 + Tantivy indexing for the dictionary completes; used by the startup recovery path (see req. 10).
    4. `description TEXT NULL` — populated at import time from the StarDict `.ifo` `description` field (if present); shown by the info button in the Dictionaries panel. NULL for shipped rows and for imports where no description was found.
12. The migration must be additive only; no existing rows are modified beyond the column defaults.
13. Only rows with `is_user_imported = true` may be edited or removed via `DictionariesWindow.qml`. The bridge backend must enforce this — `delete_dictionary`, `rename_label`, and the Replace path must reject any `dictionary_id` whose `is_user_imported` is false.

### 4.4 Dictionary List in DictionariesWindow

14. The list must show, for each user-imported dictionary: title, label, language, entry count, and the action controls below.
15. Each row must have:
    1. An **Edit** button. Clicking it opens a dialog with a single editable field: `label`. Validation rules from req. 5 apply.
    2. A **Trash** icon. Clicking it shows a confirmation dialog ("Delete dictionary `<label>` and all its entries? This cannot be undone."), then on confirm:
       - In the running app: hard-delete all `dict_words` rows for that dictionary and the `dictionaries` row.
       - The corresponding entries in FTS5 / Tantivy (matched by `source_uid = <label>`) are removed lazily by the startup re-indexing pass on next launch — i.e. the deletion follows the same close-and-restart flow as import / rename. The user must be shown the close-and-restart `MessageDialog`.
16. Editing the label must (inside the running app):
    1. Update the `dictionaries.label` column.
    2. Rewrite every affected `dict_words.uid` from `{word}/{old_label}` to `{word}/{new_label}` and every `dict_words.dict_label` value (which functions as the dictionary's `source_uid` in indexes).
    3. Set the row's `indexed_at = NULL` to flag it for re-indexing on next startup. The startup pass will delete the old-label entries from FTS5 / Tantivy (by `source_uid` term) and re-insert under the new label.
    4. Run all SQL changes inside a single transaction; on failure, roll back.
    5. Before performing the rename, the Edit dialog must display a warning informing the user that the rename takes effect after the next app restart, when the affected entries are re-indexed in FTS5 and Tantivy, and that this may take some time for large dictionaries. The user must confirm to proceed.
    6. After a successful rename, show the same close-and-restart `MessageDialog` as the import flow (req 6.5).
17. Shipped dictionaries must not be listed in `DictionariesWindow.qml` (only `is_user_imported = true`).

### 4.5 Dictionary Search UI Reorganization

The advanced search options currently live in `assets/qml/SearchBarInput.qml` inside the `Flow { id: advanced_options_row }` block, which is toggled visible by the existing `advanced_options_btn` (the `advanced_options_visible` property). The reorganisation modifies this row in place — it does not touch `DictionaryTab.qml`.

18. Inside `advanced_options_row` in `SearchBarInput.qml`, the existing inputs and checkboxes (Nikāya prefix, UID prefix, UID suffix, MS Mūla checkbox, etc.) must be wrapped under a new collapsible **"Filters"** sub-section.
    - Sub-section header: a chevron icon button + "Filters" `Label`, matching the pattern used in `ChantingPracticeReviewWindow.qml`.
    - Toggling the chevron flips `is_filters_collapsed` (default `false` — open). The wrapped controls bind `visible: !is_filters_collapsed`.
    - This is in addition to the existing outer `advanced_options_btn` gate: the whole `advanced_options_row` only renders at all when `advanced_options_visible` is true; once visible, the user can independently collapse "Filters".
19. A new collapsible **"Dictionaries"** sub-section must be added inside the same `advanced_options_row`, with the same chevron pattern. Like the rest of `advanced_options_row`, it is only visible when the user has clicked `advanced_options_btn` to open advanced options — the dictionary selection UI is treated as an advanced control.
    - Visible only when `root.search_area === "Dictionary"`.
    - Default state: **open** (`is_dictionaries_collapsed = false`).
    - Implemented as a new QML component `assets/qml/DictionarySearchDictionariesPanel.qml`, integrated into `SearchBarInput.qml` alongside the Filters sub-section.
    - Lists, in order: (a) two **built-in entries** — "DPD" and "Commentary Definitions" — and (b) the user-imported dictionaries.
    - Each row uses a wrapper element styled similarly to `user_repeater` items in `ChantingPracticeReviewWindow.qml`, containing: a checkbox, the dictionary name + label, an **info button** (`?` icon), and a lock toggle button.
    - When the row's checkbox is on, the wrapper background must be a light blue.
    - **DPD row.** The checkbox is bound to a new persisted setting `dict_search.dpd_enabled` (default `true`). The info button shows DPD's stock description text in a small dialog.
    - **Commentary Definitions row.** This row replaces the existing "Commentary Definitions in Search" checkbox + info button currently in the advanced options area — that pair must be removed from its current location. The checkbox is bound to the existing setting that backed the old "Commentary Definitions in Search" toggle (preserve the storage key and migration semantics so existing user preferences are not lost). The info button shows the same explanatory text the old info button showed.
    - **User-imported rows.** The info button is enabled only when the dictionary's `description` column is non-NULL; clicking it shows the description in a small dialog. If there is no description, the info button is hidden (or rendered disabled).
    - When there are zero user-imported dictionaries, the user-imported section of the list must show an empty-state hint message under the two built-in rows: "No imported dictionaries. See Windows > Dictionaries…" — the built-in DPD and Commentary Definitions rows must always be present regardless.
    - Changes to checkbox or lock state must trigger `advanced_options_changed()` (debounced via the existing `advanced_options_debounce_timer`) so the search re-runs.

### 4.6 Per-Dictionary Search Selection

20. Every row in the "Dictionaries" section — both built-in entries (DPD, Commentary Definitions) and each user-imported dictionary — has a checkbox controlling whether its entries are included in the dictionary search query.
21. The default state for a newly imported dictionary is **checked on**. The default state for both built-in entries (DPD, Commentary Definitions) is also **checked on**.
22. The checkbox state for every row (built-in and user-imported) must be persisted across app restarts (via `app_settings` or equivalent). Suggested keys: `dict_search.dpd_enabled`, `dict_search.commentary_definitions_enabled` (preserve the existing key for the latter to retain user history), `dict_search.user_dict_enabled.<label>`.
23. When executing a dictionary search:
    - The query must be constrained to the set of currently checked-on rows, further restricted by any active lock (see §4.7).
    - For shipped sources other than DPD and Commentary Definitions (none expected today, but to be future-proof), the query layer must include them unconditionally — only the two surfaced built-in rows are user-toggleable.
    - Filtering must occur in the same query layer that already filters by `source_uid` for shipped dictionaries. The DPD toggle removes/keeps `source_uid = "dpd"` and any DPD-related source_uids in the constraint set; the Commentary Definitions toggle removes/keeps the commentary bold-definitions `source_uid` set (the same set the existing checkbox already controls).

### 4.7 Lock (Solo) Toggle

24. Every row in the "Dictionaries" section — built-in (DPD, Commentary Definitions) and user-imported — has a checkable button with a lock icon.
25. When a lock button is activated:
    1. That row becomes the **only** dictionary contributing to the search query. All other rows — built-in *and* user-imported — are excluded from the search.
    2. All other rows enter a **disabled visual state** (their checkboxes and info buttons are visually disabled and not interactive).
    3. The other rows' underlying checkbox states must **not** be modified (so the user's prior selection is preserved).
26. When the lock is deactivated, the prior checkbox states are restored as the active selection (no state was lost because none was modified).
27. Only one lock may be active at a time. Activating a lock on a different row deactivates any previously locked row.
28. The lock scopes the entire dictionary set (built-in + user-imported). There is no "shipped always included" exception under lock — solo means solo.

### 4.8 Migration & Upgrade Strategy

29. **Schema migration.** A new Diesel migration must add the columns from req. 11 (`is_user_imported`, `language`, `indexed_at`) to `dictionaries` in `dictionaries.sqlite3`. The migration is additive only.
30. **Shipped-DB replacement upgrade story.** Because `dictionaries.sqlite3` lives under `app_assets_dir` and is replaced wholesale on each release upgrade, user-imported dictionaries must follow the existing **export → replace → re-import** pattern used for books/bookmarks/chanting in `app_data.rs::export_user_data_to_assets()`:
    1. **Export step (pre-upgrade or first-launch detection):** add a new `export_user_dictionaries(import_dir)` category alongside `export_user_books`, `export_user_bookmarks`, `export_user_chanting_data`. It writes a small SQLite snapshot at `import-me/user_dictionaries.sqlite3` containing:
       - All `dictionaries` rows where `is_user_imported = true` (with their `label`, `title`, `language`, `is_user_imported = true`; **`indexed_at` is intentionally NOT carried over** — the re-imported rows must trigger re-indexing).
       - All `dict_words` rows whose `dictionary_id` belongs to the exported set (with every column).
       - This mirrors the chanting-export pattern in `app_data.rs::export_user_chanting_data`.
    2. **Replace step:** the new shipped `dictionaries.sqlite3` overwrites the old (existing project behaviour). The export step MUST run before the replace step; if the export fails, the user must be warned and the upgrade replace step held back, otherwise user data is lost.
    3. **Re-import step (next startup):** add a new `import_user_dictionaries(import_dir)` category that opens `import-me/user_dictionaries.sqlite3`, inserts the dictionaries rows (re-keyed: new autoincrement `id` from the new shipped DB), then inserts the `dict_words` rows with their `dictionary_id` rewritten to the new `id`. Set `indexed_at = NULL` on every re-imported row so the §4.9 reconciliation pass re-indexes them. After a successful import, delete `import-me/user_dictionaries.sqlite3`.
31. The first build that ships this feature must run only the schema migration (req. 29); the export/import-me round-trip applies from the next release onward, when the shipped DB will replace the user-modified one and lose the user-imported rows.

### 4.9 Startup Recovery / Re-Indexing Window

32. On each app startup, before opening `SuttaSearchWindow`, the backend must run a **dictionary index reconciliation** pass that detects work to do via any of:
    1. A pending `import-me/user_dictionaries.sqlite3` snapshot from a release upgrade (re-import scenario — handled by `import_user_dictionaries` in §4.8 step 3, which runs before reconciliation so all rows it produces have `indexed_at = NULL`).
    2. Any user-imported dictionary row whose `indexed_at` is NULL (newly imported, just renamed, just re-imported from a release-upgrade snapshot, or interrupted indexing).
    3. Any FTS5 / Tantivy dictionary entries whose `source_uid` does not correspond to a current `dictionaries.label` (orphans from deletion or release-upgrade replacement of the shipped DB).
33. If any of the conditions above produce work, a modal **startup progress window** must be shown that:
    1. Tells the user "Re-indexing imported dictionaries — please wait." with a progress bar and per-dictionary status.
    2. Sequence: (a) consume any pending `import-me/user_dictionaries.sqlite3` (insert into the live DB with re-keyed ids and `indexed_at = NULL`); (b) drop orphan entries from FTS5 + Tantivy by `source_uid` term; (c) for each dictionary with `indexed_at = NULL`, delete any pre-existing entries with that `source_uid` from FTS5 + Tantivy (idempotency safety), insert its `dict_words` into FTS5 then Tantivy, then set `indexed_at = now()`.
    3. On completion, closes itself and proceeds to open `SuttaSearchWindow`.
34. If the user closes the startup window mid-re-index (or the process dies), the next startup must resume by retrying any not-yet-indexed dictionaries (idempotent — re-indexing must be safe to repeat).
35. After a deliberate import / rename / delete from `DictionariesWindow.qml`, the bridge must show a `MessageDialog` asking the user to **close and re-open the app** so the indexing pass runs from the startup window. v1 does not perform live in-place re-indexing inside the running app for these operations to avoid concurrent Tantivy writer / live searcher conflicts.

## 5. Non-Goals (Out of Scope)

- Exporting dictionaries.
- Editing individual word entries.
- Importing non-StarDict formats (Babylon, DSL, etc.).
- Automatic dictionary downloads from a remote catalog.
- Editing fields other than `label` (e.g. title, description, language).
- Listing or controlling shipped dictionaries from `DictionariesWindow.qml` or the new "Dictionaries" search panel.
- Re-importing migrating any pre-existing user-imported StarDict data (none exists prior to this feature).

## 6. Design Considerations

- Window layout and structure: model `DictionariesWindow.qml` on `SuttaLanguagesWindow.qml`.
- Collapsible section headers and item wrapper layout: model on `ChantingPracticeReviewWindow.qml` (its `user_repeater` and section headers).
- Progress bar: model on the progress mechanism used in `DownloadAppdataWindow.qml`.
- Active-checkbox row background: light blue (use existing theme color if a suitable one exists; otherwise add one to `theme_colors_light.json` and `theme_colors_dark.json`).
- Lock icon and chevron icons should reuse existing icons in `assets/qml/icons/` where possible.
- The new QML files must be registered in `bridges/build.rs` per the project's CLAUDE.md procedure.

## 7. Technical Considerations

- Reuse `import_stardict_as_new()` from `backend/src/stardict_parse.rs` as the underlying SQL-only import primitive (no FTS5 / Tantivy from inside the running app). Expose a thin runtime wrapper callable from the bridge that:
  - accepts a `.zip` path, label, language,
  - extracts to a temp dir (deleted after import; the `.zip` is not archived — SQL is the source of truth, mirroring the chanting/books export pattern),
  - emits progress signals to QML for the inline progress bar,
  - sets `is_user_imported = true`, `language = <chosen>`, `indexed_at = NULL`, and `description = <ifo_description_or_NULL>` on the new `dictionaries` row. The StarDict `.ifo` file may include a `description=` line; when present, capture it verbatim (trimmed) and store it.
- A new Rust bridge (`bridges/src/dictionary_manager.rs`) must expose: `import_zip(path, label, lang)`, `list_user_dictionaries()` (returned JSON includes the `description` column), `rename_label(id, new_label)`, `delete_dictionary(id)`, `validate_label(label)`, `is_label_taken_by_user(label)`, `is_label_taken_by_shipped(label)` (implemented via `SELECT DISTINCT source_uid FROM dict_words WHERE dictionary_id IN (SELECT id FROM dictionaries WHERE is_user_imported = 0)`, *not* a `dictionaries.label` lookup, so bold-definitions `ref_code` source_uids are correctly treated as built-in), `suggested_label_for_zip(zip_path)` (returns the sanitised filename stem), `get_user_dict_enabled(label)`, `set_user_dict_enabled(label, enabled)`, `get_user_dict_enabled_map()`, plus built-in toggles `get_dpd_enabled()`, `set_dpd_enabled(enabled)`, `get_commentary_definitions_enabled()`, `set_commentary_definitions_enabled(enabled)`. The bridge must enforce req. 13 (no edit / delete on `is_user_imported = false` rows) and req. 9 (single-mutex serialisation of import / rename / delete operations). Register the bridge per the QmlModule procedure in CLAUDE.md and create the `qmllint` stub in `assets/qml/com/profoundlabs/simsapa/`.
- **Tantivy schema**: the existing `build_dict_schema()` in `backend/src/search/schema.rs` exposes a `source_uid` text field (raw tokenizer, indexed) — this is the field used for delete-by-term, with the dictionary label as the term value. There is no separate `dict_label` field; do not invent one. Reuse the same field set used by the bootstrap DPD StarDict import.
- **FTS5**: use the existing dictionary FTS5 virtual table; insert/delete by `source_uid` (= label).
- **Tokenizer language**: `register_tokenizers(index, lang)` is called per Tantivy index. If the user-supplied language is not one of the supported codes, the import dialog must warn but still proceed; the indexer falls back to the default tokenizer for unknown languages. Indexing happens in the startup pass, so the warning is shown at import-dialog time only.
- **Concurrency model**: a single bridge-level `Mutex<()>` (or busy `bool`) gates `import_zip` / `rename_label` / `delete_dictionary` against each other. The startup re-indexing pass runs before `SuttaSearchWindow` opens, so it never contends with a live searcher. This is a single-user app — no IPC-level serialisation is required.
- **Indexing & live searcher contention**: by design, all FTS5 and Tantivy index writes happen in the startup re-indexing window before `SuttaSearchWindow` opens. Inside the running app no Tantivy `IndexWriter` is opened on the dict index, so no directory-lock contention with the live `IndexReader`.
- All filesystem existence checks must use `try_exists()` per CLAUDE.md (Android safety).
- Persisted per-dictionary checkbox state lives in `backend/src/app_settings.rs` (no extra DB table / migration). Keys: `dict_search.dpd_enabled = bool` (default true), `dict_search.commentary_definitions_enabled = bool` (default true — reuse the existing key that backed the old "Commentary Definitions in Search" advanced-options checkbox), `dict_search.user_dict_enabled.<label> = bool` (default true). The transient lock state (which row is solo'd) is NOT persisted across restarts.
- The query layer must accept the per-built-in toggles as part of its filter input (alongside the user-imported filter). Concretely: extend the dictionary-search filter type to carry `include_dpd: bool` and `include_commentary_definitions: bool`. The `DictionaryTab.qml` caller computes these from `app_settings` (or the bridge) before issuing the search. When a lock is active, the lock target overrides all checkbox state and the filter contains exactly the locked dictionary's `source_uid` set.
- The existing "Commentary Definitions in Search" advanced-options checkbox + info button (currently in `SearchBarInput.qml`) must be **removed** as part of this change — its state and info text are migrated into the new Dictionaries panel row. The setting key is preserved so user preferences carry over without a migration.
- **Pre-existing release-1 user data**: the very first build that ships this feature only adds the schema columns; no upgrade round-trip is required because there are no user dictionaries yet. From the next release onward, every user-imported dictionary is round-tripped through `import-me/user_dictionaries.sqlite3` on upgrade, mirroring the chanting / books / bookmarks export pattern.

## 8. Success Metrics

- A user can import a 50k-entry StarDict `.zip`, restart the app, watch the startup re-indexing window complete, and then find a known word from it via the dictionary search.
- After re-launch, the user's per-dictionary checkbox selections are restored.
- Renaming a label produces consistent results: searches against the new label return the same entries; the old label returns none.
- Deleting a dictionary removes all of its entries from SQL, FTS5, and Tantivy results.
- Existing dictionary search behavior for shipped dictionaries is unchanged when no user dictionaries are imported.
- Migration runs cleanly on an existing user `dictionaries.sqlite3` from the previous release (no data loss, no manual steps).

## 8a. Built-in Label Detection (Implementation Note)

"Shipped/built-in" labels for the purposes of label-collision validation are NOT read from `dictionaries.label`. They are computed from `dict_words.source_uid` because some shipped sources (notably bold-definitions) use a per-row `ref_code` as `source_uid` rather than the parent `dictionaries.label`. The canonical query is:

```sql
SELECT DISTINCT source_uid FROM dict_words
WHERE dictionary_id IN (SELECT id FROM dictionaries WHERE is_user_imported = 0);
```

The result of that query is the set the import dialog's `label_status` check must reject as `taken_shipped`. Cache it once per app session if performance becomes a concern (the `dict_words` table is large but `DISTINCT source_uid` is selective).

## 9. Open Questions

(None outstanding — all prior open questions have been resolved and folded into the requirements above.)
