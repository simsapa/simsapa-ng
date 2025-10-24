# PRD: Database Bootstrap Procedure - Rust Implementation

## Overview

Port the database bootstrap procedure from Python (simsapa-legacy/scripts/bootstrap.py) to Rust in the CLI module. The bootstrap process generates the application databases (appdata.sqlite3, dpd.sqlite3, dictionaries.sqlite3) from scratch using source data in bootstrap-assets-resources/.

## Background

### Current State
- Bootstrap partially implemented in `cli/src/bootstrap.rs`
- Currently copies pre-built appdata.sqlite3 from bootstrap-assets-resources/
- DPD migration already working (dpd.db → dpd.sqlite3)
- StarDict import already working

### Target State
- Generate appdata.sqlite3 from scratch like the Python version
- Modular Rust code with separate modules for each data source
- Skip unavailable resource folders (with TODO comments)
- Skip Tantivy fulltext indexing (not yet integrated)
- Connect to ArangoDB for SuttaCentral integration

## Goals

1. **Generate databases from scratch** - Don't copy pre-built appdata.sqlite3
2. **Modular architecture** - Separate Rust module for each import source
3. **Maintain compatibility** - Generate same database schema as Python version
4. **Resource-aware** - Only implement stages for available resources
5. **Future-ready** - Add TODO comments for missing resources

## Non-Goals

- Tantivy fulltext indexing (not integrated yet)
- Creating tar.bz2 release archives (can be added later)
- User data import (bookmarks, prompts - removed tables)
- Sanskrit texts (resource folder not available)

## Success Criteria

1. Bootstrap script generates valid appdata.sqlite3 from scratch
2. Database contains suttas from all available sources
3. Database contains dictionaries from all available sources
4. Completions data generated and saved to app_settings
5. Code organized in modular structure matching Python scripts
6. All TODO comments added for unavailable resources

## Technical Design

### Module Structure

Create modular structure in `cli/src/bootstrap/`:

```
cli/src/bootstrap/
├── mod.rs                    # Main orchestration & utilities
├── appdata.rs               # Main appdata bootstrap coordination
├── suttacentral.rs          # SuttaCentral import (Bilara JSON + ArangoDB)
├── dhammatalks_org.rs       # Dhammatalks.org sutta import
├── dhammapada_munindo.rs    # Dhammapada Munindo import
├── dhammapada_tipitaka.rs   # Dhammapada Tipitaka.net import
├── nyanadipa.rs             # Nyanadipa translations import
├── buddha_ujja.rs           # Hungarian Buddha Ujja import
└── completions.rs           # Autocomplete data generation
```

Note: No `dictionaries.rs` module needed - DPD import already implemented in main `bootstrap.rs`.

### Database Schema

#### Existing Tables (appdata.sqlite3)
- ✅ `app_settings` - Already exists
- ✅ `suttas` - Already exists
- ✅ `sutta_variants` - Already exists
- ✅ `sutta_comments` - Already exists
- ✅ `sutta_glosses` - Already exists

#### Existing Tables (dictionaries.sqlite3)
- ✅ `dictionaries` - Already exists (dictionaries_models.rs)
- ✅ `dict_words` - Already exists (dictionaries_models.rs)

#### Tables to Skip
- ❌ `bookmarks` - User data, not needed in bootstrap
- ❌ `prompts` - User data, not needed in bootstrap
- ❌ `sutta_links` - TODO for later (create_links.py)
- ❌ `multi_refs` - TODO for later (multi_refs.py, needs pts-refs/ folder)

### Bootstrap Stages

#### Stage 1: Setup & Initialization ✅
**Status:** Already implemented in `cli/src/bootstrap.rs`

- Clean and create folders (dist/, release/, simsapa-ng/)
- Write .env file (optional)
- Clear log.txt

#### Stage 2: Appdata Database Creation 🔄
**Status:** Needs implementation

**2.1 Create Empty Appdata Database**
- Location: `bootstrap/appdata.rs`
- Create new appdata.sqlite3 using Diesel migrations
- Insert db_version into app_settings table
- Function: `create_appdata_db() -> Result<Connection>`

**2.2 Import Dictionaries** ✅
**Status:** Already complete - no additional work needed
- DPD StarDict import already implemented in `cli/src/bootstrap.rs`
- DPD database migration already implemented
- Future dictionaries (Nyanatiloka, DPPN, etc.) can be added when resources become available

**2.3 Import Suttas to Appdata**
- Implement separate module for each source:

**2.3.1 SuttaCentral (en, pli)** ✅
- Location: `bootstrap/suttacentral.rs`
- Resource: `bootstrap-assets-resources/sc-data/`
- Dependencies: ArangoDB connection at `http://localhost:8529`
- Based on: `simsapa-legacy/scripts/suttacentral.py`
- Functions:
  - `connect_to_arangodb() -> Result<Database>`
  - `populate_suttas_from_suttacentral(db: &Connection, sc_db: Database, lang: &str, limit: Option<i32>) -> Result<()>`
  - `parse_bilara_json()` - Parse bilara JSON files
  - `parse_html_text()` - Parse HTML text format
- ArangoDB crate: Use `arangors` or `aragog`

**2.3.2 Dhammatalks.org** ✅
- Location: `bootstrap/dhammatalks_org.rs`
- Resource: `bootstrap-assets-resources/dhammatalks-org/`
- Based on: `simsapa-legacy/scripts/dhammatalks_org.py`
- Parse HTML files, extract sutta content
- Convert internal links to ssp:// format

**2.3.3 Dhammapada Munindo** ✅
- Location: `bootstrap/dhammapada_munindo.rs`
- Resource: `bootstrap-assets-resources/dhammapada-munindo/`
- Based on: `simsapa-legacy/scripts/dhammapada_munindo.py`

**2.3.4 Dhammapada Tipitaka.net** ✅
- Location: `bootstrap/dhammapada_tipitaka.rs`
- Resource: `bootstrap-assets-resources/dhammapada-tipitaka-net/`
- Based on: `simsapa-legacy/scripts/dhammapada_tipitaka_net.py`

**2.3.5 Nyanadipa Translations** ✅
- Location: `bootstrap/nyanadipa.rs`
- Resource: `bootstrap-assets-resources/nyanadipa-translations/`
- Based on: `simsapa-legacy/scripts/nyanadipa.py`

**2.3.6 CST4 (Pali Tipitaka)** ❌
- TODO: Resource folder `cst4/` not available
- Based on: `simsapa-legacy/scripts/cst4.py`
- Add TODO comment in `bootstrap/appdata.rs`

**2.4 Generate Completions Data**
- Location: `bootstrap/completions.rs`
- Based on: `simsapa-legacy/simsapa/app/completion_lists.py`
- Generate autocomplete word lists from:
  - Sutta titles (from appdata.suttas)
  - Dictionary words (from dictionaries.dict_words + dpd.DpdHeadwords.lemma_1 + dpd.DpdRoots.root_no_sign + dpd.Lookup.lookup_key)
- Parse to sublists by first 3 ASCII letters
- Save as JSON in app_settings:
  - Key: `sutta_titles_completions`
  - Key: `dict_words_completions`
- Functions:
  - `get_sutta_titles_completion_list(appdata_db: &Connection, dpd_db: &Connection, dict_db: &Connection) -> Result<HashMap<String, Vec<String>>>`
  - `get_dict_words_completion_list(appdata_db: &Connection, dpd_db: &Connection, dict_db: &Connection) -> Result<HashMap<String, Vec<String>>>`
  - `parse_to_sublists(items: Vec<String>) -> HashMap<String, Vec<String>>`
  - `save_completions(db: &Connection, key: &str, sublists: HashMap<String, Vec<String>>) -> Result<()>`

**2.5 Post-Processing** ❌
- TODO: Skip for now, add comments
- SuttaCentral multi-refs (requires ArangoDB integration)
- Populate sutta multi-refs (requires pts-refs/ folder)
- Create internal ssp:// links (requires sutta_links table)

#### Stage 3: DPD Database Migration ✅
**Status:** Already implemented

- Function: `dpd_migrate()` in `cli/src/bootstrap.rs`
- Uses: `simsapa_backend::db::dpd::import_migrate_dpd()`

#### Stage 4: Create appdata.tar.bz2 Archive ❌
**Status:** Skip for now (can add later)

- TODO: Package dpd.sqlite3 + appdata.sqlite3 + dictionaries.sqlite3
- Move to release directory

#### Stage 5: Per-Language Sutta Databases ❌
**Status:** Skip for now (requires SuttaCentral integration)

- TODO: Generate suttas_lang_{lang}.sqlite3 for each language
- Requires both sc-data/ folder AND ArangoDB
- Based on: `simsapa-legacy/scripts/bootstrap_suttas_lang.py`

#### Stage 6: Hungarian Buddha Ujja ✅
**Status:** Needs implementation

- Location: `bootstrap/buddha_ujja.rs`
- Resource: `bootstrap-assets-resources/buddha-ujja-sql/`
- Based on: `simsapa-legacy/scripts/buddha_ujja.py`
- Import Hungarian suttas from SQL files

#### Stage 7: Sanskrit Texts ❌
**Status:** Skip - resource not available

- TODO: Resource folder `sanskrit/gretil/` not available
- Based on: `simsapa-legacy/scripts/sanskrit_texts.py`

### Helper Functions & Utilities

Create shared utilities in `bootstrap/mod.rs`:

```rust
// Database connection helpers
pub fn create_appdata_db(db_path: &Path) -> Result<Connection>;
pub fn get_db_connection(db_path: &Path) -> Result<Connection>;

// Sutta helpers (port from Python helpers.py)
pub fn uid_to_ref(uid: &str) -> String;
pub fn uid_to_nikaya(uid: &str) -> String;
pub fn sutta_range_from_ref(sutta_ref: &str) -> Option<SuttaRange>;
pub fn consistent_niggahita(text: &str) -> String;
pub fn pali_to_ascii(text: &str) -> String;
pub fn compact_rich_text(html: &str) -> String;

// HTML parsing helpers
pub fn html_get_sutta_page_body(html: &str) -> String;
pub fn bilara_html_post_process(html: &str) -> String;
pub fn bilara_text_to_html(json: &str) -> String;

// Database version
pub fn insert_db_version(db: &Connection) -> Result<()>;
pub fn get_db_version_from_cargo_toml() -> Result<String>;
```

### Dependencies

Add to `cli/Cargo.toml`:

```toml
[dependencies]
# Existing dependencies...

# ArangoDB client
arangors = "0.6"  # or aragog = "0.18"

# HTML parsing
scraper = "0.17"
html5ever = "0.26"

# JSON processing (already have serde)
serde_json = "1.0"

# Text processing
regex = "1.10"
unicode-normalization = "0.1"

# Archive creation (for later tar.bz2 stage)
# tar = "0.4"
# bzip2 = "0.4"
```

## Implementation Plan

### Phase 1: Core Infrastructure
1. Create module structure in `cli/src/bootstrap/`
2. Implement helper functions in `bootstrap/mod.rs`
3. Update `bootstrap.rs` to use new module structure
4. Implement `create_appdata_db()` function

### Phase 2: Dictionary Imports ✅
**Status:** Already complete - DPD StarDict import implemented in current `cli/src/bootstrap.rs`

No additional dictionary imports needed for now. Future dictionary sources (Nyanatiloka, DPPN, etc.) can be added when resources become available.

### Phase 3: Sutta Imports (Priority Order)
1. Implement `bootstrap/dhammatalks_org.rs` (simpler, no external deps)
2. Implement `bootstrap/dhammapada_munindo.rs`
3. Implement `bootstrap/dhammapada_tipitaka.rs`
4. Implement `bootstrap/nyanadipa.rs`
5. Implement `bootstrap/suttacentral.rs` (complex, ArangoDB)
6. Implement `bootstrap/buddha_ujja.rs`

### Phase 4: Post-Processing
1. Implement `bootstrap/completions.rs`
2. Add TODO comments for links and multi-refs

### Phase 5: Main Orchestration
1. Update `bootstrap/appdata.rs` to coordinate all imports
2. Update `cli/src/bootstrap.rs` main function
3. Add logging and progress tracking

### Phase 6: Testing & Validation
1. Test each module independently
2. Full bootstrap run and database validation
3. Compare with Python-generated database

## Data Flow

```
bootstrap-assets-resources/
├── sc-data/                    → suttacentral.rs → suttas table
├── dhammatalks-org/           → dhammatalks_org.rs → suttas table
├── dhammapada-munindo/        → dhammapada_munindo.rs → suttas table
├── dhammapada-tipitaka-net/   → dhammapada_tipitaka.rs → suttas table
├── nyanadipa-translations/    → nyanadipa.rs → suttas table
├── buddha-ujja-sql/           → buddha_ujja.rs → suttas table
└── dpd-db-for-bootstrap/      → dpd_migrate() → dpd.sqlite3

All suttas + dict_words → completions.rs → app_settings (JSON)
```

## Migration Notes

### Key Differences from Python Version

1. **Database Location During Bootstrap:**
   - Python: Creates in `dist/` folder
   - Rust: Creates in `dist/simsapa-ng/app-assets/` (via SIMSAPA_DIR env var)

2. **DPD Storage:**
   - Python: DPD definitions in appdata.sqlite3 (dict_words table)
   - Rust: DPD StarDict → dictionaries.sqlite3, DPD database → dpd.sqlite3

3. **Completions Storage:**
   - Both: JSON in app_settings table
   - Format: `{"abc": ["word1", "word2"], "def": [...]}`

4. **Error Handling:**
   - Python: Exceptions and exit(1)
   - Rust: anyhow::Result with proper error propagation

### Schema Compatibility

Ensure Rust-generated tables match Python schema:

```sql
-- app_settings
CREATE TABLE app_settings (
    id INTEGER PRIMARY KEY,
    key TEXT NOT NULL,
    value TEXT
);

-- suttas (appdata)
CREATE TABLE suttas (
    id INTEGER PRIMARY KEY,
    uid TEXT NOT NULL,
    sutta_ref TEXT NOT NULL,
    nikaya TEXT NOT NULL,
    language TEXT NOT NULL,
    group_path TEXT,
    group_index INTEGER,
    order_index INTEGER,
    sutta_range_group TEXT,
    sutta_range_start INTEGER,
    sutta_range_end INTEGER,
    title TEXT,
    title_ascii TEXT,
    title_pali TEXT,
    title_trans TEXT,
    description TEXT,
    content_plain TEXT,
    content_html TEXT,
    content_json TEXT,
    content_json_tmpl TEXT,
    source_uid TEXT,
    source_info TEXT,
    source_language TEXT,
    message TEXT,
    copyright TEXT,
    license TEXT
);
```

## TODO Comments to Add

Add these TODO comments in appropriate locations:

```rust
// bootstrap/appdata.rs
// TODO: Import CST4 (Pali Tipitaka) - requires bootstrap-assets-resources/cst4/

// Future dictionary imports (when resources become available):
// TODO: Import Nyanatiloka dictionary - requires nyanatiloka-palikanon-com-fixed/
// TODO: Import DPPN dictionary - requires dppn/ folder
// TODO: Import StarDict ZIP files - requires dict/ folder with language subdirs

// bootstrap/appdata.rs (post-processing section)
// TODO: Add SuttaCentral multi-refs (requires full ArangoDB integration)
// TODO: Populate sutta multi-refs (requires bootstrap-assets-resources/pts-refs/)
// TODO: Create internal ssp:// links (requires sutta_links table implementation)

// bootstrap/mod.rs (at end of main bootstrap function)
// TODO: Generate per-language sutta databases (suttas_lang_{lang}.sqlite3)
// TODO: Import Sanskrit texts from GRETIL - requires sanskrit/gretil/ folder
// TODO: Create tar.bz2 release archives (appdata.tar.bz2, etc.)
```

## Testing Strategy

1. **Unit Tests:**
   - Test helper functions (uid_to_ref, pali_to_ascii, etc.)
   - Test HTML parsing functions
   - Test completion list generation

2. **Integration Tests:**
   - Test each import module independently
   - Verify database schema matches expected structure
   - Validate data integrity (foreign keys, etc.)

3. **End-to-End Test:**
   - Full bootstrap run
   - Compare record counts with Python version
   - Spot-check sample suttas and dictionary entries
   - Validate completions JSON structure

4. **Performance:**
   - Monitor memory usage during import
   - Track import speed (records/second)
   - Consider batch inserts for large datasets

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| ArangoDB connection failure | High | Clear error messages, check connection early |
| HTML parsing variations | Medium | Robust error handling, log parsing failures |
| Memory usage with large imports | Medium | Use batch inserts, stream processing |
| Schema incompatibility | High | Validate schema before importing data |
| Missing helper functions | Medium | Port incrementally, test each function |

## Success Metrics

- [ ] Bootstrap completes without errors
- [ ] appdata.sqlite3 contains >1000 suttas (from available sources)
- [ ] dictionaries.sqlite3 contains DPD entries
- [ ] Completions JSON generated for suttas and dict_words
- [ ] Database schema matches Python version
- [ ] All TODO comments added for unavailable resources
- [ ] Code organized in modular structure

## References

- Python bootstrap: `simsapa-legacy/scripts/bootstrap.py`
- Python appdata bootstrap: `simsapa-legacy/scripts/bootstrap_appdata.py`
- Current Rust bootstrap: `cli/src/bootstrap.rs`
- Database models: `backend/src/db/appdata_models.rs`
- Completions logic: `simsapa-legacy/simsapa/app/completion_lists.py`
