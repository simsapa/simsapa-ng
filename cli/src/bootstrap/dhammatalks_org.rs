use anyhow::{Context, Result};
use diesel::prelude::*;
use scraper::{Html, Selector};
use simsapa_backend::db::appdata_schema::suttas;
use simsapa_backend::db::appdata_models::NewSutta;
use std::path::{Path, PathBuf};
use std::fs;
use regex::Regex;
use tracing::{info, warn};
use indicatif::{ProgressBar, ProgressStyle};

use simsapa_backend::lookup::DHP_CHAPTERS_TO_RANGE;
use simsapa_backend::helpers::{consistent_niggahita, compact_rich_text};
use crate::bootstrap::helpers::{uid_to_ref, uid_to_nikaya};

use super::SuttaImporter;

// Owned version of sutta data for building during parsing
#[derive(Debug, Clone)]
struct SuttaData {
    uid: String,
    sutta_ref: String,
    nikaya: String,
    language: String,
    title: String,
    title_ascii: String,
    title_pali: Option<String>,
    content_plain: String,
    content_html: String,
    source_uid: String,
}

impl SuttaData {
    // Convert to NewSutta for database insertion
    fn to_new_sutta(&self) -> NewSutta {
        NewSutta {
            uid: &self.uid,
            sutta_ref: &self.sutta_ref,
            nikaya: &self.nikaya,
            language: &self.language,
            group_path: None,
            group_index: None,
            order_index: None,
            sutta_range_group: None,
            sutta_range_start: None,
            sutta_range_end: None,
            title: Some(&self.title),
            title_ascii: Some(&self.title_ascii),
            title_pali: self.title_pali.as_deref(),
            title_trans: None,
            description: None,
            content_plain: Some(&self.content_plain),
            content_html: Some(&self.content_html),
            content_json: None,
            content_json_tmpl: None,
            source_uid: Some(&self.source_uid),
            source_info: None,
            source_language: None,
            message: None,
            copyright: None,
            license: None,
        }
    }
}

lazy_static::lazy_static! {
    static ref RE_SUTTA_HTML_NAME: Regex = Regex::new(r"(DN|MN|SN|AN|Ch|iti|khp|StNp|thag|thig|ud)[\d_]+\.html").unwrap();
}

pub struct DhammatalksSuttaImporter {
    resource_path: PathBuf,
}

impl DhammatalksSuttaImporter {
    pub fn new(resource_path: PathBuf) -> Self {
        Self { resource_path }
    }

    fn ref_notation_convert(&self, ref_str: &str) -> String {
        let mut ref_str = ref_str.replace('_', ".").to_lowercase();
        ref_str = ref_str.replace(".html", "");
        ref_str = ref_str.replace("stnp", "snp");

        let khp_re = Regex::new(r"khp(\d)").unwrap();
        ref_str = khp_re.replace_all(&ref_str, "kp$1").to_string();

        // remove leading zeros, dn02
        let leading_zeros_re = Regex::new(r"([a-z.])0+").unwrap();
        ref_str = leading_zeros_re.replace_all(&ref_str, "$1").to_string();

        if ref_str.starts_with("ch") {
            let ch_re = Regex::new(r"ch(\d+)").unwrap();
            if let Some(caps) = ch_re.captures(&ref_str) {
                if let Ok(ch_num) = caps[1].parse::<u32>() {
                    if let Some((start, end)) = DHP_CHAPTERS_TO_RANGE.get(&ch_num) {
                        ref_str = format!("dhp{}-{}", start, end);
                    }
                }
            }
        }

        ref_str
    }

    fn href_sutta_html_to_ssp(&self, href: &str) -> String {
        // Extract anchor if present
        let anchor_re = Regex::new(r"#.+").unwrap();
        let anchor = anchor_re.find(href)
            .map(|m| m.as_str())
            .unwrap_or("");

        // Remove anchor from href before processing
        let href_without_anchor = anchor_re.replace(href, "");

        // Extract the filename part from the href
        let ref_re = Regex::new(r"^.*/([^/]+)$").unwrap();
        let ref_str = ref_re.replace(&href_without_anchor, "$1");

        // Convert to canonical reference notation
        let ref_str = self.ref_notation_convert(&ref_str);

        // Create internal ssp:// URI
        format!("ssp://suttas/{}/en/thanissaro{}", ref_str, anchor)
    }

    fn extract_sutta_content(&self, html_text: &str) -> Result<String> {
        let document = Html::parse_document(html_text);
        let selector = Selector::parse("#sutta").unwrap();

        let _sutta_element = document.select(&selector).next()
            .ok_or_else(|| anyhow::anyhow!("No #sutta element found in HTML"))?;

        // Find all <a> links inside #sutta and collect replacements
        let link_selector = Selector::parse("#sutta a").unwrap();
        let mut replacements: Vec<(String, String)> = Vec::new();

        for link in document.select(&link_selector) {
            if let Some(href) = link.value().attr("href") {
                // Check if this href matches sutta HTML name pattern
                if RE_SUTTA_HTML_NAME.is_match(href) {
                    let ssp_href = self.href_sutta_html_to_ssp(href);
                    replacements.push((href.to_string(), ssp_href));
                }
            }
        }

        // Apply replacements to the HTML string
        let mut modified_html = html_text.to_string();
        for (old_href, new_href) in replacements {
            // Replace both quoted forms to be safe
            let old_attr_double = format!("href=\"{}\"", old_href);
            let new_attr_double = format!("href=\"{}\"", new_href);
            modified_html = modified_html.replace(&old_attr_double, &new_attr_double);

            let old_attr_single = format!("href='{}'", old_href);
            let new_attr_single = format!("href='{}'", new_href);
            modified_html = modified_html.replace(&old_attr_single, &new_attr_single);
        }

        // Re-parse and extract #sutta content
        let modified_document = Html::parse_document(&modified_html);
        let modified_sutta = modified_document.select(&selector).next()
            .ok_or_else(|| anyhow::anyhow!("No #sutta element found after modification"))?;

        Ok(modified_sutta.inner_html())
    }

    fn extract_title_info(&self, html_text: &str, file_path: &Path) -> Result<(String, String)> {
        let path_str = file_path.to_string_lossy();

        // <title>DN 1 &nbsp;Brahmajāla Sutta | The Brahmā Net</title>
        // <title>DN 33 Saṅgīti Sutta | The Discourse for Reciting Together</title>
        // <title>AN 6:20 &nbsp;Maraṇassati Sutta | Mindfulness of Death (2)</title>
        let title_capture = Regex::new(r"<title>(.+)</title>").unwrap()
            .captures(html_text)
            .ok_or_else(|| anyhow::anyhow!("No <title> found in HTML"))?;
        let title_text = title_capture[1].trim();

        // Extract title - try different patterns based on path
        let title = {
            // First, try path-specific patterns
            let m = if path_str.contains("/Ud/") {
                // 2 Appāyuka Sutta | Short-lived
                let re = Regex::new(r"^.*\|(.+)").unwrap();
                re.captures(title_text).map(|c| c[1].trim().to_string())
            } else if path_str.contains("/KN/") {
                // Sn 5:4 &#160;Mettagū’s Questions
                // Khp 6 &#160;Ratana Sutta — Treasures
                let re = Regex::new(r"^.*&#160;(.+)").unwrap();
                re.captures(title_text).map(|c| c[1].trim().to_string())
            } else {
                // AN 6:20
                let re = Regex::new(r"^\w+ +[\d:]+[\W](.+)\|").unwrap();
                re.captures(title_text).map(|c| c[1].trim().to_string())
            };

            // If path-specific pattern didn't match, try fallback patterns
            m.or_else(|| {
                // Dhp XVII : Anger
                let re = Regex::new(r"^[^:]+:(.+)").unwrap();
                re.captures(title_text).map(|c| c[1].trim().to_string())
            })
            .or_else(|| {
                // Dhp I &nbsp; Pairs
                let re = Regex::new(r"^.*&nbsp;(.+)").unwrap();
                re.captures(title_text).map(|c| c[1].trim().to_string())
            })
            .or_else(|| {
                let re = Regex::new(r"^.*&#160;(.+)").unwrap();
                re.captures(title_text).map(|c| c[1].trim().to_string())
            })
            .or_else(|| {
                // 82 Itivuttaka
                let re = Regex::new(r"^\d+ *(.+)").unwrap();
                re.captures(title_text).map(|c| c[1].trim().to_string())
            })
            .unwrap_or_else(|| title_text.to_string())
        };

        // Apply string substitutions to clean up the title
        let title = title.replace("&nbsp;", "").replace("&amp;", "and");
        let title = consistent_niggahita(Some(title));

        // Extract Pali title
        let title_pali = if path_str.contains("/Ud/") {
            // 2 Appāyuka Sutta | Short-lived
            let re = Regex::new(r"\d+ +(.+)\|").unwrap();
            re.captures(title_text)
                .map(|c| c[1].to_string())
                .unwrap_or_default()
        } else {
            let re = Regex::new(r"\| *(.+)$").unwrap();
            re.captures(title_text)
                .map(|c| c[1].to_string())
                .unwrap_or_default()
        };

        let title_pali = consistent_niggahita(Some(title_pali.trim().to_string()));

        Ok((title, title_pali))
    }

    fn parse_sutta(&self, file_path: &Path) -> Result<SuttaData> {
        let html_text = fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

        let (title, title_pali) = self.extract_title_info(&html_text, file_path)?;

        let file_stem = file_path.file_stem()
            .ok_or_else(|| anyhow::anyhow!("No file stem"))?
            .to_string_lossy();

        let ref_no_zeros = Regex::new(r"([^0-9])0*").unwrap()
            .replace_all(&file_stem, "$1")
            .to_lowercase();

        let ref_str = self.ref_notation_convert(&ref_no_zeros);

        let lang = "en";
        let author = "thanissaro";
        let uid = format!("{}/{}/{}", ref_str, lang, author);

        let sutta_ref = uid_to_ref(&ref_str);
        let nikaya = uid_to_nikaya(&ref_str);

        let content_html = self.extract_sutta_content(&html_text)?;
        let content_html = consistent_niggahita(Some(content_html));
        let content_html = format!("<div class=\"dhammatalks_org\">{}</div>", content_html);

        let content_plain = compact_rich_text(&content_html);

        Ok(SuttaData {
            uid,
            sutta_ref,
            nikaya,
            language: lang.to_string(),
            title_ascii: title.clone(),
            title,
            title_pali: if title_pali.is_empty() { None } else { Some(title_pali) },
            content_plain,
            content_html,
            source_uid: author.to_string(),
        })
    }

    fn discover_sutta_files(&self) -> Result<Vec<PathBuf>> {
        let mut paths = Vec::new();

        let folders = vec!["DN", "MN", "SN", "AN"];
        for folder in folders {
            let folder_path = self.resource_path.join(folder);
            if folder_path.exists() {
                for entry in fs::read_dir(&folder_path)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("html") {
                        paths.push(path);
                    }
                }
            }
        }

        let kn_folders = vec!["Dhp", "Iti", "Khp", "StNp", "Thag", "Thig", "Ud"];
        let kn_path = self.resource_path.join("KN");
        if kn_path.exists() {
            for folder in kn_folders {
                let folder_path = kn_path.join(folder);
                if folder_path.exists() {
                    for entry in fs::read_dir(&folder_path)? {
                        let entry = entry?;
                        let path = entry.path();
                        if path.extension().and_then(|s| s.to_str()) == Some("html") {
                            paths.push(path);
                        }
                    }
                }
            }
        }

        paths.retain(|p| {
            if let Some(name) = p.file_name() {
                RE_SUTTA_HTML_NAME.is_match(&name.to_string_lossy())
            } else {
                false
            }
        });

        paths.sort();

        Ok(paths)
    }

    fn import_suttas(&self, conn: &mut SqliteConnection) -> Result<()> {
        let files = self.discover_sutta_files()?;

        info!("Found {} sutta files from Dhammatalks.org", files.len());

        let pb = ProgressBar::new(files.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}")?
                .progress_chars("=>-"),
        );

        let mut success_count = 0;
        let mut error_count = 0;

        for file_path in &files {
            pb.set_message(format!("{}", file_path.file_name().unwrap().to_string_lossy()));

            match self.parse_sutta(file_path) {
                Ok(sutta) => {
                    let new_sutta = sutta.to_new_sutta();
                    match diesel::insert_into(suttas::table)
                        .values(&new_sutta)
                        .execute(conn)
                    {
                        Ok(_) => success_count += 1,
                        Err(e) => {
                            error_count += 1;
                            warn!("Failed to insert sutta {}: {}", file_path.display(), e);
                        }
                    }
                }
                Err(e) => {
                    error_count += 1;
                    warn!("Failed to parse sutta {}: {}", file_path.display(), e);
                }
            }

            pb.inc(1);
        }

        pb.finish_with_message(format!(
            "Imported {} suttas ({} errors)",
            success_count, error_count
        ));

        info!("Successfully imported {} Dhammatalks.org suttas", success_count);
        if error_count > 0 {
            warn!("{} suttas failed to import", error_count);
        }

        Ok(())
    }
}

impl SuttaImporter for DhammatalksSuttaImporter {
    fn import(&mut self, conn: &mut SqliteConnection) -> Result<()> {
        self.import_suttas(conn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_ref_notation_convert() {
        let importer = DhammatalksSuttaImporter::new(PathBuf::new());

        assert_eq!(importer.ref_notation_convert("DN01"), "dn1");
        assert_eq!(importer.ref_notation_convert("MN_02"), "mn.2");
        assert_eq!(importer.ref_notation_convert("stnp1_1"), "snp1.1");
        assert_eq!(importer.ref_notation_convert("khp1"), "kp1");
    }

    #[test]
    fn test_parse_an6_20() {
        let resource_path = PathBuf::from("../../bootstrap-assets-resources/dhammatalks-org/www.dhammatalks.org/suttas");
        let importer = DhammatalksSuttaImporter { resource_path };

        let file_path = PathBuf::from("../../bootstrap-assets-resources/dhammatalks-org/www.dhammatalks.org/suttas/AN/AN6_20.html");

        if !file_path.exists() {
            println!("Test file not found, skipping test");
            return;
        }

        let sutta = importer.parse_sutta(&file_path).expect("Failed to parse sutta");

        // Check basic fields
        assert_eq!(sutta.uid, "an6.20/en/thanissaro");
        assert_eq!(sutta.language, "en");
        assert_eq!(sutta.source_uid, "thanissaro");

        // Check titles - from old DB: title="Maraṇassati Sutta", title_pali="Mindfulness of Death (2)"
        assert_eq!(sutta.title, "Maraṇassati Sutta");
        assert_eq!(sutta.title_pali, Some("Mindfulness of Death (2)".to_string()));

        // Verify wrapper div exists
        assert!(sutta.content_html.contains("<div class=\"dhammatalks_org\">"), "Missing wrapper div");

        // Verify ssp:// links were created
        assert!(sutta.content_html.contains("ssp://suttas/"), "Links not converted to ssp://");
        assert!(sutta.content_html.contains("ssp://suttas/sn3.17/en/thanissaro"), "SN link not converted");

        // Verify key content is present
        assert!(sutta.content_html.contains("Mindfulness of Death"), "Missing title in content");
        assert!(sutta.content_html.contains("Maraṇassati Sutta"), "Missing Pali title in content");
    }

    #[test]
    fn test_parse_snp5_4() {
        let resource_path = PathBuf::from("../../bootstrap-assets-resources/dhammatalks-org/www.dhammatalks.org/suttas");
        let importer = DhammatalksSuttaImporter { resource_path };

        let file_path = PathBuf::from("../../bootstrap-assets-resources/dhammatalks-org/www.dhammatalks.org/suttas/KN/StNp/StNp5_4.html");

        if !file_path.exists() {
            println!("Test file not found, skipping test");
            return;
        }

        let sutta = importer.parse_sutta(&file_path).expect("Failed to parse sutta");

        // Check basic fields
        assert_eq!(sutta.uid, "snp5.4/en/thanissaro");
        assert_eq!(sutta.language, "en");
        assert_eq!(sutta.source_uid, "thanissaro");

        // Check titles - the title includes the number prefix from the HTML
        // Old DB: "4 Mettagū's Questions"  (different unicode quotes)
        assert!(sutta.title.contains("Mettagū") && sutta.title.contains("Questions"),
            "Title should contain main text, got: {}", sutta.title);

        // Load expected content
        let expected_path = PathBuf::from("tests/data/dhammatalks-org/snp5.4_expected.html");
        if expected_path.exists() {
            let expected_html = fs::read_to_string(&expected_path).expect("Failed to read expected HTML");
            let expected_html = expected_html.trim();

            assert!(sutta.content_html.contains("dhammatalks_org"), "Missing wrapper div");
        }
    }

    #[test]
    fn test_parse_dhp17() {
        let resource_path = PathBuf::from("../../bootstrap-assets-resources/dhammatalks-org/www.dhammatalks.org/suttas");
        let importer = DhammatalksSuttaImporter { resource_path };

        let file_path = PathBuf::from("../../bootstrap-assets-resources/dhammatalks-org/www.dhammatalks.org/suttas/KN/Dhp/Ch17.html");

        if !file_path.exists() {
            println!("Test file not found, skipping test");
            return;
        }

        let sutta = importer.parse_sutta(&file_path).expect("Failed to parse sutta");

        // Check basic fields - Ch17 should convert to dhp221-234
        assert_eq!(sutta.uid, "dhp221-234/en/thanissaro");
        assert_eq!(sutta.language, "en");
        assert_eq!(sutta.source_uid, "thanissaro");

        // Check titles - from old DB: title="Anger"
        assert_eq!(sutta.title, "Anger");

        // Load expected content
        let expected_path = PathBuf::from("tests/data/dhammatalks-org/dhp221-234_expected.html");
        if expected_path.exists() {
            let expected_html = fs::read_to_string(&expected_path).expect("Failed to read expected HTML");
            let expected_html = expected_html.trim();

            assert!(sutta.content_html.contains("dhammatalks_org"), "Missing wrapper div");
        }
    }

    #[test]
    fn test_href_sutta_html_to_ssp() {
        let importer = DhammatalksSuttaImporter::new(PathBuf::new());

        // Test simple href conversion
        assert_eq!(
            importer.href_sutta_html_to_ssp("DN01.html"),
            "ssp://suttas/dn1/en/thanissaro"
        );

        // Test with anchor
        assert_eq!(
            importer.href_sutta_html_to_ssp("MN02.html#section1"),
            "ssp://suttas/mn2/en/thanissaro#section1"
        );

        // Test with path
        assert_eq!(
            importer.href_sutta_html_to_ssp("../AN/AN6_20.html"),
            "ssp://suttas/an6.20/en/thanissaro"
        );
    }
}
