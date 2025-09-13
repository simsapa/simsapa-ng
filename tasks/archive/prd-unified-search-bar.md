# Product Requirements Document: Unified Search Bar with Dictionary Support

## Introduction/Overview

Extend the existing search bar functionality to support both Sutta and Dictionary searches through a unified interface. Currently, users must navigate to different parts of the application to search suttas versus dictionary entries. This feature will provide a seamless search experience by allowing users to select their search area (Suttas or Dictionary) directly from the search bar and view results in the same interface.

The feature addresses the workflow needs of Pali language students who frequently look up words while reading suttas, enabling them to quickly switch between searching texts and definitions without losing context.

## Goals

1. **Unified Search Interface**: Provide a single search bar that can search both suttas and dictionary entries
2. **Consistent User Experience**: Display dictionary search results using the same FulltextResults component as sutta results
3. **Seamless Content Display**: Show dictionary content in the main content area (SuttaStackLayout) similar to sutta content
4. **Preserved Workflow**: Maintain existing search functionality while extending capabilities
5. **Reduced Context Switching**: Minimize time users spend navigating between different search interfaces

## User Stories

**As a Pali language student**, I want to search for dictionary definitions directly from the main search bar so that I can quickly look up words without leaving the sutta reading interface.

**As a scholar doing research**, I want to easily switch between searching suttas and dictionary entries so that I can cross-reference texts and definitions efficiently.

**As a general user**, I want the search results to appear in the same familiar format regardless of whether I'm searching suttas or dictionary so that I don't need to learn different interfaces.

**As a user with an existing query**, I want previous results to remain visible when I switch search areas so that I can compare results across different content types.

## Functional Requirements

### Core Search Functionality

1. **Search Area Selection**: The SearchBarInput component must include a ComboBox dropdown with options "Suttas" and "Dictionary"
2. **Dynamic Query Routing**: The system must route search queries to the appropriate backend method based on the selected search area
3. **Default Behavior**: The search area dropdown must default to "Suttas" on application startup
4. **Result Persistence**: When users switch search areas, previous results must remain visible until a new search is initiated

### Backend Integration

5. **Search Area Parameter**: The `results_page()` function in SuttaSearchWindow must accept and pass a `search_area` parameter to the backend
6. **Backend Query Routing**: The `SuttaBridge.results_page()` method must accept a `search_area` parameter and create SearchQueryTask instances with the appropriate SearchArea enum value
7. **Dictionary Search Results**: For dictionary searches, the system must use existing `db_word_to_result()` functionality to convert DictWord objects to SearchResult objects
8. **Result Format Consistency**: Dictionary search results must return SearchResult objects compatible with the FulltextResults display component

### User Interface Requirements

9. **Results Display**: Dictionary search results must be displayed in the FulltextResults component using the same layout and styling as sutta results
10. **Content Rendering**: When users click on dictionary results in FulltextResults, the system must display dictionary HTML content in the SuttaStackLayout component
11. **Rendering Style**: Dictionary content displayed in SuttaStackLayout must use the same rendering approach as the existing DictionaryTab component
12. **Search Placeholder**: The search input placeholder text must update to reflect the selected search area (e.g., "Search in suttas" vs "Search in dictionary")

### Integration Points

13. **SearchBarInput Extension**: The search_area_dropdown state must be accessible to the parent SuttaSearchWindow component
14. **Query Processing**: The handle_query() function must read the search area selection and pass it through the query processing chain
15. **Backend Compatibility**: The SearchQueryTask::new() method must support the SearchArea parameter to determine query processing logic

## Non-Goals (Out of Scope)

1. **Mixed Results Display**: The system will not show both sutta and dictionary results simultaneously in a single result set
2. **Search Area Auto-Detection**: The system will not automatically detect whether a query should search suttas or dictionary based on query content
3. **Cross-Reference Features**: Advanced features like automatic cross-referencing between sutta passages and related dictionary entries are not included
4. **New Dictionary Features**: No new dictionary-specific search features beyond what currently exists in WordSummary/dpd_lookup_json functionality
5. **Search History**: Storing or displaying previous searches across different search areas is not included
6. **Performance Optimization**: Database query optimization specific to unified search is not included in this scope

## Design Considerations

### UI/UX Requirements
- The ComboBox should be positioned to the right of the search button in SearchBarInput
- Dictionary results should maintain the same visual formatting as sutta results (ref, title, snippet)
- Content rendering should be visually consistent between sutta and dictionary displays
- The search area selection should be visually obvious to users

### Technical Considerations
- The SearchArea enum already exists in the backend and should be leveraged
- The existing `db_word_to_result()` function provides the necessary SearchResult conversion
- SuttaStackLayout component should be generalized to handle both sutta and dictionary content
- Adapt SuttaHtmlView to display either suttas (identified with `sutta_uid`) or dictionary words (identified with `word_uid`)

## Success Metrics

1. **Reduced Navigation Time**: Decrease the time users spend switching between sutta and dictionary search interfaces by providing unified access
2. **User Workflow Efficiency**: Enable users to perform dictionary lookups without losing their current sutta reading context
3. **Feature Adoption**: Measure usage of the dictionary search option in the unified search bar
4. **Interface Consistency**: Ensure dictionary results display and content rendering match existing patterns

## Implementation Stages

### Stage 1: Basic Search Area Selection
- Add ComboBox to SearchBarInput
- Update placeholder text based on search area
- Modify handle_query() to read search area selection
- Update results_page() functions to accept search_area parameter
- Route queries to appropriate backend based on selection

### Stage 2: Results Display Integration
- Ensure dictionary results display correctly in FulltextResults
- Implement dictionary content rendering in SuttaStackLayout
- Test result clicking and content display functionality

### Stage 3: Polish and Integration
- Ensure consistent behavior
- Handle edge cases and error conditions
- Verify all existing functionality remains intact
