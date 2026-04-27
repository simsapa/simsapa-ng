## Relevant Files

- `backend/src/search/schema.rs` - Tantivy schema definitions; needs new `uid_rev` / `spine_item_uid_rev` raw fields, and stale "simple_fold over-match" comments removed.
- `backend/src/search/indexer.rs` - Indexing pipeline; emits the lowercased uid + reversed-uid per doc; also where the dead bold-definitions index plumbing (`build_bold_definitions_index`, `bold_definitions_index_dir`) lives.
- `backend/src/search/searcher.rs` - Houses `add_dict_filters` / `add_library_filters` / `add_sutta_filters` (searcher.rs:533, 610, 639), the `IndexType::BoldDefinitions` arms (lines 111, 162, 375, 490, 524), `fulltext_dict`, `dict_doc_to_result` (line 691), `bold_definition_doc_to_result`, `open_single_index`, and the `bold_index*` fields.
- `backend/src/search/types.rs` - `IndexType` enum; drop the `BoldDefinitions` variant.
- `backend/src/query_task.rs` - FTS5 helpers, `results_page` (line 1816), `apply_uid_filters` (line 1791), `cached_full_fetch` (line 110), `SAFETY_LIMIT_SQL` / `SAFETY_LIMIT_TANTIVY` (lines 22, 34), `db_query_hits_count` bookkeeping, `fulltext_dict_words_all`, `query_bold_definitions_fulltext_all`, the `_all` wrapper helpers.
- `cli/src/bootstrap/` - Bootstrap pipeline; drop the separate bold-definitions index step; one re-bootstrap run after Stage 1's schema change.
- `backend/tests/test_uid_suffix_and_bold_ascii.rs` - Existing uid-suffix + bold ASCII coverage; must keep passing through every stage.
- `backend/tests/test_search_filter_pagination.rs` - Pagination + timing budgets; tightened post-refactor; gains assertions that the push-down path is exercised, not full-fetch.
- `backend/tests/test_pagination_invariants.rs` - **New**: every page except the last has exactly `page_len`; pages are contiguous (analysis §8.1).
- `backend/tests/test_bold_definitions_highlighting.rs` - **New**: a diacritic query yields highlight spans on bold-definition rows (analysis §8.4).
- `backend/tests/test_search.rs` / `test_fulltext_search_results.rs` - Smoke coverage for the unified dict + bold path; verify after Stage 4.
- `tasks/prd-dpd-bold-definitions-search.md` - Update §4.1 / §4.2 / §7 to reflect the consolidated index.
- `tasks/analysis-dpd-bold-definitions-search.md` - The superseded design doc; cross-link to this refactor.
- `PROJECT_MAP.md` - Refresh entries for the search pipeline now that the bold-definitions index and full-fetch machinery are gone.

### Notes

- Each stage must compile and have its tests passing before moving to the next; storage-layer push-down lands before pipeline simplification so filters never silently drop.
- Run Rust tests with `cd backend && cargo test`. Run a single test with `cd backend && cargo test <test_name>`.
- After Stage 1's schema/indexer change, a re-bootstrap of the index is required before the searcher is flipped to query `uid_rev` / `spine_item_uid_rev`.
- Per project guidance: do not run `make qml-test`; use `make build -B` for compile checks; only run the test suite after all sub-tasks of a top-level task are complete; targeted Edits, not bulk `sed`, when renaming symbols.

## Tasks

- [x] 1.0 Stage 1 — Tantivy push-down for uid prefix + suffix (schema + indexer + searcher)
  - [x] 1.1 Add a `uid_rev` raw field to `build_sutta_schema` and `build_dict_schema`, and a `spine_item_uid_rev` raw field to `build_library_schema` in `backend/src/search/schema.rs`; use `raw_opts` so terms are stored exactly as written.
  - [x] 1.2 In `backend/src/search/indexer.rs`, when emitting each doc, write the lowercased uid into the original field and the character-reversed lowercased uid into the new `*_rev` field for sutta, dict, and library docs (bold-definition docs included, since they live in the dict index).
  - [x] 1.3 Re-bootstrap the index once against the updated schema (via the existing bootstrap CLI step) so `uid_rev` / `spine_item_uid_rev` are populated before the searcher starts querying them.
  - [x] 1.4 In `backend/src/search/searcher.rs`, introduce a single `add_uid_filters(subqueries, filters, schema, uid_field_name, uid_rev_field_name)` helper implementing the §3.2 push-down: lowercase the input, escape it with `regex::escape`, push `RegexQuery::from_pattern("{prefix}.*", uid_field)` for `uid_prefix`, and `RegexQuery::from_pattern("{reversed_suffix}.*", uid_rev_field)` for `uid_suffix`.
  - [x] 1.5 Replace the bodies of `add_sutta_filters`, `add_dict_filters`, and `add_library_filters` to call `add_uid_filters` with the right `(uid_field, uid_rev_field)` pair, keeping any non-uid filtering they already do.
  - [x] 1.6 Delete the stale "simple_fold over-match" / "Rust-side `apply_uid_filters` is the source of truth" comments around the old `add_dict_filters` / `add_library_filters` (searcher.rs:625, 645).
  - [x] 1.7 Add a unit/integration test that builds a small in-memory tantivy index with mixed uids, runs a query with `uid_prefix = "an"` and `uid_suffix = "1.1"`, and asserts the `Count` collector total equals the expected count (no over-match, no full-corpus pass).

- [x] 2.0 Stage 2 — SQL push-down for uid prefix + suffix in every FTS5 helper, with parallel COUNT
  - [x] 2.1 Refactor `suttas_contains_match_fts5` in `backend/src/query_task.rs` to accept `page_num` / `page_len`, append conditional `AND suttas.uid LIKE ?` clauses for prefix and suffix when set, run `LIMIT ? OFFSET ?`, and execute a sibling `SELECT COUNT(*)` with the same predicate; return `(Vec<SearchResult>, total: usize)`.
  - [x] 2.2 Apply the same shape to `dict_words_contains_match_fts5` (which previously had no prefix push-down) — both prefix and suffix on `dict_words.uid`, paginated, with parallel COUNT.
  - [x] 2.3 Apply the same shape to `book_spine_items_contains_match_fts5` — both prefix and suffix on `book_spine_items.spine_item_uid`, paginated, with parallel COUNT.
  - [x] 2.4 Add suffix push-down (and confirm prefix) to `query_bold_definitions_bold_fts5` and `query_bold_definitions_commentary_fts5`; both paginated and counted server-side.
  - [x] 2.5 Add prefix + suffix push-down to the SQL backing `dpd_lookup` and `lemma_1_dpd_headword_match_fts5`, paginated and counted; the bold-definition append still flows through the helpers above (which now push everything down themselves). **Updated:** `db::dpd::dpd_lookup` now takes `uid_prefix` / `uid_suffix` parameters and pushes `AND uid LIKE ?` down at every per-phase Diesel query against `dpd_headwords` / `dpd_roots` (and the helpers `inflection_to_pali_words`, `dpd_deconstructor_to_pali_words`); the multi-phase fallback still materialises the dedup union in memory but now over a SQL-filtered candidate set.
  - [x] 2.6 Add prefix + suffix push-down to the SQL title-match paths (`suttas_title_match_*`, `library_title_match_*`); paginated with parallel COUNT.
  - [x] 2.7 Drop the `'%'` no-op pattern hack around the old "results that survive `apply_uid_filters`" comment (query_task.rs:752); helpers no longer need to anticipate a Rust post-filter.
  - [x] 2.8 Stop helpers from writing `self.db_query_hits_count` directly; each helper instead returns its `total` to the caller. Keep their `SAFETY_LIMIT_SQL` warning logs for now (Stage 3 reconsiders the cap).
  - [x] 2.9 Run `cargo test test_uid_suffix_and_bold_ascii` and `cargo test test_search_filter_pagination` — both must still pass; the pagination test now exercises real per-page SQL rather than slicing a cached set.

- [ ] 3.0 Stage 3 — Pipeline simplification: filter-aware `results_page` dispatch and removal of full-fetch / cache machinery
  - [x] 3.1 Rewrite `results_page` in `backend/src/query_task.rs` to dispatch on `(SearchMode, SearchArea)` per §3.1, calling per-mode handlers that take `page_num` and return `(Vec<SearchResult>, total: usize)`.
  - [x] 3.2 Convert each existing mode path into a `(page, total)` handler: `fulltext_suttas`, `fulltext_dict` (still merges with the legacy bold tantivy index until Stage 4 collapses it), `fulltext_library`, the three FTS5 contains-match handlers, `dpd_lookup`, `lemma_1_dpd_headword_match_fts5`, `suttas_title_match`, `library_title_match`, `uid_match`.
  - [x] 3.3 Write `self.db_query_hits_count = total as i64` exactly once at the bottom of `results_page`, after the dispatch returns; remove the per-helper assignments and the `-1` sentinel mechanism.
  - [x] 3.4 Apply highlighting (`highlight_query_in_content` with `normalize_plain_text(&self.query_text)`) only to the page-sized result set returned by the handler.
  - [x] 3.5 Delete `apply_uid_filters`, `cached_full_fetch` (and the field), `SAFETY_LIMIT_TANTIVY`, the `fetch_limit_tantivy` field, the `fetch_limit_sql` field, the `needs_full_fetch` branch, `fetch_regular_unpaginated`, `fetch_regular_dispatch`, `should_fetch_bold`, `fetch_bold_unpaginated`, `hit_sql_safety_cap`. `merge_by_score_desc` is still needed by `fulltext_dict` until Stage 4 collapses the bold index.
  - [x] 3.6 Delete the `_all` wrapper helpers (`suttas_contains_match_fts5_all`, `dict_words_contains_match_fts5_all`, `book_spine_items_contains_match_fts5_all`, `dpd_lookup_all`, `suttas_title_match_all`, `library_title_match_all`, `lemma_1_dpd_headword_match_fts5_all`, `fulltext_suttas_all`, `fulltext_dict_words_all`, `fulltext_library_all`, `query_bold_definitions_fulltext_all`).
  - [x] 3.7 Kept `SAFETY_LIMIT_SQL` as a defensive ceiling, lowered to 50_000 and documented as defense-in-depth against pathological multi-phase intermediate fetches. Real per-page bounding is now `LIMIT page_len OFFSET …` directly.
  - [x] 3.8 `make build -B` succeeds end-to-end after the deletions. Two dead-code warnings on `query_bold_definitions_bold_fts5` / `query_bold_definitions_commentary_fts5` / `bold_definition_to_search_result` are silenced with `#[allow(dead_code)]` — they remain as scaffolding for any future SQL bold-append revival.

- [x] 4.0 Stage 4 — Remove dead two-index plumbing for bold definitions; unify under the dict index
  - [x] 4.1 In `backend/src/search/indexer.rs`, route bold-definition rows into the unified `dict_words_index_dir` writer (with `is_bold_definition = true`); renamed `build_bold_definitions_index` → `append_bold_definitions_to_dict_index` (no `delete_all_documents`, opens existing per-language dict subdir), removed `bold_definitions_index_dir` from `AppGlobalPaths`.
  - [x] 4.2 In `cli/src/bootstrap/`, the bold-definitions step now appends into the unified Pāli dict index after the per-language dict build.
  - [x] 4.3 Removed `IndexType::BoldDefinitions` from the searcher's private `IndexType` enum and every match arm referencing it (`open_single_index`, `search_single_index` filter dispatch, doc-to-result dispatch).
  - [x] 4.4 Removed `bold_definitions_index` field, `open_bold_definitions_index`, `has_bold_definitions_index`, `search_bold_definitions_with_count` from `FulltextSearcher`. The searcher no longer carries any "pli"-only special-casing.
  - [x] 4.5 Unified `fulltext_dict` is now a single tantivy call against `dict_indexes`. New `SearchFilters.include_bold_definitions` (default true, serde-defaulted to true) gates `Occur::MustNot { is_bold_definition = true }` inside `add_dict_filters`; `fulltext_dict` passes `self.include_comm_bold_definitions` through. No cover-fetch, no Rust merge — first page = `TopDocs::with_limit(page_len)`.
  - [x] 4.6 In `search_single_index`'s `IndexType::Dict` arm, peek at `is_bold_definition` per doc and dispatch to `bold_definition_doc_to_result` for bold rows, otherwise `dict_doc_to_result`.
  - [x] 4.7 Deleted `merge_by_score_desc` and the hard-coded "bold-defaults" `SearchFilters` block; `fulltext_dict` no longer needs them.
  - [x] 4.8 Verified after re-bootstrap: UI shows DPD + bold-definition rows interleaved in Fulltext + Dictionary; `cargo test` passes the relevant suites.

- [x] 4.9 Replace `paginate_with_bold` cover-fetch + Rust-slice with `split_page_across_streams` boundary-aware pagination. The `_with_bold` orchestrators (`dpd_lookup_with_bold`, `headword_match_with_bold`, `dict_contains_with_bold`) now compute regular/bold offset+limit pairs from `(regular_total, page_num, page_len)` and fetch only the bold slice via true `LIMIT/OFFSET` SQL — or just the COUNT when the page lies entirely inside the regular range. The bold helpers (`query_bold_definitions_bold_fts5`, `query_bold_definitions_commentary_fts5`) now take `(offset, limit)` and skip the row fetch on `limit == 0`. Per-page cost is bounded by `page_len`, independent of `page_num`. The multi-phase regular handlers expose `_full` variants returning the full filtered union; their existing `(page_num, page_len)` versions are thin slicers over `_full`.

- [ ] 5.0 Stage 5 — Snippet-generator reuse verification and timing-budget cleanup
  - [ ] 5.1 Confirm the `search_single_index` call path still constructs `SnippetGenerator` exactly once per call (introduced in commit b18b4c0) and that with Stage 3's page-only fetch, snippet work is bounded to `page_len` rows.
  - [ ] 5.2 Run `test_search_filter_pagination` and capture new per-page timings; ensure they sit comfortably under the previous `FILTERED_PAGINATION_BUDGET`.
  - [ ] 5.3 Drop the slack added for the cached full-fetch worst case from `FILTERED_PAGINATION_BUDGET`; tighten to e.g. `PER_PAGE_BUDGET = 2s` and `FILTERED_PAGINATION_BUDGET = 8s` for a 10-page paginate.

- [ ] 6.0 Stage 6 — Tests: push-down assertions, pagination invariants, bold-definitions highlighting, tightened timing budgets
  - [ ] 6.1 Extend `backend/tests/test_search_filter_pagination.rs` with assertions that for any filtered query the storage layer (tantivy `Count` / SQL `COUNT(*)`) returned the total — i.e. the new push-down path is exercised, not a full-fetch fallback (which no longer exists). A simple way: instrument the helpers to count the rows they fetch and assert `<= page_len + small_constant`.
  - [ ] 6.2 Create `backend/tests/test_pagination_invariants.rs`: for representative filtered + unfiltered queries across each mode, assert that every page except the last has exactly `page_len` results and that successive pages are contiguous (no overlap, no gap) per analysis §8.1.
  - [ ] 6.3 Create `backend/tests/test_bold_definitions_highlighting.rs`: run a diacritic query (e.g. `viññāṇa`) in Fulltext + Dictionary with bold defs included and assert that bold-row snippets contain highlight spans on the diacritic-normalised match (analysis §8.4).
  - [ ] 6.4 Tighten the timing budgets in `test_search_filter_pagination.rs` per Stage 5's measurements.
  - [ ] 6.5 Run `cd backend && cargo test` to confirm the full suite is green.

- [ ] 7.0 Documentation sync — update PRDs and PROJECT_MAP to reflect the new pipeline
  - [ ] 7.1 Update `tasks/prd-dpd-bold-definitions-search.md` §4.1 / §4.2 / §7 to describe the consolidated dict index, the `is_bold_definition` field, and the `Occur::MustNot` semantics for `include_comm_bold_definitions = false`.
  - [ ] 7.2 Add a header note to `tasks/analysis-dpd-bold-definitions-search.md` (or its companion task list) marking §7 as superseded by this refactor and linking back here.
  - [ ] 7.3 Update `PROJECT_MAP.md` entries for the search pipeline: drop `IndexType::BoldDefinitions`, `cached_full_fetch`, `SAFETY_LIMIT_TANTIVY`, `apply_uid_filters`; mention the `uid_rev` / `spine_item_uid_rev` fields and the unified dict index.
