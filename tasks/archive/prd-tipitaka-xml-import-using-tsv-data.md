# PRD: Tipitaka XML Import Feature Using TSV Data

## Overview

Import and process Pāli suttas from VRI CST Tipitaka XML files into the Simsapa appdata SQLite database. The feature will parse idiosyncratic XML structure from the romn/ folder, transform XML tags to HTML/CSS based on tipitaka.org's XSL transform, and store suttas in the existing appdata schema.

## Motivation

The VRI CST (Vipassana Research Institute - Chaṭṭha Saṅgāyana Tipiṭaka) provides a comprehensive collection of Pāli texts in XML format. Currently, Simsapa imports suttas from SuttaCentral ArangoDB. Adding VRI CST as an additional source will:

- Provide an alternative high-quality Pāli text source
- Enable offline processing without external database dependencies
- Support the Majjhima Nikāya initially, with extensibility for other collections
- Preserve the authoritative VRI text formatting and structure

## Goals

1. Parse VRI CST Tipitaka XML files from `romn/` folder into structured Rust data
2. Transform XML tags to HTML/CSS following tipitaka.org's XSL transform patterns
3. Extract hierarchical sutta organization (nikaya → book → vagga → sutta)
4. Store parsed suttas in the existing appdata schema
5. Expose functionality via CLI command `parse-tipitaka-xml`
6. Support batch processing (entire folder) or single file processing
7. Design for extensibility to other nikayas beyond Majjhima Nikāya

## Non-Goals

- Storing the complete nikaya hierarchical structure in the database (only extract to populate sutta records)
- Supporting non-romn encodings (only romn/ folder)
- Creating a new database schema (use existing appdata_models)
- GUI for XML import (CLI only)
- Real-time/streaming import (batch processing acceptable)

## User Stories

### Story 1: Process Single File
As a developer, I want to process a single Tipitaka XML file so that I can test and debug the parsing logic on a specific sutta collection.

**Acceptance Criteria:**
- CLI command accepts a file path argument
- Detects and converts UTF-16LE encoding to UTF-8
- Normalizes CRLF line endings to Unix LF
- Parses the file and extracts all suttas with correct Pāli diacritics
- Inserts suttas into appdata.sqlite3 database
- Outputs progress and summary statistics (including detected encoding)
- Handles errors gracefully with clear error messages

### Story 2: Batch Process Entire Folder
As a developer, I want to process all XML files in the romn/ folder so that I can import the complete Tipitaka collection.

**Acceptance Criteria:**
- CLI command processes all .xml files in the folder
- Skips non-XML or malformed files with warnings
- Reports progress per file
- Provides summary statistics (total files, suttas imported, errors)
- Continues processing on individual file errors (fail-safe)

### Story 3: Parse Majjhima Nikāya Structure
As a developer, I want the parser to correctly extract the Majjhima Nikāya's hierarchical structure so that suttas are properly organized.

**Acceptance Criteria:**
- Extracts nikaya name from `<p rend="nikaya">`
- Extracts book name from `<div type="book">` and `<head rend="book">`
- Extracts vagga name from `<div type="vagga">` and `<head rend="chapter">`
- Identifies sutta boundaries using `<p rend="subhead">`
- Populates `group_path` field with hierarchy (e.g., "Majjhimanikāyo/Mūlapaṇṇāsapāḷi/Mūlapariyāyavaggo")

### Story 4: Transform XML to HTML
As a developer, I want XML tags transformed to proper HTML so that sutta content displays correctly in the app.

**Acceptance Criteria:**
- Implements transformation rules from tipitaka.org XSL
- Maps `<p rend="bodytext">` to `<p class="bodytext">`
- Maps `<p rend="nikaya">` to `<p class="nikaya">`
- Maps `<p rend="centre">` to `<p class="centered">`
- Handles other rend types (book, chapter, subhead, gatha*, etc.)
- Preserves `<hi>` tags with proper class attributes
- Handles `<note>` tags with proper formatting
- Preserves `<pb>` page break references

## Technical Design

### Architecture

```
CLI Command (parse-tipitaka-xml)
    ↓
TipitakaXmlParser Module (backend/src/tipitaka_xml_parser_tsv.rs)
    ↓
Parsed Structs (TipitakaCollection → Book → Vagga → Sutta)
    ↓
HTML Transformer
    ↓
Database Inserter (uses appdata_models::NewSutta)
    ↓
appdata.sqlite3
```

### Data Structures

```rust
// Represents the full hierarchical structure
pub struct TipitakaCollection {
    pub nikaya: String,          // "Majjhimanikāyo"
    pub books: Vec<Book>,
}

pub struct Book {
    pub id: String,              // "mn1"
    pub title: String,           // "Mūlapaṇṇāsapāḷi"
    pub vaggas: Vec<Vagga>,
}

pub struct Vagga {
    pub id: String,              // "mn1_1"
    pub title: String,           // "1. Mūlapariyāyavaggo"
    pub suttas: Vec<Sutta>,
}

pub struct Sutta {
    pub title: String,                   // "1. Mūlapariyāyasuttaṃ"
    pub content_xml: Vec<XmlElement>,    // Raw XML elements
    pub metadata: SuttaMetadata,
}

pub struct SuttaMetadata {
    pub uid: String,                     // Generated from position
    pub sutta_ref: String,               // Reference number
    pub nikaya: String,
    pub group_path: String,              // Full hierarchy path
    pub group_index: Option<i32>,
    pub order_index: Option<i32>,
}

// For XML parsing
pub enum XmlElement {
    Paragraph { rend: String, n: Option<String>, content: Vec<ContentNode> },
    HighlightedText { rend: String, content: String },
    Note { content: String },
    PageBreak { ed: String, n: String },
}

pub enum ContentNode {
    Text(String),
    Hi(String, String),  // (rend, text)
    Note(String),
    PageBreak { ed: String, n: String },
}
```

### XML to HTML Transformation Rules

Based on the XSL transform at tipitaka.org/romn/tipitaka.xsl:

| XML | HTML Output | CSS Class |
|-----|-------------|-----------|
| `<p rend="nikaya">` | `<p class="nikaya">` | 24pt, centered, bold |
| `<p rend="book">` | `<p class="book">` | 21pt, centered, bold |
| `<p rend="chapter">` | `<p class="chapter">` | 18pt, centered, bold |
| `<p rend="subhead">` | `<p class="subhead">` | 16pt, centered, bold |
| `<p rend="title">` | `<p class="title">` | 16pt, centered, bold |
| `<p rend="centre">` | `<p class="centered">` | 12pt, centered |
| `<p rend="bodytext">` | `<p class="bodytext">` | 12pt, 2em indent |
| `<p rend="gatha1">` | `<p class="gatha1">` | 12pt, 4em left margin |
| `<p rend="gatha2">` | `<p class="gatha2">` | 12pt, 4em left margin |
| `<p rend="gatha3">` | `<p class="gatha3">` | 12pt, 4em left margin |
| `<p rend="gathalast">` | `<p class="gathalast">` | 12pt, 4em left margin |
| `<hi rend="paranum">` | `<span class="paranum">` | bold, 14pt |
| `<hi rend="dot">` | `<span class="dot">` | bold, 14pt |
| `<hi rend="bold">` | `<span class="bold">` | bold |
| `<note>` | `<span class="note">` | blue, with [ ] brackets |
| `<pb ed="X" n="Y">` | `<span class="pagebreak" data-ed="X" data-n="Y"></span>` | Hidden metadata |

### Database Mapping

Map parsed sutta to `appdata_models::NewSutta`:

```rust
NewSutta {
    uid: generated_uid,              // e.g., "mn1.1" or "vri-cst/mn1.1"
    sutta_ref: sutta_number,         // e.g., "MN 1"
    nikaya: collection.nikaya,       // "Majjhimanikāyo"
    language: "pli",
    group_path: Some(full_path),     // "Majjhimanikāyo/Mūlapaṇṇāsapāḷi/Mūlapariyāyavaggo"
    group_index: Some(vagga_index),
    order_index: Some(sutta_index),
    sutta_range_group: None,
    sutta_range_start: None,
    sutta_range_end: None,
    title: Some(sutta.title),
    title_pali: Some(sutta.title),   // Same as title for Pāli
    title_ascii: None,
    title_trans: None,
    description: None,
    content_plain: Some(plain_text), // Stripped of HTML
    content_html: Some(html),        // Transformed HTML
    content_json: None,
    content_json_tmpl: None,
    source_uid: Some("vri-cst"),
    source_info: Some("VRI CST Tipitaka romn"),
    source_language: Some("pli"),
    message: None,
    copyright: Some("VRI"),
    license: None,
}
```

### CLI Command Specification

```bash
# Process a single file
simsapa-cli parse-tipitaka-xml --file path/to/s0201m.mul.xml

# Process entire romn/ folder
simsapa-cli parse-tipitaka-xml --folder path/to/romn/

# Default: process the configured romn/ folder
simsapa-cli parse-tipitaka-xml

# Options
--file <PATH>           Process a single XML file
--folder <PATH>         Process all XML files in folder
--verbose               Show detailed progress
--dry-run               Parse without database insertion
```

### File Organization

```
cli/src/
  main.rs                        # Add ParseTipitakaXml command
  tipitaka_xml_parser_tsv/
    mod.rs                       # Module exports
    types.rs                     # Data structures
    encoding.rs                  # Character encoding detection & conversion
    xml_parser.rs                # XML parsing logic
    html_transformer.rs          # XML → HTML transformation
    database_inserter.rs         # Database insertion
    uid_generator.rs             # UID generation logic
```

**Note:** The parser is in the CLI module because it's only needed for database bootstrapping, not for runtime application functionality.

### Character Encoding Handling

The VRI CST Tipitaka XML files use **UTF-16LE with BOM (Byte Order Mark)** encoding and **CRLF (Windows)** line endings, as indicated by the Emacs modeline: `utf-16le-with-signature`.

**Detection and Conversion Strategy:**

1. **Read as raw bytes** - Do not use `std::fs::read_to_string()` which assumes UTF-8
2. **Detect encoding** - Check for UTF-16LE BOM (0xFF 0xFE) at file start
3. **Convert to UTF-8** - Use `encoding_rs` crate for robust conversion
4. **Normalize line endings** - Convert CRLF to LF (Unix-style)
5. **Return clean UTF-8 string** - Ready for XML parsing

**Implementation:**

```rust
use encoding_rs::{Encoding, UTF_16LE};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use anyhow::{Context, Result};

/// Reads an XML file, detects encoding, and converts to UTF-8 with Unix line endings
pub fn read_xml_file(path: &Path) -> Result<String> {
    // Read file as raw bytes
    let mut file = File::open(path)
        .context(format!("Failed to open file: {:?}", path))?;
    
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .context(format!("Failed to read file: {:?}", path))?;
    
    // Detect encoding by checking BOM
    let (encoding, has_bom) = detect_encoding(&bytes);
    
    tracing::debug!(
        "File: {:?}, Encoding: {}, BOM: {}",
        path.file_name().unwrap_or_default(),
        encoding.name(),
        has_bom
    );
    
    // Skip BOM bytes if present
    let bytes_without_bom = if has_bom {
        match encoding {
            encoding if encoding == UTF_16LE => &bytes[2..], // UTF-16LE BOM is 2 bytes
            _ => &bytes[3..], // UTF-8 BOM is 3 bytes
        }
    } else {
        &bytes
    };
    
    // Decode to UTF-8
    let (decoded, _encoding_used, had_errors) = encoding.decode(bytes_without_bom);
    
    if had_errors {
        tracing::warn!("Encoding errors detected while decoding {:?}", path);
    }
    
    // Convert CRLF to LF (Windows to Unix line endings)
    let unix_text = decoded.replace("\r\n", "\n");
    
    Ok(unix_text)
}

/// Detects file encoding by examining BOM (Byte Order Mark)
fn detect_encoding(bytes: &[u8]) -> (&'static Encoding, bool) {
    // UTF-16LE BOM: 0xFF 0xFE
    if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
        return (UTF_16LE, true);
    }
    
    // UTF-16BE BOM: 0xFE 0xFF
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        return (encoding_rs::UTF_16BE, true);
    }
    
    // UTF-8 BOM: 0xEF 0xBB 0xBF
    if bytes.len() >= 3 && bytes[0] == 0xEF && bytes[1] == 0xBB && bytes[2] == 0xBF {
        return (encoding_rs::UTF_8, true);
    }
    
    // No BOM detected, assume UTF-8
    (encoding_rs::UTF_8, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_utf16le_bom() {
        let bytes = vec![0xFF, 0xFE, 0x41, 0x00]; // UTF-16LE "A"
        let (encoding, has_bom) = detect_encoding(&bytes);
        assert_eq!(encoding, UTF_16LE);
        assert!(has_bom);
    }

    #[test]
    fn test_detect_utf8_no_bom() {
        let bytes = vec![0x41, 0x42, 0x43]; // "ABC"
        let (encoding, has_bom) = detect_encoding(&bytes);
        assert_eq!(encoding, encoding_rs::UTF_8);
        assert!(!has_bom);
    }

    #[test]
    fn test_crlf_to_lf_conversion() {
        let input = "Line 1\r\nLine 2\r\nLine 3";
        let output = input.replace("\r\n", "\n");
        assert_eq!(output, "Line 1\nLine 2\nLine 3");
    }
}
```

**Why This Matters:**

1. **Correct character decoding** - Pāli diacritics (ā, ī, ū, ṃ, ṅ, ñ, ṭ, ḍ, ṇ, ḷ) must be preserved
2. **XML parsing compatibility** - Most XML parsers expect UTF-8
3. **Cross-platform consistency** - Unix LF line endings work everywhere
4. **Performance** - `encoding_rs` is optimized for SIMD operations
5. **Error detection** - Identifies corrupted or misencoded files

**Alternative Approaches Considered:**

| Approach | Pros | Cons |
|----------|------|------|
| `std::fs::read_to_string()` | Simple | Fails on UTF-16, assumes UTF-8 |
| `iconv` command-line | Available | Requires external dependency, slower |
| Manual byte parsing | No dependencies | Error-prone, reinventing wheel |
| `encoding_rs` crate | Fast, robust, well-tested | Small dependency added |

**Decision:** Use `encoding_rs` for production-grade encoding handling.

### Error Handling

Use `anyhow::Result` throughout with context for:

- File not found
- File read errors (permissions, I/O errors)
- Invalid XML structure
- Encoding detection failures
- Character decoding errors
- Missing required elements (nikaya, book, vagga)
- Database connection errors
- Insertion failures
- Duplicate UIDs

Example:
```rust
fn parse_xml_file(path: &Path) -> anyhow::Result<TipitakaCollection> {
    // Read and convert encoding
    let utf8_content = read_xml_file(path)
        .context(format!("Failed to read and decode XML file: {:?}", path))?;
    
    // Parse XML
    parse_xml(&utf8_content)
        .context("Failed to parse XML structure")?
}
```

### UID Generation Strategy

Generate unique UIDs for suttas:

1. **Majjhima Nikāya**: Use standard SuttaCentral-style UIDs
   - Pattern: `mn{book_number}.{sutta_number}`
   - Example: `mn1.1` for first sutta
   - Infer book and sutta numbers from file structure and titles

2. **Prefix with source**: To avoid conflicts with SuttaCentral imports
   - Pattern: `vri-cst/mn{book_number}.{sutta_number}`
   - Example: `vri-cst/mn1.1`

3. **Fallback**: If number extraction fails
   - Use file name + sequential index
   - Example: `vri-cst/s0201m.mul/001`

### Extensibility for Other Nikayas

The design should support different structural patterns:

```rust
// Enum to handle different nikaya structures
pub enum NikayaStructure {
    // Majjhima: nikaya → book → vagga → sutta
    ThreeTier {
        nikaya: String,
        book_divs: Vec<BookDiv>,
    },
    
    // Dīgha, Saṃyutta (potentially different)
    TwoTier {
        nikaya: String,
        vagga_divs: Vec<VaggaDiv>,
    },
    
    // Aṅguttara (potentially different)
    FourTier {
        nikaya: String,
        nipata_divs: Vec<NipataDiv>,
    },
}

// Parser detects structure automatically or via config
fn detect_nikaya_structure(xml: &str) -> anyhow::Result<NikayaStructure> {
    // Analyze div types and nesting
    // Return appropriate structure
}
```

## Implementation Plan

### Phase 1: XML Parsing Core (Initial PR)
1. Create `tipitaka_xml_parser_tsv` module structure
2. Implement data structures in `types.rs`
3. Implement character encoding detection and conversion (UTF-16LE to UTF-8, CRLF to LF)
4. Implement XML parsing for Majjhima Nikāya structure
5. Write unit tests with sample XML snippets and encoding tests

### Phase 2: HTML Transformation (Initial PR)
1. Implement `html_transformer.rs`
2. Map all XML tags to HTML based on XSL transform
3. Embed CSS classes
4. Generate plain text content (strip HTML)
5. Write unit tests for transformation rules

### Phase 3: Database Integration (Initial PR)
1. Implement UID generation logic
2. Implement `database_inserter.rs`
3. Map to `NewSutta` struct
4. Handle duplicate detection
5. Write integration tests

### Phase 4: CLI Command (Initial PR)
1. Add `ParseTipitakaXml` command to `cli/src/main.rs`
2. Implement file and folder processing
3. Add progress reporting
4. Add dry-run mode
5. Add verbose logging

### Phase 5: Testing & Documentation (Follow-up PR)
1. Test with complete `s0201m.mul.xml` file
2. Test batch processing
3. Document usage in README
4. Add examples to CLI help
5. Performance optimization if needed

### Phase 6: Extensibility (Future PR)
1. Test with other nikaya XML files
2. Implement structure detection
3. Support additional nikaya patterns
4. Generalize parser for all nikayas

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // === Encoding Tests ===
    
    #[test]
    fn test_detect_utf16le_bom() {
        let bytes = vec![0xFF, 0xFE, 0x41, 0x00]; // UTF-16LE BOM + "A"
        let (encoding, has_bom) = detect_encoding(&bytes);
        assert_eq!(encoding, UTF_16LE);
        assert!(has_bom);
    }

    #[test]
    fn test_detect_utf8_bom() {
        let bytes = vec![0xEF, 0xBB, 0xBF, 0x41]; // UTF-8 BOM + "A"
        let (encoding, has_bom) = detect_encoding(&bytes);
        assert_eq!(encoding, encoding_rs::UTF_8);
        assert!(has_bom);
    }

    #[test]
    fn test_detect_no_bom() {
        let bytes = vec![0x41, 0x42, 0x43]; // "ABC" in ASCII/UTF-8
        let (encoding, has_bom) = detect_encoding(&bytes);
        assert_eq!(encoding, encoding_rs::UTF_8);
        assert!(!has_bom);
    }

    #[test]
    fn test_convert_crlf_to_lf() {
        let input = "Line 1\r\nLine 2\r\nLine 3\r\n";
        let output = input.replace("\r\n", "\n");
        assert_eq!(output, "Line 1\nLine 2\nLine 3\n");
    }

    #[test]
    fn test_preserve_pali_diacritics() {
        // Test UTF-16LE encoding of Pāli text with diacritics
        let pali_text = "Majjhimanikāyo";
        let utf16_bytes: Vec<u8> = {
            let mut bytes = vec![0xFF, 0xFE]; // BOM
            for c in pali_text.encode_utf16() {
                bytes.push((c & 0xFF) as u8);
                bytes.push((c >> 8) as u8);
            }
            bytes
        };
        
        let (encoding, _) = detect_encoding(&utf16_bytes);
        let (decoded, _, _) = encoding.decode(&utf16_bytes[2..]); // Skip BOM
        assert_eq!(decoded, pali_text);
        assert!(decoded.contains('ā')); // Verify diacritic preserved
    }

    // === XML Parsing Tests ===

    #[test]
    fn test_parse_nikaya_heading() {
        let xml = r#"<p rend="nikaya">Majjhimanikāyo</p>"#;
        let result = parse_nikaya(xml).unwrap();
        assert_eq!(result, "Majjhimanikāyo");
    }

    #[test]
    fn test_parse_book_div() {
        let xml = r#"
            <div id="mn1" n="mn1" type="book">
                <head rend="book">Mūlapaṇṇāsapāḷi</head>
            </div>
        "#;
        let result = parse_book(xml).unwrap();
        assert_eq!(result.id, "mn1");
        assert_eq!(result.title, "Mūlapaṇṇāsapāḷi");
    }

    // === HTML Transformation Tests ===

    #[test]
    fn test_transform_bodytext_to_html() {
        let xml = r#"<p rend="bodytext" n="1">Content here</p>"#;
        let html = transform_to_html(xml).unwrap();
        assert_eq!(html, r#"<p class="bodytext">Content here</p>"#);
    }

    #[test]
    fn test_transform_note_to_html() {
        let xml = r#"<note>variant reading</note>"#;
        let html = transform_to_html(xml).unwrap();
        assert_eq!(html, r#"<span class="note">[variant reading]</span>"#);
    }

    #[test]
    fn test_transform_pali_with_diacritics() {
        let xml = r#"<p rend="bodytext">Evaṃ me sutaṃ</p>"#;
        let html = transform_to_html(xml).unwrap();
        assert!(html.contains("Evaṃ"));
        assert!(html.contains("sutaṃ"));
    }

    // === UID Generation Tests ===

    #[test]
    fn test_generate_uid() {
        let metadata = SuttaMetadata {
            book_number: 1,
            vagga_number: 1,
            sutta_number: 1,
            ..Default::default()
        };
        let uid = generate_uid(&metadata, "vri-cst");
        assert_eq!(uid, "vri-cst/mn1.1");
    }
}
```

### Integration Tests

```rust
// cli/tests/test_parse_tipitaka_xml.rs
#[test]
fn test_parse_single_file() {
    let test_file = "tests/data/sample_mn.xml";
    let db_path = create_test_database();
    
    let result = parse_tipitaka_xml_file(test_file, &db_path);
    assert!(result.is_ok());
    
    // Verify suttas inserted
    let conn = connect_to_db(&db_path);
    let count = get_sutta_count(&conn, "vri-cst");
    assert!(count > 0);
}

#[test]
fn test_parse_folder() {
    let test_folder = "tests/data/romn_sample/";
    let db_path = create_test_database();
    
    let result = parse_tipitaka_xml_folder(test_folder, &db_path);
    assert!(result.is_ok());
    
    let stats = result.unwrap();
    assert_eq!(stats.files_processed, 3);
    assert_eq!(stats.suttas_imported, 15);
}
```

### Manual Testing Checklist

- [ ] Parse `s0201m.mul.xml` successfully
- [ ] Verify UTF-16LE with BOM encoding detected correctly
- [ ] Verify CRLF line endings converted to LF
- [ ] Verify all Pāli diacritics preserved (ā, ī, ū, ṃ, ṅ, ñ, ṭ, ḍ, ṇ, ḷ)
- [ ] Verify all suttas extracted (check count matches expected)
- [ ] Verify HTML output renders correctly in app
- [ ] Test with UTF-8 file (should handle gracefully)
- [ ] Test with malformed XML (graceful error)
- [ ] Test with missing elements (graceful error)
- [ ] Test batch processing with multiple files
- [ ] Test dry-run mode (no database changes)
- [ ] Test verbose output
- [ ] Verify no duplicate UIDs

## Success Metrics

- Successfully detect and convert UTF-16LE with BOM encoding to UTF-8
- Convert CRLF line endings to Unix LF format
- Preserve all Pāli diacritical marks (ā, ī, ū, ṃ, ṅ, ñ, ṭ, ḍ, ṇ, ḷ) correctly
- Successfully parse `s0201m.mul.xml` extracting all suttas
- Transform XML to valid HTML following tipitaka.org patterns
- Insert suttas into appdata.sqlite3 with proper metadata
- CLI command works for both single file and folder
- Error handling provides clear, actionable messages
- Code is extensible for other nikaya structures
- Documentation enables other developers to use the feature

## Open Questions

1. **UID Conflicts**: Should we always prefix with `vri-cst/` or detect conflicts?
   - **Decision**: Use `vri-cst/` prefix to avoid conflicts with SuttaCentral imports

2. **CSS Handling**: Should we embed CSS inline or reference external stylesheet?
   - **Decision**: Use CSS classes, stylesheet already exists in assets/css/

3. **Sutta Numbering**: How to extract sutta numbers from titles like "1. Mūlapariyāyasuttaṃ"?
   - **Decision**: Regex extraction, fallback to sequential numbering

4. **Progress Reporting**: Should we use a progress bar or simple line output?
   - **Decision**: Start with simple line output, can enhance later

5. **Parallel Processing**: Should folder processing be parallel or sequential?
   - **Decision**: Sequential for initial implementation, parallel in future optimization

6. **Duplicate Handling**: What to do if sutta UID already exists?
   - **Decision**: Skip with warning, add `--overwrite` flag for future

## Related Documentation

- [Appdata Schema](../backend/src/db/appdata_models.rs)
- [CLI Commands](../cli/src/main.rs)
- [Diesel ORM Docs](https://diesel.rs/)
- [Tipitaka.org XSL Transform](https://tipitaka.org/romn/tipitaka.xsl)
- [VRI CST Tipitaka](https://tipitaka.org/)

## Dependencies

### New Crate Dependencies

```toml
# backend/Cargo.toml
[dependencies]
quick-xml = "0.31"           # For XML parsing
encoding_rs = "0.8"          # For UTF-16 conversion
regex = "1.10"               # For text extraction
html-escape = "0.2"          # For HTML entity handling
```

## Future Enhancements

1. **Support All Nikayas**: Extend to Dīgha, Saṃyutta, Aṅguttara, Khuddaka
2. **Parallel Processing**: Use rayon for folder batch processing
3. **Incremental Updates**: Track processed files, skip unchanged
4. **Validation Mode**: Verify parsed content against checksums
5. **Commentary Support**: Parse .att.xml (Aṭṭhakathā) files
6. **Sub-commentary Support**: Parse .tik.xml (Ṭīkā) files
7. **Cross-references**: Link suttas to related content
8. **Verse Extraction**: Special handling for gatha verses
9. **Configuration File**: Allow custom mapping rules
10. **Web Interface**: Future GUI for import management

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| XML structure varies more than expected | High | Design flexible parser with structure detection |
| UTF-16 encoding issues | Medium | Use robust encoding_rs library, test thoroughly |
| Large file memory usage | Medium | Stream parsing if needed, process in chunks |
| Database insertion performance | Low | Batch inserts, use transactions |
| UID collision with existing data | Medium | Use source prefix, add conflict detection |

## Appendix A: Sample XML Structure

```xml
<?xml version="1.0" encoding="UTF-16"?>
<TEI.2>
<teiHeader></teiHeader>
<text>
<body xml:space="preserve">

<p rend="centre">Namo tassa bhagavato arahato sammāsambuddhassa</p>

<p rend="nikaya">Majjhimanikāyo</p>

<div id="mn1" n="mn1" type="book">
  <head rend="book">Mūlapaṇṇāsapāḷi</head>
  
  <div id="mn1_1" n="mn1_1" type="vagga">
    <head rend="chapter">1. Mūlapariyāyavaggo</head>
    
    <p rend="subhead">1. Mūlapariyāyasuttaṃ</p>
    
    <p rend="bodytext" n="1">
      <hi rend="paranum">1</hi><hi rend="dot">.</hi> Evaṃ
      <pb ed="M" n="1.0001" />
      me sutaṃ – ekaṃ samayaṃ bhagavā ukkaṭṭhāyaṃ viharati...
    </p>
    
    <p rend="bodytext">
      Āpaṃ āpato sañjānāti<note>variant</note>...
    </p>
    
    <p rend="subhead">2. Sabbāsavasuttaṃ</p>
    
    <!-- Next sutta content -->
    
  </div>
</div>

</body>
</text>
</TEI.2>
```

## Appendix B: Example CLI Output

```bash
$ simsapa-cli parse-tipitaka-xml --file s0201m.mul.xml --verbose

Parsing Tipitaka XML file: s0201m.mul.xml

Encoding Detection:
  Detected: UTF-16LE with BOM
  Line endings: CRLF (Windows)
  Converting to UTF-8 with LF (Unix)...
  ✓ Encoding conversion successful

Parsing XML structure...

Found nikaya: Majjhimanikāyo
Found book: Mūlapaṇṇāsapāḷi (id: mn1)
Found vagga: 1. Mūlapariyāyavaggo (id: mn1_1)

Processing suttas:
  ✓ 1. Mūlapariyāyasuttaṃ (uid: vri-cst/mn1.1)
  ✓ 2. Sabbāsavasuttaṃ (uid: vri-cst/mn1.2)
  ✓ 3. Dhammadāyādasuttaṃ (uid: vri-cst/mn1.3)
  ...

Summary:
  Files processed: 1
  Suttas imported: 10
  Errors: 0
  Duration: 2.3s

✓ Successfully imported suttas to appdata.sqlite3
```
