use anyhow::{Context, Result};
use diesel::sqlite::SqliteConnection;
use std::path::{Path, PathBuf};

use crate::bootstrap::helpers::uid_to_ref;
use simsapa_backend::helpers::{consistent_niggahita, compact_rich_text};
use simsapa_backend::logger;
use html_escape;

use tipitaka_xml_parser::encoding::read_xml_file;
use tipitaka_xml_parser::integration::FileImportStats;
use tipitaka_xml_parser::types::{FragmentAdjustments, XmlFragment, FragmentType, GroupType};
use tipitaka_xml_parser::nikaya_structure::NikayaStructure;
use tipitaka_xml_parser::sutta_builder::{load_tsv_mapping, find_code_for_sutta, xml_to_html};
use tipitaka_xml_parser::{
    load_fragment_adjustments,
    detect_nikaya_structure,
    parse_into_fragments,
};

/// Sutta record matching appdata schema
#[derive(Debug, Clone)]
pub struct SuttaRecord {
    pub uid: String,
    pub sutta_ref: String,
    pub nikaya: String,
    pub language: String,
    pub group_path: Option<String>,
    pub group_index: Option<i32>,
    pub order_index: Option<i32>,
    pub title: Option<String>,
    pub title_pali: Option<String>,
    pub content_plain: Option<String>,
    pub content_html: Option<String>,
    pub source_uid: Option<String>,
}

pub struct TipitakaXmlImporter {
    root_dir: PathBuf,
    verbose: bool,
}

impl TipitakaXmlImporter {
    pub fn new(root_dir: PathBuf) -> Self {
        Self {
            root_dir,
            verbose: false,
        }
    }

    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    pub fn import(&mut self, conn: &mut SqliteConnection) -> Result<()> {
        // Limit to explicit list of files for initial bootstrap
        let files = [
            // Dīgha Nikāya
            "s0101m.mul.xml",
            "s0102m.mul.xml",
            "s0103m.mul.xml",
            "s0101a.att.xml",
            "s0102a.att.xml",
            "s0103a.att.xml",
            "s0101t.tik.xml",
            "s0102t.tik.xml",
            "s0103t.tik.xml",

            // Majjhima Nikāya
            "s0201m.mul.xml",
            "s0202m.mul.xml",
            "s0203m.mul.xml",
            "s0201a.att.xml",
            "s0202a.att.xml",
            "s0203a.att.xml",
            "s0201t.tik.xml",
            "s0202t.tik.xml",
            "s0203t.tik.xml",

            // Saṁyutta Nikāya
            "s0301m.mul.xml",
            "s0302m.mul.xml",
            "s0303m.mul.xml",
            "s0304m.mul.xml",
            "s0305m.mul.xml",
            "s0301a.att.xml",
            "s0302a.att.xml",
            "s0303a.att.xml",
            "s0304a.att.xml",
            "s0305a.att.xml",
            "s0301t.tik.xml",
            "s0302t.tik.xml",
            "s0303t.tik.xml",
            "s0304t.tik.xml",
            "s0305t.tik.xml",

            // Aṅguttara Nikāya
            // "s0401m.mul.xml",
            // "s0402m1.mul.xml",
            // "s0402m2.mul.xml",
            // "s0402m3.mul.xml",
            // "s0403m1.mul.xml",
            // "s0403m2.mul.xml",
            // "s0403m3.mul.xml",
            // "s0404m1.mul.xml",
            // "s0404m2.mul.xml",
            // "s0404m3.mul.xml",
            // "s0404m4.mul.xml",
            // "s0401a.att.xml",
            // "s0402a.att.xml",
            // "s0402a.att.xml",
            // "s0402a.att.xml",
            // "s0403a.att.xml",
            // "s0403a.att.xml",
            // "s0403a.att.xml",
            // "s0404a.att.xml",
            // "s0404a.att.xml",
            // "s0404a.att.xml",
            // "s0404a.att.xml",
            // "s0401t.tik.xml",
            // "s0402t.tik.xml",
            // "s0402t.tik.xml",
            // "s0402t.tik.xml",
            // "s0403t.tik.xml",
            // "s0403t.tik.xml",
            // "s0403t.tik.xml",
            // "s0404t.tik.xml",
            // "s0404t.tik.xml",
            // "s0404t.tik.xml",
            // "s0404t.tik.xml",
        ];

        let romn_dir = self.root_dir.join("romn");
        if !romn_dir.exists() {
            logger::warn(&format!(
                "tipitaka.org romn directory not found: {:?} - skipping",
                romn_dir
            ));
            return Ok(());
        }

        let adjustments = match load_fragment_adjustments() {
            Ok(adj) => {
                logger::info(&format!("Loaded {} fragment adjustments\n", adj.len()));
                Some(adj)
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to load fragment adjustments: {}", e));
            }
        };

        for fname in files.iter() {
            let xml_path = romn_dir.join(fname);
            if !xml_path.exists() {
                logger::warn(&format!("Missing XML file: {:?}", xml_path));
                continue;
            }

            logger::info(&format!("Importing tipitaka.org XML: {}", fname));
            match self.process_file(&xml_path, adjustments.as_ref(), Some(conn)) {
                Ok(stats) => {
                    logger::info(&format!(
                        "Imported {} (suttas: {}, inserted: {}, failed: {})",
                        stats.filename, stats.suttas_total, stats.suttas_inserted, stats.suttas_failed
                    ));
                }
                Err(e) => {
                    // FIXME should be a failing error
                    logger::error(&format!("Failed importing {}: {}", fname, e));
                }
            }
        }

        Ok(())
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
        adjustments: Option<&FragmentAdjustments>,
        conn: Option<&mut SqliteConnection>,
    ) -> Result<FileImportStats> {
        let filename = xml_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let dry_run = conn.is_none();

        if self.verbose {
            logger::info("Reading XML file...");
        }

        // Step 1: Read XML file
        let xml_content = read_xml_file(xml_path)
            .context("Failed to read XML file")?;

        if self.verbose {
            logger::info("File read successfully");
            logger::info("Detecting nikaya structure...");
        }

        // Step 2: Detect nikaya structure
        let nikaya_structure = detect_nikaya_structure(&xml_content)
            .context("Failed to detect nikaya structure")?;

        if self.verbose {
            logger::info(&format!("Detected nikaya: {} ({} levels)",
                     nikaya_structure.nikaya, nikaya_structure.levels.len()));
            logger::info("Parsing into fragments...");
        }

        // Step 3: Parse into fragments (with SC field population from embedded TSV)
        let fragments = parse_into_fragments(
            &xml_content,
            &nikaya_structure,
            &filename,
            adjustments,
            true  // Populate SC fields from embedded TSV
        ).context("Failed to parse fragments")?;

        if self.verbose {
            let sc_count = fragments.iter()
                .filter(|f| f.sc_code.is_some())
                .count();
            logger::info(&format!("Parsed {} fragments ({} with SC fields)", fragments.len(), sc_count));
            logger::info("Building sutta records...");
        }

        // Step 4: Build suttas from fragments
        let suttas = build_suttas(fragments.clone(), &nikaya_structure)
            .context("Failed to build suttas")?;

        if self.verbose {
            logger::info(&format!("Built {} sutta records", suttas.len()));
            if !dry_run {
                logger::info("Inserting into database...");
            }
        }

        let fragments_parsed = fragments.len();
        let suttas_total = suttas.len();

        // Step 5: Insert suttas into database (if not dry-run)
        let inserted = if let Some(conn) = conn {
            let count = self.insert_suttas_with_conn(suttas, conn)
                .context("Failed to insert suttas")?;

            if self.verbose {
                logger::info(&format!("Inserted {} suttas", count));
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


    /// Insert suttas using an existing connection
    fn insert_suttas_with_conn(
        &self,
        sutta_records: Vec<SuttaRecord>,
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
                        logger::error(&format!("Skipping duplicate UID: {}", record.uid));
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

/// Build sutta database records from fragments
///
/// Uses derived CST fields (cst_code, cst_sutta, etc.) from fragments when available.
/// Falls back to TSV lookup for legacy compatibility if derived fields are not present.
///
/// # Arguments
/// * `fragments` - Vector of parsed fragments with derived CST metadata
/// * `nikaya_structure` - The structure configuration for this nikaya
/// * `tsv_path` - Path to cst-vs-sc.tsv mapping file (used for legacy fallback)
///
/// # Returns
/// Vector of sutta records or error if assembly fails
pub fn build_suttas(
    fragments: Vec<XmlFragment>,
    nikaya_structure: &NikayaStructure,
) -> Result<Vec<SuttaRecord>> {
    // Load TSV mapping from embedded data
    let tsv_records = load_tsv_mapping()
        .context("Failed to load TSV mapping")?;

    let mut suttas = Vec::new();
    let mut used_codes = std::collections::HashSet::new();

    // Group sutta fragments
    let sutta_fragments: Vec<&XmlFragment> = fragments.iter()
        .filter(|f| matches!(f.frag_type, FragmentType::Sutta))
        .collect();

    for (idx, fragment) in sutta_fragments.iter().enumerate() {
        // Get sutta title - prefer cst_sutta from fragment if available
        let title = if let Some(ref cst_sutta) = fragment.cst_sutta {
            cst_sutta.clone()
        } else {
            // Fall back to extracting from group_levels
            let sutta_level = fragment.group_levels.iter()
                .find(|level| matches!(level.group_type, GroupType::Sutta));

            if let Some(level) = sutta_level {
                level.title.clone()
            } else {
                // No sutta title in group_levels - this fragment is a subsection heading
                // (e.g., "<p rend="subhead">Uddeso</p>" meaning "Summary") that was treated
                // as a fragment boundary during parsing but is not actually a separate sutta.
                // The content is preserved in the previous sutta fragment, so we skip this.
                continue;
            }
        };

        // Normalize title
        let normalized_title = consistent_niggahita(Some(title.clone()));

        // Extract nikaya name from first level
        let nikaya_name = fragment.group_levels.first()
            .map(|level| level.title.clone())
            .unwrap_or_else(|| nikaya_structure.nikaya.clone());

        // Build group path from hierarchy (excluding nikaya and sutta levels)
        let group_path = fragment.group_levels.iter()
            .filter(|level| !matches!(level.group_type, GroupType::Nikaya | GroupType::Sutta))
            .map(|level| level.title.clone())
            .collect::<Vec<_>>()
            .join(" / ");

        let group_path_opt = if group_path.is_empty() {
            None
        } else {
            Some(group_path)
        };

        // Get XML filename from fragment
        let cst_file = &fragment.cst_file;

        // Determine if this is a commentary or subcommentary
        let is_commentary = cst_file.ends_with(".att.xml");
        let is_subcommentary = cst_file.ends_with(".tik.xml");
        let is_commentary_or_sub = is_commentary || is_subcommentary;

        // Extract vagga from group_levels (if present)
        // For MN/SN, vagga structure typically aligns between base text and commentary
        // For DN, commentary doesn't have vagga structure (uses chapter=sutta directly)
        let vagga_title = fragment.group_levels.iter()
            .find(|level| matches!(level.group_type, GroupType::Vagga))
            .map(|level| level.title.as_str());

        // Get SC code - this is the primary identifier for suttas
        // Priority: sc_code > cst_code > TSV lookup
        let code = if let Some(ref sc_code) = fragment.sc_code {
            // Use the sc_code from TSV mapping (preferred)
            sc_code.clone()
        } else if let Some(ref cst_code) = fragment.cst_code {
            // Fall back to derived cst_code
            // Try to look it up in TSV to get the sc_code
            let sc_code_from_tsv = tsv_records.iter()
                .find(|r| r.tsv_cst_code == *cst_code)
                .map(|r| r.tsv_sc_code.clone());

            if let Some(sc) = sc_code_from_tsv {
                sc
            } else {
                // Use cst_code as fallback
                cst_code.clone()
            }
        } else {
            // Fall back to TSV lookup (legacy path)
            match find_code_for_sutta(&tsv_records, &cst_file, &title, vagga_title, is_commentary_or_sub, &used_codes) {
                Some(c) => c,
                None => {
                    // Log warning - could not find matching code
                    logger::warn(&format!("Could not find code for sutta '{}' in file '{}', skipping",
                             title, cst_file));
                    continue;
                }
            }
        };

        // Check if we've already used this code
        if used_codes.contains(&code) {
            logger::error(&format!("Code '{}' already used for a previous sutta, skipping duplicate for '{}' (file: {})",
                     code, title, cst_file));
            continue;
        }
        used_codes.insert(code.clone());

        // Add commentary/subcommentary suffix to code
        let uid_code = if is_commentary {
            format!("{}.att", code)
        } else if is_subcommentary {
            format!("{}.tik", code)
        } else {
            code.clone()
        };

        // Build full UID
        let uid = format!("{}/pli/cst4", uid_code);

        // Generate sutta reference
        let sutta_ref = uid_to_ref(&code);

        // Extract sutta number from code (e.g., "dn1" -> "1")
        let sutta_number = code.chars()
            .skip_while(|c| c.is_alphabetic())
            .collect::<String>();

        // Transform XML content to HTML
        let content_html = xml_to_html(&fragment.content_xml)
            .context("Failed to transform XML to HTML")?;

        // Build HTML with header
        let normalized_full_html = consistent_niggahita(Some(format!(
            "<div class=\"cst4\">\n<header>\n<h3>{} {}</h3>\n<h1>{}</h1>\n</header>\n{}</div>",
            nikaya_name,
            sutta_number,
            html_escape::encode_text(&normalized_title),
            content_html
        )));

        // Extract plain text
        let normalized_content_plain = compact_rich_text(&content_html);

        // Build sutta record
        let sutta = SuttaRecord {
            uid,
            sutta_ref,
            nikaya: nikaya_structure.nikaya.clone(),
            language: "pli".to_string(),
            group_path: Some(consistent_niggahita(group_path_opt)),
            group_index: Some(idx as i32),
            order_index: Some(idx as i32),
            title: Some(normalized_title.clone()),
            title_pali: Some(normalized_title),
            content_plain: Some(normalized_content_plain),
            content_html: Some(normalized_full_html),
            source_uid: Some("cst4".to_string()),
        };

        suttas.push(sutta);
    }

    Ok(suttas)
}
