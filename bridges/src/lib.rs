pub mod api;
pub mod sutta_bridge;
pub mod asset_manager;
pub mod storage_manager;
pub mod prompt_manager;

use regex::Regex;
use lazy_static::lazy_static;

use markdown::{to_html_with_options, Options};

fn markdown_to_html(markdown_text: &str) -> String {
    lazy_static! {
        // Regex to test if string starts with "``` markdown" code block syntax. The "markdown" identifier can be optional.
        static ref RE_STARTS_WITH_CODE_BLOCK: Regex = Regex::new(r"(?s)^```\s*(?:markdown)?\s*\n(.*)\n```$").unwrap();
    }

    // Trim whitespace before processing
    let trimmed_text = markdown_text.trim();

    // Remove wrapping markdown code block syntax if present
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

    match to_html_with_options(final_text, &Options::gfm()) {
        Ok(html) => html,
        Err(_) => final_text.to_string(), // Fallback to plain text on error
    }
}
