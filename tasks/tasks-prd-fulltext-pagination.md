## Relevant Files

- `backend/src/search/schema.rs` - Tantivy index schema definitions for sutta and dict indexes. Add `is_mula` and `is_commentary` boolean fields to sutta schema.
- `backend/src/search/indexer.rs` - Index building functions. Populate `is_mula`/`is_commentary` based on uid patterns during sutta indexing.
- `backend/src/search/searcher.rs` - Tantivy search logic. Add pagination (page_num) support, add mula/commentary filtering in `add_sutta_filters()`.
- `backend/src/search/types.rs` - `SearchFilters` struct definition. Add `include_mula` and `include_commentary` fields.
- `backend/src/query_task.rs` - Search query execution. Wire `page_num` through `fulltext_suttas()` and `fulltext_dict_words()`, remove post-fetch `.retain()` filtering, pass mula/commentary flags.
- `bridges/src/sutta_bridge.rs` - Qt bridge for search. Add bridge-level page cache on `SuttaBridgeRust`, update `results_page()` to use cache.
- `backend/src/types.rs` - Contains `SearchResultPage`, `SearchParams`, `SearchMode` definitions.
- `assets/qml/FulltextResults.qml` - QML pagination controls. Hide last-page button and SpinBox, wire first-page button.

### Notes

- Use `cd backend && cargo test` to run Rust backend tests.
- Use `make build -B` to verify compilation after changes.
- After schema changes in `schema.rs`/`indexer.rs`, manually rebuild the Tantivy index locally.
- Existing searcher tests are in `backend/src/search/searcher.rs` (line 558, `#[cfg(test)]` module with 8 tests).
- No existing tests for pagination in `query_task.rs` — verify pagination behavior via manual testing.

## Tasks

- [ ] 1.0 Add `is_mula` and `is_commentary` fields to sutta index schema and indexer
  - [ ] 1.1 In `backend/src/search/schema.rs` `build_sutta_schema()`, add two boolean fields: `builder.add_bool_field("is_mula", INDEXED | STORED)` and `builder.add_bool_field("is_commentary", INDEXED | STORED)`.
  - [ ] 1.2 In `backend/src/search/indexer.rs` `build_sutta_index()`, before the `writer.add_document()` call, compute `is_commentary` and `is_mula` from the sutta uid: `let before_first_slash = sutta.uid.split('/').next().unwrap_or(""); let is_commentary = before_first_slash.contains(".att") || before_first_slash.contains(".tik"); let is_mula = !is_commentary;`
  - [ ] 1.3 In the same `writer.add_document(doc!(...))` call, add the new fields: `is_mula_field => is_mula, is_commentary_field => is_commentary`. Get the field handles from the schema like the existing fields.
  - [ ] 1.4 Verify compilation with `cd backend && cargo test`. Manually rebuild the index locally after this change.

- [ ] 2.0 Add mula/commentary filtering to Tantivy searcher and SearchFilters
  - [ ] 2.1 In `backend/src/search/types.rs`, add two new fields to the `SearchFilters` struct: `pub include_mula: bool` and `pub include_commentary: bool`.
  - [ ] 2.2 Update all existing `SearchFilters` construction sites (in `query_task.rs` functions like `fulltext_suttas()`, `fulltext_dict_words()`, and any others) to provide the new fields. For sutta searches, read from `app_data.get_include_cst_mula_in_search_results()` and `app_data.get_include_cst_commentary_in_search_results()`. For dict searches, set both to `true` (no filtering).
  - [ ] 2.3 In `backend/src/search/searcher.rs` `add_sutta_filters()`, add filtering logic: when `filters.include_mula` is `false`, add a `(Occur::MustNot, TermQuery)` clause matching `is_mula = true`. When `filters.include_commentary` is `false`, add a `(Occur::MustNot, TermQuery)` clause matching `is_commentary = true`. Use `Term::from_field_bool(field, true)` for the term.
  - [ ] 2.4 Verify compilation and run existing searcher tests: `cd backend && cargo test`.

- [ ] 3.0 Add pagination (page_num) support to the Tantivy searcher
  - [ ] 3.1 Change `search_suttas_with_count()` signature to accept `page_num: usize` in addition to `page_len: usize`. Pass both through to `search_indexes()`.
  - [ ] 3.2 Change `search_dict_words_with_count()` signature similarly to accept `page_num: usize`. Pass both through to `search_indexes()`.
  - [ ] 3.3 Change `search_indexes()` signature to accept `page_num: usize` and `page_len: usize`. Implement the multi-index pagination algorithm: for each language index, call `search_single_index()` with `limit = (page_num + 1) * page_len` (fetch enough results to cover all pages up to the requested one). After collecting and merging results from all indexes, sort by score descending, then apply `.into_iter().skip(page_num * page_len).take(page_len)` to extract the correct page.
  - [ ] 3.4 Change `search_single_index()` to accept a `limit: usize` parameter instead of `page_len`. Use `TopDocs::with_limit(limit)` (no offset needed here since pagination is handled in `search_indexes()` after merging). The `Count` collector remains unchanged.
  - [ ] 3.5 Update the existing searcher tests to pass `page_num: 0` to the changed function signatures.
  - [ ] 3.6 Verify compilation and run tests: `cd backend && cargo test`.

- [ ] 4.0 Wire page_num through fulltext query task methods and remove post-fetch filtering
  - [ ] 4.1 In `backend/src/query_task.rs` `fulltext_suttas()`, change `_page_num: usize` to `page_num: usize`. Pass `page_num` to `searcher.search_suttas_with_count(&query_text, &filters, page_len, page_num)` (note: argument order should match the new signature from task 3.1).
  - [ ] 4.2 Remove the post-fetch `.retain()` block in `fulltext_suttas()` that filters CST mula/commentary results (the `results.retain(|r| { ... })` block and the `adjusted_total` calculation). Use the `total` returned by the searcher directly: `self.db_query_hits_count = total as i64;`.
  - [ ] 4.3 In `fulltext_dict_words()`, change `_page_num: usize` to `page_num: usize`. Pass `page_num` to `searcher.search_dict_words_with_count(&query_text, &filters, page_len, page_num)`.
  - [ ] 4.4 Verify compilation and run tests: `cd backend && cargo test`.

- [ ] 5.0 Add bridge-level page cache on SuttaBridgeRust
  - [ ] 5.1 Define a `ResultsPageCache` struct (in `sutta_bridge.rs` or a nearby module) containing: `cache_key: String` (serialized query + params for identity comparison), `pages: HashMap<usize, Vec<SearchResult>>` (page_num → highlighted results), and `total_hits: i64`.
  - [ ] 5.2 Add a `results_cache: Option<ResultsPageCache>` field to `SuttaBridgeRust` struct. Initialize to `None` in the default/constructor.
  - [ ] 5.3 In the `results_page()` function in `sutta_bridge.rs`, before creating a new `SearchQueryTask`: build a cache key string from the query text, search area, and params_json. Check if `self.results_cache` exists and its `cache_key` matches. If so, check if the requested `page_num` is in `pages`. If the page is cached, serialize the cached results with the stored `total_hits` into a `SearchResultPage` JSON and emit `results_page_ready` — skip the query entirely.
  - [ ] 5.4 If the cache key doesn't match (new search), replace `self.results_cache` with a new `ResultsPageCache` with the new key and empty pages.
  - [ ] 5.5 After running a query and highlighting results (existing code path), insert the page results into `self.results_cache.pages` and store `total_hits` before emitting the signal.
  - [ ] 5.6 Verify compilation: `make build -B`.

- [ ] 6.0 Fix QML pagination controls in FulltextResults.qml
  - [ ] 6.1 Set `visible: false` on the `fulltext_last_page_btn` Button.
  - [ ] 6.2 Set `visible: false` on the `fulltext_page_input` SpinBox.
  - [ ] 6.3 Add `onClicked` handler to `fulltext_first_page_btn`: `fulltext_list.positionViewAtBeginning(); root.page_num = 0; root.new_results_page_fn(root.page_num);`
  - [ ] 6.4 Verify compilation: `make build -B`.

- [ ] 7.0 Manual testing and verification
  - [ ] 7.1 Rebuild the Tantivy index locally to pick up the new `is_mula`/`is_commentary` schema fields.
  - [ ] 7.2 Test sutta fulltext search: perform a search, verify page 1 shows results, click next to see page 2 with different results, click prev to go back to page 1 (should be instant from cache).
  - [ ] 7.3 Test dictionary fulltext search: same pagination verification as 7.2.
  - [ ] 7.4 Verify total page count in the UI matches actual results (no short pages except possibly the last).
  - [ ] 7.5 Test first-page button jumps back to page 0.
  - [ ] 7.6 Test that performing a new search clears the cache (page 1 of new results, not stale cached results).
  - [ ] 7.7 Test with CST mula/commentary filtering toggled on/off — verify totals change and pages are correct.
