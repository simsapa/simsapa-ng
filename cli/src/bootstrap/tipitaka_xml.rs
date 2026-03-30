use anyhow::{Context, Result};
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use std::collections::HashSet;
use std::path::PathBuf;

use crate::bootstrap::helpers::uid_to_ref;
use simsapa_backend::helpers::{consistent_niggahita, sutta_html_to_plain_text, sutta_range_from_ref, pali_to_ascii};
use simsapa_backend::logger;
use html_escape;

use tipitaka_xml_parser::encoding::read_xml_file;
use tipitaka_xml_parser::detect_nikaya_structure;
use tipitaka_xml_parser::sutta_builder::xml_to_html;
use tipitaka_xml_parser::fragments_schema::xml_fragments;
use tipitaka_xml_parser::fragments_models::XmlFragmentRecord;
use tipitaka_xml_parser::types::{GroupLevel, GroupType};

static CHECKED_XML_WHITELIST: [&str; 30] = [
    // Dīgha Nikāya
    "s0101a.att.xml",
    "s0101m.mul.xml",
    "s0101t.tik.xml",
    "s0102a.att.xml",
    "s0102m.mul.xml",
    "s0102t.tik.xml",
    "s0103a.att.xml",
    "s0103m.mul.xml",
    "s0103t.tik.xml",

    // Majjhima Nikāya
    "s0201a.att.xml",
    "s0201m.mul.xml",
    "s0201t.tik.xml",
    "s0202a.att.xml",
    "s0202m.mul.xml",
    "s0202t.tik.xml",
    "s0203a.att.xml",
    "s0203m.mul.xml",
    "s0203t.tik.xml",

    // Saṁyutta Nikāya
    "s0301a.att.xml",
    "s0301m.mul.xml",
    "s0301t.tik.xml",
    "s0302a.att.xml",
    "s0302m.mul.xml",
    "s0302t.tik.xml",
    "s0303a.att.xml",
    "s0303m.mul.xml",
    "s0303t.tik.xml",
    "s0304a.att.xml",
    "s0304m.mul.xml",
    "s0304t.tik.xml",
];

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
    pub title_ascii: Option<String>,
    pub title_pali: Option<String>,
    pub content_plain: Option<String>,
    pub content_html: Option<String>,
    pub source_uid: Option<String>,
}

pub struct TipitakaXmlImporter {
    fragments_db_path: PathBuf,
    romn_dir: PathBuf,
}

impl TipitakaXmlImporter {
    pub fn new(fragments_db_path: PathBuf, romn_dir: PathBuf) -> Self {
        Self {
            fragments_db_path,
            romn_dir,
        }
    }

    /// Phase 1: Import suttas from fragments.sqlite3 for whitelisted files
    fn import_from_fragments(&self) -> Result<Vec<SuttaRecord>> {
        match self.fragments_db_path.try_exists() {
            Ok(true) => {}
            Ok(false) => {
                logger::warn(&format!(
                    "fragments.sqlite3 not found: {:?} - skipping Phase 1",
                    self.fragments_db_path
                ));
                return Ok(Vec::new());
            }
            Err(e) => {
                logger::warn(&format!(
                    "Cannot check fragments.sqlite3 path {:?}: {} - skipping Phase 1",
                    self.fragments_db_path, e
                ));
                return Ok(Vec::new());
            }
        }

        let db_url = format!("file:{}?mode=ro", self.fragments_db_path.display());
        let mut frag_conn = SqliteConnection::establish(&db_url)
            .with_context(|| format!("Failed to open fragments DB: {:?}", self.fragments_db_path))?;

        let mut suttas = Vec::new();
        let mut used_uids = HashSet::new();

        for filename in &CHECKED_XML_WHITELIST {
            let rows: Vec<XmlFragmentRecord> = xml_fragments::table
                .filter(xml_fragments::cst_file.eq(filename))
                .filter(xml_fragments::frag_type.eq("Sutta"))
                .load::<XmlFragmentRecord>(&mut frag_conn)
                .with_context(|| format!("Failed to query fragments for {}", filename))?;

            for (idx, row) in rows.iter().enumerate() {
                // Skip fragments without sc_code
                let sc_code = match &row.sc_code {
                    Some(code) => code.clone(),
                    None => {
                        logger::warn(&format!(
                            "Fragment in {} has no sc_code (cst_sutta: {:?}), skipping",
                            filename, row.cst_sutta
                        ));
                        continue;
                    }
                };

                // Determine commentary suffix from filename
                let uid_code = if filename.ends_with(".att.xml") {
                    format!("{}.att", sc_code)
                } else if filename.ends_with(".tik.xml") {
                    format!("{}.tik", sc_code)
                } else {
                    sc_code.clone()
                };

                let uid = format!("{}/pli/cst", uid_code);

                // Skip duplicate UIDs
                if used_uids.contains(&uid) {
                    logger::warn(&format!("Duplicate UID: {}, skipping", uid));
                    continue;
                }
                used_uids.insert(uid.clone());

                // Title from cst_sutta field
                let title = row.cst_sutta.clone().unwrap_or_default();
                let normalized_title = consistent_niggahita(Some(title.clone()));
                let normalized_title_ascii = pali_to_ascii(Some(&title));

                // Convert content_xml to HTML
                let content_html = xml_to_html(&row.content_xml)
                    .with_context(|| format!("Failed to convert XML to HTML for UID {}", uid))?;

                // Build group path from group_levels JSON
                let group_levels: Vec<GroupLevel> = serde_json::from_str(&row.group_levels)
                    .unwrap_or_default();

                let group_path = group_levels.iter()
                    .filter(|level| !matches!(level.group_type, GroupType::Nikaya | GroupType::Sutta))
                    .map(|level| level.title.clone())
                    .collect::<Vec<_>>()
                    .join(" / ");

                let group_path_opt = if group_path.is_empty() {
                    None
                } else {
                    Some(group_path)
                };

                // Extract nikaya name and sutta number for header
                let nikaya_name = group_levels.first()
                    .map(|level| level.title.clone())
                    .unwrap_or_else(|| row.nikaya.clone());

                let sutta_number = sc_code.chars()
                    .skip_while(|c| c.is_alphabetic())
                    .collect::<String>();

                // Wrap in div with header
                let full_html = consistent_niggahita(Some(format!(
                    "<div class=\"cst\">\n<header>\n<h3>{} {}</h3>\n<h1>{}</h1>\n</header>\n{}</div>",
                    nikaya_name,
                    sutta_number,
                    html_escape::encode_text(&normalized_title),
                    content_html
                )));

                let content_plain = sutta_html_to_plain_text(&content_html);
                let sutta_ref = uid_to_ref(&sc_code);

                suttas.push(SuttaRecord {
                    uid,
                    sutta_ref,
                    nikaya: row.nikaya.clone(),
                    language: "pli".to_string(),
                    group_path: Some(consistent_niggahita(group_path_opt)),
                    group_index: Some(idx as i32),
                    order_index: Some(idx as i32),
                    title: Some(normalized_title.clone()),
                    title_ascii: Some(normalized_title_ascii),
                    title_pali: Some(normalized_title),
                    content_plain: Some(content_plain),
                    content_html: Some(full_html),
                    source_uid: Some("cst".to_string()),
                });
            }
        }

        logger::info(&format!("Phase 1: Built {} sutta records from fragments", suttas.len()));
        Ok(suttas)
    }

    /// Phase 2: Import remaining XML files directly from romn/ directory
    fn import_romn_xml(&self) -> Result<Vec<SuttaRecord>> {
        match self.romn_dir.try_exists() {
            Ok(true) => {}
            Ok(false) => {
                logger::warn(&format!(
                    "romn directory not found: {:?} - skipping Phase 2",
                    self.romn_dir
                ));
                return Ok(Vec::new());
            }
            Err(e) => {
                logger::warn(&format!(
                    "Cannot check romn directory {:?}: {} - skipping Phase 2",
                    self.romn_dir, e
                ));
                return Ok(Vec::new());
            }
        }

        let mut xml_files: Vec<_> = std::fs::read_dir(&self.romn_dir)
            .with_context(|| format!("Failed to read romn directory: {:?}", self.romn_dir))?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                let name = entry.file_name().to_string_lossy().to_string();
                name.ends_with(".xml") && !CHECKED_XML_WHITELIST.contains(&name.as_str())
            })
            .collect();

        xml_files.sort_by_key(|e| e.file_name());

        let mut suttas = Vec::new();
        let mut used_uids = HashSet::new();

        for entry in &xml_files {
            let filename = entry.file_name().to_string_lossy().to_string();
            let xml_path = entry.path();

            let xml_content = match read_xml_file(&xml_path) {
                Ok(content) => content,
                Err(e) => {
                    logger::error(&format!("Failed to read {}: {}", filename, e));
                    continue;
                }
            };

            let nikaya = match detect_nikaya_structure(&xml_content) {
                Ok(ns) => ns.nikaya,
                Err(_) => "unknown".to_string(),
            };

            let content_html = match xml_to_html(&xml_content) {
                Ok(html) => consistent_niggahita(Some(html)),
                Err(e) => {
                    logger::error(&format!("Failed to convert {} to HTML: {}", filename, e));
                    continue;
                }
            };

            let title = filename.trim_end_matches(".xml").to_string();
            let uid = format!("{}/pli/cst", filename);

            if used_uids.contains(&uid) {
                logger::warn(&format!("Duplicate UID: {}, skipping", uid));
                continue;
            }
            used_uids.insert(uid.clone());

            let full_html = format!(
                "<div class=\"cst\">\n<header>\n<h1>{}</h1>\n</header>\n{}</div>",
                html_escape::encode_text(&title),
                content_html
            );

            let content_plain = sutta_html_to_plain_text(&content_html);
            let sutta_ref = uid_to_ref(&title);

            suttas.push(SuttaRecord {
                uid,
                sutta_ref,
                nikaya,
                language: "pli".to_string(),
                group_path: None,
                group_index: None,
                order_index: None,
                title: Some(title.clone()),
                title_ascii: Some(pali_to_ascii(Some(&title))),
                title_pali: Some(title),
                content_plain: Some(content_plain),
                content_html: Some(full_html),
                source_uid: Some("cst".to_string()),
            });
        }

        logger::info(&format!("Phase 2: Built {} sutta records from remaining XML files", suttas.len()));
        Ok(suttas)
    }

    /// Insert sutta records into the appdata database
    fn insert_suttas(&self, sutta_records: Vec<SuttaRecord>, conn: &mut SqliteConnection) -> Result<usize> {
        use simsapa_backend::db::appdata_schema::suttas;
        use simsapa_backend::db::appdata_models::NewSutta;

        let mut inserted_count = 0;

        conn.transaction::<_, anyhow::Error, _>(|conn| {
            for record in &sutta_records {
                let (range_group, range_start, range_end) = if let Some(range) = sutta_range_from_ref(&record.uid) {
                    let start = range.start.map(|s| s as i32);
                    let end = range.end.map(|e| e as i32);
                    (Some(range.group), start, end)
                } else {
                    (None, None, None)
                };

                let new_sutta = NewSutta {
                    uid: &record.uid,
                    sutta_ref: &record.sutta_ref,
                    nikaya: &record.nikaya,
                    language: &record.language,
                    group_path: record.group_path.as_deref(),
                    group_index: record.group_index,
                    order_index: record.order_index,
                    sutta_range_group: range_group.as_deref(),
                    sutta_range_start: range_start,
                    sutta_range_end: range_end,
                    title: record.title.as_deref(),
                    title_ascii: record.title_ascii.as_deref(),
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
                    logger::warn(&format!("UID already exists, skipping: {}", &record.uid));
                    continue;
                }

                diesel::insert_into(suttas::table)
                    .values(&new_sutta)
                    .execute(conn)
                    .with_context(|| format!("Failed to insert sutta: {}", record.uid))?;

                inserted_count += 1;
            }

            Ok(inserted_count)
        })
    }

    /// Main entry point: import from both phases
    pub fn import(&mut self, conn: &mut SqliteConnection) -> Result<()> {
        // Phase 1: Fragment-based import for whitelisted files
        let phase1_suttas = self.import_from_fragments()?;
        let phase1_count = phase1_suttas.len();
        let phase1_inserted = self.insert_suttas(phase1_suttas, conn)?;
        logger::info(&format!(
            "Imported from fragments.sqlite3: {} processed, {} inserted, {} skipped",
            phase1_count, phase1_inserted, phase1_count - phase1_inserted
        ));

        // Phase 2: Direct XML import for remaining files
        let phase2_suttas = self.import_romn_xml()?;
        let phase2_count = phase2_suttas.len();
        let phase2_inserted = self.insert_suttas(phase2_suttas, conn)?;
        logger::info(&format!(
            "Imported xml from romn/ folder: {} processed, {} inserted, {} skipped",
            phase2_count, phase2_inserted, phase2_count - phase2_inserted
        ));

        logger::info(&format!(
            "Tipitaka XML import complete: {} total inserted",
            phase1_inserted + phase2_inserted
        ));

        Ok(())
    }
}
