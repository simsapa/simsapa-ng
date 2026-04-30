# Tasks — Integrate StarDict Filtering Across All Dictionary Search Modes

Source PRD: [prd-integrate-stardict-filtering.md](./prd-integrate-stardict-filtering.md)

## Relevant Files

- `scripts/dictionaries-fts5-indexes.sql` — Adds `word` as a second trigram-indexed column in `dict_words_fts` and extends the seed/triggers. Requires manual re-bootstrap of the dictionaries DB after the change lands.
- `backend/src/types.rs` — `SearchMode::Combined` already exists in the enum (line 65); no schema change. `SearchParams.dict_source_uids` (line 108) is already wired. Reused by every dict mode handler.
- `backend/src/query_task.rs` — Per-mode dispatch + `apply_dict_source_uids_filter` (line 2084). Houses `dict_words_contains_match_fts5_full` (line 741), `lemma_1_dpd_headword_match_fts5_full` (line 1632), `dpd_lookup_full` (line 1380), `fulltext_dict` (line 1783), `results_page` (line 1998).
- `backend/src/search/searcher.rs` — `add_dict_filters` (line 574) already pushes `dict_source_uids` down to Tantivy for Fulltext. No behavioural change needed.
- `backend/src/app_settings.rs` — Add `dict_search_last_mode: Option<String>` and accessors. Mirrors existing `dict_search_dpd_enabled` pattern.
- `backend/src/app_data.rs` — Add `get_last_dict_search_mode()` / `set_last_dict_search_mode(mode)` helpers around the new setting.
- `bridges/src/sutta_bridge.rs` — Existing `results_page` (line 1302), `fetch_and_cache_page` (line 102), and `RESULTS_PAGE_CACHE` (top of file). Adds the `CombinedCache` struct, the `COMBINED_CACHE` static, `run_sub_query`, `fetch_combined_page`, and a dispatch branch in `results_page` for `(area=Dictionary, mode=Combined)`.
- `bridges/src/dictionary_manager.rs` — Expose two QObject methods: `get_last_dict_search_mode()` and `set_last_dict_search_mode(mode)`.
- `assets/qml/com/profoundlabs/simsapa/DictionaryManager.qml` — qmllint stubs for the two new methods.
- `assets/qml/SearchBarInput.qml` — `search_mode_dropdown` (line 226). Add `Combined` to the Dictionary lists and wire default + persistence via `DictionaryManager`.
- `assets/qml/SuttaSearchWindow.qml` — No new components. The existing `FulltextResults` mount point (line 2767) renders Combined results unchanged.
- `backend/tests/dict_modes_filtering.rs` — **NEW** integration tests for the Contains, Headword, and DPD Lookup invariants (uses the local appdata DB; no `#[ignore]`).
- `bridges/src/sutta_bridge.rs` (tests block) or `bridges/tests/combined_dict_results.rs` — **NEW** tests for Combined merge ordering, page-boundary correctness, DPD-exhausted top-up, and cache isolation.

### Notes

- All filesystem existence checks (none expected here) must use `try_exists()` per CLAUDE.md.
- After each top-level task the project must compile cleanly with `make build -B`.
- Per project memory: do not run `make qml-test`; only run tests after the full top-level task is done; do not bulk-`sed` to rename symbols.
- Combined is **bridge-orchestrated**, not UI-orchestrated. The QML interface is unchanged: one `results_page` call → one `results_page_ready` signal → renders into the existing `FulltextResults`. No new QML files. The per-task layer in `query_task.rs` rejects `SearchMode::Combined + SearchArea::Dictionary` so accidental backend invocations fail loudly.
- Combined uses one new top-level `static COMBINED_CACHE: Mutex<Option<CombinedCache>>`, isolated from `RESULTS_PAGE_CACHE` to prevent cross-warming. The struct holds both sub-query buffers (`dpd_buffer` / `ft_buffer`) plus their totals and pages-fetched counters; one mutex gives a coherent snapshot of both sides. The merged combined page is computed on demand by slicing the two buffers; no second memoisation layer. The lock is never held across an SQLite or Tantivy call — sub-queries run unlocked and write back briefly.

## Tasks

- [ ] 0.0 Schema: add `word` as a trigram-indexed column to `dict_words_fts`, update triggers, and document the manual re-bootstrap
  - [ ] 0.1 Edit `scripts/dictionaries-fts5-indexes.sql`:
        - Add `word` to the `CREATE VIRTUAL TABLE dict_words_fts USING fts5(...)` declaration alongside `definition_plain` (both indexed; trigram tokenizer unchanged).
        - Update the seed `INSERT INTO dict_words_fts (...) SELECT ...` to include `word`.
        - Update the `dict_words_fts_insert` and `dict_words_fts_update` triggers to write `NEW.word`. The `dict_words_fts_delete` trigger is unchanged (delete-by-`dict_word_id`).
  - [ ] 0.2 Note in the script header (and in `docs/`) that bumping this script requires a manual re-bootstrap of the dictionaries DB. No Diesel migration is added — the FTS table and triggers are recreated by the script.
  - [ ] 0.3 Manual step (user): re-bootstrap the dictionaries DB so the FTS table is rebuilt with the new column. Verify with `PRAGMA table_info(dict_words_fts)` that `word` is present and that a new INSERT into `dict_words` populates the FTS row.
  - [ ] 0.4 Run `make build -B`.

- [ ] 1.0 Backend: extend `ContainsMatch` + Dictionary to retrieve user-imported `dict_words` rows via the unified `dict_words_fts` path, restoring `total` accuracy
  - [ ] 1.1 Re-read `dict_words_contains_match_fts5_full` (`backend/src/query_task.rs:741`) and locate Phase 3 (the `dict_words_fts.definition_plain LIKE` block, ~line 889). Confirm Task 0 has shipped (the FTS table now has `word`). The new unified Phase 3 will JOIN `dict_words_fts` to `dict_words` so that filtering on `dict_label` rides the existing `dict_words_dict_label_idx` btree.
  - [ ] 1.2 Add a private helper `fn dict_label_in_clause(set: &[String]) -> Option<(String, Vec<String>)>` returning the placeholder string `"?, ?, …"` and the bind values, or `None` when the set is empty (caller skips the phase entirely). Place it near the top of `query_task.rs` impl alongside `normalized_filter`.
  - [ ] 1.3 Replace Phase 3 with a unified `dict_words_fts`-driven path:
        - SQL shape: `SELECT dw.* FROM dict_words dw JOIN dict_words_fts f ON f.dict_word_id = dw.id WHERE (f.word LIKE ? OR f.definition_plain LIKE ?) [AND dw.dict_label IN (?, ?, …)] [AND dw.uid LIKE ?] …` — preserving the existing `uid_prefix_pat` / `uid_suffix_pat` / `self.source` / `self.source_include` push-downs.
        - When `self.dict_source_uids` is `Some(set)` and `set.is_empty()`, skip the phase entirely.
        - When `self.dict_source_uids` is `Some(set)` non-empty, include the `dw.dict_label IN (...)` clause.
        - When `None`, drop the `dict_label` clause — search every dictionary.
  - [ ] 1.4 Add a new **Phase 5: user-headword substring** — `SELECT dw.* FROM dict_words dw JOIN dict_words_fts f ON f.dict_word_id = dw.id WHERE f.word LIKE ? AND dw.dict_label IN (?, ?, …)`. Cap at `SAFETY_LIMIT_SQL`. Skip when the inclusion set is empty or `None` collapses this into the unified Phase 3 (in which case Phase 5 contributes nothing additional and is skipped).
  - [ ] 1.5 Switch the cross-phase dedup key from `result.word` to `result.id` so multi-label collisions don't drop legitimate hits. Preserve order: DPD-driven Phases 1+2+4 first, then unified Phase 3, then Phase 5.
  - [ ] 1.6 `apply_dict_source_uids_filter` (line 2084) becomes a no-op for this mode in normal operation but stays in the dispatcher as a safety net. Emit `debug!("dict_source_uids post-filter dropped {} rows on Contains", dropped)` only when `dropped > 0` so any regression surfaces in logs.
  - [ ] 1.7 Confirm `dict_contains_with_bold` (the bold-definition wrapper composing `dict_words_contains_match_fts5_full`) still works unchanged. PRD §5.1 item 6: bold append is independent and must not be touched by this task.
  - [ ] 1.8 Update the function-level rustdoc on `dict_words_contains_match_fts5_full` to document the new phase numbering, the JOIN-based `dict_label IN (set)` push-down, and the dedup-by-id change. Keep the `total = full.len()` materialise-then-slice contract documented.
  - [ ] 1.9 Run `make build -B`. Compilation must succeed with no new warnings introduced by Phases 3/5.

- [ ] 2.0 Backend: extend `HeadwordMatch` + Dictionary to merge a user-headword path against `dict_words_fts.word`, alongside the existing DPD `lemma_1` path
  - [ ] 2.1 Re-read `lemma_1_dpd_headword_match_fts5_full` (`backend/src/query_task.rs:1632`). The DPD-only path resolves DPD `lemma_1` hits to a single `dict_words` row by `word == lemma_1`. Plan: keep this path conditional on `"dpd"` being in the inclusion set (or `dict_source_uids` being `None`), and add a parallel user-headword path for the rest of the set. **Keep the function name** — no rename, no shim — to avoid churn and preserve the descriptive DPD-path label inside the function body.
  - [ ] 2.2 Compute the "non-DPD" subset of the inclusion set: `set.iter().filter(|s| s != "dpd").collect::<Vec<_>>()`. When `dict_source_uids` is `None`, treat both paths as enabled.
  - [ ] 2.3 Add a "Path B: user-headword" SQL query: `SELECT dw.* FROM dict_words dw JOIN dict_words_fts f ON f.dict_word_id = dw.id WHERE f.word LIKE ? AND dw.dict_label IN (non_dpd_set) [AND dw.uid LIKE ?] …`. Skip when `non_dpd_set` is empty. Cap at `SAFETY_LIMIT_SQL`. Trigram FTS handles `LIKE '%term%'` efficiently; the JOIN to `dict_words` rides `dict_words_dict_label_idx`.
  - [ ] 2.4 Merge Path A (DPD) and Path B (user-headword) — deduplicate by `dict_words.id`. Sort: exact-`word == term` rows first, then contains rows; tie-break by `dict_label` then `id` so result order is stable.
  - [ ] 2.5 Confirm `headword_match_with_bold` (line 1927) still composes correctly — the bold branch is independent and must not double-count.
  - [ ] 2.6 As in §1.6, the `apply_dict_source_uids_filter` post-filter is now a safety net; emit `debug!(…)` only on non-zero drops for surveillance.
  - [ ] 2.7 Update the rustdoc to describe both paths, the merge, and pagination (`total = full.len()` materialise-then-slice unchanged).
  - [ ] 2.8 Run `make build -B`.

- [ ] 3.0 Backend: explicitly reject `SearchMode::Combined + SearchArea::Dictionary` in `results_page`; let `Combined + (Suttas|Library)` fall through to `FulltextMatch`
  - [ ] 3.1 In `query_task.rs::results_page` (line 1998), replace the `SearchMode::Combined => (Vec::new(), 0)` arm:
        - When `search_area == SearchArea::Dictionary`, return `Err("SearchMode::Combined is bridge-orchestrated; query_task must not be invoked with Combined + Dictionary".into())`.
        - When `search_area` is `Suttas` or `Library`, shadow the mode locally to `FulltextMatch` (`let mode = if matches!(self.search_mode, SearchMode::Combined) { SearchMode::FulltextMatch } else { self.search_mode };`) and dispatch as Fulltext. PRD §5.4.17 documents this fallback; no new Suttas-side combined mode in this PRD.
  - [ ] 3.2 Confirm `bridges/src/sutta_bridge.rs::results_page` already surfaces `Err` from `fetch_and_cache_page` through the `results_page_ready` error-payload path (line ~1429). No change required, but verify and note in the task log.
  - [ ] 3.3 Add a unit test in `query_task.rs` (or `backend/tests/dict_modes_filtering.rs`) asserting that `Combined + Dictionary` returns `Err`, and that `Combined + Suttas` returns the same shape as `FulltextMatch + Suttas` for a known query.
  - [ ] 3.4 Run `make build -B`.

- [ ] 4.0 Settings + bridge: persist last-used dictionary search mode (`dict_search.last_mode`, default `"Combined"`)
  - [ ] 4.1 Add `dict_search_last_mode: Option<String>` to the `AppSettings` struct in `backend/src/app_settings.rs` with `#[serde(default)]`. Default value is `None`; readers treat `None` as `"Combined"`.
  - [ ] 4.2 Add `get_last_dict_search_mode() -> String` (returns `"Combined"` when unset) and `set_last_dict_search_mode(mode: &str)` on `AppData` in `backend/src/app_data.rs`. Use the same `persist_app_settings` helper pattern used by `set_dpd_enabled`.
  - [ ] 4.3 Add `get_last_dict_search_mode(&self) -> QString` and `set_last_dict_search_mode(&self, mode: QString)` to the `DictionaryManager` bridge (`bridges/src/dictionary_manager.rs`). No mutex required — settings writes are not on the import/rename/delete critical path.
  - [ ] 4.4 Update the qmllint stub `assets/qml/com/profoundlabs/simsapa/DictionaryManager.qml` with placeholder implementations of the two new methods.
  - [ ] 4.5 Run `make build -B`.

- [ ] 5.0 QML dropdown: add `Combined` to the Dictionary search-mode dropdown, default-select it, and restore/persist via `DictionaryManager`
  - [ ] 5.1 In `assets/qml/SearchBarInput.qml::search_mode_dropdown` (line 226), update both `search_mode_label_wide.Dictionary` (line 241) and `search_mode_label_narrow.Dictionary` (line 263) lists:
        - Wide: `["Combined", "DPD Lookup", "Fulltext Match", "Contains Match", "Headword Match"]`
        - Narrow: `["Combined", "Lookup", "Fulltext", "Contains", "Headword"]`
  - [ ] 5.2 On `search_area === "Dictionary"`, set `currentIndex` to the index of `DictionaryManager.get_last_dict_search_mode()` in the wide list (default `"Combined"` → index 0). Do this in a `Connections { target: root; function onSearch_areaChanged() { ... } }` block or the existing search-area handler.
  - [ ] 5.3 On `currentIndex` change while `search_area === "Dictionary"`, call `DictionaryManager.set_last_dict_search_mode(get_text())`. Fold this into the existing `onCurrentIndexChanged` handler (line 279) without disturbing the `handle_query_fn` call.
  - [ ] 5.4 Verify other areas (Suttas, Library) keep their existing default (`currentIndex = 0`, "Fulltext Match") — adding the persisted setting must not affect them.
  - [ ] 5.5 Run `make build -B`. Manual sanity check via build only (no GUI): the QML registers and qmllint passes.

- [ ] 6.0 Bridge: implement Combined as a backend orchestrator inside `SuttaBridge::results_page` — parallel sub-fetches on page 0, side-aware top-ups on later pages, one isolated `CombinedCache`
  - [ ] 6.1 In `bridges/src/sutta_bridge.rs`, near the existing `ResultsPageCache`, define one struct and one top-level static:
        ```rust
        struct CombinedCache {
            cache_key: String,
            page_len: usize,
            // DPD sub-query buffer (cached results of the parallel DPD background fetch)
            dpd_buffer: Vec<SearchResult>,
            dpd_total: Option<i64>,
            dpd_pages_fetched: usize,
            // Fulltext sub-query buffer (cached results of the parallel Fulltext background fetch)
            ft_buffer: Vec<SearchResult>,
            ft_total: Option<i64>,
            ft_pages_fetched: usize,
        }
        static COMBINED_CACHE: Mutex<Option<CombinedCache>> = Mutex::new(None);
        ```
        Document inline that `CombinedCache` is deliberately isolated from `RESULTS_PAGE_CACHE` to prevent cross-warming between Combined sub-fetches and standalone-mode searches that share params; that the two sub-buffers cache the parallel DPD and Fulltext background queries that Combined fans out; and that the merged combined page is computed on demand from both buffers without a second memo layer.
  - [ ] 6.2 Add a private helper `fn run_sub_query(query, area, params, sub_page_num) -> Result<(Vec<SearchResult>, i64, usize), String>` that constructs a `SearchQueryTask` against `&app_data.dbm` with the requested mode (caller sets `params.mode`), calls `.results_page(sub_page_num)`, and returns `(results, total_hits, page_len)`. This is the unit run inside the parallel threads — keep it free of cache logic so it's easy to reason about.
  - [ ] 6.3 Add `fn fetch_combined_page(cache_key, query, params_json, page_num) -> Result<Option<(Vec<SearchResult>, i64, usize)>, String>` implementing the PRD §5.4.3 algorithm. The lock discipline is: **never hold the `COMBINED_CACHE` mutex across a sub-query call**; acquire it briefly to read state, drop it to fetch, re-acquire it to install results and re-check `cache_key`. Sketch:
        1. Parse `params_json` into a `SearchParams`. Build two cloned variants: `dpd_params` (with `mode = DpdLookup`) and `ft_params` (with `mode = FulltextMatch`). Both inherit `dict_source_uids` / `include_comm_bold_definitions` / language unchanged. **No deduplication** is performed across the two sides — PRD §5.4 item 16.
        2. Lock `COMBINED_CACHE`. If the stored `cache_key` differs (or the cell is `None`), replace with a fresh `CombinedCache { cache_key, page_len: L, … default … }`. Read out `(dpd_buffer.len(), dpd_total, ft_buffer.len(), ft_total)` and drop the lock.
        3. Compute the required slice `[lo, hi) = [page_num * L, (page_num + 1) * L)`.
        4. **Cold start** (`dpd_total.is_none() && ft_total.is_none()`): with the lock dropped, fan out — `thread::spawn` runs `run_sub_query` for DPD on sub-page 0, a second `thread::spawn` runs it for Fulltext on sub-page 0, `join()` both. Re-acquire the lock, re-check `cache_key` (abort with `Ok(None)` if a new search has invalidated the cell), install both buffers and totals atomically, drop the lock.
        5. **Top-up later pages**: with the lock dropped, while `dpd_buffer.len() < min(dpd_total, hi)` fetch DPD's next sub-page inline; while `ft_buffer.len() < min(ft_total, hi - dpd_total)` fetch Fulltext's next sub-page inline. In the rare case both sides need more, fan out two threads and join. Between every fetch, re-acquire the lock, re-check `cache_key`, append to the buffer, drop the lock.
        6. Acquire the lock one last time to read a coherent snapshot of `(dpd_buffer, dpd_total, ft_buffer, ft_total)`, compute the merged slice (DPD indices `[max(lo, 0), min(hi, dpd_total))` then FT indices `[max(lo - dpd_total, 0), max(hi - dpd_total, 0))`), drop the lock, and return `(merged, dpd_total + ft_total, L)`. The merged slice is *not* memoised — recomputing on revisit is cheap.
  - [ ] 6.4 In `SuttaBridge::results_page` (line 1302), parse `params_json` to a `SearchParams` **before** the existing `fetch_and_cache_page` dispatch (today the parse happens inside `fetch_and_cache_page`). When `(search_area_text == "Dictionary" && params.mode == SearchMode::Combined)`, dispatch to `fetch_combined_page` instead. Reuse the existing thread-spawn wrapper, signal-emission path, and "cache key changed during fetch → abort" semantics. For all other modes, behaviour is unchanged.
  - [ ] 6.5 Teach the prefetcher (`prefetch_pages`, line 172) to dispatch Combined the same way: parse `params_json` once, branch to `fetch_combined_page` for Combined+Dictionary, fall through to `fetch_and_cache_page` otherwise.
  - [ ] 6.6 Cache key for Combined is `format!("{}|{}|{}|combined", query_text, search_area_text, params_json_text)`. The literal `"|combined"` suffix ensures it cannot collide with any `RESULTS_PAGE_CACHE` key.
  - [ ] 6.7 Error handling: if either sub-query errors on page 0, return the error string from `fetch_combined_page` so `results_page` emits the error payload via `results_page_ready`. Do not partially populate `COMBINED_CACHE` on error — leave the cell `None` (or reset it) so the next user action starts cleanly. On a top-up failure on a later page, do not poison the cache — surface the error and let the next user action retry; partial buffers from previous successful pages remain usable.
  - [ ] 6.8 Run `make build -B`. Compilation + qmllint must succeed.

- [ ] 7.0 Tests + final `make build -B` + docs/PROJECT_MAP update
  - [ ] 7.1 Add `backend/tests/dict_modes_filtering.rs` with tests against the local appdata DB:
        - `contains_match_includes_user_dict_word_only_in_set`: search for a token present only in a user-imported dict's `word`; assert ≥1 result with the user dict in the inclusion set, 0 with it removed.
        - `contains_match_includes_user_dict_definition_only_in_set`: same for `definition_plain`.
        - `headword_match_includes_user_dict_word`: solo a user dict; expect only that dict's headword. With set `["dpd"]` only, expect zero user-dict rows.
        - `dpd_lookup_unaffected_by_user_dict_toggle`: toggling user dict checkboxes does not change DPD Lookup output.
        - `dpd_lookup_solo_user_dict_returns_zero`: with a non-DPD dict soloed, DPD Lookup returns zero results (PRD §5.3 invariant).
        - `combined_mode_dictionary_returns_err_at_query_task`: `SearchMode::Combined + SearchArea::Dictionary` returns `Err` from `query_task.rs::results_page`.
        - `combined_mode_suttas_falls_back_to_fulltext`: `Combined + Suttas` matches `FulltextMatch + Suttas` for a known query.
  - [ ] 7.2 Add Combined-specific tests in `bridges/src/sutta_bridge.rs`'s test module (or a new `bridges/tests/combined_dict_results.rs`):
        - `combined_page_zero_concatenates_dpd_then_fulltext`: page 0 lists DPD rows first, then Fulltext rows, page-trimmed to `page_len`. `total == dpd_total + ft_total`.
        - `combined_page_boundary_spans_dpd_and_fulltext`: pick `page_len` and a query so combined page 1 spans the DPD/Fulltext boundary; verify ordering.
        - `combined_dpd_exhausted_no_dpd_top_up`: after DPD is exhausted, a later page does not invoke `run_sub_query` for DPD (instrument with a counter or debug log).
        - `combined_cache_isolated_from_results_page_cache`: run Combined, then standalone DPD Lookup with same query/params; verify the standalone call hits the backend (not `COMBINED_CACHE`) by checking call counts.
        - `combined_subquery_error_emits_error_payload`: simulate a sub-query failure and assert `results_page_ready` carries an error payload.
  - [ ] 7.3 Add a `query_task.rs` unit test verifying `apply_dict_source_uids_filter` is a no-op when retrieval is already restricted by `dict_label IN (set)` (i.e. zero drops, no `total` decrement).
  - [ ] 7.4 Run `cd backend && cargo test`. Per project memory, ignore pre-existing failures; flag only newly introduced ones.
  - [ ] 7.5 Run `make build -B` one final time after all sub-tasks complete.
  - [ ] 7.6 Manual UI verification (PRD §8 items 11–13; not automated):
        - Default mode: open Dictionary tab on a fresh install (or with `dict_search_last_mode = None`) → search-mode dropdown shows `Combined`. Switch to `DPD Lookup`, restart → opens on `DPD Lookup`.
        - Lock interaction: in each of Combined / Fulltext / Contains / Headword, lock each row (DPD, Commentary, each user dict). Verify only that row contributes to results.
        - No-imports baseline: with zero user dictionaries imported, run several known queries in Contains and Headword and confirm result counts are unchanged from current behaviour.
  - [ ] 7.7 Update `PROJECT_MAP.md`: the `word` column added to `dict_words_fts`, the new `Combined` mode (bridge-orchestrated), the `CombinedCache` struct + `COMBINED_CACHE` static in `bridges/src/sutta_bridge.rs`, the `dict_label IN (set)` JOIN-based push-down on Contains/Headword, and the persisted `dict_search.last_mode` setting and bridge methods.
  - [ ] 7.8 Update `docs/` with a brief user-facing note: "Combined" is the new default dictionary mode and shows DPD lookups followed by Fulltext matches in a single ranked list, paginated together. The Dictionaries panel checkboxes / lock affect Combined, Fulltext, Contains, and Headword. DPD Lookup remains DPD-only by design. Note the manual re-bootstrap of the dictionaries DB after Task 0 lands.
