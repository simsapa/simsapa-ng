use anyhow::{Context, Result};
use diesel::sqlite::SqliteConnection;
use std::path::{Path, PathBuf};

use crate::tipitaka_xml_parser::TipitakaImporter;
use simsapa_backend::logger;

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

        // Mapping TSV lives in repo assets/
        let importer = TipitakaImporter::new(self.verbose)
            .context("Failed to initialize TipitakaImporter")?;

        for fname in files.iter() {
            let xml_path = romn_dir.join(fname);
            if !xml_path.exists() {
                logger::warn(&format!("Missing XML file: {:?}", xml_path));
                continue;
            }

            logger::info(&format!("Importing tipitaka.org XML: {}", fname));
            match importer.process_file(&xml_path, Some(conn)) {
                Ok(stats) => {
                    logger::info(&format!(
                        "Imported {} (suttas: {}, inserted: {}, failed: {})",
                        stats.filename, stats.suttas_total, stats.suttas_inserted, stats.suttas_failed
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


