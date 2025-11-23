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
                // Sort and filter out empty strings
                langs.sort();
                langs.into_iter()
                    .filter(|l| !l.is_empty())
                    .map(|l| l.to_lowercase())
                    .collect::<std::collections::HashSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>()
                    .into_iter()
                    .collect()
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

