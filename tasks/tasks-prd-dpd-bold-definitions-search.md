# Tasks: DPD Bold Definitions in Dictionary Search

Source PRD: [prd-dpd-bold-definitions-search.md](./prd-dpd-bold-definitions-search.md)

## Relevant Files

### Migration & schema (Rust + SQL)
- `backend/src/db/dpd_schema.rs` — Diesel schema for `bold_definitions`; add `uid` and `commentary_plain` columns.
- `backend/src/db/dpd_models.rs` — Diesel models for `bold_definitions` (struct at ~line 271 and related struct at ~line 578).
- `backend/src/db/dpd.rs` — **at-startup** DPD DB setup; currently runs `dpd-btree-indexes.sql` (~line 746) and `dpd-fts5-indexes.sql` (~line 786). Add a line to also run the new `dpd-bold-definitions-fts5-indexes.sql`. **Does NOT do column population** — that lives in the CLI bootstrap path.
- `scripts/dpd-btree-indexes.sql` — extend with `CREATE UNIQUE INDEX … idx_bold_definitions_uid …`.
- `scripts/dpd-bold-definitions-fts5-indexes.sql` — NEW. Two FTS5 trigram virtual tables: `bold_definitions_fts` (on `commentary_plain`) and `bold_definitions_bold_fts` (on `bold`).
- `backend/src/helpers.rs` — existing `run_fts5_indexes_sql_script` helper; reused.

### Bootstrap (one-shot `dpd.sqlite3` production)
- `cli/src/bootstrap/dpd.rs` — DPD bootstrap orchestration; currently calls `create_dictionaries_fts5_indexes`. Add: (a) `populate_bold_definitions_derived_columns` step for uid/commentary_plain; (b) call `build_bold_definitions_index` with `lang = "pli"`.
- `cli/src/main.rs` — exposes bootstrap subcommands.

### Fulltext (tantivy) index
- `backend/src/search/indexer.rs` — contains `build_sutta_index`, `build_dict_index`, `build_library_index`, `build_all_indexes`. Add `build_bold_definitions_index`. Uses `register_tokenizers(&index, lang)`.
- `backend/src/search/schema.rs` — contains `build_sutta_schema`, `build_dict_schema`, `build_library_schema`. Add `build_bold_definitions_schema`.
- `backend/src/search/searcher.rs` — fulltext searcher factory (see `IndexType` dispatch ~line 124); register the new index.
- `backend/src/lib.rs` — `AppGlobalPaths` (line ~250) with index dirs, `init_fulltext_searcher` / `with_fulltext_searcher`. Add `bold_definitions_index_dir`.

### Normalization pipeline
- `backend/src/html_content.rs` and/or helpers used by the sutta `content_plain` pipeline — reuse for deriving `commentary_plain` from `commentary`.

### Search query
- `backend/src/query_task.rs` — Dictionary search modes (branches at ~lines 1711, 1738, 1752, 1767, 1786 for modes; `fulltext_dict_words` at ~line 1904). Add bold-definitions branches for DPD Lookup / Headword Match / Contains Match / Fulltext Match; apply UID suffix filter.
- `backend/src/types.rs` — `SearchParams` / `SearchResult` types; add `uid_suffix` and `include_comm_bold_definitions` fields; add a result kind/source marker if needed.

### Rendering
- `backend/src/html_content.rs` — HTML generation; add `render_bold_definition` (header + body + footer).
- `backend/src/app_data.rs` — `render_word_html_by_uid` at line 332 is the single assembly site for dictionary HTML; extend to dispatch bold-definition uids to `render_bold_definition`. Called from `bridges/src/sutta_bridge.rs:1571` and `bridges/src/api.rs:622`.
- `assets/qml/DictionaryHtmlView.qml`, `DictionaryHtmlView_Desktop.qml`, `DictionaryHtmlView_Mobile.qml` — display of rendered result HTML (should need no changes if HTML is self-contained).
- `assets/css/dictionary.css` (and `assets/sass/` source) — styling for `.bold-definition-header`, `.bold-definition-footer`, `.bold-definition-body .headword`.

### UI — search bar & advanced options
- `assets/qml/SearchBarInput.qml` — add `uid_suffix_input` beside existing `uid_prefix_input` (~line 389); add `include_comm_bold_definitions_checkbox` modelled on `include_ms_mula_checkbox` (~line 403); expose `uid_suffix` property.
- `bridges/src/sutta_bridge.rs` — add `get_include_comm_bold_definitions_in_search_results` / `set_…` (mirror pair at ~lines 703/706 and ~3009/3014); pass `uid_suffix` and `include_comm_bold_definitions` through `SearchParams` (~lines 1094, 1477).
- `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml` — add qmllint stubs for the two new bridge functions.
- `backend/src/app_settings.rs` — add persisted setting `include_comm_bold_definitions_in_search_results` (and its app_data getter/setter in `backend/src/app_data.rs`).
- `backend/src/app_data.rs` — `get_include_comm_bold_definitions_in_search_results` / `set_…` wrappers.

### Tests
- `backend/tests/test_query_task.rs` — add tests: each of the four Dictionary search modes returns bold-definition hits; UID suffix filter restricts results; checkbox off excludes bold results.
- `backend/tests/test_dpd_lookup.rs` — add test for substring match on `bold`.
- `backend/tests/helpers/mod.rs` — may need a helper to open a `dpd.sqlite3` fixture with migrated schema.
- New: `backend/tests/test_bold_definitions_migration.rs` — smoke test that uid is unique, lowercased, and row count matches (run against local migrated DB).

### Notes
- Run Rust backend tests with `cd backend && cargo test`; run a single test with `cd backend && cargo test test_name`.
- Use `make build -B` for compilation checks (per CLAUDE.md; do not use direct cmake).
- Skip `make qml-test` unless explicitly requested (per user preference).
- Use `try_exists()` (not `.exists()`) in any filesystem check in bootstrap/migration code (CLAUDE.md Android rule).
- `SIMSAPA_DIR` runtime data is at `/home/gambhiro/prods/apps/simsapa-ng-project/bootstrap-assets-resources/dist/simsapa-ng`; migrated `dpd.sqlite3` lives at `…/app-assets/dpd.sqlite3`.

---

## Tasks

- [x] 1.0 Bootstrap migration: extend `bold_definitions` schema with `uid` and `commentary_plain`
  - [x] 1.1 Add a new bootstrap step in `cli/src/bootstrap/dpd.rs` (call it `populate_bold_definitions_derived_columns`) that opens the in-progress `dpd.sqlite3` and performs the population described in 1.2–1.4. **This belongs in the CLI bootstrap path, not in the at-startup `backend/src/db/dpd.rs` path.** The at-startup path must remain unchanged except for running the new SQL index scripts (which are already idempotent via `DROP … IF EXISTS`).
  - [x] 1.2 In that bootstrap step, add `uid TEXT` and `commentary_plain TEXT` columns to `bold_definitions` via `ALTER TABLE … ADD COLUMN IF NOT EXISTS` (or a `pragma table_info` pre-check, since SQLite's `ADD COLUMN` doesn't support `IF NOT EXISTS` directly).
  - [x] 1.3 Read all `bold_definitions` rows `ORDER BY id`, computing `uid = lowercase(bold) + "/" + lowercase(ref_code)`, and for each collision of the lowercased `(bold, ref_code)` pair append ` N/{ref_code}` with `N = 2, 3, …` (i.e. `lowercase(bold) + " " + N + "/" + lowercase(ref_code)`). Track seen counts in a `HashMap<(String, String), u32>`.
  - [x] 1.4 Compute `commentary_plain` for each row by running the same normalization pipeline used for sutta `content_plain` (HTML stripped, Pāli diacritics normalized, whitespace collapsed — reuse the existing function; do not duplicate logic).
  - [x] 1.5 UPDATE both columns in a single transaction. After successful commit, rely on the at-startup SQL index scripts to create the indexes (see 1.6 and task 2).
  - [x] 1.6 Extend `scripts/dpd-btree-indexes.sql` with `CREATE UNIQUE INDEX IF NOT EXISTS idx_bold_definitions_uid ON bold_definitions(uid);`. (No separate btree on `bold` — substring matching on `bold` is handled by the trigram FTS5 in task 2.2, not a btree.)
  - [x] 1.7 Update `backend/src/db/dpd_schema.rs` to add `uid -> Text` and `commentary_plain -> Text` to the `bold_definitions` table definition.
  - [x] 1.8 Update `backend/src/db/dpd_models.rs` structs at lines ~271 and ~578 to include the two new fields (both Diesel select + insert structs).
  - [x] 1.9 Make the bootstrap step idempotent: early-return if all rows already have non-empty `uid` (so re-running bootstrap is safe).

- [x] 2.0 FTS5 trigram indexes over `bold_definitions.commentary_plain` and `bold_definitions.bold`
  - [x] 2.1 Create `scripts/dpd-bold-definitions-fts5-indexes.sql` defining **two** FTS5 virtual tables, each following the pattern in `scripts/dpd-fts5-indexes.sql`:
    - `bold_definitions_fts(bold_definitions_id UNINDEXED, commentary_plain, tokenize='trigram', detail='none')` — for Contains Match.
    - `bold_definitions_bold_fts(bold_definitions_id UNINDEXED, bold, tokenize='trigram', detail='none')` — for DPD Lookup / Headword Match substring on the bold term (index-accelerated, avoids full scans over 360k rows).
    Include for each: `DROP TRIGGER/TABLE IF EXISTS`, `CREATE VIRTUAL TABLE`, populate via `INSERT…SELECT`, INSERT/UPDATE/DELETE triggers, `INSERT INTO <tbl>(<tbl>) VALUES('optimize')`, and a single `VACUUM` at the end.
  - [x] 2.2 In `backend/src/db/dpd.rs` (the at-startup path), run this new SQL script via `run_fts5_indexes_sql_script` after the existing `dpd-fts5-indexes.sql` step (~line 786). This path is idempotent thanks to the `DROP … IF EXISTS` in the script.
  - [x] 2.3 Mirror the existing script-path resolution pattern at line ~786 exactly.

- [x] 3.0 Tantivy fulltext index for bold definitions (with Pāli tokenizer)
  - [x] 3.1 In `backend/src/lib.rs` `AppGlobalPaths`, add `pub bold_definitions_index_dir: PathBuf` and populate it as `index_dir.join("dpd_bold_definitions")` (mirror `dict_words_index_dir`).
  - [x] 3.2 In `backend/src/search/schema.rs`, add a `build_bold_definitions_schema(lang)` factory (modelled on `build_dict_schema`) with fields: `bold_definitions_id` (INDEXED + STORED as i64), `uid` (STRING + STORED), `bold` (STRING + STORED), `commentary_plain` (TEXT with the Pāli tokenizer + STORED for snippets).
  - [x] 3.3 In `backend/src/search/indexer.rs`, add `pub fn build_bold_definitions_index(dpd_db: &DatabaseHandle, index_dir: &Path, lang: &str) -> Result<()>` modelled on `build_dict_index`. Pass `lang = "pli"` at the call site — commentary is Pāli, and this matches the tokenizer used for DPD dictionary entries.
  - [x] 3.4 Populate the index by iterating `bold_definitions` rows and writing one document per row using `commentary_plain` as the indexed text. Use the same writer patterns (`wait_merging_threads`, `sync_directory`) as `build_sutta_index`. Call `register_tokenizers(&index, lang)` like the other builders.
  - [x] 3.5 Register the new index in `build_all_indexes` in `backend/src/search/indexer.rs` so bootstrap produces it. Clean the `bold_definitions_index_dir` before writing so re-runs start fresh (mirror the pattern used by the existing builders).
  - [x] 3.6 In `backend/src/search/searcher.rs` (and `init_fulltext_searcher` / `with_fulltext_searcher` in `backend/src/lib.rs`), register a searcher over the new index directory so `query_task.rs` can query it. Use `build_bold_definitions_schema(lang)` for reader schema resolution, mirroring how `Dict` resolves via `build_dict_schema(lang)` (see `searcher.rs:124`).
  - [x] 3.7 Add a CLI bootstrap entry in `cli/src/bootstrap/dpd.rs` calling `build_bold_definitions_index` after the SQL migration step, using `paths.bold_definitions_index_dir` and `lang = "pli"`.

- [x] 4.0 Search integration in `query_task.rs` for all four Dictionary modes
  - [x] 4.1 Extend `backend/src/types.rs` `SearchParams` with `pub uid_suffix: Option<String>` and `pub include_comm_bold_definitions: bool`. Extend `SearchResult` with a `source: ResultSource` marker (`DictWord`, `Sutta`, `LibraryItem`, `BoldDefinition`) so callers can route rendering. **Every `SearchResult` variant must carry a `uid` field** (verify/adjust existing variants if any are missing), so the UID prefix/suffix filter can apply uniformly across search areas.
  - [x] 4.2 At `SearchParams` construction sites in `bridges/src/sutta_bridge.rs` (~lines 1094, 1477), set `include_comm_bold_definitions` by reading the persisted setting (`app_data.get_include_comm_bold_definitions_in_search_results()`) — do not rely on a Rust-side default. Set `uid_suffix` from the QML-supplied value.
  - [x] 4.3 Add a helper `fn query_bold_definitions_bold_fts5(query: &str) -> Vec<SearchResult>` in `backend/src/query_task.rs` using the `bold_definitions_bold_fts` virtual table (FTS5 MATCH on trigram index) joined back to `bold_definitions`. Use parameterized queries (no string concatenation of user input). The user's query is lowercased for matching but **not** passed through the Pāli normalization pipeline (DPD Lookup / Headword Match operate on the as-stored `bold` field).
  - [x] 4.4 Add a helper `fn query_bold_definitions_commentary_fts5(normalized_query: &str) -> Vec<SearchResult>` using the `bold_definitions_fts` virtual table (FTS5 MATCH on trigram index) joined back to `bold_definitions`.
  - [x] 4.5 Add a helper `fn query_bold_definitions_fulltext(normalized_query: &str, page_num: usize) -> Vec<(f32, SearchResult)>` using the tantivy searcher over `bold_definitions_index_dir`; return score + result.
  - [x] 4.6 In the DPD Lookup branch (`dpd_lookup` at `query_task.rs:1714`), after existing headword results, append bold-definition substring results (call 4.3). Gate on `params.include_comm_bold_definitions`.
  - [x] 4.7 In the Headword Match branch (`lemma_1_dpd_headword_match_fts5` at `query_task.rs:1787`), after existing results, append bold-definition substring results (call 4.3). Gate on `include_comm_bold_definitions`.
  - [x] 4.8 In the Contains Match branch (`dict_words_contains_match_fts5` at `query_task.rs:1753`), normalize the query first (reuse the existing sutta normalization), then append bold-definition results (call 4.4). Gate on `include_comm_bold_definitions`.
  - [x] 4.9 In `fulltext_dict_words` (~line 1904), after collecting regular dict-word scored hits, also collect bold-definition scored hits (call 4.5), merge the two lists by descending BM25 score, and paginate. Accept that inter-index scores are not strictly comparable — some bias is acceptable per PRD §4.3.12. Gate on `include_comm_bold_definitions`. Normalize the query for tantivy like the existing path does.
  - [x] 4.10 Apply the `uid_suffix` filter **after** assembling results, across all search areas (not just Dictionary) and both regular + bold results: `if let Some(sfx) = &params.uid_suffix { let s = sfx.to_lowercase(); results.retain(|r| r.uid.to_lowercase().ends_with(&s)); }`.

- [x] 5.0 HTML rendering for bold-definition results
  - [x] 5.1 In `backend/src/html_content.rs`, add `pub fn render_bold_definition(row: &BoldDefinition) -> String` producing:
    - header `<div class="bold-definition-header">{nikaya} › {book} ({ref_code}) › {title} › {subhead}</div>`
    - body `<div class="bold-definition-body"><span class="headword">{bold}</span> {commentary}</div>` (commentary HTML preserved)
    - footer `<div class="bold-definition-footer">{file_name}</div>`
  - [x] 5.2 Extend `render_word_html_by_uid` in `backend/src/app_data.rs:332` to detect bold-definition uids (by lookup in the `bold_definitions` table via the DPD DB) and dispatch to `render_bold_definition`; fall through to existing dict-word rendering otherwise. This is the single assembly site already used for dictionary HTML (called from `bridges/src/sutta_bridge.rs:1571` and `bridges/src/api.rs:622`).
  - [x] 5.3 Add matching styles to `assets/sass/dictionary.scss` (or existing dictionary sass source) for `.bold-definition-header` (small, muted breadcrumb), `.bold-definition-footer` (small, muted), and ensure `.bold-definition-body .headword` matches existing headword weight. Run `make sass`.
  - [x] 5.4 Confirm `commentary_plain` is not derived from the rendered HTML — it comes only from the raw `commentary` field via the normalization pipeline (already enforced in task 1.4).

- [x] 6.0 Advanced search UI — "UID suffix" filter (all search areas)
  - [x] 6.1 In `assets/qml/SearchBarInput.qml`, widen `advanced_options_visible` at line 36 so the UID prefix, UID suffix, and common filters are visible across all search areas (not just `"Suttas"`). Keep per-area-specific checkboxes (MS Mūla etc.) gated where appropriate.
  - [x] 6.2 Add a TextField `uid_suffix_input` immediately after `uid_prefix_input` (~line 389), with placeholder text "UID suffix (e.g. /vvt)". Bind `onTextChanged: advanced_options_debounce_timer.restart()`.
  - [x] 6.3 Add a readonly property `uid_suffix: uid_suffix_input.text.trim().toLowerCase()` alongside existing `uid_prefix` (~line 38).
  - [x] 6.4 Include `uid_suffix` in the signal payload wired by `advanced_options_changed()` so consumers pick it up.
  - [x] 6.5 Propagate `uid_suffix` from the QML search pipeline into `SearchParams` in `bridges/src/sutta_bridge.rs` (both construction sites at ~lines 1094 and 1477).

- [x] 7.0 Advanced search UI — `include_comm_bold_definitions_checkbox` (with info button)
  - [x] 7.1 In `backend/src/app_settings.rs`, add a persisted boolean setting `include_comm_bold_definitions_in_search_results` (default `true`).
  - [x] 7.2 In `backend/src/app_data.rs`, add `get_include_comm_bold_definitions_in_search_results` and `set_…` wrappers, modelled on the existing `include_ms_mula` pair.
  - [x] 7.3 In `bridges/src/sutta_bridge.rs`, add the CXX-Qt bridge fns `get_include_comm_bold_definitions_in_search_results` / `set_…` (mirror lines ~703/706 and ~3009/3014).
  - [x] 7.4 In `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml`, add qmllint stubs for both new functions with simple literal return values (per CLAUDE.md convention).
  - [x] 7.5 In `assets/qml/SearchBarInput.qml`, inside a `RowLayout { spacing: 2 }` block mirroring the existing `include_ms_mula_checkbox` block (~lines 402–423):
    - Add a `CheckBox { id: include_comm_bold_definitions_checkbox }` with `text: "Dictionary Commentary Definitions in Search"`, `checked:` initialized from `SuttaBridge.get_include_comm_bold_definitions_in_search_results()`, and `onCheckedChanged` calling the bridge setter + `root.advanced_options_changed()`.
    - Add a sibling flat `Button { icon.source: "icons/32x32/fa_circle-info-solid.png" }` that, on click, sets `info_dialog.title = "Dictionary Commentary Definitions in Search"` and `info_dialog.message = "Also search bold-highlighted terms extracted from Pāli commentaries (DPD bold definitions). Turn off for headword-only results."` and calls `info_dialog.open()`.
  - [x] 7.6 Gate the new checkbox's visibility to `search_area === "Dictionary"` (the checkbox is Dictionary-specific; UID prefix/suffix remain visible across areas from task 6.1).
  - [x] 7.7 In the `SearchParams` construction sites in `bridges/src/sutta_bridge.rs` (~lines 1094, 1477), read `include_comm_bold_definitions` from the persisted setting rather than using a hard default (covered also by task 4.2).

- [ ] 8.0 Tests & verification
  - [ ] 8.1 Add `backend/tests/test_bold_definitions_migration.rs` against the real migrated `dpd.sqlite3` at `/home/gambhiro/prods/apps/simsapa-ng-project/bootstrap-assets-resources/dist/simsapa-ng/app-assets/dpd.sqlite3`: assert `COUNT(*) == COUNT(DISTINCT uid)`, every uid is lowercase, every uid contains at least one `/`, row count matches source DPD.
  - [ ] 8.2 In `backend/tests/test_query_task.rs`, add a test per Dictionary mode (DPD Lookup, Headword Match, Contains Match, Fulltext Match) asserting at least one bold-definition result is returned for a known commentary-only query term.
  - [ ] 8.3 Add a test that `include_comm_bold_definitions = false` excludes all bold-definition results across the four modes.
  - [ ] 8.4 Add a test that `uid_suffix = "/vvt"` restricts results to those whose uid ends with `/vvt` (both regular and bold), across Dictionary and Suttas search areas.
  - [ ] 8.5 Add a test that every `SearchResult` returned from each search area carries a non-empty `uid` (so UID prefix/suffix can apply uniformly per PRD §4.5.16).
  - [ ] 8.6 Run `cd backend && cargo test` and fix failures. Only after all sub-tasks of this task are done.
  - [ ] 8.7 Run `make build -B` and confirm a clean build.
  - [ ] 8.8 Update `PROJECT_MAP.md` with: new tantivy index dir (`dpd_bold_definitions`), new SQL script (`dpd-bold-definitions-fts5-indexes.sql`), new bridge functions, new bootstrap step — per CLAUDE.md directive to keep it current.
