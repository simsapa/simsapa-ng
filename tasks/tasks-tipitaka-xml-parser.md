# Tasks: Tipitaka XML Parser with Fragment-Based Architecture

## Relevant Files

- `cli/src/tipitaka_xml_parser/mod.rs` - Module exports for the new XML parser
- `cli/src/tipitaka_xml_parser/types.rs` - Core data structures (XmlFragment, GroupLevel, GroupType, NikayaStructure, FragmentType)
- `cli/src/tipitaka_xml_parser/nikaya_detector.rs` - Detects nikaya from XML content and returns structure configuration
- `cli/src/tipitaka_xml_parser/nikaya_structure.rs` - Hard-coded nikaya hierarchy configurations and normalization
- `cli/src/tipitaka_xml_parser/fragment_parser.rs` - Parses XML into fragments with line tracking (uses NikayaStructure)
- `cli/src/tipitaka_xml_parser/sutta_builder.rs` - Assembles sutta database records from fragments
- `cli/src/tipitaka_xml_parser/database_inserter.rs` - Inserts suttas into appdata database
- `cli/src/tipitaka_xml_parser/integration.rs` - High-level processing API and statistics
- `cli/src/main.rs` - CLI command integration (add ParseTipitakaXml command)
- `cli/tests/tipitaka_xml_parser/test_nikaya_detection.rs` - Unit tests for nikaya detection
- `cli/tests/tipitaka_xml_parser/test_fragment_parser.rs` - Unit tests for fragment parsing
- `cli/tests/tipitaka_xml_parser/test_sutta_builder.rs` - Unit tests for sutta assembly
- `cli/tests/tipitaka_xml_parser/test_integration.rs` - End-to-end integration tests

### Notes

- Tests should be placed in `cli/tests/tipitaka_xml_parser/` directory
- Use `cd backend && cargo test` to run all backend tests
- Use `cd backend && cargo test test_name` to run a specific test
- The existing `tipitaka_xml_parser_tsv` module provides reference implementation patterns
- Reuse existing modules: `simsapa_backend::helpers::consistent_niggahita`, database schema from `simsapa_backend::db::appdata_schema`

## Tasks

### 1.0 Create module structure and implement core data types
**Estimated Time:** 4-6 hours
**Status:** COMPLETED

- [x] 1.1 Create module structure
  - [x] 1.1.1 Create `cli/src/tipitaka_xml_parser/` directory
  - [x] 1.1.2 Create `cli/src/tipitaka_xml_parser/mod.rs` with module exports
  - [x] 1.1.3 Verify module compiles with `cd backend && cargo build`

- [x] 1.2 Implement core type definitions in `types.rs`
  - [x] 1.2.1 Create `cli/src/tipitaka_xml_parser/types.rs`
  - [x] 1.2.2 Implement `FragmentType` enum (Header, Sutta)
  - [x] 1.2.3 Implement `GroupType` enum (Nikaya, Book, Vagga, Samyutta, Sutta)
  - [x] 1.2.4 Implement `GroupLevel` struct with fields: group_type, group_number, title, id
  - [x] 1.2.5 Implement `XmlFragment` struct with fields: fragment_type, content, start_line, end_line, group_levels
  - [x] 1.2.6 Add serde Serialize/Deserialize derives to all types
  - [x] 1.2.7 Add Debug and Clone derives to all types
  - [x] 1.2.8 Export all types from `mod.rs`
  - [x] 1.2.9 Verify compilation with `cd backend && cargo build`

- [x] 1.3 Add required dependencies to `cli/Cargo.toml`
  - [x] 1.3.1 Add `quick-xml` for XML parsing
  - [x] 1.3.2 Add `serde` and `serde_json` for serialization
  - [x] 1.3.3 Add `anyhow` for error handling (if not already present)
  - [x] 1.3.4 Add `tracing` for logging (if not already present)
  - [x] 1.3.5 Verify dependencies resolve with `cd backend && cargo build`

### 2.0 Implement nikaya detection and structure configuration (Phase 1)
**Estimated Time:** 1-2 days
**Critical:** This phase must complete before Phase 2, as it determines parsing strategy

- [ ] 2.1 Implement `NikayaStructure` in `nikaya_structure.rs`
  - [ ] 2.1.1 Create `cli/src/tipitaka_xml_parser/nikaya_structure.rs`
  - [ ] 2.1.2 Define `NikayaStructure` struct with fields: nikaya (String), levels (Vec<GroupType>)
  - [ ] 2.1.3 Implement `normalize_name(name: &str) -> Option<String>` method
    - [ ] 2.1.3.1 Handle "Dīghanikāyo" and "Dīghanikāye" → "digha"
    - [ ] 2.1.3.2 Handle "Majjhimanikāyo" and "Majjhimanikāye" → "majjhima"
    - [ ] 2.1.3.3 Handle "Saṃyuttanikāyo" and "Saṃyuttanikāye" → "samyutta"
    - [ ] 2.1.3.4 Handle "Aṅguttaranikāyo" and "Aṅguttaranikāye" → "anguttara"
    - [ ] 2.1.3.5 Handle "Khuddakanikāyo" and "Khuddakanikāye" → "khuddaka"
    - [ ] 2.1.3.6 Return None for unknown names
  - [ ] 2.1.4 Implement `from_nikaya_name(name: &str) -> Option<NikayaStructure>` method
    - [ ] 2.1.4.1 Hard-code DN structure: [Nikaya, Book, Sutta]
    - [ ] 2.1.4.2 Hard-code MN structure: [Nikaya, Book, Vagga, Sutta]
    - [ ] 2.1.4.3 Hard-code SN structure: [Nikaya, Book, Samyutta, Vagga, Sutta]
    - [ ] 2.1.4.4 Hard-code AN structure (verify with sample files)
    - [ ] 2.1.4.5 Hard-code KN structure (verify with sample files)
  - [ ] 2.1.5 Add unit tests for `normalize_name()` and `from_nikaya_name()`
  - [ ] 2.1.6 Export from `mod.rs`

- [ ] 2.2 Implement nikaya detector in `nikaya_detector.rs`
  - [ ] 2.2.1 Create `cli/src/tipitaka_xml_parser/nikaya_detector.rs`
  - [ ] 2.2.2 Implement `detect_nikaya_structure(xml_content: &str) -> Result<NikayaStructure>`
    - [ ] 2.2.2.1 Search for `<p rend="nikaya">` tag in XML string
    - [ ] 2.2.2.2 Extract text content between tags
    - [ ] 2.2.2.3 Call `NikayaStructure::normalize_name()` on extracted text
    - [ ] 2.2.2.4 Call `NikayaStructure::from_nikaya_name()` with normalized name
    - [ ] 2.2.2.5 Return error if nikaya tag not found
    - [ ] 2.2.2.6 Return error if nikaya name is unknown
    - [ ] 2.2.2.7 Return NikayaStructure on success
  - [ ] 2.2.3 Add tracing/logging for detection results
  - [ ] 2.2.4 Export from `mod.rs`

- [ ] 2.3 Create test infrastructure for Phase 1
  - [ ] 2.3.1 Create `cli/tests/tipitaka_xml_parser/` directory
  - [ ] 2.3.2 Create `cli/tests/tipitaka_xml_parser/test_nikaya_detection.rs`
  - [ ] 2.3.3 Create test helper to generate minimal XML samples
  - [ ] 2.3.4 Write test: detect DN with "Dīghanikāyo" (yo ending)
  - [ ] 2.3.5 Write test: detect DN with "Dīghanikāye" (ye ending)
  - [ ] 2.3.6 Write test: detect MN with "Majjhimanikāyo"
  - [ ] 2.3.7 Write test: detect MN with "Majjhimanikāye"
  - [ ] 2.3.8 Write test: detect SN with variations
  - [ ] 2.3.9 Write test: error when nikaya tag missing
  - [ ] 2.3.10 Write test: error when nikaya name unknown
  - [ ] 2.3.11 Write test: verify correct NikayaStructure levels returned
  - [ ] 2.3.12 Run tests with `cd backend && cargo test test_nikaya_detection`
  - [ ] 2.3.13 Verify all tests pass

### 3.0 Implement fragment parser with line tracking (Phase 2)
**Estimated Time:** 2-3 days
**Depends on:** Phase 1 completion

- [ ] 3.1 Implement line-tracking XML reader foundation
  - [ ] 3.1.1 Create `cli/src/tipitaka_xml_parser/fragment_parser.rs`
  - [ ] 3.1.2 Create `LineTrackingReader` wrapper around `quick_xml::Reader`
  - [ ] 3.1.3 Implement newline counting logic in reader
  - [ ] 3.1.4 Add methods: `current_line()`, `get_position()`
  - [ ] 3.1.5 Test basic line tracking with simple XML

- [ ] 3.2 Implement hierarchy tracking
  - [ ] 3.2.1 Create `HierarchyTracker` struct with fields: current_levels (Vec<GroupLevel>), nikaya_structure (NikayaStructure)
  - [ ] 3.2.2 Implement `new(nikaya_structure: NikayaStructure) -> Self`
  - [ ] 3.2.3 Implement `enter_level(&mut self, level_type: GroupType, title: String, id: Option<String>, number: Option<i32>)`
    - [ ] 3.2.3.1 Determine depth of this level type in nikaya structure
    - [ ] 3.2.3.2 Truncate current_levels to appropriate depth
    - [ ] 3.2.3.3 Push new GroupLevel
  - [ ] 3.2.4 Implement `get_current_levels(&self) -> Vec<GroupLevel>` (returns clone)
  - [ ] 3.2.5 Add unit tests for hierarchy tracking with DN, MN, SN structures

- [ ] 3.3 Implement nikaya-aware fragment boundary detection
  - [ ] 3.3.1 Create `FragmentBoundaryDetector` struct
  - [ ] 3.3.2 Implement detection for common elements (all nikayas):
    - [ ] 3.3.2.1 Detect `<p rend="nikaya">` as Nikaya level
    - [ ] 3.3.2.2 Detect `<div type="book">` as Book level start
    - [ ] 3.3.2.3 Detect `<head rend="book">` for Book title
  - [ ] 3.3.3 Implement DN-specific detection logic:
    - [ ] 3.3.3.1 Detect `<head rend="chapter">` as Sutta title (not vagga!)
    - [ ] 3.3.3.2 No vagga level processing for DN
  - [ ] 3.3.4 Implement MN-specific detection logic:
    - [ ] 3.3.4.1 Detect `<div type="vagga">` as Vagga level start
    - [ ] 3.3.4.2 Detect `<head rend="chapter">` as Vagga title
    - [ ] 3.3.4.3 Detect `<p rend="subhead">` as Sutta title
  - [ ] 3.3.5 Implement SN-specific detection logic:
    - [ ] 3.3.5.1 Detect `<div type="samyutta">` as Samyutta level start
    - [ ] 3.3.5.2 Detect `<div type="vagga">` as Vagga level start
    - [ ] 3.3.5.3 Detect `<p rend="subhead">` as Sutta title
  - [ ] 3.3.6 Add logic to extract title text from elements
  - [ ] 3.3.7 Add logic to extract id attributes from elements

- [ ] 3.4 Implement fragment extraction
  - [ ] 3.4.1 Create `parse_into_fragments(xml_content: &str, nikaya_structure: &NikayaStructure) -> Result<Vec<XmlFragment>>`
  - [ ] 3.4.2 Initialize LineTrackingReader and HierarchyTracker
  - [ ] 3.4.3 Initialize fragment collection Vec
  - [ ] 3.4.4 Implement main parsing loop:
    - [ ] 3.4.4.1 Read XML events with quick_xml
    - [ ] 3.4.4.2 Track current position for content extraction
    - [ ] 3.4.4.3 On boundary detection, create new fragment
    - [ ] 3.4.4.4 Extract raw XML content for fragment (preserve whitespace)
    - [ ] 3.4.4.5 Set fragment start_line and end_line
    - [ ] 3.4.4.6 Clone current hierarchy levels into fragment
    - [ ] 3.4.4.7 Set fragment_type (Header vs Sutta)
    - [ ] 3.4.4.8 Push fragment to collection
  - [ ] 3.4.5 Return fragment collection
  - [ ] 3.4.6 Add error handling for malformed XML

- [ ] 3.5 Test fragment parser
  - [ ] 3.5.1 Create `cli/tests/tipitaka_xml_parser/test_fragment_parser.rs`
  - [ ] 3.5.2 Create sample DN XML with known structure
  - [ ] 3.5.3 Test: Parse DN sample, verify fragment count
  - [ ] 3.5.4 Test: Verify line numbers are accurate
  - [ ] 3.5.5 Test: Verify group_levels are correct for DN structure
  - [ ] 3.5.6 Test: Verify fragment_type assignments
  - [ ] 3.5.7 Create sample MN XML with vagga level
  - [ ] 3.5.8 Test: Parse MN sample, verify vagga level captured
  - [ ] 3.5.9 Test: Verify sutta boundaries with `<p rend="subhead">`
  - [ ] 3.5.10 Test: Reconstruct XML from fragments (should match original)
  - [ ] 3.5.11 Run tests with `cd backend && cargo test test_fragment_parser`

### 4.0 Implement sutta builder and database integration (Phase 3)
**Estimated Time:** 2-3 days
**Depends on:** Phase 2 completion

- [ ] 4.1 Implement sutta assembly in `sutta_builder.rs`
  - [ ] 4.1.1 Create `cli/src/tipitaka_xml_parser/sutta_builder.rs`
  - [ ] 4.1.2 Define `SuttaRecord` struct matching appdata schema fields
  - [ ] 4.1.3 Implement `build_suttas(fragments: Vec<XmlFragment>, nikaya_structure: &NikayaStructure) -> Result<Vec<SuttaRecord>>`
    - [ ] 4.1.3.1 Group consecutive Sutta-type fragments
    - [ ] 4.1.3.2 Extract metadata from group_levels
    - [ ] 4.1.3.3 Generate UID from hierarchy (e.g., "vri-cst/dn1")
    - [ ] 4.1.3.4 Generate sutta_ref (e.g., "DN 1")
    - [ ] 4.1.3.5 Build group_path by joining level titles with "/"
    - [ ] 4.1.3.6 Extract order_index and group_index from group_number
    - [ ] 4.1.3.7 Extract title from sutta GroupLevel
    - [ ] 4.1.3.8 Set nikaya from first GroupLevel
    - [ ] 4.1.3.9 Set language to "pli"
    - [ ] 4.1.3.10 Set source_uid to "cst4"
  - [ ] 4.1.4 Implement XML to HTML transformation
    - [ ] 4.1.4.1 Research existing html_transformer in codebase
    - [ ] 4.1.4.2 Integrate or reimplement for fragment content
    - [ ] 4.1.4.3 Convert XML paragraphs to HTML
    - [ ] 4.1.4.4 Handle special elements (hi rend="paranum", etc.)
  - [ ] 4.1.5 Implement plain text extraction
    - [ ] 4.1.5.1 Research existing plain text extractors
    - [ ] 4.1.5.2 Strip XML tags from content
    - [ ] 4.1.5.3 Apply consistent_niggahita normalization
    - [ ] 4.1.5.4 Clean whitespace
  - [ ] 4.1.6 Create complete SuttaRecord instances
  - [ ] 4.1.7 Return Vec<SuttaRecord>

- [ ] 4.2 Test sutta builder
  - [ ] 4.2.1 Create `cli/tests/tipitaka_xml_parser/test_sutta_builder.rs`
  - [ ] 4.2.2 Test: Build sutta from single fragment
  - [ ] 4.2.3 Test: Build sutta from multiple consecutive fragments
  - [ ] 4.2.4 Test: Verify UID generation (DN, MN, SN patterns)
  - [ ] 4.2.5 Test: Verify sutta_ref generation
  - [ ] 4.2.6 Test: Verify group_path construction
  - [ ] 4.2.7 Test: Verify title extraction
  - [ ] 4.2.8 Test: Verify HTML transformation produces valid HTML
  - [ ] 4.2.9 Test: Verify plain text extraction removes tags
  - [ ] 4.2.10 Run tests with `cd backend && cargo test test_sutta_builder`

- [ ] 4.3 Implement database insertion in `database_inserter.rs`
  - [ ] 4.3.1 Create `cli/src/tipitaka_xml_parser/database_inserter.rs`
  - [ ] 4.3.2 Import diesel and appdata_schema types
  - [ ] 4.3.3 Implement `insert_suttas(suttas: Vec<SuttaRecord>, db_path: &Path) -> Result<usize>`
    - [ ] 4.3.3.1 Establish database connection
    - [ ] 4.3.3.2 Begin transaction
    - [ ] 4.3.3.3 Convert SuttaRecord to NewSutta (appdata_models)
    - [ ] 4.3.3.4 Insert records with diesel
    - [ ] 4.3.3.5 Handle duplicate UIDs (update vs skip vs error)
    - [ ] 4.3.3.6 Commit transaction
    - [ ] 4.3.3.7 Return count of inserted records
  - [ ] 4.3.4 Add error handling and rollback logic
  - [ ] 4.3.5 Add tracing for insertion progress

- [ ] 4.4 Test database integration
  - [ ] 4.4.1 Create test database fixture
  - [ ] 4.4.2 Test: Insert single sutta
  - [ ] 4.4.3 Test: Insert multiple suttas in transaction
  - [ ] 4.4.4 Test: Query inserted records
  - [ ] 4.4.5 Test: Verify all fields populated correctly
  - [ ] 4.4.6 Test: Handle duplicate UID scenarios
  - [ ] 4.4.7 Test: Transaction rollback on error

- [ ] 4.5 Implement high-level integration API in `integration.rs`
  - [ ] 4.5.1 Create `cli/src/tipitaka_xml_parser/integration.rs`
  - [ ] 4.5.2 Define `ProcessingStats` struct (files_processed, suttas_inserted, errors)
  - [ ] 4.5.3 Implement `process_xml_file(xml_path: &Path, db_path: &Path, verbose: bool) -> Result<ProcessingStats>`
    - [ ] 4.5.3.1 Read XML file to string
    - [ ] 4.5.3.2 Call detect_nikaya_structure()
    - [ ] 4.5.3.3 Log detected nikaya if verbose
    - [ ] 4.5.3.4 Call parse_into_fragments()
    - [ ] 4.5.3.5 Log fragment count if verbose
    - [ ] 4.5.3.6 Call build_suttas()
    - [ ] 4.5.3.7 Log sutta count if verbose
    - [ ] 4.5.3.8 Call insert_suttas()
    - [ ] 4.5.3.9 Build and return ProcessingStats
  - [ ] 4.5.4 Implement `process_directory(dir_path: &Path, db_path: &Path, verbose: bool) -> Result<ProcessingStats>`
    - [ ] 4.5.4.1 Collect all .xml files in directory
    - [ ] 4.5.4.2 Process each file with process_xml_file()
    - [ ] 4.5.4.3 Aggregate statistics
    - [ ] 4.5.4.4 Return combined stats
  - [ ] 4.5.5 Add progress reporting with tracing

### 5.0 Add CLI command integration
**Estimated Time:** 1 day
**Depends on:** Phase 3 completion

- [ ] 5.1 Update CLI command definitions
  - [ ] 5.1.1 Open `cli/src/main.rs`
  - [ ] 5.1.2 Add `ParseTipitakaXml` variant to Commands enum
  - [ ] 5.1.3 Add fields: input_path (PathBuf), output_db_path (PathBuf), verbose (bool), dry_run (bool)
  - [ ] 5.1.4 Add clap annotations for command and arguments
  - [ ] 5.1.5 Add help text describing the command

- [ ] 5.2 Implement command handler
  - [ ] 5.2.1 Add match arm for ParseTipitakaXml in main command dispatcher
  - [ ] 5.2.2 Import tipitaka_xml_parser module
  - [ ] 5.2.3 Implement handler function
    - [ ] 5.2.3.1 Validate input_path exists
    - [ ] 5.2.3.2 Create output_db_path parent directory if needed
    - [ ] 5.2.3.3 Determine if input is file or directory
    - [ ] 5.2.3.4 Call process_xml_file() or process_directory()
    - [ ] 5.2.3.5 Handle dry_run flag (skip database insertion)
    - [ ] 5.2.3.6 Display processing statistics
  - [ ] 5.2.4 Add error handling and user-friendly error messages

- [ ] 5.3 Add progress reporting
  - [ ] 5.3.1 Add progress output for each file processed
  - [ ] 5.3.2 Show running count of suttas processed
  - [ ] 5.3.3 Show final statistics summary
  - [ ] 5.3.4 Format output for readability

- [ ] 5.4 Test CLI integration
  - [ ] 5.4.1 Build CLI with `cd backend && cargo build`
  - [ ] 5.4.2 Test with sample DN file: `./target/debug/simsapa-cli parse-tipitaka-xml <file> <db>`
  - [ ] 5.4.3 Test with directory of files
  - [ ] 5.4.4 Test --verbose flag
  - [ ] 5.4.5 Test --dry-run flag
  - [ ] 5.4.6 Test error handling (missing file, invalid XML, etc.)
  - [ ] 5.4.7 Verify help text: `./target/debug/simsapa-cli parse-tipitaka-xml --help`

### 6.0 Comprehensive testing and validation
**Estimated Time:** 2-3 days
**Depends on:** All previous phases

- [ ] 6.1 Create integration test suite
  - [ ] 6.1.1 Create `cli/tests/tipitaka_xml_parser/test_integration.rs`
  - [ ] 6.1.2 Set up test data directory `cli/tests/data/tipitaka_xml_samples/`
  - [ ] 6.1.3 Add sample DN XML file
  - [ ] 6.1.4 Add sample MN XML file
  - [ ] 6.1.5 Add sample SN XML file

- [ ] 6.2 End-to-end integration tests
  - [ ] 6.2.1 Test: Parse complete DN file end-to-end
  - [ ] 6.2.2 Test: Verify expected sutta count for DN
  - [ ] 6.2.3 Test: Parse complete MN file end-to-end
  - [ ] 6.2.4 Test: Verify expected sutta count for MN
  - [ ] 6.2.5 Test: Parse complete SN file end-to-end
  - [ ] 6.2.6 Test: Verify expected sutta count for SN
  - [ ] 6.2.7 Test: Verify all database records have required fields
  - [ ] 6.2.8 Test: Verify UIDs are unique
  - [ ] 6.2.9 Test: Verify sutta_refs are correctly formatted
  - [ ] 6.2.10 Run full test suite with `cd backend && cargo test`

- [ ] 6.3 XML reconstruction validation
  - [ ] 6.3.1 Test: Parse DN to fragments, reconstruct XML
  - [ ] 6.3.2 Test: Compare reconstructed with original (whitespace normalized)
  - [ ] 6.3.3 Test: Parse MN to fragments, reconstruct XML
  - [ ] 6.3.4 Test: Verify no content lost in reconstruction

- [ ] 6.4 Comparison with TSV parser (if available)
  - [ ] 6.4.1 Process same input file with both parsers
  - [ ] 6.4.2 Compare sutta counts
  - [ ] 6.4.3 Compare sample sutta content (spot check)
  - [ ] 6.4.4 Document any discrepancies
  - [ ] 6.4.5 Investigate and resolve significant differences

- [ ] 6.5 Edge case testing
  - [ ] 6.5.1 Test: Empty XML file
  - [ ] 6.5.2 Test: Malformed XML (missing closing tags)
  - [ ] 6.5.3 Test: XML with unexpected elements
  - [ ] 6.5.4 Test: XML with missing nikaya marker
  - [ ] 6.5.5 Test: XML with unusual whitespace
  - [ ] 6.5.6 Test: Very large sutta content
  - [ ] 6.5.7 Test: Special characters and diacritics in text
  - [ ] 6.5.8 Test: Commentary files (.att, .tik variations)

- [ ] 6.6 Performance testing
  - [ ] 6.6.1 Test: Parse MN (152 suttas), measure time
  - [ ] 6.6.2 Verify parsing completes in < 10 seconds
  - [ ] 6.6.3 Test: Parse DN (34 suttas), measure time
  - [ ] 6.6.4 Verify parsing completes in < 5 seconds
  - [ ] 6.6.5 Monitor memory usage during large file processing
  - [ ] 6.6.6 Verify memory stays < 500MB for largest file
  - [ ] 6.6.7 Profile code if performance issues found
  - [ ] 6.6.8 Optimize bottlenecks

- [ ] 6.7 Error handling validation
  - [ ] 6.7.1 Test: Verify meaningful error messages
  - [ ] 6.7.2 Test: Database connection failures
  - [ ] 6.7.3 Test: Disk full scenarios
  - [ ] 6.7.4 Test: Permission errors
  - [ ] 6.7.5 Test: Concurrent access to database
  - [ ] 6.7.6 Ensure all errors propagate correctly

### 7.0 Update documentation
**Estimated Time:** 1 day
**Depends on:** Testing completion

- [ ] 7.1 Update PROJECT_MAP.md
  - [ ] 7.1.1 Add tipitaka_xml_parser module to structure
  - [ ] 7.1.2 Document each submodule's purpose
  - [ ] 7.1.3 Add cross-references to related modules
  - [ ] 7.1.4 Document test locations

- [ ] 7.2 Add module-level documentation
  - [ ] 7.2.1 Add doc comments to `mod.rs`
  - [ ] 7.2.2 Add doc comments to `types.rs` (explain each type)
  - [ ] 7.2.3 Add doc comments to `nikaya_detector.rs` (explain detection logic)
  - [ ] 7.2.4 Add doc comments to `nikaya_structure.rs` (explain hard-coded structures)
  - [ ] 7.2.5 Add doc comments to `fragment_parser.rs` (explain parsing strategy)
  - [ ] 7.2.6 Add doc comments to `sutta_builder.rs` (explain assembly process)
  - [ ] 7.2.7 Add doc comments to `database_inserter.rs`
  - [ ] 7.2.8 Add doc comments to `integration.rs` (explain high-level API)

- [ ] 7.3 Add function-level documentation
  - [ ] 7.3.1 Document all public functions with examples
  - [ ] 7.3.2 Document error conditions
  - [ ] 7.3.3 Document parameter constraints
  - [ ] 7.3.4 Document return value meanings

- [ ] 7.4 Create usage examples
  - [ ] 7.4.1 Add example in integration.rs module docs
  - [ ] 7.4.2 Document CLI usage in README or module docs
  - [ ] 7.4.3 Add code example for programmatic usage
  - [ ] 7.4.4 Document nikaya structure configuration

- [ ] 7.5 Document nikaya-specific differences
  - [ ] 7.5.1 Create table of nikaya structures
  - [ ] 7.5.2 Document fragment boundary differences per nikaya
  - [ ] 7.5.3 Document known limitations
  - [ ] 7.5.4 Document supported XML variations

- [ ] 7.6 Final verification
  - [ ] 7.6.1 Generate rustdoc: `cd backend && cargo doc --open`
  - [ ] 7.6.2 Review all generated documentation
  - [ ] 7.6.3 Fix any broken links
  - [ ] 7.6.4 Verify examples compile (with doc tests)
  - [ ] 7.6.5 Run `cd backend && cargo test --doc` for doc tests
