/// Formats an HTML string with proper indentation and newlines
///
/// # Arguments
/// * `html` - The HTML string to format
///
/// # Returns
/// A formatted HTML string with:
/// - Tags on new lines
/// - Attributes kept on the same line as the tag
/// - 4-space indentation for nested elements
/// - Preserved content inside <script> and <style> tags
pub fn html_indent(html: &str) -> String {
    let mut result = Vec::new();
    let mut indent_level: usize = 0;
    let mut current_line = String::new();
    let mut in_tag = false;
    let mut preserve_content = false;
    let mut preserve_tag_name = String::new();

    let mut chars = html.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '<' => {
                // Check if this might be a closing tag for script/style
                if preserve_content {
                    current_line.push(ch);

                    // Look ahead to see if this is the closing tag we're waiting for
                    let mut lookahead = String::from("<");
                    let mut temp_chars = chars.clone();

                    while let Some(&next_ch) = temp_chars.peek() {
                        lookahead.push(next_ch);
                        temp_chars.next();
                        if next_ch == '>' {
                            break;
                        }
                    }

                    let closing_tag = format!("</{}>", preserve_tag_name);
                    if lookahead.to_lowercase().starts_with(&closing_tag.to_lowercase()) {
                        // This is the closing tag, consume it and end preserve mode
                        for _ in 1..lookahead.len() {
                            current_line.push(chars.next().unwrap());
                        }

                        // Output the preserved content with the closing tag
                        let lines: Vec<&str> = current_line.trim().lines().collect();
                        if lines.len() > 1 {
                            // Multi-line content - preserve internal formatting
                            for (i, line) in lines.iter().enumerate() {
                                if i == lines.len() - 1 && line.starts_with("</") {
                                    // Last line is the closing tag
                                    indent_level = indent_level.saturating_sub(1);
                                    result.push(format!("{}{}",
                                        " ".repeat(indent_level * 4),
                                        line
                                    ));
                                } else {
                                    result.push(format!("{}{}",
                                        " ".repeat(indent_level * 4),
                                        line
                                    ));
                                }
                            }
                        } else {
                            // Single line content
                            indent_level = indent_level.saturating_sub(1);
                            result.push(format!("{}{}",
                                " ".repeat(indent_level * 4),
                                current_line.trim()
                            ));
                        }

                        current_line.clear();
                        preserve_content = false;
                        preserve_tag_name.clear();
                    }
                } else {
                    // Start of a tag
                    if !current_line.trim().is_empty() {
                        // Save any text content before the tag
                        result.push(format!("{}{}",
                            " ".repeat(indent_level * 4),
                            current_line.trim()
                        ));
                        current_line.clear();
                    }
                    in_tag = true;
                    current_line.push(ch);
                }
            }
            '>' => {
                current_line.push(ch);
                if in_tag && !preserve_content {
                    in_tag = false;
                    let trimmed = current_line.trim();

                    // Check if this is a script or style opening tag
                    let tag_lower = trimmed.to_lowercase();
                    if (tag_lower.starts_with("<script") || tag_lower.starts_with("<style"))
                        && !tag_lower.starts_with("</")
                        && !tag_lower.ends_with("/>") {
                        // Start preserving content
                        preserve_content = true;
                        preserve_tag_name = if tag_lower.starts_with("<script") {
                            "script".to_string()
                        } else {
                            "style".to_string()
                        };

                        result.push(format!("{}{}",
                            " ".repeat(indent_level * 4),
                            trimmed
                        ));
                        indent_level += 1;
                        current_line.clear();
                    } else if trimmed.starts_with("</") {
                        // Closing tag
                        indent_level = indent_level.saturating_sub(1);
                        result.push(format!("{}{}",
                            " ".repeat(indent_level * 4),
                            trimmed
                        ));
                        current_line.clear();
                    } else if trimmed.ends_with("/>") || is_void_element(trimmed) {
                        // Self-closing or void element
                        result.push(format!("{}{}",
                            " ".repeat(indent_level * 4),
                            trimmed
                        ));
                        current_line.clear();
                    } else if trimmed.starts_with("<!") || trimmed.starts_with("<?") {
                        // DOCTYPE, comments, or processing instructions
                        result.push(format!("{}{}",
                            " ".repeat(indent_level * 4),
                            trimmed
                        ));
                        current_line.clear();
                    } else {
                        // Opening tag
                        result.push(format!("{}{}",
                            " ".repeat(indent_level * 4),
                            trimmed
                        ));
                        indent_level += 1;
                        current_line.clear();
                    }
                }
            }
            '\n' | '\r' if !in_tag && !preserve_content => {
                // Skip newlines outside of tags and outside preserved content
                continue;
            }
            _ => {
                current_line.push(ch);
            }
        }
    }

    // Handle any remaining content
    if !current_line.trim().is_empty() {
        result.push(format!("{}{}",
            " ".repeat(indent_level * 4),
            current_line.trim()
        ));
    }

    result.join("\n")
}

/// Checks if a tag is a void element (self-closing in HTML5)
fn is_void_element(tag: &str) -> bool {
    let void_elements = [
        "area", "base", "br", "col", "embed", "hr", "img",
        "input", "link", "meta", "param", "source", "track", "wbr"
    ];

    // Extract tag name from the tag string
    let tag_lower = tag.to_lowercase();
    void_elements.iter().any(|&elem| {
        tag_lower.contains(&format!("<{}", elem)) ||
        tag_lower.contains(&format!("<{} ", elem))
    })
}

/// Extracts an HTML element with a specific ID from an indented HTML string
///
/// # Arguments
/// * `html` - The indented HTML string to search in
/// * `id` - The ID attribute value to search for
///
/// # Returns
/// * `Option<String>` - The extracted HTML element including its opening and closing tags,
///                      or None if the ID is not found
///
/// # Example
/// ```
/// let html = r#"<div>
///     <section id="content">
///         <p>Hello</p>
///     </section>
/// </div>"#;
///
/// let extracted = extract_element_by_id_from_indented(html, "content");
/// // Returns: Some("<section id=\"content\">\n        <p>Hello</p>\n    </section>")
/// ```
pub fn extract_element_by_id_from_indented(html: &str, id: &str) -> Option<String> {
    let lines: Vec<&str> = html.lines().collect();
    let id_pattern = format!(r#"id="{}""#, id);
    let id_pattern_single = format!(r#"id='{}'"#, id);

    // Find the line containing the opening tag with the specified ID
    let mut start_line_idx = None;
    let mut start_indent = 0;
    let mut tag_name = String::new();

    for (idx, line) in lines.iter().enumerate() {
        if line.contains(&id_pattern) || line.contains(&id_pattern_single) {
            // Found the line with the ID
            start_line_idx = Some(idx);

            // Calculate indentation (count leading spaces)
            start_indent = line.len() - line.trim_start().len();

            // Extract the tag name
            if let Some(tag_start) = line.trim_start().find('<') {
                let tag_content = &line.trim_start()[tag_start + 1..];
                // Find the end of the tag name (space or >)
                let tag_end = tag_content
                    .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
                    .unwrap_or(tag_content.len());
                tag_name = tag_content[..tag_end].to_string();
            }

            break;
        }
    }

    // If we didn't find the ID, return None
    let start_idx = start_line_idx?;

    // Check if it's a self-closing tag or void element
    let start_line = lines[start_idx];
    if start_line.trim().ends_with("/>") || is_void_element(start_line) {
        // Self-closing tag, return just this line
        return Some(start_line.trim().to_string());
    }

    // Find the matching closing tag at the same indentation level
    let closing_tag = format!("</{}>", tag_name);
    let mut end_line_idx = None;

    for idx in (start_idx + 1)..lines.len() {
        let line = lines[idx];
        let line_indent = line.len() - line.trim_start().len();

        // Check if this line contains a closing tag at the same or lower indentation
        if line_indent <= start_indent && line.trim().starts_with(&closing_tag) {
            end_line_idx = Some(idx);
            break;
        }
    }

    // If we found both start and end, extract the element
    if let Some(end_idx) = end_line_idx {
        let extracted_lines = &lines[start_idx..=end_idx];

        // Find the minimum indentation to remove (to left-align the result)
        let min_indent = extracted_lines
            .iter()
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.len() - line.trim_start().len())
            .min()
            .unwrap_or(0);

        // Build the result, removing the minimum indentation from each line
        let result: Vec<String> = extracted_lines
            .iter()
            .map(|line| {
                if line.len() > min_indent {
                    line[min_indent..].to_string()
                } else {
                    line.to_string()
                }
            })
            .collect();

        Some(result.join("\n"))
    } else {
        // Couldn't find closing tag, might be malformed HTML
        None
    }
}

/// Extracts multiple HTML elements by their IDs
///
/// # Arguments
/// * `html` - The HTML string to search in (will run html_indent() on it)
/// * `ids` - A slice of ID values to search for
///
/// # Returns
/// * `Vec<(String, String)>` - A vector of tuples containing (id, extracted_html)
pub fn extract_elements_by_ids(un_indented_html: &str, ids: &[&str]) -> Vec<(String, String)> {
    let mut results = Vec::new();
    let indented_html = html_indent(un_indented_html);

    for id in ids {
        if let Some(element) = extract_element_by_id_from_indented(&indented_html, id) {
            results.push((id.to_string(), element));
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_html() {
        let input = "<div><p>Hello</p></div>";
        let expected = "<div>\n    <p>\n        Hello\n    </p>\n</div>";
        assert_eq!(html_indent(input), expected);
    }

    #[test]
    fn test_attributes() {
        let input = r#"<div class="container" id="main"><span style="color: red;">Text</span></div>"#;
        let output = html_indent(input);
        assert!(output.contains(r#"<div class="container" id="main">"#));
        assert!(output.contains(r#"<span style="color: red;">"#));
    }

    #[test]
    fn test_self_closing() {
        let input = "<div><img src=\"test.jpg\" /><br><input type=\"text\"></div>";
        let output = html_indent(input);
        assert!(output.contains("    <img"));
        assert!(output.contains("    <br>"));
        assert!(output.contains("    <input"));
    }

    #[test]
    fn test_nested_elements() {
        let input = "<html><body><div><p>Text</p></div></body></html>";
        let output = html_indent(input);
        let lines: Vec<&str> = output.lines().collect();

        // Check indentation levels
        assert!(lines[0].starts_with("<html>"));
        assert!(lines[1].starts_with("    <body>"));
        assert!(lines[2].starts_with("        <div>"));
        assert!(lines[3].starts_with("            <p>"));
    }

    #[test]
    fn test_script_tag_preservation() {
        let input = r#"<html><head><script>function test() { console.log("hello"); }</script></head></html>"#;
        let output = html_indent(input);

        // Script content should be preserved
        assert!(output.contains(r#"function test() { console.log("hello"); }"#));

        // But surrounding structure should be formatted
        assert!(output.contains("<html>\n"));
        assert!(output.contains("    <head>\n"));
    }

    #[test]
    fn test_style_tag_preservation() {
        let input = r#"<html><head><style>.container { display: flex; }</style></head></html>"#;
        let output = html_indent(input);

        // Style content should be preserved
        assert!(output.contains(".container { display: flex; }"));

        // But surrounding structure should be formatted
        assert!(output.contains("<html>\n"));
        assert!(output.contains("    <head>\n"));
    }

    #[test]
    fn test_multiline_script() {
        let input = r#"<div><script>
function test() {
    console.log("line 1");
    console.log("line 2");
}
</script><p>After script</p></div>"#;

        let output = html_indent(input);

        // Check that script content formatting is preserved
        assert!(output.contains("function test() {"));
        assert!(output.contains("    console.log("));

        // Check that HTML after script is still formatted
        assert!(output.contains("    <p>"));
    }

    #[test]
    fn test_style_with_media_queries() {
        let input = r#"<style>
@media (max-width: 768px) {
    .container { width: 100%; }
}
</style>"#;

        let output = html_indent(input);

        // Media query formatting should be preserved
        assert!(output.contains("@media (max-width: 768px) {"));
        assert!(output.contains("    .container { width: 100%; }"));
    }

    #[test]
    fn test_extract_element_by_id_from_indented() {
        let html = r#"<html>
    <body>
        <div id="header">
            <h1>Title</h1>
            <nav>
                <a href="/">Home</a>
            </nav>
        </div>
        <div id="content">
            <p>Main content</p>
        </div>
    </body>
</html>"#;

        let extracted = extract_element_by_id_from_indented(html, "header").unwrap();
        assert!(extracted.contains(r#"<div id="header">"#));
        assert!(extracted.contains("<h1>Title</h1>"));
        assert!(extracted.contains("</div>"));

        let content = extract_element_by_id_from_indented(html, "content").unwrap();
        assert!(content.contains(r#"<div id="content">"#));
        assert!(content.contains("<p>Main content</p>"));
    }

    #[test]
    fn test_extract_nested_element() {
        let html = r#"<div>
    <section id="main">
        <article>
            <div id="nested">
                <p>Nested content</p>
            </div>
        </article>
    </section>
</div>"#;

        let nested = extract_element_by_id_from_indented(html, "nested").unwrap();
        assert!(nested.contains(r#"<div id="nested">"#));
        assert!(nested.contains("<p>Nested content</p>"));
        assert!(!nested.contains("<article>")); // Should not include parent
    }

    #[test]
    fn test_extract_self_closing_element() {
        let html = r#"<div>
    <img id="logo" src="logo.png" alt="Logo"/>
    <p>Text</p>
</div>"#;

        let img = extract_element_by_id_from_indented(html, "logo").unwrap();
        assert_eq!(img, r#"<img id="logo" src="logo.png" alt="Logo"/>"#);
    }

    #[test]
    fn test_extract_nonexistent_id() {
        let html = r#"<div id="exists">Content</div>"#;
        let result = extract_element_by_id_from_indented(html, "doesnotexist");
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_multiple_ids() {
        let html = r#"<html>
    <div id="first">First</div>
    <div id="second">Second</div>
    <div id="third">Third</div>
</html>"#;

        let results = extract_elements_by_ids(html, &["first", "third", "nonexistent"]);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, "first");
        assert!(results[0].1.contains("First"));
        assert_eq!(results[1].0, "third");
        assert!(results[1].1.contains("Third"));
    }
}

// Example usage
fn _example() {
    // Example 1: Format HTML
    let html = r#"<html>
<head>
<title>Test Page</title>
<style>
body {
    font-family: Arial, sans-serif;
    margin: 0;
    padding: 20px;
}
.container {
    max-width: 1200px;
    margin: 0 auto;
}
</style>
<script>
function init() {
    console.log("Page loaded");
    const elements = document.querySelectorAll('.container');
    elements.forEach(el => {
        el.style.opacity = '1';
    });
}
</script>
</head>
<body onload="init()">
<div class="container" id="main-container"><h1>Welcome</h1><p>This is a test page with <strong>formatted HTML</strong>.</p><img src="logo.png" alt="Logo"/></div>
<footer id="page-footer"><p>Copyright 2024</p><nav id="footer-nav"><a href="/privacy">Privacy</a><a href="/terms">Terms</a></nav></footer>
</body>
</html>"#;

    println!("=== HTML Formatting Example ===\n");
    println!("Original HTML:");
    println!("{}\n", html);

    let formatted = html_indent(html);
    println!("Formatted HTML:");
    println!("{}\n", formatted);

    // Example 2: Extract element by ID
    println!("=== Element Extraction Example ===\n");

    if let Some(container) = extract_element_by_id_from_indented(&formatted, "main-container") {
        println!("Extracted element with id='main-container':");
        println!("{}\n", container);
    }

    if let Some(footer) = extract_element_by_id_from_indented(&formatted, "page-footer") {
        println!("Extracted element with id='page-footer':");
        println!("{}\n", footer);
    }

    if let Some(nav) = extract_element_by_id_from_indented(&formatted, "footer-nav") {
        println!("Extracted nested element with id='footer-nav':");
        println!("{}\n", nav);
    }

    // Example 3: Extract multiple elements
    println!("=== Multiple Elements Extraction ===\n");
    let ids_to_extract = vec!["main-container", "footer-nav"];
    let extracted_elements = extract_elements_by_ids(&formatted, &ids_to_extract);

    for (id, element) in extracted_elements {
        println!("Element with id='{}':", id);
        println!("{}\n", element);
    }
}
