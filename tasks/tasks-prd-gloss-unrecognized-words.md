# Tasks: Unrecognized Words Display in GlossTab

## Relevant Files

- `assets/qml/GlossTab.qml` - Modified to add unrecognized words collection and display functionality
- `assets/qml/UnrecognizedWordsList.qml` - New reusable component for displaying clickable unrecognized words with Button styling
- `assets/qml/SuttaSearchWindow.qml` - Modified to connect GlossTab signals to WordSummary lookup
- `assets/qml/WordSummary.qml` - Component that receives lookup requests via signal connections
- `bridges/build.rs` - Modified to register new UnrecognizedWordsList QML component
- `assets/qml/tst_GlossTab.qml` - Existing unit tests (no modifications needed - all pass)
- `assets/qml/tst_UnrecognizedWordsList.qml` - Unit tests for new component (not yet created)

### Notes

- Unit tests should be placed alongside the QML files they are testing in the `assets/qml/` directory
- Use `make qml-test` to run QML tests with offscreen platform
- New QML components must be registered in `bridges/build.rs` qml_files list

## Tasks

- [x] 1.0 Add data collection for unrecognized words during DPD lookup
  - [x] 1.1 Add properties to GlossTab for storing unrecognized word collections (`global_unrecognized_words: []`, `paragraph_unrecognized_words: {}`)
  - [x] 1.2 Modify `process_word_for_glossing()` function to track words where DPD lookup returns empty results
  - [x] 1.3 Create helper function `collect_unrecognized_words()` to aggregate failed lookups per paragraph and globally
  - [x] 1.4 Update `update_all_glosses()` and `update_gloss()` functions to collect and store unrecognized words
- [x] 2.0 Create reusable UnrecognizedWordsList component with Button styling  
  - [x] 2.1 Create new QML file `assets/qml/UnrecognizedWordsList.qml` with required properties (word_list, prefix_text)
  - [x] 2.2 Implement horizontal Flow layout for Button elements that wrap to available width
  - [x] 2.3 Style buttons with `bg_color_darker` background, rounded corners, and hover effects (`bg_color_lighter`)
  - [x] 2.4 Add signal `wordClicked(string word)` for external handling of word clicks
  - [x] 2.5 Register new component in `bridges/build.rs` qml_files list
- [x] 3.0 Integrate unrecognized words display in global location
  - [x] 3.1 Add UnrecognizedWordsList component under `main_gloss_input_group` after the button row
  - [x] 3.2 Bind component's `word_list` property to `global_unrecognized_words`
  - [x] 3.3 Set prefix_text to "Click for deconstructor lookup:"
  - [x] 3.4 Implement visibility logic to hide when `global_unrecognized_words` is empty
  - [x] 3.5 Connect `wordClicked` signal to WordSummary lookup functionality
- [x] 4.0 Integrate unrecognized words display per-paragraph
  - [x] 4.1 Modify paragraph delegate component to include UnrecognizedWordsList above "Dictionary definitions from DPD:" text
  - [x] 4.2 Bind word_list to paragraph-specific unrecognized words from `paragraph_unrecognized_words[paragraph_index]`
  - [x] 4.3 Implement same styling and signal connections as global component
  - [x] 4.4 Add visibility logic for per-paragraph lists
- [x] 5.0 Implement WordSummary lookup integration via signals
  - [x] 5.1 Add signal `requestWordSummary(string word)` to GlossTab root
  - [x] 5.2 Connect UnrecognizedWordsList `wordClicked` signals to `requestWordSummary`
  - [x] 5.3 Connect GlossTab's `requestWordSummary` to SuttaSearchWindow's `set_summary_query()` function
  - [x] 5.4 Ensure proper signal propagation through parent window hierarchy
- [ ] 6.0 Add performance optimization and testing
  - [x] 6.1 Implement 20-word limit with "and X more..." display in UnrecognizedWordsList component
  - [x] 6.2 Add proper cleanup of unrecognized words collections when text changes
  - [ ] 6.3 Create unit tests in `tst_UnrecognizedWordsList.qml` for component functionality
  - [ ] 6.4 Add test cases to `tst_GlossTab.qml` for unrecognized words collection and display
  - [x] 6.5 Test integration with large texts and verify performance remains acceptable
