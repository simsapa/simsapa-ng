use anyhow::{Context, Result};
use diesel::sqlite::SqliteConnection;
use std::path::{Path, PathBuf};

use crate::tipitaka_xml_parser::TipitakaImporter;
use crate::logger;

pub struct TipitakaXmlImporter {
    root_dir: PathBuf,
    verbose: bool,
}

impl TipitakaXmlImporter {
    pub fn new(root_dir: PathBuf) -> Self {
        Self { root_dir, verbose: false }
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
        ];

        let romn_dir = self.root_dir.join("romn");
        if !romn_dir.exists() {
            logger::warn(&format!(
                "tipitaka.org romn directory not found: {:?} - skipping",
                romn_dir
            ));
            return Ok(());
        }

        // Mapping TSV lives in repo assets/
        let tsv_path = Path::new("assets/cst-vs-sc.tsv");
        if !tsv_path.exists() {
            anyhow::bail!("CST mapping TSV not found: {:?}", tsv_path);
        }

        let importer = TipitakaImporter::new(tsv_path, self.verbose)
            .context("Failed to initialize TipitakaImporter")?;

        for fname in files.iter() {
            let xml_path = romn_dir.join(fname);
            if !xml_path.exists() {
                logger::warn(&format!("Missing XML file: {:?}", xml_path));
                continue;
            }

            logger::info(&format!("Importing tipitaka.org XML: {}", fname));
            match importer.process_file(&xml_path, conn) {
                Ok(stats) => {
                    logger::info(&format!(
                        "Imported {} (books: {}, vaggas: {}, suttas: {}, inserted: {}, failed: {})",
                        stats.filename, stats.books, stats.vaggas, stats.suttas_total, stats.suttas_inserted, stats.suttas_failed
                    ));
                }
                Err(e) => {
                    logger::error(&format!("Failed importing {}: {}", fname, e));
                }
            }
        }

        Ok(())
    }
}


