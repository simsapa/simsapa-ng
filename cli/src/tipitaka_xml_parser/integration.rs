//! Integration module that ties together all Tipitaka XML import functionality
//!
//! This module provides a high-level API for processing Tipitaka XML files
//! and importing them into the SQLite database.

use anyhow::{Context, Result};
use diesel::sqlite::SqliteConnection;
use std::path::Path;

use super::encoding::read_xml_file;
use super::xml_parser::parse_xml;
use super::html_transformer::{transform_to_html, extract_plain_text};
use super::database_inserter::{insert_sutta, insert_suttas_batch};
use super::uid_generator::CstMapping;
use super::types::{TipitakaCollection, Sutta};

/// Statistics for a single file import
#[derive(Debug, Default)]
pub struct FileImportStats {
    pub filename: String,
    pub nikaya: String,
    pub books: usize,
    pub vaggas: usize,
    pub suttas_total: usize,
    pub suttas_inserted: usize,
    pub suttas_failed: usize,
}

/// Complete import process for a single XML file
pub struct TipitakaImporter {
    cst_mapping: CstMapping,
    verbose: bool,
}

impl TipitakaImporter {
    /// Create a new importer with CST mapping
    pub fn new(tsv_path: &Path, verbose: bool) -> Result<Self> {
        let cst_mapping = CstMapping::load_from_tsv(tsv_path)
            .context("Failed to load CST mapping")?;
        
        Ok(Self {
            cst_mapping,
            verbose,
        })
    }
    
    /// Process a single XML file and import into database
    pub fn process_file(
        &self,
        xml_path: &Path,
        conn: &mut SqliteConnection,
    ) -> Result<FileImportStats> {
        let filename = xml_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        if self.verbose {
            println!("  → Reading and converting encoding...");
        }
        
        // Step 1: Read and convert encoding
        let xml_content = read_xml_file(xml_path)
            .context("Failed to read XML file")?;
        
        if self.verbose {
            println!("  ✓ Encoding conversion successful");
            println!("  → Parsing XML structure...");
        }
        
        // Step 2: Parse XML
        let collection = parse_xml(&xml_content)
            .context("Failed to parse XML")?;
        
        if self.verbose {
            println!("  ✓ XML parsed successfully");
            println!("  → Generating UIDs and transforming to HTML...");
        }
        
        // Step 3: Process each sutta
        let mut suttas_to_insert = Vec::new();
        let mut sutta_index = 0;
        
        for book in &collection.books {
            for vagga in &book.vaggas {
                for sutta in &vagga.suttas {
                    sutta_index += 1;
                    
                    // Generate UID
                    let uid = self.generate_sutta_uid(&filename, &book.id, sutta_index);
                    
                    // Build group path
                    let group_path = format!("{}/{}/{}", 
                        collection.nikaya, 
                        book.title, 
                        vagga.title
                    );
                    
                    // Transform to HTML
                    let html = transform_to_html(&sutta.content_xml)
                        .context("Failed to transform to HTML")?;
                    
                    // Extract plain text
                    let plain = extract_plain_text(&sutta.content_xml);
                    
                    // Create sutta with metadata
                    let mut sutta_with_metadata = sutta.clone();
                    sutta_with_metadata.metadata.uid = uid;
                    sutta_with_metadata.metadata.sutta_ref = format!("{} {}", 
                        book.id.to_uppercase(), 
                        sutta_index
                    );
                    sutta_with_metadata.metadata.group_path = group_path;
                    sutta_with_metadata.metadata.order_index = Some(sutta_index as i32);
                    
                    suttas_to_insert.push((sutta_with_metadata, html, plain));
                }
            }
        }
        
        if self.verbose {
            println!("  ✓ Prepared {} suttas for insertion", suttas_to_insert.len());
            println!("  → Inserting into database...");
        }
        
        // Step 4: Insert into database
        let inserted = insert_suttas_batch(conn, &suttas_to_insert)
            .context("Failed to insert suttas")?;
        
        if self.verbose {
            println!("  ✓ Inserted {} suttas", inserted);
        }
        
        // Calculate statistics
        let book_count = collection.books.len();
        let vagga_count: usize = collection.books.iter()
            .map(|b| b.vaggas.len())
            .sum();
        let sutta_count = suttas_to_insert.len();
        let failed = sutta_count - inserted;
        
        Ok(FileImportStats {
            filename,
            nikaya: collection.nikaya,
            books: book_count,
            vaggas: vagga_count,
            suttas_total: sutta_count,
            suttas_inserted: inserted,
            suttas_failed: failed,
        })
    }
    
    /// Generate UID for a sutta using CST mapping
    fn generate_sutta_uid(&self, filename: &str, book_id: &str, sutta_index: usize) -> String {
        // Try to build CST code from book_id and sutta index
        // Example: book_id="mn1", sutta_index=1 -> cst_code="mn1.1.1"
        let cst_code = format!("{}.1.{}", book_id, sutta_index);
        
        // Try to get mapped UID
        if let Some(uid) = self.cst_mapping.generate_uid(filename, &cst_code) {
            return uid;
        }
        
        // Fallback to filename-based UID
        CstMapping::generate_fallback_uid(filename, sutta_index)
    }
    
    /// Process a single file in dry-run mode (no database insertion)
    pub fn process_file_dry_run(&self, xml_path: &Path) -> Result<FileImportStats> {
        let filename = xml_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        // Step 1: Read and convert encoding
        let xml_content = read_xml_file(xml_path)?;
        
        // Step 2: Parse XML
        let collection = parse_xml(&xml_content)?;
        
        // Calculate statistics
        let book_count = collection.books.len();
        let vagga_count: usize = collection.books.iter()
            .map(|b| b.vaggas.len())
            .sum();
        let sutta_count: usize = collection.books.iter()
            .flat_map(|b| &b.vaggas)
            .map(|v| v.suttas.len())
            .sum();
        
        Ok(FileImportStats {
            filename,
            nikaya: collection.nikaya,
            books: book_count,
            vaggas: vagga_count,
            suttas_total: sutta_count,
            suttas_inserted: 0,
            suttas_failed: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    
    #[test]
    fn test_dry_run_process() {
        // This test requires the actual XML file to exist
        let xml_path = PathBuf::from("../bootstrap-assets-resources/tipitaka-org-vri-cst/tipitaka-xml/romn/s0201m.mul.xml");
        
        if !xml_path.exists() {
            // Skip test if file doesn't exist
            return;
        }
        
        let tsv_path = PathBuf::from("assets/cst-vs-sc.tsv");
        if !tsv_path.exists() {
            return;
        }
        
        let importer = TipitakaImporter::new(&tsv_path, false).unwrap();
        let stats = importer.process_file_dry_run(&xml_path).unwrap();
        
        assert_eq!(stats.nikaya, "Majjhimanikāyo");
        assert!(stats.suttas_total > 0);
    }
}
