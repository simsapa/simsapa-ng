# Tasks: DPD Bold Definitions Search — Pipeline Refactor

Source: `tasks/analysis-dpd-bold-definitions-search.md` (§7 target design, §7.7 stages A–H).

## Relevant Files

- `backend/src/query_task.rs` — Core file being refactored: `results_page`, `run_mode_for_area`, `needs_post_filter`, `apply_uid_filters`, all mode handlers, and the bold-definition FTS5/tantivy helpers.
- `backend/src/db/dpd_models.rs` — `BoldDefinition` struct; needs `#[derive(QueryableByName)]` + per-field `sql_type` annotations for the single-JOIN rewrite.
- `backend/src/db/dpd.rs` — `populate_bold_definitions_derived_columns`, `import_migrate_dpd`, `create_dpd_indexes`; touched by the PRD sync (Stage G).
- `backend/src/search/mod.rs` (and submodules) — Tantivy searchers used by `fulltext_*` paths; likely need a `with_limit(SAFETY_LIMIT)` variant.
- `backend/src/helpers.rs` — `normalize_plain_text`, `highlight_query_in_content`; used by new highlight path.
- `backend/src/types.rs` — `SearchResult`, `SearchMode`, `SearchArea`, `SearchFilters` definitions referenced throughout the pipeline.
- `scripts/dpd-bold-definitions-fts5-indexes.sql` — FTS5 index script referenced by PRD drift (§2.10).
- `backend/tests/test_query_task.rs` — Existing query-task tests; extend with pipeline invariants.
- `backend/tests/test_uid_suffix_and_bold_ascii.rs` — Existing uid suffix / `bold_ascii` coverage to extend with combined prefix+suffix cases.
- `backend/tests/test_fulltext_search_results.rs` — Existing fulltext test; extend with no-drop merge assertion.
- `backend/tests/test_pagination_invariants.rs` *(new)* — Page-size invariant test across `(mode, area)` combinations.
- `backend/tests/test_bold_definitions_highlighting.rs` *(new)* — Diacritic-query highlight assertion for bold-definition snippets.
- `tasks/prd-dpd-bold-definitions-search.md` — PRD to amend in Stage G (§4.1, §4.2, §7).
- `tasks/tasks-prd-dpd-bold-definitions-search.md` — Existing task list (1.1, 2.2) to update in Stage G.

### Notes

- Tests live in `backend/tests/`. Run with `cd backend && cargo test [name]`.
- Build with `make build -B`.
- Integration tests use the real DBs under `/home/gambhiro/prods/apps/simsapa-ng-project/bootstrap-assets-resources/dist/simsapa-ng/`.
- Default `SAFETY_LIMIT` per §7.6 is `100_000`. Define two constants in `query_task.rs`: `SAFETY_LIMIT_SQL: i64 = 100_000` (for Diesel binds) and `SAFETY_LIMIT_TANTIVY: usize = 100_000` (for `TopDocs::with_limit`). Keep them in sync.
- Do not run `make qml-test` and do not gate local integration tests behind `#[ignore]`.
- **Stage ordering warning:** Stages C and D must land together (same commit / same PR). Stage C strips bold-append from the old paginated handlers, but those handlers are still live until Stage D rewrites `results_page` to use the `_all` variants. Committing C without D silently regresses Dictionary bold-definition results.

## Tasks

- [x] 1.0 Stage A — Collapse bold-definition FTS5 helpers to a single JOIN (§2.7, §2.8)
  - [x] 1.1 In `backend/src/db/dpd_models.rs`, add `#[derive(QueryableByName)]` to `BoldDefinition` and annotate every field with the matching `#[diesel(sql_type = …)]` (mirroring the columns selected by `SELECT bd.*`).
  - [x] 1.2 Define module-level `SAFETY_LIMIT_SQL: i64 = 100_000` and `SAFETY_LIMIT_TANTIVY: usize = 100_000` near the top of `backend/src/query_task.rs`. Document that the two must be kept in sync.
  - [x] 1.3 Rewrite `query_bold_definitions_bold_fts5` as a single `SELECT bd.* FROM bold_definitions_bold_fts f JOIN bold_definitions bd ON bd.id = f.bold_definitions_id WHERE f MATCH ? [AND bd.uid LIKE ?] ORDER BY bd.id LIMIT ?` using `diesel::sql_query(...).load::<BoldDefinition>(...)` with `SAFETY_LIMIT_SQL` as the bind.
  - [x] 1.4 Rewrite `query_bold_definitions_commentary_fts5` the same way against `bold_definitions_commentary_fts`.
  - [x] 1.5 Delete the `BdId` / id-only `QueryableByName` intermediate structs and the second Diesel load-by-`id.eq_any(&ids)` round-trip.
  - [x] 1.6 Remove the inner `ORDER BY f.bold_definitions_id` from the FTS5 subquery; keep only the outer `ORDER BY bd.id`.
  - [x] 1.7 `cd backend && cargo test` — confirm no regressions against the real `dpd.sqlite3`. (test_dpd_deconstructor_list pre-existing failure, unrelated; bold/uid tests all pass.)

- [x] 2.0 Stage B — Add unpaginated `_all` variants of mode handlers with a `SAFETY_LIMIT` cap
  - [x] 2.1 For each FTS5 handler, add an `_all` sibling. **Implementation note:** to limit churn, the `_all` siblings are thin wrappers around the existing paginated handlers via `run_with_safety_cap_sql`/`_tantivy` — they save/restore `self.page_len` and `self.db_query_hits_count`, call the underlying handler with `page_len = SAFETY_LIMIT` and `page_num = 0`, then warn if the cap was hit. Net effect (no pagination, no `db_query_hits_count` mutation, SAFETY_LIMIT cap) matches the spec; Stage D inlines and removes the paginated handlers entirely.
  - [x] 2.2 For `dpd_lookup`, added `dpd_lookup_all` (wraps `dpd_lookup(0)` with `page_len = SAFETY_LIMIT_SQL`, returning the full merged set on page 0).
  - [x] 2.3 For tantivy handlers (`fulltext_suttas`, `fulltext_dict_words`, `fulltext_library`), added `_all` variants that delegate via `run_with_safety_cap_tantivy` (page_len = SAFETY_LIMIT_TANTIVY). `SearchResult.score: Option<f32>` confirmed at `backend/src/types.rs:144`.
  - [x] 2.4 Push-down deferral honoured: Suttas FTS5 path retains its existing `uid LIKE ?%` push-down; Dictionary/Library paths still rely on the Rust filter (intentional deviation from analysis §2.5).
  - [x] 2.5 `run_with_safety_cap_sql` / `_tantivy` emit `warn!` with mode, area, and query when results length ≥ cap. Bold-definition helpers (Stage A) carry their own per-helper warn.
  - [x] 2.6 `_all` wrappers explicitly restore `self.db_query_hits_count` to its prior value, so they never own the counter.
  - [x] 2.7 Old paginated handlers untouched — `run_mode_for_area` still compiles and behaves as before.
  - [x] 2.8 `make build -B` succeeds (only dead-code warnings on the new `_all` methods, which is expected until Stages C/D wire them up).

- [x] 3.0 Stage C — Move bold-definition appending out of mode handlers into a single seam
  - [x] 3.1 Removed the inline bold-append blocks from `dpd_lookup`, `dict_words_contains_match_fts5`, `lemma_1_dpd_headword_match_fts5`, and `fulltext_dict_words`. Each handler now returns its mode-native results only.
  - [x] 3.2 Added `should_fetch_bold(&self) -> bool`.
  - [x] 3.3 Added `fetch_bold_unpaginated(&self) -> Result<Vec<SearchResult>>`; normalises the query for Contains/Fulltext via `normalize_plain_text`.
  - [x] 3.4 Stage B already added `query_bold_definitions_fulltext_all` (uses `SAFETY_LIMIT_TANTIVY`, preserves `SearchResult.score`).
  - [x] 3.5 Filter contract documented inline on `query_bold_definitions_fulltext_all`: deliberately passes empty `SearchFilters`; uid gating owned by `apply_uid_filters`.
  - [x] 3.6 Confirmed: dict-area handlers no longer reference `bold_definitions` (only the dedicated bold helpers do).

- [x] 4.0 Stage D — Replace `results_page` body with the §7.1 pipeline
  - [x] 4.1 Added `fetch_regular_unpaginated` dispatching on `(search_mode, search_area)` to the `_all` handlers.
  - [x] 4.2 Added `merge_by_score_desc(a, b)` (stable linear merge on `SearchResult.score`).
  - [x] 4.3 `results_page` rewritten: fetch regular → fetch bold (if gated) → merge (score-desc for Fulltext, concat otherwise) → `apply_uid_filters` → set `db_query_hits_count` once → paginate → highlight only the returned page.
  - [x] 4.4 Deleted `needs_post_filter`, the `self.page_len = 10_000` save/restore dance, and the old `run_mode_for_area`. (Old `query_bold_definitions_fulltext` paginated variant also deleted.)
  - [x] 4.5 Inlined: each former paginated handler is now its `_all` form (page_num and COUNT removed, `LIMIT page_len OFFSET …` replaced with `LIMIT SAFETY_LIMIT_SQL`, `db_query_hits_count` writes dropped, warn on cap). Tantivy `fulltext_*_all` handlers call the searcher with `(SAFETY_LIMIT_TANTIVY, 0)` directly. The `run_with_safety_cap_*` wrappers and the unused `CountResult` struct were deleted.
  - [x] 4.6 `self.page_len` is read-only outside the two `run_with_safety_cap_*` wrapper helpers (verified by grep — only those helpers assign it, always paired with a save/restore).
  - [x] 4.7 Audit complete. `total_hits()` consumers: `bridges/src/api.rs:897, :987` and `bridges/src/sutta_bridge.rs:140`. All three read `total_hits()` strictly after `results_page()` returns; the new pipeline writes `db_query_hits_count = filtered.len()` exactly once at the end of `results_page`. No mid-flight reads.

- [x] 5.0 Stage E — Highlight non-DPD snippets with the normalized query (§2.4)
  - [x] 5.1 Introduced `highlight_row(&self, r: SearchResult) -> SearchResult` per §7.4 — skips DPD rows and otherwise calls `highlight_query_in_content(&normalize_plain_text(&self.query_text), &r.snippet)`.
  - [x] 5.2 Replaced the inline closure in `results_page` with `.map(|r| self.highlight_row(r))`.
  - [ ] 5.3 Confirm `bold_definitions` rows now receive highlighted spans for diacritic queries (manual spot-check against the real DB — pending user verification).

- [x] 6.0 Stage F — Unified uid prefix/suffix filter (§7.5)
  - [x] 6.1 `apply_uid_filters` simplified to §7.5 form: normalize prefix/suffix once, early-return when both empty, then filter by lowercased `r.uid` using `is_none_or` for both checks.
  - [x] 6.2 Removed `prefix_handled_by_sql` branching. Filter applies uniformly across all areas.
  - [x] 6.3 SQL-side `uid LIKE 'prefix%'` push-down kept in Suttas FTS5 paths (and now Dictionary/Library + bold helpers from the perf fix). Filter is idempotent against rows that already satisfy it.
  - [x] 6.4 `cargo test --test test_uid_suffix_and_bold_ascii` — all 5 tests pass.

- [ ] 7.0 Stage G — Sync PRD and existing task list with as-built (§2.9–§2.11)
  - [ ] 7.1 In `tasks/prd-dpd-bold-definitions-search.md` §4.1, add `bold_ascii TEXT NOT NULL` to the `bold_definitions` column list (mirrors `word_ascii`) and note it is populated from `bold` via pali-to-ASCII.
  - [ ] 7.2 In PRD §4.2, add `bold_ascii` to the `bold_definitions_bold_fts` schema, the `INSERT…SELECT` statement, and all AFTER INSERT/UPDATE/DELETE triggers.
  - [ ] 7.3 In PRD §7, clarify that `dpd-bold-definitions-fts5-indexes.sql` and `create_dpd_indexes` run only in the bootstrap path; remove the "at-startup also / ~line 786" wording.
  - [ ] 7.4 In PRD §7, acknowledge `populate_bold_definitions_derived_columns` lives in `backend/src/db/dpd.rs` alongside `import_migrate_dpd` (not in `cli/`).
  - [ ] 7.5 Update `tasks/tasks-prd-dpd-bold-definitions-search.md` tasks 1.1 and 2.2 to match the PRD amendments above.

- [ ] 8.0 Stage H — Tests
  - [ ] 8.1 Create `backend/tests/test_pagination_invariants.rs`: for each `(mode, area)` with a query that yields > `page_len` results, assert every page except the last has exactly `page_len` rows and page indices are contiguous (no gaps, no duplicates by `uid`+`table_name`).
  - [ ] 8.2 Add a **pure unit test** for `merge_by_score_desc`: hand-constructed input vectors with known `score` values, assert (a) output length equals sum of inputs (no drops), (b) output is sorted non-increasing by score, (c) items with equal scores preserve original relative order. This is deterministic and independent of DB state, unlike an integration-level "bold outranks dict" assertion which is fragile because inter-index BM25 scores are not comparable (analysis §2.2, §4.3.12). Place in `backend/src/query_task.rs` as a `#[cfg(test)] mod tests` block or a new unit test file.
  - [ ] 8.3 Extend `backend/tests/test_fulltext_search_results.rs` with a weaker integration-level "no-drop" assertion: run a Fulltext Dictionary query that exercises both the dict tantivy index and the bold tantivy index; assert `db_query_hits_count == regular_count + bold_count` and that paging through every page returns exactly `db_query_hits_count` distinct rows. This tests the pipeline contract without relying on score comparisons.
  - [ ] 8.4 Create `backend/tests/test_bold_definitions_highlighting.rs`: run a diacritic query (e.g. `bhikkhū`) in ContainsMatch+Dictionary and assert the returned `bold_definitions` snippet contains at least one highlight span.
  - [ ] 8.5 Extend `backend/tests/test_uid_suffix_and_bold_ascii.rs` with a combined `uid_prefix` + `uid_suffix` case across Suttas, Dictionary, and Library.
  - [ ] 8.6 Add a **safety-cap smoke test**: issue a ContainsMatch query broad enough to approach `SAFETY_LIMIT_SQL` (e.g. a single common letter); assert it completes within a generous wall-clock bound (e.g. 10s) and, if it hits the cap, the `tracing::warn!` from 2.5 fires (capture via `tracing-test` or similar). This guards analysis §7.8 risk 1 (broad queries).
  - [ ] 8.7 `cd backend && cargo test` — full suite must pass.
  - [ ] 8.8 `make build -B` — confirm clean build.
