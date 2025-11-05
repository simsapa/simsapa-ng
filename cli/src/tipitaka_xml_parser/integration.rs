//! High-level integration API
//!
//! This module provides the high-level API for processing XML files
//! and directories with the fragment-based parser.

use std::path::Path;
use anyhow::{Context, Result};
use diesel::sqlite::SqliteConnection;

use simsapa_backend::logger;

use super::encoding::read_xml_file;
use super::{
    detect_nikaya_structure,
    parse_into_fragments,
    build_suttas,
    insert_suttas,
};
use super::types::FragmentAdjustments;

/// Statistics for a single file import
#[derive(Debug, Clone, Default)]
pub struct FileImportStats {
    pub filename: String,
    pub nikaya: String,
    pub fragments_parsed: usize,
    pub suttas_total: usize,
    pub suttas_inserted: usize,
    pub suttas_failed: usize,
}

/// Statistics from processing operations
#[derive(Debug, Clone, Default)]
pub struct ProcessingStats {
    /// Number of files processed
    pub files_processed: usize,
    /// Number of suttas inserted into database
    pub suttas_inserted: usize,
    /// Number of errors encountered
    pub errors: usize,
}

/// Complete import process for Tipitaka XML files using fragment-based parser
pub struct TipitakaImporter {
    adjustments: Option<FragmentAdjustments>,
    verbose: bool,
}

impl TipitakaImporter {
    /// Create a new importer
    ///
    /// # Arguments
    /// * `verbose` - Whether to output verbose logging
    ///
    /// # Returns
    /// New TipitakaImporter instance
    pub fn new(verbose: bool) -> Result<Self> {
        Ok(Self {
            adjustments: None,
            verbose,
        })
    }

    /// Set fragment adjustments for the importer
    pub fn with_adjustments(mut self, adjustments: FragmentAdjustments) -> Self {
        self.adjustments = Some(adjustments);
        self
    }

    /// Process a single XML file
    ///
    /// # Arguments
    /// * `xml_path` - Path to the XML file to process
    /// * `conn` - Optional database connection for inserting suttas (None for dry-run)
    ///
    /// # Returns
    /// Import statistics or error if processing fails
    pub fn process_file(
        &self,
        xml_path: &Path,
        conn: Option<&mut SqliteConnection>,
    ) -> Result<FileImportStats> {
        let filename = xml_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let dry_run = conn.is_none();

        if self.verbose {
            logger::info("  → Reading XML file...");
        }

        // Step 1: Read XML file
        let xml_content = read_xml_file(xml_path)
            .context("Failed to read XML file")?;

        if self.verbose {
            logger::info("  ✓ File read successfully");
            logger::info("  → Detecting nikaya structure...");
        }

        // Step 2: Detect nikaya structure
        let nikaya_structure = detect_nikaya_structure(&xml_content)
            .context("Failed to detect nikaya structure")?;

        if self.verbose {
            logger::info(&format!("  ✓ Detected nikaya: {} ({} levels)",
                     nikaya_structure.nikaya, nikaya_structure.levels.len()));
            logger::info("  → Parsing into fragments...");
        }

        // Step 3: Parse into fragments (with SC field population from embedded TSV)
        let fragments = parse_into_fragments(
            &xml_content,
            &nikaya_structure,
            &filename,
            self.adjustments.as_ref(),
            true  // Populate SC fields from embedded TSV
        ).context("Failed to parse fragments")?;

        if self.verbose {
            let sc_count = fragments.iter()
                .filter(|f| f.sc_code.is_some())
                .count();
            logger::info(&format!("  ✓ Parsed {} fragments ({} with SC fields)", fragments.len(), sc_count));
            logger::info("  → Building sutta records...");
        }

        // Step 4: Build suttas from fragments
        let suttas = build_suttas(fragments.clone(), &nikaya_structure)
            .context("Failed to build suttas")?;

        if self.verbose {
            logger::info(&format!("  ✓ Built {} sutta records", suttas.len()));
            if !dry_run {
                logger::info("  → Inserting into database...");
            }
        }

        let fragments_parsed = fragments.len();
        let suttas_total = suttas.len();

        // Step 5: Insert suttas into database (if not dry-run)
        let inserted = if let Some(conn) = conn {
            let count = self.insert_suttas_with_conn(suttas, conn)
                .context("Failed to insert suttas")?;
            
            if self.verbose {
                logger::info(&format!("  ✓ Inserted {} suttas", count));
            }
            
            count
        } else {
            0
        };

        let failed = if dry_run { 0 } else { suttas_total - inserted };

        Ok(FileImportStats {
            filename,
            nikaya: nikaya_structure.nikaya,
            fragments_parsed,
            suttas_total,
            suttas_inserted: inserted,
            suttas_failed: failed,
        })
    }

    /// Export fragments from an XML file to a fragments database
    ///
    /// # Arguments
    /// * `xml_path` - Path to the XML file to process
    /// * `fragments_db_path` - Path to the fragments database
    ///
    /// # Returns
    /// Number of fragments exported or error if export fails
    pub fn export_fragments(&self, xml_path: &Path, fragments_db_path: &Path) -> Result<usize> {
        use super::export_fragments_to_db;

        let filename = xml_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Read and parse XML
        let xml_content = read_xml_file(xml_path)?;
        let nikaya_structure = detect_nikaya_structure(&xml_content)?;
        
        // Parse into fragments
        let fragments = parse_into_fragments(
            &xml_content,
            &nikaya_structure,
            &filename,
            self.adjustments.as_ref(),
            true
        )?;

        // Export to fragments database
        export_fragments_to_db(&fragments, &nikaya_structure, fragments_db_path)
    }

    /// Insert suttas using an existing connection
    fn insert_suttas_with_conn(
        &self,
        sutta_records: Vec<super::sutta_builder::SuttaRecord>,
        conn: &mut SqliteConnection,
    ) -> Result<usize> {
        use diesel::prelude::*;
        use simsapa_backend::db::appdata_schema::suttas;
        use simsapa_backend::db::appdata_models::NewSutta;

        let mut inserted_count = 0;

        // Use a transaction for batch insertion
        conn.transaction::<_, anyhow::Error, _>(|conn| {
            for record in &sutta_records {
                // Convert SuttaRecord to NewSutta
                let new_sutta = NewSutta {
                    uid: &record.uid,
                    sutta_ref: &record.sutta_ref,
                    nikaya: &record.nikaya,
                    language: &record.language,
                    group_path: record.group_path.as_deref(),
                    group_index: record.group_index,
                    order_index: record.order_index,
                    sutta_range_group: None,
                    sutta_range_start: None,
                    sutta_range_end: None,
                    title: record.title.as_deref(),
                    title_ascii: None,
                    title_pali: record.title_pali.as_deref(),
                    title_trans: None,
                    description: None,
                    content_plain: record.content_plain.as_deref(),
                    content_html: record.content_html.as_deref(),
                    content_json: None,
                    content_json_tmpl: None,
                    source_uid: record.source_uid.as_deref(),
                    source_info: None,
                    source_language: Some("pli"),
                    message: None,
                    copyright: None,
                    license: None,
                };

                // Check if UID already exists
                let exists: bool = suttas::table
                    .filter(suttas::uid.eq(&record.uid))
                    .count()
                    .get_result::<i64>(conn)
                    .map(|c| c > 0)
                    .unwrap_or(false);

                if exists {
                    if self.verbose {
                        logger::error(&format!("    Skipping duplicate UID: {}", record.uid));
                    }
                    continue;
                }

                // Insert the sutta
                diesel::insert_into(suttas::table)
                    .values(&new_sutta)
                    .execute(conn)
                    .with_context(|| format!("Failed to insert sutta: {}", record.uid))?;

                inserted_count += 1;
            }

            Ok(inserted_count)
        })
    }
}

/// Process a single XML file (convenience function)
///
/// # Arguments
/// * `xml_path` - Path to the XML file
/// * `db_path` - Path to the database file
/// * `verbose` - Whether to output verbose logging
///
/// # Returns
/// Processing statistics or error if processing fails
pub fn process_xml_file(
    xml_path: &Path,
    db_path: &Path,
    verbose: bool,
) -> Result<ProcessingStats> {
    // TODO: Implement convenience wrapper
    Ok(ProcessingStats::default())
}

/// Process all XML files in a directory (convenience function)
///
/// # Arguments
/// * `dir_path` - Path to the directory containing XML files
/// * `db_path` - Path to the database file
/// * `verbose` - Whether to output verbose logging
///
/// # Returns
/// Aggregated processing statistics or error if processing fails
pub fn process_directory(
    dir_path: &Path,
    db_path: &Path,
    verbose: bool,
) -> Result<ProcessingStats> {
    // TODO: Implement convenience wrapper
    Ok(ProcessingStats::default())
}
