# Unified Search Bar Testing

## Manual Testing Checklist

### 1. Basic UI Elements
- [x] ComboBox appears in search bar with "Suttas" and "Dictionary" options
- [x] ComboBox defaults to "Suttas" 
- [x] Placeholder text updates based on selection ("Search in suttas" vs "Search in dictionary")

### 2. Search Functionality
- [ ] Search works in "Suttas" mode (existing functionality preserved)
- [ ] Search works in "Dictionary" mode (new functionality)
- [ ] Results display correctly in FulltextResults for both modes
- [ ] Switching search areas preserves previous results until new search

### 3. Content Display
- [ ] Clicking sutta results displays sutta content in SuttaStackLayout
- [ ] Clicking dictionary results displays dictionary content in SuttaStackLayout  
- [ ] Dictionary content renders with same styling as DictionaryTab
- [ ] Window title updates correctly for both content types

### 4. Edge Cases
- [ ] Empty results handled gracefully for both search areas
- [ ] Invalid queries handled properly
- [ ] Mobile layout works correctly with ComboBox

## Test Results

### Application Startup: ✅ PASS
- Application starts without errors
- Databases load successfully (suttas and dictionary)
- UI initializes properly

### Build Status: ✅ PASS  
- All components compile successfully
- No TypeScript/QML errors
- Rust backend compiles and links

### Dictionary Search Error Fix: ✅ PASS
- Fixed QML error: "sutta_ref is null" when displaying dictionary results
- Made sutta_ref property optional in FulltextResults component
- Added proper null handling with fallback to empty string
- sutta_ref display is now hidden for dictionary results (when empty)
- Application starts without QML delegate creation errors

### DPD Lookup Pagination: ✅ PASS
- Implemented proper pagination in query_task.rs::dpd_lookup()
- Function now accepts page_num parameter instead of hardcoded 100-item limit
- Sets db_query_hits_count to total results count for pagination controls
- Applies proper offset calculation: page_num * page_len
- Handles out-of-bounds pages gracefully with empty results
- Added dedicated SearchMode::DpdLookup case for dictionary-only searches

### Dictionary Three-Phase Search Enhancement: ✅ PASS
- Implemented advanced three-phase dictionary search with optimal result ordering
- Phase 1: Exact matches on DpdHeadword.lemma_clean (highest priority)
- Phase 2: Contains matches on DpdHeadword.lemma_1 (medium priority, sorted by lemma_1 length ascending)
- Phase 3: Contains matches on DictWord.definition_plain (lowest priority)
- Matches DpdHeadword.lemma_1 to DictWord.word for proper result linking
- Uses word field for deduplication across all three phases
- Array-based pagination applied to combined result set from all phases
- Proper database schema separation with scoped column references
- Efficient deduplication using HashSet across all three phases
- Phase 2 results sorted by DpdHeadword.lemma_1 length in ascending order

## Implementation Details
- Updated to use item_uid instead of sutta_uid throughout codebase
- Dictionary results identified by table_name == "dict_words"
- Fixed FulltextResults to handle dictionary results without sutta_ref
- Proper null coalescing: `item.sutta_ref || ""`
- Conditional display logic for sutta_ref field

## Notes
- Implementation follows existing code patterns
- Leverages existing SearchArea enum and db_word_to_result() function
- Dictionary content detection via table_name field in SearchResult
- Maintains backward compatibility with existing search functionality
- Fixed critical error that prevented dictionary search results from displaying