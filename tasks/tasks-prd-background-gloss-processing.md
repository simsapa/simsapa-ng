# Tasks: Background Gloss Processing for Simsapa

## Relevant Files

### Rust Backend Files
- `bridges/src/sutta_bridge.rs` - Main bridge implementation, needs new background processing functions
- `backend/src/lib.rs` - Core backend library, may need utility functions
- `bridges/build.rs` - Build configuration, may need QML type definitions updates

### QML Frontend Files
- `assets/qml/GlossTab.qml` - Main gloss tab UI, needs signal connections and button state management
- `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml` - QML type definition, needs new function signatures
- `assets/qml/com/profoundlabs/simsapa/qmldir` - QML module directory, may need updates

### Test Files
- `assets/qml/tests/tst_GlossTabBackgroundProcessing.qml` - New QML test file for background processing

### Documentation
- `PROJECT_MAP.md` - Project structure documentation, needs updates for new components

## Tasks

### Phase 1: Rust Backend Implementation

- [ ] 1.0 Setup Background Processing Infrastructure
  - [ ] 1.1 Add signal definitions to `sutta_bridge.rs` for `allParagraphsGlossReady` and `paragraphGlossReady`
  - [ ] 1.2 Import required threading dependencies (`std::thread`, signal handling)
  - [ ] 1.3 Create error handling structures for background processing failures
  - [ ] 1.4 Add logging infrastructure for debugging background operations

- [ ] 2.0 Port Word Processing Logic to Rust
  - [ ] 2.1 Implement `clean_stem()` function in Rust (port from QML logic)
  - [ ] 2.2 Implement `is_common_word()` function with common words list checking
  - [ ] 2.3 Create `process_word_for_glossing()` equivalent that calls `dpd_lookup_json()`
  - [ ] 2.4 Implement unrecognized word collection logic
  - [ ] 2.5 Implement global stem deduplication logic

- [ ] 3.0 Implement Background Processing Functions
  - [ ] 3.1 Create `process_all_paragraphs_background()` function with thread spawning
  - [ ] 3.2 Create `process_paragraph_background()` function for single paragraph processing
  - [ ] 3.3 Implement JSON input parsing for processing options and paragraph data
  - [ ] 3.4 Implement JSON output formatting matching the specified data structure
  - [ ] 3.5 Add proper error handling and signal emission for both success and failure cases

- [ ] 4.0 Add Comprehensive Error Handling
  - [ ] 4.1 Handle individual word lookup failures gracefully (continue processing other words)
  - [ ] 4.2 Add timeout handling for long-running operations
  - [ ] 4.3 Implement proper thread cleanup on errors
  - [ ] 4.4 Add error signal emission with detailed error messages
  - [ ] 4.5 Add logging for debugging failed operations

### Phase 2: QML Integration

- [ ] 5.0 Update QML Type Definitions
  - [ ] 5.1 Add new function signatures to `assets/qml/com/profoundlabs/simsapa/SuttaBridge.qml`
  - [ ] 5.2 Update `qmldir` file if needed for new type definitions
  - [ ] 5.3 Update `bridges/build.rs` to include new QML files if created

- [ ] 6.0 Implement Signal Connections in GlossTab
  - [ ] 6.1 Add `Connections` block for `SuttaBridge` signals in `GlossTab.qml`
  - [ ] 6.2 Implement `onAllParagraphsGlossReady` signal handler
  - [ ] 6.3 Implement `onParagraphGlossReady` signal handler
  - [ ] 6.4 Add error signal handlers for background processing failures

- [ ] 7.0 Update Button Click Handlers
  - [ ] 7.1 Modify `update_all_glosses_btn` click handler to call background function
  - [ ] 7.2 Modify `update_gloss_btn` click handler to call background function
  - [ ] 7.3 Prepare input data structure (paragraphs, options) for background functions
  - [ ] 7.4 Remove synchronous processing loops from QML button handlers

- [ ] 8.0 Implement UI State Management
  - [ ] 8.1 Add stopwatch icon display logic for processing buttons
  - [ ] 8.2 Implement button disable/enable logic during processing
  - [ ] 8.3 Add processing state tracking variables
  - [ ] 8.4 Implement button state reset on completion or error
  - [ ] 8.5 Ensure concurrent processing prevention

- [ ] 9.0 Update Result Processing Logic
  - [ ] 9.1 Modify result handling to work with background signal responses
  - [ ] 9.2 Update paragraph model with background processing results
  - [ ] 9.3 Maintain existing unrecognized word tracking integration
  - [ ] 9.4 Preserve session saving/loading functionality

### Phase 3: Testing Implementation

- [ ] 10.0 Create QML Test Infrastructure
  - [ ] 10.1 Create `tst_GlossTabBackgroundProcessing.qml` test file
  - [ ] 10.2 Set up mock data for DPD lookup responses
  - [ ] 10.3 Create test paragraphs with known/unknown words
  - [ ] 10.4 Set up signal spy components for testing background signals

- [ ] 11.0 Implement Core Functionality Tests
  - [ ] 11.1 Test background processing with mock word lookup responses
  - [ ] 11.2 Test unrecognized word collection accuracy
  - [ ] 11.3 Test global stem deduplication logic
  - [ ] 11.4 Test common word filtering functionality
  - [ ] 11.5 Test processing options handling (no_duplicates_globally, skip_common)

- [ ] 12.0 Implement UI State Tests
  - [ ] 12.1 Test button state changes during processing (stopwatch icon, disabled state)
  - [ ] 12.2 Test signal emission and handling sequence
  - [ ] 12.3 Test concurrent processing prevention
  - [ ] 12.4 Test button state reset on completion and error scenarios

- [ ] 13.0 Implement Error Handling Tests
  - [ ] 13.1 Test individual word lookup failure handling
  - [ ] 13.2 Test processing continuation after errors
  - [ ] 13.3 Test error signal emission and handling
  - [ ] 13.4 Test timeout scenarios and cleanup

### Phase 4: Validation and Integration

- [ ] 14.0 Data Integrity Validation
  - [ ] 14.1 Compare background processing results with current synchronous implementation
  - [ ] 14.2 Validate identical data structures are returned
  - [ ] 14.3 Test unrecognized word tracking matches exactly
  - [ ] 14.4 Verify global deduplication produces identical results
  - [ ] 14.5 Confirm common word filtering works identically

- [ ] 15.0 Performance Testing
  - [ ] 15.1 Test UI responsiveness during background processing with large texts
  - [ ] 15.2 Measure processing time comparison with synchronous implementation
  - [ ] 15.3 Test memory usage with large text processing
  - [ ] 15.4 Verify no thread leaks or hanging processes
  - [ ] 15.5 Test processing cancellation and cleanup

- [ ] 16.0 Integration Testing
  - [ ] 16.1 Test integration with existing session saving/loading
  - [ ] 16.2 Test compatibility with all current gloss features
  - [ ] 16.3 Test interaction with other GlossTab functionality
  - [ ] 16.4 Verify no regression in related features

- [ ] 17.0 Final Cleanup and Documentation
  - [ ] 17.1 Remove or comment out old synchronous processing functions
  - [ ] 17.2 Update code comments and documentation
  - [ ] 17.3 Update `PROJECT_MAP.md` with new background processing components
  - [ ] 17.4 Add usage examples and troubleshooting notes

## Build/Test Commands

- **Build Rust Backend:** `cd backend && cargo build`
- **Build Qt Application:** `make build -B`
- **Run QML Tests:** `make qml-test`
- **Run Specific Test:** `cd backend && cargo test test_background_gloss`
- **Check Rust Code:** `cd backend && cargo check`

## Implementation Notes

### Signal Pattern to Follow
Follow the established pattern from `SuttaBridge.results_page()`:
```rust
thread::spawn(move || {
    // Background processing logic
    let result = process_data();
    
    // Emit signal with results
    self_.emit_processing_complete(result);
});
```

### Data Structure Requirements
Input JSON format:
```json
{
  "paragraphs": ["text1", "text2"],
  "options": {
    "no_duplicates_globally": true,
    "skip_common": true,
    "common_words": ["ti", "ca"],
    "existing_global_stems": {},
    "existing_paragraph_unrecognized": {},
    "existing_global_unrecognized": []
  }
}
```

Output JSON format:
```json
{
  "success": true,
  "paragraphs": [{
    "paragraph_index": 0,
    "words_data": [...],
    "unrecognized_words": [...]
  }],
  "global_unrecognized_words": [...],
  "updated_global_stems": {...}
}
```

### UI State Management Pattern
```qml
property bool is_processing: false

Button {
    enabled: !is_processing
    icon.source: is_processing ? "stopwatch-icon" : "normal-icon"
    
    onClicked: {
        is_processing = true
        SuttaBridge.process_all_paragraphs_background(data)
    }
}

Connections {
    target: SuttaBridge
    function onAllParagraphsGlossReady(results) {
        is_processing = false
        // Process results
    }
}
```

## Risk Mitigation

- **State Management Complexity:** Use single boolean flags per button, follow existing patterns
- **Signal Timing Issues:** Implement proper signal-slot connections with error handling
- **Data Integrity:** Comprehensive testing with known datasets and result comparison
- **Performance Regression:** Benchmark against current implementation before integration

## Success Criteria

- [ ] UI remains responsive during all gloss processing operations
- [ ] Processing buttons show appropriate state (stopwatch icon, disabled)
- [ ] All current functionality preserved with identical results
- [ ] No crashes or hangs during background processing
- [ ] Processing time within 10% of current synchronous implementation
- [ ] 100% test pass rate for all new functionality

This comprehensive task breakdown covers all aspects of implementing background gloss processing while maintaining full compatibility with existing functionality.