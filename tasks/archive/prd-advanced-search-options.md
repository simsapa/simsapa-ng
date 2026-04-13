# PRD: Advanced Search Options

## Introduction/Overview

Add an advanced search options panel to the sutta search bar that consolidates search filtering controls currently scattered across the Settings window. A new checkable button (with a settings icon) is added after the S/D/L buttons in `SearchBarInput`. When toggled on, a new row appears below the search mode and language filter row, containing nikaya/uid prefix filters and source/commentary inclusion checkboxes. This improves discoverability and puts search-relevant options where users need them — right next to the search bar.

## Goals

1. Add a toggleable advanced search options row to `SearchBarInput` for sutta search.
2. Provide nikaya and uid "starts with" text filters that narrow search results by prefix.
3. Provide checkboxes for MS Mūla, CST Mūla, CST Commentaries, and CST Commentary in Translations — replacing the corresponding settings in `AppSettingsWindow`.
4. Persist all advanced search option values in `AppSettings`.
5. Support prefix filtering in both FulltextMatch (tantivy) and ContainsMatch (FTS5) search modes.

## User Stories

- As a user searching for suttas, I want to filter results by nikaya prefix (e.g. "mn", "an1") so I can narrow results to a specific collection.
- As a user, I want to filter results by uid prefix (e.g. "vin") so I can focus on a specific text group.
- As a user, I want to quickly toggle whether CST commentary, CST mūla, or MS mūla results appear in search, without navigating to the Settings window.
- As a user, I want to see info about what each checkbox does by clicking an info icon next to it.

## Functional Requirements

### UI: Advanced Search Options Button

1. Add a checkable button after the S/D/L buttons in `SearchBarInput.qml`, using the icon `icons/32x32/system-uicons--settings.png`.
2. When the button is checked (toggled on), show the advanced search options row below the existing search mode + language filter row.
3. When unchecked, hide the advanced search options row.
4. The button should visually remain in the "checked/pressed" state while the row is open.
5. The button and advanced options row are only enabled when the Suttas search area is selected. When Dictionary or Library is selected, the button is disabled and the row is hidden.

### UI: Advanced Search Options Row

6. The row contains the following controls, laid out horizontally (wrapping if needed):

   **Nikaya filter:**
   - A text input labeled "Nikaya" with placeholder text "e.g. mn, an1"
   - Filters results to those whose nikaya field starts with the entered value.

   **UID filter:**
   - A text input labeled "UID" with placeholder text "e.g. vin"
   - Filters results to those whose uid field starts with the entered value.

   Both text inputs apply their filter with a debounce delay on text change (not requiring Enter).

   **Source checkboxes:**
   - "MS Mūla" checkbox (default: ON)
   - "CST Mūla" checkbox (default: OFF)
   - "CST Commentaries" checkbox (default: ON)

   **Translation options:**
   - "Include CST Commentary in Translations" checkbox (default: from current AppSettings)
   - "Include CST Mūla in Translations" checkbox (default: from current AppSettings)

7. Each checkbox is followed by a small info icon button. Clicking it shows a dialog with the description text currently used in `AppSettingsWindow` for that setting.

### Settings Migration

8. Remove the following checkboxes from `AppSettingsWindow.qml`:
   - "Include CST Commentary in Search Results" (lines 605-619)
   - "Include CST Mūla in Search Results" (lines 621-635)
   - "Include CST Commentary in Translations" (lines 645-659)
   - "Include CST Mūla in Translations" (lines 661-675)

9. The existing `AppSettings` fields are reused:
   - `include_cst_mula_in_search_results` (default: false)
   - `include_cst_commentary_in_search_results` (default: true)
   - `include_cst_commentary_in_translations`
   - `include_cst_mula_in_translations`

10. Add new `AppSettings` fields:
    - `include_ms_mula_in_search_results: bool` (default: true)
    - `search_nikaya_prefix: String` (default: empty)
    - `search_uid_prefix: String` (default: empty)

11. All advanced search option values are persisted to `AppSettings` when changed and restored on app startup.

### Backend: Tantivy FulltextMatch

12. Change the `uid` field tokenizer in the sutta tantivy schema from `simple_fold` to `raw` so that prefix matching works on the full uid string. This requires a manual re-index.

13. In `SearchFilters`, add:
    - `uid_prefix: Option<String>` — for uid "starts with" filtering
    - `nikaya_prefix: Option<String>` — replaces the current exact-match `nikaya: Option<String>` (or rename the existing field and change its query logic)
    - `include_ms_mula: bool` — for filtering MS-sourced mūla texts

14. In `add_sutta_filters()` in `searcher.rs`:
    - For nikaya prefix: use `tantivy::query::RegexQuery` with pattern `^{prefix}` on the `nikaya` field (which uses `raw` tokenizer).
    - For uid prefix: use `tantivy::query::RegexQuery` with pattern `^{prefix}` on the `uid` field (now `raw` tokenizer).
    - For MS Mūla exclusion: add a `MustNot` query for `is_mula=true AND source_uid="ms"`, mirroring the existing CST mūla filter pattern.

### Backend: ContainsMatch (FTS5)

15. Extend the `suttas_fts` virtual table in `appdata-fts5-indexes.sql` to include `nikaya` and `uid` as `UNINDEXED` columns (for filtering, not full-text indexing). This requires a manual re-index of the FTS5 data.

16. Update the FTS5 INSERT trigger and the Rust FTS5 query code in `query_task.rs` to:
    - Add `AND nikaya LIKE '{prefix}%'` when nikaya prefix is set.
    - Add `AND uid LIKE '{prefix}%'` when uid prefix is set.
    - Add MS Mūla exclusion: `AND NOT (source_uid = 'ms' AND uid NOT LIKE '%.att%' AND uid NOT LIKE '%.tik%')` when `include_ms_mula` is false. (Adjust the exact condition based on how MS mūla is identified in the data — MS texts have `source_uid = 'ms'` and `is_mula = true`.)

### Backend: Bridge Layer

17. Pass the new filter values (nikaya prefix, uid prefix, include_ms_mula) from QML through `SearchParams` in the sutta bridge to `SearchQueryTask` and then to `SearchFilters`.

18. Add bridge getter/setter methods on `SuttaBridge` for the new `AppSettings` fields (`include_ms_mula_in_search_results`, `search_nikaya_prefix`, `search_uid_prefix`), following the existing pattern for `include_cst_mula_in_search_results`.

## Non-Goals (Out of Scope)

- Advanced search options for Dictionary or Library search areas (future work).
- Regex or wildcard patterns in the nikaya/uid filter inputs — only simple prefix ("starts with") matching.
- Rewriting the existing CST mūla/commentary filter logic — we reuse the existing pattern and add MS Mūla alongside it.
- Automatic re-indexing of tantivy or FTS5 — the user will manually trigger re-indexing after schema changes.

## Design Considerations

- The advanced options button uses the existing icon at `assets/icons/32x32/system-uicons--settings.png`.
- The advanced options row should match the visual style of the existing search mode/language filter row (same height, spacing, font size).
- Info icon buttons should use a small "i" circle icon or similar, consistent with the app's icon style.
- On narrow screens / mobile, the row contents should wrap gracefully.
- The info dialog can be a simple `MessageDialog` or `Dialog` component with the description text.

## Technical Considerations

- **Tantivy uid field tokenizer change:** The `uid` field in `backend/src/search/schema.rs` must be changed from `simple_fold` to `raw`. This is a breaking change to the tantivy index and requires a full re-index.
- **FTS5 schema change:** Adding `nikaya UNINDEXED` and `uid UNINDEXED` columns to `suttas_fts` in `appdata-fts5-indexes.sql` requires dropping and recreating the virtual table and its triggers, then re-populating.
- **Regex escaping:** User input for nikaya/uid prefix filters must be regex-escaped before being used in `RegexQuery` patterns to prevent injection of arbitrary regex. Alternatively, use tantivy's byte-level prefix scan if available.
- **Key files to modify:**
  - `assets/qml/SearchBarInput.qml` — new button + options row
  - `assets/qml/AppSettingsWindow.qml` — remove migrated checkboxes
  - `backend/src/app_settings.rs` — new fields
  - `backend/src/search/schema.rs` — uid tokenizer change
  - `backend/src/search/types.rs` — SearchFilters additions
  - `backend/src/search/searcher.rs` — prefix query + MS mūla filter logic
  - `backend/src/query_task.rs` — FTS5 query changes + new filter params
  - `bridges/src/sutta_bridge.rs` — pass new params, add getters/setters
  - `scripts/appdata-fts5-indexes.sql` — add nikaya, uid columns

## Success Metrics

- Users can filter sutta search results by nikaya and uid prefix in both FulltextMatch and ContainsMatch modes.
- Source inclusion checkboxes (MS Mūla, CST Mūla, CST Commentaries) correctly filter results.
- All advanced option values persist across app restarts.
- Settings Window no longer contains the migrated checkboxes.
- No regression in search performance or correctness for existing queries.

## Resolved Questions

1. **MS Mūla data identification:** MS texts are identified by `source_uid = "ms"` and `is_mula = true`. There are no MS commentary texts — no separate handling needed.
2. **Prefix filter application:** Nikaya/uid prefix filters apply immediately on text change with debounce (not on Enter).
3. **MS Mūla info text:** Use a simple description similar in style to the existing CST option descriptions.
4. **CST Mūla in Translations:** Yes, move this checkbox to advanced search options as well.
