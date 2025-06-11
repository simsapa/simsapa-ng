use crate::db::appdata_models::*;
use crate::db::appdata_schema::suttas::dsl::*;
use diesel::prelude::*;

use std::collections::{BTreeMap, HashMap};
use serde::Deserialize;
use serde_json::Value;
use regex::Regex;
use anyhow::{anyhow, Context, Result};

use crate::db::DbConn;
use crate::helpers::bilara_text_to_segments;

// Represents the application data and settings
pub struct AppData {
    pub db_conn: DbConn,
    // App settings can be HashMap, key order is not critical
    pub app_settings: HashMap<String, Value>,
    pub api_url: String,
}

impl AppData {
    pub fn new(appdata_db_conn: DbConn, app_settings: HashMap<String, Value>, api_url: String) -> Self {
        AppData {
            db_conn: appdata_db_conn,
            app_settings,
            api_url,
        }
    }

    /// Fetches the corresponding Pali sutta for a translated sutta.
    pub fn get_pali_for_translated(&mut self, sutta: &Sutta) -> Result<Option<Sutta>> {
        if sutta.language == "pli" {
            return Ok(None);
        }

        // Use regex to extract the base UID part (e.g., "mn1" from "mn1/en/bodhi")
        let re = Regex::new("^([^/]+)/.*").expect("Invalid regex");
        let uid_ref = re.replace(&sutta.uid, "$1").to_string();

        let res = suttas
            .select(Sutta::as_select())
            .filter(uid.ne(&sutta.uid))
            .filter(language.eq("pli"))
            .filter(uid.like(format!("{}/%", uid_ref)))
            .first(&mut self.db_conn)
            .optional() // Makes it return Result<Option<USutta>> instead of erroring if not found
            .context("Database query failed for Pali sutta")?;

        Ok(res)
    }

    /// Converts sutta data into a BTreeMap of segments, potentially including variants, comments, glosses.
    /// Returns BTreeMap to preserve order.
    pub fn sutta_to_segments_json(
        &mut self,
        sutta: &Sutta,
        use_template: bool,
    ) -> Result<BTreeMap<String, String>> {
        use crate::db::appdata_schema::{sutta_variants, sutta_comments, sutta_glosses};

        let variant_record = sutta_variants::table
            .filter(sutta_variants::sutta_uid.eq(&sutta.uid))
            .select(SuttaVariant::as_select())
            .first::<SuttaVariant>(&mut self.db_conn)
            .optional()
            .context("Database query failed for SuttaVariant")?;
        // Extract the content_json string if the record was found
        let variant_json_str: Option<String> = variant_record.and_then(|v| v.content_json);

        let comment_record = sutta_comments::table
            .filter(sutta_comments::sutta_uid.eq(&sutta.uid))
            .select(SuttaComment::as_select())
            .first::<SuttaComment>(&mut self.db_conn)
            .optional()
            .context("Database query failed for SuttaComment")?;
        let comment_json_str: Option<String> = comment_record.and_then(|c| c.content_json);

        let gloss_record = sutta_glosses::table
            .filter(sutta_glosses::sutta_uid.eq(&sutta.uid))
            .select(SuttaGloss::as_select())
            .first::<SuttaGloss>(&mut self.db_conn)
            .optional()
            .context("Database query failed for SuttaGloss")?;
        let gloss_json_str: Option<String> = gloss_record.and_then(|g| g.content_json);

        // Get settings, providing default values
        let show_variants = self.get_setting_or("show_all_variant_readings", false);
        let show_glosses = self.get_setting_or("show_glosses", false);

        let tmpl_str = if use_template {
            sutta.content_json_tmpl.as_deref()
        } else {
            None
        };

        let content_str = sutta.content_json.as_deref()
            .ok_or_else(|| anyhow!("Sutta {} is missing content_json", sutta.uid))?;

        bilara_text_to_segments(
            content_str,
            tmpl_str,
            variant_json_str.as_deref(),
            comment_json_str.as_deref(),
            gloss_json_str.as_deref(),
            show_variants,
            show_glosses,
        )
    }

    // Helper to get a setting value or default
    pub fn get_setting_or<T: Default + Clone>(&self, key: &str, default: T) -> T
    where
        T: for<'de> Deserialize<'de>,
    {
         self.app_settings.get(key)
             .and_then(|v| serde_json::from_value(v.clone()).ok())
             .unwrap_or(default)
    }

    pub fn get_theme_name(&self) -> Option<String> {
        // FIXME return value from db lookup
        // "system".to_string()
        // "light".to_string()
        Some("dark".to_string())
    }
}
