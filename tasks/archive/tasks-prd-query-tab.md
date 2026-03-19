## Relevant Files

- `backend/src/search/searcher.rs` - Add `debug_query()` and `tokenize_to_string()` methods to `FulltextSearcher`
- `backend/src/search/tokenizer.rs` - Reference for tokenizer pipelines (`{lang}_stem`, `{lang}_normalize`)
- `backend/src/search/mod.rs` - Search module structure (may need to re-export helpers)
- `bridges/src/sutta_bridge.rs` - Add `debug_query` bridge method and `debugQueryReady` signal
- `bridges/build.rs` - Register `QueryTab.qml` in `qml_files` list
- `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml` - Add qmllint type definition for `debug_query` and `debugQueryReady`
- `assets/qml/QueryTab.qml` - New file: QueryTab sidebar component
- `assets/qml/SuttaSearchWindow.qml` - Add QueryTab to sidebar TabBar/StackLayout, wire signals, add `has_query_error` property
- `assets/qml/SearchBarInput.qml` - Add `has_query_error` property, update search button icon logic for three states

### Notes

- Rust tests: `cd backend && cargo test`
- QML tests: `make qml-test`
- Build verification: `make build -B`
- Follow the async bridge pattern from `results_page()` in `sutta_bridge.rs` (spawn thread, emit signal)
- Follow `TocTab.qml` as the structural template for `QueryTab.qml`

## Tasks

- [x] 1.0 Implement `tokenize_to_string()` and `debug_query()` in the Rust backend
  - [x] 1.1 Add a `tokenize_to_string()` method on `FulltextSearcher` that accepts an `&Index`, a tokenizer name (e.g., `"pli_stem"`), and query text, runs the tokenizer pipeline, and returns tokens as a comma-separated string
  - [x] 1.2 Add a `debug_query()` method on `FulltextSearcher` that accepts query text and `SearchFilters`, determines which language indexes to search (respecting filters), and returns a formatted `Result<String>`
  - [x] 1.3 In `debug_query()`, for each relevant language index: call `tokenize_to_string()` for both `{lang}_stem` and `{lang}_normalize` tokenizers, include stemming effect analysis (whether stemmed differs from exact)
  - [x] 1.4 In `debug_query()`, parse the query with `QueryParser` for both `content` and `content_exact` fields, format the parsed query ASTs using `{:#?}` debug representation
  - [x] 1.5 In `debug_query()`, include total docs count per searched index via `reader.searcher().num_docs()`
  - [x] 1.6 Handle `parse_query()` failures gracefully: catch the error, include it in the output string, but still return partial results (tokens, doc count) — do not short-circuit
  - [x] 1.7 Write unit tests for `tokenize_to_string()` and `debug_query()` (including a test with an invalid query to verify partial results on parse error)

- [x] 2.0 Add `debug_query` bridge method and signal to `SuttaBridge`
  - [x] 2.1 Add a `#[qsignal] fn debug_query_ready(self: Pin<&mut SuttaBridge>, debug_json: QString)` signal definition in the bridge extern block
  - [x] 2.2 Implement `pub fn debug_query(self: Pin<&mut Self>, query: &QString, search_area: &QString, params_json: &QString)` method that spawns a thread, calls the backend's `debug_query()`, and emits `debug_query_ready` with the result via `qt_thread.queue()`
  - [x] 2.3 Include error information in the emitted JSON when the backend returns an error (e.g., `{"error": "...", "debug_text": "..."}` for partial results, or `{"debug_text": "..."}` for success)
  - [x] 2.4 For non-fulltext search modes, generate a parameter summary string in the bridge method (mode name, search area, query text, language filter, source filter, regex, fuzzy distance) and return it as the debug text
  - [x] 2.5 Add the qmllint type definition in `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml`: the `debug_query` function signature and `debugQueryReady` signal

- [x] 3.0 Create `QueryTab.qml` component and integrate into sidebar
  - [x] 3.1 Create `assets/qml/QueryTab.qml` following the `TocTab.qml` pattern: pragma ComponentBehavior, standard imports, required properties (`window_id`, `is_dark`), `ColumnLayout` root
  - [x] 3.2 Add an error banner area at the top: a `Rectangle` with colored background (e.g., red/orange tint) containing a `Text` element for the parse error message, only visible when `error_text` property is non-empty
  - [x] 3.3 Add a `ScrollView` containing a selectable `TextEdit` (or `TextArea`) with monospace font (`font.family: "monospace"`) for the debug output, read-only, respecting `is_dark` for colors
  - [x] 3.4 Add a "Copy" button (e.g., in a toolbar row above the scroll area) that copies the full debug text (error + debug info) to clipboard using an invisible `TextEdit` helper (same pattern as `clip` in `SuttaSearchWindow.qml`)
  - [x] 3.5 Expose properties: `debug_text` (string), `error_text` (string), `has_error` (bool); expose a function `update_debug(debug_text, error_text)` that sets properties without clearing previous debug_text when only error_text changes
  - [x] 3.6 Add an empty state message (e.g., "Type a query to see debug info") shown when `debug_text` is empty
  - [x] 3.7 Register `QueryTab.qml` in the `qml_files` list in `bridges/build.rs`
  - [x] 3.8 In `SuttaSearchWindow.qml`, add a `TabButton` for QueryTab after the TOC tab button with text "Query", icon `icons/32x32/fa_circle-info-solid.png` (used instead of `bx_code_block.png` which doesn't exist), and `id: query_tab_btn`
  - [x] 3.9 In `SuttaSearchWindow.qml`, add the `QueryTab` component in the `StackLayout` after `TocTab`, with `id: query_tab`, passing `window_id` and `is_dark`, with `Layout.fillWidth: true` and `Layout.fillHeight: true`

- [x] 4.0 Wire signal flow: search input → debug_query → QueryTab display
  - [x] 4.1 In `SuttaSearchWindow.qml`, add a `Connections` block for `SuttaBridge.onDebugQueryReady` that parses the JSON result and calls `query_tab.update_debug()` with the debug text and any error text
  - [x] 4.2 In `SuttaSearchWindow.qml`, create a function `trigger_debug_query()` that reads the current search input text, search area, and params, then calls `SuttaBridge.debug_query()`
  - [x] 4.3 Add a separate debounce `Timer` for debug queries (e.g., `id: debug_query_timer`, interval matching search timer) that calls `trigger_debug_query()` on triggered
  - [x] 4.4 Connected in `SuttaSearchWindow.qml` directly via `Connections` on `search_bar_input.search_input` to restart the debug timer on `onTextChanged`
  - [x] 4.5 Ensure the debug query is also triggered when a search is explicitly executed (Enter key or search button click), not only on typing

- [x] 5.0 Implement query error state and icon changes
  - [x] 5.1 Add `property bool has_query_error: false` to the root of `SuttaSearchWindow.qml`
  - [x] 5.2 In the `onDebugQueryReady` handler, set `has_query_error = true` when the result JSON contains an `"error"` key, and `has_query_error = false` otherwise
  - [x] 5.3 In `SearchBarInput.qml`, add `required property bool has_query_error` and update the `search_btn` icon binding to three states (error → warning icon, loading → stopwatch, default → search)
  - [x] 5.4 Pass `has_query_error` from `SuttaSearchWindow.qml` to the `SearchBarInput` component instance
  - [x] 5.5 Update the QueryTab `TabButton` icon binding to show warning icon on error
  - [x] 5.6 Clear `has_query_error` when the user modifies query text via `Connections` on `search_bar_input.search_input` `onTextChanged`

- [x] 6.0 Preserve results and debug info on parse errors
  - [x] 6.1 In the `onResultsPageReady` handler in `SuttaSearchWindow.qml`, when the result JSON contains an `"error"` key, do NOT call `fulltext_results.set_search_result_page()` — skip the results update and only update the error state
  - [x] 6.2 In `QueryTab.update_debug()`, when only `error_text` is provided (non-empty) and `debug_text` is empty or unchanged, keep the existing `debug_text` and only update the error banner
  - [x] 6.3 Verify that typing a valid query after an error clears the error banner and updates both debug info and results normally (implemented via `onTextChanged` clearing `has_query_error` and restarting debug timer)
  - [x] 6.4 Build and verify compilation with `make build -B`
