## Relevant Files

- `backend/src/app_settings.rs` - AppSettings struct: add `include_ms_mula_in_search_results` field and default
- `backend/src/types.rs` - SearchParams struct: add `nikaya_prefix`, `uid_prefix`, `include_ms_mula` fields
- `backend/src/query_task.rs` - SearchQueryTask: add new fields, FTS5 query filtering for nikaya/uid prefix and MS Mūla
- `backend/src/search/types.rs` - SearchFilters: add `uid_prefix`, `include_ms_mula`, rename `nikaya` to prefix semantics
- `backend/src/search/schema.rs` - Tantivy sutta schema: change `uid` tokenizer from `simple_fold` to `raw`
- `backend/src/search/searcher.rs` - Add RegexQuery prefix filters and MS Mūla exclusion in `add_sutta_filters()`
- `scripts/appdata-fts5-indexes.sql` - Extend `suttas_fts` with `nikaya UNINDEXED` and `uid UNINDEXED` columns
- `bridges/src/sutta_bridge.rs` - Add getter/setter for `include_ms_mula_in_search_results`, pass new params through
- `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml` - Add qmllint type definition for new bridge methods
- `assets/qml/SearchBarInput.qml` - Add advanced search options button and collapsible row with filters/checkboxes
- `assets/qml/SuttaSearchWindow.qml` - Update `get_search_params_from_ui()` to include new filter values
- `assets/qml/AppSettingsWindow.qml` - Remove migrated CST checkbox sections

### Notes

- Tantivy `uid` tokenizer change and FTS5 schema change both require manual re-indexing after implementation.
- Run `cd backend && cargo test` after backend tasks are complete.
- Run `make build -B` to verify compilation after all tasks.
- Nikaya/uid prefix values are ephemeral (per-session), not persisted in AppSettings.

## Tasks

- [ ] 1.0 Backend: Extend AppSettings and SearchParams with new fields
  - [ ] 1.1 In `backend/src/app_settings.rs`, add `include_ms_mula_in_search_results: bool` field to the `AppSettings` struct with a doc comment. Set default to `true` in the `Default` impl.
  - [ ] 1.2 In `backend/src/types.rs`, add `nikaya_prefix: Option<String>`, `uid_prefix: Option<String>`, and `include_ms_mula: bool` fields to the `SearchParams` struct. Set defaults in `Default` impl: `nikaya_prefix: None`, `uid_prefix: None`, `include_ms_mula: true`.
  - [ ] 1.3 In `backend/src/query_task.rs`, add `nikaya_prefix: Option<String>`, `uid_prefix: Option<String>`, and `include_ms_mula: bool` fields to the `SearchQueryTask` struct. Wire them from `SearchParams` in the `new()` constructor (lines ~78-79 area).
  - [ ] 1.4 In `backend/src/search/types.rs`, add `uid_prefix: Option<String>` and `include_ms_mula: bool` to `SearchFilters`. Rename the existing `nikaya: Option<String>` field to `nikaya_prefix: Option<String>` (update all usages). Set defaults: `uid_prefix: None`, `include_ms_mula: true`. Update all call sites that construct `SearchFilters` to use the renamed field.

- [ ] 2.0 Backend: Tantivy schema and query changes
  - [ ] 2.1 In `backend/src/search/schema.rs`, change the `uid` field in `build_sutta_schema()` from `simple_fold` tokenizer to `raw` tokenizer (matching how `nikaya` is already defined). This requires a manual tantivy re-index.
  - [ ] 2.2 In `backend/src/search/searcher.rs` `add_sutta_filters()`, replace the existing exact-match `TermQuery` for `nikaya` (lines 449-454) with a `RegexQuery` using pattern `^{escaped_prefix}` for prefix matching. Ensure the user input is regex-escaped to prevent injection.
  - [ ] 2.3 In `add_sutta_filters()`, add a new uid prefix filter block: when `filters.uid_prefix` is `Some` and non-empty, create a `RegexQuery` with pattern `^{escaped_prefix}` on the `uid` field and add it as `Occur::Must`.
  - [ ] 2.4 In `add_sutta_filters()`, add MS Mūla exclusion: when `!filters.include_ms_mula`, build a `BooleanQuery` for `is_mula=true AND source_uid="ms"` and add it as `Occur::MustNot`, mirroring the existing CST mūla pattern (lines 467-476).
  - [ ] 2.5 Update all test `SearchFilters` constructions in `searcher.rs` tests to include the new fields (`uid_prefix: None`, `include_ms_mula: true`, and `nikaya_prefix` instead of `nikaya`).

- [ ] 3.0 Backend: FTS5 schema and ContainsMatch query changes
  - [ ] 3.1 In `scripts/appdata-fts5-indexes.sql`, add `nikaya UNINDEXED` and `uid UNINDEXED` columns to the `suttas_fts` virtual table definition (after `source_uid UNINDEXED`).
  - [ ] 3.2 Update the `INSERT INTO suttas_fts` population query to include `nikaya` and `uid` columns from the `suttas` table. Verify column names match the actual suttas table schema.
  - [ ] 3.3 Update the three FTS5 triggers (`suttas_fts_insert`, `suttas_fts_update`, `suttas_fts_delete`) to include the `nikaya` and `uid` columns.
  - [ ] 3.4 In `backend/src/query_task.rs`, in the `suttas_contains_match_fts5()` method (lines ~728-800), add nikaya prefix filtering: when `self.nikaya_prefix` is `Some` and non-empty, append `AND f.nikaya LIKE '{prefix}%'` to `extra_where`. Use parameterized queries or proper escaping.
  - [ ] 3.5 In the same method, add uid prefix filtering: when `self.uid_prefix` is `Some` and non-empty, append `AND f.uid LIKE '{prefix}%'` to `extra_where`.
  - [ ] 3.6 In the same method, add MS Mūla exclusion: when `!self.include_ms_mula`, append `AND NOT (f.source_uid = 'ms')` to `extra_where`. (MS has no commentaries, so excluding source_uid='ms' when mūla is excluded is sufficient.)
  - [ ] 3.7 Apply the same nikaya prefix, uid prefix, and MS Mūla exclusion filtering to other sutta query methods that use `extra_where` or diesel filters: `suttas_title_match()` (lines ~1275-1342) and any other sutta search paths that apply CST filtering.

- [ ] 4.0 Bridge: Add getter/setter methods and pass new params
  - [ ] 4.1 In `bridges/src/sutta_bridge.rs`, add QML-callable method declarations in the `extern "RustQt"` block for `get_include_ms_mula_in_search_results() -> bool` and `set_include_ms_mula_in_search_results(enabled: bool)`, following the existing pattern (lines ~607-628).
  - [ ] 4.2 Implement the getter/setter methods in the impl block (lines ~2941-2979 area), delegating to `app_data.get_include_ms_mula_in_search_results()` / `app_data.set_include_ms_mula_in_search_results()`. Ensure the corresponding AppData methods exist or add them.
  - [ ] 4.3 In `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml`, add qmllint type definitions for the new bridge methods: `get_include_ms_mula_in_search_results(): bool` and `set_include_ms_mula_in_search_results(enabled: bool)`.
  - [ ] 4.4 Verify that the `SearchParams` JSON deserialization in `results_page()` correctly picks up the new `nikaya_prefix`, `uid_prefix`, and `include_ms_mula` fields from the params JSON string passed by QML.

- [ ] 5.0 UI: Add advanced search options button and row to SearchBarInput
  - [ ] 5.1 In `SearchBarInput.qml`, add a new checkable `Button` after the S/D/L buttons row (after line 178), using `icons/32x32/system-uicons--settings.png` as the icon. The button should be `checkable: true` and only `enabled` when `search_area === "Suttas"`. Add a property `advanced_options_visible` bound to the button's checked state and search area.
  - [ ] 5.2 Add a new `Row` or `RowLayout` below the `search_options_layout` (after line 271) that is visible only when `advanced_options_visible` is true. This row contains all the advanced search option controls.
  - [ ] 5.3 Add two `TextField` inputs in the advanced options row: "Nikaya" (placeholder: "e.g. mn, an1") and "UID" (placeholder: "e.g. vin"). Add a `Timer` component for debounce (~300ms) that triggers search on text change.
  - [ ] 5.4 Add three checkboxes: "MS Mūla" (default: checked, bound to `SuttaBridge.get_include_ms_mula_in_search_results()`), "CST Mūla" (bound to `SuttaBridge.get_include_cst_mula_in_search_results()`), "CST Commentaries" (bound to `SuttaBridge.get_include_cst_commentary_in_search_results()`). Each checkbox calls the corresponding SuttaBridge setter on change.
  - [ ] 5.5 Add two translation checkboxes: "Include CST Commentary in Translations" (bound to `SuttaBridge.get_include_cst_commentary_in_translations()`) and "Include CST Mūla in Translations" (bound to `SuttaBridge.get_include_cst_mula_in_translations()`). Each calls the corresponding setter on change.
  - [ ] 5.6 Add small info icon buttons next to each checkbox. Clicking an info button opens a `Dialog` or `MessageDialog` showing the description text (reuse the label text from the current AppSettingsWindow for CST options; write a similar short description for MS Mūla).
  - [ ] 5.7 Expose properties or signals from SearchBarInput so the parent (`SuttaSearchWindow`) can read the current nikaya_prefix, uid_prefix, and checkbox values when constructing search params. Also expose a signal (e.g. `advanced_options_changed`) that the parent can connect to for re-triggering search on debounced changes.

- [ ] 6.0 UI: Remove migrated settings from AppSettingsWindow
  - [ ] 6.1 In `assets/qml/AppSettingsWindow.qml`, remove the "Include CST Commentary in Search Results" checkbox and its description label (lines ~605-619).
  - [ ] 6.2 Remove the "Include CST Mūla in Search Results" checkbox and its description label (lines ~621-635).
  - [ ] 6.3 Remove the "Include CST Commentary in Translations" checkbox and its description label (lines ~645-659).
  - [ ] 6.4 Remove the "Include CST Mūla in Translations" checkbox and its description label (lines ~661-675).
  - [ ] 6.5 Remove the initialization code for these four checkboxes in the `Component.onCompleted` handler (lines ~888-891).

- [ ] 7.0 Integration: Wire advanced options into search flow
  - [ ] 7.1 In `assets/qml/SuttaSearchWindow.qml`, update `get_search_params_from_ui()` (lines ~506-539) to read `nikaya_prefix` and `uid_prefix` from the SearchBarInput text fields and `include_ms_mula` from the MS Mūla checkbox, and include them in the returned params object.
  - [ ] 7.2 Connect the `advanced_options_changed` signal (or debounce timer) from SearchBarInput to re-trigger search in SuttaSearchWindow, so that changing a filter or checkbox re-executes the current query with updated params.
  - [ ] 7.3 Verify the full flow: QML params JSON → bridge `results_page()` → `SearchQueryTask` → tantivy `SearchFilters` / FTS5 `extra_where` — ensure all new fields are threaded through correctly for both FulltextMatch and ContainsMatch modes.
  - [ ] 7.4 Build the project with `make build -B` to verify compilation. Run `cd backend && cargo test` to verify backend tests pass.
