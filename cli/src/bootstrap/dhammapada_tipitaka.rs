use anyhow::{Context, Result};
use diesel::prelude::*;
use scraper::{Html, Selector};
use simsapa_backend::db::appdata_schema::suttas;
use simsapa_backend::lookup::DHP_CHAPTERS_TO_RANGE;
use simsapa_backend::helpers::{consistent_niggahita, compact_rich_text, pali_to_ascii};
use std::path::{Path, PathBuf};
use std::fs;
use std::collections::HashMap;
use regex::Regex;
use tracing::{info, warn};
use indicatif::{ProgressBar, ProgressStyle};

use crate::bootstrap::helpers::{uid_to_ref, uid_to_nikaya};
use crate::bootstrap::SuttaData;

use super::SuttaImporter;

pub struct DhammapadaTipitakaImporter {
    resource_path: PathBuf,
}

#[derive(Debug)]
struct VerseData {
    dhp_num: u32,
    content_html: String,
    title_li: String,
}

impl DhammapadaTipitakaImporter {
    pub fn new(resource_path: PathBuf) -> Self {
        Self { resource_path }
    }

    fn discover_verse_files(&self) -> Result<Vec<PathBuf>> {
        let html_dir = self.resource_path.join("www.tipitaka.net/tipitaka/dhp");

        if !html_dir.exists() {
            anyhow::bail!("HTML directory not found: {}", html_dir.display());
        }

        let mut files = Vec::new();
        let entries = fs::read_dir(&html_dir)
            .with_context(|| format!("Failed to read directory: {}", html_dir.display()))?;

        let re = Regex::new(r"^verseload[a-f0-9]+\.html$").unwrap();

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(file_name) = path.file_name() {
                    if let Some(name_str) = file_name.to_str() {
                        if re.is_match(name_str) {
                            files.push(path);
                        }
                    }
                }
            }
        }

        files.sort();
        Ok(files)
    }

    fn parse_verse_file(&self, path: &Path) -> Result<VerseData> {
        let html_text = fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {}", path.display()))?;

        let re = Regex::new(r"verseload\.php\?verse=(\d+)\w* by HTTrack").unwrap();
        let caps = re.captures(&html_text)
            .with_context(|| format!("Could not find verse number in file: {}", path.display()))?;

        let dhp_num: u32 = caps.get(1)
            .unwrap()
            .as_str()
            .parse()
            .with_context(|| format!("Failed to parse verse number in: {}", path.display()))?;

        let html = Html::parse_document(&html_text);

        let title_selector = Selector::parse(".main > p:first-child > strong").unwrap();
        let title = if let Some(element) = html.select(&title_selector).next() {
            let title_html = element.html();
            let title_text = title_html
                .replace('\n', " ")
                .replace("<br>", " ")
                .replace("<br/>", " ");
            title_text
        } else {
            format!("Dhammapada Verse {}", dhp_num)
        };

        let title_id = format!("title_{}", dhp_num);

        let blockquote_selector = Selector::parse(".main > blockquote").unwrap();
        let content_html = if let Some(element) = html.select(&blockquote_selector).next() {
            let mut html_content = element.inner_html();
            if !html_content.contains(&title_id) {
                html_content = format!("<a id=\"{}\"></a>{}", title_id, html_content);
            }
            html_content
        } else {
            anyhow::bail!("No main blockquote in {}", path.display());
        };

        let title_li = format!("<li><a href=\"#{}\">{}</a></li>", title_id, title);

        Ok(VerseData {
            dhp_num,
            content_html,
            title_li,
        })
    }

    fn dhp_verse_to_chapter(&self, verse_num: u32) -> Option<(u32, u32)> {
        for (_chapter, &(start, end)) in DHP_CHAPTERS_TO_RANGE.iter() {
            if verse_num >= start && verse_num <= end {
                return Some((start, end));
            }
        }
        None
    }

    fn group_verses_by_chapter(&self, verses: Vec<VerseData>) -> HashMap<String, (Vec<String>, Vec<String>)> {
        let mut chapters: HashMap<String, (Vec<String>, Vec<String>)> = HashMap::new();

        for verse in verses {
            if let Some((start, end)) = self.dhp_verse_to_chapter(verse.dhp_num) {
                let ref_key = format!("dhp{}-{}", start, end);

                let entry = chapters.entry(ref_key).or_insert_with(|| (Vec::new(), Vec::new()));
                entry.0.push(verse.content_html);
                entry.1.push(verse.title_li);
            } else {
                warn!("Could not find chapter for verse {}", verse.dhp_num);
            }
        }

        chapters
    }

    fn create_sutta_from_chapter(&self, ref_key: &str, content_parts: Vec<String>, toc_parts: Vec<String>) -> Result<SuttaData> {
        let toc_html = format!("<ul class=\"toc\">{}</ul>", toc_parts.join(""));
        let full_content = format!("{}{}", toc_html, content_parts.join(""));

        let title = consistent_niggahita(Some("Dhammapada".to_string()));
        let title_ascii = pali_to_ascii(Some(&title));
        let title_pali = Some(title.clone());

        let lang = "en";
        let author = "daw";
        let uid = format!("{}/{}/{}", ref_key, lang, author);

        let content_html = format!("<div class=\"tipitaka_net\">{}</div>", consistent_niggahita(Some(full_content)));
        let content_plain = compact_rich_text(&content_html);

        Ok(SuttaData {
            source_uid: author.to_string(),
            title,
            title_ascii,
            title_pali,
            uid,
            sutta_ref: uid_to_ref(ref_key),
            nikaya: uid_to_nikaya(ref_key),
            language: lang.to_string(),
            content_html,
            content_plain,
        })
    }

    fn import_suttas(&self, conn: &mut SqliteConnection) -> Result<()> {
        info!("Discovering verse files...");
        let files = self.discover_verse_files()?;
        info!("Found {} verse files", files.len());

        let pb = ProgressBar::new(files.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("=>-"),
        );
        pb.set_message("Parsing verse files...");

        let mut verses = Vec::new();
        for file_path in &files {
            match self.parse_verse_file(file_path) {
                Ok(verse) => verses.push(verse),
                Err(e) => {
                    warn!("Failed to parse {}: {}", file_path.display(), e);
                }
            }
            pb.inc(1);
        }
        pb.finish_with_message("Verse files parsed");

        info!("Grouping verses by chapter...");
        let chapters = self.group_verses_by_chapter(verses);
        info!("Grouped into {} chapters", chapters.len());

        let pb = ProgressBar::new(chapters.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("=>-"),
        );
        pb.set_message("Inserting suttas...");

        let mut inserted = 0;
        for (ref_key, (content_parts, toc_parts)) in chapters {
            match self.create_sutta_from_chapter(&ref_key, content_parts, toc_parts) {
                Ok(sutta_data) => {
                    let new_sutta = sutta_data.to_new_sutta();
                    match diesel::insert_into(suttas::table)
                        .values(&new_sutta)
                        .execute(conn)
                    {
                        Ok(_) => {
                            inserted += 1;
                        }
                        Err(e) => {
                            warn!("Failed to insert sutta {}: {}", ref_key, e);
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to create sutta for {}: {}", ref_key, e);
                }
            }
            pb.inc(1);
        }
        pb.finish_with_message(format!("Inserted {} suttas", inserted));

        info!("Dhammapada Tipitaka.net import complete: {} suttas inserted", inserted);
        Ok(())
    }
}

impl SuttaImporter for DhammapadaTipitakaImporter {
    fn import(&mut self, conn: &mut SqliteConnection) -> Result<()> {
        self.import_suttas(conn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verse_file_pattern_match() {
        let re = Regex::new(r"^verseload[a-f0-9]+\.html$").unwrap();
        assert!(re.is_match("verseload0076.html"));
        assert!(re.is_match("verseload01aa.html"));
        assert!(re.is_match("verseloadf659.html"));
        assert!(!re.is_match("index.html"));
        assert!(!re.is_match("verseload.html"));
    }

    #[test]
    fn test_verse_number_extraction() {
        let html_text = r#"<!-- Mirrored from www.tipitaka.net/tipitaka/dhp/verseload.php?verse=270 by HTTrack -->"#;
        let re = Regex::new(r"verseload\.php\?verse=(\d+)\w* by HTTrack").unwrap();
        let caps = re.captures(html_text).unwrap();
        let verse_num: u32 = caps.get(1).unwrap().as_str().parse().unwrap();
        assert_eq!(verse_num, 270);
    }

    #[test]
    fn test_verse_number_extraction_with_letter() {
        let html_text = r#"<!-- Mirrored from www.tipitaka.net/tipitaka/dhp/verseload.php?verse=416b by HTTrack -->"#;
        let re = Regex::new(r"verseload\.php\?verse=(\d+)\w* by HTTrack").unwrap();
        let caps = re.captures(html_text).unwrap();
        let verse_num: u32 = caps.get(1).unwrap().as_str().parse().unwrap();
        assert_eq!(verse_num, 416);
    }

    #[test]
    fn test_dhp_verse_to_chapter() {
        let importer = DhammapadaTipitakaImporter::new(PathBuf::from("/tmp"));

        assert_eq!(importer.dhp_verse_to_chapter(1), Some((1, 20)));
        assert_eq!(importer.dhp_verse_to_chapter(20), Some((1, 20)));
        assert_eq!(importer.dhp_verse_to_chapter(76), Some((76, 89)));
        assert_eq!(importer.dhp_verse_to_chapter(270), Some((256, 272)));
        assert_eq!(importer.dhp_verse_to_chapter(423), Some((383, 423)));
        assert_eq!(importer.dhp_verse_to_chapter(424), None);
    }

    #[test]
    fn test_uid_format() {
        let ref_key = "dhp1-20";
        let lang = "en";
        let author = "daw";
        let uid = format!("{}/{}/{}", ref_key, lang, author);
        assert_eq!(uid, "dhp1-20/en/daw");
    }

    #[test]
    fn test_parse_verse_file_270() {
        let test_data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/data/dhammapada-tipitaka-net");

        if !test_data_dir.exists() {
            return;
        }

        let importer = DhammapadaTipitakaImporter::new(PathBuf::from("/tmp"));
        let verse_file = test_data_dir.join("verseload0076.html");

        if !verse_file.exists() {
            return;
        }

        let result = importer.parse_verse_file(&verse_file);
        assert!(result.is_ok());

        let verse_data = result.unwrap();
        assert_eq!(verse_data.dhp_num, 270);
        assert!(verse_data.content_html.contains("Balisika"));
        assert!(verse_data.title_li.contains("title_270"));
    }

    #[test]
    fn test_parse_verse_file_2() {
        let test_data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/data/dhammapada-tipitaka-net");

        if !test_data_dir.exists() {
            return;
        }

        let importer = DhammapadaTipitakaImporter::new(PathBuf::from("/tmp"));
        let verse_file = test_data_dir.join("verseload01aa.html");

        if !verse_file.exists() {
            return;
        }

        let result = importer.parse_verse_file(&verse_file);
        assert!(result.is_ok());

        let verse_data = result.unwrap();
        assert_eq!(verse_data.dhp_num, 2);
        assert_eq!(importer.dhp_verse_to_chapter(verse_data.dhp_num), Some((1, 20)));
    }

    #[test]
    fn test_group_verses_by_chapter() {
        let importer = DhammapadaTipitakaImporter::new(PathBuf::from("/tmp"));

        let verses = vec![
            VerseData {
                dhp_num: 1,
                content_html: "verse 1".to_string(),
                title_li: "li 1".to_string(),
            },
            VerseData {
                dhp_num: 2,
                content_html: "verse 2".to_string(),
                title_li: "li 2".to_string(),
            },
            VerseData {
                dhp_num: 21,
                content_html: "verse 21".to_string(),
                title_li: "li 21".to_string(),
            },
        ];

        let chapters = importer.group_verses_by_chapter(verses);
        assert_eq!(chapters.len(), 2);
        assert!(chapters.contains_key("dhp1-20"));
        assert!(chapters.contains_key("dhp21-32"));

        let chapter1 = chapters.get("dhp1-20").unwrap();
        assert_eq!(chapter1.0.len(), 2);
        assert_eq!(chapter1.1.len(), 2);
    }
}
