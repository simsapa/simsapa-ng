# PRD: Fix FulltextMatch Search Pagination

## 1. Introduction/Overview

The `SearchMode::FulltextMatch` results do not paginate correctly. Clicking the next/previous page buttons in `FulltextResults.qml` always returns the same first page of results. This affects both sutta and dictionary fulltext searches.

The root causes are:

1. **`fulltext_suttas()` and `fulltext_dict_words()` ignore the `page_num` parameter** — the Tantivy searcher always returns only the first `page_len` results via `TopDocs::with_limit(page_len)` with no offset.
2. **The page cache (`highlighted_result_pages`) is ineffective** — a new `SearchQueryTask` is created for every page request due to a Rust lifetime constraint (`&'a DbManager`), so the cache HashMap is always empty.
3. **CST mula/commentary filtering happens post-fetch** in `fulltext_suttas()`, causing inaccurate total counts and potentially short pages.

The working `ContainsMatch` implementation uses SQL `LIMIT/OFFSET` for pagination and `COUNT(*)` for totals, serving as the reference model.

## 2. Goals

1. Clicking next/previous page buttons in fulltext search results displays the correct page of results.
2. Total hit counts accurately reflect filtered results so page counts are correct.
3. Previously viewed pages are served from cache without re-querying Tantivy or re-highlighting.
4. Results are stably ordered across pages (no duplicates or gaps when paginating).
5. Multi-language index results are correctly merged and ranked by score across all pages.

## 3. User Stories

- As a user, I want to browse through fulltext sutta search results page by page so I can find relevant suttas beyond the first 10 results.
- As a user, I want to browse through fulltext dictionary search results page by page.
- As a user, I want to go back to a previously viewed page of results without waiting for a re-query.
- As a user, I want accurate page counts so I know how many pages of results exist.

## 4. Functional Requirements

### 4.1 Tantivy Searcher: Add offset/limit pagination

**Files:** `backend/src/search/searcher.rs`

**FR-1:** Change `search_suttas_with_count()` and `search_dict_words_with_count()` signatures to accept `page_num` in addition to `page_len`.

**FR-2:** Change `search_indexes()` and `search_single_index()` to accept `page_num` and `page_len`, and use `TopDocs::with_limit(limit).and_offset(offset)` for Tantivy pagination.

**FR-3: Multi-index pagination algorithm.** When searching across multiple language indexes, results must be correctly ranked by score across all indexes for any page. The algorithm:

- For each language index, fetch `(page_num + 1) * page_len` results (enough to cover all pages up to the requested one) using `TopDocs::with_limit((page_num + 1) * page_len)`.
- Merge all per-index results into a single list sorted by score descending.
- Apply `skip(page_num * page_len).take(page_len)` to get the correct page.
- The `Count` collector continues to return the total across all indexes (unchanged).

This ensures stable cross-language ranking. The cost is fetching more results from Tantivy on later pages, but Tantivy's `TopDocs` collector is efficient and the extra documents are lightweight scored references until materialized.

**FR-4:** `search_indexes()` returns `(total_hits_across_all_indexes, paginated_results_for_requested_page)`.

### 4.2 CST/Commentary Filtering: Add indexed boolean fields

**Files:** `backend/src/search/schema.rs`, `backend/src/search/indexer.rs`, `backend/src/search/searcher.rs`, `backend/src/query_task.rs`

The current approach filters CST mula/commentary results post-fetch in `fulltext_suttas()` using `.retain()` on uid patterns. This causes inaccurate total counts and potentially short pages. The fix is to add indexed boolean fields at index build time so Tantivy can filter efficiently during the query.

**FR-5:** Add `is_mula` and `is_commentary` boolean fields to `SearchFilters`:

```rust
pub struct SearchFilters {
    // ... existing fields ...
    pub include_mula: bool,
    pub include_commentary: bool,
}
```

**FR-6: Add `is_mula` and `is_commentary` fields to the Tantivy sutta index schema.**

In `schema.rs`, add two new `INDEXED | STORED` boolean fields to `build_sutta_schema()`:

```rust
builder.add_bool_field("is_mula", INDEXED | STORED);
builder.add_bool_field("is_commentary", INDEXED | STORED);
```

These are general-purpose fields that apply to all sutta entries, not just CST sources. The classification logic at index build time (in `indexer.rs`) determines the values based on the uid:

- **`is_commentary = true`** when the uid contains `.att` or `.tik` before the first `/` (matches atthakatha and tika texts). Examples: `dn1.att/pli/cst`, `sn1.2.tik.xml/pli/cst`.
- **`is_mula = true`** when `is_commentary` is false (the sutta is a root/mula text). This covers all sources (ms, bodhi, cst, etc.), not just CST.

This generalizes beyond the current CST-only filtering. All data needed for this classification is available at index time from the `Sutta.uid` field.

**FR-7: Populate the new fields during index building.**

In `indexer.rs` `build_sutta_index()`, determine `is_mula` and `is_commentary` for each sutta before adding the document:

```rust
let before_first_slash = sutta.uid.split('/').next().unwrap_or("");
let is_commentary = before_first_slash.contains(".att") || before_first_slash.contains(".tik");
let is_mula = !is_commentary;

writer.add_document(doc!(
    // ... existing fields ...
    is_mula_field => is_mula,
    is_commentary_field => is_commentary,
))?;
```

**FR-8: Filter using the indexed fields in `add_sutta_filters()`.**

In `searcher.rs`, when `include_mula` is false, add a `MustNot` clause for `is_mula = true`. When `include_commentary` is false, add a `MustNot` clause for `is_commentary = true`. Since these are indexed boolean fields, Tantivy handles the filtering during the search and the `Count` collector returns accurate totals.

**FR-9: Remove the post-fetch `.retain()` filtering** in `fulltext_suttas()` (`query_task.rs`). The filtering is now handled by Tantivy, so post-fetch filtering is no longer needed. The `total` returned by the searcher is already accurate.

**FR-10: Pass `include_mula` and `include_commentary` flags** from `fulltext_suttas()` to `SearchFilters` using the existing `app_data.get_include_cst_mula_in_search_results()` and `app_data.get_include_cst_commentary_in_search_results()` settings.

### 4.3 Query Task: Use page_num in fulltext methods

**Files:** `backend/src/query_task.rs`

**FR-11:** In `fulltext_suttas()`, change `_page_num` to `page_num` and pass it through to the searcher's `search_suttas_with_count()`.

**FR-12:** In `fulltext_dict_words()`, change `_page_num` to `page_num` and pass it through to the searcher's `search_dict_words_with_count()`.

### 4.4 Page Cache: Persist across page navigations

**Files:** `bridges/src/sutta_bridge.rs`, `backend/src/query_task.rs`

Currently, `results_page()` in `sutta_bridge.rs` creates a new `SearchQueryTask` on every call because the struct holds `&'a DbManager`. This makes the `highlighted_result_pages` cache useless.

**FR-13:** Introduce a persistent cache on `SuttaBridgeRust` that survives across page requests for the same query. Store a struct on the bridge containing:
- The cache key: query text + search parameters (search mode, search area, language, source).
- A `HashMap<usize, Vec<SearchResult>>` keyed by page number.
- The `total_hits` count (so cached pages can return the correct total without re-querying).

On page request:
1. Check if the cache key matches the current request parameters.
2. If the key matches and the requested page is cached, serialize and emit `results_page_ready` immediately.
3. If the key doesn't match (new search), clear the cache and update the key.
4. If the page is not cached, create `SearchQueryTask`, run the query, highlight, store results in bridge cache, then emit.

This avoids the lifetime issue entirely since the cache owns its data (cloned `Vec<SearchResult>`, not references).

**FR-14:** The cache must be invalidated (cleared) when the cache key (query text + parameters) changes, which naturally happens when the user performs a new search or changes any search parameter.

**FR-15:** `total_hits` should be stored once on the first query and returned with all subsequent cached page responses.

### 4.5 QML: Fix pagination controls

**Files:** `assets/qml/FulltextResults.qml`

**FR-16:** Set `fulltext_last_page_btn` to `visible: false` (deferred for later implementation).

**FR-17:** Set the `SpinBox` (`fulltext_page_input`) to `visible: false` (deferred for later implementation).

**FR-18:** Wire `fulltext_first_page_btn` to jump to page 0:
```qml
onClicked: {
    fulltext_list.positionViewAtBeginning();
    root.page_num = 0;
    root.new_results_page_fn(root.page_num);
}
```

## 5. Non-Goals (Out of Scope)

- Last page button functionality (hidden for now).
- SpinBox page input functionality (hidden for now).
- Infinite scroll / virtual scrolling as an alternative to pagination.
- Changes to `ContainsMatch` or other search modes (they work correctly).
- Performance optimization of Tantivy queries beyond what's needed for correct pagination.

## 6. Technical Considerations

### Tantivy API

- `TopDocs::with_limit(n).and_offset(offset)` is the standard Tantivy pagination API. The `Count` collector returns total matches independent of limit/offset.
- When combining results from multiple language indexes, each index's `Count` is additive for total hits.

### Lifetime constraint

- `SearchQueryTask<'a>` holds `&'a DbManager` which prevents storing it on the CXX-Qt bridge struct. The bridge-level cache (FR-10 Option A) sidesteps this by caching only the results, not the task.

### Existing cache mechanism

- The `highlighted_result_pages` HashMap in `SearchQueryTask` can remain for use within a single task lifetime (e.g., if future refactoring stores the task). But for now, the bridge-level cache (FR-13) is the effective cache.

### Index structure

- Sutta indexes are per-language: `sutta_indexes: HashMap<String, (Index, IndexReader)>`.
- Dict indexes are per-language: `dict_indexes: HashMap<String, (Index, IndexReader)>`.

### Index schema changes

- Adding `is_mula` and `is_commentary` boolean fields requires rebuilding the Tantivy indexes. Since the app hasn't been released yet, manually rebuild the index locally after the schema change. No `INDEX_VERSION` bump needed for now.
- The `is_mula`/`is_commentary` classification is general-purpose and source-agnostic. It uses uid patterns (`.att`, `.tik` before the first `/`) that apply across all sources, not just CST. This makes the fields useful for future filtering needs beyond the current CST settings.

## 7. Success Metrics

- Fulltext sutta search: clicking next/prev shows different, correct pages of results.
- Fulltext dictionary search: same as above.
- Going back to a previously viewed page returns instantly (from cache).
- Total page count shown in the UI matches the actual number of pages of results.
- No duplicate results across pages; no results missing between pages.

## 8. Resolved Questions

1. **Dict word index schema:** No — dictionary entries don't have mula/commentary distinction. The `is_mula`/`is_commentary` fields are only added to the sutta index schema. `add_dict_filters()` skips these fields.
2. **Index version migration:** Not needed — the app hasn't been released yet. Manually rebuild the index locally after the schema change.
