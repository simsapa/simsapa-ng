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

- [ ] 3.0 Stage C — Move bold-definition appending out of mode handlers into a single seam
  - ⚠️ **Must land with Stage D in the same commit.** Stage C strips bold-append from the old paginated handlers, which are still called by `results_page` until Stage D swaps in the `_all` variants. C-without-D silently regresses Dictionary bold-definition results.
  - [ ] 3.1 Remove the inline bold-append blocks from `dpd_lookup` (lines ~1542–1568), `dict_words_contains_match_fts5` (~1245–1259), `lemma_1_dpd_headword_match_fts5` (~1936–1944), and `fulltext_dict_words` (~2227–2293).
  - [ ] 3.2 Add `should_fetch_bold(&self) -> bool` per §7.2 — true iff area == Dictionary, `include_comm_bold_definitions`, and mode ∈ {DpdLookup, HeadwordMatch, ContainsMatch, FulltextMatch}.
  - [ ] 3.3 Add `fetch_bold_unpaginated(&self) -> Result<Vec<SearchResult>>` that dispatches on `search_mode` to the appropriate helper (normalizing the query for Contains/Fulltext modes via `normalize_plain_text`).
  - [ ] 3.4 For Fulltext mode, add/adjust `query_bold_definitions_fulltext_all` so it fetches up to `SAFETY_LIMIT_TANTIVY` score-sorted hits (no per-page slicing) and preserves `SearchResult.score`.
  - [ ] 3.5 **Document the filter contract for bold fetching.** `fetch_bold_unpaginated` (and the tantivy bold helper specifically) deliberately pass `uid_prefix: None` / `uid_suffix: None` / unfiltered `SearchFilters` — uid gating is owned by the unified Rust filter in Stage F (§7 decision 2.6). Add an inline comment at the call site so a future reader doesn't "fix" the apparent inconsistency by re-pushing filters into the helpers. The FTS5 bold helpers from Stage A may retain their `bd.uid LIKE ?` push-down as an optimization; that's idempotent with the Stage F filter.
  - [ ] 3.6 Confirm dict-area mode handlers no longer mention `bold_definitions` after this stage.

- [ ] 4.0 Stage D — Replace `results_page` body with the §7.1 pipeline
  - ⚠️ **Must land with Stage C in the same commit** (see Stage C banner).
  - [ ] 4.1 Add `fetch_regular_unpaginated(&mut self) -> Result<Vec<SearchResult>>` dispatching on `(search_mode, search_area)` to the `_all` handlers from Stage B.
  - [ ] 4.2 Add `merge_by_score_desc(a, b)` per §7.3 (stable linear merge on `SearchResult.score`).
  - [ ] 4.3 Rewrite `results_page` body as: fetch regular → fetch bold (if `should_fetch_bold`) → mode-specific merge (score-desc for Fulltext, concat otherwise) → `apply_uid_filters` → set `db_query_hits_count = filtered.len()` → paginate once via `[start..end]` → highlight only the returned page.
  - [ ] 4.4 Delete `needs_post_filter`, the `self.page_len = 10_000` save/restore dance, and the old `run_mode_for_area` (now subsumed by `fetch_regular_unpaginated`).
  - [ ] 4.5 Delete the now-unused paginated mode handlers retained in Stage B.7.
  - [ ] 4.6 Verify `self.page_len` is read-only throughout the file (grep for assignments).
  - [ ] 4.7 **Audit `db_query_hits_count` readers.** Grep across `backend/`, `bridges/`, `assets/qml/`, and `src-ts/` for any consumer that reads this counter. Confirm none expect it to be set by a specific mode handler mid-flight — it is now written exactly once per call, at the end of `results_page`. Document findings in the commit message.

- [ ] 5.0 Stage E — Highlight non-DPD snippets with the normalized query (§2.4)
  - [ ] 5.1 Introduce `highlight_row(&self, r: SearchResult) -> SearchResult` per §7.4 that skips DPD rows (`dpd_headwords`, `dpd_roots`, or `dict_words` with a DPD `source_uid`) and otherwise calls `highlight_query_in_content(&normalize_plain_text(&self.query_text), &r.snippet)`.
  - [ ] 5.2 Route the Stage-D page through `highlight_row` (replacing the inline `highlight_query_in_content` call site at ~lines 2163–2173).
  - [ ] 5.3 Confirm `bold_definitions` rows now receive highlighted spans for diacritic queries (manual spot-check against the real DB).

- [ ] 6.0 Stage F — Unified uid prefix/suffix filter (§7.5)
  - [ ] 6.1 Simplify `apply_uid_filters` to the §7.5 form: normalize prefix/suffix once, early-return on both-empty, then filter by lowercased `r.uid`.
  - [ ] 6.2 Remove any `prefix_handled_by_sql` branching; the Rust filter is idempotent against SQL-prefiltered rows.
  - [ ] 6.3 Keep the SQL-side `uid LIKE ?%` push-down in suttas FTS5 `_all` variants purely as an optimization; verify it doesn't change semantics.
  - [ ] 6.4 `cd backend && cargo test test_uid_suffix_and_bold_ascii` to confirm existing coverage still passes.

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
