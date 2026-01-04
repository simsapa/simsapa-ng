# PRD: Browser Extension API Support

## 1. Introduction/Overview

This document describes the implementation of localhost API routes in the Rust + QML Simsapa app to support the existing browser extensions (Firefox and Chrome). The browser extensions provide a sidebar for searching suttas and dictionary words, with results displayed in the extension and the ability to open items in the main Simsapa application.

The legacy Python app provided these API endpoints via Flask on `http://localhost:4848`. This feature ports the necessary routes to the current Rust backend using the Rocket web framework.

### Problem Statement

Users of the Simsapa browser extensions cannot use them with the new Rust + QML app because the required API endpoints are not implemented. This prevents users from:
- Searching suttas and dictionary words from within their browser
- Opening search results in the Simsapa application
- Using autocomplete features in the extension search inputs

## 2. Goals

1. **Full Browser Extension Compatibility**: Implement all API routes required by the browser extension sidebar (`sidebar.ts`)
2. **Response Format Parity**: Return exactly the same JSON structure as the Python API for seamless compatibility
3. **Window Integration**: Enable the extension to open content in the Simsapa application windows
4. **Search Functionality**: Provide search capabilities using ContainsMatch as a placeholder until fulltext search is implemented

## 3. User Stories

1. **As a browser user**, I want to search for Pali words in my browser sidebar so that I can quickly look up definitions while reading online.

2. **As a browser user**, I want to search for suttas by title or content so that I can find relevant texts without switching to the main application.

3. **As a browser user**, I want to click on a search result to open it in the Simsapa application so that I can read the full content with all features.

4. **As a browser user**, I want autocomplete suggestions when typing search queries so that I can find content faster.

5. **As a browser user**, I want to filter search results by language and dictionary source so that I can narrow down results to relevant content.

6. **As a browser user**, I want to copy glossary information from dictionary results so that I can use it in my notes or documents.

## 4. Functional Requirements

### 4.1 API Routes to Implement

The following routes are required by the browser extension (from `sidebar.ts`):

| # | Route | Method | Purpose | sidebar.ts Line |
|---|-------|--------|---------|-----------------|
| 1 | `/` | GET | Health check / server online detection | 623 |
| 2 | `/suttas_fulltext_search` | POST | Search suttas (use ContainsMatch placeholder) | 297 |
| 3 | `/dict_combined_search` | POST | Search dictionary words | 300 |
| 4 | `/suttas/<uid>` | GET | Open sutta in application window | 370 |
| 5 | `/lookup_window_query` | POST | Open word lookup in application | 375 |
| 6 | `/words/<uid>.json` | GET | Get dictionary word data as JSON | 447 |
| 7 | `/sutta_and_dict_search_options` | GET | Get available filter options | 530 |
| 8 | `/sutta_titles_flat_completion_list` | GET | Get sutta title autocomplete list | 558 |
| 9 | `/dict_words_flat_completion_list` | GET | Get dictionary word autocomplete list | 582 |

### 4.2 Route Specifications

#### 4.2.1 GET `/` (Index/Health Check)

**Purpose**: Allow the extension to detect if the Simsapa server is running.

**Current Implementation**: Already exists in `api.rs:294-314`, returns HTML page.

**Required Change**: The existing implementation is sufficient. The extension only checks for a successful response (`response.ok`).

---

#### 4.2.2 POST `/suttas_fulltext_search`

**Purpose**: Search suttas by text query with optional language filtering.

**Request Body**:
```json
{
  "query_text": "string (required)",
  "page_num": "integer (optional, default: 0)",
  "suttas_lang": "string (optional, default: 'Languages')",
  "suttas_lang_include": "boolean (optional, default: true)"
}
```

**Response** (matches Python `ApiSearchResult`):
```json
{
  "hits": 42,
  "results": [
    {
      "uid": "sn12.2/en/bodhi",
      "schema_name": "appdata",
      "table_name": "suttas",
      "source_uid": "sn",
      "title": "Vibhanga Sutta",
      "ref": "SN 12.2",
      "nikaya": "SN",
      "author": "Bodhi",
      "snippet": "...matched text with <b>highlight</b>...",
      "page_number": null,
      "score": null,
      "rank": null
    }
  ]
}
```

**Implementation Notes**:
- Use ContainsMatch search as placeholder (fulltext search not yet implemented)
- Query both `appdata.suttas` tables
- Apply language filter if `suttas_lang` is not "Languages"
- `suttas_lang_include: true` means include only that language; `false` means exclude
- Page size: 20 results per page

**Python Reference**: `api.py:253-287`

---

#### 4.2.3 POST `/dict_combined_search`

**Purpose**: Search dictionary words with language and dictionary source filtering.

**Request Body**:
```json
{
  "query_text": "string (required)",
  "page_num": "integer (optional, default: 0)",
  "dict_lang": "string (optional, default: 'Languages')",
  "dict_lang_include": "boolean (optional, default: true)",
  "dict_dict": "string (optional, default: 'Dictionaries')",
  "dict_dict_include": "boolean (optional, default: true)"
}
```

**Response**:
```json
{
  "hits": 15,
  "results": [
    {
      "uid": "dhamma/dpd",
      "schema_name": "dpd",
      "table_name": "dpd_headwords",
      "source_uid": "dpd",
      "title": "dhamma",
      "ref": null,
      "nikaya": null,
      "author": null,
      "snippet": "...definition preview...",
      "page_number": null,
      "score": null,
      "rank": null
    }
  ],
  "deconstructor": ["dhamma + ssa", "dham + massa"]
}
```

**Implementation Notes**:
- Search across `appdata.dict_words`, `userdata.dict_words`, `dpd.dpd_headwords`, `dpd.dpd_roots`
- The `deconstructor` field contains word breakdown suggestions
- Apply language and dictionary filters as specified

**Deconstructor Implementation**:
The deconstructor feature can be implemented using existing backend functions:
- Call `DpdDbHandle::dpd_deconstructor_list(query)` at `backend/src/db/dpd.rs:126-141`
- This is the same function used by `SuttaBridge.dpd_deconstructor_list()` in `WordSummary.qml`
- Returns `Vec<String>` with suggestions like `["kamma + pattā", "kamma + apattā"]`

**Python Reference**: `api.py:289-333`

---

#### 4.2.4 GET `/suttas/<uid>`

**Purpose**: Open a sutta in the Simsapa application window.

**URL Parameters**:
- `uid`: Sutta UID path (e.g., `sn12.2/en/bodhi`)

**Query Parameters**:
- `window_type`: Optional, e.g., `Sutta+Study` (URL encoded)

**Response**: Plain text message (extension ignores response body)
```
The Simsapa window should appear with 'sn12.2/en/bodhi'. You can close this tab.
```

**Implementation Notes**:
- Call `callback_open_sutta_search_window` with the sutta data JSON
- The window_type parameter can be used to determine which window type to open
- Return 200 OK even if showing the window

**Python Reference**: `api.py:487-504`

---

#### 4.2.5 POST `/lookup_window_query`

**Purpose**: Open the word lookup window and search for a word.

**Request Body**:
```json
{
  "query_text": "string (required)"
}
```

**Response**: Plain text `"OK"` with status 200

**Implementation Notes**:
- This should open a SuttaSearchWindow
- Set the search query input to the provided `query_text`
- Run the search by calling the appropriate QML function (e.g., `handle_query()`)
- The existing `callback_run_lookup_query` FFI function should be used

**Python Reference**: `api.py:416-431`

---

#### 4.2.6 GET `/words/<uid>.json`

**Purpose**: Get full dictionary word data as JSON for copying glossary information.

**URL Parameters**:
- `uid`: Word UID (e.g., `dhamma/dpd`)

**Response**: Array of dictionary word objects
```json
[
  {
    "uid": "dhamma/dpd",
    "lemma_1": "dhamma",
    "pos": "masc",
    "grammar": "masc, a stem",
    "meaning_1": "nature; character",
    "construction": "dham + ma",
    "word": "dhamma",
    "definition_plain": "nature; character; truth; the teaching..."
  }
]
```

**Implementation Notes**:
- Query the word by UID from appropriate table (dpd_headwords, dpd_roots, dict_words)
- Return the full record as a dictionary/object
- DPD words have specific fields (`lemma_1`, `pos`, `grammar`, `meaning_1`, `construction`)
- Other dict_words have different fields (`word`, `definition_plain`)
- Return empty array `[]` if word not found

**Python Reference**: `api.py:524-589`

---

#### 4.2.7 GET `/sutta_and_dict_search_options`

**Purpose**: Get available filter options for the search dropdowns.

**Response**:
```json
{
  "sutta_languages": ["pli", "en", "de", "hu", "it", "pt"],
  "dict_languages": ["pli", "en", "de"],
  "dict_sources": ["DPD", "NCPED", "PTS", "Nyanatiloka"]
}
```

**Implementation Notes**:
- Query database for distinct language values from suttas table
- Query database for distinct language values from dict_words tables
- Query database for distinct source/dictionary labels
- Return sorted lists

**Python Reference**: `api.py:376-392`

---

#### 4.2.8 GET `/sutta_titles_flat_completion_list`

**Purpose**: Get list of sutta titles for autocomplete.

**Response**: Array of strings
```json
["AN 1.1 - Rūpādi Vagga", "AN 1.2 - Nīvaraṇappahāna Vagga", ...]
```

**Implementation Notes**:
- Return empty array `[]` as placeholder for now
- Future implementation should query sutta titles from database
- Results should be sorted using Pali sort order

**Python Reference**: `api.py:360-374`

---

#### 4.2.9 GET `/dict_words_flat_completion_list`

**Purpose**: Get list of dictionary words for autocomplete.

**Response**: Array of strings
```json
["dhamma", "dukkha", "nibbāna", ...]
```

**Implementation Notes**:
- Return empty array `[]` as placeholder for now
- The browser extension currently loads this from a bundled JSON file, so this route is not critical
- Future implementation could query DPD lemmas and roots

**Python Reference**: `api.py:335-358`

---

### 4.3 Data Structures

#### SearchResult (for both sutta and dict results)
```rust
#[derive(Serialize)]
struct SearchResult {
    uid: String,
    schema_name: String,      // "appdata", "userdata", or "dpd"
    table_name: String,       // "suttas", "dict_words", "dpd_headwords", "dpd_roots"
    source_uid: Option<String>,
    title: String,
    #[serde(rename = "ref")]
    sutta_ref: Option<String>,
    nikaya: Option<String>,
    author: Option<String>,
    snippet: String,
    page_number: Option<i32>,
    score: Option<f64>,
    rank: Option<i32>,
}
```

#### ApiSearchResult
```rust
#[derive(Serialize)]
struct ApiSearchResult {
    hits: i32,
    results: Vec<SearchResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    deconstructor: Option<Vec<String>>,
}
```

#### SearchOptions
```rust
#[derive(Serialize)]
struct SearchOptions {
    sutta_languages: Vec<String>,
    dict_languages: Vec<String>,
    dict_sources: Vec<String>,
}
```

## 5. Non-Goals (Out of Scope)

1. **Full-text search implementation**: Use ContainsMatch as a placeholder; fulltext search will be implemented separately
2. **Graph generation**: The `/generate_graph` route is not used by the browser extension
4. **Bookmark routes**: `/get_bookmarks_*` routes are not used by the browser extension
5. **Queue/message routes**: `/queues/*` routes are for internal app communication
6. **Sutta study routes**: `/sutta_study_*` routes are not used by the browser extension
7. **Browser extension modifications**: The extension code will not be modified; API must match existing expectations
8. **Port configuration UI**: The hardcoded port 4848 is sufficient for now

## 6. Design Considerations

### 6.1 Architecture

The API routes will be added to the existing `bridges/src/api.rs` file, which uses the Rocket web framework. The routes will:

1. Use the existing `DbManager` for database access
2. Use existing FFI callbacks for window operations
3. Share the `Arc<DbManager>` state with existing routes

### 6.2 Database Access

The implementation will need to query:
- `appdata.sqlite3`: suttas, dict_words tables
- `dpd.sqlite3`: dpd_headwords, dpd_roots tables (if available)
- `userdata.sqlite3`: user-added content (if available)

The existing `DbManager` in `backend/src/db/` provides access to these databases.

### 6.3 Search Implementation

For the initial implementation:
- Use ContainsMatch (SQL LIKE queries) for text searching
- This is already implemented in the backend for other features
- Performance may be slower than fulltext search but functional

### 6.4 CORS

The existing CORS configuration (`CorsOptions::default()`) allows all origins, which is required for browser extensions to make requests to localhost.

## 7. Technical Considerations

### 7.1 Dependencies

No new dependencies required. The implementation uses:
- `rocket` - Web framework (already in use)
- `rocket_cors` - CORS support (already in use)
- `serde` / `serde_json` - JSON serialization (already in use)

### 7.2 Existing Backend Functions to Use

The following backend functions are already implemented and can be used:

#### Sutta Search
| Function | Location | Description |
|----------|----------|-------------|
| `SearchQueryTask::new()` | `backend/src/query_task.rs:41-72` | Create search task with params |
| `SearchQueryTask::results_page()` | `backend/src/query_task.rs:1073-1201` | Get paginated search results |
| `SearchQueryTask::suttas_contains_match_fts5()` | `backend/src/query_task.rs:621-719` | FTS5-based contains match for suttas |
| `SearchQueryTask::total_hits()` | `backend/src/query_task.rs:1204-1206` | Get total hit count |
| `AppdataDbHandle::get_sutta()` | `backend/src/db/appdata.rs:55-73` | Get sutta by UID |
| `AppdataDbHandle::get_full_sutta_uid()` | `backend/src/db/appdata.rs:75-134` | Resolve partial UID to full UID |

#### Dictionary Search
| Function | Location | Description |
|----------|----------|-------------|
| `SearchQueryTask::dict_words_contains_match_fts5()` | `backend/src/query_task.rs:721-955` | Three-phase dict search |
| `DpdDbHandle::dpd_lookup()` | `backend/src/db/dpd.rs:165-387` | Core DPD lookup with inflections |
| `DpdDbHandle::dpd_lookup_json()` | `backend/src/db/dpd.rs:402-412` | DPD lookup returning JSON |
| `DictionariesDbHandle::get_word()` | `backend/src/db/dictionaries.rs:11-29` | Get dict word by UID |

#### Deconstructor (Fully Implementable)
| Function | Location | Description |
|----------|----------|-------------|
| `DpdDbHandle::dpd_deconstructor_list()` | `backend/src/db/dpd.rs:126-141` | Get deconstructor suggestions |
| `DpdDbHandle::dpd_deconstructor_query()` | `backend/src/db/dpd.rs:69-124` | Query deconstructor with fallback |
| `Lookup::deconstructor_unpack()` | `backend/src/db/dpd_models.rs:170-187` | Parse JSON to Vec<String> |

**Note**: `dpd_deconstructor_list()` is already used by `SuttaBridge.dpd_deconstructor_list()` in `WordSummary.qml`

#### Get Word/Sutta Data
| Function | Location | Description |
|----------|----------|-------------|
| `AppData::render_sutta_content()` | `backend/src/app_data.rs:128-225` | Render sutta HTML |
| `AppData::get_dpd_headword_by_uid()` | `backend/src/app_data.rs:470-501` | Get DPD headword as JSON |
| `AppData::get_dpd_root_by_root_key()` | `backend/src/app_data.rs:503-534` | Get DPD root as JSON |

#### Search Options
| Function | Location | Description |
|----------|----------|-------------|
| `DbManager::get_sutta_languages()` | `backend/src/db/mod.rs:162-164` | Get distinct sutta languages |
| `AppdataDbHandle::get_sutta_languages()` | `backend/src/db/appdata.rs:18-53` | Query distinct languages |
| `AppdataDbHandle::get_sutta_language_labels_with_counts()` | `backend/src/db/appdata.rs:471-507` | Languages with counts |

#### Helper Functions
| Function | Location | Description |
|----------|----------|-------------|
| `normalize_query_text()` | `backend/src/helpers.rs:501+` | Normalize query text |
| `query_text_to_uid_field_query()` | `backend/src/helpers.rs:56-157` | Convert input to UID query |
| `SearchQueryTask::highlight_query_in_content()` | `backend/src/query_task.rs:90-110` | Add highlight spans |
| `SearchQueryTask::fragment_around_query()` | `backend/src/query_task.rs:196-218` | Create snippet around match |

### 7.3 Backend Functions to Add

The following functions need to be added:

1. **Dictionary languages**: `DictionariesDbHandle::get_distinct_languages() -> Vec<String>`
   - Query: `SELECT DISTINCT language FROM dict_words WHERE language IS NOT NULL`

2. **Dictionary sources**: `DictionariesDbHandle::get_distinct_sources() -> Vec<String>`
   - Query: `SELECT DISTINCT dict_label FROM dict_words`

### 7.4 Related Types

| Type | Location | Description |
|------|----------|-------------|
| `SearchParams` | `backend/src/types.rs:84-110` | Search parameters struct |
| `SearchMode` | `backend/src/types.rs:63-82` | Enum: ContainsMatch, DpdLookup, etc. |
| `SearchArea` | `backend/src/types.rs:56-61` | Enum: Suttas, Dictionary, Library |
| `SearchResult` | `backend/src/types.rs:112-132` | Result with uid, title, snippet, etc. |
| `SearchResultPage` | `backend/src/types.rs:268-274` | Paginated results container |

### 7.5 Database Access Pattern

```rust
// Get app data and database manager
let app_data = get_app_data();

// Access different databases
app_data.dbm.appdata    // AppdataDbHandle - suttas, dict_words
app_data.dbm.dictionaries  // DictionariesDbHandle - dict_words
app_data.dbm.dpd        // DpdDbHandle - dpd_headwords, dpd_roots, lookup
app_data.dbm.userdata   // AppdataDbHandle - user content
```

### 7.6 QML Integration

The `/lookup_window_query` POST route needs to:
1. Open a SuttaSearchWindow (or reuse existing)
2. Set the search input text
3. Trigger the search

This may require a new C++ callback function or modification of existing `callback_run_lookup_query`.

### 7.7 File Locations

- **API routes**: `bridges/src/api.rs`
- **Database queries**: `backend/src/db/` (appdata.rs, dpd.rs, etc.)
- **Integration tests**: `bridges/tests/` (new directory/files)
- **Test configuration**: `bridges/.env`

## 8. Success Metrics

1. **Functional Compatibility**: All 8 API routes return valid responses that the browser extension can parse
2. **Extension Operation**: The browser extension sidebar loads without errors when the Rust app is running
3. **Search Works**: Users can search and see results in both Suttas and Dictionary tabs
4. **Window Opening**: Clicking "Show in Simsapa" opens the correct content in the application
5. **Filter Options**: Language and dictionary dropdown menus populate with options from the database
6. **No Regressions**: Existing API routes continue to function correctly

## 9. Testing Requirements

### 9.1 Test Environment Setup

Create `bridges/.env` file:
```
SIMSAPA_DIR=../../bootstrap-assets-resources/dist/simsapa-ng
ENABLE_PRINT_LOG=true
```

### 9.2 Integration Tests

Create integration tests in `bridges/tests/test_browser_extension_api.rs`:

1. **Test server health check**: GET `/` returns 200
2. **Test sutta search**: POST `/suttas_fulltext_search` with valid query returns expected structure
3. **Test dict search**: POST `/dict_combined_search` with valid query returns expected structure
4. **Test search options**: GET `/sutta_and_dict_search_options` returns non-empty arrays
5. **Test completion lists**: GET `/sutta_titles_flat_completion_list` returns array (empty OK)
6. **Test word JSON**: GET `/words/<valid_uid>.json` returns array with word data
7. **Test empty results**: Search with nonsense query returns `hits: 0` and empty results

### 9.3 Manual Testing

The user will manually test with the browser extensions:
1. Install/enable the Simsapa browser extension
2. Start the Rust Simsapa application
3. Verify the extension sidebar shows "online" status
4. Test searching in both tabs
5. Test opening results in the application
6. Test filter dropdowns
7. Test autocomplete (if implemented)

## 10. Open Questions

1. **DPD Database Access**: Is the DPD database (dpd.sqlite3) always available, or should the code handle its absence gracefully?

2. **Snippet Generation**: How should search result snippets be generated? Should they show context around the match, or just the beginning of the content? (Note: `SearchQueryTask::fragment_around_query()` can be used)

3. **Rate Limiting**: Should there be any rate limiting on the search endpoints to prevent performance issues?

4. **Caching**: Should search results or completion lists be cached to improve performance?

## 11. Implementation Order

Suggested order of implementation:

1. **Phase 1 - Core Search Routes**
   - POST `/suttas_fulltext_search`
   - POST `/dict_combined_search`
   - GET `/sutta_and_dict_search_options`

2. **Phase 2 - Window Integration**
   - GET `/suttas/<uid>`
   - POST `/lookup_window_query`

3. **Phase 3 - Data Routes**
   - GET `/words/<uid>.json`

4. **Phase 4 - Completion Lists (Placeholders)**
   - GET `/sutta_titles_flat_completion_list`
   - GET `/dict_words_flat_completion_list`

5. **Phase 5 - Testing**
   - Integration tests
   - Manual browser extension testing

---

## Appendix A: Existing Bridge Layer Functions

These `SuttaBridge` functions (in `bridges/src/sutta_bridge.rs`) wrap backend functionality and can serve as reference implementations:

| Function | Line | Backend Call |
|----------|------|--------------|
| `results_page()` | 858-919 | `SearchQueryTask::new()` + `results_page()` |
| `dpd_deconstructor_list()` | 934-942 | `app_data.dbm.dpd.dpd_deconstructor_list()` |
| `dpd_lookup_json()` | 944-948 | `app_data.dbm.dpd.dpd_lookup_json()` |
| `get_sutta_html()` | 972-995 | `get_sutta()` + `render_sutta_content()` |
| `get_word_html()` | 997-1062 | `get_word()` from dictionaries |
| `get_sutta_language_labels()` | 2247-2265 | `get_sutta_languages()` |
| `normalize_query_text()` | 930-932 | `helpers::normalize_query_text()` |

---

## Appendix B: Python API Reference

Key functions from `simsapa-legacy/simsapa/app/api.py`:

**Note**: Use these as reference for expected behavior, but prefer using the existing Rust backend functions documented in Section 7.2.

### Search Parameters (Python)
```python
@dataclass
class SearchParams:
    mode: SearchMode          # FulltextMatch, Combined, etc.
    page_len: int = 20
    lang: Optional[str] = None
    lang_include: bool = True
    source: Optional[str] = None
    source_include: bool = True
    enable_regex: bool = False
    fuzzy_distance: int = 0
```

### API Search Result (Python)
```python
ApiSearchResult = TypedDict('ApiSearchResult', {
    'hits': int,
    'results': List[Dict],
    'deconstructor': Optional[List[str]],
})
```

## Appendix C: Deconstructor Implementation Details

The deconstructor feature breaks compound Pali words into their component parts. It is already implemented in the Rust backend and used by `WordSummary.qml`.

### Call Chain
```
Browser Extension
    ↓ POST /dict_combined_search
API Route (api.rs)
    ↓ calls
DpdDbHandle::dpd_deconstructor_list(query)  // backend/src/db/dpd.rs:126-141
    ↓ calls
DpdDbHandle::dpd_deconstructor_query(query, false)  // dpd.rs:69-124
    ↓ returns
Lookup struct with deconstructor JSON field
    ↓ calls
Lookup::deconstructor_unpack()  // dpd_models.rs:170-187
    ↓ returns
Vec<String> e.g. ["kamma + pattā", "kamma + apattā"]
```

### Example Usage in API Route
```rust
// In the dict_combined_search handler:
let app_data = get_app_data();
let deconstructor_results = app_data.dbm.dpd.dpd_deconstructor_list(&query_text);

// Include in response
ApiSearchResult {
    hits: results.len() as i32,
    results: search_results,
    deconstructor: Some(deconstructor_results),
}
```

### Existing QML Reference
The same pattern is used in `SuttaBridge.dpd_deconstructor_list()` at `bridges/src/sutta_bridge.rs:934-942`:
```rust
fn dpd_deconstructor_list(&self, query: String) -> Vec<String> {
    let app_data = get_app_data();
    app_data.dbm.dpd.dpd_deconstructor_list(&query)
}
```

---

## Appendix D: Browser Extension Request Examples

### Sutta Search Request (from sidebar.ts:314-329)
```javascript
const data = {
  query_text: query_text,
  suttas_lang: suttas_lang,           // e.g., "en" or null
  suttas_lang_include: suttas_lang_include,  // true/false
  dict_lang: dict_lang,
  dict_lang_include: dict_lang_include,
  dict_dict: dict_dict,
  dict_dict_include: dict_dict_include,
};

fetch(url, {
  method: "POST",
  headers: {
    "Content-Type": "application/json; charset=utf-8",
  },
  body: JSON.stringify(data),
})
```

### Show Word Request (from sidebar.ts:374-385)
```javascript
const url = SIMSAPA_BASE_URL + "/lookup_window_query";
const data = { query_text: uid };

fetch(url, {
  method: "POST",
  headers: {
    "Content-Type": "application/json; charset=utf-8",
  },
  body: JSON.stringify(data),
})
```
