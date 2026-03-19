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

- [ ] 1.0 Implement `tokenize_to_string()` and `debug_query()` in the Rust backend
  - [ ] 1.1 Add a `tokenize_to_string()` method on `FulltextSearcher` that accepts an `&Index`, a tokenizer name (e.g., `"pli_stem"`), and query text, runs the tokenizer pipeline, and returns tokens as a comma-separated string
  - [ ] 1.2 Add a `debug_query()` method on `FulltextSearcher` that accepts query text and `SearchFilters`, determines which language indexes to search (respecting filters), and returns a formatted `Result<String>`
  - [ ] 1.3 In `debug_query()`, for each relevant language index: call `tokenize_to_string()` for both `{lang}_stem` and `{lang}_normalize` tokenizers, include stemming effect analysis (whether stemmed differs from exact)
  - [ ] 1.4 In `debug_query()`, parse the query with `QueryParser` for both `content` and `content_exact` fields, format the parsed query ASTs using `{:#?}` debug representation
  - [ ] 1.5 In `debug_query()`, include total docs count per searched index via `reader.searcher().num_docs()`
  - [ ] 1.6 Handle `parse_query()` failures gracefully: catch the error, include it in the output string, but still return partial results (tokens, doc count) — do not short-circuit
  - [ ] 1.7 Write unit tests for `tokenize_to_string()` and `debug_query()` (including a test with an invalid query to verify partial results on parse error)

- [ ] 2.0 Add `debug_query` bridge method and signal to `SuttaBridge`
  - [ ] 2.1 Add a `#[qsignal] fn debug_query_ready(self: Pin<&mut SuttaBridge>, debug_json: QString)` signal definition in the bridge extern block
  - [ ] 2.2 Implement `pub fn debug_query(self: Pin<&mut Self>, query: &QString, search_area: &QString, params_json: &QString)` method that spawns a thread, calls the backend's `debug_query()`, and emits `debug_query_ready` with the result via `qt_thread.queue()`
  - [ ] 2.3 Include error information in the emitted JSON when the backend returns an error (e.g., `{"error": "...", "debug_text": "..."}` for partial results, or `{"debug_text": "..."}` for success)
  - [ ] 2.4 For non-fulltext search modes, generate a parameter summary string in the bridge method (mode name, search area, query text, language filter, source filter, regex, fuzzy distance) and return it as the debug text
  - [ ] 2.5 Add the qmllint type definition in `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml`: the `debug_query` function signature and `debugQueryReady` signal

- [ ] 3.0 Create `QueryTab.qml` component and integrate into sidebar
  - [ ] 3.1 Create `assets/qml/QueryTab.qml` following the `TocTab.qml` pattern: pragma ComponentBehavior, standard imports, required properties (`window_id`, `is_dark`), `ColumnLayout` root
  - [ ] 3.2 Add an error banner area at the top: a `Rectangle` with colored background (e.g., red/orange tint) containing a `Text` element for the parse error message, only visible when `error_text` property is non-empty
  - [ ] 3.3 Add a `ScrollView` containing a selectable `TextEdit` (or `TextArea`) with monospace font (`font.family: "monospace"`) for the debug output, read-only, respecting `is_dark` for colors
  - [ ] 3.4 Add a "Copy" button (e.g., in a toolbar row above the scroll area) that copies the full debug text (error + debug info) to clipboard using an invisible `TextEdit` helper (same pattern as `clip` in `SuttaSearchWindow.qml`)
  - [ ] 3.5 Expose properties: `debug_text` (string), `error_text` (string), `has_error` (bool); expose a function `update_debug(debug_text, error_text)` that sets properties without clearing previous debug_text when only error_text changes
  - [ ] 3.6 Add an empty state message (e.g., "Type a query to see debug info") shown when `debug_text` is empty
  - [ ] 3.7 Register `QueryTab.qml` in the `qml_files` list in `bridges/build.rs`
  - [ ] 3.8 In `SuttaSearchWindow.qml`, add a `TabButton` for QueryTab after the TOC tab button with text "Query", icon `icons/32x32/bx_code_block.png`, and `id: query_tab_btn`
  - [ ] 3.9 In `SuttaSearchWindow.qml`, add the `QueryTab` component in the `StackLayout` after `TocTab`, with `id: query_tab`, passing `window_id` and `is_dark`, with `Layout.fillWidth: true` and `Layout.fillHeight: true`

- [ ] 4.0 Wire signal flow: search input → debug_query → QueryTab display
  - [ ] 4.1 In `SuttaSearchWindow.qml`, add a `Connections` block for `SuttaBridge.onDebugQueryReady` that parses the JSON result and calls `query_tab.update_debug()` with the debug text and any error text
  - [ ] 4.2 In `SuttaSearchWindow.qml`, create a function `trigger_debug_query()` that reads the current search input text, search area, and params, then calls `SuttaBridge.debug_query()`
  - [ ] 4.3 Add a separate debounce `Timer` for debug queries (e.g., `id: debug_query_timer`, interval matching search timer) that calls `trigger_debug_query()` on triggered
  - [ ] 4.4 In `SearchBarInput.qml`, emit a signal (e.g., `debug_query_requested()`) on `onTextChanged` alongside the existing `user_typed()` call, or connect in `SuttaSearchWindow.qml` directly to restart the debug timer
  - [ ] 4.5 Ensure the debug query is also triggered when a search is explicitly executed (Enter key or search button click), not only on typing

- [ ] 5.0 Implement query error state and icon changes
  - [ ] 5.1 Add `property bool has_query_error: false` to the root of `SuttaSearchWindow.qml`
  - [ ] 5.2 In the `onDebugQueryReady` handler, set `has_query_error = true` when the result JSON contains an `"error"` key, and `has_query_error = false` otherwise
  - [ ] 5.3 In `SearchBarInput.qml`, add `required property bool has_query_error` and update the `search_btn` icon binding to: `has_query_error ? "icons/32x32/fa_triangle-exclamation-solid.png" : (is_loading ? "icons/32x32/fa_stopwatch-solid.png" : "icons/32x32/bx_search_alt_2.png")`
  - [ ] 5.4 Pass `has_query_error` from `SuttaSearchWindow.qml` to the `SearchBarInput` component instance
  - [ ] 5.5 Update the QueryTab `TabButton` icon binding: `icon.source: root.has_query_error ? "icons/32x32/fa_triangle-exclamation-solid.png" : "icons/32x32/bx_code_block.png"`
  - [ ] 5.6 Clear `has_query_error` when the user modifies query text: in the `onTextChanged` handler or the debug timer restart, set `has_query_error = false`

- [ ] 6.0 Preserve results and debug info on parse errors
  - [ ] 6.1 In the `onResultsPageReady` handler in `SuttaSearchWindow.qml`, when the result JSON contains an `"error"` key, do NOT call `fulltext_results.update_page()` or clear the results list — skip the results update and only update the error state
  - [ ] 6.2 In `QueryTab.update_debug()`, when only `error_text` is provided (non-empty) and `debug_text` is empty or unchanged, keep the existing `debug_text` and only update the error banner
  - [ ] 6.3 Verify that typing a valid query after an error clears the error banner and updates both debug info and results normally
  - [ ] 6.4 Build and verify compilation with `make build -B`
