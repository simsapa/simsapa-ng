# Tasks: Unified Search Bar with Dictionary Support

## Relevant Files

- `assets/qml/SearchBarInput.qml` - Add ComboBox for search area selection and expose state to parent component
- `assets/qml/SuttaSearchWindow.qml` - Update query handling to read search area and pass through pipeline  
- `bridges/src/sutta_bridge.rs` - Modify results_page() to accept search_area parameter and route appropriately
- `backend/src/query_task.rs` - Already supports SearchArea enum, verify dictionary search integration
- `backend/src/types.rs` - Contains SearchArea enum definition (Suttas, Dictionary)
- `assets/qml/FulltextResults.qml` - Verify compatibility with dictionary search results display
- `assets/qml/SuttaStackLayout.qml` - Extend to handle both sutta and dictionary content display
- `assets/qml/SuttaHtmlView.qml` - Add support for rendering dictionary content alongside sutta content
- `assets/qml/DictionaryHtmlView.qml` - Reference implementation for dictionary content rendering
- `bridges/build.rs` - May need updates if new QML files are created

### Notes

- The SearchArea enum already exists in backend/src/types.rs with Suttas and Dictionary variants
- Dictionary search functionality exists via db_word_to_result() in query_task.rs  
- DictionaryHtmlView provides the rendering pattern to follow for dictionary content
- SuttaHtmlView uses a Loader pattern that can be extended to support dictionary content
- No new test files are specified as this feature primarily extends existing UI components

## Tasks

- [ ] 1.0 Extend SearchBarInput UI with Search Area Selection
  - [ ] 1.1 Add ComboBox dropdown with "Suttas" and "Dictionary" options positioned after search button
  - [ ] 1.2 Set default selection to "Suttas" to maintain existing behavior
  - [ ] 1.3 Update placeholder text dynamically based on search area selection ("Search in suttas" vs "Search in dictionary")
  - [ ] 1.4 Expose search_area_dropdown.currentText property to parent SuttaSearchWindow component
  - [ ] 1.5 Ensure ComboBox styling matches existing UI components and works on mobile

- [ ] 2.0 Update Query Processing Pipeline for Search Area Support  
  - [ ] 2.1 Modify handle_query() in SuttaSearchWindow to read search area from SearchBarInput
  - [ ] 2.2 Update start_search_query_workers() to accept and pass search_area parameter
  - [ ] 2.3 Modify results_page() in SuttaSearchWindow to accept search_area parameter and pass to backend
  - [ ] 2.4 Ensure search area state is preserved during query processing but allow result persistence when switching areas
  - [ ] 2.5 Update get_search_params_from_ui() if needed to include search area context

- [ ] 3.0 Extend Backend to Support Dictionary Search via Unified Interface
  - [ ] 3.1 Modify SuttaBridge.results_page() signature to accept search_area parameter from QML
  - [ ] 3.2 Update SearchQueryTask::new() call to use search_area parameter instead of hardcoded SearchArea::Suttas
  - [ ] 3.3 Verify existing db_word_to_result() functionality works correctly for dictionary SearchArea
  - [ ] 3.4 Ensure SearchResult objects from dictionary searches are compatible with FulltextResults display
  - [ ] 3.5 Test that dictionary search results contain proper sutta_uid, sutta_title, sutta_ref fields for FulltextResults

- [ ] 4.0 Adapt Content Display Components for Dictionary Results
  - [ ] 4.1 Analyze how DictionaryHtmlView renders dictionary content for reference implementation
  - [ ] 4.2 Extend SuttaHtmlView to detect dictionary content vs sutta content (via sutta_uid pattern or additional property)
  - [ ] 4.3 Implement dictionary content loading in SuttaHtmlView using DictionaryHtmlView rendering approach
  - [ ] 4.4 Update SuttaStackLayout to handle dictionary content items alongside sutta content items
  - [ ] 4.5 Ensure dictionary content displayed in SuttaStackLayout uses same styling approach as DictionaryTab
  - [ ] 4.6 Verify FulltextResults can properly trigger dictionary content display when dictionary results are clicked

- [ ] 5.0 Integration Testing and Validation
  - [ ] 5.1 Test basic search area switching between Suttas and Dictionary modes
  - [ ] 5.2 Verify dictionary search returns results in FulltextResults with proper formatting
  - [ ] 5.3 Test clicking dictionary results properly displays content in SuttaStackLayout
  - [ ] 5.4 Ensure existing sutta search functionality remains unchanged
  - [ ] 5.5 Test result persistence when switching search areas (previous results remain until new search)
  - [ ] 5.6 Verify mobile compatibility for ComboBox and dictionary content display
  - [ ] 5.7 Test edge cases: empty dictionary results, search area switching during active search
  - [ ] 5.8 Validate that window title updates and tab behavior work correctly with dictionary content