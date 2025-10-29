# PRD: SuttaCentral Import - Rust Implementation

## Overview

Implement the SuttaCentral import functionality in Rust for the bootstrap procedure. This involves importing suttas from SuttaCentral's Bilara JSON format and HTML text format, querying an ArangoDB database for metadata, and populating the appdata.sqlite3 database with suttas, variants, and comments for both Pāli (pli) and English (en) languages.

## Background

### Current State
- Python implementation exists in `simsapa-legacy/scripts/suttacentral.py`
- Placeholder Rust file exists at `cli/src/bootstrap/suttacentral.rs` (2 lines only)
- Bootstrap process has TODO comment at line 160 in `cli/src/bootstrap/mod.rs`
- Other importers (Dhammatalks, Nyanadipa, etc.) are already implemented in Rust

### Target State
- Complete Rust implementation of SuttaCentral import
- Imports suttas for languages 'en' and 'pli'
- Queries ArangoDB for sutta metadata and content
- Processes both Bilara JSON format and HTML text format
- Inserts suttas, variants, and comments into appdata.sqlite3

## Goals

1. **Port Python functionality to Rust** - Translate `suttacentral.py` to Rust
2. **ArangoDB integration** - Connect to and query ArangoDB at localhost:8529
3. **Support both formats** - Handle Bilara JSON and HTML text formats
4. **Import variants and comments** - Include sutta_variants and sutta_comments tables
5. **Language support** - Import for both 'en' and 'pli' languages
6. **Progress tracking** - Show import progress to user

## Non-Goals

- **Skip `add_sc_multi_refs()`** - Not necessary for initial implementation (can be added later)
- Sanskrit text imports (different source)
- Tantivy fulltext indexing (not yet integrated)
- Per-language sutta databases (suttas_lang_{lang}.sqlite3)

## Success Criteria

1. Bootstrap imports suttas from SuttaCentral for 'en' and 'pli' languages
2. Suttas are correctly parsed from both Bilara JSON and HTML formats
3. Sutta variants and comments are imported and linked to parent suttas
4. Database records match Python-generated structure
5. Import completes without errors and shows progress
6. Code follows existing Rust importer patterns (e.g., NyanadipaImporter)

## Technical Design

### Architecture Overview

```
┌──────────────────────────────────────────────────────────────┐
│                    Bootstrap Process                          │
│                   (cli/src/bootstrap/mod.rs)                  │
└────────────────────────┬─────────────────────────────────────┘
                         │
                         ▼
┌──────────────────────────────────────────────────────────────┐
│              SuttaCentralImporter                             │
│           (cli/src/bootstrap/suttacentral.rs)                 │
├──────────────────────────────────────────────────────────────┤
│  • new(sc_data_dir: PathBuf) -> Self                         │
│  • import(&mut self, conn: &mut SqliteConnection) -> Result  │
│  • connect_to_arangodb() -> Result<Database>                 │
│  • import_for_language(&mut self, lang: &str) -> Result      │
├──────────────────────────────────────────────────────────────┤
│  Private Methods:                                             │
│  • get_titles(db, lang) -> HashMap<String, String>           │
│  • get_suttas(db, lang) -> HashMap<String, SuttaData>        │
│  • import_sutta_variants(conn, db, lang) -> Result           │
│  • import_sutta_comments(conn, db, lang) -> Result           │
│  • convert_paths_to_content(doc, sc_data_dir) -> Result      │
└──────────────────────────────────────────────────────────────┘
                         │
                         ▼
┌──────────────────────────────────────────────────────────────┐
│                  ArangoDB Database                            │
│                  (localhost:8529)                             │
├──────────────────────────────────────────────────────────────┤
│  Collections:                                                 │
│  • html_text - Legacy HTML format suttas                     │
│  • sc_bilara_texts - New Bilara JSON format                  │
│  • names - Sutta titles by language                          │
└──────────────────────────────────────────────────────────────┘
                         │
                         ▼
┌──────────────────────────────────────────────────────────────┐
│              appdata.sqlite3 Database                         │
├──────────────────────────────────────────────────────────────┤
│  Tables:                                                      │
│  • suttas - Main sutta content                               │
│  • sutta_variants - Alternative text versions                │
│  • sutta_comments - Commentary text                          │
└──────────────────────────────────────────────────────────────┘
```

### Data Flow

#### Phase 1: Get Titles from ArangoDB

```python
# Python: suttacentral.py lines 216-236
def get_titles(db: DBHandle, language = 'en') -> dict[str, str]:
    # For Pāli: Query 'names' collection where is_root == true
    # For English: Query 'names' collection where lang == 'en'
    # Returns: { "dn1": "The All-embracing Net of Views", ... }
```

**Purpose:** Build a lookup table of sutta UIDs to their translated titles.

**Rust Implementation Strategy:**
- Query ArangoDB 'names' collection
- Use AQL query with language filter
- Return HashMap<String, String> mapping uid → title

#### Phase 2: Get Suttas from ArangoDB

```python
# Python: suttacentral.py lines 282-423
def get_suttas(db, schema, sc_data_dir, language, limit) -> dict[str, USutta]:
    # Step 1: Query sc_bilara_texts for HTML templates (lang='pli', contains '_html')
    # Step 2: Query html_text collection for legacy HTML format
    # Step 3: Query sc_bilara_texts for Bilara JSON format
    # Step 4: Convert file paths to content
    # Step 5: Build sutta records with metadata
    # Returns: { "dn1/en/bodhi": Sutta(...), "mn1/pli/ms": Sutta(...), ... }
```

**Purpose:** Fetch all sutta content from ArangoDB and convert to database records.

**Processing Steps:**

1. **Collect HTML templates** (Bilara format only)
   - Query: `FOR x IN sc_bilara_texts FILTER x.lang == 'pli' && x._key LIKE '%_html'`
   - Store in: `HashMap<String, String>` (uid → template JSON)
   - These templates define the HTML structure for rendering Bilara JSON

2. **Query html_text collection**
   - Query: `FOR x IN html_text FILTER x.lang == @language`
   - Records have: uid, lang, author_uid, name (title), text (full HTML page)
   - UID format: `{uid}/{lang}/{author}` e.g. "dn1/en/bodhi"
   - Parse HTML to extract body content
   - Apply post-processing (consistent_niggahita, compact_rich_text)

3. **Query sc_bilara_texts collection**
   - Query: `FOR x IN sc_bilara_texts FILTER x.lang == @language`
   - Records have: uid, lang, muids (metadata UIDs), file_path, text (JSON)
   - UID format: `{uid}/{lang}/{author}` e.g. "mn1/pli/ms"
   - Text is JSON: `{"segment:id": "segment text", ...}`
   - Use HTML template (if available) to convert JSON → HTML
   - Title comes from titles HashMap

4. **Filter and deduplicate**
   - Ignore site pages (/site/), collections (/sutta/sa/, /sutta/ma/)
   - Ignore blurbs, name translations, comments (in this phase)
   - Prefer Bilara format over html_text when both exist
   - Keep only 'root' versions, skip 'reference' and 'variant' duplicates

5. **Build SuttaData records**
   - Extract metadata: uid, lang, author
   - Calculate: sutta_ref (e.g. "SN 12.23"), nikaya (e.g. "sn")
   - Apply text processing: consistent_niggahita, pali_to_ascii
   - Store content_html, content_plain, content_json, content_json_tmpl

**Rust Implementation Strategy:**
- Use AQL queries via arangors crate
- Process results in batches
- Handle file_path → content conversion
- Return Vec<SuttaData> for insertion

#### Phase 3: Import Sutta Variants

```python
# Python: suttacentral.py lines 449-538
def add_sutta_variants(appdata_db, schema, sc_db, sc_data_dir, language, limit):
    # Query: FOR x IN sc_bilara_texts WHERE lang == @language && 'variant' in x.muids
    # For each variant:
    #   1. Find parent sutta by uid in database
    #   2. Create SuttaVariant record linked to sutta_id
    #   3. Store JSON content only (no HTML rendering for variants)
```

**Purpose:** Import alternative text versions (manuscript variants) and link to parent suttas.

**Rust Implementation Strategy:**
- Query ArangoDB for variant records
- Look up sutta_id from database using uid
- Insert into sutta_variants table with foreign key

#### Phase 4: Import Sutta Comments

```python
# Python: suttacentral.py lines 540-629
def add_sutta_comments(appdata_db, schema, sc_db, sc_data_dir, language, limit):
    # Query: FOR x IN sc_bilara_texts WHERE lang == @language && 'comment' in x.muids
    # For each comment:
    #   1. Find parent sutta by uid in database
    #   2. Create SuttaComment record linked to sutta_id
    #   3. Store JSON content
```

**Purpose:** Import commentary text and link to parent suttas.

**Rust Implementation Strategy:**
- Similar to variants
- Query for records with 'comment' in muids
- Insert into sutta_comments table

### Helper Functions

#### UID Generation

```python
# html_text_uid(x) -> str
# Returns: "dn1/en/bodhi"
def html_text_uid(x):
    return f"{x['uid']}/{x['lang']}/{x['author_uid']}"

# bilara_text_uid(x) -> str  
# Returns: "dn1/pli/ms"
def bilara_text_uid(x):
    # Extract author from muids array
    # Remove 'translation', 'root', 'reference', 'variant', 'comment', 'html', lang
    # Remaining items = author(s)
    # Handle special cases: /pli/ms/ and /pli/vri/ in file_path
    # Multiple authors: join with "-"
```

**Purpose:** Generate unique identifiers for suttas combining uid/lang/author.

**Rust Implementation:**
- `html_text_uid(doc: &Document) -> String`
- `bilara_text_uid(doc: &Document) -> String`

#### Text Processing

```python
# From simsapa.app.helpers
consistent_niggahita(text: str) -> str    # Normalize niggahita character
pali_to_ascii(text: str) -> str           # Convert Pāli to ASCII for search
compact_rich_text(html: str) -> str       # Strip HTML, normalize whitespace
html_get_sutta_page_body(html: str) -> str # Extract <body> from full HTML page
bilara_html_post_process(html: str) -> str # Clean up Bilara HTML output
bilara_text_to_html(json: str, tmpl: str) -> str # Convert Bilara JSON → HTML using template
```

**Purpose:** Text normalization and format conversion.

**Rust Implementation:**
- Already exist in `simsapa_backend::helpers`
- Functions: `consistent_niggahita()`, `pali_to_ascii()`, `compact_rich_text()`
- Need to add: `bilara_text_to_html()`, `bilara_html_post_process()`, `html_get_sutta_page_body()`

#### Path Conversion

```python
# Python: suttacentral.py lines 425-446
def convert_paths_to_content(doc, sc_data_dir: Path):
    conversions = (
        ('file_path', 'text', lambda f: f.read()),
        ('markup_path', 'markup', lambda f: f.read()),
        ('strings_path', 'strings', json.load),
    )
    # For each conversion:
    #   1. Replace '/opt/sc/sc-flask/sc-data' with sc_data_dir
    #   2. Read file content
    #   3. Add to document as new property
```

**Purpose:** ArangoDB stores file paths, need to read actual content from disk.

**Rust Implementation:**
- Modify Document in-place
- Read files from sc-data directory
- Parse JSON for strings_path

#### Filtering Functions

```python
# _res_is_ignored(r: Dict) -> bool
# Ignore: /site/, /xplayground/, /sutta/sa/, /sutta/ma/, -blurbs_, -name_translation
# Ignore: 'comment' in muids (handled separately)
# Ignore: HTML templates (already collected)

# _uid_is_ignored(uid: str) -> bool  
# Ignore: ends with '/none', '-blurbs', '-name'
# Ignore: ends with '/than' or '/thanissaro' (use dhammatalks.org instead)
```

**Purpose:** Filter out non-sutta content and duplicates.

**Rust Implementation:**
- Helper functions for filtering
- Apply during sutta collection phase

### Database Schema

#### Suttas Table

```sql
CREATE TABLE suttas (
    id INTEGER PRIMARY KEY,
    uid TEXT NOT NULL,                    -- "dn1/en/bodhi"
    sutta_ref TEXT NOT NULL,              -- "DN 1"
    nikaya TEXT NOT NULL,                 -- "dn"
    language TEXT NOT NULL,               -- "en" or "pli"
    group_path TEXT,
    group_index INTEGER,
    order_index INTEGER,
    sutta_range_group TEXT,               -- For ranges like "sn12.1-10"
    sutta_range_start INTEGER,
    sutta_range_end INTEGER,
    title TEXT,                           -- "The All-embracing Net of Views"
    title_ascii TEXT,                     -- ASCII version for search
    title_pali TEXT,
    title_trans TEXT,
    description TEXT,
    content_plain TEXT,                   -- Plain text for search/indexing
    content_html TEXT,                    -- Rendered HTML (html_text format)
    content_json TEXT,                    -- Bilara JSON segments
    content_json_tmpl TEXT,               -- Bilara HTML template
    source_uid TEXT,                      -- "bodhi", "ms"
    source_info TEXT,
    source_language TEXT,
    message TEXT,
    copyright TEXT,
    license TEXT
);
```

#### SuttaVariant Table

```sql
CREATE TABLE sutta_variants (
    id INTEGER PRIMARY KEY,
    sutta_id INTEGER NOT NULL,            -- Foreign key to suttas.id
    sutta_uid TEXT NOT NULL,              -- Parent sutta uid
    language TEXT,
    source_uid TEXT,
    content_json TEXT,                    -- Variant text in JSON format
    FOREIGN KEY(sutta_id) REFERENCES suttas(id)
);
```

#### SuttaComment Table

```sql
CREATE TABLE sutta_comments (
    id INTEGER PRIMARY KEY,
    sutta_id INTEGER NOT NULL,            -- Foreign key to suttas.id
    sutta_uid TEXT NOT NULL,              -- Parent sutta uid
    language TEXT,
    source_uid TEXT,
    content_json TEXT,                    -- Comment text in JSON format
    FOREIGN KEY(sutta_id) REFERENCES suttas(id)
);
```

### Module Structure

```
cli/src/bootstrap/
├── suttacentral.rs          # Main implementation
└── helpers.rs               # Shared utilities (already exists)
```

### Function Summary

#### Main Functions

**`SuttaCentralImporter::new(sc_data_dir: PathBuf) -> Self`**
- Initialize importer with path to sc-data directory
- Store directory path for later file reading

**`SuttaCentralImporter::import(&mut self, conn: &mut SqliteConnection) -> Result<()>`**
- Implements SuttaImporter trait
- Orchestrates the full import process
- Calls import_for_language() for each language

**`connect_to_arangodb() -> Result<Database>`**
- Connect to ArangoDB at http://localhost:8529
- Username: "root", Password: "test"
- Database: "suttacentral"
- Return database handle for queries

**`import_for_language(&mut self, conn: &mut SqliteConnection, db: &Database, lang: &str, limit: Option<i32>) -> Result<()>`**
- Import all suttas for a single language
- Steps:
  1. Get titles from ArangoDB
  2. Get suttas from ArangoDB
  3. Insert suttas into database
  4. Import variants
  5. Import comments

#### Query Functions

**`get_titles(db: &Database, lang: &str) -> Result<HashMap<String, String>>`**
- Query ArangoDB 'names' collection
- For Pāli: `FILTER x.is_root == true`
- For other languages: `FILTER x.lang == @language`
- Return: `{ "dn1": "title", ... }`

**`get_suttas(db: &Database, titles: &HashMap<String, String>, sc_data_dir: &Path, lang: &str, limit: Option<i32>) -> Result<HashMap<String, SuttaData>>`**
- Query html_text and sc_bilara_texts collections
- Build SuttaData records from ArangoDB documents
- Apply filtering and deduplication
- Convert file paths to content
- Return: `{ "dn1/en/bodhi": SuttaData(...), ... }`

**`get_bilara_templates(db: &Database, sc_data_dir: &Path) -> Result<HashMap<String, String>>`**
- Query sc_bilara_texts for HTML templates
- Query: `FILTER x.lang == 'pli' && x._key LIKE '%_html'`
- Return: `{ "dn1": "template_json", ... }`

#### Import Functions

**`import_sutta_variants(conn: &mut SqliteConnection, db: &Database, sc_data_dir: &Path, lang: &str, limit: Option<i32>) -> Result<()>`**
- Query ArangoDB for variant records
- Query: `FILTER x.lang == @language && POSITION(x.muids, 'variant')`
- For each variant:
  - Get bilara_text_uid
  - Look up sutta_id in database
  - Insert NewSuttaVariant record

**`import_sutta_comments(conn: &mut SqliteConnection, db: &Database, sc_data_dir: &Path, lang: &str, limit: Option<i32>) -> Result<()>`**
- Query ArangoDB for comment records
- Query: `FILTER x.lang == @language && POSITION(x.muids, 'comment')`
- For each comment:
  - Get bilara_text_uid
  - Look up sutta_id in database
  - Insert NewSuttaComment record

#### Helper Functions

**`html_text_uid(doc: &Document) -> Result<String>`**
- Extract: uid, lang, author_uid from document
- Return: `"{uid}/{lang}/{author_uid}"`

**`bilara_text_uid(doc: &Document) -> Result<String>`**
- Extract: uid, lang, muids, file_path from document
- Determine author from muids or file_path
- Return: `"{uid}/{lang}/{author}"`

**`convert_paths_to_content(doc: &mut Document, sc_data_dir: &Path) -> Result<()>`**
- For each path field (file_path, markup_path, strings_path):
  - Replace `/opt/sc/sc-flask/sc-data` with `sc_data_dir`
  - Read file content
  - Add to document as text/markup/strings property

**`res_is_ignored(doc: &Document) -> bool`**
- Check if document should be ignored
- Returns true for: site pages, playgrounds, SA/MA collections, blurbs, name translations, comments, HTML templates

**`uid_is_ignored(uid: &str) -> bool`**
- Check if UID should be ignored
- Returns true for: /none author, blurbs, names, Thanissaro translations

**`html_text_to_sutta(doc: &Document, title: &str) -> Result<SuttaData>`**
- Convert html_text document to SuttaData
- Parse HTML to extract body
- Apply post-processing
- Generate metadata (sutta_ref, nikaya, etc.)

**`bilara_text_to_sutta(doc: &Document, title: &str, tmpl_json: Option<&str>) -> Result<SuttaData>`**
- Convert sc_bilara_texts document to SuttaData
- Use template to convert JSON → HTML (if template available)
- Generate plain text for indexing
- Generate metadata

### Dependencies

Add to `cli/Cargo.toml`:

```toml
[dependencies]
# Existing dependencies...

# ArangoDB client
arangors = "0.6"
arangors-lite = "0.6"  # Lighter version if needed

# Already have these (verify):
serde_json = "1.0"
scraper = "0.17"
regex = "1.10"
indicatif = "0.17"  # For progress bars
```

### Error Handling

- Use `anyhow::Result` for all functions
- Context on errors: `.context("Failed to query ArangoDB")`
- Log warnings for skipped records
- Continue processing on non-critical errors
- Fail fast on critical errors (DB connection, missing files)

## Implementation Plan

### Phase 1: ArangoDB Connection & Queries

1. Implement `connect_to_arangodb()`
2. Implement `get_titles()` with AQL query
3. Test connection and title retrieval
4. Add error handling for connection failures

### Phase 2: Helper Functions

1. Port text processing functions (if not already in backend)
   - `html_get_sutta_page_body()`
   - `bilara_html_post_process()`
   - `bilara_text_to_html()`
2. Implement UID generation functions
   - `html_text_uid()`
   - `bilara_text_uid()`
3. Implement filtering functions
   - `res_is_ignored()`
   - `uid_is_ignored()`
4. Implement `convert_paths_to_content()`

### Phase 3: Sutta Retrieval

1. Implement `get_bilara_templates()`
2. Implement `get_suttas()` with both html_text and sc_bilara_texts queries
3. Implement `html_text_to_sutta()`
4. Implement `bilara_text_to_sutta()`
5. Add deduplication logic
6. Test with small dataset (BOOTSTRAP_LIMIT=10)

### Phase 4: Database Import

1. Implement main import logic in `import_for_language()`
2. Batch insert suttas into database
3. Add progress bars with indicatif
4. Test full import for one language

### Phase 5: Variants & Comments

1. Implement `import_sutta_variants()`
2. Implement `import_sutta_comments()`
3. Test foreign key relationships
4. Verify records in database

### Phase 6: Integration

1. Create `SuttaCentralImporter` struct implementing `SuttaImporter` trait
2. Update `cli/src/bootstrap/mod.rs` to call importer
3. Remove TODO comment at line 160
4. Test full bootstrap with both languages

### Phase 7: Testing & Validation

1. Compare record counts with Python version
2. Spot-check sample suttas (DN 1, MN 1, SN 12.23)
3. Verify HTML rendering
4. Verify variants and comments are linked correctly
5. Test error cases (ArangoDB down, missing files)

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_html_text_uid() {
        // Test UID generation from html_text documents
    }

    #[test]
    fn test_bilara_text_uid() {
        // Test UID generation from bilara documents
        // Include edge cases: multiple authors, /pli/ms/, /pli/vri/
    }

    #[test]
    fn test_res_is_ignored() {
        // Test filtering logic
    }

    #[test]
    fn test_uid_is_ignored() {
        // Test UID filtering
    }

    #[test]
    fn test_convert_paths_to_content() {
        // Test path replacement and file reading
    }
}
```

### Integration Tests

1. **Test with small dataset**
   - Set `BOOTSTRAP_LIMIT=10`
   - Verify 10 suttas imported per language
   - Check variants and comments

2. **Test ArangoDB queries**
   - Verify title retrieval
   - Verify sutta retrieval
   - Verify variant retrieval
   - Verify comment retrieval

3. **Test database insertion**
   - Verify suttas table populated
   - Verify sutta_variants table populated
   - Verify sutta_comments table populated
   - Verify foreign key relationships

### Manual Testing

1. **Compare with Python output**
   ```bash
   # Python version
   cd simsapa-legacy/scripts
   python bootstrap_appdata.py
   sqlite3 ../../bootstrap-assets-resources/dist/appdata.sqlite3 "SELECT COUNT(*) FROM suttas WHERE language='en';"

   # Rust version
   cd cli
   cargo run --bin simsapa_cli bootstrap
   sqlite3 ../../bootstrap-assets-resources/dist/simsapa-ng/app-assets/appdata.sqlite3 "SELECT COUNT(*) FROM suttas WHERE language='en';"
   ```

2. **Spot-check sample suttas**
   - DN 1 (html_text format, English)
   - MN 1 (bilara format, Pāli)
   - SN 12.23 (with variants)
   - AN 4.10 (with comments)

3. **Verify HTML rendering**
   - Check that Bilara JSON + template → valid HTML
   - Check that html_text body extraction works
   - Check that consistent_niggahita is applied

## Data Flow Summary

### Overall Process

```
1. connect_to_arangodb() → Database handle
   ↓
2. For each language ['en', 'pli']:
   ↓
3. get_titles(db, lang) → HashMap<uid, title>
   ↓
4. get_bilara_templates(db) → HashMap<uid, template>
   ↓
5. get_suttas(db, titles, templates, lang) → HashMap<uid, SuttaData>
   ├─ Query html_text collection
   ├─ Query sc_bilara_texts collection  
   ├─ Filter ignored records
   ├─ Deduplicate (prefer Bilara)
   └─ Convert to SuttaData
   ↓
6. Insert suttas into database
   ↓
7. import_sutta_variants(conn, db, lang)
   ├─ Query sc_bilara_texts for variants
   ├─ Look up parent sutta_id
   └─ Insert NewSuttaVariant records
   ↓
8. import_sutta_comments(conn, db, lang)
   ├─ Query sc_bilara_texts for comments
   ├─ Look up parent sutta_id
   └─ Insert NewSuttaComment records
   ↓
9. Done for language
```

### Python Function Mapping

| Python Function | Purpose | Rust Implementation |
|----------------|---------|---------------------|
| `get_suttacentral_db()` | Connect to ArangoDB | `connect_to_arangodb()` |
| `html_text_uid(x)` | Generate UID for html_text | `html_text_uid(&Document)` |
| `bilara_text_uid(x)` | Generate UID for bilara | `bilara_text_uid(&Document)` |
| `html_text_to_sutta(x, ...)` | Convert html_text → Sutta | `html_text_to_sutta(&Document, ...)` |
| `bilara_text_to_sutta(x, ...)` | Convert bilara → Sutta | `bilara_text_to_sutta(&Document, ...)` |
| `get_titles(db, lang)` | Get title mapping | `get_titles(&Database, &str)` |
| `_res_is_ignored(r)` | Filter ignored records | `res_is_ignored(&Document)` |
| `_uid_is_ignored(uid)` | Filter ignored UIDs | `uid_is_ignored(&str)` |
| `get_suttas(db, ...)` | Get all suttas | `get_suttas(&Database, ...)` |
| `convert_paths_to_content(doc, ...)` | Read files from disk | `convert_paths_to_content(&mut Document, ...)` |
| `add_sutta_variants(...)` | Import variants | `import_sutta_variants(...)` |
| `add_sutta_comments(...)` | Import comments | `import_sutta_comments(...)` |
| `populate_suttas_from_suttacentral(...)` | Main entry point | `SuttaCentralImporter::import()` |

### Key Algorithms

#### 1. Title Lookup

```
Input: language (e.g., "en")
Process:
  1. Query ArangoDB names collection
  2. Filter by language (or is_root for Pāli)
  3. Build HashMap: uid → name
Output: HashMap<String, String>
```

#### 2. Sutta Deduplication

```
Priority order (highest to lowest):
  1. Bilara 'root' version (muids contains 'root')
  2. Bilara translation (not reference/variant)
  3. html_text version

When duplicate UID found:
  - If new record is 'root' → replace existing
  - If existing is Bilara and new is html_text → keep existing
  - If new is reference/variant → skip new
  - Otherwise → log warning as unknown duplicate
```

#### 3. Author Extraction (Bilara)

```
Input: document with muids and file_path
Process:
  1. Copy muids array
  2. Remove: 'translation', 'root', 'reference', 'variant', 'comment', 'html', language
  3. If 1 item remains → use as author
  4. If 0 items and path contains '/pli/ms/' → author = "ms"
  5. If 0 items and path contains '/pli/vri/' → author = "vri"
  6. If multiple items → join with "-" (e.g., "laera-quaresma")
Output: author string
```

#### 4. Bilara JSON → HTML Conversion

```
Input: 
  - content_json: {"segment:1": "text1", "segment:2": "text2", ...}
  - template_json: {"segment:1": "<p>{}</p>", "segment:2": "<p>{}</p>", ...}

Process:
  1. Parse both JSON objects
  2. For each segment in content:
     - Get template for segment
     - Replace {} with segment text
     - Append to output HTML
  3. Apply bilara_html_post_process()
  4. Wrap in <div class="suttacentral bilara-text">

Output: HTML string
```

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| ArangoDB connection failure | High - blocks all imports | Clear error message, document setup steps, check connection early |
| File path resolution issues | High - content missing | Robust path handling, log missing files, continue processing |
| Memory usage with large datasets | Medium | Use batch processing, stream results, consider pagination |
| UID generation edge cases | Medium | Comprehensive test cases, log warnings for unusual patterns |
| JSON parsing failures | Medium | Robust error handling, log problematic records, continue processing |
| Template missing for Bilara suttas | Low | Gracefully degrade to plain text, log warning |
| Duplicate UID handling | Low | Clear precedence rules, log decisions |

## Success Metrics

- [ ] Bootstrap imports suttas for 'en' and 'pli' languages without errors
- [ ] Database contains expected number of suttas (verify with Python baseline)
- [ ] Sutta variants correctly linked to parent suttas
- [ ] Sutta comments correctly linked to parent suttas
- [ ] Sample suttas match Python-generated content
- [ ] HTML rendering works correctly for both formats
- [ ] Progress bars show during import
- [ ] Code passes all unit tests
- [ ] Code follows existing Rust patterns in bootstrap module

## Future Enhancements

These are explicitly out of scope for this PRD but may be added later:

1. **Multi-refs from SuttaCentral** (`add_sc_multi_refs()`)
   - Query text_extra_info collection
   - Add PTS references to suttas
   - Link suttas via multi_refs relationship

2. **Per-language databases**
   - Generate suttas_lang_{lang}.sqlite3
   - One database per language for faster loading

3. **Additional languages**
   - Support languages beyond 'en' and 'pli'
   - Dynamic language list from ArangoDB

4. **Caching**
   - Cache ArangoDB queries
   - Speed up repeated bootstrap runs

5. **Incremental updates**
   - Only import changed suttas
   - Support for updating existing database

## References

- **Python implementation:** `simsapa-legacy/scripts/suttacentral.py`
- **Python bootstrap:** `simsapa-legacy/scripts/bootstrap_appdata.py`
- **Python helpers:** `simsapa-legacy/scripts/helpers.py`
- **Current Rust bootstrap:** `cli/src/bootstrap/mod.rs`
- **Existing importers:** `cli/src/bootstrap/nyanadipa.rs`, `cli/src/bootstrap/dhammatalks_org.rs`
- **Database models:** `backend/src/db/appdata_models.rs`
- **Text helpers:** `backend/src/helpers.rs`
- **PRD template:** `tasks/archive/prd-bootstrap-database-rust-implementation.md`
- **ArangoDB docs:** https://docs.rs/arangors/latest/arangors/
