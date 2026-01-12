use std::sync::RwLock;
use std::path::Path;

use indexmap::IndexMap;

use diesel::prelude::*;
use regex::Regex;
use lazy_static::lazy_static;
use anyhow::{anyhow, Context, Result};

use crate::db::{appdata_models::*, DbManager};
use crate::db::appdata_schema::suttas::dsl::*;

use crate::logger::{error, info};
use crate::types::SuttaQuote;
use crate::app_settings::AppSettings;
use crate::helpers::{bilara_text_to_segments, bilara_line_by_line_html, bilara_content_json_to_html};
use crate::{get_app_globals, init_app_globals};

/// Represents the application data and settings
#[derive(Debug)]
pub struct AppData {
    pub dbm: DbManager,
    pub app_settings_cache: RwLock<AppSettings>,
    pub api_url: String,
}

impl AppData {
    pub fn new() -> Self {
        init_app_globals();
        let dbm = DbManager::new().expect("Can't create DbManager");
        let app_settings_cache = RwLock::new(dbm.userdata.get_app_settings().clone());
        let g = get_app_globals();

        AppData {
            dbm,
            app_settings_cache,
            api_url: g.api_url.clone(),
        }
    }

    /// Fetches the corresponding Pali sutta for a translated sutta.
    pub fn get_pali_for_translated(&self, sutta: &Sutta) -> Result<Option<Sutta>> {
        if sutta.language == "pli" {
            return Ok(None);
        }

        // Use regex to extract the base UID part (e.g., "mn1" from "mn1/en/bodhi")
        let re = Regex::new("^([^/]+)/.*").expect("Invalid regex");
        let uid_ref = re.replace(&sutta.uid, "$1").to_string();

        let db_conn = &mut self.dbm.appdata.get_conn().expect("No appdata conn");

        let res = suttas
            .select(Sutta::as_select())
            .filter(uid.ne(&sutta.uid))
            .filter(language.eq("pli"))
            .filter(uid.like(format!("{}/%", uid_ref)))
            .first(db_conn)
            .optional() // Makes it return Result<Option<USutta>> instead of erroring if not found
            .context("Database query failed for Pali sutta")?;

        Ok(res)
    }

    /// Converts sutta data into an IndexMap of segments, potentially including variants, comments, glosses.
    /// Returns IndexMap to preserve JSON insertion order.
    pub fn sutta_to_segments_json(
        &self,
        sutta: &Sutta,
        use_template: bool,
    ) -> Result<IndexMap<String, String>> {
        use crate::db::appdata_schema::{sutta_variants, sutta_comments, sutta_glosses};

        let db_conn = &mut self.dbm.appdata.get_conn().expect("No appdata conn");

        let variant_record = sutta_variants::table
            .filter(sutta_variants::sutta_uid.eq(&sutta.uid))
            .select(SuttaVariant::as_select())
            .first::<SuttaVariant>(db_conn)
            .optional()
            .context("Database query failed for SuttaVariant")?;
        // Extract the content_json string if the record was found
        let variant_json_str: Option<String> = variant_record.and_then(|v| v.content_json);

        let comment_record = sutta_comments::table
            .filter(sutta_comments::sutta_uid.eq(&sutta.uid))
            .select(SuttaComment::as_select())
            .first::<SuttaComment>(db_conn)
            .optional()
            .context("Database query failed for SuttaComment")?;
        let comment_json_str: Option<String> = comment_record.and_then(|c| c.content_json);

        let gloss_record = sutta_glosses::table
            .filter(sutta_glosses::sutta_uid.eq(&sutta.uid))
            .select(SuttaGloss::as_select())
            .first::<SuttaGloss>(db_conn)
            .optional()
            .context("Database query failed for SuttaGloss")?;
        let gloss_json_str: Option<String> = gloss_record.and_then(|g| g.content_json);

        let tmpl_str = if use_template {
            sutta.content_json_tmpl.as_deref()
        } else {
            None
        };

        let content_str = sutta.content_json.as_deref()
            .ok_or_else(|| anyhow!("Sutta {} is missing content_json", sutta.uid))?;

        let app_settings = self.app_settings_cache.read().expect("Failed to read app settings");

        bilara_text_to_segments(
            content_str,
            tmpl_str,
            variant_json_str.as_deref(),
            comment_json_str.as_deref(),
            gloss_json_str.as_deref(),
            app_settings.show_all_variant_readings,
            app_settings.show_glosses,
        )
    }

    /// Renders the complete HTML page for a sutta.
    ///
    /// See also: simsapa/simsapa/app/export_helpers.py::render_sutta_content()
    pub fn render_sutta_content(
        &self,
        sutta: &Sutta,
        sutta_quote: Option<&SuttaQuote>,
        js_extra_pre: Option<String>,
    ) -> Result<String> {
        let app_settings = self.app_settings_cache.read().expect("Failed to read app settings");

        let content_html_body = if let Some(ref content_json_str) = sutta.content_json {
            if !content_json_str.is_empty() {
                // Check setting for line-by-line view
                let line_by_line = app_settings.show_translation_and_pali_line_by_line;

                // Attempt to fetch Pali sutta if needed
                let pali_sutta_result = if line_by_line && sutta.language != "pli" {
                    self.get_pali_for_translated(sutta)
                } else {
                    Ok(None)
                };
                let pali_sutta = pali_sutta_result.context("Failed to get Pali sutta for translated version")?;

                if let (true, Some(pali_sutta)) = (line_by_line, pali_sutta) {
                    // Generate line-by-line HTML
                    let translated_segments = self.sutta_to_segments_json(sutta, false)
                                                      .context("Failed to generate translated segments for line-by-line view")?;
                    let pali_segments = self.sutta_to_segments_json(&pali_sutta, false)
                                                .context("Failed to generate Pali segments for line-by-line view")?;

                    let tmpl_str = sutta.content_json_tmpl.as_deref()
                                                          .ok_or_else(|| anyhow!("Sutta {} requires content_json_tmpl for line-by-line view", sutta.uid))?;
                    // Parse template into IndexMap as well
                    let tmpl_json: IndexMap<String, String> = serde_json::from_str(tmpl_str)
                        .with_context(|| format!("Failed to parse template JSON into IndexMap for line-by-line view (Sutta: {})", sutta.uid))?;

                    bilara_line_by_line_html(&translated_segments, &pali_segments, &tmpl_json)?
                } else {
                    // Generate standard HTML view (using template within sutta_to_segments_json)
                    let segments_json = self.sutta_to_segments_json(sutta, true)
                                                .context("Failed to generate segments for standard view")?;
                    bilara_content_json_to_html(&segments_json)?
                }
            } else {
                "<div class='suttacentral bilara-text'></div>".to_string()
            }

        } else if let Some(ref html) = sutta.content_html {
            if !html.is_empty() { html.clone() }
            else { "<div class='suttacentral bilara-text'></div>".to_string() }

        } else if let Some(ref plain) = sutta.content_plain {
            if !plain.is_empty() { format!("<div class='suttacentral bilara-text'><pre>{}</pre></div>", plain) }
            else { "<div class='suttacentral bilara-text'></div>".to_string() }

        } else {
            "<div class='suttacentral bilara-text'><p>No content.</p></div>".to_string()
        };

        // Get display settings
        let font_size = app_settings.sutta_font_size;
        let max_width = app_settings.sutta_max_width;

        // Format CSS and JS extras
        let css_extra = format!("html {{ font-size: {}px; }} body {{ max-width: {}ex; }}", font_size, max_width);

        let mut js_extra = format!("const SUTTA_UID = '{}';", sutta.uid);

        if let Some(js_pre) = js_extra_pre {
            js_extra = format!("{}; {}", js_pre, js_extra);
        }

        js_extra.push_str(&format!(" const SHOW_BOOKMARKS = {};", app_settings.show_bookmarks));

        if let Some(quote) = sutta_quote {
            // Escape the quote text for JavaScript string literal
            let escaped_text = quote.quote.replace('\\', "\\\\").replace('"', "\\\"");
            js_extra.push_str(&format!(r#" document.addEventListener("DOMContentLoaded", function(event) {{ highlight_and_scroll_to("{}"); }}); const SHOW_QUOTE = "{}";"#, escaped_text, escaped_text));
        }

        // Build body_class with theme and language
        let mut body_class = self.get_theme_name();

        // Add language-specific class (e.g., lang-he for Hebrew, lang-th for Thai)
        if !sutta.language.is_empty() {
            body_class.push_str(&format!(" lang-{}", sutta.language));
        }

        // Only add navigation if sutta has range information
        let nav_html = if sutta.sutta_range_group.is_some() &&
                          sutta.sutta_range_start.is_some() &&
                          sutta.sutta_range_end.is_some() {
            info(&format!("Sutta {} has range info: group={:?}, start={:?}, end={:?}",
                sutta.uid, sutta.sutta_range_group, sutta.sutta_range_start, sutta.sutta_range_end));

            // Query prev/next suttas to determine navigation button state
            let prev_sutta = self.dbm.appdata.get_prev_sutta(&sutta.uid).ok().flatten();
            let next_sutta = self.dbm.appdata.get_next_sutta(&sutta.uid).ok().flatten();

            let is_first_sutta = prev_sutta.is_none();
            let is_last_sutta = next_sutta.is_none();

            info(&format!("Sutta {} navigation: has_prev={}, has_next={}",
                sutta.uid, !is_first_sutta, !is_last_sutta));

            // Build the navigation HTML by replacing placeholders
            use crate::html_content::PREV_NEXT_CHAPTER_HTML;
            PREV_NEXT_CHAPTER_HTML
                .replace("{current_spine_item_uid}", &sutta.uid)
                .replace("{current_book_uid}", &sutta.uid)  // Use sutta.uid for both since suttas don't have book_uid
                .replace("{is_first_chapter}", &is_first_sutta.to_string())
                .replace("{is_last_chapter}", &is_last_sutta.to_string())
                .replace("{api_url}", &self.api_url)
        } else {
            info(&format!("Sutta {} missing range info - no navigation buttons", sutta.uid));
            // No range information, don't show navigation buttons
            String::new()
        };

        // Wrap content in the full HTML page structure
        use crate::html_content::sutta_html_page_with_nav;
        let final_html = sutta_html_page_with_nav(
            &content_html_body,
            Some(self.api_url.to_string()),
            Some(css_extra.to_string()),
            Some(js_extra.to_string()),
            Some(body_class),
            Some(nav_html),
        );

        Ok(final_html)
    }

    /// Render a book spine item as complete HTML page
    ///
    /// Similar to render_sutta_content, but for book spine items (only for Epub chapters and HTML, PDFs are shown with .url instead of .loadHtml()).
    pub fn render_book_spine_item_html(
        &self,
        spine_item: &BookSpineItem,
        window_id: Option<String>,
        js_extra_pre: Option<String>,
    ) -> Result<String> {
        let app_settings = self.app_settings_cache.read().expect("Failed to read app settings");

        // Get book information to check enable_embedded_css flag
        let book_enable_embedded_css = if let Ok(Some(book)) = self.dbm.appdata.get_book_by_uid(&spine_item.book_uid) {
            book.enable_embedded_css
        } else {
            true // Default to true if book not found
        };

        // Use content_html if available, otherwise fall back to plain text
        let mut content_html_body = if let Some(ref html) = spine_item.content_html {
            if !html.is_empty() {
                html.clone()
            } else {
                "<div class='book-content'><p>No content.</p></div>".to_string()
            }
        } else if let Some(ref plain) = spine_item.content_plain {
            if !plain.is_empty() {
                format!("<div class='book-content'><pre>{}</pre></div>", plain)
            } else {
                "<div class='book-content'><p>No content.</p></div>".to_string()
            }
        } else {
            "<div class='book-content'><p>No content.</p></div>".to_string()
        };

        // Remove embedded CSS if enable_embedded_css is false
        if !book_enable_embedded_css {
            use regex::Regex;
            lazy_static! {
                static ref CSS_LINK_RE: Regex = Regex::new(r"<link[^>]*>").unwrap();
                static ref CSS_STYLE_RE: Regex = Regex::new(r"<style[^>]*>.*?</style>").unwrap();
            }

            content_html_body = CSS_LINK_RE.replace_all(&content_html_body, "").into_owned();
            content_html_body = CSS_STYLE_RE.replace_all(&content_html_body, "").into_owned();
        }

        // Get display settings
        let font_size = app_settings.sutta_font_size;
        let max_width = app_settings.sutta_max_width;

        // Format CSS and JS extras
        let css_extra = format!("html {{ font-size: {}px; }} body {{ max-width: {}ex; }}", font_size, max_width);

        let mut js_extra = format!("const BOOK_SPINE_ITEM_UID = '{}';", spine_item.spine_item_uid);

        if let Some(window_id_value) = window_id {
            js_extra.push_str(&format!(" const WINDOW_ID = '{}'; window.WINDOW_ID = WINDOW_ID;", window_id_value));
        }

        if let Some(js_pre) = js_extra_pre {
            js_extra = format!("{}; {}", js_pre, js_extra);
        }

        js_extra.push_str(&format!(" const SHOW_BOOKMARKS = {};", app_settings.show_bookmarks));

        // Build body_class with theme and language
        let mut body_class = self.get_theme_name();

        // Add language-specific class if available
        if let Some(ref lang) = spine_item.language
            && !lang.is_empty() {
                body_class.push_str(&format!(" lang-{}", lang));
            }

        // Query prev/next spine items to determine navigation button state
        let prev_item = self.dbm.appdata.get_prev_book_spine_item(&spine_item.spine_item_uid).ok().flatten();
        let next_item = self.dbm.appdata.get_next_book_spine_item(&spine_item.spine_item_uid).ok().flatten();

        let is_first_chapter = prev_item.is_none();
        let is_last_chapter = next_item.is_none();

        // Build the navigation HTML by replacing placeholders
        use crate::html_content::PREV_NEXT_CHAPTER_HTML;
        let nav_html = PREV_NEXT_CHAPTER_HTML
            .replace("{current_spine_item_uid}", &spine_item.spine_item_uid)
            .replace("{current_book_uid}", &spine_item.book_uid)
            .replace("{is_first_chapter}", &is_first_chapter.to_string())
            .replace("{is_last_chapter}", &is_last_chapter.to_string())
            .replace("{api_url}", &self.api_url);

        // Wrap content in the full HTML page structure
        use crate::html_content::sutta_html_page_with_nav;
        let final_html = sutta_html_page_with_nav(
            &content_html_body,
            Some(self.api_url.to_string()),
            Some(css_extra.to_string()),
            Some(js_extra.to_string()),
            Some(body_class),
            Some(nav_html),
        );

        Ok(final_html)
    }

    pub fn get_theme_name(&self) -> String {
        let app_settings = self.app_settings_cache.read().expect("Failed to read app settings");
        app_settings.theme_name_as_string()
    }

    pub fn set_theme_name(&self, theme_name: &str) {
        use crate::db::appdata_schema::app_settings;

        let mut app_settings = self.app_settings_cache.write().expect("Failed to write app settings");
        app_settings.set_theme_name_from_str(theme_name);

        let a = app_settings.clone();
        let settings_json = serde_json::to_string(&a).expect("Can't encode JSON");

        let db_conn = &mut self.dbm.userdata.get_conn().expect("Can't get db conn");

        match diesel::update(app_settings::table)
            .filter(app_settings::key.eq("app_settings"))
            .set(app_settings::value.eq(Some(settings_json)))
            .execute(db_conn)
        {
            Ok(_) => {}
            Err(e) => error(&format!("{}", e))
        };
    }

    pub fn set_ai_models_auto_retry(&self, auto_retry: bool) {
        use crate::db::appdata_schema::app_settings;

        let mut app_settings = self.app_settings_cache.write().expect("Failed to write app settings");
        app_settings.ai_models_auto_retry = auto_retry;

        let a = app_settings.clone();
        let settings_json = serde_json::to_string(&a).expect("Can't encode JSON");

        let db_conn = &mut self.dbm.userdata.get_conn().expect("Can't get db conn");

        match diesel::update(app_settings::table)
            .filter(app_settings::key.eq("app_settings"))
            .set(app_settings::value.eq(Some(settings_json)))
            .execute(db_conn)
        {
            Ok(_) => {}
            Err(e) => error(&format!("{}", e))
        };
    }

    pub fn get_api_key(&self, key_name: &str) -> String {
        let app_settings = self.app_settings_cache.read().expect("Failed to read app settings");
        app_settings.api_keys.get(key_name).cloned().unwrap_or_default()
    }

    pub fn set_api_keys(&self, api_keys_json: &str) {
        use crate::db::appdata_schema::app_settings;

        let api_keys_map: IndexMap<String, String> = match serde_json::from_str(api_keys_json) {
            Ok(keys) => keys,
            Err(e) => {
                error(&format!("Failed to parse API keys JSON: {}", e));
                return;
            }
        };

        let mut app_settings = self.app_settings_cache.write().expect("Failed to write app settings");
        app_settings.api_keys = api_keys_map;

        let a = app_settings.clone();
        let settings_json = serde_json::to_string(&a).expect("Can't encode JSON");

        let db_conn = &mut self.dbm.userdata.get_conn().expect("Can't get db conn");

        match diesel::update(app_settings::table)
            .filter(app_settings::key.eq("app_settings"))
            .set(app_settings::value.eq(Some(settings_json)))
            .execute(db_conn)
        {
            Ok(_) => {}
            Err(e) => error(&format!("{}", e))
        };
    }

    pub fn get_system_prompt(&self, prompt_name: &str) -> String {
        let app_settings = self.app_settings_cache.read().expect("Failed to read app settings");
        app_settings.system_prompts.get(prompt_name).cloned().unwrap_or_default()
    }

    pub fn set_system_prompts_json(&self, prompts_json: &str) {
        use crate::db::appdata_schema::app_settings;

        let prompts_map: IndexMap<String, String> = match serde_json::from_str(prompts_json) {
            Ok(prompts) => prompts,
            Err(e) => {
                error(&format!("Failed to parse system prompts JSON: {}", e));
                return;
            }
        };

        let mut app_settings = self.app_settings_cache.write().expect("Failed to write app settings");
        app_settings.system_prompts = prompts_map;

        let a = app_settings.clone();
        let settings_json = serde_json::to_string(&a).expect("Can't encode JSON");

        let db_conn = &mut self.dbm.userdata.get_conn().expect("Can't get db conn");

        match diesel::update(app_settings::table)
            .filter(app_settings::key.eq("app_settings"))
            .set(app_settings::value.eq(Some(settings_json)))
            .execute(db_conn)
        {
            Ok(_) => {}
            Err(e) => error(&format!("{}", e))
        };
    }

    pub fn get_system_prompts_json(&self) -> String {
        let app_settings = self.app_settings_cache.read().expect("Failed to read app settings");
        serde_json::to_string(&app_settings.system_prompts).unwrap_or_default()
    }

    pub fn get_providers_json(&self) -> String {
        let app_settings = self.app_settings_cache.read().expect("Failed to read app settings");
        serde_json::to_string(&app_settings.providers).unwrap_or_default()
    }

    pub fn set_providers_json(&self, providers_json: &str) {
        use crate::db::appdata_schema::app_settings;
        use crate::app_settings::Provider;

        let providers_vec: Vec<Provider> = match serde_json::from_str(providers_json) {
            Ok(providers) => providers,
            Err(e) => {
                error(&format!("Failed to parse providers JSON: {}", e));
                return;
            }
        };

        let mut app_settings = self.app_settings_cache.write().expect("Failed to write app settings");
        app_settings.providers = providers_vec;

        let a = app_settings.clone();
        let settings_json = serde_json::to_string(&a).expect("Can't encode JSON");

        let db_conn = &mut self.dbm.userdata.get_conn().expect("Can't get db conn");

        match diesel::update(app_settings::table)
            .filter(app_settings::key.eq("app_settings"))
            .set(app_settings::value.eq(Some(settings_json)))
            .execute(db_conn)
        {
            Ok(_) => (),
            Err(e) => error(&format!("Failed to update app settings: {}", e)),
        }
    }

    pub fn get_dpd_headword_by_uid(&self, uid_str: &str) -> Option<String> {
        use diesel::prelude::*;
        use crate::db::dpd_schema::dpd_headwords::dsl::*;

        let mut conn = match self.dbm.dpd.get_conn() {
            Ok(c) => c,
            Err(e) => {
                error(&format!("Failed to get DPD connection: {}", e));
                return None;
            }
        };

        let result = dpd_headwords
            .filter(uid.eq(uid_str))
            .first::<crate::db::dpd_models::DpdHeadword>(&mut conn);

        match result {
            Ok(headword) => {
                match serde_json::to_string(&headword) {
                    Ok(json) => Some(json),
                    Err(e) => {
                        error(&format!("Failed to serialize DPD headword: {}", e));
                        None
                    }
                }
            }
            Err(e) => {
                error(&format!("Failed to query DPD headword for uid {}: {}", uid_str, e));
                None
            }
        }
    }

    pub fn get_dpd_root_by_root_key(&self, root_key_str: &str) -> Option<String> {
        use diesel::prelude::*;
        use crate::db::dpd_schema::dpd_roots::dsl::*;

        let mut conn = match self.dbm.dpd.get_conn() {
            Ok(c) => c,
            Err(e) => {
                error(&format!("Failed to get DPD connection: {}", e));
                return None;
            }
        };

        let result = dpd_roots
            .filter(root.eq(root_key_str))
            .first::<crate::db::dpd_models::DpdRoot>(&mut conn);

        match result {
            Ok(dpd_root) => {
                match serde_json::to_string(&dpd_root) {
                    Ok(json) => Some(json),
                    Err(e) => {
                        error(&format!("Failed to serialize DPD root: {}", e));
                        None
                    }
                }
            }
            Err(e) => {
                error(&format!("Failed to query DPD root for root_key {}: {}", root_key_str, e));
                None
            }
        }
    }

    pub fn get_anki_template_front(&self) -> String {
        let app_settings = self.app_settings_cache.read().expect("Failed to read app settings");
        app_settings.anki_template_front.clone()
    }

    pub fn set_anki_template_front(&self, template: &str) {
        use crate::db::appdata_schema::app_settings;

        let mut app_settings = self.app_settings_cache.write().expect("Failed to write app settings");
        app_settings.anki_template_front = template.to_string();

        let a = app_settings.clone();
        let settings_json = serde_json::to_string(&a).expect("Can't encode JSON");

        let db_conn = &mut self.dbm.userdata.get_conn().expect("Can't get db conn");

        match diesel::update(app_settings::table)
            .filter(app_settings::key.eq("app_settings"))
            .set(app_settings::value.eq(Some(settings_json)))
            .execute(db_conn)
        {
            Ok(_) => (),
            Err(e) => error(&format!("Failed to update app settings: {}", e)),
        }
    }

    pub fn get_anki_template_back(&self) -> String {
        let app_settings = self.app_settings_cache.read().expect("Failed to read app settings");
        app_settings.anki_template_back.clone()
    }

    pub fn set_anki_template_back(&self, template: &str) {
        use crate::db::appdata_schema::app_settings;

        let mut app_settings = self.app_settings_cache.write().expect("Failed to write app settings");
        app_settings.anki_template_back = template.to_string();

        let a = app_settings.clone();
        let settings_json = serde_json::to_string(&a).expect("Can't encode JSON");

        let db_conn = &mut self.dbm.userdata.get_conn().expect("Can't get db conn");

        match diesel::update(app_settings::table)
            .filter(app_settings::key.eq("app_settings"))
            .set(app_settings::value.eq(Some(settings_json)))
            .execute(db_conn)
        {
            Ok(_) => (),
            Err(e) => error(&format!("Failed to update app settings: {}", e)),
        }
    }

    pub fn get_anki_template_cloze_front(&self) -> String {
        let app_settings = self.app_settings_cache.read().expect("Failed to read app settings");
        app_settings.anki_template_cloze_front.clone()
    }

    pub fn set_anki_template_cloze_front(&self, template: &str) {
        use crate::db::appdata_schema::app_settings;

        let mut app_settings = self.app_settings_cache.write().expect("Failed to write app settings");
        app_settings.anki_template_cloze_front = template.to_string();

        let a = app_settings.clone();
        let settings_json = serde_json::to_string(&a).expect("Can't encode JSON");

        let db_conn = &mut self.dbm.userdata.get_conn().expect("Can't get db conn");

        match diesel::update(app_settings::table)
            .filter(app_settings::key.eq("app_settings"))
            .set(app_settings::value.eq(Some(settings_json)))
            .execute(db_conn)
        {
            Ok(_) => (),
            Err(e) => error(&format!("Failed to update app settings: {}", e)),
        }
    }

    pub fn get_anki_template_cloze_back(&self) -> String {
        let app_settings = self.app_settings_cache.read().expect("Failed to read app settings");
        app_settings.anki_template_cloze_back.clone()
    }

    pub fn set_anki_template_cloze_back(&self, template: &str) {
        use crate::db::appdata_schema::app_settings;

        let mut app_settings = self.app_settings_cache.write().expect("Failed to write app settings");
        app_settings.anki_template_cloze_back = template.to_string();

        let a = app_settings.clone();
        let settings_json = serde_json::to_string(&a).expect("Can't encode JSON");

        let db_conn = &mut self.dbm.userdata.get_conn().expect("Can't get db conn");

        match diesel::update(app_settings::table)
            .filter(app_settings::key.eq("app_settings"))
            .set(app_settings::value.eq(Some(settings_json)))
            .execute(db_conn)
        {
            Ok(_) => (),
            Err(e) => error(&format!("Failed to update app settings: {}", e)),
        }
    }

    pub fn get_anki_export_format(&self) -> String {
        let app_settings = self.app_settings_cache.read().expect("Failed to read app settings");
        match app_settings.anki_export_format {
            crate::app_settings::AnkiExportFormat::Simple => "Simple".to_string(),
            crate::app_settings::AnkiExportFormat::Templated => "Templated".to_string(),
            crate::app_settings::AnkiExportFormat::DataCsv => "DataCsv".to_string(),
        }
    }

    pub fn set_anki_export_format(&self, format: &str) {
        use crate::db::appdata_schema::app_settings;
        use crate::app_settings::AnkiExportFormat;

        let export_format = match format {
            "Simple" => AnkiExportFormat::Simple,
            "Templated" => AnkiExportFormat::Templated,
            "DataCsv" => AnkiExportFormat::DataCsv,
            _ => {
                error(&format!("Unknown Anki export format: {}", format));
                return;
            }
        };

        let mut app_settings = self.app_settings_cache.write().expect("Failed to write app settings");
        app_settings.anki_export_format = export_format;

        let a = app_settings.clone();
        let settings_json = serde_json::to_string(&a).expect("Can't encode JSON");

        let db_conn = &mut self.dbm.userdata.get_conn().expect("Can't get db conn");

        match diesel::update(app_settings::table)
            .filter(app_settings::key.eq("app_settings"))
            .set(app_settings::value.eq(Some(settings_json)))
            .execute(db_conn)
        {
            Ok(_) => (),
            Err(e) => error(&format!("Failed to update app settings: {}", e)),
        }
    }

    pub fn get_anki_include_cloze(&self) -> bool {
        let app_settings = self.app_settings_cache.read().expect("Failed to read app settings");
        app_settings.anki_include_cloze
    }

    pub fn set_anki_include_cloze(&self, include: bool) {
        use crate::db::appdata_schema::app_settings;

        let mut app_settings = self.app_settings_cache.write().expect("Failed to write app settings");
        app_settings.anki_include_cloze = include;

        let a = app_settings.clone();
        let settings_json = serde_json::to_string(&a).expect("Can't encode JSON");

        let db_conn = &mut self.dbm.userdata.get_conn().expect("Can't get db conn");

        match diesel::update(app_settings::table)
            .filter(app_settings::key.eq("app_settings"))
            .set(app_settings::value.eq(Some(settings_json)))
            .execute(db_conn)
        {
            Ok(_) => (),
            Err(e) => error(&format!("Failed to update app settings: {}", e)),
        }
    }

    pub fn get_search_as_you_type(&self) -> bool {
        let app_settings = self.app_settings_cache.read().expect("Failed to read app settings");
        app_settings.search_as_you_type
    }

    pub fn set_search_as_you_type(&self, enabled: bool) {
        use crate::db::appdata_schema::app_settings;

        let mut app_settings = self.app_settings_cache.write().expect("Failed to write app settings");
        app_settings.search_as_you_type = enabled;

        let a = app_settings.clone();
        let settings_json = serde_json::to_string(&a).expect("Can't encode JSON");

        let db_conn = &mut self.dbm.userdata.get_conn().expect("Can't get db conn");

        match diesel::update(app_settings::table)
            .filter(app_settings::key.eq("app_settings"))
            .set(app_settings::value.eq(Some(settings_json)))
            .execute(db_conn)
        {
            Ok(_) => (),
            Err(e) => error(&format!("Failed to update app settings: {}", e)),
        }
    }

    pub fn get_open_find_in_sutta_results(&self) -> bool {
        let app_settings = self.app_settings_cache.read().expect("Failed to read app settings");
        app_settings.open_find_in_sutta_results
    }

    pub fn set_open_find_in_sutta_results(&self, enabled: bool) {
        use crate::db::appdata_schema::app_settings;

        let mut app_settings = self.app_settings_cache.write().expect("Failed to write app settings");
        app_settings.open_find_in_sutta_results = enabled;

        let a = app_settings.clone();
        let settings_json = serde_json::to_string(&a).expect("Can't encode JSON");

        let db_conn = &mut self.dbm.userdata.get_conn().expect("Can't get db conn");

        match diesel::update(app_settings::table)
            .filter(app_settings::key.eq("app_settings"))
            .set(app_settings::value.eq(Some(settings_json)))
            .execute(db_conn)
        {
            Ok(_) => (),
            Err(e) => error(&format!("Failed to update app settings: {}", e)),
        }
    }

    pub fn set_sutta_language_filter_key(&self, key: String) {
        use crate::db::appdata_schema::app_settings;

        let mut app_settings = self.app_settings_cache.write().expect("Failed to write app settings");
        app_settings.sutta_language_filter_key = key;

        let a = app_settings.clone();
        let settings_json = serde_json::to_string(&a).expect("Can't encode JSON");

        let db_conn = &mut self.dbm.userdata.get_conn().expect("Can't get db conn");

        match diesel::update(app_settings::table)
            .filter(app_settings::key.eq("app_settings"))
            .set(app_settings::value.eq(Some(settings_json)))
            .execute(db_conn)
        {
            Ok(_) => (),
            Err(e) => error(&format!("Failed to update app settings: {}", e)),
        }
    }

    pub fn set_mobile_top_bar_margin_system(&self) {
        use crate::db::appdata_schema::app_settings;

        let mut app_settings = self.app_settings_cache.write().expect("Failed to write app settings");
        app_settings.set_mobile_top_bar_margin_system();

        let a = app_settings.clone();
        let settings_json = serde_json::to_string(&a).expect("Can't encode JSON");

        let db_conn = &mut self.dbm.userdata.get_conn().expect("Can't get db conn");

        match diesel::update(app_settings::table)
            .filter(app_settings::key.eq("app_settings"))
            .set(app_settings::value.eq(Some(settings_json)))
            .execute(db_conn)
        {
            Ok(_) => (),
            Err(e) => error(&format!("Failed to update app settings: {}", e)),
        }
    }

    pub fn set_mobile_top_bar_margin_custom(&self, value: u32) {
        use crate::db::appdata_schema::app_settings;

        let mut app_settings = self.app_settings_cache.write().expect("Failed to write app settings");
        app_settings.set_mobile_top_bar_margin_custom(value);

        let a = app_settings.clone();
        let settings_json = serde_json::to_string(&a).expect("Can't encode JSON");

        let db_conn = &mut self.dbm.userdata.get_conn().expect("Can't get db conn");

        match diesel::update(app_settings::table)
            .filter(app_settings::key.eq("app_settings"))
            .set(app_settings::value.eq(Some(settings_json)))
            .execute(db_conn)
        {
            Ok(_) => (),
            Err(e) => error(&format!("Failed to update app settings: {}", e)),
        }
    }

    /// Import an EPUB document into the database
    pub fn import_epub_to_db(&self, epub_path: &std::path::Path, book_uid: &str, custom_title: Option<&str>, custom_author: Option<&str>, custom_language: Option<&str>, custom_enable_embedded_css: Option<bool>) -> Result<()> {
        let db_conn = &mut self.dbm.appdata.get_conn()
            .context("Failed to get database connection")?;
        crate::epub_import::import_epub_to_db(db_conn, epub_path, book_uid, custom_title, custom_author, custom_language, custom_enable_embedded_css)
    }

    /// Import a PDF document into the database
    pub fn import_pdf_to_db(&self, pdf_path: &std::path::Path, book_uid: &str, custom_title: Option<&str>, custom_author: Option<&str>, custom_language: Option<&str>, custom_enable_embedded_css: Option<bool>) -> Result<()> {
        let db_conn = &mut self.dbm.appdata.get_conn()
            .context("Failed to get database connection")?;
        crate::pdf_import::import_pdf_to_db(db_conn, pdf_path, book_uid, custom_title, custom_author, custom_language, custom_enable_embedded_css)
    }

    /// Import an HTML document into the database
    pub fn import_html_to_db(&self, html_path: &std::path::Path, book_uid: &str, custom_title: Option<&str>, custom_author: Option<&str>, custom_language: Option<&str>, custom_enable_embedded_css: Option<bool>) -> Result<()> {
        let db_conn = &mut self.dbm.appdata.get_conn()
            .context("Failed to get database connection")?;
        crate::html_import::import_html_to_db(db_conn, html_path, book_uid, custom_title, custom_author, custom_language, custom_enable_embedded_css)
    }

    pub fn get_first_time_start(&self) -> bool {
        let app_settings = self.app_settings_cache.read().expect("Failed to read app settings");
        app_settings.first_time_start
    }

    pub fn set_first_time_start(&self, is_first_time: bool) {
        use crate::db::appdata_schema::app_settings;

        let mut app_settings = self.app_settings_cache.write().expect("Failed to write app settings");
        app_settings.first_time_start = is_first_time;

        let a = app_settings.clone();
        let settings_json = serde_json::to_string(&a).expect("Can't encode JSON");

        let db_conn = &mut self.dbm.userdata.get_conn().expect("Can't get db conn");

        match diesel::update(app_settings::table)
            .filter(app_settings::key.eq("app_settings"))
            .set(app_settings::value.eq(Some(settings_json)))
            .execute(db_conn)
        {
            Ok(_) => (),
            Err(e) => error(&format!("Failed to update app settings: {}", e)),
        }
    }

    pub fn check_and_configure_for_first_start(&self) {
        if !self.get_first_time_start() {
            return;
        }

        if let Some(total_memory_gb) = self.get_system_memory_gb()
            && total_memory_gb <= 8 {
                self.set_search_as_you_type(false);
            }

        self.set_first_time_start(false);
    }

    /// Get the database version from the appdata database.
    ///
    /// Queries the `app_settings` table for the 'db_version' key.
    ///
    /// # Returns
    ///
    /// * `Some(String)` - The database version if found
    /// * `None` - If the database doesn't exist or version not found
    pub fn get_db_version(&self) -> Option<String> {
        use crate::db::appdata_schema::app_settings;

        let db_conn = &mut self.dbm.appdata.get_conn().ok()?;

        app_settings::table
            .filter(app_settings::key.eq("db_version"))
            .select(app_settings::value)
            .first::<Option<String>>(db_conn)
            .ok()?
    }

    /// Get the release channel from app settings.
    ///
    /// # Returns
    ///
    /// * `Some(String)` - The release channel if configured
    /// * `None` - If not configured (will default to "simsapa-ng")
    pub fn get_release_channel(&self) -> Option<String> {
        let app_settings = self.app_settings_cache.read().expect("Failed to read app settings");
        app_settings.release_channel.clone()
    }

    /// Get whether to notify about Simsapa updates.
    ///
    /// # Returns
    ///
    /// `true` if update notifications are enabled (default), `false` otherwise
    pub fn get_notify_about_simsapa_updates(&self) -> bool {
        let app_settings = self.app_settings_cache.read().expect("Failed to read app settings");
        app_settings.notify_about_simsapa_updates
    }

    /// Set whether to notify about Simsapa updates.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to show update notifications on startup
    pub fn set_notify_about_simsapa_updates(&self, enabled: bool) {
        use crate::db::appdata_schema::app_settings;

        let mut app_settings = self.app_settings_cache.write().expect("Failed to write app settings");
        app_settings.notify_about_simsapa_updates = enabled;

        let a = app_settings.clone();
        let settings_json = serde_json::to_string(&a).expect("Can't encode JSON");

        let db_conn = &mut self.dbm.userdata.get_conn().expect("Can't get db conn");

        match diesel::update(app_settings::table)
            .filter(app_settings::key.eq("app_settings"))
            .set(app_settings::value.eq(Some(settings_json)))
            .execute(db_conn)
        {
            Ok(_) => (),
            Err(e) => error(&format!("Failed to update app settings: {}", e)),
        }
    }

    fn get_system_memory_gb(&self) -> Option<u64> {
        get_system_memory_bytes().map(|bytes| bytes / 1024 / 1024 / 1024)
    }

    /// Export user data to the import-me folder for database upgrade.
    ///
    /// This creates an "import-me" folder in the simsapa directory and exports:
    /// - app_settings.json: The current application settings
    /// - download_languages.txt: CSV list of languages in the database (except 'san', 'en', 'pli')
    /// - download_select_sanskrit_bundle.txt: If 'san' language is present
    /// - appdata.sqlite3: A database with user-imported books and their related data
    ///
    /// User-imported books are those not in the original dataset.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If export was successful
    /// * `Err` - If any error occurred during export
    pub fn export_user_data_to_assets(&self) -> Result<()> {
        let globals = get_app_globals();
        let app_assets_dir = &globals.paths.app_assets_dir;
        let import_dir = app_assets_dir.join("import-me");

        info(&format!("export_user_data_to_assets(): Creating import-me folder at {}", import_dir.display()));

        // Create import-me folder if it doesn't exist
        if !import_dir.exists() {
            std::fs::create_dir_all(&import_dir)
                .with_context(|| format!("Failed to create import-me directory: {}", import_dir.display()))?;
        }

        // Export app_settings.json to import-me folder
        self.export_app_settings_json(&import_dir)?;

        // Export download_languages.txt and download_select_sanskrit_bundle.txt
        // to app_assets_dir. These are read by DownloadAppdataWindow to auto-fill
        // language selection during database upgrades.
        self.export_download_languages()?;

        // Export user-imported books to import-me folder
        self.export_user_books(&import_dir)?;

        info("export_user_data_to_assets(): Export completed successfully");
        Ok(())
    }

    /// Export app settings to JSON file.
    fn export_app_settings_json(&self, import_dir: &Path) -> Result<()> {
        let app_settings = self.app_settings_cache.read().expect("Failed to read app settings");
        let settings_json = serde_json::to_string_pretty(&*app_settings)
            .context("Failed to serialize app settings to JSON")?;

        let settings_path = import_dir.join("app_settings.json");
        std::fs::write(&settings_path, settings_json)
            .with_context(|| format!("Failed to write app_settings.json to {}", settings_path.display()))?;

        info(&format!("Exported app_settings.json to {}", settings_path.display()));
        Ok(())
    }

    /// Export download languages to marker files.
    ///
    /// Creates download_languages.txt with a CSV list of languages (except 'san', 'en', 'pli').
    /// If 'san' is present, creates download_select_sanskrit_bundle.txt.
    ///
    /// These files are written to app_assets_dir and are read by DownloadAppdataWindow
    /// to auto-fill the language selection during database upgrades.
    fn export_download_languages(&self) -> Result<()> {
        let globals = get_app_globals();
        let languages = self.dbm.get_sutta_languages();

        // Check for Sanskrit bundle
        if languages.contains(&"san".to_string()) {
            let sanskrit_path = &globals.paths.download_select_sanskrit_bundle_marker;
            std::fs::write(sanskrit_path, "True")
                .with_context(|| format!("Failed to write download_select_sanskrit_bundle.txt to {}", sanskrit_path.display()))?;
            info(&format!("Created download_select_sanskrit_bundle.txt at {}", sanskrit_path.display()));
        }

        // Filter out default languages
        let filtered_languages: Vec<String> = languages
            .into_iter()
            .filter(|lang| lang != "san" && lang != "en" && lang != "pli")
            .collect();

        if !filtered_languages.is_empty() {
            let languages_csv = filtered_languages.join(", ");
            let languages_path = &globals.paths.download_languages_marker;
            std::fs::write(languages_path, &languages_csv)
                .with_context(|| format!("Failed to write download_languages.txt to {}", languages_path.display()))?;
            info(&format!("Exported download_languages.txt with: {}", languages_csv));
        }

        Ok(())
    }

    /// Export user-imported books to a new appdata.sqlite3 database.
    ///
    /// User-imported books are those not in the original dataset (identified by their UIDs).
    fn export_user_books(&self, import_dir: &Path) -> Result<()> {
        use crate::db::appdata_schema::{books, book_spine_items, book_resources};
        use crate::db::APPDATA_MIGRATIONS;
        use diesel::sqlite::SqliteConnection;
        use diesel_migrations::MigrationHarness;

        // UIDs of books in the original dataset
        let original_book_uids = vec![
            "buddhadhamma",
            "bmc",
            "cbmc",
            "nibbana-sermons",
            "bhikkhu-manual",
            "the-island",
            "its-essential-meaning",
            "pali-lessons",
            "pali-lessons-answerkey",
            "way-of-meditation",
        ];

        // Get user-imported books (not in original dataset)
        let db_conn = &mut self.dbm.appdata.get_conn()
            .context("Failed to get appdata connection")?;

        let user_books: Vec<Book> = books::table
            .filter(books::uid.ne_all(&original_book_uids))
            .load::<Book>(db_conn)
            .context("Failed to load user books")?;

        if user_books.is_empty() {
            info("No user-imported books to export");
            return Ok(());
        }

        info(&format!("Found {} user-imported books to export", user_books.len()));

        // Create export database
        let export_db_path = import_dir.join("appdata.sqlite3");
        if export_db_path.exists() {
            std::fs::remove_file(&export_db_path)
                .with_context(|| format!("Failed to remove existing export database: {}", export_db_path.display()))?;
        }

        let export_db_url = format!("sqlite://{}", export_db_path.display());
        let mut export_conn = SqliteConnection::establish(&export_db_url)
            .with_context(|| format!("Failed to create export database: {}", export_db_path.display()))?;

        // Create tables using diesel migrations to ensure schema stays in sync
        export_conn.run_pending_migrations(APPDATA_MIGRATIONS)
            .map_err(|e| anyhow!("Failed to run migrations on export database: {}", e))?;

        // Export each user book with its related data
        for book in &user_books {
            // Insert book using diesel model struct
            let new_book = NewBook {
                uid: &book.uid,
                document_type: &book.document_type,
                title: book.title.as_deref(),
                author: book.author.as_deref(),
                language: book.language.as_deref(),
                file_path: book.file_path.as_deref(),
                metadata_json: book.metadata_json.as_deref(),
                enable_embedded_css: book.enable_embedded_css,
                toc_json: book.toc_json.as_deref(),
            };

            diesel::insert_into(books::table)
                .values(&new_book)
                .execute(&mut export_conn)
                .with_context(|| format!("Failed to insert book: {}", book.uid))?;

            // Get the inserted book's ID (it will be different from the source ID)
            let exported_book_id: i32 = books::table
                .filter(books::uid.eq(&book.uid))
                .select(books::id)
                .first(&mut export_conn)
                .with_context(|| format!("Failed to get exported book ID for: {}", book.uid))?;

            // Get and insert spine items for this book
            let spine_items: Vec<BookSpineItem> = book_spine_items::table
                .filter(book_spine_items::book_id.eq(book.id))
                .load::<BookSpineItem>(db_conn)
                .with_context(|| format!("Failed to load spine items for book: {}", book.uid))?;

            for spine_item in &spine_items {
                let new_spine_item = NewBookSpineItem {
                    book_id: exported_book_id,
                    book_uid: &spine_item.book_uid,
                    spine_item_uid: &spine_item.spine_item_uid,
                    spine_index: spine_item.spine_index,
                    resource_path: &spine_item.resource_path,
                    title: spine_item.title.as_deref(),
                    language: spine_item.language.as_deref(),
                    content_html: spine_item.content_html.as_deref(),
                    content_plain: spine_item.content_plain.as_deref(),
                };

                diesel::insert_into(book_spine_items::table)
                    .values(&new_spine_item)
                    .execute(&mut export_conn)
                    .with_context(|| format!("Failed to insert spine item: {}", spine_item.spine_item_uid))?;
            }

            // Get and insert resources for this book
            let resources: Vec<BookResource> = book_resources::table
                .filter(book_resources::book_id.eq(book.id))
                .load::<BookResource>(db_conn)
                .with_context(|| format!("Failed to load resources for book: {}", book.uid))?;

            for resource in &resources {
                let new_resource = NewBookResource {
                    book_id: exported_book_id,
                    book_uid: &resource.book_uid,
                    resource_path: &resource.resource_path,
                    mime_type: resource.mime_type.as_deref(),
                    content_data: resource.content_data.as_deref(),
                };

                diesel::insert_into(book_resources::table)
                    .values(&new_resource)
                    .execute(&mut export_conn)
                    .with_context(|| format!("Failed to insert resource: {}", resource.resource_path))?;
            }

            info(&format!("Exported book: {} with {} spine items and {} resources",
                         book.uid, spine_items.len(), resources.len()));
        }

        info(&format!("Exported {} user books to {}", user_books.len(), export_db_path.display()));
        Ok(())
    }

    /// Import user data from the import-me folder after database upgrade.
    ///
    /// This reads from the "import-me" folder in the simsapa directory and imports:
    /// - app_settings.json: Restores the application settings
    /// - appdata.sqlite3: Imports user books back into the new database
    ///
    /// After successful import, the import-me folder is deleted.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If import was successful (or nothing to import)
    /// * `Err` - If any error occurred during import
    pub fn import_user_data_from_assets(&self) -> Result<()> {
        let globals = get_app_globals();
        let import_dir = globals.paths.app_assets_dir.join("import-me");

        if !import_dir.exists() {
            info("import_user_data_from_assets(): No import-me folder found, skipping import");
            return Ok(());
        }

        info(&format!("import_user_data_from_assets(): Found import-me folder at {}", import_dir.display()));

        // Import app_settings.json
        if let Err(e) = self.import_app_settings_json(&import_dir) {
            error(&format!("Failed to import app settings: {}", e));
            // Continue with other imports even if settings fail
        }

        // Import user books from appdata.sqlite3
        if let Err(e) = self.import_user_books(&import_dir) {
            error(&format!("Failed to import user books: {}", e));
            // Continue with cleanup even if books import fails
        }

        // Clean up: remove the import-me folder
        if let Err(e) = std::fs::remove_dir_all(&import_dir) {
            error(&format!("Failed to remove import-me folder: {}", e));
        } else {
            info("import_user_data_from_assets(): Removed import-me folder after import");
        }

        info("import_user_data_from_assets(): Import completed");
        Ok(())
    }

    /// Import app settings from JSON file.
    fn import_app_settings_json(&self, import_dir: &Path) -> Result<()> {
        use crate::db::appdata_schema::app_settings;

        let settings_path = import_dir.join("app_settings.json");

        if !settings_path.exists() {
            info("No app_settings.json found in import-me folder");
            return Ok(());
        }

        let settings_json = std::fs::read_to_string(&settings_path)
            .with_context(|| format!("Failed to read app_settings.json from {}", settings_path.display()))?;

        let imported_settings: AppSettings = serde_json::from_str(&settings_json)
            .with_context(|| "Failed to parse app_settings.json")?;

        // Update the app settings cache with imported settings
        {
            let mut cache = self.app_settings_cache.write().expect("Failed to write app settings");
            *cache = imported_settings;
        }

        // Save the imported settings to the database
        let cache = self.app_settings_cache.read().expect("Failed to read app settings");
        let serialized_json = serde_json::to_string(&*cache)
            .context("Failed to serialize app settings to JSON")?;

        let db_conn = &mut self.dbm.userdata.get_conn()
            .context("Failed to get userdata connection")?;

        diesel::update(app_settings::table)
            .filter(app_settings::key.eq("app_settings"))
            .set(app_settings::value.eq(Some(serialized_json)))
            .execute(db_conn)
            .context("Failed to update app settings in database")?;

        info(&format!("Imported app settings from {}", settings_path.display()));
        Ok(())
    }

    /// Import user books from the export database.
    ///
    /// Reads books from the import-me/appdata.sqlite3 and inserts them into the new database.
    fn import_user_books(&self, import_dir: &Path) -> Result<()> {
        use crate::db::appdata_schema::{books, book_spine_items, book_resources};
        use diesel::sqlite::SqliteConnection;

        let import_db_path = import_dir.join("appdata.sqlite3");

        if !import_db_path.exists() {
            info("No appdata.sqlite3 found in import-me folder");
            return Ok(());
        }

        info(&format!("Importing user books from {}", import_db_path.display()));

        // Open the import database
        let import_db_url = format!("sqlite://{}", import_db_path.display());
        let mut import_conn = SqliteConnection::establish(&import_db_url)
            .with_context(|| format!("Failed to open import database: {}", import_db_path.display()))?;

        // Get all books from the import database
        let import_books: Vec<Book> = books::table
            .load::<Book>(&mut import_conn)
            .context("Failed to load books from import database")?;

        if import_books.is_empty() {
            info("No books to import from appdata.sqlite3");
            return Ok(());
        }

        info(&format!("Found {} books to import", import_books.len()));

        // Get connection to the current appdata database
        let db_conn = &mut self.dbm.appdata.get_conn()
            .context("Failed to get appdata connection")?;

        // Import each book with its related data
        for book in &import_books {
            // Check if book already exists in the current database (by UID)
            let existing_book: Option<i32> = books::table
                .filter(books::uid.eq(&book.uid))
                .select(books::id)
                .first(db_conn)
                .optional()
                .context("Failed to check for existing book")?;

            if existing_book.is_some() {
                info(&format!("Book {} already exists, skipping", book.uid));
                continue;
            }

            // Insert the book
            let new_book = NewBook {
                uid: &book.uid,
                document_type: &book.document_type,
                title: book.title.as_deref(),
                author: book.author.as_deref(),
                language: book.language.as_deref(),
                file_path: book.file_path.as_deref(),
                metadata_json: book.metadata_json.as_deref(),
                enable_embedded_css: book.enable_embedded_css,
                toc_json: book.toc_json.as_deref(),
            };

            diesel::insert_into(books::table)
                .values(&new_book)
                .execute(db_conn)
                .with_context(|| format!("Failed to insert book: {}", book.uid))?;

            // Get the inserted book's ID
            let new_book_id: i32 = books::table
                .filter(books::uid.eq(&book.uid))
                .select(books::id)
                .first(db_conn)
                .with_context(|| format!("Failed to get new book ID for: {}", book.uid))?;

            // Get and insert spine items from import database
            let spine_items: Vec<BookSpineItem> = book_spine_items::table
                .filter(book_spine_items::book_id.eq(book.id))
                .load::<BookSpineItem>(&mut import_conn)
                .with_context(|| format!("Failed to load spine items for book: {}", book.uid))?;

            for spine_item in &spine_items {
                let new_spine_item = NewBookSpineItem {
                    book_id: new_book_id,
                    book_uid: &spine_item.book_uid,
                    spine_item_uid: &spine_item.spine_item_uid,
                    spine_index: spine_item.spine_index,
                    resource_path: &spine_item.resource_path,
                    title: spine_item.title.as_deref(),
                    language: spine_item.language.as_deref(),
                    content_html: spine_item.content_html.as_deref(),
                    content_plain: spine_item.content_plain.as_deref(),
                };

                diesel::insert_into(book_spine_items::table)
                    .values(&new_spine_item)
                    .execute(db_conn)
                    .with_context(|| format!("Failed to insert spine item: {}", spine_item.spine_item_uid))?;
            }

            // Get and insert resources from import database
            let resources: Vec<BookResource> = book_resources::table
                .filter(book_resources::book_id.eq(book.id))
                .load::<BookResource>(&mut import_conn)
                .with_context(|| format!("Failed to load resources for book: {}", book.uid))?;

            for resource in &resources {
                let new_resource = NewBookResource {
                    book_id: new_book_id,
                    book_uid: &resource.book_uid,
                    resource_path: &resource.resource_path,
                    mime_type: resource.mime_type.as_deref(),
                    content_data: resource.content_data.as_deref(),
                };

                diesel::insert_into(book_resources::table)
                    .values(&new_resource)
                    .execute(db_conn)
                    .with_context(|| format!("Failed to insert resource: {}", resource.resource_path))?;
            }

            info(&format!("Imported book: {} with {} spine items and {} resources",
                         book.uid, spine_items.len(), resources.len()));
        }

        info(&format!("Successfully imported {} user books", import_books.len()));
        Ok(())
    }
}

impl Default for AppData {
    fn default() -> Self {
        Self::new()
    }
}

/// Get the total system memory in bytes.
///
/// NOTE: Cannot use sysinfo::System because it requires higher Android API levels.
/// Therefore, we use platform-specific implementations.
pub fn get_system_memory_bytes() -> Option<u64> {
    #[cfg(target_os = "android")]
    {
        get_android_memory_bytes()
    }

    #[cfg(target_os = "linux")]
    {
        get_linux_memory_bytes()
    }

    #[cfg(target_os = "macos")]
    {
        get_macos_memory_bytes()
    }

    #[cfg(target_os = "ios")]
    {
        get_ios_memory_bytes()
    }

    #[cfg(target_os = "windows")]
    {
        get_windows_memory_bytes()
    }
}

/// Get the number of CPU cores.
///
/// NOTE: Cannot use sysinfo::System because it requires higher Android API levels.
/// Therefore, we use platform-specific implementations.
pub fn get_cpu_cores() -> Option<u32> {
    #[cfg(any(target_os = "android", target_os = "linux"))]
    {
        get_linux_cpu_cores()
    }

    #[cfg(target_os = "macos")]
    {
        get_macos_cpu_cores()
    }

    #[cfg(target_os = "ios")]
    {
        get_ios_cpu_cores()
    }

    #[cfg(target_os = "windows")]
    {
        get_windows_cpu_cores()
    }
}

/// Get the maximum CPU frequency in MHz.
///
/// NOTE: Cannot use sysinfo::System because it requires higher Android API levels.
/// Therefore, we use platform-specific implementations.
pub fn get_cpu_max_frequency_mhz() -> Option<u64> {
    #[cfg(any(target_os = "android", target_os = "linux"))]
    {
        get_linux_cpu_max_frequency_mhz()
    }

    #[cfg(target_os = "macos")]
    {
        get_macos_cpu_max_frequency_mhz()
    }

    #[cfg(target_os = "ios")]
    {
        get_ios_cpu_max_frequency_mhz()
    }

    #[cfg(target_os = "windows")]
    {
        get_windows_cpu_max_frequency_mhz()
    }
}

#[cfg(target_os = "android")]
fn get_android_memory_bytes() -> Option<u64> {
    use std::fs;

    // Read /proc/meminfo which is available on Android
    let meminfo = fs::read_to_string("/proc/meminfo").ok()?;

    for line in meminfo.lines() {
        if line.starts_with("MemTotal:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let kb = parts[1].parse::<u64>().ok()?;
                return Some(kb * 1024); // Convert KB to bytes
            }
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn get_linux_memory_bytes() -> Option<u64> {
    use std::fs;

    let meminfo = fs::read_to_string("/proc/meminfo").ok()?;

    for line in meminfo.lines() {
        if line.starts_with("MemTotal:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let kb = parts[1].parse::<u64>().ok()?;
                return Some(kb * 1024); // Convert KB to bytes
            }
        }
    }
    None
}

#[cfg(target_os = "macos")]
fn get_macos_memory_bytes() -> Option<u64> {
    use std::process::Command;

    let output = Command::new("sysctl")
        .arg("-n")
        .arg("hw.memsize")
        .output()
        .ok()?;

    let bytes_str = String::from_utf8(output.stdout).ok()?;
    bytes_str.trim().parse::<u64>().ok()
}

#[cfg(target_os = "windows")]
fn get_windows_memory_bytes() -> Option<u64> {
    use std::mem;

    #[repr(C)]
    struct MEMORYSTATUSEX {
        dw_length: u32,
        dw_memory_load: u32,
        ull_total_phys: u64,
        ull_avail_phys: u64,
        ull_total_page_file: u64,
        ull_avail_page_file: u64,
        ull_total_virtual: u64,
        ull_avail_virtual: u64,
        ull_avail_extended_virtual: u64,
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GlobalMemoryStatusEx(lpbuffer: *mut MEMORYSTATUSEX) -> i32;
    }

    unsafe {
        let mut status: MEMORYSTATUSEX = mem::zeroed();
        status.dw_length = mem::size_of::<MEMORYSTATUSEX>() as u32;

        if GlobalMemoryStatusEx(&mut status) != 0 {
            Some(status.ull_total_phys)
        } else {
            None
        }
    }
}

// CPU cores implementations

#[cfg(any(target_os = "android", target_os = "linux"))]
fn get_linux_cpu_cores() -> Option<u32> {
    use std::fs;

    // Try reading from /proc/cpuinfo
    if let Ok(cpuinfo) = fs::read_to_string("/proc/cpuinfo") {
        let count = cpuinfo.lines()
            .filter(|line| line.starts_with("processor"))
            .count();
        if count > 0 {
            return Some(count as u32);
        }
    }

    // Fallback: try reading from /sys/devices/system/cpu/present
    if let Ok(present) = fs::read_to_string("/sys/devices/system/cpu/present") {
        // Format is like "0-7" for 8 cores
        let trimmed = present.trim();
        if let Some(pos) = trimmed.rfind('-')
            && let Ok(max) = trimmed[pos + 1..].parse::<u32>() {
                return Some(max + 1);
            }
    }

    None
}

#[cfg(target_os = "macos")]
fn get_macos_cpu_cores() -> Option<u32> {
    use std::process::Command;

    let output = Command::new("sysctl")
        .arg("-n")
        .arg("hw.ncpu")
        .output()
        .ok()?;

    let cores_str = String::from_utf8(output.stdout).ok()?;
    cores_str.trim().parse::<u32>().ok()
}

#[cfg(target_os = "windows")]
fn get_windows_cpu_cores() -> Option<u32> {
    use std::env;

    // Use NUMBER_OF_PROCESSORS environment variable
    env::var("NUMBER_OF_PROCESSORS")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
}

// CPU frequency implementations

#[cfg(any(target_os = "android", target_os = "linux"))]
fn get_linux_cpu_max_frequency_mhz() -> Option<u64> {
    use std::fs;

    // Try reading from scaling_max_freq (in KHz)
    if let Ok(freq_str) = fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/scaling_max_freq")
        && let Ok(khz) = freq_str.trim().parse::<u64>() {
            return Some(khz / 1000); // Convert KHz to MHz
        }

    // Fallback: try cpuinfo_max_freq
    if let Ok(freq_str) = fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_max_freq")
        && let Ok(khz) = freq_str.trim().parse::<u64>() {
            return Some(khz / 1000);
        }

    // Fallback: try parsing /proc/cpuinfo for "cpu MHz"
    if let Ok(cpuinfo) = fs::read_to_string("/proc/cpuinfo") {
        for line in cpuinfo.lines() {
            if line.starts_with("cpu MHz")
                && let Some(pos) = line.find(':') {
                    let mhz_str = line[pos + 1..].trim();
                    if let Ok(mhz) = mhz_str.parse::<f64>() {
                        return Some(mhz as u64);
                    }
                }
        }
    }

    None
}

#[cfg(target_os = "macos")]
fn get_macos_cpu_max_frequency_mhz() -> Option<u64> {
    use std::process::Command;

    // sysctl hw.cpufrequency returns frequency in Hz
    let output = Command::new("sysctl")
        .arg("-n")
        .arg("hw.cpufrequency")
        .output()
        .ok()?;

    let hz_str = String::from_utf8(output.stdout).ok()?;
    let hz = hz_str.trim().parse::<u64>().ok()?;
    Some(hz / 1_000_000) // Convert Hz to MHz
}

#[cfg(target_os = "windows")]
fn get_windows_cpu_max_frequency_mhz() -> Option<u64> {
    use std::process::Command;

    // Use WMIC to get max clock speed
    let output = Command::new("wmic")
        .args(["cpu", "get", "MaxClockSpeed", "/value"])
        .output()
        .ok()?;

    let output_str = String::from_utf8(output.stdout).ok()?;
    for line in output_str.lines() {
        if line.starts_with("MaxClockSpeed=") {
            let mhz_str = line.trim_start_matches("MaxClockSpeed=");
            return mhz_str.trim().parse::<u64>().ok();
        }
    }

    None
}

// iOS implementations using sysctlbyname FFI
// Note: iOS uses the same sysctl API as macOS but we can't spawn processes,
// so we use direct FFI calls to sysctlbyname.

#[cfg(target_os = "ios")]
fn get_ios_memory_bytes() -> Option<u64> {
    use std::ffi::CStr;
    use std::mem;
    use std::ptr;

    #[link(name = "System")]
    unsafe extern "C" {
        fn sysctlbyname(
            name: *const i8,
            oldp: *mut std::ffi::c_void,
            oldlenp: *mut usize,
            newp: *mut std::ffi::c_void,
            newlen: usize,
        ) -> i32;
    }

    let name = CStr::from_bytes_with_nul(b"hw.memsize\0").ok()?;
    let mut value: u64 = 0;
    let mut size = mem::size_of::<u64>();

    unsafe {
        let result = sysctlbyname(
            name.as_ptr(),
            &mut value as *mut u64 as *mut std::ffi::c_void,
            &mut size,
            ptr::null_mut(),
            0,
        );

        if result == 0 {
            Some(value)
        } else {
            None
        }
    }
}

#[cfg(target_os = "ios")]
fn get_ios_cpu_cores() -> Option<u32> {
    use std::ffi::CStr;
    use std::mem;
    use std::ptr;

    #[link(name = "System")]
    unsafe extern "C" {
        fn sysctlbyname(
            name: *const i8,
            oldp: *mut std::ffi::c_void,
            oldlenp: *mut usize,
            newp: *mut std::ffi::c_void,
            newlen: usize,
        ) -> i32;
    }

    // Try hw.ncpu first (total CPUs including logical)
    let name = CStr::from_bytes_with_nul(b"hw.ncpu\0").ok()?;
    let mut value: i32 = 0;
    let mut size = mem::size_of::<i32>();

    unsafe {
        let result = sysctlbyname(
            name.as_ptr(),
            &mut value as *mut i32 as *mut std::ffi::c_void,
            &mut size,
            ptr::null_mut(),
            0,
        );

        if result == 0 && value > 0 {
            return Some(value as u32);
        }
    }

    // Fallback to hw.physicalcpu
    let name = CStr::from_bytes_with_nul(b"hw.physicalcpu\0").ok()?;
    let mut value: i32 = 0;
    let mut size = mem::size_of::<i32>();

    unsafe {
        let result = sysctlbyname(
            name.as_ptr(),
            &mut value as *mut i32 as *mut std::ffi::c_void,
            &mut size,
            ptr::null_mut(),
            0,
        );

        if result == 0 && value > 0 {
            Some(value as u32)
        } else {
            None
        }
    }
}

#[cfg(target_os = "ios")]
fn get_ios_cpu_max_frequency_mhz() -> Option<u64> {
    use std::ffi::CStr;
    use std::mem;
    use std::ptr;

    #[link(name = "System")]
    unsafe extern "C" {
        fn sysctlbyname(
            name: *const i8,
            oldp: *mut std::ffi::c_void,
            oldlenp: *mut usize,
            newp: *mut std::ffi::c_void,
            newlen: usize,
        ) -> i32;
    }

    // Try hw.cpufrequency (returns Hz)
    let name = CStr::from_bytes_with_nul(b"hw.cpufrequency\0").ok()?;
    let mut value: u64 = 0;
    let mut size = mem::size_of::<u64>();

    unsafe {
        let result = sysctlbyname(
            name.as_ptr(),
            &mut value as *mut u64 as *mut std::ffi::c_void,
            &mut size,
            ptr::null_mut(),
            0,
        );

        if result == 0 && value > 0 {
            return Some(value / 1_000_000); // Convert Hz to MHz
        }
    }

    // Fallback to hw.cpufrequency_max
    let name = CStr::from_bytes_with_nul(b"hw.cpufrequency_max\0").ok()?;
    let mut value: u64 = 0;
    let mut size = mem::size_of::<u64>();

    unsafe {
        let result = sysctlbyname(
            name.as_ptr(),
            &mut value as *mut u64 as *mut std::ffi::c_void,
            &mut size,
            ptr::null_mut(),
            0,
        );

        if result == 0 && value > 0 {
            Some(value / 1_000_000) // Convert Hz to MHz
        } else {
            // On modern iOS devices, CPU frequency info may not be available via sysctl
            // Return a reasonable default for modern iOS devices
            Some(2400) // 2.4 GHz - typical for modern iPhones
        }
    }
}
