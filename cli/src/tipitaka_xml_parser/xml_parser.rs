//! XML parsing logic for Tipitaka files
//! Parses the idiosyncratic VRI CST XML structure

use anyhow::{Context, Result};
use quick_xml::events::Event;
use quick_xml::Reader;
use crate::tipitaka_xml_parser::types::*;

/// Parse a Tipitaka XML file into hierarchical structure
///
/// Handles the Majjhima Nikaya structure:
/// - `<p rend="nikaya">` → nikaya name
/// - `<div type="book">` + `<head rend="book">` → book
/// - `<div type="vagga">` + `<head rend="chapter">` → vagga
/// - `<p rend="subhead">` → sutta title (marks sutta boundaries)
pub fn parse_xml(content: &str) -> Result<TipitakaCollection> {
    let mut reader = Reader::from_str(content);
    reader.trim_text(true);
    
    let mut nikaya = String::new();
    let mut books = Vec::new();
    let mut current_book: Option<Book> = None;
    let mut current_vagga: Option<Vagga> = None;
    let mut current_sutta: Option<Sutta> = None;
    
    let mut buf = Vec::new();
    
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let tag_name = e.name();
                let tag_str = std::str::from_utf8(tag_name.as_ref())?;
                
                match tag_str {
                    "div" => {
                        // Check type attribute
                        if let Some(div_type) = get_attribute(e, b"type") {
                            if div_type == "book" {
                                // Save previous book if exists
                                if let Some(mut book) = current_book.take() {
                                    if let Some(vagga) = current_vagga.take() {
                                        book.vaggas.push(vagga);
                                    }
                                    books.push(book);
                                }
                                
                                // Start new book
                                let id = get_attribute(e, b"id").unwrap_or_default();
                                current_book = Some(Book {
                                    id,
                                    title: String::new(),
                                    vaggas: Vec::new(),
                                });
                            } else if div_type == "vagga" {
                                // Save previous vagga if exists
                                if let Some(vagga) = current_vagga.take() {
                                    if let Some(ref mut book) = current_book {
                                        book.vaggas.push(vagga);
                                    }
                                }
                                
                                // Start new vagga
                                let id = get_attribute(e, b"id").unwrap_or_default();
                                current_vagga = Some(Vagga {
                                    id,
                                    title: String::new(),
                                    suttas: Vec::new(),
                                });
                            }
                        }
                    }
                    "p" => {
                        if let Some(rend) = get_attribute(e, b"rend") {
                            match rend.as_str() {
                                "nikaya" => {
                                    // Read nikaya name
                                    nikaya = read_text_content(&mut reader)?;
                                }
                                "subhead" => {
                                    // Save previous sutta if exists
                                    if let Some(sutta) = current_sutta.take() {
                                        if let Some(ref mut vagga) = current_vagga {
                                            vagga.suttas.push(sutta);
                                        }
                                    }
                                    
                                    // Start new sutta
                                    let title = read_text_content(&mut reader)?;
                                    current_sutta = Some(Sutta {
                                        title,
                                        content_xml: Vec::new(),
                                        metadata: SuttaMetadata {
                                            uid: String::new(),
                                            sutta_ref: String::new(),
                                            nikaya: nikaya.clone(),
                                            group_path: String::new(),
                                            group_index: None,
                                            order_index: None,
                                        },
                                    });
                                }
                                _ => {
                                    // Parse as paragraph element for sutta content
                                    if current_sutta.is_some() {
                                        let paragraph = parse_paragraph(&mut reader, e, &rend)?;
                                        if let Some(ref mut sutta) = current_sutta {
                                            sutta.content_xml.push(paragraph);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    "head" => {
                        if let Some(rend) = get_attribute(e, b"rend") {
                            let text = read_text_content(&mut reader)?;
                            match rend.as_str() {
                                "book" => {
                                    if let Some(ref mut book) = current_book {
                                        book.title = text;
                                    }
                                }
                                "chapter" => {
                                    if let Some(ref mut vagga) = current_vagga {
                                        vagga.title = text;
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(anyhow::anyhow!("XML parse error at position {}: {:?}", reader.buffer_position(), e)),
            _ => {}
        }
        
        buf.clear();
    }
    
    // Save final structures
    if let Some(sutta) = current_sutta.take() {
        if let Some(ref mut vagga) = current_vagga {
            vagga.suttas.push(sutta);
        }
    }
    if let Some(vagga) = current_vagga.take() {
        if let Some(ref mut book) = current_book {
            book.vaggas.push(vagga);
        }
    }
    if let Some(book) = current_book.take() {
        books.push(book);
    }
    
    Ok(TipitakaCollection {
        nikaya,
        books,
    })
}

/// Parse a paragraph element with its content nodes
fn parse_paragraph(
    reader: &mut Reader<&[u8]>,
    start: &quick_xml::events::BytesStart,
    rend: &str,
) -> Result<XmlElement> {
    let n = get_attribute(start, b"n");
    let mut content = Vec::new();
    let mut buf = Vec::new();
    
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name_bytes = e.name();
                let tag_name = std::str::from_utf8(name_bytes.as_ref())?.to_string();
                match tag_name.as_str() {
                    "hi" => {
                        let hi_rend = get_attribute(e, b"rend").unwrap_or_default();
                        let text = read_text_content(reader)?;
                        content.push(ContentNode::Hi(hi_rend, text));
                    }
                    "note" => {
                        let text = read_text_content(reader)?;
                        content.push(ContentNode::Note(text));
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(ref e)) => {
                let name_bytes = e.name();
                let tag_name = std::str::from_utf8(name_bytes.as_ref())?.to_string();
                if tag_name == "pb" {
                    let ed = get_attribute(e, b"ed").unwrap_or_default();
                    let n = get_attribute(e, b"n").unwrap_or_default();
                    content.push(ContentNode::PageBreak { ed, n });
                }
            }
            Ok(Event::Text(ref e)) => {
                let text = e.unescape()?.to_string();
                if !text.trim().is_empty() {
                    content.push(ContentNode::Text(text));
                }
            }
            Ok(Event::End(ref e)) => {
                let name_bytes = e.name();
                let tag_name = std::str::from_utf8(name_bytes.as_ref())?.to_string();
                if tag_name == "p" {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(anyhow::anyhow!("Error parsing paragraph: {:?}", e)),
            _ => {}
        }
        buf.clear();
    }
    
    Ok(XmlElement::Paragraph {
        rend: rend.to_string(),
        n,
        content,
    })
}

/// Get an attribute value from a BytesStart element
fn get_attribute(element: &quick_xml::events::BytesStart, attr_name: &[u8]) -> Option<String> {
    element
        .attributes()
        .filter_map(|a| a.ok())
        .find(|a| a.key.as_ref() == attr_name)
        .map(|a| {
            String::from_utf8(a.value.to_vec()).unwrap_or_default()
        })
}

/// Read text content until the closing tag
fn read_text_content(reader: &mut Reader<&[u8]>) -> Result<String> {
    let mut text = String::new();
    let mut buf = Vec::new();
    
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Text(e)) => {
                text.push_str(&e.unescape()?);
            }
            Ok(Event::End(_)) => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(anyhow::anyhow!("Error reading text: {:?}", e)),
            _ => {}
        }
        buf.clear();
    }
    
    Ok(text.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_nikaya_heading() {
        let xml = r#"<?xml version="1.0"?>
            <text>
                <body>
                    <p rend="nikaya">Majjhimanikāyo</p>
                </body>
            </text>"#;
        
        let result = parse_xml(xml).unwrap();
        assert_eq!(result.nikaya, "Majjhimanikāyo");
    }

    #[test]
    fn test_parse_book_structure() {
        let xml = r#"<?xml version="1.0"?>
            <text>
                <body>
                    <p rend="nikaya">Majjhimanikāyo</p>
                    <div id="mn1" n="mn1" type="book">
                        <head rend="book">Mūlapaṇṇāsapāḷi</head>
                    </div>
                </body>
            </text>"#;
        
        let result = parse_xml(xml).unwrap();
        assert_eq!(result.books.len(), 1);
        assert_eq!(result.books[0].id, "mn1");
        assert_eq!(result.books[0].title, "Mūlapaṇṇāsapāḷi");
    }

    #[test]
    fn test_parse_vagga_structure() {
        let xml = r#"<?xml version="1.0"?>
            <text>
                <body>
                    <p rend="nikaya">Majjhimanikāyo</p>
                    <div id="mn1" type="book">
                        <head rend="book">Mūlapaṇṇāsapāḷi</head>
                        <div id="mn1_1" type="vagga">
                            <head rend="chapter">1. Mūlapariyāyavaggo</head>
                        </div>
                    </div>
                </body>
            </text>"#;
        
        let result = parse_xml(xml).unwrap();
        assert_eq!(result.books[0].vaggas.len(), 1);
        assert_eq!(result.books[0].vaggas[0].id, "mn1_1");
        assert_eq!(result.books[0].vaggas[0].title, "1. Mūlapariyāyavaggo");
    }

    #[test]
    fn test_parse_sutta_subhead() {
        let xml = r#"<?xml version="1.0"?>
            <text>
                <body>
                    <p rend="nikaya">Majjhimanikāyo</p>
                    <div id="mn1" type="book">
                        <head rend="book">Mūlapaṇṇāsapāḷi</head>
                        <div id="mn1_1" type="vagga">
                            <head rend="chapter">1. Mūlapariyāyavaggo</head>
                            <p rend="subhead">1. Mūlapariyāyasuttaṃ</p>
                        </div>
                    </div>
                </body>
            </text>"#;
        
        let result = parse_xml(xml).unwrap();
        assert_eq!(result.books[0].vaggas[0].suttas.len(), 1);
        assert_eq!(result.books[0].vaggas[0].suttas[0].title, "1. Mūlapariyāyasuttaṃ");
    }

    #[test]
    fn test_parse_paragraph_with_hi() {
        let xml = r#"<?xml version="1.0"?>
            <text>
                <body>
                    <p rend="nikaya">Majjhimanikāyo</p>
                    <div id="mn1" type="book">
                        <head rend="book">Test</head>
                        <div id="mn1_1" type="vagga">
                            <head rend="chapter">Test Vagga</head>
                            <p rend="subhead">Test Sutta</p>
                            <p rend="bodytext" n="1"><hi rend="paranum">1</hi><hi rend="dot">.</hi> Evaṃ me sutaṃ</p>
                        </div>
                    </div>
                </body>
            </text>"#;
        
        let result = parse_xml(xml).unwrap();
        let sutta = &result.books[0].vaggas[0].suttas[0];
        
        assert_eq!(sutta.title, "Test Sutta");
        assert_eq!(sutta.content_xml.len(), 1);
        
        if let XmlElement::Paragraph { rend, n, content } = &sutta.content_xml[0] {
            assert_eq!(rend, "bodytext");
            assert_eq!(n.as_ref().unwrap(), "1");
            assert!(content.len() >= 3); // paranum, dot, text
        } else {
            panic!("Expected Paragraph element");
        }
    }
}
