# PRD: Integrate User-Imported StarDict Filtering Across All Dictionary Search Modes

Source feature this builds on: [prd-stardict-dictionary-import.md](./prd-stardict-dictionary-import.md) and its task list [tasks-prd-stardict-dictionary-import.md](./tasks-prd-stardict-dictionary-import.md).

## 1. Background

Task 8.0 of the StarDict-import feature wired the per-dictionary checkbox / lock selection (built-in **DPD**, built-in **Commentary Definitions**, and each **user-imported** dictionary) into the dictionary search query layer. As shipped, the wiring works correctly only for `SearchMode::FulltextMatch` on `SearchArea::Dictionary` — i.e. `fulltext_dict` in `backend/src/query_task.rs`. The other dictionary search modes either ignore user-imported rows entirely or only partially honour the inclusion set.

This PRD plans the work needed to extend the same selection / solo logic to every dictionary search mode, with one deliberate exception: **DPD Lookup must remain DPD-only**. It also introduces a new default mode, **Combined**, which is **backend-orchestrated**: the `SuttaBridge::results_page` entry point detects `Combined + Dictionary`, fires `DpdLookup` and `FulltextMatch` as two parallel `SearchQueryTask` runs on page 0, merges them into a single virtual stream (DPD first, then Fulltext), and emits one `results_page_ready` signal with a slice of that stream. From QML's perspective Combined is a normal single-list result like every other mode — the dropdown gains one entry and the result view is unchanged.

## 2. Findings — How filtering works today

### 2.1 The selection contract (already in place)

- `SearchParams.dict_source_uids: Option<Vec<String>>` (`backend/src/types.rs:108`) carries the inclusion set; `None` = no constraint, `Some([])` = no `dict_words` rows match, `Some([labels…])` = those `dict_label`s only.
- `SearchParams.include_comm_bold_definitions: bool` independently gates the bold-definition rows.
- `SuttaSearchWindow.qml::compute_dict_search_filter()` (`assets/qml/SuttaSearchWindow.qml:583`) builds these two values from the Dictionaries panel state (DPD checkbox, Commentary Definitions checkbox, per-user-dict checkboxes, lock target).
- The selection-set is therefore already correctly assembled in QML before any backend call.

### 2.2 Where it is honoured today

- **Tantivy / Fulltext path** — `backend/src/search/searcher.rs::add_dict_filters` (line 574) consumes both `filters.dict_source_uids` and `filters.include_bold_definitions` and pushes the constraint down to the dict index. This is what makes Fulltext work: the inclusion set decides which `source_uid` terms are MUST'd into the query, and bold rows are gated independently.
- **Generic SQL post-filter** — `query_task.rs::apply_dict_source_uids_filter` (line 2084) is invoked by `results_page` (line 2065) for every non-Fulltext dictionary mode. It drops any `dict_words` row whose `source_uid` (== `dict_label`) is not in the inclusion set. Bold-definition rows are passed through untouched.

### 2.3 Why other modes don't behave as the user expects

The post-filter works on whatever rows the per-mode handler **returned**. The handlers themselves push only `self.source` (the legacy single-string source filter) into SQL — they do not retrieve user-imported rows that happen to lie outside the legacy filter, and several of them are structurally DPD-only:

1. **`ContainsMatch` + Dictionary** (`dict_words_contains_match_fts5_full`, line 741):
   - Phases 1, 2, 4 look up matches in `dpd_headwords` first, then resolve to `dict_words` only via `dict_words.word == headword.lemma_1`. User-imported dicts whose words do not exist in DPD are invisible to these phases.
   - Phase 3 does match user dicts (it scans `dict_words_fts.definition_plain`), so user dicts contribute a partial subset of results.
   - Net effect: the inclusion set is honoured *as a filter* but user-dict matches are systematically under-recalled.

2. **`HeadwordMatch` + Dictionary** (`lemma_1_dpd_headword_match_fts5_full`, line 1632):
   - Drives the search entirely from `dpd_headwords_fts.lemma_1`, then resolves each hit to a single `dict_words` row by exact `word == lemma_1`. User-imported headwords that are not DPD lemmas never appear.
   - Net effect: a user-imported dictionary checkbox toggles nothing — the mode is structurally DPD-only.

3. **`DpdLookup` + Dictionary** (`dpd_lookup_full`, line 1380):
   - Pure DPD. Per the user's directive, this is intentional and **must remain so**.

4. **`UidMatch` + Dictionary**: searches across SQL by uid prefix; the post-filter handles the inclusion set correctly, no change required.

5. **Bold-definition rows** are folded in via the per-mode `*_with_bold` wrappers (`dict_contains_with_bold` / `headword_match_with_bold` / `dpd_lookup_with_bold`). They are gated by `include_comm_bold_definitions`, which `compute_dict_search_filter` already drives correctly.

### 2.4 The `Combined` mode

`SearchMode::Combined` exists in `backend/src/types.rs:65` but is not yet implemented (`results_page` returns `(Vec::new(), 0)` for it, line 2049) and is not exposed in the `assets/qml/SearchBarInput.qml` dropdown.

### 2.5 Pagination today (per mode)

How `total` and `page_len` are produced today, since changes to retrieval interact directly with pagination:

- **`fulltext_dict`** (Tantivy): `searcher.search_dict_with_count(query, filters, page_len, page_num)` pushes pagination *down* — Tantivy returns the requested page along with a true `total_hits`. No materialisation. The `dict_source_uids` MUST'd into the query already restricts the corpus, so `total` is post-filter accurate.
- **`fulltext_suttas` / `fulltext_library`**: same shape, pagination down to Tantivy.
- **`dict_words_contains_match_fts5`** (Contains + Dictionary): `dict_words_contains_match_fts5_full()` materialises the union of phases 1–4 into `Vec<DictWord>` (capped at `SAFETY_LIMIT_SQL`), then the per-page wrapper slices `[offset .. offset+page_len]`. `total = full.len()`.
- **`lemma_1_dpd_headword_match_fts5`** (Headword + Dictionary): same shape — materialise `_full`, slice. `total = full.len()`.
- **`dpd_lookup`** (DPD Lookup + Dictionary): same shape — `dpd_lookup_full` materialises the multi-phase fallback union, slice. `total = full.len()`.
- **`*_with_bold` wrappers**: these compose the regular `_full` with bold rows, pagewise. They preserve the materialise-then-slice pattern.
- **`uid_match`**: materialised list, slice.
- **Generic post-filter `apply_dict_source_uids_filter`** (`query_task.rs:2084`): runs *after* the per-mode handler returned its page-sliced output. It decrements `total` only by the number of dropped rows that happened to land on the current page — it does **not** know how many dropped rows exist across all pages. Concretely:
  - The page-sliced result is correct (correct rows shown).
  - `total` returned to the UI is **inaccurate** when filtering drops rows: the page count / pager controls based on `total / page_len` will overestimate the number of pages, and later pages may be empty after filtering. This is a pre-existing issue for non-Tantivy dict modes whenever the user toggles off any dictionary with results.

### 2.6 Pagination implications of the §3 changes

- Adding a `dict_words_fts` retrieval phase to `ContainsMatch` and `HeadwordMatch` increases `_full` length but keeps the materialise-then-slice contract. `total` becomes the new union length, which is consistent with the page contents because the union is built **with** `dict_label IN (set)` pushed into the SQL — so the post-filter has nothing to drop and the `total` mismatch from §2.5 disappears for these modes.
- For `DpdLookup` (unchanged), the post-filter becomes a no-op when `"dpd"` is in the set; when the user solos a non-DPD dictionary the page is empty but `total` is still DPD's full length — this is the same pre-existing inconsistency. We accept it: soloing a non-DPD row under DPD-only mode is an unusual configuration, and the user-facing fix is to switch modes rather than expect DPD Lookup to honour user dicts.
- `Combined` (see §5.4) reports `dpd_total + fulltext_total` to the UI. This is exact whenever both sub-queries are exact (they are: DPD's materialise-and-slice and Fulltext's Tantivy push-down both yield true totals).

### 2.7 Findings — Index coverage for the new retrieval paths

The §5.1 / §5.2 changes hinge on what is actually indexed today. `scripts/dictionaries-fts5-indexes.sql` declares:

```sql
CREATE VIRTUAL TABLE dict_words_fts USING fts5(
    dict_word_id UNINDEXED,
    language     UNINDEXED,
    dict_label   UNINDEXED,
    definition_plain,
    tokenize='trigram',
    detail='none'
);
```

Two consequences:

1. **`dict_words_fts.word` does not exist.** The FTS table only trigram-indexes `definition_plain`. A user-headword substring search (`word LIKE '%term%'` against an arbitrary user dictionary) would fall back to a full-table `dict_words` scan with leading-wildcard `LIKE` — the existing btree `(dict_label, word)` index cannot serve that predicate. With multiple imported dictionaries this is a hot path we should not introduce.
2. **`dict_label` is `UNINDEXED` inside `dict_words_fts`.** It is stored alongside the FTS row but not indexed for filtering, so `WHERE dict_label IN (…)` against `dict_words_fts` is a scan. The right shape is to JOIN to `dict_words` (where `dict_words_dict_label_idx` and the composite `(dict_label, word)` index exist) and filter on the JOIN.

The fix for (1) is a one-line schema addition: add `word` as a second indexed column to `dict_words_fts`. The trigram index then serves both Phase 3 (`definition_plain LIKE`) and the new Phase 5 (`word LIKE`). The fix for (2) is purely a query-shape choice — JOIN to `dict_words`. This unblocks §5.1 and §5.2 efficiently and removes the §7 "fallback to a `dict_words` table scan" caveat from the original draft.

This change requires the dictionaries DB to be re-bootstrapped so the FTS table is rebuilt with the new column. The user has agreed to do this manually after the schema change lands. See §5.0 and Task 0.

## 3. Goals

1. Make the per-dictionary checkbox + lock state genuinely affect results in `ContainsMatch` and `HeadwordMatch` for `SearchArea::Dictionary` — not only as a post-filter on a DPD-driven candidate set, but by retrieving matches from each enabled dictionary in the first place.
2. Keep `DpdLookup` strictly DPD-only — it is the user's "DPD canonical view" mode and must not be diluted with user-imported entries.
3. Add `Combined` as the new default dictionary search mode. Combined is **backend-orchestrated**: `SuttaBridge::results_page` runs `DpdLookup` and `FulltextMatch` in parallel on the first page, merges them as `[DPD … , Fulltext …]`, and serves combined pages by slicing that virtual stream. The QML side is unchanged from any other mode.
4. Preserve current Fulltext behaviour exactly (it already works).
5. Keep bold-definition gating independent and consistent across every mode.

## 4. Non-Goals

- Changing how the Dictionaries panel is built or how `compute_dict_search_filter` decides what to send. Backend / bridge are the only changing surfaces for filtering.
- Adding new search modes beyond `Combined`.
- Cross-mode UID/result ranking changes beyond what `Combined`'s flat concatenation requires (DPD first, then Fulltext, no interleaving, no dedup).
- Indexer / reconciliation changes beyond adding `word` to the `dict_words_fts` virtual table — user-imported dictionaries are already indexed into both Tantivy and `dict_words_fts`, and the existing INSERT/UPDATE/DELETE triggers extend trivially to populate the new column.

## 5. Functional Requirements

### 5.0 Schema: extend `dict_words_fts` to trigram-index `word`

1. Update `scripts/dictionaries-fts5-indexes.sql`:
   - Add `word` as an indexed column alongside `definition_plain`.
   - Update the seed `INSERT … SELECT` to populate `word` from `dict_words.word`.
   - Update the `dict_words_fts_insert` and `dict_words_fts_update` triggers to write `NEW.word`. The DELETE trigger is unchanged (delete-by-`dict_word_id`).
2. The change requires a manual re-bootstrap of the dictionaries DB. There is no migration file because the FTS table and triggers are rebuilt by re-running the script. Document the re-bootstrap step in the task list and in `docs/`.
3. With this in place, both Phase 3 (`definition_plain LIKE`) and the new Phase 5 (`word LIKE`) ride the same trigram index.

### 5.1 `ContainsMatch` + Dictionary

1. Retrieval must include matches from every dictionary in `dict_source_uids` (or every dictionary, when `None`), not only those whose `word` matches a DPD `lemma_1`.
2. Replace Phase 3 with a unified `dict_words_fts`-driven retrieval that uses both indexed columns:
   - SQL shape: `SELECT dw.* FROM dict_words dw JOIN dict_words_fts f ON f.dict_word_id = dw.id WHERE (f.word LIKE ? OR f.definition_plain LIKE ?) AND dw.dict_label IN (?, ?, …)` — with the existing `d.uid LIKE ?` push-downs preserved.
   - When `dict_source_uids` is `Some(set)` and `set.is_empty()`, skip the phase entirely (zero contribution).
   - When `dict_source_uids` is `None`, drop the `dw.dict_label IN (…)` clause — search every dictionary.
   - Filtering on `dw.dict_label` rides `dict_words_dict_label_idx` (and the composite index when paired with `word`); filtering on `f.word` / `f.definition_plain` rides the trigram index.
3. The existing DPD-driven phases (1, 2, 4) remain — they still surface DPD lemma hits — but their result rows must continue to flow through `apply_dict_source_uids_filter`. When the user solos a non-DPD dictionary, DPD-driven phases will yield zero rows after filtering, which is the correct behaviour.
4. Result ordering within the page: keep the current "exact lemma → contains lemma → definition" precedence for DPD; append user-dict word/definition hits after them, ordered by `dict_words.id`.
5. Deduplicate across phases by `dict_words.id` (the existing dedup keys on `result.word`, which collides across user dicts that share a token; switch to id).
6. The bold-definition append (`dict_contains_with_bold`) is unchanged.

### 5.2 `HeadwordMatch` + Dictionary

7. Headword matching must consider the **headword field of every selected dictionary**, not only DPD's `lemma_1`. For user-imported StarDict dictionaries, the headword is `dict_words.word`.
8. Add a "Path B: user-headword" SQL path that, for the non-DPD subset of the inclusion set, selects `dict_words` rows via the FTS-trigram-backed `f.word LIKE ?` predicate joined to `dict_words` for `dict_label IN (non_dpd_set)`. Cap at `SAFETY_LIMIT_SQL`. Skip when the non-DPD set is empty.
9. Path A (DPD `lemma_1`) is preserved verbatim and is conditional on `"dpd"` being in the inclusion set (or `dict_source_uids` being `None`).
10. Merge Path A and Path B:
    - Deduplicate by `dict_words.id`.
    - Sort: exact-`word == term` rows first, then contains rows; tie-break by `dict_label` then `id` so result order is stable.
11. The bold-definition append (`headword_match_with_bold`) is unchanged.

### 5.3 `DpdLookup` + Dictionary

12. **No change**. `dpd_lookup_full` continues to return DPD-only matches. The post-filter `apply_dict_source_uids_filter` already drops nothing because every DPD result has `source_uid = "dpd"` and DPD is ordinarily in the inclusion set; in the rare case where the user solos a non-DPD dictionary, the post-filter correctly returns zero rows.
13. The Dictionaries panel still drives the `include_comm_bold_definitions` flag, which `dpd_lookup_with_bold` honours.

### 5.4 `Combined` mode (backend-orchestrated, merged virtual stream)

`Combined` is implemented entirely at the **bridge** layer — `bridges/src/sutta_bridge.rs`. It is **not** a per-mode handler in `query_task.rs`; the per-task-layer rejection of `SearchMode::Combined + SearchArea::Dictionary` is preserved so that any accidental backend invocation fails loudly. From QML's perspective Combined behaves like any other mode: one `results_page` call, one `results_page_ready` signal, one paged result list.

#### 5.4.1 Virtual stream

14. Define the merged stream as the concatenation `[DPD_0, DPD_1, …, DPD_{dpd_total-1}, FT_0, FT_1, …, FT_{ft_total-1}]`. Combined page `P` (with combined page length `L`) is the slice `[P*L, P*L + L)` of that stream. Per-source offset bookkeeping is therefore derivable from `P`, `L`, `dpd_total`, and `ft_total` rather than tracked per page.
15. Combined `total` reported to the UI is `dpd_total + ft_total`. This is exact: DPD's materialise-and-slice produces a true `dpd_total`, and Fulltext's Tantivy push-down produces a true `ft_total`. Pager arithmetic in QML is unchanged.
16. No deduplication across DPD and Fulltext blocks. A row appearing in both is informative ("DPD has it as a lemma; Fulltext also matched the definition body"). Revisit only if user feedback later asks for dedup.
17. Combined is rejected for `SearchArea::Dictionary` at the per-task-layer (`query_task.rs::results_page`) and dispatched at the bridge layer (`SuttaBridge::results_page`). For `SearchArea::Suttas` and `SearchArea::Library`, the bridge falls through to the existing `FulltextMatch` path (we are not introducing a Suttas-side combined mode in this PRD).

#### 5.4.2 Threading model

18. The existing `SuttaBridge::results_page` already runs in a `thread::spawn` and replies via `qt_thread.queue(…)`. For Combined we fork **two** child threads inside that thread, one per sub-query, and `join()` them before merging:
    - Each child constructs its own `SearchQueryTask` against `&app_data.dbm`. Because `app_data` is effectively `'static` and each task opens its own SQLite connection, no shared mutable state crosses thread boundaries. No `Arc<Mutex<…>>` is required for the queries themselves.
    - Diesel's `SqliteConnection` is `!Sync`, but we never share one — each thread takes its own.
    - Tantivy's `Searcher` is `Send + Sync`; running it concurrently with a SQLite read is safe.
    - The fan-out only happens on cold-cache fetches that need data from a side. Top-up fetches on later pages are typically single-sided and run inline.
19. No async runtime is introduced. The pattern matches the rest of the bridge (lines 181, 1039, 1053, 1070… all use plain `std::thread`).

#### 5.4.3 Algorithm — per Combined page request

For combined page `P` with combined page length `L`:

20. Compute the required global slice `[lo, hi) = [P*L, (P+1)*L)`.
21. Determine which sub-buffers must be filled to cover `[lo, hi)`:
    - DPD slice: indexes `[max(lo, 0), min(hi, dpd_total))` if `lo < dpd_total`, else empty.
    - FT slice: indexes `[max(lo - dpd_total, 0), max(hi - dpd_total, 0))` if `hi > dpd_total`, else empty.
22. **Page 0 (cold start)**: `dpd_total` and `ft_total` are unknown. Spawn DPD-page-0 and Fulltext-page-0 concurrently, `join()`, populate `dpd_buffer` / `ft_buffer`, record both totals. This is the only point where parallelism gives a wall-clock win — both backends warm at once.
23. **Subsequent pages**: top up only the side(s) whose buffer falls short of the required slice end. In the common case where DPD is exhausted (`P*L >= dpd_total`), only Fulltext is fetched, with sub-page offset `P*L - dpd_total`. In the rare case both sides need more, run them concurrently again.
24. Slice the merged stream to produce the combined page, memoise it, ship it to QML.
25. Sub-page sizing equals the combined page length `L`. This means page 0 may over-fetch up to `2L` rows (one full sub-page from each side, of which only `L` are displayed); the surplus is buffered and consumed by later pages. Subsequent pages over-fetch at most `L` rows (one side only). Don't try to size sub-pages adaptively — the buffering reuses every fetched row eventually, so the cost is at most one extra sub-page per side per session.

#### 5.4.4 Caching

26. **One `CombinedCache` struct, isolated from `RESULTS_PAGE_CACHE`.** Combined holds both sub-query buffers in a single struct under a single `Mutex`, mirroring the shape and lifecycle of the existing `RESULTS_PAGE_CACHE`:
    ```rust
    struct CombinedCache {
        cache_key: String,
        page_len: usize,
        dpd_buffer: Vec<SearchResult>,
        dpd_total: Option<i64>,
        dpd_pages_fetched: usize,
        ft_buffer: Vec<SearchResult>,
        ft_total: Option<i64>,
        ft_pages_fetched: usize,
    }
    static COMBINED_CACHE: Mutex<Option<CombinedCache>> = Mutex::new(None);
    ```
    `RESULTS_PAGE_CACHE` is untouched by Combined. The two sub-buffers are distinct fields (`dpd_buffer` / `ft_buffer`) that cache the parallel DPD and Fulltext background queries, but they always change together within a Combined search and are therefore held in one cell under one mutex — the standard Rust idiom for fields that share a lifecycle. The merged combined page is computed on demand by slicing the two `Vec<SearchResult>`s; there is no third memoisation layer because the slice is cheap and the prefetcher already keeps both buffers warm.
27. Cache key for Combined is `format!("{}|{}|{}|combined", query_text, area, params_json)` — distinct from the standalone-DPD-Lookup and standalone-Fulltext keys in `RESULTS_PAGE_CACHE`. Switching modes between Combined and one of its sub-modes triggers fresh fetches, which is the desired behaviour: it preserves the invariant that `RESULTS_PAGE_CACHE` is a single-mode cache and avoids any chance of a Combined sub-fetch warming or being warmed by an unrelated single-mode call (params equality across modes is not enough to guarantee result equivalence — bold-definition gating, post-filter ordering, and result formatting can diverge).
28. **One lock acquisition per critical section.** Reading the merged slice needs a coherent snapshot of `(dpd_buffer, dpd_total, ft_buffer, ft_total)`, which the single mutex provides for free. The "cache key changed → abort" check is one comparison; either both sides reflect the active `cache_key` or the cache cell is `None` (post-reset) — there is no intermediate state where one side is current and the other isn't.
29. **Lock discipline.** The page-0 fan-out runs the two sub-queries *without* holding the lock; each child thread takes the lock briefly only to write back its buffer/total. Top-up fetches on later pages do the same: drop the lock, run the sub-query, re-acquire the lock to install the result and re-check `cache_key`. The lock is never held across an SQLite or Tantivy call.
30. Prefetch (today: P+1 inline, P+2/P+3 in background) extends to Combined. The prefetcher dispatches through the same Combined entry point, so it inherits the merge logic and the side-aware top-up.

#### 5.4.5 Error handling

31. If either sub-query errors on page 0, surface the error through the existing `results_page_ready` error-payload path. Do not partial-render; Combined is an atomic operation per page from QML's perspective.
32. If a top-up fetch fails on a later page (rare; the index is already warm), return the error the same way and do not poison the cache — drop the partial top-up. The next user action retries.

#### 5.4.6 Pagination semantics, summarised

| Sub-query  | Pagination source                        | `total` accuracy |
|------------|------------------------------------------|------------------|
| DPD Lookup | `dpd_lookup_full` materialise + slice    | exact (modulo the post-filter caveat from §2.5, which doesn't bite here because `"dpd"` is in the set when DPD is enabled) |
| Fulltext   | Tantivy `search_dict_with_count`         | exact (push-down) |
| Combined   | merged virtual stream of the two above   | exact (`dpd_total + ft_total`) |

### 5.5 QML wiring

33. `assets/qml/SearchBarInput.qml::search_mode_dropdown`:
    - For `search_area === "Dictionary"`, the model becomes:
      ```
      ["Combined", "DPD Lookup", "Fulltext Match", "Contains Match", "Headword Match"]
      ```
      with narrow-screen labels:
      ```
      ["Combined", "Lookup", "Fulltext", "Contains", "Headword"]
      ```
    - The default `currentIndex` for Dictionary becomes `0` (Combined). Today the dropdown remembers its own `currentIndex` only by position; switching to a new default requires adjusting the initial `currentIndex` when `search_area === "Dictionary"` and there is no persisted user choice yet.
34. The JSON sent to the backend uses the wide label string (existing `get_text()` contract). The literal `"Combined"` deserialises to the `SearchMode::Combined` enum variant.
35. No change to `compute_dict_search_filter` — its output already feeds through `dict_source_uids` and `include_comm_bold_definitions` which every backend handler will read.
36. No new QML components are added. Combined returns through the same `results_page_ready` signal as every other mode and renders into the existing `FulltextResults` mount point.

### 5.6 Persisted default

37. Persist the user's last-used dictionary search mode under a new `app_settings` key `dict_search.last_mode` (default `"Combined"`). On `SuttaSearchWindow` open, when `search_area === "Dictionary"`, restore that index. When the user switches to Dictionary from another area, restore as well. Switching the dropdown updates the setting.
38. Migration: existing installs do not have the key. The default kicks in on first read. No data migration required.

## 6. Behavioural Matrix (target state)

| Mode            | DPD checkbox | Commentary checkbox | User-imported checkboxes | Solo any row | Notes                                    |
|-----------------|--------------|---------------------|--------------------------|--------------|------------------------------------------|
| Combined        | ✔ filters    | ✔ filters bold       | ✔ filters                | ✔            | Default. Bridge fires DPD + Fulltext in parallel on page 0; merges into `[DPD …, Fulltext …]`; single signal back to QML. |
| DPD Lookup      | (always on)  | ✔ filters bold       | (ignored)                | ✔ (DPD-only) | Stays DPD-only.                          |
| Fulltext Match  | ✔ filters    | ✔ filters bold       | ✔ filters                | ✔            | Already works. No change.                |
| Contains Match  | ✔ filters    | ✔ filters bold       | ✔ filters (NEW retrieval) | ✔            | Adds unified `dict_words_fts` retrieval phase covering both `word` and `definition_plain`. |
| Headword Match  | ✔ filters    | ✔ filters bold       | ✔ filters (NEW retrieval) | ✔            | Adds user-headword path via `dict_words_fts.word`. |
| Uid Match       | ✔ filters    | n/a                  | ✔ filters                | ✔            | Post-filter is sufficient.               |

"Solo X" means: when `compute_dict_search_filter` returns a single-element `dict_source_uids` set, that mode returns matches only from that dictionary (or zero matches if the mode is structurally incompatible — e.g. soloing a user dictionary under DPD Lookup yields zero results, which is the documented behaviour).

## 7. Technical Considerations

- **Schema change is one-shot.** Adding `word` to `dict_words_fts` is a re-bootstrap — there is no Diesel migration because the FTS table is recreated by `scripts/dictionaries-fts5-indexes.sql`. The user re-bootstraps manually after the script change lands.
- **Trigram FTS matches `LIKE '%term%'` efficiently.** Both Phase 3 and Phase 5 stay on `LIKE` predicates rather than `MATCH`, which keeps the SQL shape consistent with existing code and lets the trigram tokenizer handle substring matching transparently.
- **`dict_label` is filtered via JOIN to `dict_words`**, which has both `dict_words_dict_label_idx` and the composite `(dict_label, word)` index. Filtering on `dict_words_fts.dict_label` directly is a scan because the column is `UNINDEXED` in the virtual table.
- **`apply_dict_source_uids_filter` stays in the dispatcher** as a safety net even when the per-mode handler restricts retrieval. It becomes a no-op for Contains and Headword in normal operation; emit `debug!("…dropped {} rows", n)` on any non-zero drop so regressions surface quickly.
- **Combined is bridge-orchestrated, not UI-orchestrated.** The QML interface — `results_page` call → `results_page_ready` signal — is identical to every other mode. No new QML files, no two-pager rendering, no render gates. The dropdown change and the persisted-mode setting are the only QML touch-points.
- **One isolated `CombinedCache` for Combined**, a top-level `static Mutex<Option<CombinedCache>>` separate from `RESULTS_PAGE_CACHE`. The struct holds both the DPD and Fulltext sub-buffers (`dpd_buffer` / `ft_buffer`) plus their totals and pages-fetched counters; one mutex covers a coherent snapshot of both sides. The merged combined page is computed on demand by slicing both buffers; no second memoisation layer. The lock is never held across an SQLite or Tantivy call — sub-queries run unlocked and write back briefly.
- **Each parallel sub-query opens its own SQLite connection** via the existing pool / `dbm` access pattern. No shared `SqliteConnection` crosses threads.
- **`SearchMode::Combined` is rejected by `query_task.rs::results_page` for `SearchArea::Dictionary`** so accidental backend invocations fail loudly. The bridge is the sole orchestrator.
- All filesystem existence checks (none expected here, but for any new code paths) must use `try_exists()` per CLAUDE.md.

## 8. Test Plan

1. **Schema**: after re-bootstrap, verify `PRAGMA table_info(dict_words_fts)` lists `word` and `definition_plain` and that the INSERT/UPDATE triggers populate `word` for new rows.
2. **ContainsMatch + user dict word**: import a small fixture StarDict whose `word` column contains a token absent from DPD (e.g. `xyzzy`). Search `xyzzy`. With the user dict checkbox **on**, expect ≥1 hit; with it **off**, expect 0 hits.
3. **ContainsMatch + user dict definition_plain**: same fixture, term present only in `definition_plain`. Verify the count is unchanged after the change (existing Phase 3 behaviour preserved).
4. **HeadwordMatch + user dict**: same fixture. Search `xyzzy`. With the user dict checkbox **on**, expect the headword to appear; with it **off**, expect 0 hits. Solo the user dict → only that headword.
5. **DpdLookup + user dict checkbox toggling**: confirm toggling user-imported checkboxes does not change the result set. Solo a user dict → expect 0 results.
6. **Combined — page 0**: search a term that hits both DPD lemmas and a user dictionary's content. Expect a single results list with DPD rows first, then Fulltext rows, page-trimmed to `page_len`. `total = dpd_total + ft_total`.
7. **Combined — page boundary**: pick a query and `page_len` such that combined page 1 spans the DPD/Fulltext boundary (DPD ends mid-page). Verify the page contains the tail of DPD followed by the head of Fulltext, in that order.
8. **Combined — DPD-exhausted page**: paginate beyond `dpd_total / page_len`. Verify the bridge does not re-issue a DPD sub-fetch (instrument with a debug log) and that the page contents come entirely from the Fulltext buffer at offset `P*L - dpd_total`.
9. **Combined — cache isolation**: run a Combined search, then immediately run a standalone DPD Lookup with the same query and params. Verify the standalone call hits the backend (not the Combined sub-cache) and produces a result independent of Combined's state.
10. **Combined — sub-query error**: simulate a Fulltext failure on page 0 and verify Combined surfaces an error payload through `results_page_ready` rather than partial-rendering.
11. **Default mode**: open Dictionary tab on a fresh install → search-mode dropdown shows `Combined`. Switch to `DPD Lookup`, restart → opens on `DPD Lookup`.
12. **Lock interaction across modes**: in each of Combined / Fulltext / Contains / Headword, lock each row in turn (DPD, Commentary, each user dict). Verify only that row contributes.
13. **No-imports baseline**: with zero user dictionaries imported, verify result counts in Contains and Headword for several known queries are unchanged from current behaviour.
14. **Solo non-DPD under DPD Lookup**: lock a user dictionary while `DPD Lookup` is selected. Expect 0 results (documented behaviour, §5.3).
15. **`make build -B`** must succeed. `cd backend && cargo test` must not introduce new failures.

## 9. Implementation Sketch (file map, not a task list)

- `scripts/dictionaries-fts5-indexes.sql` — add `word` column to `dict_words_fts`; update seed `INSERT … SELECT` and INSERT/UPDATE triggers.
- `backend/src/types.rs` — no change; `SearchMode::Combined` already exists in the enum.
- `backend/src/query_task.rs`:
  - Add `dict_label_in_clause` helper (placeholders + binds).
  - Replace `dict_words_contains_match_fts5_full` Phase 3 with a unified `dict_words_fts`-driven path covering both `word` and `definition_plain`, JOINed to `dict_words` for `dict_label IN (set)`.
  - Add Phase 5 for user-headword substring on `dict_words.word` via `dict_words_fts.word LIKE`.
  - Switch dedup key from `result.word` to `result.id`.
  - Extend `lemma_1_dpd_headword_match_fts5_full` with a Path B against `dict_words_fts.word LIKE` for the non-DPD subset; merge with the existing DPD path. Keep the function name (avoids the rename churn from the original draft); document Path A / Path B inside.
  - In `results_page`, for `SearchMode::Combined + SearchArea::Dictionary` return an explicit `Err`. For `SearchMode::Combined + (Suttas | Library)` shadow to `FulltextMatch` and dispatch normally.
- `backend/src/search/searcher.rs` — no behavioural change required; `add_dict_filters` already handles the Fulltext side.
- `backend/src/app_settings.rs` — add `dict_search_last_mode: Option<String>` (default `None`, treated as `"Combined"` on read).
- `backend/src/app_data.rs` — add `get_last_dict_search_mode()` / `set_last_dict_search_mode(mode)` accessors.
- `bridges/src/sutta_bridge.rs`:
  - Add `CombinedCache` struct + `static COMBINED_CACHE: Mutex<Option<CombinedCache>>` near the existing `RESULTS_PAGE_CACHE`.
  - Add `fetch_combined_page(cache_key, query, params_json, page_num)` that runs the §5.4.3 algorithm: page-0 fan-out via two `thread::spawn` + `join`, later-page side-aware top-up.
  - In `SuttaBridge::results_page`, dispatch `(area=Dictionary, mode=Combined)` to `fetch_combined_page` instead of `fetch_and_cache_page`. The signal-emission and "cache key changed → abort" paths are reused unchanged.
  - Teach the prefetcher to delegate Combined the same way.
- `bridges/src/dictionary_manager.rs` — add `get_last_dict_search_mode()` / `set_last_dict_search_mode(mode)` for the dropdown.
- `assets/qml/com/profoundlabs/simsapa/DictionaryManager.qml` — qmllint stubs for the two new methods.
- `assets/qml/SearchBarInput.qml` — add `Combined` to the Dictionary model lists; restore + persist via `DictionaryManager`.
- Tests:
  - `backend/tests/dict_modes_filtering.rs` — fixture-driven tests for the Contains and Headword retrieval changes and for DPD Lookup's invariants (§8 items 2–5, 14).
  - `bridges/tests/combined_dict_results.rs` (or inline in `sutta_bridge.rs`'s test module) — Combined-specific tests covering merge ordering, page-boundary correctness, DPD-exhausted top-up, and cache isolation (§8 items 6–9).

## 10. Open Questions

- Should the visual presentation of Combined later distinguish DPD vs Fulltext rows (e.g. a small per-row badge or a divider row between blocks)? Default for v1: no visual distinction; rows are listed in merged order. Re-evaluate after dogfooding.
- Should `word_ascii` also be trigram-indexed alongside `word` for diacritic-insensitive headword search? Out of scope for this PRD; revisit if user feedback asks for it.
- Should `HeadwordMatch` expose a separate "exact only" sub-toggle now that user dictionaries can swamp the result list with contains-style matches? Default for v1: keep the existing `LIKE '%term%'` semantics for symmetry with DPD's behaviour. Revisit if the result set becomes noisy.
