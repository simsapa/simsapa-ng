use anyhow::Result;
use epub::doc::EpubDoc;
use html5ever::parse_document;
use html5ever::tendril::TendrilSink;
use lopdf::Document;
use markup5ever_rcdom::{Handle, NodeData, RcDom};
use std::fs;
use std::path::Path;

use crate::pdf_import::{extract_pdf_metadata, extract_xmp_author};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DocumentMetadata {
    pub title: String,
    pub author: String,
}

impl Default for DocumentMetadata {
    fn default() -> Self {
        Self {
            title: String::new(),
            author: String::new(),
        }
    }
}

/// Extract metadata (title and author) from a document file
pub fn extract_document_metadata(file_path: &Path) -> Result<DocumentMetadata> {
    let mut metadata = DocumentMetadata::default();

    let path_str = file_path.to_string_lossy().to_string();
    let lower_path = path_str.to_lowercase();

    if lower_path.ends_with(".epub") {
        add_epub_metadata_from_file(file_path, &mut metadata)?;
    } else if lower_path.ends_with(".html") || lower_path.ends_with(".htm") {
        add_html_metadata_from_file(file_path, &mut metadata)?;
    } else if lower_path.ends_with(".pdf") {
        add_pdf_metadata_from_file(file_path, &mut metadata)?;
    }

    Ok(metadata)
}

/// Extract metadata from EPUB file
fn add_epub_metadata_from_file(file_path: &Path, metadata: &mut DocumentMetadata) -> Result<()> {
    let doc = EpubDoc::new(file_path)?;

    metadata.title = doc
        .mdata("title")
        .or_else(|| doc.mdata("dc:title"))
        .map(|item| item.value.clone())
        .unwrap_or_default();

    metadata.author = doc
        .mdata("creator")
        .or_else(|| doc.mdata("dc:creator"))
        .map(|item| item.value.clone())
        .unwrap_or_default();

    Ok(())
}

/// Extract metadata from HTML file
fn add_html_metadata_from_file(file_path: &Path, metadata: &mut DocumentMetadata) -> Result<()> {
    let html_content = fs::read_to_string(file_path)?;

    let dom = parse_document(RcDom::default(), Default::default())
        .from_utf8()
        .read_from(&mut html_content.as_bytes())?;

    walk_html_for_metadata(&dom.document, metadata);

    Ok(())
}

/// Extract metadata from PDF file
fn add_pdf_metadata_from_file(file_path: &Path, metadata: &mut DocumentMetadata) -> Result<()> {
    let doc = Document::load(file_path)?;

    metadata.title = extract_pdf_metadata(&doc, b"Title")
        .unwrap_or_default();

    // Try to extract author from multiple sources in order of preference:
    // 1. PDF Info dictionary Author field
    // 2. XMP metadata dc:creator (Dublin Core)
    // 3. XMP metadata pdf:Author (PDF-specific)
    // 4. PDF Info dictionary Creator field (application that created it)
    metadata.author = extract_pdf_metadata(&doc, b"Author")
        .or_else(|| extract_xmp_author(&doc))
        .or_else(|| extract_pdf_metadata(&doc, b"Creator"))
        .unwrap_or_default();

    Ok(())
}

/// Walk the HTML DOM to find metadata
fn walk_html_for_metadata(handle: &Handle, metadata: &mut DocumentMetadata) {
    match handle.data {
        NodeData::Element {
            ref name,
            ref attrs,
            ..
        } => {
            let tag_name = name.local.as_ref();

            // Extract title from <title> tag
            if tag_name == "title" && metadata.title.is_empty() {
                if let Some(text) = get_element_text(handle) {
                    metadata.title = text;
                }
            }

            // Extract meta tags
            if tag_name == "meta" {
                let attrs = attrs.borrow();
                let name_attr = attrs
                    .iter()
                    .find(|a| a.name.local.as_ref() == "name")
                    .map(|a| a.value.to_string());
                let property_attr = attrs
                    .iter()
                    .find(|a| a.name.local.as_ref() == "property")
                    .map(|a| a.value.to_string());
                let content_attr = attrs
                    .iter()
                    .find(|a| a.name.local.as_ref() == "content")
                    .map(|a| a.value.to_string());

                if let Some(content) = content_attr {
                    if let Some(name) = name_attr.or(property_attr) {
                        let key = name.to_lowercase();
                        if (key.contains("author") || key == "dc:creator") && metadata.author.is_empty() {
                            metadata.author = content;
                        }
                    }
                }
            }
        }
        _ => {}
    }

    // Recursively walk children
    for child in handle.children.borrow().iter() {
        walk_html_for_metadata(child, metadata);
    }
}

/// Get the text content of an HTML element
fn get_element_text(handle: &Handle) -> Option<String> {
    let mut text = String::new();

    fn collect_text(handle: &Handle, text: &mut String) {
        match handle.data {
            NodeData::Text { ref contents } => {
                text.push_str(&contents.borrow());
            }
            _ => {}
        }

        for child in handle.children.borrow().iter() {
            collect_text(child, text);
        }
    }

    collect_text(handle, &mut text);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_html_metadata_from_string() {
        let html = r#"
            <!DOCTYPE html>
            <html>
            <head>
                <title>Test Book Title</title>
                <meta name="author" content="Test Author" />
            </head>
            <body>
                <p>Content</p>
            </body>
            </html>
        "#;

        // Create a temporary file
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_metadata.html");
        fs::write(&test_file, html).unwrap();

        let metadata = extract_document_metadata(&test_file).unwrap();

        assert_eq!(metadata.title, "Test Book Title");
        assert_eq!(metadata.author, "Test Author");

        // Cleanup
        fs::remove_file(test_file).ok();
    }
}
