# Implementation Tasks: Tipitaka XML Import

Generated from: `tasks/prd-tipitaka-xml-import.md`

## Overview

Implement VRI CST Tipitaka XML parser to import Pāli suttas from the romn/ folder into the appdata SQLite database. The implementation follows the phased approach from the PRD and uses patterns from existing importers like `NyanadipaImporter`.

## Task List

### Phase 1: Project Setup and Module Structure

#### Task 1.1: Create Module Structure
**Priority:** P0 (Blocking)  
**Estimated Effort:** 30 minutes

Create the module structure for the Tipitaka XML parser.

**Files to Create:**
- `backend/src/tipitaka_xml_parser.rs` (main module file)
- `backend/src/tipitaka_xml_parser/mod.rs` (re-exports)
- `backend/src/tipitaka_xml_parser/types.rs` (data structures)
- `backend/src/tipitaka_xml_parser/encoding.rs` (UTF-16 handling)
- `backend/src/tipitaka_xml_parser/xml_parser.rs` (XML parsing)
- `backend/src/tipitaka_xml_parser/html_transformer.rs` (XML to HTML)
- `backend/src/tipitaka_xml_parser/database_inserter.rs` (DB insertion)
- `backend/src/tipitaka_xml_parser/uid_generator.rs` (UID generation)

**Files to Modify:**
- `backend/src/lib.rs` - Add `pub mod tipitaka_xml_parser;`
- `backend/Cargo.toml` - Add dependencies (see Task 1.2)

**Acceptance Criteria:**
- [ ] Module structure exists and compiles
- [ ] Empty module files with appropriate module declarations
- [ ] Module is accessible from backend crate

**Dependencies:** None

---

#### Task 1.2: Add Dependencies
**Priority:** P0 (Blocking)  
**Estimated Effort:** 15 minutes

Add required crate dependencies to backend/Cargo.toml.

**Files to Modify:**
- `backend/Cargo.toml`

**Changes:**
```toml
[dependencies]
# XML parsing
quick-xml = "0.31"

# Character encoding detection and conversion
encoding_rs = "0.8"

# Pattern matching for text extraction
regex = "1.10"

# HTML entity handling
html-escape = "0.2"
```

**Acceptance Criteria:**
- [ ] Dependencies added to Cargo.toml
- [ ] `cargo build` in backend/ succeeds
- [ ] No version conflicts with existing dependencies

**Dependencies:** None

---

### Phase 2: Data Structures and Types

#### Task 2.1: Implement Core Data Structures
**Priority:** P0 (Blocking)  
**Estimated Effort:** 1 hour

Implement the hierarchical data structures for representing Tipitaka XML content.

**Files to Modify:**
- `backend/src/tipitaka_xml_parser/types.rs`

**Implementation:**
```rust
use serde::{Deserialize, Serialize};

/// Represents the full hierarchical structure of a nikaya
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TipitakaCollection {
    pub nikaya: String,          // "Majjhimanikāyo"
    pub books: Vec<Book>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Book {
    pub id: String,              // "mn1"
    pub title: String,           // "Mūlapaṇṇāsapāḷi"
    pub vaggas: Vec<Vagga>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vagga {
    pub id: String,              // "mn1_1"
    pub title: String,           // "1. Mūlapariyāyavaggo"
    pub suttas: Vec<Sutta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sutta {
    pub title: String,                   // "1. Mūlapariyāyasuttaṃ"
    pub content_xml: Vec<XmlElement>,    // Raw XML elements
    pub metadata: SuttaMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuttaMetadata {
    pub uid: String,                     // Generated from position
    pub sutta_ref: String,               // Reference number (e.g., "MN 1")
    pub nikaya: String,
    pub group_path: String,              // Full hierarchy path
    pub group_index: Option<i32>,
    pub order_index: Option<i32>,
}

/// XML content elements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum XmlElement {
    Paragraph { 
        rend: String, 
        n: Option<String>, 
        content: Vec<ContentNode> 
    },
    HighlightedText { 
        rend: String, 
        content: String 
    },
    Note { 
        content: String 
    },
    PageBreak { 
        ed: String, 
        n: String 
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentNode {
    Text(String),
    Hi(String, String),  // (rend, text)
    Note(String),
    PageBreak { ed: String, n: String },
}
```

**Acceptance Criteria:**
- [ ] All structs defined with proper derives
- [ ] Structures match PRD specification
- [ ] Code compiles without warnings
- [ ] Proper documentation comments added

**Dependencies:** Task 1.1

---

### Phase 3: Character Encoding Handling

#### Task 3.1: Implement UTF-16LE Detection and Conversion
**Priority:** P0 (Blocking)  
**Estimated Effort:** 1.5 hours

Implement encoding detection and conversion from UTF-16LE with BOM to UTF-8.

**Files to Modify:**
- `backend/src/tipitaka_xml_parser/encoding.rs`

**Reference:** PRD lines 247-376 (encoding section with complete implementation example)

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
```

**Acceptance Criteria:**
- [ ] `read_xml_file()` correctly reads UTF-16LE files with BOM
- [ ] Function converts to UTF-8 successfully
- [ ] CRLF line endings converted to LF
- [ ] Pāli diacritics preserved (ā, ī, ū, ṃ, ṅ, ñ, ṭ, ḍ, ṇ, ḷ)
- [ ] Proper error handling with context
- [ ] Logging for encoding detection

**Dependencies:** Task 1.2

---

#### Task 3.2: Write Encoding Tests
**Priority:** P1 (High)  
**Estimated Effort:** 1 hour

Write comprehensive unit tests for encoding detection and conversion.

**Files to Modify:**
- `backend/src/tipitaka_xml_parser/encoding.rs` (add test module)

**Tests to Implement:**
- `test_detect_utf16le_bom()` - Detect UTF-16LE BOM correctly
- `test_detect_utf16be_bom()` - Detect UTF-16BE BOM correctly  
- `test_detect_utf8_bom()` - Detect UTF-8 BOM correctly
- `test_detect_no_bom()` - Default to UTF-8 when no BOM
- `test_convert_crlf_to_lf()` - CRLF conversion works
- `test_preserve_pali_diacritics()` - Diacritics preserved through conversion

**Reference:** PRD lines 330-356 (test examples)

**Acceptance Criteria:**
- [ ] All test cases pass
- [ ] Edge cases covered (empty files, corrupted BOM)
- [ ] Pāli diacritics test verifies: ā, ī, ū, ṃ, ṅ, ñ, ṭ, ḍ, ṇ, ḷ
- [ ] `cargo test` in backend/ passes

**Dependencies:** Task 3.1

---

### Phase 4: XML Parsing Core

#### Task 4.1: Implement XML Parser for Majjhima Nikaya Structure
**Priority:** P0 (Blocking)  
**Estimated Effort:** 3 hours

Parse XML structure to extract nikaya, books, vaggas, and suttas.

**Files to Modify:**
- `backend/src/tipitaka_xml_parser/xml_parser.rs`

**Implementation Strategy:**
1. Use `quick_xml::Reader` for streaming XML parsing
2. Track current hierarchical context (nikaya → book → vagga → sutta)
3. Extract text from `<p rend="nikaya">`, `<head rend="book">`, `<head rend="chapter">`, `<p rend="subhead">`
4. Build hierarchical structure as parsing progresses
5. Store raw XML elements in `XmlElement` enum

**Key Functions:**
```rust
use quick_xml::Reader;
use quick_xml::events::Event;
use anyhow::Result;
use super::types::*;

pub fn parse_xml(xml_content: &str) -> Result<TipitakaCollection> {
    let mut reader = Reader::from_str(xml_content);
    reader.trim_text(true);
    
    // Implementation: streaming parse
    // Track state: current_book, current_vagga, current_sutta
    // Build TipitakaCollection incrementally
    
    todo!()
}

fn parse_paragraph(reader: &mut Reader<&[u8]>, start_tag: &BytesStart) -> Result<XmlElement> {
    // Extract rend attribute
    // Extract n attribute
    // Parse content nodes (text, <hi>, <note>, <pb>)
    todo!()
}
```

**Acceptance Criteria:**
- [ ] Correctly extracts nikaya name from `<p rend="nikaya">`
- [ ] Extracts book name from `<head rend="book">` and id from `<div type="book">`
- [ ] Extracts vagga name from `<head rend="chapter">` and id from `<div type="vagga">`
- [ ] Identifies sutta boundaries using `<p rend="subhead">`
- [ ] Preserves paragraph numbers in `n` attribute
- [ ] Handles nested elements: `<hi>`, `<note>`, `<pb>`
- [ ] Returns complete `TipitakaCollection` structure

**Dependencies:** Task 2.1, Task 3.1

---

#### Task 4.2: Write XML Parsing Tests
**Priority:** P1 (High)  
**Estimated Effort:** 1.5 hours

Write unit tests for XML parsing using sample XML snippets.

**Files to Create:**
- `backend/tests/data/sample_mn.xml` (test data)

**Files to Modify:**
- `backend/src/tipitaka_xml_parser/xml_parser.rs` (add test module)

**Tests to Implement:**
- `test_parse_nikaya_heading()` - Extracts nikaya name
- `test_parse_book_div()` - Extracts book id and title
- `test_parse_vagga_div()` - Extracts vagga id and title
- `test_parse_sutta_subhead()` - Identifies sutta title
- `test_parse_bodytext_paragraph()` - Parses bodytext paragraph with content
- `test_parse_nested_hi_elements()` - Handles `<hi rend="paranum">` etc.
- `test_parse_note_element()` - Handles `<note>` elements
- `test_parse_pagebreak_element()` - Handles `<pb ed="M" n="1.0001">`

**Acceptance Criteria:**
- [ ] All test cases pass
- [ ] Sample XML covers all rend types
- [ ] Tests verify hierarchical structure correctness
- [ ] `cargo test xml_parser` passes

**Dependencies:** Task 4.1

---

### Phase 5: HTML Transformation

#### Task 5.1: Implement XML to HTML Transformer
**Priority:** P0 (Blocking)  
**Estimated Effort:** 2 hours

Transform XML elements to HTML following tipitaka.org XSL transform patterns.

**Files to Modify:**
- `backend/src/tipitaka_xml_parser/html_transformer.rs`

**Reference:** PRD lines 152-172 (transformation table)

**Implementation:**
```rust
use super::types::*;
use anyhow::Result;

/// Transforms a sutta's XML elements to HTML
pub fn transform_to_html(sutta: &Sutta) -> Result<String> {
    let mut html = String::new();
    
    for element in &sutta.content_xml {
        match element {
            XmlElement::Paragraph { rend, n, content } => {
                html.push_str(&transform_paragraph(rend, n.as_deref(), content)?);
            },
            XmlElement::Note { content } => {
                html.push_str(&format!("<span class=\"note\">[{}]</span>", content));
            },
            XmlElement::PageBreak { ed, n } => {
                html.push_str(&format!(
                    "<span class=\"pagebreak\" data-ed=\"{}\" data-n=\"{}\"></span>",
                    ed, n
                ));
            },
            _ => {}
        }
    }
    
    Ok(html)
}

fn transform_paragraph(rend: &str, n: Option<&str>, content: &[ContentNode]) -> Result<String> {
    let class = match rend {
        "nikaya" => "nikaya",
        "book" => "book",
        "chapter" => "chapter",
        "subhead" => "subhead",
        "title" => "title",
        "centre" => "centered",
        "bodytext" => "bodytext",
        "gatha1" => "gatha1",
        "gatha2" => "gatha2",
        "gatha3" => "gatha3",
        "gathalast" => "gathalast",
        _ => rend, // Fallback to original rend
    };
    
    let mut html = format!("<p class=\"{}\">", class);
    
    for node in content {
        match node {
            ContentNode::Text(text) => html.push_str(text),
            ContentNode::Hi(hi_rend, text) => {
                let hi_class = match hi_rend.as_str() {
                    "paranum" => "paranum",
                    "dot" => "dot",
                    "bold" => "bold",
                    _ => hi_rend,
                };
                html.push_str(&format!("<span class=\"{}\">{}</span>", hi_class, text));
            },
            ContentNode::Note(text) => {
                html.push_str(&format!("<span class=\"note\">[{}]</span>", text));
            },
            ContentNode::PageBreak { ed, n } => {
                html.push_str(&format!(
                    "<span class=\"pagebreak\" data-ed=\"{}\" data-n=\"{}\"></span>",
                    ed, n
                ));
            },
        }
    }
    
    html.push_str("</p>");
    Ok(html)
}

/// Generates plain text from HTML by stripping tags
pub fn html_to_plain_text(html: &str) -> String {
    use scraper::{Html, Selector};
    let document = Html::parse_document(html);
    document.root_element().text().collect::<Vec<_>>().join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
```

**Acceptance Criteria:**
- [ ] All XML rend types mapped to correct CSS classes
- [ ] Paragraph elements transformed correctly
- [ ] `<hi>` elements transformed to `<span>` with class
- [ ] `<note>` elements wrapped in brackets with class
- [ ] `<pb>` elements converted to hidden metadata spans
- [ ] Plain text generation strips all HTML
- [ ] Output HTML is valid

**Dependencies:** Task 2.1

---

#### Task 5.2: Write HTML Transformation Tests
**Priority:** P1 (High)  
**Estimated Effort:** 1 hour

Write unit tests for HTML transformation.

**Files to Modify:**
- `backend/src/tipitaka_xml_parser/html_transformer.rs` (add test module)

**Tests to Implement:**
- `test_transform_bodytext_to_html()` - Basic paragraph transformation
- `test_transform_nikaya_to_html()` - Nikaya heading transformation
- `test_transform_hi_paranum()` - `<hi rend="paranum">` transformation
- `test_transform_note_to_html()` - Note transformation with brackets
- `test_transform_pagebreak()` - Page break to metadata span
- `test_transform_pali_with_diacritics()` - Diacritics preserved
- `test_html_to_plain_text()` - Plain text extraction

**Reference:** PRD lines 580-602 (test examples)

**Acceptance Criteria:**
- [ ] All transformation rules tested
- [ ] Diacritics preservation verified
- [ ] Edge cases covered (empty paragraphs, special characters)
- [ ] `cargo test html_transformer` passes

**Dependencies:** Task 5.1

---

### Phase 6: UID Generation

#### Task 6.1: Implement UID Generator
**Priority:** P0 (Blocking)  
**Estimated Effort:** 1.5 hours

Generate unique identifiers for suttas following SuttaCentral patterns with vri-cst prefix.

**Files to Modify:**
- `backend/src/tipitaka_xml_parser/uid_generator.rs`

**Reference:** PRD lines 405-421 (UID generation strategy)

**Implementation:**
```rust
use regex::Regex;
use anyhow::Result;
use super::types::SuttaMetadata;

/// Generates a UID for a sutta
/// Pattern: vri-cst/mn{book_number}.{sutta_number}
/// Example: vri-cst/mn1.1
pub fn generate_uid(metadata: &SuttaMetadata, book_number: u32, vagga_number: u32, sutta_number: u32) -> String {
    let nikaya_abbrev = nikaya_to_abbrev(&metadata.nikaya);
    format!("vri-cst/{}{}.{}", nikaya_abbrev, book_number, sutta_number)
}

/// Extracts sutta number from title
/// Example: "1. Mūlapariyāyasuttaṃ" -> Some(1)
pub fn extract_sutta_number(title: &str) -> Option<u32> {
    let re = Regex::new(r"^(\d+)\.").ok()?;
    re.captures(title)
        .and_then(|cap| cap.get(1))
        .and_then(|m| m.as_str().parse::<u32>().ok())
}

/// Extracts vagga number from title
/// Example: "1. Mūlapariyāyavaggo" -> Some(1)
pub fn extract_vagga_number(title: &str) -> Option<u32> {
    extract_sutta_number(title) // Same pattern
}

/// Converts nikaya name to abbreviation
/// Example: "Majjhimanikāyo" -> "mn"
pub fn nikaya_to_abbrev(nikaya: &str) -> String {
    let lower = nikaya.to_lowercase();
    if lower.starts_with("majjhima") {
        "mn".to_string()
    } else if lower.starts_with("dīgha") || lower.starts_with("digha") {
        "dn".to_string()
    } else if lower.starts_with("saṃyutta") || lower.starts_with("samyutta") {
        "sn".to_string()
    } else if lower.starts_with("aṅguttara") || lower.starts_with("anguttara") {
        "an".to_string()
    } else if lower.starts_with("khuddaka") {
        "kn".to_string()
    } else {
        // Fallback: take first 2 characters
        nikaya.chars().take(2).collect::<String>().to_lowercase()
    }
}

/// Generates sutta_ref from UID
/// Example: "vri-cst/mn1.1" -> "MN 1.1"
pub fn generate_sutta_ref(uid: &str) -> String {
    let re = Regex::new(r"vri-cst/([a-z]+)(\d+)\.(\d+)").unwrap();
    if let Some(cap) = re.captures(uid) {
        let nikaya = cap.get(1).unwrap().as_str().to_uppercase();
        let book = cap.get(2).unwrap().as_str();
        let sutta = cap.get(3).unwrap().as_str();
        format!("{} {}.{}", nikaya, book, sutta)
    } else {
        uid.to_string()
    }
}
```

**Acceptance Criteria:**
- [ ] Generates UIDs with vri-cst prefix
- [ ] Extracts sutta/vagga numbers from Pāli titles correctly
- [ ] Converts nikaya names to abbreviations (mn, dn, sn, an, kn)
- [ ] Generates sutta_ref in format "MN 1.1"
- [ ] Handles edge cases (missing numbers, non-standard titles)

**Dependencies:** Task 2.1

---

#### Task 6.2: Write UID Generation Tests
**Priority:** P1 (High)  
**Estimated Effort:** 45 minutes

Write unit tests for UID generation.

**Files to Modify:**
- `backend/src/tipitaka_xml_parser/uid_generator.rs` (add test module)

**Tests to Implement:**
- `test_generate_uid()` - Basic UID generation
- `test_extract_sutta_number()` - Number extraction from titles
- `test_nikaya_to_abbrev()` - All nikaya abbreviations
- `test_generate_sutta_ref()` - Reference generation
- `test_handle_missing_number()` - Graceful fallback

**Reference:** PRD lines 604-616 (test example)

**Acceptance Criteria:**
- [ ] All test cases pass
- [ ] Edge cases tested (no number, multiple digits)
- [ ] All five nikayas tested
- [ ] `cargo test uid_generator` passes

**Dependencies:** Task 6.1

---

### Phase 7: Database Integration

#### Task 7.1: Implement Database Inserter
**Priority:** P0 (Blocking)  
**Estimated Effort:** 2 hours

Insert parsed suttas into appdata.sqlite3 database.

**Files to Modify:**
- `backend/src/tipitaka_xml_parser/database_inserter.rs`

**Reference:** PRD lines 174-205 (database mapping)

**Implementation:**
```rust
use diesel::prelude::*;
use anyhow::{Context, Result};
use simsapa_backend::db::appdata_models::NewSutta;
use simsapa_backend::db::appdata_schema::suttas;
use simsapa_backend::helpers::{pali_to_ascii, consistent_niggahita};
use super::types::{TipitakaCollection, Book, Vagga, Sutta};
use super::html_transformer::{transform_to_html, html_to_plain_text};
use super::uid_generator::{generate_uid, generate_sutta_ref, nikaya_to_abbrev};

pub fn insert_collection(
    conn: &mut SqliteConnection,
    collection: &TipitakaCollection,
) -> Result<usize> {
    let mut total_inserted = 0;
    
    for (book_idx, book) in collection.books.iter().enumerate() {
        total_inserted += insert_book(conn, collection, book, book_idx as u32 + 1)?;
    }
    
    Ok(total_inserted)
}

fn insert_book(
    conn: &mut SqliteConnection,
    collection: &TipitakaCollection,
    book: &Book,
    book_number: u32,
) -> Result<usize> {
    let mut total_inserted = 0;
    
    for (vagga_idx, vagga) in book.vaggas.iter().enumerate() {
        total_inserted += insert_vagga(
            conn, 
            collection, 
            book, 
            vagga, 
            book_number,
            vagga_idx as u32 + 1
        )?;
    }
    
    Ok(total_inserted)
}

fn insert_vagga(
    conn: &mut SqliteConnection,
    collection: &TipitakaCollection,
    book: &Book,
    vagga: &Vagga,
    book_number: u32,
    vagga_number: u32,
) -> Result<usize> {
    let mut total_inserted = 0;
    
    for (sutta_idx, sutta) in vagga.suttas.iter().enumerate() {
        insert_sutta(
            conn,
            collection,
            book,
            vagga,
            sutta,
            book_number,
            vagga_number,
            sutta_idx as u32 + 1,
        )?;
        total_inserted += 1;
    }
    
    Ok(total_inserted)
}

fn insert_sutta(
    conn: &mut SqliteConnection,
    collection: &TipitakaCollection,
    book: &Book,
    vagga: &Vagga,
    sutta: &Sutta,
    book_number: u32,
    vagga_number: u32,
    sutta_number: u32,
) -> Result<()> {
    // Generate UID and ref
    let uid = generate_uid(&sutta.metadata, book_number, vagga_number, sutta_number);
    let sutta_ref = generate_sutta_ref(&uid);
    
    // Transform content to HTML
    let content_html = transform_to_html(sutta)?;
    let content_plain = html_to_plain_text(&content_html);
    
    // Build group path
    let group_path = format!("{}/{}/{}", 
        collection.nikaya, 
        book.title, 
        vagga.title
    );
    
    // Apply text processing
    let title = consistent_niggahita(Some(sutta.title.clone()));
    let title_ascii = pali_to_ascii(Some(&title));
    let content_html = consistent_niggahita(Some(content_html));
    
    // Create NewSutta record
    let new_sutta = NewSutta {
        uid: uid.clone(),
        sutta_ref,
        nikaya: nikaya_to_abbrev(&collection.nikaya),
        language: "pli".to_string(),
        group_path: Some(group_path),
        group_index: Some(vagga_number as i32),
        order_index: Some(sutta_number as i32),
        sutta_range_group: None,
        sutta_range_start: None,
        sutta_range_end: None,
        title: Some(title),
        title_pali: Some(sutta.title.clone()),
        title_ascii,
        title_trans: None,
        description: None,
        content_plain: Some(content_plain),
        content_html: Some(content_html),
        content_json: None,
        content_json_tmpl: None,
        source_uid: Some("vri-cst".to_string()),
        source_info: Some("VRI CST Tipitaka romn".to_string()),
        source_language: Some("pli".to_string()),
        message: None,
        copyright: Some("VRI".to_string()),
        license: None,
    };
    
    // Insert into database
    diesel::insert_into(suttas::table)
        .values(&new_sutta)
        .execute(conn)
        .context(format!("Failed to insert sutta: {}", uid))?;
    
    Ok(())
}
```

**Acceptance Criteria:**
- [ ] Inserts all suttas from TipitakaCollection
- [ ] Maps to NewSutta struct correctly
- [ ] Applies text processing (consistent_niggahita, pali_to_ascii)
- [ ] Generates plain text content
- [ ] Sets correct metadata (group_path, group_index, order_index)
- [ ] Handles errors gracefully with context
- [ ] Returns count of inserted suttas

**Dependencies:** Task 2.1, Task 5.1, Task 6.1

---

#### Task 7.2: Write Database Integration Tests
**Priority:** P1 (High)  
**Estimated Effort:** 1.5 hours

Write integration tests for database insertion.

**Files to Create:**
- `backend/tests/test_tipitaka_xml_parser.rs`

**Files to Modify:**
- `backend/tests/data/sample_mn.xml` (if not exists from Task 4.2)

**Tests to Implement:**
- `test_insert_single_sutta()` - Insert one sutta successfully
- `test_insert_multiple_suttas()` - Insert multiple suttas
- `test_verify_metadata()` - Verify group_path, indices correct
- `test_verify_content()` - Verify HTML and plain text correct
- `test_duplicate_uid_handling()` - Handle duplicate UIDs gracefully

**Reference:** PRD lines 620-650 (integration test examples)

**Acceptance Criteria:**
- [ ] Tests use in-memory SQLite database
- [ ] Verify suttas inserted with correct metadata
- [ ] Verify content_html and content_plain populated
- [ ] Foreign key constraints work
- [ ] `cargo test test_tipitaka_xml_parser` passes

**Dependencies:** Task 7.1

---

### Phase 8: CLI Command Implementation

#### Task 8.1: Create TipitakaXmlImporter Struct
**Priority:** P0 (Blocking)  
**Estimated Effort:** 1 hour

Create importer struct implementing SuttaImporter trait.

**Files to Create:**
- `cli/src/bootstrap/tipitaka_xml.rs`

**Files to Modify:**
- `cli/src/bootstrap/mod.rs` - Add `mod tipitaka_xml;` and register importer

**Implementation:**
```rust
use anyhow::{Context, Result};
use diesel::prelude::*;
use std::path::PathBuf;
use std::fs;
use indicatif::{ProgressBar, ProgressStyle};

use simsapa_backend::tipitaka_xml_parser::encoding::read_xml_file;
use simsapa_backend::tipitaka_xml_parser::xml_parser::parse_xml;
use simsapa_backend::tipitaka_xml_parser::database_inserter::insert_collection;

use crate::bootstrap::SuttaImporter;

pub struct TipitakaXmlImporter {
    xml_dir: PathBuf,
}

impl TipitakaXmlImporter {
    pub fn new(xml_dir: PathBuf) -> Self {
        Self { xml_dir }
    }
    
    fn discover_xml_files(&self) -> Result<Vec<PathBuf>> {
        if !self.xml_dir.exists() {
            anyhow::bail!("XML directory not found: {:?}", self.xml_dir);
        }
        
        let mut files = Vec::new();
        
        for entry in fs::read_dir(&self.xml_dir)
            .with_context(|| format!("Failed to read directory: {:?}", self.xml_dir))?
        {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("xml") {
                files.push(path);
            }
        }
        
        files.sort();
        Ok(files)
    }
    
    fn parse_file(&self, path: &PathBuf) -> Result<TipitakaCollection> {
        // Read and convert encoding
        let utf8_content = read_xml_file(path)
            .context(format!("Failed to read XML file: {:?}", path))?;
        
        // Parse XML
        parse_xml(&utf8_content)
            .context("Failed to parse XML structure")
    }
}

impl SuttaImporter for TipitakaXmlImporter {
    fn import(&mut self, conn: &mut SqliteConnection) -> Result<()> {
        let files = self.discover_xml_files()?;
        
        if files.is_empty() {
            tracing::warn!("No XML files found in: {:?}", self.xml_dir);
            return Ok(());
        }
        
        let pb = ProgressBar::new(files.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}")?
                .progress_chars("##-")
        );
        
        let mut total_suttas = 0;
        
        for file_path in files {
            pb.set_message(format!("Processing: {}", file_path.file_name().unwrap().to_string_lossy()));
            
            match self.parse_file(&file_path) {
                Ok(collection) => {
                    match insert_collection(conn, &collection) {
                        Ok(count) => {
                            total_suttas += count;
                            tracing::info!("Inserted {} suttas from {:?}", count, file_path);
                        }
                        Err(e) => {
                            tracing::error!("Failed to insert suttas from {:?}: {}", file_path, e);
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to parse {:?}: {}", file_path, e);
                }
            }
            
            pb.inc(1);
        }
        
        pb.finish_with_message(format!("Imported {} suttas from {} files", total_suttas, files.len()));
        
        Ok(())
    }
}
```

**Acceptance Criteria:**
- [ ] Implements SuttaImporter trait
- [ ] Discovers XML files in directory
- [ ] Processes files with progress bar
- [ ] Handles errors gracefully (logs and continues)
- [ ] Reports summary statistics
- [ ] Follows pattern from NyanadipaImporter

**Dependencies:** Task 3.1, Task 4.1, Task 7.1

**Reference:** `cli/src/bootstrap/nyanadipa.rs:15-120` for importer pattern

---

#### Task 8.2: Add CLI Command ParseTipitakaXml
**Priority:** P0 (Blocking)  
**Estimated Effort:** 1 hour

Add CLI command to parse Tipitaka XML files.

**Files to Modify:**
- `cli/src/main.rs`

**Implementation:**
```rust
#[derive(Subcommand)]
enum Commands {
    // ... existing commands ...
    
    /// Parse VRI CST Tipitaka XML files and import to database
    ParseTipitakaXml {
        /// Path to a single XML file
        #[arg(long)]
        file: Option<PathBuf>,
        
        /// Path to folder containing XML files
        #[arg(long)]
        folder: Option<PathBuf>,
        
        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
        
        /// Dry run (parse without database insertion)
        #[arg(long)]
        dry_run: bool,
    },
}

// In main() match statement:
Commands::ParseTipitakaXml { file, folder, verbose, dry_run } => {
    if verbose {
        logger::init_logger_with_level(tracing::Level::DEBUG);
    } else {
        logger::init_logger();
    }
    
    let xml_path = if let Some(file_path) = file {
        file_path
    } else if let Some(folder_path) = folder {
        folder_path
    } else {
        // Default to configured path
        get_default_tipitaka_xml_path()?
    };
    
    if dry_run {
        // Parse only, no DB insertion
        parse_tipitaka_xml_dry_run(&xml_path)?;
    } else {
        // Full import
        init_app_data();
        let app_data = get_app_data();
        let mut conn = app_data.dbm.get_mut_appdata_db()?;
        
        let mut importer = if xml_path.is_file() {
            bootstrap::tipitaka_xml::TipitakaXmlImporter::new(xml_path.parent().unwrap().to_path_buf())
        } else {
            bootstrap::tipitaka_xml::TipitakaXmlImporter::new(xml_path)
        };
        
        importer.import(&mut conn)?;
    }
}
```

**Acceptance Criteria:**
- [ ] Command accepts --file, --folder, --verbose, --dry-run options
- [ ] Single file processing works
- [ ] Folder batch processing works
- [ ] Dry-run mode parses without DB insertion
- [ ] Verbose mode enables debug logging
- [ ] Default path fallback works
- [ ] Help text is clear and accurate

**Dependencies:** Task 8.1

**Reference:** PRD lines 209-225 (CLI specification)

---

#### Task 8.3: Implement Dry-Run Mode
**Priority:** P2 (Medium)  
**Estimated Effort:** 45 minutes

Implement dry-run mode for testing parsing without DB changes.

**Files to Modify:**
- `cli/src/main.rs` (add helper function)

**Implementation:**
```rust
fn parse_tipitaka_xml_dry_run(xml_path: &Path) -> Result<()> {
    use simsapa_backend::tipitaka_xml_parser::encoding::read_xml_file;
    use simsapa_backend::tipitaka_xml_parser::xml_parser::parse_xml;
    
    if xml_path.is_file() {
        println!("Dry-run: Parsing file: {:?}", xml_path);
        let utf8_content = read_xml_file(xml_path)?;
        let collection = parse_xml(&utf8_content)?;
        
        println!("✓ Nikaya: {}", collection.nikaya);
        println!("✓ Books: {}", collection.books.len());
        
        let mut total_suttas = 0;
        for book in &collection.books {
            for vagga in &book.vaggas {
                total_suttas += vagga.suttas.len();
            }
        }
        
        println!("✓ Suttas: {}", total_suttas);
    } else {
        // Process folder
        println!("Dry-run: Parsing folder: {:?}", xml_path);
        let importer = bootstrap::tipitaka_xml::TipitakaXmlImporter::new(xml_path.to_path_buf());
        let files = importer.discover_xml_files()?;
        
        println!("Found {} XML files", files.len());
        
        for file in files {
            println!("  - {:?}", file.file_name().unwrap());
        }
    }
    
    Ok(())
}
```

**Acceptance Criteria:**
- [ ] Parses files without database connection
- [ ] Reports statistics (nikaya, books, suttas count)
- [ ] Lists files in folder mode
- [ ] No database modifications
- [ ] Helpful for debugging XML parsing issues

**Dependencies:** Task 8.2

---

### Phase 9: Testing and Validation

#### Task 9.1: Create Test Data
**Priority:** P1 (High)  
**Estimated Effort:** 1 hour

Create comprehensive test data files.

**Files to Create:**
- `backend/tests/data/sample_mn_minimal.xml` - Minimal valid XML (1 sutta)
- `backend/tests/data/sample_mn_full.xml` - Full structure (1 book, 1 vagga, 3 suttas)
- `backend/tests/data/sample_mn_utf16le.xml` - UTF-16LE with BOM encoding
- `backend/tests/data/sample_mn_crlf.xml` - Windows CRLF line endings

**XML Structure:**
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
      Test content with <note>variant reading</note> and more text.
    </p>
    
  </div>
</div>

</body>
</text>
</TEI.2>
```

**Acceptance Criteria:**
- [ ] Minimal XML is valid and parseable
- [ ] Full XML covers all structural elements
- [ ] UTF-16LE file has proper BOM
- [ ] CRLF file has Windows line endings
- [ ] All files include Pāli diacritics
- [ ] Files parse successfully

**Dependencies:** None (can be done in parallel)

---

#### Task 9.2: Manual Testing with Real Data
**Priority:** P1 (High)  
**Estimated Effort:** 2 hours

Test with actual VRI CST Tipitaka XML file (s0201m.mul.xml).

**Prerequisites:**
- Access to VRI CST Tipitaka romn/ folder
- s0201m.mul.xml file (Majjhima Nikāya, Mūlapaṇṇāsapāḷi)

**Testing Steps:**
1. Run dry-run mode on single file
2. Verify encoding detection (UTF-16LE with BOM)
3. Verify CRLF conversion
4. Verify diacritics preservation
5. Check parsed structure (nikaya, books, vaggas, suttas)
6. Run full import to test database
7. Verify inserted suttas in database
8. Check HTML rendering in app

**Acceptance Criteria:**
- [ ] UTF-16LE with BOM detected correctly
- [ ] CRLF converted to LF
- [ ] All Pāli diacritics preserved (ā, ī, ū, ṃ, ṅ, ñ, ṭ, ḍ, ṇ, ḷ)
- [ ] Expected number of suttas extracted (verify with manual count)
- [ ] HTML output is valid and renders correctly
- [ ] Database records match expected structure
- [ ] No errors or warnings during import

**Dependencies:** Task 8.2

**Reference:** PRD lines 652-667 (manual testing checklist)

---

#### Task 9.3: Batch Processing Test
**Priority:** P1 (High)  
**Estimated Effort:** 1 hour

Test batch processing of multiple XML files.

**Prerequisites:**
- Multiple XML files from romn/ folder

**Testing Steps:**
1. Run `parse-tipitaka-xml --folder path/to/romn/`
2. Verify all XML files discovered
3. Verify progress bar shows correctly
4. Verify error handling (skip malformed files)
5. Verify summary statistics
6. Check database for all imported suttas

**Acceptance Criteria:**
- [ ] All valid XML files processed
- [ ] Malformed files skipped with warnings
- [ ] Progress bar updates correctly
- [ ] Summary shows files processed, suttas imported
- [ ] No duplicate UIDs in database
- [ ] Processing continues on individual file errors

**Dependencies:** Task 8.2

**Reference:** PRD lines 49-56 (batch processing user story)

---

### Phase 10: Documentation and Cleanup

#### Task 10.1: Add Documentation Comments
**Priority:** P2 (Medium)  
**Estimated Effort:** 1 hour

Add comprehensive rustdoc comments to all public functions.

**Files to Modify:**
- All files in `backend/src/tipitaka_xml_parser/`
- `cli/src/bootstrap/tipitaka_xml.rs`

**Documentation Requirements:**
- Module-level documentation explaining purpose
- Function-level documentation with examples
- Parameter and return value descriptions
- Error conditions documented
- Cross-references to related functions

**Example:**
```rust
/// Reads an XML file, detects encoding (UTF-16LE with BOM), and converts to UTF-8.
///
/// This function handles the VRI CST Tipitaka XML files which use UTF-16LE encoding
/// with BOM and CRLF line endings. It automatically detects the encoding, skips the
/// BOM, converts to UTF-8, and normalizes line endings to Unix LF.
///
/// # Arguments
///
/// * `path` - Path to the XML file to read
///
/// # Returns
///
/// Returns the file content as a UTF-8 string with Unix line endings.
///
/// # Errors
///
/// Returns an error if:
/// - The file cannot be opened or read
/// - The encoding cannot be detected or converted
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
/// use simsapa_backend::tipitaka_xml_parser::encoding::read_xml_file;
///
/// let content = read_xml_file(Path::new("s0201m.mul.xml"))?;
/// assert!(content.contains("Majjhimanikāyo"));
/// ```
pub fn read_xml_file(path: &Path) -> Result<String> {
    // ...
}
```

**Acceptance Criteria:**
- [ ] All public functions documented
- [ ] Module-level documentation added
- [ ] Examples provided where helpful
- [ ] Error conditions documented
- [ ] `cargo doc` builds without warnings

**Dependencies:** All implementation tasks

---

#### Task 10.2: Update PROJECT_MAP.md
**Priority:** P2 (Medium)  
**Estimated Effort:** 30 minutes

Update PROJECT_MAP.md with new module information.

**Files to Modify:**
- `PROJECT_MAP.md`

**Changes:**
Add section under backend modules:
```markdown
### Tipitaka XML Parser

**Location:** `backend/src/tipitaka_xml_parser/`

**Purpose:** Parse VRI CST Tipitaka XML files and import into appdata database

**Key Modules:**
- `encoding.rs` - UTF-16LE to UTF-8 conversion, BOM detection
- `xml_parser.rs` - Parse XML structure (nikaya → book → vagga → sutta)
- `html_transformer.rs` - Transform XML to HTML following tipitaka.org XSL
- `database_inserter.rs` - Insert parsed suttas into appdata.sqlite3
- `uid_generator.rs` - Generate UIDs (vri-cst/mn1.1 format)
- `types.rs` - Data structures (TipitakaCollection, Book, Vagga, Sutta)

**CLI Command:**
- `simsapa-cli parse-tipitaka-xml` - Import Tipitaka XML files
```

**Acceptance Criteria:**
- [ ] New module documented
- [ ] CLI command listed
- [ ] Key functions referenced
- [ ] Markdown formatting correct

**Dependencies:** All implementation tasks

---

#### Task 10.3: Add Usage Examples to README (if applicable)
**Priority:** P3 (Low)  
**Estimated Effort:** 30 minutes

Add usage examples to README or create TIPITAKA_XML_IMPORT.md guide.

**Files to Create (optional):**
- `docs/tipitaka-xml-import.md`

**Content:**
```markdown
# Tipitaka XML Import Guide

## Overview

Import VRI CST Tipitaka XML files into Simsapa appdata database.

## Prerequisites

- VRI CST Tipitaka XML files (romn/ folder)
- Simsapa CLI tool built

## Usage

### Import Single File

```bash
simsapa-cli parse-tipitaka-xml --file path/to/s0201m.mul.xml
```

### Import Entire Folder

```bash
simsapa-cli parse-tipitaka-xml --folder path/to/romn/
```

### Dry Run (Parse Only)

```bash
simsapa-cli parse-tipitaka-xml --file s0201m.mul.xml --dry-run --verbose
```

## Expected Output

```
Parsing Tipitaka XML file: s0201m.mul.xml

Encoding Detection:
  Detected: UTF-16LE with BOM
  Line endings: CRLF (Windows)
  Converting to UTF-8 with LF (Unix)...
  ✓ Encoding conversion successful

[Processing progress bar]

Summary:
  Files processed: 1
  Suttas imported: 10
  Duration: 2.3s

✓ Successfully imported suttas to appdata.sqlite3
```

## Troubleshooting

### Encoding Errors

If you see encoding errors, verify the XML file is UTF-16LE with BOM.

### Missing Files

Ensure the XML files are in the correct directory and have .xml extension.
```

**Acceptance Criteria:**
- [ ] Usage examples clear and accurate
- [ ] Troubleshooting section helpful
- [ ] Example output matches actual output

**Dependencies:** Task 8.2

---

### Phase 11: Future Extensibility (Optional)

#### Task 11.1: Design Nikaya Structure Detection
**Priority:** P3 (Low - Future Enhancement)  
**Estimated Effort:** 2 hours

Design system to detect and handle different nikaya structural patterns.

**Files to Create:**
- `backend/src/tipitaka_xml_parser/structure_detector.rs`

**Implementation:**
```rust
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

/// Detects nikaya structure by analyzing div types and nesting
pub fn detect_nikaya_structure(xml: &str) -> Result<NikayaStructure> {
    // Analyze div type attributes
    // Determine nesting depth
    // Return appropriate structure enum
    todo!()
}
```

**Acceptance Criteria:**
- [ ] Design documented
- [ ] Enum structure defined
- [ ] Detection strategy outlined
- [ ] Not blocking current implementation

**Dependencies:** None (future work)

**Reference:** PRD lines 423-453 (extensibility section)

---

## Summary Statistics

**Total Tasks:** 27 tasks across 11 phases  
**Estimated Total Effort:** ~30-35 hours  
**Critical Path:** Tasks 1.1 → 1.2 → 2.1 → 3.1 → 4.1 → 5.1 → 6.1 → 7.1 → 8.1 → 8.2

**Priority Breakdown:**
- P0 (Blocking): 13 tasks (~20 hours)
- P1 (High): 9 tasks (~10 hours)  
- P2 (Medium): 4 tasks (~3 hours)
- P3 (Low): 1 task (~2 hours)

**Phase Dependencies:**
- Phase 1-2: Independent, can start immediately
- Phase 3-4: Depends on Phase 1-2
- Phase 5-6: Depends on Phase 2
- Phase 7: Depends on Phase 2, 5, 6
- Phase 8: Depends on Phase 3, 4, 7
- Phase 9: Depends on Phase 8
- Phase 10-11: Can be done in parallel with Phase 9

## Testing Strategy Summary

1. **Unit Tests** - Each module has comprehensive unit tests
2. **Integration Tests** - Database insertion tested end-to-end
3. **Manual Tests** - Real XML files tested with actual data
4. **Encoding Tests** - UTF-16LE, BOM, CRLF handling verified
5. **Diacritics Tests** - Pāli characters preserved throughout

## Success Metrics

- [ ] Successfully parse s0201m.mul.xml (Majjhima Nikāya book 1)
- [ ] All Pāli diacritics preserved (ā, ī, ū, ṃ, ṅ, ñ, ṭ, ḍ, ṇ, ḷ)
- [ ] UTF-16LE with BOM encoding detected and converted
- [ ] CRLF line endings normalized to LF
- [ ] HTML output renders correctly in app
- [ ] CLI command works for both single file and folder
- [ ] Dry-run mode works for testing
- [ ] All unit tests pass
- [ ] All integration tests pass
- [ ] Code follows existing patterns (NyanadipaImporter style)
- [ ] Documentation complete and accurate

## Notes for Implementation

- Follow the coding style guide in AGENTS.md (snake_case, etc.)
- Use `anyhow::Result` for error handling throughout
- Add tracing/logging at appropriate levels (info, debug, warn, error)
- Use progress bars with `indicatif` for user feedback
- Reference NyanadipaImporter (cli/src/bootstrap/nyanadipa.rs) for patterns
- Keep modules focused and testable
- Write tests as you implement (TDD approach encouraged)
