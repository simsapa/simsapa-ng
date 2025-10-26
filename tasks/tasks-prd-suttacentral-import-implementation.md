# Task List: SuttaCentral Import Implementation

Based on PRD: `prd-suttacentral-import-implementation.md`

## Relevant Files

- `cli/src/bootstrap/suttacentral.rs` - Main SuttaCentral importer implementation with ArangoDB queries and data processing
- `cli/src/bootstrap/helpers.rs` - Shared helper functions for UID generation and text processing (already exists, may need extensions)
- `cli/src/bootstrap/mod.rs` - Bootstrap orchestrator, needs to instantiate and call SuttaCentralImporter
- `backend/src/helpers.rs` - Text processing functions (already has most needed functions)
- `backend/src/db/appdata_models.rs` - Database models for Sutta, SuttaVariant, SuttaComment (already exists)
- `backend/src/db/appdata_schema.rs` - Database schema definitions (already exists)
- `cli/Cargo.toml` - Dependencies file (already has arangors = "0.6.0")

### Notes

- ArangoDB dependency (`arangors = "0.6.0"`) is already present in `cli/Cargo.toml`
- Most text processing helpers already exist in `backend/src/helpers.rs`: `consistent_niggahita`, `pali_to_ascii`, `compact_rich_text`, `html_get_sutta_page_body`, `bilara_html_post_process`, `bilara_text_to_html`
- Other importers (NyanadipaImporter, DhammatalksSuttaImporter) provide good reference patterns
- Run bootstrap with: `cd cli && cargo run --bin simsapa_cli bootstrap`
- Test with small dataset: `BOOTSTRAP_LIMIT=10 cargo run --bin simsapa_cli bootstrap`
- Unit tests location: `cli/src/bootstrap/suttacentral.rs` (use `#[cfg(test)]` module)

## Tasks

- [ ] 1.0 Setup ArangoDB connection and implement title retrieval
  - [ ] 1.1 Create `connect_to_arangodb()` function in `cli/src/bootstrap/suttacentral.rs`
    - Connect to ArangoDB at `http://localhost:8529`
    - Use credentials: username="root", password="test"
    - Access database "suttacentral"
    - Return `Result<Database>` using arangors crate
    - Add error context: `.context("Failed to connect to ArangoDB")`
  - [ ] 1.2 Implement `get_titles()` function to query 'names' collection
    - Accept parameters: `db: &Database, lang: &str`
    - For Pāli (lang="pli"): Query WHERE `is_root == true`
    - For other languages: Query WHERE `lang == @language`
    - AQL query: `LET docs = (FOR x IN names FILTER [condition] RETURN x) RETURN docs`
    - Parse results into `HashMap<String, String>` mapping uid → title
    - Return `Result<HashMap<String, String>>`
  - [ ] 1.3 Add unit test for ArangoDB connection
    - Test `connect_to_arangodb()` can establish connection
    - Skip test if ArangoDB not running (use conditional compilation or test attribute)
    - Log clear error message if connection fails
  - [ ] 1.4 Add unit test for title retrieval
    - Test `get_titles()` returns expected format for 'en' language
    - Test `get_titles()` returns expected format for 'pli' language
    - Verify HashMap contains sample UIDs like "dn1", "mn1"

- [ ] 2.0 Implement helper functions for UID generation and filtering
  - [ ] 2.1 Implement `html_text_uid()` function
    - Accept parameter: `doc: &Document` (from ArangoDB)
    - Extract fields: `uid`, `lang`, `author_uid`
    - Return format: `"{uid}/{lang}/{author_uid}"` e.g. "dn1/en/bodhi"
    - Return `Result<String>` with error if fields missing
  - [ ] 2.2 Implement `bilara_text_uid()` function
    - Accept parameter: `doc: &Document`
    - Extract fields: `uid`, `lang`, `muids` (array), `file_path`
    - Remove from muids: 'translation', 'root', 'reference', 'variant', 'comment', 'html', language
    - If 1 item remains: use as author
    - If 0 items and path contains '/pli/ms/': author = "ms"
    - If 0 items and path contains '/pli/vri/': author = "vri"
    - If multiple items: join with "-" (e.g. "laera-quaresma")
    - Return format: `"{uid}/{lang}/{author}"`
    - Return `Result<String>` with error if UID cannot be determined
  - [ ] 2.3 Implement `res_is_ignored()` function
    - Accept parameter: `doc: &Document`
    - Return `true` if file_path contains: '/site/', '/xplayground/', '/sutta/sa/', '/sutta/ma/', '-blurbs_', '-name_translation'
    - Return `true` if muids contains 'comment'
    - Return `true` if HTML template (muids contains 'html' in bilara collection)
    - Return `false` otherwise
  - [ ] 2.4 Implement `uid_is_ignored()` function
    - Accept parameter: `uid: &str`
    - Return `true` if uid ends with: '/none', '-blurbs', '-name'
    - Return `true` if uid ends with: '/than', '/thanissaro' (use dhammatalks.org instead)
    - Return `false` otherwise
  - [ ] 2.5 Implement `convert_paths_to_content()` function
    - Accept parameters: `doc: &mut Document, sc_data_dir: &Path`
    - For 'file_path': Replace '/opt/sc/sc-flask/sc-data' with sc_data_dir, read file, add as 'text' field
    - For 'markup_path': Replace path, read file, add as 'markup' field
    - For 'strings_path': Replace path, read file, parse JSON, add as 'strings' field
    - Skip if field doesn't exist or is null
    - Log warning if file not found, continue processing
    - Return `Result<()>`
  - [ ] 2.6 Add unit tests for UID generation functions
    - Test `html_text_uid()` with sample document
    - Test `bilara_text_uid()` with single author
    - Test `bilara_text_uid()` with multiple authors (should join with "-")
    - Test `bilara_text_uid()` with /pli/ms/ path (author="ms")
    - Test `bilara_text_uid()` with /pli/vri/ path (author="vri")
  - [ ] 2.7 Add unit tests for filtering functions
    - Test `res_is_ignored()` returns true for site pages
    - Test `res_is_ignored()` returns true for comments
    - Test `res_is_ignored()` returns false for valid suttas
    - Test `uid_is_ignored()` returns true for '/none', '-blurbs', '/thanissaro'
    - Test `uid_is_ignored()` returns false for valid UIDs

- [ ] 3.0 Implement Bilara template collection
  - [ ] 3.1 Implement `get_bilara_templates()` function
    - Accept parameters: `db: &Database, sc_data_dir: &Path`
    - Query sc_bilara_texts collection: `FOR x IN sc_bilara_texts FILTER x.lang == 'pli' && x._key LIKE '%_html' RETURN x`
    - Filter results: only include if file_path contains 'sc_bilara_data/html' AND muids contains 'html'
    - For each result: call `convert_paths_to_content()` to read template JSON from disk
    - Build HashMap: uid → template_json (text field)
    - Return `Result<HashMap<String, String>>`
  - [ ] 3.2 Add unit test for template collection
    - Test query returns expected format
    - Test templates are properly keyed by uid
    - Verify sample template like "dn1" exists (if ArangoDB available)

- [ ] 4.0 Implement sutta retrieval from ArangoDB (both html_text and sc_bilara_texts)
  - [ ] 4.1 Implement `html_text_to_sutta()` function
    - Accept parameters: `doc: &Document, title: &str`
    - Extract uid using `html_text_uid(doc)`
    - Extract author as last component of uid (split by '/')
    - Parse HTML from 'text' field using `html_get_sutta_page_body()`
    - Apply post-processing: `bilara_html_post_process()`, `consistent_niggahita()`
    - Wrap in `<div class="suttacentral html-text">...</div>`
    - Generate content_plain using `compact_rich_text()`
    - Calculate sutta_ref using `uid_to_ref()` helper
    - Calculate nikaya using `uid_to_nikaya()` helper
    - Apply `pali_to_ascii()` to title for title_ascii
    - Build and return `SuttaData` struct
  - [ ] 4.2 Implement `bilara_text_to_sutta()` function
    - Accept parameters: `doc: &Document, title: &str, tmpl_json: Option<&str>`
    - Extract uid using `bilara_text_uid(doc)`
    - Extract author as last component of uid
    - Get JSON content from 'text' field
    - If template available: use `bilara_text_to_html()` to convert JSON → HTML
    - If template not available: parse JSON and join values with "\n\n", log warning
    - For Bilara: set content_html to NULL (not saved to reduce DB size per Python version)
    - Store JSON content in content_json field
    - Store template in content_json_tmpl field
    - Generate content_plain using `compact_rich_text()` (from HTML or plain text)
    - Apply text processing: `consistent_niggahita()`, `pali_to_ascii()`
    - Calculate sutta_ref and nikaya
    - Build and return `SuttaData` struct
  - [ ] 4.3 Implement `get_suttas()` function - main query logic
    - Accept parameters: `db: &Database, titles: &HashMap<String, String>, templates: &HashMap<String, String>, sc_data_dir: &Path, lang: &str, limit: Option<i32>`
    - Initialize HashMap to collect suttas: `HashMap<String, SuttaData>`
    - Initialize counters: total_results, ignored, known_dup, unknown_dup
    - Query 1: html_text collection: `FOR x IN html_text FILTER x.lang == @language RETURN x`
    - Query 2: sc_bilara_texts collection: `FOR x IN sc_bilara_texts FILTER x.lang == @language RETURN x`
    - Apply limit if provided: take first N results from each query
    - For each result from both queries:
      - Call `convert_paths_to_content()` to read file content
      - Check `res_is_ignored()` - skip if true, increment ignored counter
      - Generate uid using appropriate function (html_text_uid or bilara_text_uid)
      - Check `uid_is_ignored()` - skip if true, increment ignored counter
      - Get title from titles HashMap
      - Get template from templates HashMap (for bilara only)
      - If uid not in suttas HashMap: add using html_text_to_sutta() or bilara_text_to_sutta()
      - If uid exists: apply deduplication logic (see 4.4)
    - Log summary: total results, ignored, duplicates
    - Return `Result<HashMap<String, SuttaData>>`
  - [ ] 4.4 Implement deduplication logic within `get_suttas()`
    - When duplicate UID found, check muids:
    - If new record has 'reference' or 'variant' in muids: skip (known_dup++), keep existing
    - If new record has 'root' in muids: replace existing (known_dup++), prefer root version
    - If new record is from sc_bilara_texts and existing is from html_text: replace (known_dup++), prefer Bilara
    - Otherwise: log warning as unknown duplicate (unknown_dup++), keep existing
  - [ ] 4.5 Add unit tests for sutta conversion functions
    - Test `html_text_to_sutta()` with sample html_text document
    - Test `bilara_text_to_sutta()` with template
    - Test `bilara_text_to_sutta()` without template (should still work)
    - Verify SuttaData fields are properly populated
    - Verify text processing functions are applied

- [ ] 5.0 Implement sutta variants and comments import
  - [ ] 5.1 Implement `import_sutta_variants()` function
    - Accept parameters: `conn: &mut SqliteConnection, db: &Database, sc_data_dir: &Path, lang: &str, limit: Option<i32>`
    - Query sc_bilara_texts: `FOR x IN sc_bilara_texts FILTER x.lang == @language && POSITION(x.muids, 'variant') RETURN x`
    - Apply limit if provided
    - Initialize progress bar with indicatif
    - For each variant record:
      - Call `convert_paths_to_content()` to read JSON
      - Check `res_is_ignored()` - skip if true
      - Get sutta_uid using `bilara_text_uid()`
      - Check `uid_is_ignored()` - skip if true
      - Query database: `SELECT id FROM suttas WHERE uid = ?` to get sutta_id
      - If sutta not found: log error, continue
      - Extract source_uid (last component of sutta_uid)
      - Apply `consistent_niggahita()` to JSON content
      - Create NewSuttaVariant record with: sutta_id, sutta_uid, language, source_uid, content_json
      - Insert into database using diesel
    - Log summary: "{count} sutta variants imported"
    - Return `Result<()>`
  - [ ] 5.2 Implement `import_sutta_comments()` function
    - Accept parameters: same as import_sutta_variants
    - Query sc_bilara_texts: `FOR x IN sc_bilara_texts FILTER x.lang == @language && POSITION(x.muids, 'comment') RETURN x`
    - Apply same logic as variants but insert into sutta_comments table
    - Use NewSuttaComment instead of NewSuttaVariant
    - Log summary: "{count} sutta comments imported"
    - Return `Result<()>`
  - [ ] 5.3 Add unit tests for variants and comments
    - Test foreign key lookup finds correct sutta_id
    - Test NewSuttaVariant record creation
    - Test NewSuttaComment record creation
    - Test error handling when parent sutta not found

- [ ] 6.0 Integrate SuttaCentralImporter with bootstrap process
  - [ ] 6.1 Create `SuttaCentralImporter` struct
    - Add to `cli/src/bootstrap/suttacentral.rs`
    - Fields: `sc_data_dir: PathBuf`
    - Implement `new(sc_data_dir: PathBuf) -> Self`
  - [ ] 6.2 Implement `SuttaImporter` trait for `SuttaCentralImporter`
    - Implement `import(&mut self, conn: &mut SqliteConnection) -> Result<()>`
    - Connect to ArangoDB using `connect_to_arangodb()`
    - For each language in ['en', 'pli']: call `import_for_language()`
    - Handle errors gracefully, log progress
  - [ ] 6.3 Implement `import_for_language()` private method
    - Accept parameters: `&mut self, conn: &mut SqliteConnection, db: &Database, lang: &str, limit: Option<i32>`
    - Step 1: Get titles using `get_titles(db, lang)`
    - Step 2: Get templates using `get_bilara_templates(db, &self.sc_data_dir)`
    - Step 3: Get suttas using `get_suttas(db, &titles, &templates, &self.sc_data_dir, lang, limit)`
    - Step 4: Insert suttas into database
      - Initialize progress bar
      - For each sutta in HashMap: convert to NewSutta, insert with diesel
      - Log: "Adding {lang}, count {len} ..."
    - Step 5: Import variants using `import_sutta_variants(conn, db, &self.sc_data_dir, lang, limit)`
    - Step 6: Import comments using `import_sutta_comments(conn, db, &self.sc_data_dir, lang, limit)`
    - Log: "DONE: {lang}"
    - Return `Result<()>`
  - [ ] 6.4 Update `cli/src/bootstrap/mod.rs` to use SuttaCentralImporter
    - Import SuttaCentralImporter at top of file
    - Find TODO comment at line 160: "TODO: Import suttas from SuttaCentral for lang 'en' and 'pli'"
    - Replace TODO with implementation block:
      ```rust
      {
          let sc_data_dir = bootstrap_assets_dir.join("sc-data");
          if sc_data_dir.exists() {
              tracing::info!("Importing suttas from SuttaCentral");
              let mut importer = SuttaCentralImporter::new(sc_data_dir);
              importer.import(&mut conn)?;
          } else {
              tracing::warn!("SuttaCentral data directory not found, skipping");
          }
      }
      ```
    - Ensure proper error handling
  - [ ] 6.5 Verify bootstrap process compiles and runs
    - Run `cd cli && cargo check` to verify compilation
    - Fix any compilation errors
    - Run with BOOTSTRAP_LIMIT=10 to test with small dataset

- [ ] 7.0 Testing and validation
  - [ ] 7.1 Test with small dataset (BOOTSTRAP_LIMIT=10)
    - Set environment variable: `BOOTSTRAP_LIMIT=10`
    - Run bootstrap: `cd cli && cargo run --bin simsapa_cli bootstrap`
    - Verify ArangoDB connection succeeds
    - Verify titles are retrieved
    - Verify suttas are imported (should see ~10 per language)
    - Verify variants are imported
    - Verify comments are imported
    - Check for any errors or warnings in logs
  - [ ] 7.2 Verify database records
    - Query suttas table: `SELECT COUNT(*) FROM suttas WHERE language='en'`
    - Query suttas table: `SELECT COUNT(*) FROM suttas WHERE language='pli'`
    - Query sutta_variants table: `SELECT COUNT(*) FROM sutta_variants`
    - Query sutta_comments table: `SELECT COUNT(*) FROM sutta_comments`
    - Verify counts are reasonable
  - [ ] 7.3 Spot-check sample suttas
    - Query DN 1 (English): `SELECT uid, title, substr(content_html, 1, 100) FROM suttas WHERE uid LIKE 'dn1/%' AND language='en'`
    - Query MN 1 (Pāli): `SELECT uid, title, content_json IS NOT NULL FROM suttas WHERE uid LIKE 'mn1/%' AND language='pli'`
    - Verify title is not empty
    - Verify content exists (HTML or JSON)
    - Check for proper text processing (niggahita, ASCII)
  - [ ] 7.4 Test full import (remove BOOTSTRAP_LIMIT)
    - Unset BOOTSTRAP_LIMIT or set to empty
    - Run full bootstrap (this will take several minutes)
    - Monitor memory usage
    - Verify import completes without errors
    - Compare record counts with Python baseline (if available)
  - [ ] 7.5 Test error cases
    - Test with ArangoDB not running: should fail gracefully with clear error message
    - Test with missing sc-data directory: should skip with warning
    - Test with corrupted JSON files: should log error and continue
    - Verify error messages are helpful for debugging
  - [ ] 7.6 Document any known issues or limitations
    - Note in comments if certain edge cases are not handled
    - Document differences from Python implementation (if any)
    - Add TODO comments for future enhancements (e.g., add_sc_multi_refs)
