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

use crate::bootstrap::helpers::uid_to_ref;
use simsapa_backend::helpers::consistent_niggahita;
use simsapa_backend::logger;

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
pub struct TipitakaImporterUsingTSV {
    cst_mapping: CstMapping,
    verbose: bool,
}

impl TipitakaImporterUsingTSV {
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

        // Normalize for mapping and detect commentary type
        let mapping_filename = normalize_filename_for_mapping(&filename);
        let commentary_suffix = detect_commentary_suffix(&filename);

        if self.verbose {
            logger::info(&format!("  → Reading and converting encoding..."));
        }

        // Step 1: Read and convert encoding
        let s = read_xml_file(xml_path).context("Failed to read XML file")?;
        let xml_content = consistent_niggahita(Some(s));

        if self.verbose {
            logger::info(&format!("  ✓ Encoding conversion successful"));
            logger::info(&format!("  → Parsing XML structure..."));
        }

        // Step 2: Parse XML
        let collection = parse_xml(&xml_content)
            .context("Failed to parse XML")?;

        if self.verbose {
            logger::info(&format!("  ✓ XML parsed successfully"));
            logger::info(&format!("  → Generating UIDs and transforming to HTML..."));
        }

        // Step 3: Get sutta boundaries from TSV
        let boundaries = self.cst_mapping.get_sutta_boundaries(&mapping_filename);

        if boundaries.is_none() {
            logger::warn(&format!("No sutta boundaries found for {}, using parsed structure", filename));
        }

        // Step 4: Process each sutta using TSV boundaries if available
        let mut suttas_to_insert = Vec::new();

        if let Some(boundaries) = boundaries {
            // Use TSV boundaries to correctly identify suttas
            logger::info(&format!("Using TSV boundaries: {} suttas found", boundaries.len()));
            for (sutta_idx, boundary) in boundaries.iter().enumerate() {
                // Collect all paragraphs for this sutta
                let mut sutta_content = Vec::new();
                let sutta_title = boundary.title.clone();

                if self.verbose {
                    logger::info(&format!("  → Processing sutta {}: {} (paranum {})",
                        boundary.sc_code, sutta_title, boundary.start_paranum));
                }

                // Determine the range of paranums for this sutta
                let start_paranum = boundary.start_paranum;
                let end_paranum = if sutta_idx + 1 < boundaries.len() {
                    boundaries[sutta_idx + 1].start_paranum - 1
                } else {
                    i32::MAX
                };

                // Collect all paragraphs in this paranum range from all books/vaggas
                for book in &collection.books {
                    for vagga in &book.vaggas {
                        for parsed_sutta in &vagga.suttas {
                            for element in &parsed_sutta.content_xml {
                                if let super::types::XmlElement::Paragraph { n, .. } = element {
                                    if let Some(n_str) = n {
                                        if let Ok(paranum) = n_str.parse::<i32>() {
                                            if paranum >= start_paranum && paranum <= end_paranum {
                                                sutta_content.push(element.clone());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if sutta_content.is_empty() {
                    logger::warn(&format!("No content found for sutta: {}", sutta_title));
                    continue;
                }

                if self.verbose {
                    logger::info(&format!("    ✓ Collected {} paragraphs (paranums {}-{})",
                        sutta_content.len(), start_paranum, end_paranum));
                }

                // Build group path from TSV boundary data
                let group_path = format!("{}/{}/{}",
                    collection.nikaya,
                    boundary.book,
                    boundary.vagga
                );

                // Extract vagga number from vagga title (e.g., "5. Cūḷayamakavaggo" -> 5)
                let group_index = boundary.vagga
                    .split('.')
                    .next()
                    .and_then(|s| s.trim().parse::<i32>().ok());

                // Transform to HTML
                let html = transform_to_html(&sutta_content)
                    .context("Failed to transform to HTML")?;

                // Extract plain text
                let plain = extract_plain_text(&sutta_content);

                // Create sutta with proper metadata
                let uid_code = match commentary_suffix.as_deref() {
                    Some(".att") => format!("{}.att", boundary.sc_code),
                    Some(".tik") => format!("{}.tik", boundary.sc_code),
                    _ => boundary.sc_code.clone(),
                };
                let uid = format!("{}/pli/cst4", uid_code);
                let sutta_ref = uid_to_ref(&boundary.sc_code);

                let sutta = super::types::Sutta {
                    title: sutta_title,
                    content_xml: sutta_content,
                    metadata: super::types::SuttaMetadata {
                        uid: uid.clone(),
                        sutta_ref,
                        nikaya: collection.nikaya.clone(),
                        group_path,
                        group_index,
                        order_index: Some((sutta_idx + 1) as i32),
                    },
                };

                suttas_to_insert.push((sutta, html, plain));
            }
        } else {
            // Instead of fallback import, emit an error and continue
            logger::error(&format!("No valid TSV boundaries for file {filename}, skipping import of this file"));
            // No suttas_to_insert pushed; produce empty import
            // Early return for this file
            return Ok(FileImportStats {
                filename,
                nikaya: collection.nikaya,
                books: collection.books.len(),
                vaggas: collection.books.iter().map(|b| b.vaggas.len()).sum(),
                suttas_total: 0,
                suttas_inserted: 0,
                suttas_failed: 0,
            });
        }

        if self.verbose {
            logger::info(&format!("  ✓ Prepared {} suttas for insertion", suttas_to_insert.len()));
            logger::info(&format!("  → Inserting into database..."));
        }

        // Step 4: Insert into database
        let inserted = insert_suttas_batch(conn, &suttas_to_insert)
            .context("Failed to insert suttas")?;

        if self.verbose {
            logger::info(&format!("  ✓ Inserted {} suttas", inserted));
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

    /// Generate UID with optional commentary suffix, using mapping filename for code lookup
    fn generate_sutta_uid_with_suffix(
        &self,
        mapping_filename: &str,
        original_filename: &str,
        book_id: &str,
        sutta_index: usize,
        commentary_suffix: Option<&str>,
    ) -> String {
        let cst_code = format!("{}.1.{}", book_id, sutta_index);
        if let Some(base_code) = self.cst_mapping.generate_code(mapping_filename, &cst_code) {
            let code_with_suffix = match commentary_suffix {
                Some(".att") => format!("{}.att", base_code),
                Some(".tik") => format!("{}.tik", base_code),
                _ => base_code,
            };
            return format!("{}/pli/cst4", code_with_suffix);
        }
        // Fallback retains original filename behavior
        CstMapping::generate_fallback_uid(original_filename, sutta_index)
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

/// Detect commentary suffix based on filename extension
fn detect_commentary_suffix(filename: &str) -> Option<String> {
    if filename.ends_with(".att.xml") {
        Some(".att".to_string())
    } else if filename.ends_with(".tik.xml") {
        Some(".tik".to_string())
    } else {
        None
    }
}

/// Normalize filename for CST mapping: map commentary/sub-commentary to corresponding .mul file
fn normalize_filename_for_mapping(filename: &str) -> String {
    if filename.ends_with("a.att.xml") {
        return filename.replace("a.att.xml", "m.mul.xml");
    }
    if filename.ends_with("t.tik.xml") {
        return filename.replace("t.tik.xml", "m.mul.xml");
    }
    filename.to_string()
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

        let importer = TipitakaImporterUsingTSV::new(&tsv_path, false).unwrap();
        let stats = importer.process_file_dry_run(&xml_path).unwrap();

        assert_eq!(stats.nikaya, "Majjhimanikāyo");
        assert!(stats.suttas_total > 0);
    }
}
