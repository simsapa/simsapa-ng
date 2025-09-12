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

## Notes
- Implementation follows existing code patterns
- Leverages existing SearchArea enum and db_word_to_result() function
- Dictionary content detection via table_name field in SearchResult
- Maintains backward compatibility with existing search functionality