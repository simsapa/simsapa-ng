# PRD: PTS Reference Search Window

## Introduction/Overview

Users of the Simsapa app often work with traditional sutta reference systems (PTS references, DPR references, titles) but the app's database uses SuttaCentral UIDs (e.g., "sn56.102"). This feature will create a new window that allows users to search for suttas using these traditional reference systems and convert them to SuttaCentral UIDs that can be opened in the app.

**Problem:** Users cannot easily convert between traditional reference systems (PTS, DPR, titles) and the SuttaCentral UIDs used internally by the application.

**Goal:** Provide a dedicated search window where users can look up suttas by PTS reference, DPR reference, or title, and quickly open them or copy their SuttaCentral URLs.

## Goals

1. Enable users to search for suttas using PTS references (e.g., "D ii 20"), DPR references, and titles
2. Replicate the proven search functionality from the JavaScript reference converter in Rust
3. Display search results with clear indication of which results exist in the user's database
4. Provide quick actions to open suttas or copy URLs directly from search results
5. Ensure search accuracy through comprehensive test coverage matching the JavaScript implementation

## User Stories

1. **As a Pali student**, I want to search for "M iii 10" and find the corresponding sutta in my database, so that I can quickly look up references from my study materials.

2. **As a researcher**, I want to search for a sutta by its traditional title and see all matching results, so that I can find suttas when I only remember part of the name.

3. **As a teacher**, I want to copy the SuttaCentral URL of a sutta to share it with students, so that they can access the same text online.

4. **As a practitioner**, I want to see which search results are available in my downloaded database, so that I know which suttas I can read offline.

5. **As a user familiar with DPR references**, I want to search using DPR reference format and find the corresponding suttas, so that I can work with my existing reference materials.

## Functional Requirements

### Backend (Rust) Implementation

1. **FR-1:** Create a new module `backend/src/pts_reference_search.rs` containing all search functions
   
2. **FR-2:** Implement `search_by_text(query: &str, field: &str)` function that:
   - Normalizes search query by converting to lowercase and removing diacritics using `helpers::latinize()`
   - Searches the specified field ('identifier', 'name', 'pts_reference', 'dpr_reference') in the JSON data
   - Returns a vector of matching entries

3. **FR-3:** Implement `parse_pts_reference(pts_ref: &str)` function that:
   - Parses PTS reference strings (e.g., "D ii 20") into components: nikaya, volume, page
   - Returns `Option<PTSReference>` struct with these components
   - Handles invalid input gracefully by returning None

4. **FR-4:** Implement `search_by_pts_reference(query: &str)` function that:
   - Uses `parse_pts_reference()` to parse the search query
   - Implements range-based matching where a page number finds the sutta it falls within
   - For example, searching "D ii 100" should find the sutta that starts before page 100 and continues past it
   - Falls back to text search if parsing fails

5. **FR-5:** Implement `search(query: &str, field: &str)` universal search function that:
   - Detects whether to use PTS reference search or text search based on the field parameter
   - Routes to appropriate search function

6. **FR-6:** Load JSON data from `app_settings::SUTTA_REFERENCE_CONVERTER_JSON` constant

7. **FR-7:** Define return types:
   - `ReferenceSearchResult` struct containing: identifier, name, pts_reference, dpr_reference, url
   - Implements serialization for passing to QML

8. **FR-8:** Create comprehensive Rust tests in `backend/tests/test_pts_reference_search.rs` that:
   - Replicate all test cases from `search_functions.js`
   - Test PTS reference exact matches (e.g., "D i 47" finds DN 2)
   - Test PTS reference range matches (e.g., "D i 50" finds DN 2 which starts at D i 47)
   - Test PTS reference volume boundaries (e.g., "D ii 1" finds DN 14)
   - Test text searches (case-insensitive, diacritic-insensitive)
   - Test DPR reference searches
   - Verify all tests pass before proceeding to UI implementation

### Bridge Implementation

9. **FR-9:** Create or extend a Rust bridge (likely in `sutta_bridge.rs`) to expose search functions to QML:
   - `search_reference(query: String, field: String) -> String` - returns JSON string of results
   - `verify_sutta_uid_exists(uid: String) -> bool` - checks if UID exists in database
   - `extract_uid_from_url(url: String) -> String` - parses UID from SuttaCentral URL

10. **FR-10:** Add corresponding QML type definition functions in the qmllint stub file (e.g., `SuttaBridge.qml`)

### Frontend (QML) Implementation

11. **FR-11:** Create `assets/qml/ReferenceSearchWindow.qml` as a new ApplicationWindow:
    - Window title: "Reference Search"
    - Appropriate size for desktop/mobile
    - Proper theming support using ThemeHelper

12. **FR-12:** Add menu item in SuttaSearchWindow:
    - Menu: Windows > Reference Search
    - Keyboard shortcut (optional): Ctrl+Shift+R
    - Opens ReferenceSearchWindow when clicked

13. **FR-13:** Implement search input controls:
    - Text input field with placeholder "Enter PTS reference, DPR reference, or title..."
    - ComboBox with options: "PTS Ref", "DPR Ref", "Title"
    - Search should trigger in real-time as user types (debounced)

14. **FR-14:** Implement dual result list display:
    
    **JSON Results List:**
    - Shows all matches found in the reference JSON data
    - Displays: identifier, name, pts_reference
    - For entries not in database: show second line with "(Not found in database)" in subdued color
    
    **Database Results List:**
    - Shows only results that exist in the user's database
    - Displays: sutta_ref (formatted reference) and title
    - Each item has two buttons:
      - "Open" - Opens the sutta in SuttaSearchWindow HTML view
      - "Copy URL" - Copies SuttaCentral URL to clipboard

15. **FR-15:** Implement UID extraction and verification:
    - Parse UID from the "url" field (e.g., extract "sn56.102" from "https://suttacentral.net/sn56.102")
    - Call bridge function to verify UID exists in database
    - Update UI accordingly based on verification result

16. **FR-16:** Implement result item actions:
    - "Open" button calls appropriate bridge/window manager function to display sutta
    - "Copy URL" button uses ClipboardManager to copy full SuttaCentral URL

17. **FR-17:** Handle empty states:
    - Show helpful message when no search query entered
    - Show "No results found" when search returns empty
    - Consider showing example queries (e.g., "Try: D ii 20, MN 1, or Brahmajala")

18. **FR-18:** Implement proper error handling:
    - Handle JSON parsing errors gracefully
    - Show error dialog if search fails
    - Handle clipboard operation failures

### Build Configuration

19. **FR-19:** Add new QML files to `bridges/build.rs` qml_files list:
    - `../assets/qml/ReferenceSearchWindow.qml`

20. **FR-20:** If creating new bridge type, register in QmlModule in `bridges/build.rs`

21. **FR-21:** Create qmllint type definition for any new bridges in `assets/qml/com/profoundlabs/simsapa/`

## Non-Goals (Out of Scope)

1. **NG-1:** Automatic downloading or updating of the reference JSON data (uses embedded static data only)

2. **NG-2:** Editing or modifying reference data within the app

3. **NG-3:** Support for other reference systems beyond PTS, DPR, and title (can be added later)

4. **NG-4:** Batch operations (opening multiple suttas at once, bulk copying)

5. **NG-5:** Search history or saved searches

6. **NG-6:** Fuzzy matching or spelling corrections (uses exact substring matching with normalization)

7. **NG-7:** Integration with other windows beyond SuttaSearchWindow (no word lookup integration, etc.)

## Design Considerations

### UI/UX

- **Layout:** Two-column layout on desktop (JSON results | Database results), stacked on mobile
- **Typography:** Use consistent font sizes from root window properties (pointSize)
- **Colors:** Follow system palette, use subdued text color for "(Not found in database)" indicator
- **Responsiveness:** Search should feel instant (debounce ~300ms)
- **Mobile:** Ensure touch-friendly button sizes, proper scrolling behavior

### Search Field Mapping

Based on `search_functions.js`:
- **PTS Ref** → searches 'pts_reference' field using range-based matching
- **DPR Ref** → searches 'dpr_reference' field using text matching
- **Title** → searches both 'identifier' and 'name' fields using text matching

### Reference to Existing Components

- Study `SuttaLanguagesWindow.qml` for window structure and pattern
- Use `ClipboardManager` for copying URLs (see existing usage in codebase)
- Use existing window manager patterns for opening suttas in SuttaSearchWindow
- Reference `DownloadProgressFrame.qml` for two-list layout inspiration

## Technical Considerations

### Data Structure

The JSON data from `sutta-reference-converter.json` contains entries like:
```json
{
  "identifier": "DN 1",
  "name": "Brahmajāla Sutta",
  "pts_reference": "D i 1",
  "dpr_reference": "DN 1",
  "url": "https://suttacentral.net/dn1"
}
```

### Database Schema

- The appdata database stores suttas with `uid` field (e.g., "dn1", "sn56.102")
- Need to query database to verify UID exists: `SELECT EXISTS(SELECT 1 FROM suttas WHERE uid = ?)`

### Normalization Strategy

JavaScript uses this diacritics map:
```javascript
{ṁ:'m', ŋ:'m', ā:'a', ī:'i', ū:'u', ṭ:'t', ñ:'n', ṇ:'n', ṅ:'n', ḍ:'d', ś:'s', ḷ:'l', ḥ:'h', ṃ:'m'}
```

Rust should use existing `helpers::latinize()` which already handles these conversions.

### PTS Range Matching Algorithm

The key algorithm from JavaScript:
1. Parse query into: nikaya + volume + page (e.g., "D ii 20" → {nikaya: "d", volume: "ii", page: 20})
2. For each JSON entry:
   - Parse its PTS reference
   - If nikaya and volume match:
     - Check if query page equals entry page (exact match)
     - OR check if query page falls between this entry and the next entry (range match)
3. Handle volume boundaries (when next entry has different volume, assume end of range is page 1300)

### Thread Safety

- Search operations should be fast enough to run on UI thread (in-memory JSON search)
- If performance becomes an issue, consider moving to background thread with loading indicator

### Integration Points

- **Window Manager:** Need access to function that opens sutta by UID in SuttaSearchWindow
- **Clipboard Manager:** Already exists as bridge, use for copying URLs
- **SuttaBridge:** May need to add reference search functions here or create new bridge
- **Database:** Need query function to check if UID exists

## Success Metrics

1. **SM-1:** All Rust tests pass, matching the behavior of JavaScript tests (100% test pass rate)
2. **SM-2:** Users can successfully convert any valid PTS, DPR, or title reference to a SuttaCentral UID
3. **SM-3:** Search returns results in under 200ms for typical queries (measured with ~2000+ JSON entries)
4. **SM-4:** Zero crashes or errors during normal search operations
5. **SM-5:** Clipboard copy operation succeeds 100% of the time on supported platforms

## Open Questions

1. **OQ-1:** Should the window remain open after opening a sutta, or close automatically?
   - **Recommendation:** Keep it open for repeated lookups

2. **OQ-2:** Should there be a keyboard shortcut to focus the search input?
   - **Recommendation:** Yes, make Ctrl+F focus search input when window is active

3. **OQ-3:** How should the search handle multiple results when opening? (e.g., title search finds 10 suttas)
   - **Current spec:** User must click "Open" on individual results in the Database Results list

4. **OQ-4:** Should the window size/position persist between sessions?
   - **Recommendation:** Yes, if this is standard pattern in other windows

5. **OQ-5:** Should we show a count of results (e.g., "23 results found")?
   - **Recommendation:** Yes, show count above each results list

6. **OQ-6:** For the title search, should we prioritize exact matches over partial matches in result ordering?
   - **Recommendation:** Future enhancement, current spec shows results in JSON order

## Implementation Phases

**Phase 1: Backend (Critical Path)**
- Create `pts_reference_search.rs` module
- Implement all search functions with proper normalization
- Write and pass all tests
- Verify test coverage matches JavaScript version

**Phase 2: Bridge Integration**
- Add bridge functions to expose search to QML
- Implement UID extraction and database verification
- Create qmllint type definitions

**Phase 3: Frontend**
- Create ReferenceSearchWindow.qml
- Implement search UI with ComboBox and input
- Display JSON results list
- Add menu item in SuttaSearchWindow

**Phase 4: Database Integration & Actions**
- Implement database results filtering
- Add "Open" button functionality
- Add "Copy URL" button functionality
- Test complete workflow

**Phase 5: Polish**
- Add empty states and error handling
- Optimize performance if needed
- Mobile layout testing
- Documentation

## Appendix: Test Cases to Replicate

From `search_functions.js`, these test cases must pass in Rust:

1. Search DN 1 by identifier: "DN 1" in 'identifier' → should find DN 1
2. Search DN 2 by PTS ref (exact): "D i 47" in 'pts_reference' → should find D i 47
3. Search DN 2 by PTS ref (in-between): "D i 50" in 'pts_reference' → should find D i 47
4. Search DN 14 by PTS ref (exact at volume boundary): "D ii 1" in 'pts_reference' → should find D ii 1
5. Search DN 14 by PTS ref (in-between): "D ii 20" in 'pts_reference' → should find D ii 1
6. Search MN by PTS ref (in-between): "M iii 10" in 'pts_reference' → should find M iii 7
7. Search by name (case insensitive): "brahmajala" in 'name' → should find "Brahmajāla"
8. Search KN by DPR reference: "KN 1" in 'dpr_reference' → should find KN 1
