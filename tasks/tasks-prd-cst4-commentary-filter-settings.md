## Relevant Files

- `backend/src/app_settings.rs` - AppSettings struct definition and Default impl. Add 4 new boolean fields here.
- `backend/src/app_data.rs` - Getter/setter pairs for settings (cache + database persistence). Add 4 getter/setter pairs here.
- `bridges/src/sutta_bridge.rs` - CXX-Qt bridge exposing Rust functions to QML. Add 4 getter/setter bridge functions here.
- `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml` - qmllint type definitions for SuttaBridge. Add function signatures here.
- `assets/qml/AppSettingsWindow.qml` - Settings UI. Add 4 checkboxes in the "Find" section.
- `backend/src/query_task.rs` - Search query logic with source filtering. Add CST4 mūla and commentary filtering here.
- `backend/src/db/appdata.rs` - `get_translations_data_json_for_sutta_uid()` and `sort_suttas()`. Modify to conditionally filter commentary and CST4 mūla.

### Notes

- Follow the existing getter/setter pattern: `get_search_as_you_type()` at `app_data.rs:927` and `set_search_as_you_type()` at `app_data.rs:932`.
- Follow the existing bridge pattern: declaration at `sutta_bridge.rs:474-478`, implementation at `sutta_bridge.rs:2582-2590`.
- Follow the existing QML checkbox pattern: `search_as_you_type_checkbox` at `AppSettingsWindow.qml:550`, loaded at `Component.onCompleted` around line 789.
- Source filtering in search queries currently exists only in `suttas_contains_or_regex_match_page()` at `query_task.rs:627-637`. The `suttas_contains_match_fts5()` function has a `// TODO` for source filtering at line 699.
- `get_translations_data_json_for_sutta_uid()` is at `appdata.rs:136-181`. It currently has no app_settings parameters — the new filter booleans must be passed in.
- `sort_suttas()` is at `appdata.rs:1128-1158`. It buckets `pli` + `/ms` first, then other `pli`, then remaining by language. CST4 mūla will land in `pli_others` which is correct ordering.
- Use `cd backend && cargo test` to run Rust tests. Use `make build -B` to verify compilation.

## Tasks

- [x] 1.0 Add four new boolean fields to `AppSettings` and implement getter/setter pairs in `AppData`
  - [x] 1.1 In `backend/src/app_settings.rs`, add four fields to the `AppSettings` struct: `include_commentary_in_translations: bool`, `include_cst4_mula_in_search_results: bool`, `include_commentary_in_search_results: bool`, `include_cst4_mula_in_translations: bool`.
  - [x] 1.2 In the `Default` impl for `AppSettings` (same file), set defaults: `include_commentary_in_translations: false`, `include_cst4_mula_in_search_results: false`, `include_commentary_in_search_results: true`, `include_cst4_mula_in_translations: false`.
  - [x] 1.3 In `backend/src/app_data.rs`, add a getter for each new field following the `get_search_as_you_type()` pattern (read from `app_settings_cache` via `RwLock::read()`).
  - [x] 1.4 In `backend/src/app_data.rs`, add a setter for each new field following the `set_search_as_you_type()` pattern (write to cache, serialize full struct to JSON, update database row with `diesel::update`).
  - [x] 1.5 Verify compilation with `cd backend && cargo test` (no new tests needed yet, just ensure it compiles).

- [x] 2.0 Expose the four new settings on the SuttaBridge and add qmllint type definitions
  - [x] 2.1 In `bridges/src/sutta_bridge.rs`, add `#[qinvokable]` declarations for 8 new functions (getter + setter for each setting) in the bridge trait block, following the pattern at lines 474-478.
  - [x] 2.2 In `bridges/src/sutta_bridge.rs`, add the implementations for the 8 functions in the impl block, each delegating to the corresponding `AppData` method via `get_app_data()`, following the pattern at lines 2582-2590.
  - [x] 2.3 In `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml`, add function signatures for all 8 new bridge functions with correct types (e.g., `function get_include_commentary_in_translations(): bool { return false; }`).
  - [x] 2.4 Verify compilation with `make build -B`.

- [x] 3.0 Add four checkboxes to the "Find" section of `AppSettingsWindow.qml`
  - [x] 3.1 Add a `CheckBox` with id `include_commentary_in_translations_checkbox`, text "Include Commentary in Translations", and a description `Label` beneath it: "When loading the translations for the current sutta, include the commentaries (Aṭṭhakathā: .att, Ṭīkā: .tik)." Wire `onCheckedChanged` to `SuttaBridge.set_include_commentary_in_translations(checked)`.
  - [x] 3.2 Add a `CheckBox` with id `include_cst4_mula_in_search_results_checkbox`, text "Include CST4 Mūla in Search Results", and a description `Label`: "Include the CST4 (Chaṭṭha Saṅgāyana Tipiṭaka 4) Mūla Pāli texts in the search results. By default the Pāli results only include the MS (Mahāsaṅgīti) sources, since the CST4 versions are often duplicates. This does not filter out CST4 commentary (.att, .tik) records." Wire `onCheckedChanged` to `SuttaBridge.set_include_cst4_mula_in_search_results(checked)`.
  - [x] 3.3 Add a `CheckBox` with id `include_commentary_in_search_results_checkbox`, text "Include Commentary in Search Results", and a description `Label`: "Include commentary (Aṭṭhakathā: .att, Ṭīkā: .tik) records in sutta search results." Wire `onCheckedChanged` to `SuttaBridge.set_include_commentary_in_search_results(checked)`.
  - [x] 3.4 Add a `CheckBox` with id `include_cst4_mula_in_translations_checkbox`, text "Include CST4 Mūla in Translations", and a description `Label`: "When loading translations for the current sutta, include the CST4 Pāli version in addition to the MS Pāli." Wire `onCheckedChanged` to `SuttaBridge.set_include_cst4_mula_in_translations(checked)`.
  - [x] 3.5 In the `Component.onCompleted` block, load initial values for all four checkboxes by calling the corresponding bridge getters (e.g., `include_commentary_in_translations_checkbox.checked = SuttaBridge.get_include_commentary_in_translations()`).
  - [x] 3.6 Verify compilation with `make build -B`.

- [x] 4.0 Implement CST4 mūla and commentary filtering in search query functions
  - [x] 4.1 Determine how search functions access `AppSettings`. The `SearchQueryTask` struct or its calling context needs access to `app_settings_cache`. Check if `AppData` is already accessible in the search path and add the two relevant booleans (`include_cst4_mula_in_search_results`, `include_commentary_in_search_results`) as fields on `SearchQueryTask`, or read them from `get_app_data()` within the query functions.
  - [x] 4.2 In `suttas_contains_or_regex_match_page()` (around line 627 in `query_task.rs`), after the existing source filtering block, add CST4 mūla filtering: when `include_cst4_mula_in_search_results` is `false`, add filter conditions to exclude UIDs matching `%/cst4` that do NOT match `%.att%/cst4`, `%.tik%/cst4`, or `%.xml%/cst4`. Apply to both `query` and `count_query`.
  - [x] 4.3 In the same function, add commentary filtering: when `include_commentary_in_search_results` is `false`, add filter conditions to exclude UIDs matching `%.att/%` (but not `%.xml%`) and `%.tik/%` (but not `%.xml%`). Apply to both `query` and `count_query`.
  - [x] 4.4 In `suttas_contains_match_fts5()` (around line 699), add the same CST4 mūla and commentary filtering logic. This function builds a raw SQL query, so the filters will need to be appended as SQL WHERE clauses using the same LIKE/NOT LIKE patterns.
  - [x] 4.5 Review any other sutta search functions in `query_task.rs` that already have source filtering and apply the same CST4/commentary filters there as well. Added filtering to `suttas_title_match()` and `fulltext_suttas()` (tantivy post-filtering).
  - [x] 4.6 Verify with `cd backend && cargo test` that existing tests pass and the new filtering logic compiles.

- [x] 5.0 Implement CST4 mūla and commentary filtering in translation tab loading
  - [x] 5.1 Modify `get_translations_data_json_for_sutta_uid()` in `backend/src/db/appdata.rs` to accept two boolean parameters: `include_commentary` and `include_cst4_mula`.
  - [x] 5.2 When `include_commentary` is `false`, remove the `.att/%` and `.tik/%` LIKE patterns from the query filter, so only `uid_ref/%` matches are fetched.
  - [x] 5.3 When `include_cst4_mula` is `false`, add a post-query filter (or SQL filter) to exclude records where UID ends with `/cst4` AND the reference part does NOT contain `.att`, `.tik`, or `.xml`. Commentary records like `mn1.att/pli/cst4` must NOT be excluded by this filter.
  - [x] 5.4 Update all callers of `get_translations_data_json_for_sutta_uid()` to pass the two new boolean values, reading them from `app_settings_cache` via `get_app_data()`.
  - [x] 5.5 Verify with `cd backend && cargo test` and `make build -B`.

- [x] 6.0 Verify sort order in `sort_suttas()` places CST4 after MS within the Pāli group
  - [x] 6.1 Review `sort_suttas()` at `appdata.rs:1128-1158`. Confirm that CST4 mūla records (e.g., `mn1/pli/cst4`) land in the `pli_others` bucket (they don't end with `/ms`), which is appended after the `pli/ms` results. This should already produce the correct order: `mn1/pli/ms`, then `mn1/pli/cst4`, then commentaries, then other languages.
  - [x] 6.2 If commentary records (e.g., `mn1.att/pli/cst4`) are not sorting correctly within the Pāli group (they should appear after `pli/cst4` mūla), add secondary sorting within `pli_others` to sort by UID so that mūla comes before `.att` and `.tik`.
  - [x] 6.3 Write a unit test for `sort_suttas()` that verifies the expected ordering: `mn1/pli/ms` → `mn1/pli/cst4` → `mn1.att/pli/cst4` → `mn1.tik/pli/cst4` → `mn1/en/bodhi` → `mn1/en/sujato`.
