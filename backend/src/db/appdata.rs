use diesel::prelude::*;
use regex::Regex;
use anyhow::Result;
use serde::Serialize;

use crate::get_app_data;
use crate::db::appdata_models::*;
use crate::db::DatabaseHandle;
use crate::app_settings::AppSettings;
use crate::logger::{info, error};

static COMMON_WORDS_JSON: &'static str = include_str!("../../../assets/common-words.json");

pub type AppdataDbHandle = DatabaseHandle;

impl AppdataDbHandle {
    /// Get distinct sutta languages from the database
    pub fn get_sutta_languages(&self) -> Vec<String> {
        use crate::db::appdata_schema::suttas::dsl::*;

        let result = self.do_read(|db_conn| {
            suttas
                .select(language)
                .distinct()
                .load::<String>(db_conn)
        });

        match result {
            Ok(mut langs) => {
                // Filter out empty strings, convert to lowercase, and deduplicate
                langs.sort();
                let mut seen = std::collections::HashSet::new();
                let mut unique_langs: Vec<String> = Vec::new();

                for lang in langs {
                    if !lang.is_empty() {
                        let lowercase_lang = lang.to_lowercase();
                        if seen.insert(lowercase_lang.clone()) {
                            unique_langs.push(lowercase_lang);
                        }
                    }
                }

                // Sort again to ensure consistent alphabetical order
                unique_langs.sort();
                unique_langs
            },
            Err(e) => {
                error(&format!("get_sutta_languages(): {}", e));
                Vec::new()
            }
        }
    }

    pub fn get_sutta(&self, sutta_uid: &str) -> Option<Sutta> {
        use crate::db::appdata_schema::suttas::dsl::*;

        let sutta = self.do_read(|db_conn| {
            suttas
                .filter(uid.eq(sutta_uid))
                .select(Sutta::as_select())
                .first(db_conn)
                .optional()
        });

        match sutta {
            Ok(x) => x,
            Err(e) => {
                error(&format!("{}", e));
                None
            },
        }
    }

    pub fn get_translations_data_json_for_sutta_uid(&self, sutta_uid: &str) -> String {
        // See sutta_search_window_state.py::_add_related_tabs()

        // Capture the reference before the first '/'
        let re = Regex::new(r"^([^/]+)/.*").expect("Invalid regex");
        let uid_ref = re.replace(&sutta_uid, "$1").to_string();

        use crate::db::appdata_schema::suttas::dsl::*;

        let app_data = get_app_data();
        let _lock = app_data.dbm.appdata.write_lock.lock();
        let mut db_conn = app_data.dbm.appdata.get_conn().expect("get_translations_data_json_for_sutta_uid(): No appdata conn");

        let mut res: Vec<Sutta> = Vec::new();

        // Find suttas with the same reference code, including atthakatha (.att) and tika (.tik).
        if let Ok(a) = suttas
            .select(Sutta::as_select())
            .filter(uid.ne(sutta_uid))
            .filter(
                uid.like(format!("{}/%", uid_ref))
                   .or(uid.like(format!("{}.att/%", uid_ref)))
                   .or(uid.like(format!("{}.tik/%", uid_ref)))
            )
            .load(&mut db_conn) {
                res.extend(a);
            }

        #[derive(Serialize)]
        struct TranslationData {
            item_uid: String,
            sutta_title: String,
            sutta_ref: String,
        }

        let res_sorted_data: Vec<TranslationData> = sort_suttas(res)
            .into_iter().map(|s| TranslationData {
                item_uid: s.uid,
                sutta_title: s.title.unwrap_or("".to_string()),
                sutta_ref: s.sutta_ref,
            }).collect();

        serde_json::to_string(&res_sorted_data).expect("Can't encode JSON")
    }

    pub fn get_app_settings(&self) -> AppSettings {
        use crate::db::appdata_schema::app_settings::dsl::*;

        let json = self.do_read(|db_conn| {
            app_settings
                .filter(key.eq("app_settings"))
                .select(AppSetting::as_select())
                .first(db_conn)
                .optional()
        });

        match json {
            Ok(None) => AppSettings::default(),
            Ok(Some(setting)) => {
                setting.value
                       .map(|val| serde_json::from_str(&val).expect("Can't decode JSON"))
                       .unwrap_or_default()
            },
            Err(e) => {
                error(&format!("{}", e));
                AppSettings::default()
            }
        }
    }

    pub fn get_common_words_json(&self) -> String {
        use crate::db::appdata_schema::app_settings::dsl::*;

        let json = self.do_read(|db_conn| {
            app_settings
                .filter(key.eq("common_words_json"))
                .select(AppSetting::as_select())
                .first(db_conn)
                .optional()
        });

        match json {
            Ok(None) => String::from(COMMON_WORDS_JSON),
            Ok(Some(setting)) => {
                setting.value.unwrap_or(String::from(COMMON_WORDS_JSON))
            }
            Err(e) => {
                error(&format!("{}", e));
                String::from(COMMON_WORDS_JSON)
            }
        }
    }

    pub fn save_common_words_json(&self, words_json: &str) -> Result<usize> {
        use crate::db::appdata_schema::app_settings::dsl::*;

        self.do_write(|db_conn| {
            let existing_setting = app_settings
                .filter(key.eq("common_words_json"))
                .first::<AppSetting>(db_conn)
                .optional()?;

            match existing_setting {
                Some(setting) => {
                    diesel::update(app_settings.find(setting.id))
                        .set(value.eq(Some(words_json)))
                        .execute(db_conn)
                }
                None => {
                    let new_setting = NewAppSetting {
                        key: "common_words_json",
                        value: Some(words_json),
                    };

                    diesel::insert_into(app_settings)
                        .values(&new_setting)
                        .execute(db_conn)
                }
            }
        })
    }
}

pub fn delete_sutta() {
    use crate::db::appdata_schema::suttas::dsl::*;

    let pattern = "unwholesome";

    let app_data = get_app_data();
    let _lock = app_data.dbm.appdata.write_lock.lock();
    let db_conn = &mut app_data.dbm.appdata.get_conn().expect("Can't get db conn");

    let num_deleted = diesel::delete(suttas.filter(content_html.like(pattern)))
        .execute(db_conn)
        .expect("Error deleting suttas");

    info(&format!("Deleted {} suttas", num_deleted));
}

fn sort_suttas(res: Vec<Sutta>) -> Vec<Sutta> {
    // Sort Pali ms first as the results.
    // Then add Pali other sources,
    // then the non-Pali items, sorted by language.
    //
    // Single-pass manual bucketing means we walk the vector once,
    // avoiding per-element cloning.

    let mut results = Vec::new();
    let mut pli_others = Vec::new();
    let mut remaining = Vec::new();

    for s in res.into_iter() {
        if s.language == "pli" {
            if s.uid.ends_with("/ms") {
                results.push(s);
            } else {
                pli_others.push(s);
            }
        } else {
            remaining.push(s);
        }
    }

    // Sort non-pli by language
    remaining.sort_by(|a, b| a.language.cmp(&b.language));
    // Assemble final list
    results.extend(pli_others);
    results.extend(remaining);
    results
}

impl AppdataDbHandle {
    /// Remove suttas and related data for specific language codes
    /// Returns true if deletion was successful
    /// The progress_callback is called after each language is removed with (current_index, total_count, language_code)
    pub fn remove_sutta_languages<F>(&self, language_codes: Vec<String>, mut progress_callback: F) -> Result<bool>
    where
        F: FnMut(usize, usize, &str),
    {
        use crate::db::appdata_schema;

        if language_codes.is_empty() {
            return Ok(true);
        }

        info(&format!("remove_sutta_languages(): Removing languages: {:?}", language_codes));

        let total_count = language_codes.len();
        let mut any_deleted = false;

        // Process each language one by one to provide progress updates
        for (index, lang_code) in language_codes.iter().enumerate() {
            let current_index = index + 1;
            info(&format!("Removing language {}/{}: {}", current_index, total_count, lang_code));

            // Call progress callback BEFORE starting to remove this language
            progress_callback(current_index, total_count, lang_code);

            let result = self.do_write(|db_conn| {
                // Delete suttas for this language
                // SQLite automatically handles CASCADE DELETE for child tables
                // (sutta_variants, sutta_comments, sutta_glosses) because:
                // 1. Foreign keys have ON DELETE CASCADE in the schema
                // 2. Foreign keys are enabled via PRAGMA foreign_keys = ON (see ConnectionCustomizer)
                // 3. Diesel's delete() executes standard SQL DELETE which respects CASCADE
                let suttas_deleted = diesel::delete(
                    appdata_schema::suttas::table
                        .filter(appdata_schema::suttas::language.eq(lang_code))
                ).execute(db_conn)?;

                info(&format!("Deleted {} suttas for language {} (child records deleted via CASCADE)", suttas_deleted, lang_code));

                Ok(suttas_deleted > 0)
            });

            match result {
                Ok(deleted) => {
                    if deleted {
                        any_deleted = true;
                    }
                    info(&format!("Successfully removed language {}", lang_code));
                },
                Err(e) => {
                    error(&format!("Failed to remove language {}: {}", lang_code, e));
                    return Err(e);
                }
            }
        }

        info("remove_sutta_languages(): All languages removed successfully");
        Ok(any_deleted)
    }

    /// Get sutta languages with their counts in format "code|Name|Count"
    /// Returns a vector of strings sorted alphabetically by language code
    pub fn get_sutta_language_labels_with_counts(&self) -> Vec<String> {
        use crate::db::appdata_schema;
        use crate::lookup::LANG_CODE_TO_NAME;

        let result = self.do_read(|db_conn| {
            appdata_schema::suttas::table
                .group_by(appdata_schema::suttas::language)
                .select((appdata_schema::suttas::language, diesel::dsl::count(appdata_schema::suttas::id)))
                .load::<(String, i64)>(db_conn)
        });

        match result {
            Ok(lang_counts) => {
                let mut labels: Vec<String> = lang_counts
                    .into_iter()
                    .filter(|(lang, _)| !lang.is_empty())
                    .map(|(lang_code, count)| {
                        let lang_name = LANG_CODE_TO_NAME
                            .get(lang_code.as_str())
                            .copied()
                            .unwrap_or(&lang_code);
                        format!("{}|{}|{}", lang_code, lang_name, count)
                    })
                    .collect();

                // Sort alphabetically by language code
                labels.sort();
                labels
            },
            Err(e) => {
                error(&format!("get_sutta_language_labels_with_counts(): {}", e));
                Vec::new()
            }
        }
    }
}

