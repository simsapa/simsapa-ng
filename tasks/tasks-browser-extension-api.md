# Tasks: Browser Extension API Support

This task list implements the Browser Extension API routes as specified in `prd-browser-extension-api.md`.

## Relevant Files

### Modified Files

- `bridges/src/api.rs` - Main API routes file with all new browser extension endpoints (ApiSearchResult, ApiSearchRequest, SearchOptions, LookupWindowRequest structs and 8 route handlers)
- `backend/src/db/dictionaries.rs` - Added `get_distinct_languages()` and `get_distinct_sources()` methods for filter options
- `backend/src/db/dictionaries_models.rs` - Added `serde::Serialize` derive to `DictWord` struct for JSON serialization
- `cpp/window_manager.cpp` - Modified `run_lookup_query()` to open SuttaSearchWindow instead of WordLookupWindow

### Reference Files (not modified)

- `backend/src/db/appdata.rs` - Existing `get_sutta_languages()`, `get_sutta()`, `get_dpd_headword_by_uid()`, `get_dpd_root_by_root_key()` functions
- `backend/src/db/dpd.rs` - Existing `dpd_deconstructor_list()` function
- `backend/src/query_task.rs` - Existing `SearchQueryTask` with `results_page()` and `total_hits()` methods
- `backend/src/types.rs` - Existing `SearchResult`, `SearchParams`, `SearchMode`, `SearchArea` types
- `cpp/gui.cpp` - Existing FFI callback declarations

### Notes

- Rust tests: `cd backend && cargo test`
- Build: `make build -B`
- The API server runs on localhost:4848 by default
- CORS is already configured to allow all origins (required for browser extensions)

## Tasks

- [x] 1.0 Add API data structures for browser extension responses
  - [x] 1.1 Create `ApiSearchResult` struct in `bridges/src/api.rs` with fields: `hits` (i32), `results` (Vec<SearchResult>), `deconstructor` (Option<Vec<String>>)
  - [x] 1.2 Create `ApiSearchRequest` struct for POST body parsing with fields: `query_text` (String), `page_num` (Option<i32>), `suttas_lang` (Option<String>), `suttas_lang_include` (Option<bool>), `dict_lang` (Option<String>), `dict_lang_include` (Option<bool>), `dict_dict` (Option<String>), `dict_dict_include` (Option<bool>)
  - [x] 1.3 Create `SearchOptions` struct with fields: `sutta_languages` (Vec<String>), `dict_languages` (Vec<String>), `dict_sources` (Vec<String>)
  - [x] 1.4 Create `LookupWindowRequest` struct with field: `query_text` (String)
  - [x] 1.5 Add necessary `#[derive(Serialize, Deserialize)]` and serde attributes (e.g., `#[serde(skip_serializing_if = "Option::is_none")]` for deconstructor field)
  - [x] 1.6 Verify the project compiles with `make build -B`

- [x] 2.0 Implement dictionary database helper functions
  - [x] 2.1 Add `get_distinct_languages()` method to `DictionariesDbHandle` in `backend/src/db/dictionaries.rs` that returns `Vec<String>` of distinct language values from dict_words table
  - [x] 2.2 Add `get_distinct_sources()` method to `DictionariesDbHandle` in `backend/src/db/dictionaries.rs` that returns `Vec<String>` of distinct dict_label values from dict_words table
  - [x] 2.3 Handle NULL values gracefully in both functions (filter out None values)
  - [x] 2.4 Sort the returned vectors alphabetically
  - [x] 2.5 Verify compilation with `make build -B`

- [x] 3.0 Implement GET `/sutta_and_dict_search_options` route
  - [x] 3.1 Add route handler function `get_search_options()` in `bridges/src/api.rs`
  - [x] 3.2 Call `dbm.appdata.get_sutta_languages()` to get sutta languages
  - [x] 3.3 Call `dbm.dictionaries.get_distinct_languages()` to get dictionary languages
  - [x] 3.4 Call `dbm.dictionaries.get_distinct_sources()` to get dictionary sources
  - [x] 3.5 Return `Json<SearchOptions>` with the collected data
  - [x] 3.6 Register the route in `rocket::build().mount()` routes list
  - [x] 3.7 Test with curl: `curl http://localhost:4848/sutta_and_dict_search_options`

- [x] 4.0 Implement POST `/suttas_fulltext_search` route
  - [x] 4.1 Add route handler function `suttas_fulltext_search()` in `bridges/src/api.rs` that accepts `Json<ApiSearchRequest>`
  - [x] 4.2 Parse request body and extract `query_text`, `page_num` (default 0), `suttas_lang`, `suttas_lang_include` (default true)
  - [x] 4.3 Create `SearchParams` with `mode: SearchMode::ContainsMatch`, `lang` filter if provided and not "Languages"
  - [x] 4.4 Create `SearchQueryTask` with `SearchArea::Suttas` and call the search
  - [x] 4.5 Use `SearchQueryTask::suttas_contains_match_fts5()` for the actual search
  - [x] 4.6 Get paginated results using `results_page()` with page_len=20
  - [x] 4.7 Convert results to `ApiSearchResult` format with total hits count
  - [x] 4.8 Register the route in routes list
  - [x] 4.9 Test with curl: `curl -X POST -H "Content-Type: application/json" -d '{"query_text":"dhamma"}' http://localhost:4848/suttas_fulltext_search`

- [x] 5.0 Implement POST `/dict_combined_search` route
  - [x] 5.1 Add route handler function `dict_combined_search()` in `bridges/src/api.rs`
  - [x] 5.2 Parse request body for `query_text`, `page_num`, `dict_lang`, `dict_lang_include`, `dict_dict`, `dict_dict_include`
  - [x] 5.3 Create `SearchParams` with appropriate language and source filters
  - [x] 5.4 Create `SearchQueryTask` with `SearchArea::Dictionary` and perform search using `dict_words_contains_match_fts5()`
  - [x] 5.5 Call `dbm.dpd.dpd_deconstructor_list(&query_text)` to get deconstructor results
  - [x] 5.6 Get paginated results with page_len=20
  - [x] 5.7 Return `ApiSearchResult` with results and deconstructor field populated
  - [x] 5.8 Register the route in routes list
  - [x] 5.9 Test with curl: `curl -X POST -H "Content-Type: application/json" -d '{"query_text":"dhamma"}' http://localhost:4848/dict_combined_search`

- [x] 6.0 Basic manual test with browser extension
  - [x] 6.1 Build and run the Simsapa app with `make build -B && make run`
  - [x] 6.2 Install/enable the Simsapa browser extension in Firefox or Chrome
  - [x] 6.3 Verify the extension sidebar shows "online" status (green indicator) when app is running
  - [x] 6.4 Test sutta search: enter a search term (e.g., "dhamma") in the Suttas tab and verify results appear
  - [x] 6.5 Test dictionary search: enter a search term in the Dictionary tab and verify results appear
  - [x] 6.6 Verify search results display correctly with title, snippet, and other metadata
  - [x] 6.7 Verify filter dropdowns populate with language and source options
  - [x] 6.8 Document any issues found for follow-up fixes

- [x] 7.0 Implement GET `/suttas/<uid>` route for opening suttas in app
  - [x] 7.1 Add route handler function `open_sutta_by_uid()` in `bridges/src/api.rs` with path parameter `uid: PathBuf`
  - [x] 7.2 Convert PathBuf to string using `pathbuf_to_forward_slash_string()` helper (already exists in api.rs)
  - [x] 7.3 Query sutta from database using `dbm.appdata.get_sutta(&uid_str)`
  - [x] 7.4 If sutta found, create JSON with `item_uid`, `table_name`, `sutta_title`, `sutta_ref`, `snippet` fields
  - [x] 7.5 Call `ffi::callback_open_sutta_search_window()` with the JSON string
  - [x] 7.6 Return plain text response: "The Simsapa window should appear with '{uid}'. You can close this tab."
  - [x] 7.7 Handle sutta not found case - try fallback to pli/ms version like existing `open_sutta_window()` does
  - [x] 7.8 Register the route (note: this is different from existing `/open_sutta_window/<uid>` - new route is `/suttas/<uid>`)
  - [x] 7.9 Test by opening browser to `http://localhost:4848/suttas/sn12.2/en/bodhi`

- [x] 8.0 Implement POST `/lookup_window_query` route
  - [x] 8.1 Add route handler function `lookup_window_query_post()` in `bridges/src/api.rs` that accepts `Json<LookupWindowRequest>`
  - [x] 8.2 Extract `query_text` from the request body
  - [x] 8.3 Call existing `ffi::callback_run_lookup_query()` with the query text (this FFI function already exists)
  - [x] 8.4 Return plain text "OK" with Status::Ok
  - [x] 8.5 Register the route in routes list
  - [x] 8.6 Modified `WindowManager::run_lookup_query()` in C++ to open SuttaSearchWindow and run search (instead of WordLookupWindow)
  - [x] 8.7 Test with curl: `curl -X POST -H "Content-Type: application/json" -d '{"query_text":"dhamma"}' http://localhost:4848/lookup_window_query`

- [x] 9.0 Implement GET `/words/<uid>.json` route
  - [x] 9.1 Add route handler function `get_word_json()` in `bridges/src/api.rs` with path parameter for uid
  - [x] 9.2 Parse the uid to determine word type: if ends with "/dpd" and starts with number, it's DPD headword; if contains root pattern, it's DPD root; otherwise it's dict_word
  - [x] 9.3 For DPD headwords: query using `dpd_headwords` table by id, serialize full record to JSON
  - [x] 9.4 For DPD roots: query using `dpd_roots` table, serialize full record to JSON
  - [x] 9.5 For dict_words: use `dbm.dictionaries.get_word()`, serialize to JSON
  - [x] 9.6 Return `Json<Vec<serde_json::Value>>` - array with single word object, or empty array if not found
  - [x] 9.7 Register the route in routes list
  - [x] 9.8 Added `serde::Serialize` derive to `DictWord` struct
  - [x] 9.9 Test with curl: `curl http://localhost:4848/words/dhamma/dpd.json`

- [x] 10.0 Implement GET completion list placeholder routes
  - [x] 10.1 Add route handler `sutta_titles_completion()` that returns `Json<Vec<String>>` with empty vector
  - [x] 10.2 Add route handler `dict_words_completion()` that returns `Json<Vec<String>>` with empty vector
  - [x] 10.3 Register both routes: `/sutta_titles_flat_completion_list` and `/dict_words_flat_completion_list`
  - [x] 10.4 Add TODO comments noting these should query database in future implementation
  - [x] 10.5 Verify routes return empty JSON arrays: `curl http://localhost:4848/sutta_titles_flat_completion_list`

- [x] 11.0 Integration testing and verification
  - [x] 11.1 Verify all 9 routes are registered in the Rocket mount() call (8 browser extension routes + index health check)
  - [x] 11.2 Run full build: `make build -B`
  - [x] 11.3 Run Rust tests: 126 unit tests pass, 27 Anki tests pass, 4 other tests pass (11 database comparison tests fail but are unrelated to browser extension API)
  - [ ] 11.4 Test with actual browser extension: verify sidebar loads and shows "online"
  - [ ] 11.5 Test sutta search from extension and verify results display
  - [ ] 11.6 Test dictionary search from extension and verify results with deconstructor
  - [ ] 11.7 Test clicking "Show in Simsapa" button to open sutta in app window
  - [ ] 11.8 Test clicking word result to open in lookup window
  - [ ] 11.9 Test filter dropdowns populate correctly
  - [ ] 11.10 Document any remaining issues or future improvements needed
