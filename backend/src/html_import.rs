use anyhow::{anyhow, Result};
use diesel::prelude::*;
use html5ever::parse_document;
use html5ever::tendril::TendrilSink;
use markup5ever_rcdom::{Handle, NodeData, RcDom};
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::db::appdata_models::{NewBook, NewBookResource, NewBookSpineItem};
use crate::db::appdata_schema::{book_resources, book_spine_items, books};
use crate::helpers::{compact_rich_text, strip_html};

/// Chapter extracted from HTML document
#[derive(Debug, Clone)]
struct Chapter {
    title: String,
    content_html: String,
    #[allow(dead_code)]
    level: usize, // Heading level (1-6) - reserved for future use
}

/// Import an HTML file into the database
///
/// # Arguments
/// * `db_conn` - Mutable reference to SQLite database connection
/// * `html_path` - Path to the HTML file to import
/// * `book_uid` - Unique identifier for this book
///
/// # Returns
/// * `Result<()>` - Ok if successful, Err with details if failed
pub fn import_html_to_db(
    db_conn: &mut SqliteConnection,
    html_path: &Path,
    book_uid: &str,
    custom_title: Option<&str>,
    custom_author: Option<&str>,
) -> Result<()> {
    tracing::info!("Importing HTML from {:?} with UID: {}", html_path, book_uid);

    // Read HTML file
    let html_content = fs::read_to_string(html_path)
        .map_err(|e| anyhow!("Failed to read HTML file: {}", e))?;

    // Parse HTML document
    let dom = parse_document(RcDom::default(), Default::default())
        .from_utf8()
        .read_from(&mut html_content.as_bytes())
        .map_err(|e| anyhow!("Failed to parse HTML: {}", e))?;

    // Extract metadata from file
    let metadata = extract_metadata(&dom);
    let extracted_title = metadata
        .get("title")
        .cloned()
        .unwrap_or_else(|| "Untitled".to_string());
    let extracted_author = metadata.get("author").cloned().unwrap_or_default();

    // Use custom values if provided and non-empty, otherwise use extracted metadata
    let title = custom_title
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string())
        .unwrap_or(extracted_title);

    let author = custom_author
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string())
        .unwrap_or(extracted_author);
    let language = metadata.get("language").cloned().unwrap_or_default();

    tracing::info!(
        "HTML metadata - Title: {}, Author: {}, Language: {}",
        title,
        author,
        language
    );

    // Split into chapters by headings
    let chapters = split_by_headings(&dom);
    tracing::info!("Found {} chapters", chapters.len());

    // Extract embedded resources (images, CSS, etc.)
    let resources = extract_resources(&html_content, html_path)?;
    tracing::info!("Found {} embedded resources", resources.len());

    // Serialize metadata to JSON
    let metadata_json = serde_json::to_string(&metadata)?;

    // Insert book record
    let file_path_str = html_path.to_string_lossy().to_string();
    let new_book = NewBook {
        uid: book_uid,
        document_type: "html",
        title: Some(&title),
        author: if author.is_empty() {
            None
        } else {
            Some(&author)
        },
        language: if language.is_empty() {
            None
        } else {
            Some(&language)
        },
        file_path: Some(&file_path_str),
        metadata_json: Some(&metadata_json),
        enable_embedded_css: true, // Default to enabled for HTML
        toc_json: None, // HTML files don't have TOC support yet
    };

    diesel::insert_into(books::table)
        .values(&new_book)
        .execute(db_conn)?;

    let book_id: i32 = books::table
        .filter(books::uid.eq(book_uid))
        .select(books::id)
        .first(db_conn)?;

    tracing::info!("Inserted book record with id: {}", book_id);

    // Get the HTML filename for all chapters
    let html_filename = html_path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("index.html");

    // Insert chapters as spine items
    for (spine_index, chapter) in chapters.iter().enumerate() {
        let spine_item_uid = format!("{}.{}", book_uid, spine_index);
        
        // Rewrite resource links to use API endpoint
        let content_html = rewrite_resource_links(&chapter.content_html, book_uid);
        
        // Convert HTML to plain text for FTS5 indexing
        let content_plain = html_to_plain_text(&content_html);

        let new_spine_item = NewBookSpineItem {
            book_id,
            book_uid,
            spine_item_uid: &spine_item_uid,
            spine_index: spine_index as i32,
            resource_path: html_filename,
            title: Some(&chapter.title),
            language: if language.is_empty() {
                None
            } else {
                Some(&language)
            },
            content_html: Some(&content_html),
            content_plain: Some(&content_plain),
        };

        diesel::insert_into(book_spine_items::table)
            .values(&new_spine_item)
            .execute(db_conn)?;

        tracing::debug!("Inserted spine item: {}", spine_item_uid);
    }

    // Insert resources
    let mut resource_count = 0;
    for (resource_path, resource_data) in resources {
        let mime_type = guess_mime_type(&resource_path);

        let new_resource = NewBookResource {
            book_id,
            book_uid,
            resource_path: &resource_path,
            mime_type: Some(&mime_type),
            content_data: Some(&resource_data),
        };

        diesel::insert_into(book_resources::table)
            .values(&new_resource)
            .execute(db_conn)?;

        resource_count += 1;
        tracing::debug!("Inserted resource: {} ({})", resource_path, mime_type);
    }

    tracing::info!(
        "Successfully imported HTML: {} spine items, {} resources",
        chapters.len(),
        resource_count
    );
    Ok(())
}

/// Extract metadata from HTML document
///
/// Looks for meta tags and title element
fn extract_metadata(dom: &RcDom) -> HashMap<String, String> {
    let mut metadata = HashMap::new();

    // Walk the DOM to find head elements
    fn walk_head(handle: &Handle, metadata: &mut HashMap<String, String>) {
        match handle.data {
            NodeData::Element {
                ref name,
                ref attrs,
                ..
            } => {
                let tag_name = name.local.as_ref();

                // Extract title
                if tag_name == "title" {
                    if let Some(text) = get_element_text(handle) {
                        metadata.insert("title".to_string(), text);
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
                            if key.contains("author") || key == "dc:creator" {
                                metadata.insert("author".to_string(), content);
                            } else if key.contains("language") || key == "dc:language" {
                                metadata.insert("language".to_string(), content);
                            } else {
                                metadata.insert(key, content);
                            }
                        }
                    }

                    // Handle lang attribute on html tag
                    if tag_name == "html" {
                        if let Some(lang_attr) = attrs.iter().find(|a| a.name.local.as_ref() == "lang") {
                            metadata.insert("language".to_string(), lang_attr.value.to_string());
                        }
                    }
                }
            }
            _ => {}
        }

        // Recursively walk children
        for child in handle.children.borrow().iter() {
            walk_head(child, metadata);
        }
    }

    walk_head(&dom.document, &mut metadata);
    metadata
}

/// Get the text content of an element
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

/// Split HTML document into chapters based on heading tags (h1-h6)
fn split_by_headings(dom: &RcDom) -> Vec<Chapter> {
    let mut chapters = Vec::new();
    let mut current_content = String::new();
    let mut current_title = String::from("Introduction");
    let mut current_level = 0;
    let mut found_first_heading = false;

    fn walk_body(
        handle: &Handle,
        chapters: &mut Vec<Chapter>,
        current_content: &mut String,
        current_title: &mut String,
        current_level: &mut usize,
        found_first_heading: &mut bool,
    ) {
        match handle.data {
            NodeData::Element {
                ref name,
                ref attrs,
                ..
            } => {
                let tag_name = name.local.as_ref();

                // Check if this is a heading (h1-h6)
                if let Some(level) = heading_level(tag_name) {
                    // Save previous chapter if we have content
                    if *found_first_heading && !current_content.trim().is_empty() {
                        chapters.push(Chapter {
                            title: current_title.clone(),
                            content_html: current_content.clone(),
                            level: *current_level,
                        });
                        current_content.clear();
                    }

                    // Start new chapter
                    if let Some(text) = get_element_text(handle) {
                        *current_title = text;
                        *current_level = level;
                        *found_first_heading = true;
                    }
                    return; // Don't include heading in content
                }

                // Skip script and style tags
                if tag_name == "script" || tag_name == "style" {
                    return;
                }

                // Skip head section
                if tag_name == "head" {
                    return;
                }

                // Serialize element to HTML
                let attrs_str = serialize_attrs(attrs);
                current_content.push_str(&format!("<{}{}>", tag_name, attrs_str));

                // Recursively process children
                for child in handle.children.borrow().iter() {
                    walk_body(
                        child,
                        chapters,
                        current_content,
                        current_title,
                        current_level,
                        found_first_heading,
                    );
                }

                current_content.push_str(&format!("</{}>", tag_name));
            }
            NodeData::Text { ref contents } => {
                let text_str = contents.borrow().to_string();
                let text = html_escape::encode_text(&text_str);
                current_content.push_str(&text);
            }
            NodeData::Document => {
                // Process all children of document
                for child in handle.children.borrow().iter() {
                    walk_body(
                        child,
                        chapters,
                        current_content,
                        current_title,
                        current_level,
                        found_first_heading,
                    );
                }
            }
            _ => {}
        }
    }

    walk_body(
        &dom.document,
        &mut chapters,
        &mut current_content,
        &mut current_title,
        &mut current_level,
        &mut found_first_heading,
    );

    // Add the last chapter if there's content
    if !current_content.trim().is_empty() {
        chapters.push(Chapter {
            title: current_title,
            content_html: current_content,
            level: current_level,
        });
    }

    // If no chapters were found, create a single chapter with the entire body
    if chapters.is_empty() {
        let body_html = extract_body_html(dom);
        chapters.push(Chapter {
            title: "Document".to_string(),
            content_html: body_html,
            level: 0,
        });
    }

    chapters
}

/// Extract the body HTML from the document
fn extract_body_html(dom: &RcDom) -> String {
    let mut html = String::new();

    fn find_body(handle: &Handle, html: &mut String) -> bool {
        match handle.data {
            NodeData::Element { ref name, .. } => {
                if name.local.as_ref() == "body" {
                    serialize_node(handle, html);
                    return true;
                }
            }
            _ => {}
        }

        for child in handle.children.borrow().iter() {
            if find_body(child, html) {
                return true;
            }
        }
        false
    }

    if !find_body(&dom.document, &mut html) {
        // No body found, serialize entire document
        serialize_node(&dom.document, &mut html);
    }

    html
}

/// Serialize a node to HTML string
fn serialize_node(handle: &Handle, output: &mut String) {
    match handle.data {
        NodeData::Element {
            ref name,
            ref attrs,
            ..
        } => {
            let tag_name = name.local.as_ref();
            
            // Skip script and style tags
            if tag_name == "script" || tag_name == "style" || tag_name == "head" {
                return;
            }

            let attrs_str = serialize_attrs(attrs);
            output.push_str(&format!("<{}{}>", tag_name, attrs_str));

            for child in handle.children.borrow().iter() {
                serialize_node(child, output);
            }

            output.push_str(&format!("</{}>", tag_name));
        }
        NodeData::Text { ref contents } => {
            let text_str = contents.borrow().to_string();
            let text = html_escape::encode_text(&text_str);
            output.push_str(&text);
        }
        NodeData::Document => {
            for child in handle.children.borrow().iter() {
                serialize_node(child, output);
            }
        }
        _ => {}
    }
}

/// Serialize element attributes to string
fn serialize_attrs(attrs: &std::cell::RefCell<Vec<markup5ever::Attribute>>) -> String {
    let attrs = attrs.borrow();
    if attrs.is_empty() {
        return String::new();
    }

    let mut result = String::new();
    for attr in attrs.iter() {
        let name = attr.name.local.as_ref();
        let value = html_escape::encode_quoted_attribute(&attr.value);
        result.push_str(&format!(r#" {}="{}""#, name, value));
    }
    result
}

/// Get heading level (1-6) from tag name, or None
fn heading_level(tag: &str) -> Option<usize> {
    match tag {
        "h1" => Some(1),
        "h2" => Some(2),
        "h3" => Some(3),
        "h4" => Some(4),
        "h5" => Some(5),
        "h6" => Some(6),
        _ => None,
    }
}

/// Extract embedded resources from HTML
///
/// Looks for data URIs and external references
fn extract_resources(
    html_content: &str,
    html_path: &Path,
) -> Result<HashMap<String, Vec<u8>>> {
    let mut resources = HashMap::new();

    // Extract data URIs (e.g., data:image/png;base64,...)
    lazy_static::lazy_static! {
        static ref RE_DATA_URI: Regex = 
            Regex::new(r#"(?i)(src|href)=["']data:([^;]+);base64,([^"']+)["']"#).unwrap();
    }

    let mut img_count = 0;
    for caps in RE_DATA_URI.captures_iter(html_content) {
        let mime_type = &caps[2];
        let base64_data = &caps[3];

        // Decode base64
        if let Ok(decoded) = base64_decode(base64_data) {
            let extension = mime_to_extension(mime_type);
            let resource_path = format!("embedded_{}.{}", img_count, extension);
            resources.insert(resource_path, decoded);
            img_count += 1;
        }
    }

    // Look for external file references relative to HTML file
    if let Some(parent_dir) = html_path.parent() {
        lazy_static::lazy_static! {
            static ref RE_SRC: Regex = Regex::new(r#"(?i)(src|href)=["']([^"':]+)["']"#).unwrap();
        }

        for caps in RE_SRC.captures_iter(html_content) {
            let path = &caps[2];

            // Skip absolute URLs and data URIs
            if path.starts_with("http://")
                || path.starts_with("https://")
                || path.starts_with("//")
                || path.starts_with("data:")
                || path.starts_with('#')
            {
                continue;
            }

            // Try to read the file
            let resource_path = parent_dir.join(path);
            if resource_path.exists() && resource_path.is_file() {
                if let Ok(data) = fs::read(&resource_path) {
                    resources.insert(path.to_string(), data);
                    tracing::debug!("Extracted external resource: {}", path);
                }
            }
        }
    }

    Ok(resources)
}

/// Decode base64 string
fn base64_decode(input: &str) -> Result<Vec<u8>> {
    use base64::Engine;
    let engine = base64::engine::general_purpose::STANDARD;
    engine
        .decode(input.trim())
        .map_err(|e| anyhow!("Failed to decode base64: {}", e))
}

/// Convert MIME type to file extension
fn mime_to_extension(mime: &str) -> &str {
    match mime {
        "image/png" => "png",
        "image/jpeg" | "image/jpg" => "jpg",
        "image/gif" => "gif",
        "image/svg+xml" => "svg",
        "image/webp" => "webp",
        "text/css" => "css",
        "application/javascript" | "text/javascript" => "js",
        _ => "dat",
    }
}

/// Guess MIME type from file extension
fn guess_mime_type(path: &str) -> String {
    let extension = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    match extension.to_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "css" => "text/css",
        "js" => "application/javascript",
        "woff" | "woff2" => "font/woff",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        _ => "application/octet-stream",
    }
    .to_string()
}

/// Rewrite resource links in HTML to use the API endpoint format
fn rewrite_resource_links(html: &str, book_uid: &str) -> String {
    lazy_static::lazy_static! {
        static ref RE_SRC: Regex = Regex::new(r#"(?i)(src|href)=["']([^"']+)["']"#).unwrap();
    }

    RE_SRC
        .replace_all(html, |caps: &regex::Captures| {
            let attr = &caps[1];
            let path = &caps[2];

            // Skip absolute URLs, data URIs, anchors, and ssp:// links
            if path.starts_with("http://")
                || path.starts_with("https://")
                || path.starts_with("//")
                || path.starts_with("data:")
                || path.starts_with('/')
                || path.starts_with('#')
                || path.starts_with("ssp://")
            {
                return caps[0].to_string();
            }

            // Rewrite to API endpoint
            format!(r#"{}="/book_resources/{}/{}""#, attr, book_uid, path)
        })
        .to_string()
}

/// Convert HTML content to plain text for FTS5 indexing
fn html_to_plain_text(html: &str) -> String {
    let stripped = strip_html(html);
    compact_rich_text(&stripped)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heading_level() {
        assert_eq!(heading_level("h1"), Some(1));
        assert_eq!(heading_level("h2"), Some(2));
        assert_eq!(heading_level("h6"), Some(6));
        assert_eq!(heading_level("div"), None);
    }

    #[test]
    fn test_mime_to_extension() {
        assert_eq!(mime_to_extension("image/png"), "png");
        assert_eq!(mime_to_extension("image/jpeg"), "jpg");
        assert_eq!(mime_to_extension("text/css"), "css");
    }

    #[test]
    fn test_guess_mime_type() {
        assert_eq!(guess_mime_type("image.png"), "image/png");
        assert_eq!(guess_mime_type("style.css"), "text/css");
        assert_eq!(guess_mime_type("unknown.xyz"), "application/octet-stream");
    }

    #[test]
    fn test_rewrite_resource_links() {
        let html = r#"<img src="images/photo.jpg"><link href="style.css">"#;
        let result = rewrite_resource_links(html, "testbook");
        assert!(result.contains(r#"src="/book_resources/testbook/images/photo.jpg""#));
        assert!(result.contains(r#"href="/book_resources/testbook/style.css""#));
    }

    #[test]
    fn test_rewrite_resource_links_absolute() {
        let html = r###"<a href="http://example.com">Link</a><a href="#anchor">Anchor</a>"###;
        let result = rewrite_resource_links(html, "testbook");
        assert!(result.contains(r###"href="http://example.com""###));
        assert!(result.contains(r###"href="#anchor""###));
    }

    #[test]
    fn test_rewrite_resource_links_skip_ssp_protocol() {
        let html = r#"<a href="ssp://suttas/an5.129/en/thanissaro">AN 5:129</a>"#;
        let result = rewrite_resource_links(html, "testbook");
        assert_eq!(result, html); // Should remain unchanged
    }
}
