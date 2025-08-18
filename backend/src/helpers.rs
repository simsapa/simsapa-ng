use std::collections::{BTreeMap, HashSet, HashMap};

use regex::Regex;
use lazy_static::lazy_static;
use html_escape::decode_html_entities;
use anyhow::{Context, Result};

use crate::types::SearchResult;
use crate::lookup::*;

lazy_static! {
    // MN44; MN 118; AN 4.10; Sn 4:2; Dhp 182; Thag 1207; Vism 152
    // Must not match part of the path in a url, <a class="link" href="ssp://suttas/mn44/en/sujato">
    //
    // r"(?i)(?<!/)\b(DN|MN|SN|AN|Pv|Vv|Vism|iti|kp|khp|snp|th|thag|thig|ud|uda|dhp)[ \.]*(\d[\d\.:]*)\b"
    // r"(?i)(?<!/)\b(D|DN|M|MN|S|SN|A|AN|Pv|Vv|Vin|Vism|iti|kp|khp|snp|th|thag|thig|ud|uda|dhp)[ \.]+([ivxIVX]+)[ \.]+(\d[\d\.]*)\b"
    // (?<!/) error: look-around, including look-ahead and look-behind, is not supported
    pub static ref RE_ALL_BOOK_SUTTA_REF: Regex = Regex::new(
        r"(?i)\b(DN|MN|SN|AN|Pv|Vv|Vism|iti|kp|khp|snp|th|thag|thig|ud|uda|dhp)[ \.]*(\d[\d\.:]*)\b"
    ).unwrap();

    // Vin.iii.40; AN.i.78; D iii 264; SN i 190; M. III. 203.
    pub static ref RE_ALL_PTS_VOL_SUTTA_REF: Regex = Regex::new(
        r"(?i)\b(D|DN|M|MN|S|SN|A|AN|Pv|Vv|Vin|Vism|iti|kp|khp|snp|th|thag|thig|ud|uda|dhp)[ \.]+([ivxIVX]+)[ \.]+(\d[\d\.]*)\b"
    ).unwrap();
}

#[derive(Debug, Clone)]
pub struct SuttaRange {
    // sn30.7-16
    pub group: String,      // sn30
    pub start: Option<u32>, // 7
    pub end: Option<u32>,   // 16
}

pub fn is_book_sutta_ref(reference: &str) -> bool {
    RE_ALL_BOOK_SUTTA_REF.is_match(reference)
}

pub fn is_pts_sutta_ref(reference: &str) -> bool {
    RE_ALL_PTS_VOL_SUTTA_REF.is_match(reference)
}

pub fn query_text_to_uid_field_query(query_text: &str) -> String {
    let query_text = query_text.to_lowercase();
    if query_text.starts_with("uid:") {
        return query_text.to_string();
    }

    // Detect if query is already uid-like, e.g. sn56.11/pli/ms
    if is_complete_sutta_uid(&query_text) {
        return format!("uid:{}", query_text);
    }

    // Or it could be a partial uid, e.g. sn56.11/pli
    lazy_static! {
        static ref re_partial_uid: Regex = Regex::new(r"/[a-z0-9-]+$").unwrap();
    }
    if re_partial_uid.is_match(&query_text) {
        return format!("uid:{}", query_text);
    }

    // Replace user input sutta refs such as 'SN 56.11' with query expression uid:sn56.11
    let mut result = query_text.to_string();

    for cap in RE_ALL_BOOK_SUTTA_REF.captures_iter(&query_text) {
        let full_match = cap.get(0).unwrap().as_str();
        let nikaya = cap.get(1).unwrap().as_str().to_lowercase();
        let number = cap.get(2).unwrap().as_str();
        let replacement = format!("uid:{}{}", nikaya, number);
        result = result.replace(full_match, &replacement);
    }

    result
}

pub fn sutta_range_from_ref(reference: &str) -> Option<SuttaRange> {
    // logger.info(f"sutta_range_from_ref(): {ref}")

    /*
    sn30.7-16/pli/ms -> SuttaRange(group: 'sn30', start: 7, end: 16)
    sn30.1/pli/ms -> SuttaRange(group: 'sn30', start: 1, end: 1)
    dn1-5/bodhi/en -> SuttaRange(group: 'dn', start: 1, end: 5)
    dn12/bodhi/en -> SuttaRange(group: 'dn', start: 12, end: 12)
    dn2-a -> -> SuttaRange(group: 'dn-a', start: 2, end: 2)
    pli-tv-pvr10
    */

    /*
    Problematic:

    _id: text_extra_info/21419
    uid: sn22.57_a
    acronym: SN 22.57(*) + AN 2.19(*)
    volpage: PTS SN iii 61–63 + AN i 58
    */

    let mut ref_str = reference.to_string();

    if ref_str.contains('/') {
        ref_str = ref_str.split('/').next()?.to_string();
    }

    ref_str = ref_str.replace("--", "-");

    // FIXME: convert Regex to lazy_static

    // sn22.57_a -> sn22.57
    ref_str = regex::Regex::new(r"_a$").unwrap()
        .replace(&ref_str, "")
        .to_string();

    // an2.19_an3.29 -> an2.19
    // an3.29_sn22.57 -> an3.29
    ref_str = regex::Regex::new(r"_[as]n.*$").unwrap()
        .replace(&ref_str, "")
        .to_string();

    // snp1.2(33-34) -> snp1.2
    if ref_str == "snp1.2(33-34)" {
        ref_str = "snp1.2".to_string();
    }

    // Atthakata
    if ref_str.ends_with("-a") {
        // dn2-a -> dn-a2
        ref_str = regex::Regex::new(r"([a-z-]+)([0-9-]+)-a").unwrap()
            .replace(&ref_str, "${1}-a${2}")
            .to_string();
    }

    if !ref_str.chars().any(|c| c.is_ascii_digit()) {
        return Some(SuttaRange {
            group: ref_str,
            start: None,
            end: None,
        });
    }

    let (group, numeric) = if ref_str.contains('.') {
        let parts: Vec<&str> = ref_str.split('.').collect();
        if parts.len() < 2 {
            return None;
        }
        (parts[0].to_string(), parts[1].to_string())
    } else {
        let re = regex::Regex::new(r"([a-z-]+)([0-9-]+)").unwrap();
        let caps = re.captures(&ref_str)?;
        // FIXME: if not m: logger.warn(f"Cannot determine range for {ref}")
        (
            caps.get(1)?.as_str().to_string(), // group
            caps.get(2)?.as_str().to_string(), // numeric
        )
    };

    let (start, end) = if numeric.contains('-') {
        let parts: Vec<&str> = numeric.split('-').collect();
        if parts.len() < 2 {
            return None;
        }
        (
            parts[0].parse::<u32>().ok()?,
            parts[1].parse::<u32>().ok()?,
        )
    } else {
        let num = numeric.parse::<u32>().ok()?;
        (num, num)
    };
    // FIXME: except Exception as e: logger.warn(f"Cannot determine range for {ref}: {e}")

    Some(SuttaRange {
        group,
        start: Some(start),
        end: Some(end),
    })
}

pub fn normalize_sutta_ref(reference: &str, for_ebooks: bool) -> String {
    let mut ref_str = reference.to_lowercase();

    ref_str = regex::Regex::new(r"uda *(\d)").unwrap()
        .replace_all(&ref_str, "ud $1")
        .to_string();

    ref_str = regex::Regex::new(r"khp *(\d)").unwrap()
        .replace_all(&ref_str, "kp $1")
        .to_string();

    ref_str = regex::Regex::new(r"th *(\d)").unwrap()
        .replace_all(&ref_str, "thag $1")
        .to_string();

    if for_ebooks {
        ref_str = regex::Regex::new(r"[\. ]*([ivx]+)[\. ]*").unwrap()
            .replace_all(&ref_str, " $1 ")
            .to_string();
    } else {
        // FIXME: the pattern below breaks PTS linking in Buddhadhamma, but the
        // pattern above breaks Mil. uid query lookup.

        // M.III.24 -> M I 24
        ref_str = regex::Regex::new(r"[\. ]([ivx]+)[\. ]").unwrap()
            .replace_all(&ref_str, " $1 ")
            .to_string();
    }

    ref_str = regex::Regex::new(r"^d ").unwrap()
        .replace(&ref_str, "dn ")
        .to_string();

    ref_str = regex::Regex::new(r"^m ").unwrap()
        .replace(&ref_str, "mn ")
        .to_string();

    ref_str = regex::Regex::new(r"^s ").unwrap()
        .replace(&ref_str, "sn ")
        .to_string();

    ref_str = regex::Regex::new(r"^a ").unwrap()
        .replace(&ref_str, "an ")
        .to_string();

    ref_str.trim().to_string()
}

pub fn normalize_sutta_uid(uid: &str) -> String {
    normalize_sutta_ref(uid, false).replace(' ', "")
}

pub fn dhp_verse_to_chapter(verse_num: u32) -> Option<String> {
    for (a, b) in DHP_CHAPTERS_TO_RANGE.values() {
        if verse_num >= *a && verse_num <= *b {
            return Some(format!("dhp{}-{}", a, b));
        }
    }
    None
}

pub fn dhp_chapter_ref_for_verse_num(num: u32) -> Option<String> {
    for (ch, (start, end)) in DHP_CHAPTERS_TO_RANGE.iter() {
        if num >= *ch && num <= *ch {
            return Some(format!("dhp{}-{}", start, end));
        }
    }
    None
}

pub fn thag_verse_to_uid(verse_num: u32) -> Option<String> {
    // v1 - v120 are thag1.x
    if verse_num <= 120 {
        return Some(format!("thag1.{}", verse_num));
    }

    for (uid, (a, b)) in THAG_UID_TO_RANGE.iter() {
        if verse_num >= *a && verse_num <= *b {
            return Some(uid.to_string());
        }
    }
    None
}

pub fn thig_verse_to_uid(verse_num: u32) -> Option<String> {
    // v1 - v18 are thig1.x
    if verse_num <= 18 {
        return Some(format!("thig1.{}", verse_num));
    }

    for (uid, (a, b)) in THIG_UID_TO_RANGE.iter() {
        if verse_num >= *a && verse_num <= *b {
            return Some(uid.to_string());
        }
    }
    None
}

pub fn snp_verse_to_uid(verse_num: u32) -> Option<String> {
    for (uid, (a, b)) in SNP_UID_TO_RANGE.iter() {
        if verse_num >= *a && verse_num <= *b {
            return Some(uid.to_string());
        }
    }
    None
}

pub fn is_complete_sutta_uid(uid: &str) -> bool {
    let uid = uid.trim_matches('/');

    if !uid.contains('/') {
        return false;
    }

    if uid.split('/').count() != 3 {
        return false;
    }

    true
}

pub fn is_complete_word_uid(uid: &str) -> bool {
    // Check if uid contains a /, i.e. if it specifies the dictionary
    // (dhammacakkhu/dpd).
    uid.trim_matches('/').contains('/')
}

pub fn consistent_niggahita(text: Option<String>) -> String {
    // Use only ṁ, both in content and query strings.
    //
    // CST4 uses ṁ
    // SuttaCentral MS uses ṁ
    // Aj Thanissaro's BMC uses ṁ
    // Uncommon Wisdom uses ṁ
    //
    // PTS books use ṃ
    // Digital Pali Reader MS uses ṃ
    // Bodhirasa DPD uses ṃ
    // Bhikkhu Bodhi uses ṃ
    // Forest Sangha Pubs uses ṃ
    // Buddhadhamma uses ṃ

    match text {
        Some(text) => {
            text.replace("ṃ", "ṁ")
        }
        None => String::from("")
    }
}

lazy_static! {
    static ref RE_TRAIL_TI: Regex = Regex::new(r#"[’'"”]n*ti$"#).unwrap();
    static ref RE_NTI: Regex =    Regex::new(r#"n*[’'"”]n*ti"#).unwrap();
    static ref RE_PUNCT: Regex = Regex::new(r#"[\.,;:\!\?'’"”…—–-]+"#).unwrap();
    static ref RE_MANY_SPACES: Regex = Regex::new(r#"  +"#).unwrap();
}

pub fn extract_words(text: &str) -> Vec<String> {
    let text = text.trim();
    if text.is_empty() {
        return Vec::new();
    }

    lazy_static! {
        static ref re_nonword: Regex = Regex::new(r"[^\w]+").unwrap();
        static ref re_digits: Regex = Regex::new(r"\d+").unwrap();
    }

    let text = text.replace("\n", " ").to_string();
    // gantun’ti gantu’nti -> gantuṁ ti
    let text = RE_NTI.replace_all(&text, "ṁ ti").into_owned();
    let text = re_nonword.replace_all(&text, " ").into_owned();
    let text = re_digits.replace_all(&text, " ").into_owned();
    let text = RE_MANY_SPACES.replace_all(&text, " ").into_owned();
    let text = text.trim();

    text.split(" ")
        .map(|i| i.to_string())
        .collect()
}

pub fn clean_word(word: &str) -> String {
    lazy_static! {
        static ref re_start_nonword: Regex = Regex::new(r"^[^\w]+").unwrap();
        static ref re_end_nonword: Regex = Regex::new(r"[^\w]+$").unwrap();
    }

    let lowercased = word.to_lowercase();
    let without_start = re_start_nonword.replace(&lowercased, "");
    let without_end = re_end_nonword.replace(&without_start, "");
    without_end.into_owned()
}

pub fn normalize_query_text(text: Option<String>) -> String {
    let text = consistent_niggahita(text);
    if text.is_empty() {
        return text;
    }
    if text.starts_with("uid:") {
        return text;
    }

    let text = clean_word(&text);
    let text = RE_TRAIL_TI.replace_all(&text, "ti").into_owned();
    let text = text.replace("-", "");
    let text = RE_PUNCT.replace_all(&text, " ").into_owned();
    let text = RE_MANY_SPACES.replace_all(&text, " ").into_owned();
    let text = text.trim().to_string();

    text
}

/// Convert Pāḷi text to ASCII equivalents.
pub fn pali_to_ascii(text: Option<&str>) -> String {
    let text = match text {
        Some(t) => t,
        None => return String::new(),
    };

    // including √ (root sign) and replacing it with space, which gets stripped
    // if occurs at the beginning or end
    let from_chars = "āīūṁṃṅñṭḍṇḷṛṣśĀĪŪṀṂṄÑṬḌṆḶṚṢŚ√";
    let to_chars =   "aiummnntdnlrssAIUMMNNTDNLRSS ";

    let translation: HashMap<char, char> = from_chars.chars()
        .zip(to_chars.chars())
        .collect();

    text.chars()
        .map(|c| translation.get(&c).copied().unwrap_or(c))
        .collect::<String>()
        .trim()
        .to_string()
}

/// Sanitize a word to UID form: remove punctuation, replace spaces with hyphens.
pub fn word_uid_sanitize(word: &str) -> String {
    lazy_static! {
        static ref RE_PUNCT: Regex = Regex::new(r"[\.,;:\(\)]").unwrap();
        static ref RE_DASH: Regex = Regex::new(r"--+").unwrap();
    }
    let mut w = RE_PUNCT.replace_all(word, " ").to_string();
    w = w.replace("'", "")
         .replace("\"", "")
         .replace(' ', "-");
    w = RE_DASH.replace_all(&w, "-").to_string();
    w
}

/// Create a UID by combining sanitized word and dictionary label.
pub fn word_uid(word: &str, dict_label: &str) -> String {
    format!("{}/{}",
            word_uid_sanitize(word).to_lowercase(),
            dict_label.to_lowercase())
}

/// Remove punctuation from text, normalizing whitespace.
pub fn remove_punct(text: Option<&str>) -> String {
    let mut s = match text {
        Some(t) => t.to_string(),
        None => return String::new(),
    };

    lazy_static! {
        static ref RE_PUNCT: Regex = Regex::new(r"[\.,;\?\!“”‘’…—-]").unwrap();
        static ref RE_SPACES: Regex = Regex::new(r" {2,}").unwrap();
    }

    // Replace punctuation marks with space. Removing them can join lines or words.
    s = RE_PUNCT.replace_all(&s, " ").to_string();

    // Newline and tab to space
    s = s.replace("\n", " ")
         .replace("\t", " ");

    // Separate 'ti from the word, avoid joining it when ' is removed
    s = s.replace("'ti", " ti");

    // Remove remaining quote marks.
    //
    // Quote marks can occur in compounds: manopubbaṅ'gamā dhammā
    s = s.replace("'", "")
         .replace("\"", "");

    // Normalize double spaces to single
    s = RE_SPACES.replace_all(&s, " ").to_string();

    s
}

pub fn compact_plain_text(text: &str) -> String {
    // NOTE: Don't remove new lines here, useful for matching beginning of lines when setting snippets.
    // TODO: But remove_punct() removes new lines, is that a problem?
    lazy_static! {
        static ref RE_SPACES: Regex = Regex::new(r" {2,}").unwrap();
    }
    // Replace multiple spaces to one.
    let mut s = RE_SPACES.replace_all(text, " ").to_string();
    s = s.replace('{', "").replace('}', "");

    // Make lowercase and remove punctuation to help matching query strings.
    s = s.to_lowercase();
    s = remove_punct(Some(&s));
    s = consistent_niggahita(Some(s));
    s.trim().to_string()
}

/// Compact rich HTML text: strip tags, normalize, then compact plain.
pub fn compact_rich_text(text: &str) -> String {
    lazy_static! {
        static ref RE_REF_LINK: Regex = Regex::new(r#"<a class=.ref\b[^>]+>[^<]*</a>"#).unwrap();
        // Respect word boundaries for <b> <strong> <i> <em> so that dhamm<b>āya</b> becomes dhammāya, not dhamm āya.
        // Also matches corresponding closing tags
        static ref RE_TAG_BOUNDARY: Regex = Regex::new(r"(\w*)<(/?)(b|strong|i|em)([^>]*)>(\w*)").unwrap();
    }

    // All on one line
    let mut s = text.replace("\n", " ");

    // remove SuttaCentral ref links
    s = RE_REF_LINK.replace_all(&s, "").to_string();

    s = s.replace("<br>", " ")
         .replace("<br/>", " ");

    s = RE_TAG_BOUNDARY.replace_all(&s, |caps: &regex::Captures| {
        format!("{}{}", &caps[1], &caps[5])
    }).to_string();

    // Make sure there is space before and after other tags, so words don't get joined after removing tags.
    //
    // <td>dhammassa</td>
    // <td>dhammāya</td>
    //
    // should become
    //
    // dhammassa dhammāya

    // ensure spaces around other tags
    s = s.replace('<', " <")
         .replace("</", " </")
         .replace('>', "> ");

    s = strip_html(&s);
    compact_plain_text(&s)
}

/// Strip HTML tags, scripts, styles, comments, and decode entities.
pub fn strip_html(text: &str) -> String {
    lazy_static! {
        // thumb up and thumb down emoji
        static ref RE_THUMBS: Regex = Regex::new(r"[\u{1F44D}\u{1F44E}]+").unwrap();
        static ref RE_DOCTYPE: Regex = Regex::new(r"(?i)<!doctype html>").unwrap();
        static ref RE_HEAD: Regex = Regex::new(r"<head(.*?)</head>").unwrap();
        static ref RE_STYLE: Regex = Regex::new(r"<style(.*?)</style>").unwrap();
        static ref RE_SCRIPT: Regex = Regex::new(r"<script(.*?)</script>").unwrap();
        static ref RE_COMMENT: Regex = Regex::new(r"<!--(.*?)-->").unwrap();
        static ref RE_TAG: Regex = Regex::new(r"</*\w[^>]*>").unwrap();
        static ref RE_SPACES: Regex = Regex::new(r" {2,}").unwrap();
    }
    // Decode HTML entities first (e.g., &amp; -> &)
    let mut s = decode_html_entities(text).to_string();
    // Remove html
    s = RE_THUMBS.replace_all(&s, "").to_string();
    s = RE_DOCTYPE.replace_all(&s, "").to_string();
    s = RE_HEAD.replace_all(&s, "").to_string();
    s = RE_STYLE.replace_all(&s, "").to_string();
    s = RE_SCRIPT.replace_all(&s, "").to_string();
    s = RE_COMMENT.replace_all(&s, "").to_string();
    s = RE_TAG.replace_all(&s, "").to_string();
    // Normalize spaces
    s = RE_SPACES.replace_all(&s, " ").to_string();
    s.trim().to_string()
}

/// Clean root info from HTML, returning plain text.
pub fn root_info_clean_plaintext(html: &str) -> String {
    let mut s = strip_html(html);
    s = s.replace('･', " ");
    s = s.replace("Pāḷi Root:", "");
    lazy_static! {
        static ref RE_BASES: Regex = Regex::new(r"Bases:.*$").unwrap();
    }
    s = RE_BASES.replace_all(&s, "").to_string();
    s.trim().to_string()
}

/// Replace accented Pāḷi characters with ASCII latin equivalents.
pub fn latinize(text: &str) -> String {
    let accents = ["ā","ī","ū","ṃ","ṁ","ṅ","ñ","ṭ","ḍ","ṇ","ḷ","ṛ","ṣ","ś"];
    let latin  =  ["a","i","u","m","m","n","n","t","d","n","l","r","s","s"];
    let mut s = text.to_string().to_lowercase();
    for (a, l) in accents.iter().zip(latin.iter()) {
        s = s.replace(a, l);
    }
    s
}

/// Extracts the content of the <body> tag from an HTML string using basic string finding.
pub fn html_get_sutta_page_body(html_page: &str) -> Result<String> {
    // Only parse if it looks like a full HTML document
    if html_page.contains("<html") || html_page.contains("<HTML") {
        // Find the start of the body tag
        let body_start_pos = html_page.to_lowercase().find("<body");
        let body_end_pos = html_page.to_lowercase().find("</body>");

        if let Some(start_index_tag) = body_start_pos {
            // Find the closing '>' of the start tag
            if let Some(start_index_content_offset) = html_page[start_index_tag..].find('>') {
                let content_start = start_index_tag + start_index_content_offset + 1;
                // From the start of the closing body tag
                if let Some(end_index) = body_end_pos {
                    if end_index >= content_start {
                        // Extract the content between the tags
                        Ok(html_page[content_start..end_index].to_string())
                    } else {
                        // log::warn!("HTML document is missing a closing </body> tag");
                        // Return content from start tag to end of string as fallback
                        Ok(html_page[content_start..].to_string())
                    }
                } else {
                    Ok(html_page[content_start..].to_string())
                }
            } else {
                // log::error!("Could not find closing '>' for <body> tag");
                Ok(html_page.to_string())
            }
        } else {
            // log::error!("HTML document is missing a <body> tag");
            // Return the original string if body is not found
            Ok(html_page.to_string())
        }
    } else {
        // If no <html> tag, assume it's already just the body content
        Ok(html_page.to_string())
    }
}

/// Performs post-processing on Bilara HTML content:
/// - Add .noindex to <footer> in suttacentral html
pub fn bilara_html_post_process(body: &str) -> String {
    body.replace("<footer>", "<footer class='noindex'>")
}

/// Converts Bilara text JSON data into a BTreeMap of processed HTML segments, preserving key order.
pub fn bilara_text_to_segments(
    content_json_str: &str,
    tmpl_json_str: Option<&str>,
    variant_json_str: Option<&str>,
    comment_json_str: Option<&str>,
    gloss_json_str: Option<&str>,
    show_variant_readings: bool,
    show_glosses: bool,
) -> Result<BTreeMap<String, String>> {

    // Parse the JSON strings into BTreeMaps to preserve order
    let mut content_json: BTreeMap<String, String> = serde_json::from_str(content_json_str)
        .with_context(|| format!("Failed to parse content JSON: '{}'", content_json_str))?;

    // Optional JSONs are also parsed into BTreeMaps
    let tmpl_json: Option<BTreeMap<String, String>> = tmpl_json_str
        .map(|s| serde_json::from_str(s))
        .transpose() // Converts Option<Result<T, E>> to Result<Option<T>, E>
        .with_context(|| format!("Failed to parse template JSON: '{:?}'", tmpl_json_str))?;

    let variant_json: Option<BTreeMap<String, String>> = variant_json_str
        .map(|s| serde_json::from_str(s))
        .transpose()
        .with_context(|| format!("Failed to parse variant JSON: '{:?}'", variant_json_str))?;

    let comment_json: Option<BTreeMap<String, String>> = comment_json_str
        .map(|s| serde_json::from_str(s))
        .transpose()
        .with_context(|| format!("Failed to parse comment JSON: '{:?}'", comment_json_str))?;

    let gloss_json: Option<BTreeMap<String, String>> = gloss_json_str
        .map(|s| serde_json::from_str(s))
        .transpose()
        .with_context(|| format!("Failed to parse gloss JSON: '{:?}'", gloss_json_str))?;

    // Iterate through the content keys (BTreeMap iterator preserves order)
    // We modify the map in place, so we need to collect keys first if we were removing/inserting differently,
    // but since we are just updating values, iterating directly might be okay.
    // However, collecting keys is safer if logic becomes more complex.
    let keys: Vec<String> = content_json.keys().cloned().collect();

    for i in keys {
        // Get the original content, update it, and put it back.
        // Need to handle the case where the key might have been removed, though unlikely here.
        if let Some(original_content) = content_json.get(&i).cloned() {
            let mut segment_additions = String::new();

            // Append Variant HTML
            if let Some(ref variants) = variant_json {
                if let Some(txt) = variants.get(&i).map(|s| s.trim()).filter(|s| !s.is_empty()) {
                    let mut classes = vec!["variant"];
                    if !show_variant_readings { classes.push("hide"); }
                    let s = format!(r#"
                                    <span class='variant-wrap'>
                                        <span class='mark'>⧫</span>
                                        <span class='{}'>({})</span>
                                    </span>"#,
                                    classes.join(" "), txt);
                    segment_additions.push_str(&s);
                }
            }

            // Append Comment HTML
            if let Some(ref comments) = comment_json {
                if let Some(txt) = comments.get(&i).map(|s| s.trim()).filter(|s| !s.is_empty()) {
                    let s = format!(r#"<span class='comment-wrap'><span class='mark'>✱</span><span class='comment hide'>({})</span></span>"#,
                                    txt);
                    segment_additions.push_str(&s);
                }
            }

            // Append Gloss HTML
            if let Some(ref glosses) = gloss_json {
                if let Some(txt) = glosses.get(&i).map(|s| s.trim()).filter(|s| !s.is_empty()) {
                    let mut classes = vec!["gloss"];
                    if !show_glosses { classes.push("hide"); }
                    let gloss_id = format!("gloss_{}", i.replace(":", "_").replace(".", "_"));
                    let s = format!(r#"<span class='gloss-wrap' onclick="toggle_gloss('#{}')"><span class='mark'><svg class="ssp-icon-button__icon"><use xlink:href="\#icon-table"></use></svg></span></span><div class='{}'>{}</div>"#,
                                    gloss_id, classes.join(" "), txt);
                    segment_additions.push_str(&s);
                }
            }

            /*
            Template JSON example:
            {
                "mn10:0.1": "<article id='mn10'><header><ul><li class='division'>{}</li></ul>",
                "mn10:0.2": "<h1 class='sutta-title'>{}</h1></header>",
                "mn10:1.1": "<p><span class='evam'>{}</span>",
                "mn10:1.2": "{}",
                "mn10:1.3": "{}",
                "mn10:1.4": "{}</p>",
            }
            */

            // Combine original content with additions
            let final_segment_content = format!("{}{}", original_content, segment_additions);

            // Apply template if available
            let final_segment = if let Some(ref tmpl) = tmpl_json {
                if let Some(template_str) = tmpl.get(&i) {
                    // Wrap the combined content before inserting into the template
                    let wrapped_content = format!("<span data-tmpl-key='{}'>{}</span>", i, final_segment_content);
                    template_str.replace("{}", &wrapped_content)
                } else {
                    // No template for this key
                    final_segment_content
                }
            } else {
                // No template map at all
                final_segment_content
            };

            // Update the map with the processed segment
            content_json.insert(i.clone(), final_segment);
        }
    }

    // Return the modified BTreeMap
    Ok(content_json)
}

/// Converts a BTreeMap of processed HTML segments into a single HTML string, preserving order.
pub fn bilara_content_json_to_html(content_json: &BTreeMap<String, String>) -> Result<String> {
    // BTreeMap iteration is already sorted by key.
    let page: String = content_json
        .values()
        .cloned() // Get owned Strings
        .collect::<Vec<String>>()
        .join("\n\n");

    let body = html_get_sutta_page_body(&page)?;
    let processed_body = bilara_html_post_process(&body);

    let content_html = format!("<div class='suttacentral bilara-text'>{}</div>", processed_body);

    Ok(content_html)
}

/// Creates line-by-line HTML view combining translated and Pali segments using BTreeMaps.
pub fn bilara_line_by_line_html(
    translated_json: &BTreeMap<String, String>,
    pali_json: &BTreeMap<String, String>,
    tmpl_json: &BTreeMap<String, String>,
) -> Result<String> {
    // Result map will also be a BTreeMap to maintain order for the final conversion
    let mut content_json: BTreeMap<String, String> = BTreeMap::new();

    // Iterate through the translated map (already sorted by key)
    for (i, translated_segment) in translated_json.iter() {
        let pali_segment = pali_json.get(i).cloned().unwrap_or_default(); // Get Pali or empty string

        let combined_segment = format!(
            "<span class='segment'>
                <span class='translated'>{}</span>
                <span class='pali'>{}</span>
            </span>",
            translated_segment, pali_segment
        );

        // Apply template if available
        if let Some(template_str) = tmpl_json.get(i) {
            content_json.insert(i.clone(), template_str.replace("{}", &combined_segment));
        } else {
            // If no template for this key, use the combined segment directly
            content_json.insert(i.clone(), combined_segment);
        }
    }

    // Convert the combined segments map (which now respects template structure) to final HTML
    bilara_content_json_to_html(&content_json)
}


/// Convenience function to convert Bilara text JSON directly to HTML.
pub fn bilara_text_to_html(
    content_json_str: &str,
    tmpl_json_str: &str,
    variant_json_str: Option<&str>,
    comment_json_str: Option<&str>,
    gloss_json_str: Option<&str>,
    show_variant_readings: bool,
    show_glosses: bool,
) -> Result<String> {
    let content_json = bilara_text_to_segments(
        content_json_str,
        Some(tmpl_json_str),
        variant_json_str,
        comment_json_str,
        gloss_json_str,
        show_variant_readings,
        show_glosses,
    )?;

    bilara_content_json_to_html(&content_json)
}

/// Remove duplicates based on title, schema_name, and uid
pub fn unique_search_results(mut results: Vec<SearchResult>) -> Vec<SearchResult> {
    let mut seen: HashSet<String> = HashSet::new();
    results.retain(|item| {
        let key = format!("{} {} {}", item.title, item.schema_name, item.uid);
        if seen.contains(&key) {
            false
        } else {
            seen.insert(key);
            true
        }
    });
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pali_to_ascii() {
        assert_eq!(pali_to_ascii(Some("dhammāya")), "dhammaya");
        assert_eq!(pali_to_ascii(Some("saṁsāra")), "samsara");
        assert_eq!(pali_to_ascii(Some("Ñāṇa")), "Nana");
        assert_eq!(pali_to_ascii(Some("  √muc  ")), "muc");
        assert_eq!(pali_to_ascii(None), "");
    }

    #[test]
    fn test_word_uid_sanitize() {
        assert_eq!(word_uid_sanitize("word.with,punct;"), "word-with-punct-");
        assert_eq!(word_uid_sanitize("word (bracket)"), "word-bracket-");
        assert_eq!(word_uid_sanitize("word's quote\""), "words-quote");
        assert_eq!(word_uid_sanitize("word--with---dashes"), "word-with-dashes");
        assert_eq!(word_uid_sanitize("  leading space  "), "-leading-space-");
    }

    #[test]
    fn test_word_uid() {
        assert_eq!(word_uid("kammavācā", "PTS"), "kammavācā/pts");
        assert_eq!(word_uid("paṭisallāna", "dpd"), "paṭisallāna/dpd");
    }

    #[test]
    fn test_remove_punct() {
        assert_eq!(remove_punct(Some("Hello, world! How are you? …")), "Hello world How are you ");
        assert_eq!(remove_punct(Some("Line1.\nLine2;")), "Line1 Line2 ");
        assert_eq!(remove_punct(Some("nibbāpethā'ti")), "nibbāpethā ti");
        assert_eq!(remove_punct(Some("  Multiple   spaces.  ")), " Multiple spaces ");
        assert_eq!(remove_punct(None), "");
    }

    #[test]
    fn test_compact_plain_text() {
        assert_eq!(compact_plain_text("  HELLO, World! ṃ {test}  "), "hello world ṁ test");
        assert_eq!(compact_plain_text("Saṃsāra."), "saṁsāra");
    }

    #[test]
    fn test_strip_html() {
        assert_eq!(strip_html("<p>Hello <b>world</b></p>"), "Hello world");
        assert_eq!(strip_html("Text with &amp; entity."), "Text with & entity.");
        assert_eq!(strip_html("<head><title>T</title></head><body>Text</body>"), "Text");
        assert_eq!(strip_html("👍 Text 👎"), "Text");
    }

    #[test]
    fn test_compact_rich_text() {
        assert_eq!(compact_rich_text("<p>Hello, <b>W</b>orld! ṃ</p>\n<a class=\"ref\">ref</a>"), "hello world ṁ");
        assert_eq!(compact_rich_text("dhamm<b>āya</b>"), "dhammāya");
        assert_eq!(compact_rich_text("<i>italic</i> test"), "italic test");
        assert_eq!(compact_rich_text("<td>dhammassa</td><td>dhammāya</td>"), "dhammassa dhammāya");
    }

    #[test]
    fn test_root_info_clean_plaintext() {
        let html = "<div>Pāḷi Root: √gam ･ Bases: gacchati etc.</div>";
        assert_eq!(root_info_clean_plaintext(html), "√gam");
    }

    #[test]
    fn test_latinize() {
        assert_eq!(latinize("dhammāya"), "dhammaya");
        assert_eq!(latinize("saṁsāra"), "samsara");
        assert_eq!(latinize("Ñāṇa"), "nana");
    }

    #[test]
    fn test_consistent_niggahita() {
        assert_eq!(consistent_niggahita(Some("saṃsāra".to_string())), "saṁsāra");
        assert_eq!(consistent_niggahita(Some("dhammaṁ".to_string())), "dhammaṁ");
    }

    #[test]
    fn test_clean_word() {
        assert_eq!(clean_word("Hello"), "hello");
        assert_eq!(clean_word("!!!Hello!!!"), "hello");
        assert_eq!(clean_word("  Word123  "), "word123");
        assert_eq!(clean_word("@#$test@#$"), "test");
        assert_eq!(clean_word(""), "");
        assert_eq!(clean_word("!!!"), "");
    }

    #[test]
    fn test_clean_word_pali_examples() {
        let test_words = [
            "‘sakkomi",
            "gantun’",
            "sampannasīlā,",
            "(Yathā",
            "vitthāretabbaṁ.)",
            "anāsavaṁ …",
        ];

        let cleaned_words: Vec<String> = test_words
            .iter()
            .map(|word| clean_word(word))
            .collect();

        let expected_words = [
            "sakkomi",
            "gantun",
            "sampannasīlā",
            "yathā",
            "vitthāretabbaṁ",
            "anāsavaṁ",
        ];

        assert_eq!(cleaned_words.join(" "), expected_words.join(" "));
    }

    #[test]
    fn test_normalize_query_text() {
        let mut texts: HashMap<&str, &str> = HashMap::new();
        texts.insert(
            "Anāsavañca vo, bhikkhave, desessāmi",
            "anāsavañca vo bhikkhave desessāmi",
        );
        texts.insert(
            "padakkhiṇaṁ mano-kammaṁ",
            "padakkhiṇaṁ manokammaṁ",
        );
        texts.insert(
            "saraṇaṁ…pe॰…anusāsanī’’ti?",
            "saraṇaṁ pe॰ anusāsanī ti",
        );
        texts.insert(
            "katamañca, bhikkhave, nibbānaṁ…pe॰… abyāpajjhañca [abyāpajjhañca (sī॰ syā॰ kaṁ॰ pī॰)] vo, bhikkhave, desessāmi abyāpajjhagāmiñca maggaṁ.",
            "katamañca bhikkhave nibbānaṁ pe॰ abyāpajjhañca [abyāpajjhañca (sī॰ syā॰ kaṁ॰ pī॰)] vo bhikkhave desessāmi abyāpajjhagāmiñca maggaṁ",
        );

        for (query_text, expected) in texts.into_iter() {
            assert_eq!(normalize_query_text(Some(query_text.to_string())), expected.to_string());
        }
    }

    #[test]
    fn test_extract_words_basic() {
        let results = extract_words("Hello world test");
        assert_eq!(results.len(), 3);
        assert_eq!(results[0], "Hello");
        assert_eq!(results[1], "world");
        assert_eq!(results[2], "test");

        // Test punctuation
        let results = extract_words("Hello, world!");
        assert_eq!(results.len(), 2);
        assert_eq!(results[0], "Hello");
        assert_eq!(results[1], "world");

        // Test empty string
        let results = extract_words("");
        assert_eq!(results.len(), 0);

        // Unicode text
        let results = extract_words("Pāḷi ñāṇa");
        assert_eq!(results.len(), 2);
        assert_eq!(results[0], "Pāḷi");
        assert_eq!(results[1], "ñāṇa");

        // Multiple spaces
        let results = extract_words("word1    word2");
        assert_eq!(results.len(), 2);

        // Filter punctuation and non-words
        let results = extract_words("(48.50) samādhi1 ... hey ho! !!");
        assert_eq!(results.len(), 3);
        assert_eq!(results[0], "samādhi");
        assert_eq!(results[1], "hey");
        assert_eq!(results[2], "ho");
    }

    #[test]
    fn test_extract_words_nti() {
        let text = "yaṁ jaññā — ‘sakkomi ajjeva gantun’ti gantu”nti.";
        let words: String = extract_words(text).join(" ");
        let expected_words = "yaṁ jaññā sakkomi ajjeva gantuṁ ti gantuṁ ti".to_string();
        assert_eq!(words, expected_words);
    }

    #[test]
    fn test_extract_words_filter_numbers() {
        let text = "18. idha nandati";
        let words: String = extract_words(text).join(" ");
        let expected_words = "idha nandati".to_string();
        assert_eq!(words, expected_words);
    }

    // Test cases for book references
    const BOOK_REF_TEST_CASES: &[(&str, &str)] = &[
        // test input, expected uid (second value not used in is_book_sutta_ref test)
        ("MN 1", "mn1"),
        ("MN1", "mn1"),
        ("MN44", "mn44"),
        ("MN 118", "mn118"),
        ("AN 4.10", "an4.10"),
        ("Sn 4:2", "sn4.2"),
        ("Dhp 182", "dhp179-196"),
        ("Thag 1207", "thag20.1"),
    ];

    #[test]
    fn test_is_book_sutta_ref() {
        for (case, _expected) in BOOK_REF_TEST_CASES {
            let is_ref = is_book_sutta_ref(case);
            println!("{}: {}", case, is_ref);
            assert!(is_ref, "Failed for case: {}", case);
        }

        // Additional tests from original
        assert!(is_book_sutta_ref("MN 118"));
        assert!(is_book_sutta_ref("AN 4.10"));
        assert!(is_book_sutta_ref("Dhp 182"));
        // FIXME assert!(!is_book_sutta_ref("ssp://suttas/mn44/en/sujato"));
    }

    #[test]
    fn test_query_text_to_uid() {
        let query_text = "SN 44.22";
        let uid = query_text_to_uid_field_query(query_text);
        assert_eq!(uid, "uid:sn44.22");
    }

    // #[test]
    // fn test_not_matching_url_path_sep() {
    //     // Regex must not match part of the path sep (/) in a url, only mn44
    //     // <a class="link" href="ssp://suttas/mn44/en/sujato">
    //     let text = "/mn44/en/sujato";
    //     let is_ref = is_book_sutta_ref(text) || is_pts_sutta_ref(text);
    //     FIXME assert!(!is_ref, "Should not match URL with leading slash");
    // }

    #[test]
    fn test_does_match_complete_uid() {
        // But it should match without the leading "/"
        let text = "mn44/en/sujato";
        let is_ref = is_book_sutta_ref(text) || is_pts_sutta_ref(text);
        assert!(is_ref, "Should match complete UID without leading slash");
    }

    #[test]
    fn test_normalize_sutta_ref() {
        assert_eq!(normalize_sutta_ref("M.III.24", false), "mn iii 24");
        assert_eq!(normalize_sutta_ref("d 1", false), "dn 1");
        assert_eq!(normalize_sutta_ref("uda 5", false), "ud 5");
    }

    #[test]
    fn test_sutta_range() {
        let range = sutta_range_from_ref("sn30.7-16/pli/ms").unwrap();
        assert_eq!(range.group, "sn30");
        assert_eq!(range.start, Some(7));
        assert_eq!(range.end, Some(16));

        let range = sutta_range_from_ref("dn12/bodhi/en").unwrap();
        assert_eq!(range.group, "dn");
        assert_eq!(range.start, Some(12));
        assert_eq!(range.end, Some(12));
    }

    #[test]
    fn test_dhp_verse_to_chapter() {
        assert_eq!(dhp_verse_to_chapter(182), Some("dhp179-196".to_string()));
        assert_eq!(dhp_verse_to_chapter(15), Some("dhp1-20".to_string()));
        assert_eq!(dhp_verse_to_chapter(25), Some("dhp21-32".to_string()));
    }

    #[test]
    fn test_thag_verse_to_uid() {
        assert_eq!(thag_verse_to_uid(50), Some("thag1.50".to_string()));
        assert_eq!(thag_verse_to_uid(121), Some("thag2.1".to_string()));
        assert_eq!(thag_verse_to_uid(122), Some("thag2.1".to_string()));
    }

    #[test]
    fn test_is_complete_sutta_uid() {
        assert!(is_complete_sutta_uid("mn44/en/sujato"));
        assert!(!is_complete_sutta_uid("mn44"));
        assert!(!is_complete_sutta_uid("mn44/en"));
        assert!(!is_complete_sutta_uid("mn44/en/sujato/extra"));
    }

    #[test]
    fn test_is_complete_word_uid() {
        assert!(is_complete_word_uid("dhammacakkhu/dpd"));
        assert!(!is_complete_word_uid("dhammacakkhu"));
    }
}
