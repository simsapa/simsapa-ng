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

    // Generate HTML content that embeds the PDF
    let content_html = generate_pdf_embed_html(book_uid);

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
    let spine_item_uid = format!("{}.0", book_uid);
    let new_spine_item = NewBookSpineItem {
        book_id,
        book_uid,
        spine_item_uid: &spine_item_uid,
        spine_index: 0,
        title: Some(&title),
        language: if language.is_empty() { None } else { Some(&language) },
        content_html: Some(&content_html),
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
        return Some(String::from_utf8_lossy(bytes).to_string());
    }
    
    // Try to extract as name
    if let Ok(name_bytes) = value.as_name() {
        return Some(String::from_utf8_lossy(name_bytes).to_string());
    }
    
    None
}

/// Generate HTML that embeds a PDF using embedpdf.js
fn generate_pdf_embed_html(book_uid: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>PDF Viewer</title>
</head>
<body>
    <div id="pdf-viewer" style="height: 100vh"></div>
    
    <script async type="module">
        import EmbedPDF from 'https://snippet.embedpdf.com/embedpdf.js';
        
        EmbedPDF.init({{
            type: 'container',
            target: document.getElementById('pdf-viewer'),
            src: '/book_resources/{}/document.pdf'
        }});
    </script>
</body>
</html>"#,
        book_uid
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_pdf_embed_html() {
        let html = generate_pdf_embed_html("testpdf");
        assert!(html.contains(r#"src: '/book_resources/testpdf/document.pdf'"#));
        assert!(html.contains(r#"import EmbedPDF from 'https://snippet.embedpdf.com/embedpdf.js'"#));
        assert!(html.contains(r#"EmbedPDF.init"#));
        assert!(html.contains(r#"id="pdf-viewer""#));
        assert!(html.contains("<!DOCTYPE html>"));
    }
}
