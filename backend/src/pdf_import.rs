use anyhow::{anyhow, Result};
use diesel::prelude::*;
use lopdf::Document;
use std::fs;
use std::path::Path;

use crate::db::appdata_models::{NewBook, NewBookResource, NewBookSpineItem};
use crate::db::appdata_schema::{book_resources, book_spine_items, books};
use crate::helpers::compact_rich_text;

/// Import a PDF file into the database
///
/// # Arguments
/// * `db_conn` - Mutable reference to SQLite database connection
/// * `pdf_path` - Path to the PDF file to import
/// * `book_uid` - Unique identifier for this book
///
/// # Returns
/// * `Result<()>` - Ok if successful, Err with details if failed
pub fn import_pdf_to_db(
    db_conn: &mut SqliteConnection,
    pdf_path: &Path,
    book_uid: &str,
) -> Result<()> {
    tracing::info!("Importing PDF from {:?} with UID: {}", pdf_path, book_uid);

    // Extract metadata using lopdf
    let doc = Document::load(pdf_path)
        .map_err(|e| anyhow!("Failed to load PDF: {}", e))?;

    // Extract metadata
    let title = extract_pdf_metadata(&doc, b"Title")
        .unwrap_or_else(|| "Untitled".to_string());

    let author = extract_pdf_metadata(&doc, b"Author")
        .unwrap_or_else(|| String::new());

    let language = extract_pdf_metadata(&doc, b"Language")
        .or_else(|| extract_pdf_metadata(&doc, b"Lang"))
        .unwrap_or_else(|| String::new());

    tracing::info!("PDF metadata - Title: {}, Author: {}, Language: {}", title, author, language);

    // Extract plain text from PDF using pdf-extract
    let content_plain = match pdf_extract::extract_text(pdf_path) {
        Ok(text) => {
            tracing::debug!("Extracted {} characters of plain text", text.len());
            compact_rich_text(&text)
        }
        Err(e) => {
            tracing::warn!("Failed to extract text from PDF: {}. Using empty text.", e);
            String::new()
        }
    };

    // Read PDF file into memory as binary data
    let pdf_bytes = fs::read(pdf_path)
        .map_err(|e| anyhow!("Failed to read PDF file: {}", e))?;

    tracing::info!("Read PDF file: {} bytes", pdf_bytes.len());

    // Insert book record
    let file_path_str = pdf_path.to_string_lossy().to_string();
    let new_book = NewBook {
        uid: book_uid,
        document_type: "pdf",
        title: Some(&title),
        author: if author.is_empty() { None } else { Some(&author) },
        language: if language.is_empty() { None } else { Some(&language) },
        file_path: Some(&file_path_str),
        metadata_json: None, // PDFs don't have structured metadata like EPUBs
    };

    diesel::insert_into(books::table)
        .values(&new_book)
        .execute(db_conn)?;

    let book_id: i32 = books::table
        .filter(books::uid.eq(book_uid))
        .select(books::id)
        .first(db_conn)?;

    tracing::info!("Inserted book record with id: {}", book_id);

    // Insert single spine item (PDFs are treated as single documents)
    // content_html is None for PDFs - the API will serve the PDF viewer template
    let spine_item_uid = format!("{}.0", book_uid);
    let new_spine_item = NewBookSpineItem {
        book_id,
        book_uid,
        spine_item_uid: &spine_item_uid,
        spine_index: 0,
        title: Some(&title),
        language: if language.is_empty() { None } else { Some(&language) },
        content_html: None,
        content_plain: Some(&content_plain),
    };

    diesel::insert_into(book_spine_items::table)
        .values(&new_spine_item)
        .execute(db_conn)?;

    tracing::info!("Inserted spine item: {}", spine_item_uid);

    // Insert PDF as a resource
    let new_resource = NewBookResource {
        book_id,
        book_uid,
        resource_path: "document.pdf",
        mime_type: Some("application/pdf"),
        content_data: Some(&pdf_bytes),
    };

    diesel::insert_into(book_resources::table)
        .values(&new_resource)
        .execute(db_conn)?;

    tracing::info!("Successfully imported PDF: 1 spine item, 1 resource ({} bytes)", pdf_bytes.len());
    Ok(())
}

/// Extract metadata field from PDF document
fn extract_pdf_metadata(doc: &Document, key: &[u8]) -> Option<String> {
    // Get the Info dictionary reference - convert Result to Option
    let info_obj_ref = doc.trailer.get(b"Info").ok()?;
    let info_ref = info_obj_ref.as_reference().ok()?;

    // Get the Info object
    let info_obj = doc.get_object(info_ref).ok()?;

    // Convert to dictionary
    let info_dict = info_obj.as_dict().ok()?;

    // Get the value for the key - convert Result to Option
    let value = info_dict.get(key).ok()?;

    // Try to extract as string (most common case)
    // as_str() returns &[u8], so we need to convert it to String
    if let Ok(bytes) = value.as_str() {
        let decoded = decode_pdf_text_string(bytes);
        return Some(trim_pdf_string(&decoded));
    }

    // Try to extract as name
    if let Ok(name_bytes) = value.as_name() {
        let decoded = decode_pdf_text_string(name_bytes);
        return Some(trim_pdf_string(&decoded));
    }

    None
}

/// Decode PDF text string which may be UTF-16 BE (with BOM) or PDFDocEncoding/Latin1
fn decode_pdf_text_string(bytes: &[u8]) -> String {
    // Check for UTF-16 BE BOM (0xFE 0xFF)
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        // UTF-16 BE with BOM - decode starting after the BOM
        let utf16_bytes = &bytes[2..];

        // Convert bytes to u16 values (big-endian)
        let mut utf16_chars: Vec<u16> = Vec::new();
        for chunk in utf16_bytes.chunks(2) {
            if chunk.len() == 2 {
                utf16_chars.push(u16::from_be_bytes([chunk[0], chunk[1]]));
            }
        }

        String::from_utf16_lossy(&utf16_chars)
    }
    // Check for UTF-16 LE BOM (0xFF 0xFE)
    else if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
        // UTF-16 LE with BOM - decode starting after the BOM
        let utf16_bytes = &bytes[2..];

        // Convert bytes to u16 values (little-endian)
        let mut utf16_chars: Vec<u16> = Vec::new();
        for chunk in utf16_bytes.chunks(2) {
            if chunk.len() == 2 {
                utf16_chars.push(u16::from_le_bytes([chunk[0], chunk[1]]));
            }
        }

        String::from_utf16_lossy(&utf16_chars)
    }
    // No BOM - assume PDFDocEncoding or Latin1 (subset of UTF-8)
    else {
        String::from_utf8_lossy(bytes).to_string()
    }
}

/// Trim whitespace and common control characters from PDF strings
fn trim_pdf_string(s: &str) -> String {
    s.trim()
        .trim_matches('\u{0000}') // NULL
        .trim_matches('\u{FEFF}') // Zero-width no-break space (BOM)
        .trim()
        .to_string()
}
