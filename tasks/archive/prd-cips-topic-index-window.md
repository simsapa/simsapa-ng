# PRD: CIPS Topic Index Window

## 1. Introduction/Overview

The CIPS (Comprehensive Index of Pāli Suttas) Topic Index Window integrates the CIPS general index data into Simsapa, providing users with a searchable, alphabetically-organized topic index for navigating Pāli suttas. Users can browse topics by letter, search for specific headwords, follow cross-references between related topics, and open suttas directly from the index.

**Problem Solved:** Currently, users must visit the external CIPS website to browse the comprehensive sutta topic index. This feature brings that functionality directly into the app, enabling offline access and seamless integration with the sutta reader.

## 2. Goals

1. Parse the CIPS `general-index.csv` data into a JSON file for static inclusion in the Rust backend
2. Create a `TopicIndexWindow.qml` that displays the topic index with alphabet navigation
3. Enable searching/filtering of headwords with minimum 3 character requirement
4. Allow users to navigate cross-references between topics
5. Enable opening suttas directly from topic entries
6. Support both desktop and mobile platforms with appropriate UI layouts

## 3. User Stories

1. **As a sutta reader**, I want to browse topics alphabetically so that I can discover suttas related to specific themes.

2. **As a Pāli student**, I want to search for topics by English or Pāli terms so that I can quickly find relevant suttas.

3. **As a researcher**, I want to follow cross-references between related topics so that I can explore connected themes across the canon.

4. **As a mobile user**, I want the topic index to work well on my phone/tablet so that I can browse topics on the go.

5. **As a new user**, I want to understand what CIPS is and how to use the topic index so that I can make the most of this feature.

## 4. Functional Requirements

### 4.1 CSV Parser (CLI Module)

1. The parser must be implemented in the `cli` module as a new subcommand
2. The parser must accept command-line arguments for input CSV path and output JSON path
3. The parser must read the tab-separated CSV file from CIPS (`general-index.csv`)
4. The parser must sort headwords alphabetically, with Pāli accented letters sorting under their latinized equivalents using the existing `latinize()` helper function (e.g., "Ānanda" sorts under "A" as "ananda")
5. The parser must group entries by first letter into the following JSON structure:
   ```json
   [
     {
       "letter": "A",
       "headwords": [
         {
           "headword": "abandoning (pajahati, pahāna)",
           "entries": [
             {"sub": "blemishes in oneself", "ref": "MN5", "type": "sutta"},
             {"sub": "striving", "ref": "DN33:1.11.0", "type": "sutta"}
           ]
         },
         {
           "headword": "abandoning spiritual path",
           "entries": [
             {"sub": "", "ref": "disrobing", "type": "xref"}
           ]
         }
       ]
     },
     {"letter": "B", "headwords": [...]},
     ...
   ]
   ```
6. The parser must identify cross-references (entries containing "xref" in the sub-word column) and set `type: "xref"` with the target headword in the `ref` field
7. The parser must identify sutta references (entries not containing "xref") and set `type: "sutta"` with the sutta reference in the `ref` field
8. Entries with empty sub-word columns must have `sub: ""` (empty string)
9. The output JSON file must be saved to `assets/general-index.json`

### 4.2 Static JSON Inclusion

10. The JSON file must be included in the Rust backend as a static string constant `CIPS_GENERAL_INDEX_JSON` in `app_settings.rs`, following the pattern of `SUTTA_REFERENCE_CONVERTER_JSON`
11. The backend must provide functions to:
    - Load and parse the JSON data
    - Search headwords (case-insensitive, partial match, including Pāli terms in parentheses)
    - Get all headwords for a specific letter
    - Look up a specific headword by name (for xref navigation)

### 4.3 TopicIndexWindow.qml

#### Header Section
12. The window must have an "Info" button at the top-left corner (following LibraryWindow pattern)
13. The window must have a "Close" button at the top-right corner on desktop (following LibraryWindow pattern)
14. The window title must be "Topic Index - Simsapa"

#### Info Dialog
15. Clicking the "Info" button must open a dialog containing:
    - Brief explanation of CIPS (Comprehensive Index of Pāli Suttas)
    - Instructions on how to use the topic index
    - Link to the CIPS website (https://cips.dhammatalks.net/)
    - Attribution/credits for the CIPS project

#### Search Section
16. The window must have a search input field below the header (following ReferenceSearchWindow pattern)
17. Search must require a minimum of 3 characters before filtering results
18. Search must be case-insensitive
19. Search must match partial words (e.g., "abando" matches "abandoning")
20. Search must match Pāli terms in parentheses (e.g., searching "pajahati" finds "abandoning (pajahati, pahāna)")
21. Search must match both headwords and sub-entries (e.g., searching "blemishes" finds the entry under "abandoning (pajahati, pahāna)")
22. Search must use debounced input (300ms delay) to avoid excessive filtering

#### Alphabet Navigation
22. The window must display a row of letter buttons (A-Z) below the search input
23. Letter buttons must behave as radio buttons - only one letter can be active at a time
24. Clicking a letter button must switch to display that letter's topic section exclusively
25. The default state must have letter "A" enabled/selected
26. When search input has 3+ characters, alphabet buttons must become disabled (grayed out)
27. When search input is cleared or has fewer than 3 characters, alphabet buttons must re-enable

#### Topic List Display
28. Topics must be displayed in an indented list format (similar to spine-item sub-chapters in BooksList.qml)
29. Headwords must be displayed as section headers (bold or prominent styling)
30. Sub-entries must be indented under their parent headword
31. Sub-entries with empty sub-word must display only the link/xref (no sub-word text)
32. Sutta links must display formatted reference with space (e.g., "AN 4.10" not "AN4.10") followed by Pāli title (e.g., "AN 4.10 Yogasutta")
33. Sutta references must be styled as clickable links with dashed underline (matching HTML sutta link style) using palette.link color
34. Cross-references must be styled distinctly (e.g., italicized or with "see:" prefix)

#### Interaction Behavior
35. Clicking a sutta link must:
    - Use the full sutta_ref with segment ID from the JSON (e.g., "dn3:2.3.0" not just "dn3")
    - Include the hashtag fragment to navigate to the specific location (e.g., "#dn3:2.3.0")
    - Open the sutta in a new tab or new window based on "Open in new window" checkbox state
36. The window must include an "Open in new window" checkbox (following ReferenceSearchWindow pattern)
37. Clicking a cross-reference link must:
    - Switch to the appropriate letter section containing the referenced headword
    - Scroll to and highlight the referenced headword
38. Clicking a headword in search results must:
    - Clear the search input
    - Switch to the letter section containing that headword
    - Scroll to and highlight the headword

#### Mobile Support
39. The window must use full screen dimensions on mobile (`Screen.desktopAvailableWidth` x `Screen.desktopAvailableHeight`)
40. The window must include `top_bar_margin` spacing for mobile status bar (following DatabaseValidationDialog pattern)
41. The window must include extra bottom margin (60px) for mobile navigation bar (following DatabaseValidationDialog pattern)
42. Mobile must have Close button in the top-right only (no bottom Close button needed)

### 4.4 Window Management

43. The window must be accessible from SuttaSearchWindow menu: Windows > "Topic Index..."
44. The window must use `Qt.Dialog` flags
45. The window must use the application's theme (light/dark mode support via ThemeHelper)

### 4.5 Data Loading

46. The topic index JSON must be deserialized only once, on first TopicIndexWindow open
47. Subsequent TopicIndexWindow opens must reuse the already-parsed data (no re-parsing)
48. Loading must be handled via `sutta_bridge.rs` with a cached data structure
49. While loading, the search input and alphabet buttons must be disabled
50. While loading, a "Loading..." message must be displayed instead of the headword list
51. Once loaded, the search input and alphabet buttons must become enabled and the headword list displayed

## 5. Non-Goals (Out of Scope)

1. **Editing the index** - Users cannot add, modify, or delete index entries
2. **Offline sutta content** - If a sutta is not in the local database, it will show "not found" (no web fallback)
3. **Multiple language support** - The index is English-only (matching CIPS source)
4. **Bookmark integration** - No ability to bookmark specific index entries
5. **History tracking** - No tracking of previously viewed topics
6. **Custom sorting** - No user-configurable sort order
7. **Export functionality** - No export of search results or selected topics

## 6. Design Considerations

### UI Layout Reference

The window layout should follow these existing components:
- **Header pattern:** LibraryWindow.qml (Info/Close buttons positioning)
- **Search input:** ReferenceSearchWindow.qml (TextField with debounce timer)
- **Indented list:** BooksList.qml / ChapterListItem.qml (depth-based indentation)
- **Mobile layout:** DatabaseValidationDialog.qml (top_bar_margin, bottom margin for buttons)

### Visual Hierarchy

```
┌─────────────────────────────────────────┐
│ [Info]                          [Close] │  <- Header (desktop & mobile)
├─────────────────────────────────────────┤
│ [Search input field________________]    │  <- Search
├─────────────────────────────────────────┤
│ [A][B][C][D][E][F]...[X][Y][Z]         │  <- Alphabet nav (radio buttons)
├─────────────────────────────────────────┤
│ ▼ A                                     │  <- Letter section
│   abandoning (pajahati, pahāna)         │  <- Headword
│     • blemishes in oneself              │  <- Sub-entry text
│       MN 5 Anaṅgaṇasutta                │  <- Sutta link (dashed underline)
│     • striving                          │
│       DN 33:1.11.0 Saṅgītisutta         │  <- Sutta link with segment
│   abandoning spiritual path             │  <- Headword
│     • see: disrobing                    │  <- Sub-entry (xref)
│   ...                                   │
└─────────────────────────────────────────┘
```

### Highlighting

When navigating to a headword (from search results or xref), apply a highlight effect:
- Background color change (e.g., palette.highlight with reduced opacity)
- The highlight remains until the user selects another item (no timeout or fade-out needed)
- This helps users locate the target headword in the list

## 7. Technical Considerations

### Existing Code to Reuse

- `backend/src/helpers.rs`: `latinize()` function for sorting Pāli terms
- `backend/src/app_settings.rs`: Pattern for static JSON inclusion (`SUTTA_REFERENCE_CONVERTER_JSON`)
- `bridges/src/sutta_bridge.rs`: `get_full_sutta_uid()` for sutta lookup, `emit_show_sutta_from_reference_search()` for opening suttas
- `assets/qml/ThemeHelper.qml`: Theme support
- `assets/qml/Logger.qml`: Logging

### New Files Required

1. `cli/src/bootstrap/parse_cips_index.rs` - CSV parser implementation
2. `assets/general-index.json` - Generated JSON data file
3. `assets/qml/TopicIndexWindow.qml` - Main window component
4. `assets/qml/TopicIndexInfoDialog.qml` - Info dialog component (optional, could be inline)
5. `assets/qml/com/profoundlabs/simsapa/TopicIndexBridge.qml` - QML type definition for qmllint (if new bridge needed)

### Data Size Consideration

The CSV has ~21,742 lines. The JSON file will be larger (includes Pāli titles) but should still be reasonable for static inclusion. Consider:
- Minified JSON (no pretty-printing) to reduce size
- The data is loaded/parsed once on first TopicIndexWindow open, then cached

### Bridge Functions Needed (in `sutta_bridge.rs`)

Functions to add to `SuttaBridge`:
- `load_topic_index()` - Parse JSON and cache the data structure (called once on first window open)
- `is_topic_index_loaded()` - Check if data is already cached (to show loading state)
- `get_letters()` - Return array of available letters
- `get_headwords_for_letter(letter: String)` - Return headwords for a letter
- `search_headwords(query: String)` - Search with partial matching
- `get_headword_by_id(headword_id: String)` - Get full entry for a headword (for xref navigation)

The cache should be stored in a static or lazy-initialized structure so it persists across TopicIndexWindow instances.

## 8. Success Metrics

1. **Functionality:** All 21,742 index entries are correctly parsed and displayable
2. **Performance:** Initial load time under 1 second; search results appear within 300ms of typing
3. **Usability:** Users can navigate from topic to sutta in 3 clicks or fewer
4. **Cross-platform:** Window displays correctly on desktop (Linux, macOS, Windows) and mobile (Android, iOS)
5. **Accessibility:** Alphabet navigation allows quick access to any letter section

## 9. CSV Parsing Procedures (Based on CIPS JavaScript)

This section documents the algorithms used in the original CIPS JavaScript code that should be replicated in the Rust CLI parser.

### 9.1 CSV File Format

The `general-index.csv` is **TAB-delimited** with 3 columns per line:
```
[headword]\t[subheading]\t[locator]
```

Example rows:
```
abandoning spiritual path		xref disrobing
giving up		xref abandoning (pajahati, pahāna)
abandoning (pajahati, pahāna)	blemishes in oneself	MN5
abandoning (pajahati, pahāna)	striving	DN33:1.11.0
killing, giving up	Buddha has	DN1:1.7.0.1
```

### 9.2 Diacritic Normalization for Alphabetical Grouping

To determine which letter section a headword belongs to, normalize using Unicode NFD:

```javascript
// JavaScript approach
function normalizeDiacriticString(string) {
  return string
    .normalize("NFD")                    // Separates diacritics from letters
    .replace(/[\u0300-\u036f]/g, "");    // Removes diacritic characters
}
```

Result: `ā → a`, `ī → i`, `ū → u`, `ñ → n`, `Ā → A`

In Rust, use the existing `latinize()` helper which performs equivalent normalization.

### 9.3 Sorting Algorithm

#### 9.3.1 Headword Sorting

Headwords are sorted using locale-aware comparison with `sensitivity: "base"` (ignores case and diacritics). Additionally, **ignore words** are stripped from the beginning before comparison:

```
IGNORE_WORDS = ["in", "of", "with", "from", "to", "for", "on", "the", "as", "a", "an", "vs.", "and"]
```

Algorithm:
1. Remove leading curly quotes (`"`)
2. Strip leading ignore words (repeatedly, in case of "of the ...")
3. Compare using case-insensitive, diacritic-insensitive comparison

Example: `"the Buddha"` sorts as `"Buddha"`, `"of the aggregates"` sorts as `"aggregates"`

#### 9.3.2 Citation (Locator) Sorting

Sutta references are sorted in **canonical book order**, then **naturally** within each book:

```
BOOK_ORDER = ["DN", "MN", "SN", "AN", "Kp", "Dhp", "Ud", "Iti", "Snp", "Vv", "Pv", "Thag", "Thig"]
```

Natural sorting ensures: `DN1, DN2, DN10, DN11` (not `DN1, DN10, DN11, DN2`)

### 9.4 Data Structure Building

The parser builds a nested structure:

```
Letter → Headword → Sub-entry → {locators[], xrefs[]}
```

Processing logic:
1. For each CSV row `[headword, sub, locator]`:
2. Determine letter section: `normalizeDiacriticString(headword.charAt(0)).toUpperCase()`
3. If headword doesn't exist in letter section, create it
4. If sub-entry doesn't exist under headword, create it with empty `{locators: [], xrefs: []}`
5. If locator contains "xref", add to `xrefs[]`; otherwise add to `locators[]`

#### 9.4.1 Blank Sub-entry Handling

If sub-heading is blank (whitespace only) AND locator is NOT an xref, replace with `"—"` (em-dash):

```javascript
if (/^\s*$/.test(subhead) && !locator.includes("xref")) {
  subhead = "—";
}
```

This creates a placeholder for direct headword→sutta links without a sub-topic.

### 9.5 Cross-Reference (xref) Detection

Cross-references are identified by the presence of `"xref"` in the locator field:

```javascript
if (/xref/.test(locator)) {
  // It's a cross-reference
  const target = locator.replace("xref ", "");  // Extract target headword
}
```

#### 9.5.1 "See" vs "See Also" Logic

- **"see"** - used when headword has ONLY xrefs (no locators, no other sub-entries)
- **"see also"** - used when headword has both xrefs and other content

### 9.6 Sutta Reference Parsing

#### 9.6.1 Segment ID Format

SuttaCentral segment IDs follow the pattern: `BOOK:chapter.section.subsection`

Examples:
- `DN33:1.11.0` - Dīgha Nikāya 33, segment 1.11.0
- `MN2:3.5` - Majjhima Nikāya 2, segment 3.5
- `AN4.159` - Aṅguttara Nikāya 4.159 (no segment, just sutta number)

#### 9.6.2 URL Construction

Standard SuttaCentral URL pattern:
```
https://suttacentral.net/{sutta_ref}/en/sujato{#segment}
```

Parsing (note: in Simsapa, the citation/locator is called `sutta_ref`):
```javascript
function suttaRefOnly(locator) {
  if (locator.includes(":")) {
    return locator.split(":")[0].toLowerCase();  // "dn33" from "DN33:1.11.0"
  }
  return locator.toLowerCase();
}

function segmentOnly(locator) {
  if (locator.includes(":")) {
    return "#" + locator.toLowerCase();  // "#dn33:1.11.0"
  }
  return "";
}

// Full URL: https://suttacentral.net/dn33/en/sujato#dn33:1.11.0
```

The full locator with segment (e.g., "dn33:1.11.0") should be preserved in the JSON so the app can navigate to the specific segment location.

#### 9.6.3 Book Identification

Extract book abbreviation from locator:
```javascript
function justBook(location) {
  return location.replace(/[:0-9.–-]/g, "").toLowerCase();
}
// "DN33:1.11.0" → "dn"
// "AN4.159" → "an"
```

#### 9.6.4 Special Cases: Vimānavatthu/Petavatthu

These texts use different URLs (suttafriends.org) with a chapter-verse conversion table. For simplicity, in our implementation we can:
- Use the standard SuttaCentral URL pattern
- Or add special handling if these texts are in the local database

### 9.7 ID Normalization for Anchors

Creates valid, unique IDs from headwords for scroll-to navigation:

```javascript
function makeNormalizedId(text) {
  return text
    .trim()
    .replace(/ā/g, "aa")   // Long vowels → doubled
    .replace(/ī/g, "ii")
    .replace(/ū/g, "uu")
    .replace(/Ā/g, "Aa")
    .replace("xref ", "")
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")  // Remove remaining diacritics
    .replace(/\s/g, "-")               // Spaces → hyphens
    .replace(/[,;.…""'"'/()]/g, "");   // Remove punctuation
}
```

Examples:
- `"nibbāna"` → `"nibbaana"`
- `"actions (kamma)"` → `"actions-kamma"`
- `"Ānanda, Ven."` → `"Aananda-Ven"`

### 9.8 JSON Output Structure

Based on the above processing, the final JSON structure should be:

```json
[
  {
    "letter": "A",
    "headwords": [
      {
        "headword": "abandoning (pajahati, pahāna)",
        "headword_id": "abandoning-pajahati-pahaana",
        "entries": [
          {"sub": "—", "refs": [{"sutta_ref": "mn5", "title": "Anaṅgaṇasutta", "type": "sutta"}]},
          {"sub": "blemishes in oneself", "refs": [{"sutta_ref": "mn5", "title": "Anaṅgaṇasutta", "type": "sutta"}]},
          {"sub": "striving", "refs": [{"sutta_ref": "dn33:1.11.0", "title": "Saṅgītisutta", "type": "sutta"}]}
        ]
      },
      {
        "headword": "abandoning spiritual path",
        "headword_id": "abandoning-spiritual-path",
        "entries": [
          {"sub": "", "refs": [{"ref": "disrobing", "type": "xref"}]}
        ]
      }
    ]
  },
  {"letter": "B", "headwords": [...]},
  ...
]
```

**Notes:**
- Multiple refs per sub-entry are possible when a sub-topic references multiple suttas
- Sutta entries use `sutta_ref` (lowercase, includes segment ID if present) and `title` (Pāli title from the Pāli source, e.g., "mn5/pli/ms")
- The `sutta_ref` includes the segment ID when present (e.g., "dn33:1.11.0") for navigation to specific locations
- Display format should add space between book and number: "DN 33:1.11.0" not "DN33:1.11.0"
- Titles are included in JSON to avoid database lookups at display time
- Cross-references use `ref` field with the target headword name

### 9.9 Validation

The parser should validate:
1. All xref targets exist as headwords in the index
2. No duplicate headword+sub combinations (merge refs if found)
3. All locators follow expected format (BOOK + number, optional segment)

## 10. Resolved Questions

1. **Letter button style:** Radio buttons - only one letter visible at a time

2. **Sutta name display:** Include Pāli titles in JSON (from Pāli source like "mn5/pli/ms") to save database lookups. Display format: "AN 4.10 Yogasutta"

3. **Mobile Close button:** Top-right only, no bottom Close button needed

4. **Highlighting behavior:** Highlight remains until user selects another item (no timeout/fade-out)

5. **Menu placement:** Windows > "Topic Index..." in the SuttaSearchWindow menu

6. **Keyboard shortcuts:** Not a concern for now

7. **"Open in new window" option:** Yes, include checkbox like in ReferenceSearchWindow

8. **Search scope:** Yes, search matches both headwords and sub-entries (e.g., searching "blemishes" finds the entry under "abandoning")
