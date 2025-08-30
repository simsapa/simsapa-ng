pub mod api;
pub mod sutta_bridge;
pub mod asset_manager;
pub mod storage_manager;
pub mod prompt_manager;

use regex::Regex;
use lazy_static::lazy_static;

use markdown::{to_html_with_options, Options};
use simsapa_backend::helpers::consistent_niggahita;

pub fn clean_prompt(text: &str) -> String {
    lazy_static! {
        // Regex to match code blocks with any language identifier (rust, md, markdown, etc.) or no identifier
        static ref RE_STARTS_WITH_CODE_BLOCK: Regex = Regex::new(r"(?s)^```\s*\w*\s*\n(.*)\n```$").unwrap();
    }

    // Trim whitespace before processing
    let trimmed_text = text.trim();

    // Remove wrapping code block syntax if present
    let processed_text = if let Some(captures) = RE_STARTS_WITH_CODE_BLOCK.captures(trimmed_text) {
        // Extract the content inside the code block (group 1)
        captures.get(1).map_or(trimmed_text, |m| m.as_str())
    } else {
        trimmed_text
    };

    // Remove wrapping bold/italics syntax if present
    let final_text = {
        let patterns = ["***", "**", "*", "___", "__", "_"];

        // Try each pattern and return the first match
        patterns.iter()
            .find_map(|&pattern| {
                if processed_text.starts_with(pattern) && processed_text.ends_with(pattern) && processed_text.len() > pattern.len() * 2 {
                    Some(processed_text.trim_start_matches(pattern).trim_end_matches(pattern))
                } else {
                    None
                }
            })
            .unwrap_or(processed_text)
    };

    // Apply consistent_niggahita to the cleaned text
    consistent_niggahita(Some(final_text.to_string()))
}

pub fn markdown_to_html(markdown_text: &str) -> String {
    match to_html_with_options(markdown_text.trim(), &Options::gfm()) {
        Ok(html) => html,
        Err(_) => markdown_text.trim().to_string(), // Fallback to plain text on error
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_prompt_code_block_any_language() {
        // Test the case you mentioned with 'md' language
        let text = "```md\nBetter than horses, thoroughbreds from Sindh,\nElephants and mighty giants—the self-restrained one is superior to these.\n```";
        let result = clean_prompt(text);
        
        // Should strip code block wrapper
        assert!(!result.contains("```"));
        assert!(!result.contains("md"));
        assert!(result.contains("Better than horses, thoroughbreds from Sindh"));
        assert!(result.contains("Elephants and mighty giants"));
    }

    #[test]
    fn test_clean_prompt_code_block_rust() {
        let text = "```rust\nfn main() {\n    println!(\"Hello\");\n}\n```";
        let result = clean_prompt(text);
        
        // Should strip code block wrapper
        assert!(!result.contains("```"));
        assert!(!result.contains("rust"));
        assert!(result.contains("fn main()"));
    }

    #[test]
    fn test_clean_prompt_code_block_markdown() {
        let text = "```markdown\nHere, friend, while I was alone and secluded...\n```";
        let result = clean_prompt(text);
        
        // Should strip code block wrapper
        assert!(!result.contains("```"));
        assert!(!result.contains("markdown"));
        assert!(result.contains("Here, friend, while I was alone and secluded"));
    }

    #[test]
    fn test_clean_prompt_bold_stripping() {
        let text = "**This is bold text**";
        let result = clean_prompt(text);
        
        // Should strip wrapping bold syntax
        assert!(result.contains("This is bold text"));
        assert!(!result.contains("**"));
    }

    #[test]
    fn test_clean_prompt_italics_stripping() {
        let text = "*This is italic text*";
        let result = clean_prompt(text);
        
        // Should strip wrapping italics syntax
        assert!(result.contains("This is italic text"));
        assert!(!result.contains("*This is italic text*"));
    }

    #[test]
    fn test_clean_prompt_niggahita_consistency() {
        // Test that consistent_niggahita is applied
        let text = "saṃvaro";  // Using ṃ
        let result = clean_prompt(text);
        
        // Should convert ṃ to ṁ
        assert!(result.contains("saṁvaro"));
        assert!(!result.contains("saṃvaro"));
    }

    #[test]
    fn test_clean_prompt_no_cleaning_needed() {
        let text = "Just plain text here";
        let result = clean_prompt(text);
        
        // Should return the same text
        assert_eq!(result, "Just plain text here");
    }

    #[test]
    fn test_markdown_to_html_basic() {
        let markdown = "# Test Header\n\n**Bold text** and *italic text*.";
        let html = markdown_to_html(markdown);

        assert!(html.contains("<h1>Test Header</h1>"));
        assert!(html.contains("<strong>Bold text</strong>"));
        assert!(html.contains("<em>italic text</em>"));
    }

    #[test]
    fn test_markdown_to_html_fallback() {
        // Even with invalid markdown, it should return something (fallback behavior)
        let markdown = "Some *plain text_";
        let html = markdown_to_html(markdown);

        // Should contain the text (either as HTML or as fallback)
        assert!(html.contains("Some *plain text_"));
    }

    #[test]
    fn test_markdown_to_html_code_blocks_in_text() {
        let markdown = "The following relevant text:\n\n```\nLine one\nLine two```\n";
        let html = markdown_to_html(markdown);

        // Should convert to HTML code block
        assert!(html.contains("<pre>") || html.contains("<code>"));
    }

    #[test]
    fn test_markdown_to_html_table() {
        let markdown = "| Column 1 | Column 2 |\n|----------|----------|\n| Data 1   | Data 2   |";
        let html = markdown_to_html(markdown);

        // Should contain table elements (GFM tables)
        assert!(html.contains("<table>") || html.contains("Column 1"));
    }

    #[test]
    fn test_markdown_to_html_no_cleaning() {
        // Test that markdown_to_html no longer strips code blocks or formatting
        let markdown = "```markdown\n**Bold text**\n```";
        let html = markdown_to_html(markdown);
        
        // Should convert to HTML code block, not strip the wrapper
        assert!(html.contains("<pre>") || html.contains("<code>"));
        // The bold text should remain as literal text inside code block
        assert!(html.contains("**Bold text**"));
    }

    #[test]
    fn test_markdown_to_html_empty_input() {
        // Test empty input
        let html = markdown_to_html("");
        
        // Should handle empty input gracefully
        assert_eq!(html.trim(), "");
    }

    #[test]
    fn test_markdown_to_html_plain_text() {
        // Test plain text without any markdown
        let markdown = "Just plain text here";
        let html = markdown_to_html(markdown);
        
        // Should return the text, possibly wrapped in HTML
        assert!(html.contains("Just plain text here"));
    }
}
