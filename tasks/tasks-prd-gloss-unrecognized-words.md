# Tasks: Unrecognized Words Display in GlossTab

## Relevant Files

- `assets/qml/GlossTab.qml` - Main component that needs modification for unrecognized words display
- `assets/qml/UnrecognizedWordsList.qml` - New reusable component for displaying clickable unrecognized words (to be created)
- `assets/qml/SuttaSearchWindow.qml` - Target window for WordSummary lookup integration
- `assets/qml/WordSummary.qml` - Component that will receive the lookup requests
- `bridges/build.rs` - Needs to register new QML component file
- `assets/qml/tst_GlossTab.qml` - Unit tests for GlossTab functionality
- `assets/qml/tst_UnrecognizedWordsList.qml` - Unit tests for new component (to be created)

### Notes

- Unit tests should be placed alongside the QML files they are testing in the `assets/qml/` directory
- Use `make qml-test` to run QML tests with offscreen platform
- New QML components must be registered in `bridges/build.rs` qml_files list

## Tasks

- [ ] 1.0 Add data collection for unrecognized words during DPD lookup
  - [ ] 1.1 Add properties to GlossTab for storing unrecognized word collections (`global_unrecognized_words: []`, `paragraph_unrecognized_words: {}`)
  - [ ] 1.2 Modify `process_word_for_glossing()` function to track words where DPD lookup returns empty results
  - [ ] 1.3 Create helper function `collect_unrecognized_words()` to aggregate failed lookups per paragraph and globally
  - [ ] 1.4 Update `update_all_glosses()` and `update_gloss()` functions to collect and store unrecognized words
- [ ] 2.0 Create reusable UnrecognizedWordsList component with Button styling  
  - [ ] 2.1 Create new QML file `assets/qml/UnrecognizedWordsList.qml` with required properties (word_list, prefix_text)
  - [ ] 2.2 Implement horizontal Flow layout for Button elements that wrap to available width
  - [ ] 2.3 Style buttons with `bg_color_darker` background, rounded corners, and hover effects (`bg_color_lighter`)
  - [ ] 2.4 Add signal `wordClicked(string word)` for external handling of word clicks
  - [ ] 2.5 Register new component in `bridges/build.rs` qml_files list
- [ ] 3.0 Integrate unrecognized words display in global location
  - [ ] 3.1 Add UnrecognizedWordsList component under `main_gloss_input_group` after the button row
  - [ ] 3.2 Bind component's `word_list` property to `global_unrecognized_words`
  - [ ] 3.3 Set prefix_text to "Click for deconstructor lookup:"
  - [ ] 3.4 Implement visibility logic to hide when `global_unrecognized_words` is empty
  - [ ] 3.5 Connect `wordClicked` signal to WordSummary lookup functionality
- [ ] 4.0 Integrate unrecognized words display per-paragraph
  - [ ] 4.1 Modify paragraph delegate component to include UnrecognizedWordsList above "Dictionary definitions from DPD:" text
  - [ ] 4.2 Bind word_list to paragraph-specific unrecognized words from `paragraph_unrecognized_words[paragraph_index]`
  - [ ] 4.3 Implement same styling and signal connections as global component
  - [ ] 4.4 Add visibility logic for per-paragraph lists
- [ ] 5.0 Implement WordSummary lookup integration via signals
  - [ ] 5.1 Add signal `requestWordSummary(string word)` to GlossTab root
  - [ ] 5.2 Connect UnrecognizedWordsList `wordClicked` signals to `requestWordSummary`
  - [ ] 5.3 Connect GlossTab's `requestWordSummary` to SuttaSearchWindow's `set_summary_query()` function
  - [ ] 5.4 Ensure proper signal propagation through parent window hierarchy
- [ ] 6.0 Add performance optimization and testing
  - [ ] 6.1 Implement 20-word limit with "and X more..." display in UnrecognizedWordsList component
  - [ ] 6.2 Add proper cleanup of unrecognized words collections when text changes
  - [ ] 6.3 Create unit tests in `tst_UnrecognizedWordsList.qml` for component functionality
  - [ ] 6.4 Add test cases to `tst_GlossTab.qml` for unrecognized words collection and display
  - [ ] 6.5 Test integration with large texts and verify performance remains acceptable
