# PRD: QueryTab — Query Debug & Pipeline Inspector

## 1. Introduction/Overview

Add a **QueryTab** sidebar tab to `SuttaSearchWindow.qml` that displays debug information about the current search query. This tab shows how the query is tokenized, parsed, and executed by the Tantivy fulltext engine, making the search pipeline transparent to developers and power users.

When a query has a parse error, the tab displays the error prominently, the tab icon changes to a warning indicator, and the search button shows a warning icon — all without clearing existing search results or previous debug info.

This tab will later serve as the foundation for a composable query pipeline UI.

## 2. Goals

- Provide real-time debug visibility into the fulltext search query pipeline.
- Display tokenization (stemmed and normalized), parsed query ASTs, and index stats.
- Surface query parse errors without disrupting existing search results.
- Lay groundwork for a future composable query pipeline UI.

## 3. User Stories

- As a developer, I want to see how my query text is tokenized (stemmed vs. exact) so I can understand why certain results appear or don't.
- As a developer, I want to see the parsed Tantivy query structure so I can debug complex boolean queries.
- As a user, I want to know when my query has a syntax error without losing my current search results.
- As a developer, I want to see total document counts per index so I can verify index health.

## 4. Functional Requirements

### 4.1 QueryTab QML Component

1. Create `QueryTab.qml` as a new sidebar tab component following the pattern established by `TocTab.qml`.
2. Add `QueryTab` as the last tab (index 5) in the `rightside_tabs` TabBar in `SuttaSearchWindow.qml`, after the TOC tab.
3. Use icon `icons/32x32/bx_code_block.png` for the tab button in its normal state.
4. Register `QueryTab.qml` in the `qml_files` list in `bridges/build.rs`.

### 4.2 Debug Info Content

5. The debug info must include, adapted from `pali-stemmer-in-snowball/pali-search/src/searcher.rs::debug_query()`:
   - **Stemmed tokens**: Output of the `{lang}_stem` tokenizer applied to the query text.
   - **Exact (normalized) tokens**: Output of the `{lang}_normalize` tokenizer applied to the query text.
   - **Stemming effect analysis**: Whether stemming changed the tokens (i.e., stemmed differs from exact).
   - **Parsed stemmed query AST**: Debug representation of the Tantivy query on the `content` field.
   - **Parsed exact boost query AST**: Debug representation of the Tantivy query on the `content_exact` field (with 2.0x boost noted).
   - **Total docs in index**: Number of documents in each searched index.

### 4.3 Backend `debug_query()` Method

6. Implement a `debug_query()` method in `backend/src/search/searcher.rs` on `FulltextSearcher` that accepts the query text and search filters, and returns a formatted debug string containing all items from requirement 5.
7. Implement a `tokenize_to_string()` helper (in `backend/src/search/tokenizer.rs` or on `FulltextSearcher`) that runs a given tokenizer pipeline on input text and returns the resulting tokens as a comma-separated string.
8. When `parse_query()` fails, `debug_query()` must still return partial results (tokens, doc count) along with the error message — it must not short-circuit on parse failure.

### 4.4 Bridge Integration

9. Add a `debug_query` method to `SuttaBridge` in `bridges/src/sutta_bridge.rs` that calls the backend's `debug_query()` and emits the result via a new signal (e.g., `debug_query_ready(QString)`).
10. Add the corresponding qmllint type definition in `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml`.

### 4.5 Signal Flow & Update Timing

11. When the user types into `search_input` in `SearchBarInput.qml`, the debug query must be updated on every keystroke (debounced via the existing search timer mechanism, same as search-as-you-type).
12. The `SuttaSearchWindow.qml` must connect the text-changed signal to trigger `SuttaBridge.debug_query()` and route the result to `QueryTab`.
13. The QueryTab must update its display when it receives new debug info via the signal.

### 4.6 Query Parse Error Handling

14. When a query parse error occurs (from `results_page` or `debug_query`), existing search results in `FulltextResults` must NOT be cleared.
15. When a query parse error occurs, existing debug info in `QueryTab` must NOT be cleared. Instead, the error message must be displayed **above** the existing debug info.
16. The error display area should be visually distinct (e.g., colored background or border) so it stands out from the debug info.

### 4.7 Search Button Icon States

17. In `SearchBarInput.qml`, the `search_btn` icon must support three states:
    - **Idle**: `icons/32x32/bx_search_alt_2.png` (existing behavior)
    - **Loading**: `icons/32x32/fa_stopwatch-solid.png` (existing behavior)
    - **Query error**: `icons/32x32/fa_triangle-exclamation-solid.png` (new)
18. A new property (e.g., `has_query_error: bool`) must be added to `SearchBarInput` (or the root window) to track error state.
19. The error icon state must be cleared when the user modifies the query text.

### 4.8 QueryTab Button Icon States

20. The QueryTab's `TabButton` icon must change to `icons/32x32/fa_triangle-exclamation-solid.png` when there is an active query parse error.
21. The tab icon must revert to `icons/32x32/bx_code_block.png` when the error is cleared (user modifies query text).

### 4.9 Non-Fulltext Search Mode Summary

22. For non-fulltext search modes (ContainsMatch, TitleMatch, HeadwordMatch, etc.), the QueryTab must display a simple parameter summary including:
    - Current search mode name
    - Search area (Suttas, Dictionary, Library)
    - Query text
    - Language filter (if set)
    - Source filter (if set)
    - Regex enabled (if applicable)
    - Fuzzy distance (if applicable)
23. The full tokenization/AST debug info (requirements 5–8) only applies to FulltextMatch mode. Other modes show only the parameter summary.

### 4.10 Copy to Clipboard

24. The debug text in `QueryTab` must be selectable.
25. A "Copy" button must be provided that copies the full debug output (including any error message) to the system clipboard.
26. Use an invisible `TextEdit` helper for clipboard operations, following the same pattern as `clipboard_helper` used elsewhere in the UI (e.g., the `clip` TextEdit in `SuttaSearchWindow.qml`).

## 5. Non-Goals (Out of Scope)

- Composable query pipeline UI (future work — this PRD only lays the foundation).
- Persisting debug info across sessions.
- Exposing debug info to end users as a "feature" — this is a developer/power-user tool.

## 6. Design Considerations

- **Layout**: `QueryTab` should use a `ColumnLayout` with a scrollable `ScrollView` or `Flickable` containing the debug text, since query ASTs can be long.
- **Error banner**: The parse error (when present) should appear as a highlighted banner at the top of the tab, above the scrollable debug content.
- **Text display**: Use a monospace font (`font.family: "monospace"`) for the debug output, similar to a terminal/log view, since it contains structured text (token lists, AST dumps).
- **Dark mode**: The component must respect the `is_dark` property passed from the parent, consistent with other tabs.

## 7. Technical Considerations

- **Tokenizer access**: The `tokenize_to_string()` helper needs access to registered tokenizers on a Tantivy `Index`. Since tokenizers are registered per-index, the helper should accept an `&Index` parameter or be a method on `FulltextSearcher` which already holds index references.
- **Multi-language indexes**: `FulltextSearcher` holds a `HashMap<String, (Index, IndexReader)>` for multiple languages. `debug_query()` should show debug info for the language(s) that would actually be searched given the current filter settings (respecting the language filter dropdown).
- **Thread safety**: `debug_query()` should run in a spawned thread (same pattern as `results_page()`) to avoid blocking the UI, and emit results via a Qt signal.
- **Debouncing**: Reuse the existing `search_timer` debounce mechanism in `SearchBarInput.qml` rather than adding a separate timer.
- **Error state propagation**: The `has_query_error` state needs to flow from the bridge signal handler in `SuttaSearchWindow.qml` down to both `SearchBarInput` (for the search button icon) and the `QueryTab` button (for the tab icon).

## 8. Success Metrics

- Debug info updates within the debounce interval when typing in the search input.
- Query parse errors are displayed without clearing results or previous debug info.
- All three icon states (idle, loading, error) render correctly on the search button.
- Tab icon correctly reflects error state.
- Debug output matches the information shown by `pali-stemmer-in-snowball`'s `debug_query()`, adapted for multi-language indexes.

## 9. Open Questions

- Should the debug info show data for all language indexes or only the one matching the current language filter? (Current assumption: only filtered languages, matching `search_indexes()` behavior.)
- What monospace font is available across all target platforms (Linux, macOS, Android)? Qt's generic `"monospace"` family should work but may need verification on Android.
