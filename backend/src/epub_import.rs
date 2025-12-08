use anyhow::{anyhow, Result};
use diesel::prelude::*;
use epub::doc::EpubDoc;
use regex::Regex;
use std::collections::HashMap;
use std::path::Path;

use crate::db::appdata_models::{NewBook, NewBookResource, NewBookSpineItem};
use crate::db::appdata_schema::{book_resources, book_spine_items, books};
use crate::helpers::{compact_rich_text, strip_html};

/// Extract title from HTML content
/// Returns the title from <title> tag, taking only the part before the first '|' separator
/// Returns None if no title found or if title is empty/whitespace only
fn extract_html_title(content_bytes: &[u8]) -> Option<String> {
    lazy_static::lazy_static! {
        static ref TITLE_RE: Regex = Regex::new(r"(?i)<title[^>]*>([^<]+)</title>").unwrap();
    }

    let content = String::from_utf8_lossy(content_bytes);

    if let Some(caps) = TITLE_RE.captures(&content) {
        if let Some(title_match) = caps.get(1) {
            let title = title_match.as_str().trim();

            // Extract the part before '|' separator if present
            let title_part = title.split('|').next().unwrap_or(title).trim();

            // Only skip if truly empty or already says "Untitled"
            if title_part.is_empty() || title_part.eq_ignore_ascii_case("untitled") {
                return None;
            }

            return Some(title_part.to_string());
        }
    }

    None
}

/// Import an EPUB file into the database
///
/// # Arguments
/// * `db_conn` - Mutable reference to SQLite database connection
/// * `epub_path` - Path to the EPUB file to import
/// * `book_uid` - Unique identifier for this book (e.g., "ess")
///
/// # Returns
/// * `Result<()>` - Ok if successful, Err with details if failed
pub fn import_epub_to_db(
    db_conn: &mut SqliteConnection,
    epub_path: &Path,
    book_uid: &str,
) -> Result<()> {
    tracing::info!("Importing EPUB from {:?} with UID: {}", epub_path, book_uid);

    // Open the EPUB file
    let mut doc = EpubDoc::new(epub_path)
        .map_err(|e| anyhow!("Failed to open EPUB file: {}", e))?;

    // Extract metadata
    let title = doc
        .mdata("title")
        .or_else(|| doc.mdata("dc:title"))
        .map(|item| item.value.clone())
        .unwrap_or_else(|| "Untitled".to_string());

    let author = doc
        .mdata("creator")
        .or_else(|| doc.mdata("dc:creator"))
        .map(|item| item.value.clone())
        .unwrap_or_else(|| String::new());

    let language = doc
        .mdata("language")
        .or_else(|| doc.mdata("dc:language"))
        .map(|item| item.value.clone())
        .unwrap_or_else(|| String::new());

    // Serialize metadata to JSON
    let mut metadata_items = Vec::new();
    for item in &doc.metadata {
        metadata_items.push(serde_json::json!({
            "key": &item.property,
            "value": &item.value
        }));
    }
    let metadata_json = serde_json::to_string(&metadata_items)?;

    tracing::info!("EPUB metadata - Title: {}, Author: {}, Language: {}", title, author, language);

    // Extract TOC (table of contents) and create a mapping from resource paths to chapter titles
    let mut toc_map: HashMap<String, String> = HashMap::new();
    for nav_point in doc.toc.iter() {
        if let Some(content_path_str) = nav_point.content.to_str() {
            // Remove fragment identifier if present (e.g., "chapter1.xhtml#section2" -> "chapter1.xhtml")
            let path_without_fragment = content_path_str.split('#').next().unwrap_or(content_path_str);
            toc_map.insert(path_without_fragment.to_string(), nav_point.label.clone());
        }
    }

    // Insert book record
    let file_path_str = epub_path.to_string_lossy().to_string();
    let new_book = NewBook {
        uid: book_uid,
        document_type: "epub",
        title: Some(&title),
        author: if author.is_empty() { None } else { Some(&author) },
        language: if language.is_empty() { None } else { Some(&language) },
        file_path: Some(&file_path_str),
        metadata_json: Some(&metadata_json),
        enable_embedded_css: true,
    };

    diesel::insert_into(books::table)
        .values(&new_book)
        .execute(db_conn)?;

    let book_id: i32 = books::table
        .filter(books::uid.eq(book_uid))
        .select(books::id)
        .first(db_conn)?;

    tracing::info!("Inserted book record with id: {}", book_id);

    // Process spine items (chapters)
    let spine_len = doc.spine.len();
    tracing::info!("Processing {} spine items", spine_len);

    // First, collect resource paths and IDs from spine
    let spine_basic_info: Vec<(usize, String, String)> = doc
        .spine
        .iter()
        .enumerate()
        .filter_map(|(idx, spine_item)| {
            let resource_item = doc.resources.get(&spine_item.idref)?;
            let resource_path = resource_item.path.to_str()?;
            Some((idx, spine_item.idref.clone(), resource_path.to_string()))
        })
        .collect();

    // Now collect titles by extracting content for each spine item
    let mut spine_info: Vec<(usize, String, String)> = Vec::new();

    for (idx, idref, resource_path) in spine_basic_info {
        // Try to get title from TOC first
        let title = if let Some(toc_title) = toc_map.get(&resource_path) {
            toc_title.clone()
        } else {
            // No TOC entry, try to extract from HTML title tag
            if let Some((content_bytes, _)) = doc.get_resource(&idref) {
                extract_html_title(&content_bytes).unwrap_or_else(|| "Untitled".to_string())
            } else {
                "Untitled".to_string()
            }
        };

        spine_info.push((idx, resource_path, title));
    }

    for (spine_index, resource_path, title) in spine_info {
        tracing::debug!("Processing spine item {}/{}: {}", spine_index + 1, spine_len, resource_path);

        // Set current chapter and extract content
        doc.set_current_chapter(spine_index);
        let (content_bytes, _mime) = doc
            .get_current()
            .ok_or_else(|| anyhow!("Failed to get content for spine item {}", spine_index))?;

        let content_html = String::from_utf8_lossy(&content_bytes).to_string();

        // Extract base directory from spine item path (e.g., "OEBPS/" from "OEBPS/cover.xhtml")
        let base_dir = std::path::Path::new(&resource_path)
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("");

        // Rewrite resource links to use API endpoint
        let content_html = rewrite_resource_links(&content_html, book_uid, base_dir);

        // Convert HTML to plain text for FTS5 indexing
        let content_plain = html_to_plain_text(&content_html);

        // Generate spine_item_uid
        let spine_item_uid = format!("{}.{}", book_uid, spine_index);

        // Insert spine item
        let new_spine_item = NewBookSpineItem {
            book_id,
            book_uid,
            spine_item_uid: &spine_item_uid,
            spine_index: spine_index as i32,
            title: Some(&title),
            language: if language.is_empty() { None } else { Some(&language) },
            content_html: Some(&content_html),
            content_plain: Some(&content_plain),
        };

        diesel::insert_into(book_spine_items::table)
            .values(&new_spine_item)
            .execute(db_conn)?;

        tracing::debug!("Inserted spine item: {}", spine_item_uid);
    }

    // Extract and store all resources (images, CSS, fonts)
    tracing::info!("Processing {} resources", doc.resources.len());
    let mut resource_count = 0;

    // Collect resource information first to avoid borrow checker issues
    let resource_list: Vec<(String, String, String)> = doc
        .resources
        .iter()
        .filter_map(|(id, resource_item)| {
            let path_str = resource_item.path.to_str()?;

            // Skip XHTML documents (they're already processed as spine items)
            if resource_item.mime.contains("xhtml") || resource_item.mime.contains("html") {
                return None;
            }

            Some((id.clone(), path_str.to_string(), resource_item.mime.clone()))
        })
        .collect();

    // Now process resources
    for (resource_id, resource_path_str, mime_type) in resource_list {
        // Get resource data
        if let Some((data, _)) = doc.get_resource(&resource_id) {
            let new_resource = NewBookResource {
                book_id,
                book_uid,
                resource_path: &resource_path_str,
                mime_type: Some(&mime_type),
                content_data: Some(&data),
            };

            diesel::insert_into(book_resources::table)
                .values(&new_resource)
                .execute(db_conn)?;

            resource_count += 1;
            tracing::debug!("Inserted resource: {} ({})", resource_path_str, mime_type);
        }
    }

    tracing::info!("Successfully imported EPUB: {} spine items, {} resources", spine_len, resource_count);
    Ok(())
}

/// Rewrite resource links in HTML to use the API endpoint format
///
/// Converts relative paths like "../images/photo.jpg" to "/book_resources/<book_uid>/OEBPS/images/photo.jpg"
/// The base_dir parameter (e.g., "OEBPS") is the directory containing the HTML file
fn rewrite_resource_links(html: &str, book_uid: &str, base_dir: &str) -> String {
    lazy_static::lazy_static! {
        // Match src="..." and href="..." attributes
        static ref RE_SRC: Regex = Regex::new(r#"(?i)(src|href)=["']([^"']+)["']"#).unwrap();
    }

    RE_SRC
        .replace_all(html, |caps: &regex::Captures| {
            let attr = &caps[1];
            let path = &caps[2];

            // Skip absolute URLs (http://, https://, //, etc.)
            if path.starts_with("http://")
                || path.starts_with("https://")
                || path.starts_with("//")
                || path.starts_with('/')
                || path.starts_with('#')
            {
                return caps[0].to_string();
            }

            // Resolve relative path from the HTML file's directory
            let full_path = if !base_dir.is_empty() {
                // Combine base_dir with the relative path
                let combined = format!("{}/{}", base_dir, path);
                // Normalize the combined path (resolve ../ and ./)
                normalize_path(&combined)
            } else {
                // No base directory, just normalize the path
                normalize_path(path)
            };

            // Rewrite to API endpoint
            format!(r#"{}="/book_resources/{}/{}""#, attr, book_uid, full_path)
        })
        .to_string()
}

/// Normalize a relative path by removing ../ and ./ components
fn normalize_path(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').collect();
    let mut normalized: Vec<&str> = Vec::new();

    for part in parts {
        match part {
            ".." => {
                normalized.pop();
            }
            "." | "" => {
                // Skip
            }
            _ => {
                normalized.push(part);
            }
        }
    }

    normalized.join("/")
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
    fn test_extract_html_title() {
        // Test extracting title with pipe separator
        let html = b"<html><head><title>Chapter 1 | Book Title</title></head></html>";
        assert_eq!(extract_html_title(html), Some("Chapter 1".to_string()));

        // Test extracting title without separator
        let html = b"<html><head><title>Simple Title</title></head></html>";
        assert_eq!(extract_html_title(html), Some("Simple Title".to_string()));

        // Test extracting title with extra whitespace
        let html = b"<html><head><title>  Spaced Title  |  Extra  </title></head></html>";
        assert_eq!(extract_html_title(html), Some("Spaced Title".to_string()));

        // Test generic titles that should be accepted
        let html = b"<html><head><title>Cover | Book Title</title></head></html>";
        assert_eq!(extract_html_title(html), Some("Cover".to_string()));

        let html = b"<html><head><title>Quote | Book Title</title></head></html>";
        assert_eq!(extract_html_title(html), Some("Quote".to_string()));

        // Test "Untitled" which should return None
        let html = b"<html><head><title>Untitled</title></head></html>";
        assert_eq!(extract_html_title(html), None);

        // Test empty title
        let html = b"<html><head><title></title></head></html>";
        assert_eq!(extract_html_title(html), None);

        // Test whitespace-only title
        let html = b"<html><head><title>   </title></head></html>";
        assert_eq!(extract_html_title(html), None);

        // Test no title tag
        let html = b"<html><head></head></html>";
        assert_eq!(extract_html_title(html), None);

        // Test case insensitive "Untitled"
        let html = b"<html><head><title>UNTITLED</title></head></html>";
        assert_eq!(extract_html_title(html), None);
    }

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path("../images/photo.jpg"), "images/photo.jpg");
        assert_eq!(normalize_path("../../styles/main.css"), "styles/main.css");
        assert_eq!(normalize_path("./image.png"), "image.png");
        assert_eq!(normalize_path("images/photo.jpg"), "images/photo.jpg");
        assert_eq!(normalize_path("../fonts/../images/photo.jpg"), "images/photo.jpg");
    }

    #[test]
    fn test_rewrite_resource_links() {
        let html = "<img src=\"../images/photo.jpg\"><link href=\"styles/main.css\">";
        let result = rewrite_resource_links(html, "testbook", "OEBPS");
        // println!("Result: {}", result);
        // With base_dir "OEBPS" and path "../images/photo.jpg"
        // Combined: "OEBPS/../images/photo.jpg" → normalized to "images/photo.jpg"
        assert!(result.contains("src=\"/book_resources/testbook/images/photo.jpg\""));
        // With base_dir "OEBPS" and path "styles/main.css"
        // Combined: "OEBPS/styles/main.css"
        assert!(result.contains("href=\"/book_resources/testbook/OEBPS/styles/main.css\""));
    }

    #[test]
    fn test_rewrite_resource_links_absolute() {
        let html = "<a href=\"http://example.com\">Link</a><a href=\"#anchor\">Anchor</a>";
        let result = rewrite_resource_links(html, "testbook", "OEBPS");
        // Absolute URLs and anchors should not be rewritten
        assert!(result.contains("href=\"http://example.com\""));
        assert!(result.contains("href=\"#anchor\""));
    }

    #[test]
    fn test_rewrite_resource_links_with_oebps_prefix() {
        // Test typical EPUB structure where HTML is in OEBPS/ and references assets relatively
        let html = r#"<img src="assets/photos/cover.jpg"><link href="styles/main.css">"#;
        let result = rewrite_resource_links(html, "bmc", "OEBPS");
        assert!(result.contains("src=\"/book_resources/bmc/OEBPS/assets/photos/cover.jpg\""));
        assert!(result.contains("href=\"/book_resources/bmc/OEBPS/styles/main.css\""));
    }

    #[test]
    fn test_rewrite_resource_links_empty_base_dir() {
        // Test when base_dir is empty (for non-EPUB or flat structure)
        let html = "<img src=\"images/photo.jpg\">";
        let result = rewrite_resource_links(html, "testbook", "");
        assert!(result.contains("src=\"/book_resources/testbook/images/photo.jpg\""));
    }

    #[test]
    fn test_rewrite_resource_links_nested_html() {
        // Test HTML in OEBPS/Text/ referencing ../Images/ (goes up to OEBPS/Images/)
        let html = r#"<img src="../Images/bmc1_cover.jpg"><link href="../Styles/style.css">"#;
        let result = rewrite_resource_links(html, "bmc", "OEBPS/Text");
        // println!("Nested result: {}", result);
        // OEBPS/Text/../Images/bmc1_cover.jpg → OEBPS/Images/bmc1_cover.jpg
        assert!(result.contains("src=\"/book_resources/bmc/OEBPS/Images/bmc1_cover.jpg\""));
        assert!(result.contains("href=\"/book_resources/bmc/OEBPS/Styles/style.css\""));
    }

    #[test]
    fn test_rewrite_resource_links_nested_html_direct_ref() {
        // Test HTML in OEBPS/Text/ with direct reference to Images/ (relative, not ../Images/)
        let html = r#"<img src="Images/bmc1_cover.jpg">"#;
        let result = rewrite_resource_links(html, "bmc", "OEBPS/Text");
        // println!("Direct ref result: {}", result);
        // This would resolve to OEBPS/Text/Images/bmc1_cover.jpg
        assert!(result.contains("src=\"/book_resources/bmc/OEBPS/Text/Images/bmc1_cover.jpg\""));
    }
}
