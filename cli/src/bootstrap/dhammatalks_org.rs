use anyhow::{Context, Result};
use diesel::prelude::*;
use scraper::{Html, Selector};
use simsapa_backend::db::appdata_schema::suttas;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use regex::Regex;
use tracing::{info, warn};
use indicatif::{ProgressBar, ProgressStyle};

use simsapa_backend::lookup::DHP_CHAPTERS_TO_RANGE;
use simsapa_backend::helpers::{consistent_niggahita, compact_rich_text};
use crate::bootstrap::helpers::{uid_to_ref, uid_to_nikaya};

use super::SuttaImporter;

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

    fn parse_html_file(&self, file_path: &Path) -> Result<Html> {
        let html_text = fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read HTML file: {}", file_path.display()))?;
        Ok(Html::parse_document(&html_text))
    }

    fn extract_sutta_content(&self, html: &Html) -> Result<String> {
        let selector = Selector::parse("#sutta").unwrap();

        if let Some(element) = html.select(&selector).next() {
            let content_html = element.inner_html();
            // FIXME Replace sutta links with internal ssp://
            Ok(content_html)
        } else {
            Err(anyhow::anyhow!("No #sutta element found in HTML"))
        }
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

        let html = Html::parse_document(&html_text);
        let content_html = self.extract_sutta_content(&html)?;
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
                    match diesel::insert_into(suttas::table)
                        .values((
                            suttas::uid.eq(&sutta.uid),
                            suttas::sutta_ref.eq(&sutta.sutta_ref),
                            suttas::nikaya.eq(&sutta.nikaya),
                            suttas::language.eq(&sutta.language),
                            suttas::group_path.eq::<Option<String>>(None),
                            suttas::group_index.eq::<Option<i32>>(None),
                            suttas::order_index.eq::<Option<i32>>(None),
                            suttas::sutta_range_group.eq::<Option<String>>(None),
                            suttas::sutta_range_start.eq::<Option<i32>>(None),
                            suttas::sutta_range_end.eq::<Option<i32>>(None),
                            suttas::title.eq(&sutta.title),
                            suttas::title_ascii.eq(&sutta.title_ascii),
                            suttas::title_pali.eq(&sutta.title_pali),
                            suttas::title_trans.eq::<Option<String>>(None),
                            suttas::description.eq::<Option<String>>(None),
                            suttas::content_plain.eq(&sutta.content_plain),
                            suttas::content_html.eq(&sutta.content_html),
                            suttas::content_json.eq::<Option<String>>(None),
                            suttas::content_json_tmpl.eq::<Option<String>>(None),
                            suttas::source_uid.eq(&sutta.source_uid),
                            suttas::source_info.eq::<Option<String>>(None),
                            suttas::source_language.eq::<Option<String>>(None),
                            suttas::message.eq::<Option<String>>(None),
                            suttas::copyright.eq::<Option<String>>(None),
                            suttas::license.eq::<Option<String>>(None),
                        ))
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

    #[test]
    fn test_ref_notation_convert() {
        let importer = DhammatalksSuttaImporter::new(PathBuf::new());

        assert_eq!(importer.ref_notation_convert("DN01"), "dn1");
        assert_eq!(importer.ref_notation_convert("MN_02"), "mn.2");
        assert_eq!(importer.ref_notation_convert("stnp1_1"), "snp1.1");
        assert_eq!(importer.ref_notation_convert("khp1"), "kp1");
    }
}
