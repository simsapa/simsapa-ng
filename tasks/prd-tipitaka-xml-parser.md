# PRD: Tipitaka XML Parser with Fragment-Based Architecture

**Status:** Draft
**Created:** 2025-10-31
**Author:** AI Development Agent

## Overview

This PRD outlines the implementation of a proper XML parser for VRI CST Tipitaka XML files that preserves the original XML structure in fragments while supporting hierarchical grouping levels that vary by nikaya.

### Background

The current implementation (`parse-tipitaka-xml-using-tsv` command) processes VRI CST Tipitaka XML files by using TSV spreadsheet data to determine sutta boundaries, without actually parsing the XML structure itself. While functional, this approach:

- Doesn't preserve original XML content
- Doesn't capture line number information
- Relies on external TSV data rather than the XML structure
- Doesn't handle the varying hierarchical structures of different nikayas

The new parser will properly parse XML files into fragments that preserve the original content, capture line numbers, and support flexible hierarchical grouping.

## Goals

### Primary Goals

1. **Parse XML into Fragments**: Create a `XmlFragment` structure that preserves original XML content with line number tracking
2. **Support Variable Hierarchies**: Handle different nikaya structures (DN: Nikaya > Book > Sutta, MN: Nikaya > Book > Vagga > Sutta, etc.)
3. **Enable Reconstruction**: Make it possible to reconstruct the entire original XML file from fragments
4. **Maintain Database Compatibility**: Generate suttas for the existing appdata schema

### Secondary Goals

1. **Improve Maintainability**: Create a clean, testable architecture
2. **Support Future Extensions**: Design for potential commentary parsing, structural analysis, etc.
3. **Provide Detailed Logging**: Track parsing progress and issues

## Non-Goals

- Modifying the appdata database schema
- Parsing non-VRI XML formats
- Real-time parsing or streaming (batch processing is acceptable)
- GUI or interactive parsing tools

## User Stories

### Story 1: Parse XML with Line Numbers
**As a** developer maintaining the Simsapa database
**I want** to parse Tipitaka XML files while preserving original content and line numbers
**So that** I can trace database entries back to their source XML locations

**Acceptance Criteria:**
- Each `XmlFragment` stores original unparsed XML content
- Start and end line numbers are captured for each fragment
- The entire XML file can be reconstructed from fragments

### Story 2: Handle Variable Nikaya Hierarchies
**As a** developer processing different nikayas
**I want** the parser to handle varying hierarchical structures
**So that** Dīgha, Majjhima, and other nikayas are correctly represented

**Acceptance Criteria:**
- Dīgha Nikāya: Nikaya > Book > Sutta (3 levels)
- Majjhima Nikāya: Nikaya > Book > Vagga > Sutta (4 levels)
- Saṃyutta Nikāya: Nikaya > Book > Saṃyutta > Vagga > Sutta (5 levels)
- Each fragment knows its position in the hierarchy

### Story 3: Command-Line Processing
**As a** developer building the Simsapa database
**I want** to process XML files via CLI with the same interface as the existing command
**So that** I can migrate smoothly from TSV-based to XML-based parsing

**Acceptance Criteria:**
- Command: `parse-tipitaka-xml <input> <output_db> [--verbose] [--dry-run]`
- Same arguments as `parse-tipitaka-xml-using-tsv`
- Progress output showing files, suttas processed
- Statistics summary at completion

## Technical Design

### Data Structures

#### XmlFragment

```rust
/// Represents a fragment of the original XML file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XmlFragment {
    /// Type of fragment (header metadata or sutta content)
    pub fragment_type: FragmentType,
    
    /// Original unparsed XML content
    pub content: String,
    
    /// Starting line number in original file (1-indexed)
    pub start_line: usize,
    
    /// Ending line number in original file (1-indexed)
    pub end_line: usize,
    
    /// Current hierarchical levels at this fragment
    pub group_levels: Vec<GroupLevel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FragmentType {
    /// Header/metadata fragment (nikaya name, book titles, etc.)
    Header,
    
    /// Sutta content fragment
    Sutta,
}
```

#### GroupLevel

```rust
/// Represents one level in the hierarchical structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupLevel {
    /// Type of this hierarchical level
    pub group_type: GroupType,
    
    /// Sequential number within this level (e.g., book 1, vagga 2)
    pub group_number: Option<i32>,
    
    /// Title/name of this group (e.g., "Mūlapariyāyavaggo")
    pub title: String,
    
    /// XML ID attribute if present (e.g., "mn1_1")
    pub id: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum GroupType {
    Nikaya,
    Book,
    Vagga,
    Samyutta,
    Sutta,
}
```

#### Nikaya Configuration

```rust
/// Defines the hierarchical structure for each nikaya
pub struct NikayaStructure {
    /// Nikaya identifier
    pub nikaya: String,
    
    /// Ordered list of group types from top to bottom
    /// Example MN: [Nikaya, Book, Vagga, Sutta]
    /// Example DN: [Nikaya, Book, Sutta]
    pub levels: Vec<GroupType>,
}

impl NikayaStructure {
    /// Get predefined structures for known nikayas
    pub fn get_structure(nikaya_name: &str) -> Option<NikayaStructure> {
        // Returns structure based on nikaya name
    }
}
```

### Module Structure

Create new module: `cli/src/tipitaka_xml_parser/`

```
tipitaka_xml_parser/
├── mod.rs                    # Module exports
├── types.rs                  # XmlFragment, GroupLevel, GroupType, etc.
├── nikaya_detector.rs        # Detect nikaya from XML content (PHASE 1)
├── nikaya_structure.rs       # Hard-coded nikaya hierarchy configurations
├── fragment_parser.rs        # Parse XML into fragments (uses NikayaStructure)
├── sutta_builder.rs          # Build Sutta from fragments
├── database_inserter.rs      # Insert into appdata schema
└── integration.rs            # High-level API
```

### High-Level Processing Flow

```rust
// Main processing function
pub fn process_xml_file(xml_path: &Path) -> Result<Vec<SuttaRecord>> {
    // PHASE 1: Detect nikaya FIRST - this determines parsing strategy
    let xml_content = fs::read_to_string(xml_path)?;
    let nikaya_structure = detect_nikaya_structure(&xml_content)?;
    
    println!("Detected nikaya: {} with {} levels", 
        nikaya_structure.nikaya, nikaya_structure.levels.len());
    
    // PHASE 2: Parse into fragments using nikaya-specific strategy
    let fragments = parse_into_fragments(&xml_content, &nikaya_structure)?;
    
    // PHASE 3: Build suttas from fragments
    let suttas = build_suttas(fragments, &nikaya_structure)?;
    
    Ok(suttas)
}
```

### Parsing Algorithm

#### Phase 1: Nikaya Detection and Structure

This MUST happen first, as it determines the parsing strategy for subsequent phases.

```rust
/// Detect nikaya from XML file content and return its hierarchical structure
pub fn detect_nikaya_structure(xml_content: &str) -> Result<NikayaStructure> {
    // 1. Search for <p rend="nikaya"> element in XML content string
    // 2. Extract nikaya name text (e.g., "Dīghanikāyo", "Dīghanikāye", 
    //    "Majjhimanikāyo", "Majjhimanikāye")
    // 3. Normalize name (handle both -o and -e endings)
    // 4. Map to hard-coded structure configuration
    // 5. Return NikayaStructure with level configuration
    
    // Example mapping:
    // "Dīghanikāyo" | "Dīghanikāye" -> DN structure [Nikaya, Book, Sutta]
    // "Majjhimanikāyo" | "Majjhimanikāye" -> MN structure [Nikaya, Book, Vagga, Sutta]
    // etc.
}

impl NikayaStructure {
    /// Get hard-coded structure for a normalized nikaya name
    pub fn from_nikaya_name(name: &str) -> Option<NikayaStructure> {
        match name {
            "digha" => Some(NikayaStructure {
                nikaya: "Dīghanikāyo".to_string(),
                levels: vec![GroupType::Nikaya, GroupType::Book, GroupType::Sutta],
            }),
            "majjhima" => Some(NikayaStructure {
                nikaya: "Majjhimanikāyo".to_string(),
                levels: vec![GroupType::Nikaya, GroupType::Book, GroupType::Vagga, GroupType::Sutta],
            }),
            "samyutta" => Some(NikayaStructure {
                nikaya: "Saṃyuttanikāyo".to_string(),
                levels: vec![GroupType::Nikaya, GroupType::Book, GroupType::Samyutta, GroupType::Vagga, GroupType::Sutta],
            }),
            // Add other nikayas as needed
            _ => None,
        }
    }
    
    /// Normalize nikaya name from XML content
    /// "Dīghanikāyo" | "Dīghanikāye" -> "digha"
    /// "Majjhimanikāyo" | "Majjhimanikāye" -> "majjhima"
    pub fn normalize_name(name: &str) -> Option<String> {
        let lowercase = name.to_lowercase();
        if lowercase.starts_with("dīgha") {
            Some("digha".to_string())
        } else if lowercase.starts_with("majjhima") {
            Some("majjhima".to_string())
        } else if lowercase.starts_with("saṃyutta") || lowercase.starts_with("samyutta") {
            Some("samyutta".to_string())
        } else if lowercase.starts_with("aṅguttara") || lowercase.starts_with("anguttara") {
            Some("anguttara".to_string())
        } else if lowercase.starts_with("khuddaka") {
            Some("khuddaka".to_string())
        } else {
            None
        }
    }
}
```

#### Phase 2: Fragment Extraction

Fragment extraction strategy is based on the detected NikayaStructure.

```rust
/// Parse XML file into fragments with line number tracking
/// Strategy varies based on nikaya structure
pub fn parse_into_fragments(
    xml_content: &str, 
    nikaya_structure: &NikayaStructure
) -> Result<Vec<XmlFragment>> {
    // 1. Use quick_xml with line tracking
    // 2. Identify fragment boundaries based on nikaya structure:
    //    - For DN: <head rend="chapter"> marks sutta titles (no vagga level)
    //    - For MN: <p rend="subhead"> marks sutta titles (has vagga level)
    //    - For SN: Additional <div type="samyutta"> level
    // 3. Capture raw XML text including whitespace
    // 4. Track start/end line numbers
    // 5. Build group_levels array based on current position
}
```

#### Phase 3: Group Level Tracking

```rust
/// Track current position in hierarchy as we parse
struct HierarchyTracker {
    current_levels: Vec<GroupLevel>,
    nikaya_structure: NikayaStructure,
}

impl HierarchyTracker {
    /// Update levels when entering a new div or head element
    pub fn enter_level(&mut self, level_type: GroupType, title: String, id: Option<String>) {
        // Push new level or update existing level at same depth
    }
    
    /// Get current levels snapshot for fragment
    pub fn get_current_levels(&self) -> Vec<GroupLevel> {
        self.current_levels.clone()
    }
}
```

#### Phase 4: Sutta Assembly

```rust
/// Build Sutta database records from fragments
pub fn build_suttas(
    fragments: Vec<XmlFragment>,
    nikaya_structure: &NikayaStructure
) -> Result<Vec<SuttaRecord>> {
    // 1. Group consecutive Sutta fragments
    // 2. Extract metadata from group_levels
    // 3. Generate UID based on position (e.g., "vri-cst/dn1")
    // 4. Convert XML to HTML (reuse existing html_transformer)
    // 5. Extract plain text (reuse existing extractors)
    // 6. Create SuttaRecord for database insertion
}
```

### XML Fragment Boundaries

The parser identifies fragments using these XML elements. **Note:** Fragment boundary detection varies by nikaya structure.

#### Nikaya Detection (Phase 1)

- `<p rend="nikaya">Dīghanikāyo</p>` or `<p rend="nikaya">Dīghanikāye</p>` - Dīgha Nikāya
- `<p rend="nikaya">Majjhimanikāyo</p>` or `<p rend="nikaya">Majjhimanikāye</p>` - Majjhima Nikāya
- Similar patterns for other nikayas (Saṃyutta, Aṅguttara, etc.)

#### Common Header Fragments

- `<p rend="nikaya">` - Nikaya name
- `<head rend="book">` - Book title
- `<div type="book">` - Book division start

#### Nikaya-Specific Fragment Boundaries

**Dīgha Nikāya (DN)** - Structure: [Nikaya, Book, Sutta]
- `<head rend="chapter">` - **Sutta title** (not vagga, goes directly to sutta level)
- No `<div type="vagga">` or `<p rend="subhead">` elements

**Majjhima Nikāya (MN)** - Structure: [Nikaya, Book, Vagga, Sutta]
- `<div type="vagga">` - Vagga division start
- `<head rend="chapter">` - Vagga title
- `<p rend="subhead">` - **Sutta title**

**Saṃyutta Nikāya (SN)** - Structure: [Nikaya, Book, Saṃyutta, Vagga, Sutta]
- `<div type="samyutta">` - Saṃyutta division start
- `<div type="vagga">` - Vagga division start
- `<p rend="subhead">` - **Sutta title**

#### Sutta Content

For all nikayas, sutta content consists of:
- All `<p>` elements with various `rend` attributes (bodytext, gatha, etc.)
- Content continues until the next sutta boundary marker

### Line Number Tracking

Use `quick_xml::Reader` with position tracking:

```rust
let mut reader = Reader::from_str(content);
reader.trim_text(false); // Preserve whitespace

// Track line numbers by counting newlines
let mut current_line = 1;
let mut fragment_start_line = 1;

loop {
    let pos_before = reader.buffer_position();
    match reader.read_event() {
        // ... event handling
    }
    let pos_after = reader.buffer_position();
    
    // Count newlines in processed text
    let text_slice = &content[pos_before..pos_after];
    current_line += text_slice.chars().filter(|&c| c == '\n').count();
}
```

### Database Schema Mapping

Map fragments to existing `appdata_schema::suttas` table:

```rust
pub struct SuttaRecord {
    pub uid: String,              // "vri-cst/dn1" format
    pub sutta_ref: String,        // "DN 1" format
    pub nikaya: String,           // From first GroupLevel
    pub language: String,         // "pli"
    pub group_path: String,       // Join group_levels titles with "/"
    pub group_index: Option<i32>, // From group_number
    pub order_index: Option<i32>, // Sequential sutta number
    pub title: String,            // From sutta title fragment
    pub content_plain: String,    // Extracted plain text
    pub content_html: String,     // Transformed HTML
    pub source_uid: String,       // "cst4"
    // ... other fields
}
```

### CLI Integration

Add to `cli/src/main.rs`:

```rust
Commands::ParseTipitakaXml {
    input_path: PathBuf,
    output_db_path: PathBuf,
    verbose: bool,
    dry_run: bool,
}

fn parse_tipitaka_xml(
    input_path: &Path,
    output_db_path: &Path,
    verbose: bool,
    dry_run: bool,
) -> Result<(), String> {
    use tipitaka_xml_parser::TipitakaXmlParser;
    
    let parser = TipitakaXmlParser::new(verbose);
    
    // Process single file or directory
    let files = collect_xml_files(input_path)?;
    
    for file in files {
        let stats = parser.process_file(&file, output_db_path, dry_run)?;
        // Display statistics
    }
}
```

## Implementation Plan

### Phase 1: Nikaya Detection and Core Structures (1-2 days)
- [ ] Create `tipitaka_xml_parser` module structure
- [ ] Implement `GroupLevel`, `GroupType` enums
- [ ] Implement `NikayaStructure` with hard-coded configurations
  - [ ] Define structures for DN, MN, SN, AN, KN
  - [ ] Implement `from_nikaya_name()` with hard-coded mappings
  - [ ] Implement `normalize_name()` for handling -o/-e endings
- [ ] Implement `detect_nikaya_structure(xml_content: &str)`
  - [ ] Search for `<p rend="nikaya">` in XML string
  - [ ] Extract and normalize nikaya name
  - [ ] Return appropriate hard-coded structure
- [ ] Write unit tests
  - [ ] Test nikaya detection for DN, MN variants (-o and -e endings)
  - [ ] Test normalize_name() function
  - [ ] Test structure retrieval

### Phase 2: Fragment Parser (2-3 days)
- [ ] Implement `XmlFragment` data structure
- [ ] Implement line-tracking XML reader
- [ ] Build fragment extraction logic with nikaya-aware parsing
  - [ ] DN-specific: `<head rend="chapter">` as sutta boundary
  - [ ] MN-specific: `<p rend="subhead">` as sutta boundary with vagga level
  - [ ] SN-specific: Additional samyutta level handling
- [ ] Implement `HierarchyTracker` for level management
  - [ ] Accept `NikayaStructure` to know expected levels
  - [ ] Track current position in hierarchy
- [ ] Test with sample DN and MN XML files
- [ ] Verify XML reconstruction from fragments

### Phase 3: Sutta Builder (2-3 days)
- [ ] Implement sutta assembly from fragments
- [ ] Generate UIDs based on hierarchy
- [ ] Integrate existing HTML transformation
- [ ] Integrate existing plain text extraction
- [ ] Test database record generation

### Phase 4: Database Integration (1-2 days)
- [ ] Implement database inserter
- [ ] Add transaction support
- [ ] Handle duplicate detection
- [ ] Test with actual database

### Phase 5: CLI Integration (1 day)
- [ ] Add `parse-tipitaka-xml` command
- [ ] Implement file collection (single file / directory)
- [ ] Add progress reporting
- [ ] Add statistics output

### Phase 6: Testing & Validation (2-3 days)
- [ ] Test with all nikaya XML files
- [ ] Compare output with TSV-based parser
- [ ] Validate database schema compliance
- [ ] Performance testing with large files
- [ ] Edge case testing

### Phase 7: Documentation (1 day)
- [ ] Update PROJECT_MAP.md
- [ ] Add module documentation
- [ ] Add usage examples
- [ ] Document nikaya structure configuration

**Total Estimated Time:** 10-15 days

## Testing Strategy

### Unit Tests

1. **Nikaya Detection Tests** (Critical - Phase 1)
   - Detect "Dīghanikāyo" variant
   - Detect "Dīghanikāye" variant
   - Detect "Majjhimanikāyo" variant
   - Detect "Majjhimanikāye" variant
   - Detect other nikayas (SN, AN, KN)
   - Handle missing nikaya marker (error case)
   - Verify correct NikayaStructure returned
   - Verify normalize_name() handles diacritics correctly

2. **Fragment Parser Tests**
   - Parse with DN structure (no vagga level)
   - Parse with MN structure (with vagga level)
   - Parse with SN structure (with samyutta level)
   - Parse book divisions
   - Parse sutta content with correct boundaries per nikaya
   - Verify line number tracking
   - Test XML reconstruction

3. **Hierarchy Tracker Tests**
   - Level entry/exit
   - DN structure (3 levels)
   - MN structure (4 levels)
   - SN structure (5 levels)

4. **Sutta Builder Tests**
   - UID generation
   - Metadata extraction
   - HTML transformation
   - Plain text extraction

### Integration Tests

1. **End-to-End Parsing**
   - Parse complete DN file
   - Parse complete MN file
   - Parse complete SN file
   - Verify database records

2. **Database Integration**
   - Insert suttas
   - Query inserted records
   - Verify field values

### Validation Tests

1. **Comparison with TSV Parser**
   - Same input file
   - Compare sutta counts
   - Compare content output
   - Identify discrepancies

2. **XML Reconstruction**
   - Parse to fragments
   - Reconstruct XML
   - Compare with original (allowing for whitespace normalization)

## Success Metrics

1. **Correctness**
   - ✅ All XML files parse without errors
   - ✅ Sutta count matches expected values
   - ✅ Database records pass schema validation
   - ✅ Fragment line numbers are accurate

2. **Completeness**
   - ✅ XML can be reconstructed from fragments
   - ✅ All hierarchy levels are captured
   - ✅ All sutta metadata is extracted

3. **Performance**
   - ⏱️ Parse MN (152 suttas) in < 10 seconds
   - ⏱️ Parse DN (34 suttas) in < 5 seconds
   - 💾 Memory usage < 500MB for largest file

4. **Maintainability**
   - 📝 Code coverage > 80%
   - 📚 All public APIs documented
   - 🧪 Integration tests pass

## Risk Assessment

### Technical Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| XML format variations not captured | Medium | High | Extensive testing with all nikayas; fallback handling |
| Line tracking inaccuracies | Low | Medium | Unit tests with known line positions; verification tests |
| Memory issues with large files | Low | Medium | Streaming approach; fragment batching |
| UID generation conflicts | Low | High | Use existing uid_generator patterns; add collision detection |

### Schedule Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Unknown XML structures | Medium | High | Early testing with all nikayas; iterative refinement |
| Performance issues | Low | Medium | Profiling; optimization if needed |
| Integration challenges | Low | Medium | Reuse existing modules (html_transformer, etc.) |

## Dependencies

### Internal
- `simsapa_backend::db::appdata_models::NewSutta`
- `simsapa_backend::db::appdata_schema`
- `simsapa_backend::helpers::consistent_niggahita`
- Existing `html_transformer` module (can be reused or reimplemented)

### External
- `quick_xml` - XML parsing
- `diesel` - Database ORM
- `anyhow` - Error handling
- `serde` - Serialization
- `tracing` - Logging

## Open Questions

1. **Should we preserve the TSV-based parser?**
   - **Recommendation:** Yes, keep as `parse-tipitaka-xml-using-tsv` for backward compatibility
   - New parser is `parse-tipitaka-xml`

2. **How to handle XML variants (commentaries .att, .tik)?**
   - **Recommendation:** Detect via filename suffix, apply to UID generation
   - Same fragment structure, different UID format

3. **Should fragments be serializable to disk?**
   - **Recommendation:** Yes, add `--output-fragments` flag for debugging
   - Serialize as JSON for inspection

4. **How to handle malformed XML?**
   - **Recommendation:** Skip malformed files with error logging
   - Provide `--strict` flag to abort on errors

## Future Enhancements

### Phase 2 Features (Future)
- Fragment-based search (find sutta by line number)
- XML diff tool (compare versions)
- Commentary linking (cross-reference .mul, .att, .tik files)
- Structure validation (verify nikaya hierarchy consistency)

### Phase 3 Features (Future)
- Non-VRI XML format support
- Parallel file processing
- Incremental updates (only changed files)
- Fragment-based editing tools

## Appendix

### Sample Nikaya Structures

**Dīgha Nikāya (DN):**
```
Dīghanikāyo (Nikaya)
├── Sīlakkhandhavaggapāḷi (Book)
│   ├── Brahmajālasutta (Sutta)
│   ├── Sāmaññaphalasutta (Sutta)
│   └── ...
├── Mahāvaggapāḷi (Book)
│   └── ...
└── Pāthikavaggapāḷi (Book)
    └── ...
```

**Majjhima Nikāya (MN):**
```
Majjhimanikāyo (Nikaya)
├── Mūlapaṇṇāsapāḷi (Book)
│   ├── Mūlapariyāyavaggo (Vagga)
│   │   ├── Mūlapariyāyasutta (Sutta)
│   │   ├── Sabbāsavasutta (Sutta)
│   │   └── ...
│   ├── Sīhanādavaggo (Vagga)
│   └── ...
└── ...
```

**Saṃyutta Nikāya (SN):**
```
Saṃyuttanikāyo (Nikaya)
├── Sagāthāvaggo (Book)
│   ├── Devatāsaṃyutta (Saṃyutta)
│   │   ├── Naḷavaggo (Vagga)
│   │   │   ├── Oghataraṇasutta (Sutta)
│   │   │   └── ...
│   │   └── ...
│   └── ...
└── ...
```

### Example Nikaya Detection

**Input XML (Dīgha Nikāya with -yo ending):**
```xml
<?xml version="1.0" encoding="UTF-8"?>
<TEI>
  <text>
    <body>
      <p rend="nikaya">Dīghanikāyo</p>
      <div type="book" id="dn1">
        <head rend="book">Sīlakkhandhavaggapāḷi</head>
        ...
```

**Detection Result:**
```rust
NikayaStructure {
    nikaya: "Dīghanikāyo".to_string(),
    levels: vec![GroupType::Nikaya, GroupType::Book, GroupType::Sutta],
}
```

**Input XML (Majjhima Nikāya with -ye ending):**
```xml
<?xml version="1.0" encoding="UTF-8"?>
<TEI>
  <text>
    <body>
      <p rend="nikaya">Majjhimanikāye</p>
      <div type="book" id="mn1">
        <head rend="book">Mūlapaṇṇāsapāḷi</head>
        ...
```

**Detection Result:**
```rust
NikayaStructure {
    nikaya: "Majjhimanikāyo".to_string(),
    levels: vec![GroupType::Nikaya, GroupType::Book, GroupType::Vagga, GroupType::Sutta],
}
```

### Example Fragment Output

```json
{
  "fragment_type": "Sutta",
  "content": "<p rend=\"subhead\">1. Mūlapariyāyasuttaṃ</p>\n<p rend=\"bodytext\" n=\"1\"><hi rend=\"paranum\">1</hi><hi rend=\"dot\">.</hi> Evaṃ me sutaṃ...</p>",
  "start_line": 42,
  "end_line": 156,
  "group_levels": [
    {
      "group_type": "Nikaya",
      "group_number": null,
      "title": "Majjhimanikāyo",
      "id": null
    },
    {
      "group_type": "Book",
      "group_number": 1,
      "title": "Mūlapaṇṇāsapāḷi",
      "id": "mn1"
    },
    {
      "group_type": "Vagga",
      "group_number": 1,
      "title": "1. Mūlapariyāyavaggo",
      "id": "mn1_1"
    },
    {
      "group_type": "Sutta",
      "group_number": 1,
      "title": "1. Mūlapariyāyasuttaṃ",
      "id": null
    }
  ]
}
```

---

**Document Version:** 1.1
**Last Updated:** 2025-10-31
**Changes:** Updated to reflect Phase 1 as nikaya detection with hard-coded structure mappings
