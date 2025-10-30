// Character encoding detection and conversion for Tipitaka XML files
// Handles UTF-16LE with BOM to UTF-8 conversion and CRLF to LF normalization

use anyhow::{Context, Result};
use encoding_rs::{Encoding, UTF_16LE, UTF_16BE, UTF_8};
use std::fs::File;
use std::io::Read;
use std::path::Path;

use simsapa_backend::logger;

/// Reads an XML file, detects encoding, and converts to UTF-8 with Unix line endings
pub fn read_xml_file(path: &Path) -> Result<String> {
    // Read file as raw bytes
    let mut file = File::open(path)
        .context(format!("Failed to open file: {:?}", path))?;
    
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .context(format!("Failed to read file: {:?}", path))?;
    
    // Detect encoding by checking BOM
    let (encoding, has_bom) = detect_encoding(&bytes);
    
    logger::info(&format!(
        "File: {:?}, Encoding: {}, BOM: {}",
        path.file_name().unwrap_or_default(),
        encoding.name(),
        has_bom
    ));
    
    // Skip BOM bytes if present
    let bytes_without_bom = if has_bom {
        match encoding {
            e if e == UTF_16LE || e == UTF_16BE => &bytes[2..], // UTF-16 BOM is 2 bytes
            _ => &bytes[3..], // UTF-8 BOM is 3 bytes
        }
    } else {
        &bytes
    };
    
    // Decode to UTF-8
    let (decoded, _encoding_used, had_errors) = encoding.decode(bytes_without_bom);
    
    if had_errors {
        logger::warn(&format!("Encoding errors detected while decoding {:?}", path));
    }
    
    // Convert CRLF to LF (Windows to Unix line endings)
    let unix_text = decoded.replace("\r\n", "\n");
    
    Ok(unix_text)
}

/// Detects file encoding by examining BOM (Byte Order Mark)
fn detect_encoding(bytes: &[u8]) -> (&'static Encoding, bool) {
    // UTF-16LE BOM: 0xFF 0xFE
    if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
        return (UTF_16LE, true);
    }
    
    // UTF-16BE BOM: 0xFE 0xFF
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        return (UTF_16BE, true);
    }
    
    // UTF-8 BOM: 0xEF 0xBB 0xBF
    if bytes.len() >= 3 && bytes[0] == 0xEF && bytes[1] == 0xBB && bytes[2] == 0xBF {
        return (UTF_8, true);
    }
    
    // No BOM detected, assume UTF-8
    (UTF_8, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_utf16le_bom() {
        let bytes = vec![0xFF, 0xFE, 0x41, 0x00]; // UTF-16LE BOM + "A"
        let (encoding, has_bom) = detect_encoding(&bytes);
        assert_eq!(encoding, UTF_16LE);
        assert!(has_bom);
    }

    #[test]
    fn test_detect_utf16be_bom() {
        let bytes = vec![0xFE, 0xFF, 0x00, 0x41]; // UTF-16BE BOM + "A"
        let (encoding, has_bom) = detect_encoding(&bytes);
        assert_eq!(encoding, UTF_16BE);
        assert!(has_bom);
    }

    #[test]
    fn test_detect_utf8_bom() {
        let bytes = vec![0xEF, 0xBB, 0xBF, 0x41]; // UTF-8 BOM + "A"
        let (encoding, has_bom) = detect_encoding(&bytes);
        assert_eq!(encoding, UTF_8);
        assert!(has_bom);
    }

    #[test]
    fn test_detect_no_bom() {
        let bytes = vec![0x41, 0x42, 0x43]; // "ABC" in ASCII/UTF-8
        let (encoding, has_bom) = detect_encoding(&bytes);
        assert_eq!(encoding, UTF_8);
        assert!(!has_bom);
    }

    #[test]
    fn test_crlf_to_lf_conversion() {
        let input = "Line 1\r\nLine 2\r\nLine 3";
        let output = input.replace("\r\n", "\n");
        assert_eq!(output, "Line 1\nLine 2\nLine 3");
    }
}
