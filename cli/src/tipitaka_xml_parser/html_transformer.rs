//! XML to HTML transformation following tipitaka.org XSL patterns

use anyhow::Result;
use crate::tipitaka_xml_parser::types::*;

/// Transform XML elements to HTML
pub fn transform_to_html(xml_elements: &[XmlElement]) -> Result<String> {
    let mut html = String::new();
    
    for element in xml_elements {
        match element {
            XmlElement::Paragraph { rend, n, content } => {
                let class = match rend.as_str() {
                    "centre" => "centered",
                    _ => rend.as_str(),
                };
                
                html.push_str(&format!("<p class=\"{}\">", class));
                
                // Add paragraph number if present
                if let Some(num) = n {
                    html.push_str(&format!("<span class=\"paranum\">{}</span> ", num));
                }
                
                // Transform content nodes
                for node in content {
                    html.push_str(&transform_content_node(node));
                }
                
                html.push_str("</p>\n");
            }
            XmlElement::HighlightedText { rend, content } => {
                html.push_str(&format!("<span class=\"{}\">{}</span>", rend, html_escape::encode_text(content)));
            }
            XmlElement::Note { content } => {
                html.push_str(&format!("<span class=\"note\">[{}]</span>", html_escape::encode_text(content)));
            }
            XmlElement::PageBreak { ed, n } => {
                html.push_str(&format!("<span class=\"pagebreak\" data-ed=\"{}\" data-n=\"{}\"></span>", ed, n));
            }
        }
    }
    
    Ok(html)
}

/// Transform a content node to HTML
fn transform_content_node(node: &ContentNode) -> String {
    match node {
        ContentNode::Text(text) => html_escape::encode_text(text).to_string(),
        ContentNode::Hi(rend, text) => {
            format!("<span class=\"{}\">{}</span>", rend, html_escape::encode_text(text))
        }
        ContentNode::Note(text) => {
            format!("<span class=\"note\">[{}]</span>", html_escape::encode_text(text))
        }
        ContentNode::PageBreak { ed, n } => {
            format!("<span class=\"pagebreak\" data-ed=\"{}\" data-n=\"{}\"></span>", ed, n)
        }
    }
}

/// Extract plain text from XML elements (strip HTML)
pub fn extract_plain_text(xml_elements: &[XmlElement]) -> String {
    let mut text = String::new();
    
    for element in xml_elements {
        match element {
            XmlElement::Paragraph { content, .. } => {
                for node in content {
                    text.push_str(&extract_text_from_node(node));
                    text.push(' ');
                }
                text.push('\n');
            }
            XmlElement::HighlightedText { content, .. } => {
                text.push_str(content);
            }
            XmlElement::Note { .. } => {
                // Skip notes in plain text
            }
            XmlElement::PageBreak { .. } => {
                // Skip page breaks in plain text
            }
        }
    }
    
    text.trim().to_string()
}

/// Extract text from a content node
fn extract_text_from_node(node: &ContentNode) -> String {
    match node {
        ContentNode::Text(text) => text.clone(),
        ContentNode::Hi(_, text) => text.clone(),
        ContentNode::Note(_) => String::new(), // Skip notes
        ContentNode::PageBreak { .. } => String::new(), // Skip page breaks
    }
}
