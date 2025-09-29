# PRD: Unrecognized Words Display in GlossTab

## Overview
Add functionality to display unrecognized words (words that don't return results from SuttaBridge.dpd_lookup_json()) in the GlossTab component, allowing users to easily identify and look up words that aren't found in the DPD dictionary.

## Problem Statement
Currently when users gloss Pāli text, words that aren't found in the DPD dictionary fail silently - users can only see this in console logs. This makes it difficult to identify which words need manual lookup or alternative dictionary searches.

## Success Criteria
- Users can immediately see which words weren't found during glossing
- Users can click unrecognized words to look them up in WordSummary
- Unrecognized words are displayed at both global and per-paragraph levels
- Performance remains good even with large texts containing many unrecognized words

## User Stories

### Primary User Story
**As a** Pāli text reader  
**I want to** see which words weren't found during glossing  
**So that** I can manually look them up or know which words may need alternative dictionaries

### Secondary User Stories
- **As a user**, I want to click on unrecognized words to search them in WordSummary
- **As a user**, I want to see unrecognized words for the entire text and per paragraph
- **As a user**, I don't want the interface cluttered when all words are recognized

## Functional Requirements

### FR1: Global Unrecognized Words List
- Display list of unrecognized words for entire `gloss_text_input` under `main_gloss_input_group`
- Prefix list with message: "Click for deconstructor lookup:"
- Words in Button elements in a horizontal list, wrapping to the width of available space, the Button elements having flat background color `bg_color_darker` with slighly rounded corners
- List updates after clicking `update_all_glosses_btn` or `update_gloss_btn`
- Hide element list when no unrecognized words exist

### FR2: Per-Paragraph Unrecognized Words List
- Display unrecognized words for each glossed paragraph above "Dictionary definitions from DPD:" text
- Prefix list with message: "Click for deconstructor lookup:"
- Same styling and interaction as global list
- Updates with paragraph-specific glossing actions
- Hide element when paragraph has no unrecognized words

### FR3: Word Interaction
- Clicking on any unrecognized word opens SuttaSearchWindow with WordSummary lookup
- Pass clicked word as search term to WordSummary functionality

### FR4: Performance Optimization
- Limit display to first 20 unrecognized words with "and X more..." suffix when count exceeds limit
- Maintain responsive UI performance with large texts

### FR5: Data Collection
- Collect unrecognized words during DPD lookup process
- Track words where `dpd_lookup_json()` returns empty results
- Maintain separate collections for global and per-paragraph tracking

## Technical Requirements

### TR1: Data Structure
- Add properties to GlossTab for storing unrecognized words collections
- `property var global_unrecognized_words: []`
- `property var paragraph_unrecognized_words: {}`

### TR2: Backend Integration
- Unrecognized words are those for which DPD lookup logic returns no results

### TR3: UI Components
- Create reusable component for displaying clickable unrecognized words lists
- Add click handlers for WordSummary integration
- Use signals to send words to WordSummary lookup

### TR4: Integration Points
- Connect with existing `update_all_glosses_btn` and `update_gloss_btn` functionality
- Integrate with SuttaSearchWindow WordSummary lookup feature using signals

## UI/UX Requirements

### Visual Design
- **Styling and Layout**: Words in Button elements in a horizontal list, wrapping to the width of available space, the Button elements having flat background color `bg_color_darker` with slighly rounded corners
- **Visibility**: Show only when unrecognized words exist
- **Performance**: Limit display to 20 words + count indicator

### Interaction Design
- **Click behavior**: Single click opens WordSummary lookup
- **Visual feedback**: When the Button is hovered, it should have `bg_color_lighter` color, clicking should show the button being pressed

## Technical Implementation Notes

### File Locations
- **Primary component**: `assets/qml/GlossTab.qml`
- **Backend logic**: `bridges/src/sutta_bridge.rs` (if modifications needed)
- **Integration**: SuttaSearchWindow connection for WordSummary

### Key Functions to Modify/Create
- Modify existing gloss update functions to collect unrecognized words
- Create helper function for formatting unrecognized words display
- Add click handlers for WordSummary integration

## Success Metrics
- Users can identify unrecognized words immediately after glossing
- Click-through functionality to WordSummary works reliably
- No performance degradation with large texts (>1000 words)
- UI remains clean when no unrecognized words exist

## Dependencies
- Existing GlossTab functionality
- SuttaBridge DPD lookup system
- SuttaSearchWindow WordSummary feature
- Qt QML Button element capabilities

