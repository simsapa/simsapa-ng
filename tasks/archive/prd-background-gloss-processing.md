# PRD: Background Gloss Processing

## Overview

Implement background thread processing for gloss operations in GlossTab.qml to prevent UI freezing during word lookup operations. Currently, the `update_all_glosses()` and `update_paragraph_gloss()` functions run synchronously in the main thread, causing the UI to freeze while processing large texts with many words.

## Problem Statement

### Current Issues
- UI freezes during gloss processing due to synchronous word lookups
- The processing loops in `populate_paragraph_words_data()` and `process_paragraph_for_glossing()` block the main thread
- Each call to `SuttaBridge.dpd_lookup_json()` is synchronous and can be slow
- User cannot interact with the application while processing

### Impact
- Poor user experience, especially with longer texts
- Application appears unresponsive during processing
- No indication of processing progress beyond the operation starting

## Solution Requirements

### Functional Requirements

1. **Background Processing**
   - Move word processing loops to background threads using Rust bridge
   - Follow the same pattern as `SuttaBridge.results_page()` with `thread::spawn()` and signal emission
   - Maintain identical processing logic and results

2. **UI Responsiveness**
   - Keep UI interactive during background processing
   - Show stopwatch icon on processing buttons (`update_all_glosses_btn`, `update_gloss_btn`)
   - Disable processing buttons during operation to prevent concurrent processing

3. **Data Integrity**
   - Preserve all current functionality including:
     - Unrecognized word tracking (global and per-paragraph)
     - Common word filtering
     - Global deduplication logic
     - Stem tracking and filtering
   - Return identical data structures as current implementation

4. **Error Handling**
   - Handle word lookup failures gracefully
   - Maintain current error reporting mechanisms
   - Continue processing other words if individual lookups fail

### Technical Requirements

#### New Rust Bridge Functions
Create new background processing functions in `SuttaBridge`:

1. **`process_all_paragraphs_background()`**
   - Input: paragraphs text, processing options JSON
   - Output: Signal `allParagraphsGlossReady(results_json)`
   - Replaces: `update_all_glosses()` processing loop

2. **`process_paragraph_background()`**
   - Input: paragraph text, paragraph index, processing options JSON
   - Output: Signal `paragraphGlossReady(paragraph_index, results_json)`
   - Replaces: `update_paragraph_gloss()` processing loop

#### Input Data Structure
```json
{
  "paragraphs": ["paragraph1 text", "paragraph2 text"],
  "options": {
    "no_duplicates_globally": true,
    "skip_common": true,
    "common_words": ["ti", "ca", "vā"],
    "existing_global_stems": {},
    "existing_paragraph_unrecognized": {},
    "existing_global_unrecognized": []
  }
}
```

#### Output Data Structure
```json
{
  "success": true,
  "paragraphs": [
    {
      "paragraph_index": 0,
      "words_data": [
        {
          "original_word": "dhamma",
          "results": [...],
          "selected_index": 0,
          "stem": "dhamma",
          "example_sentence": ""
        }
      ],
      "unrecognized_words": ["word1", "word2"]
    }
  ],
  "global_unrecognized_words": ["word1", "word2", "word3"],
  "updated_global_stems": {"dhamma": true, "sutta": true}
}
```

#### Processing Logic Transfer
Move these functions to Rust backend:
- `extract_words()` ✓ (already exists)
- `process_word_for_glossing()` logic
- `clean_stem()` logic
- `is_common_word()` logic
- Unrecognized word collection logic
- Stem deduplication logic

#### Signal Handling
Add signal connections in QML:
```qml
Connections {
    target: SuttaBridge
    
    function onAllParagraphsGlossReady(results_json) {
        // Update paragraph_model, reset button states
    }
    
    function onParagraphGlossReady(paragraph_index, results_json) {
        // Update specific paragraph, reset button state
    }
}
```

#### UI State Management
1. **Button States**
   - Show stopwatch icon while processing
   - Disable button during processing
   - Re-enable button on completion/error

2. **Progress Indication**
   - Use existing stopwatch icon pattern from `SearchBarInput.qml`
   - No additional progress indicators required

### QML Testing Requirements

Create comprehensive QML tests with mock data:

1. **Test File**: `tst_GlossTabBackgroundProcessing.qml`
2. **Test Cases**:
   - Background processing with mock word lookup responses
   - Unrecognized word collection
   - Global stem deduplication
   - Common word filtering
   - Button state changes during processing
   - Signal emission and handling
   - Error handling scenarios

3. **Mock Data**:
   - Predefined DPD lookup responses
   - Various paragraph texts with known/unknown words
   - Different processing options combinations

## Implementation Plan

### Phase 1: Rust Backend Implementation
1. Add new background processing functions to `sutta_bridge.rs`
2. Implement word processing logic in Rust
3. Add signal definitions for completion events
4. Add comprehensive error handling

### Phase 2: QML Integration
1. Add signal connections in `GlossTab.qml`
2. Modify button click handlers to call background functions
3. Implement button state management (stopwatch icon, disable/enable)
4. Update result processing to handle background responses

### Phase 3: Testing
1. Create QML tests with mock data
2. Test all processing scenarios
3. Verify UI state management
4. Test error handling

### Phase 4: Validation
1. Compare results with current synchronous implementation
2. Performance testing with large texts
3. UI responsiveness validation

## Acceptance Criteria

### Functional Criteria
- [ ] UI remains interactive during gloss processing
- [ ] Processing buttons show stopwatch icon when active
- [ ] Processing buttons are disabled during operation
- [ ] All current gloss functionality preserved
- [ ] Unrecognized word tracking works identically
- [ ] Global deduplication logic preserved
- [ ] Common word filtering maintained
- [ ] Session saving/loading unaffected

### Technical Criteria
- [ ] Background processing uses thread::spawn() pattern
- [ ] Signals emitted for completion events
- [ ] Identical data structures returned
- [ ] Error handling maintains current behavior
- [ ] QML tests cover all processing scenarios
- [ ] No regression in processing accuracy

### Performance Criteria
- [ ] UI responsiveness maintained during processing
- [ ] Processing time comparable to current implementation
- [ ] Memory usage reasonable for large texts
- [ ] No hanging threads or resource leaks

## Risk Assessment

### Low Risk
- **UI freezing eliminated**: Background threading is proven pattern in codebase
- **Data consistency**: Identical processing logic ported to Rust
- **Testing coverage**: Comprehensive QML tests with mock data

### Medium Risk
- **Complex state management**: Managing multiple processing states requires careful implementation
- **Signal timing**: Ensuring proper signal emission and handling sequence

### Mitigation Strategies
- Follow established `SuttaBridge.results_page()` pattern exactly
- Implement comprehensive logging for debugging
- Create extensive test coverage before implementation
- Preserve existing synchronous functions as reference during development

## Dependencies

### Internal Dependencies
- `SuttaBridge` rust bridge implementation
- `dpd_lookup_json()` function
- `extract_words()` function
- Existing QML signal/slot mechanisms

### External Dependencies
- None (uses existing DPD database and processing)

## Success Metrics

1. **User Experience**: No UI freezing during gloss operations
2. **Functionality**: 100% feature parity with current implementation
3. **Performance**: Processing time within 10% of current implementation
4. **Reliability**: Zero crashes or hangs during processing
5. **Testing**: 100% test pass rate with comprehensive coverage

This implementation will significantly improve user experience while maintaining all existing functionality and data accuracy.
