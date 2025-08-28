use std::sync::RwLock;
use std::collections::BTreeMap;

use diesel::prelude::*;
use regex::Regex;
use anyhow::{anyhow, Context, Result};

use crate::db::{appdata_models::*, DbManager};
use crate::db::appdata_schema::suttas::dsl::*;

use crate::logger::error;
use crate::types::SuttaQuote;
use crate::app_settings::AppSettings;
use crate::helpers::{bilara_text_to_segments, bilara_line_by_line_html, bilara_content_json_to_html};
use crate::html_content::sutta_html_page;
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

    /// Converts sutta data into a BTreeMap of segments, potentially including variants, comments, glosses.
    /// Returns BTreeMap to preserve order.
    pub fn sutta_to_segments_json(
        &self,
        sutta: &Sutta,
        use_template: bool,
    ) -> Result<BTreeMap<String, String>> {
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

                if line_by_line && pali_sutta.is_some() {
                    // Generate line-by-line HTML
                    let pali_sutta = pali_sutta.unwrap();
                    let translated_segments = self.sutta_to_segments_json(sutta, false)
                                                      .context("Failed to generate translated segments for line-by-line view")?;
                    let pali_segments = self.sutta_to_segments_json(&pali_sutta, false)
                                                .context("Failed to generate Pali segments for line-by-line view")?;

                    let tmpl_str = sutta.content_json_tmpl.as_deref()
                                                          .ok_or_else(|| anyhow!("Sutta {} requires content_json_tmpl for line-by-line view", sutta.uid))?;
                    // Parse template into BTreeMap as well
                    let tmpl_json: BTreeMap<String, String> = serde_json::from_str(tmpl_str)
                        .with_context(|| format!("Failed to parse template JSON into BTreeMap for line-by-line view (Sutta: {})", sutta.uid))?;

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

        // Wrap content in the full HTML page structure
        let final_html = sutta_html_page(
            &content_html_body,
            Some(self.api_url.to_string()),
            Some(css_extra.to_string()),
            Some(js_extra.to_string()),
            Some(self.get_theme_name()),
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

    pub fn get_api_key(&self, key_name: &str) -> String {
        let app_settings = self.app_settings_cache.read().expect("Failed to read app settings");
        app_settings.api_keys.get(key_name).cloned().unwrap_or_default()
    }

    pub fn set_api_keys(&self, api_keys_json: &str) {
        use crate::db::appdata_schema::app_settings;

        let api_keys_map: BTreeMap<String, String> = match serde_json::from_str(api_keys_json) {
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

        let prompts_map: BTreeMap<String, String> = match serde_json::from_str(prompts_json) {
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

    pub fn get_models_json(&self) -> String {
        let app_settings = self.app_settings_cache.read().expect("Failed to read app settings");
        serde_json::to_string(&app_settings.models).unwrap_or_default()
    }

    pub fn set_models_json(&self, models_json: &str) {
        use crate::db::appdata_schema::app_settings;
        use crate::app_settings::ModelEntry;

        let models_vec: Vec<ModelEntry> = match serde_json::from_str(models_json) {
            Ok(models) => models,
            Err(e) => {
                error(&format!("Failed to parse models JSON: {}", e));
                return;
            }
        };

        let mut app_settings = self.app_settings_cache.write().expect("Failed to write app settings");
        app_settings.models = models_vec;

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
}
