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

    pub fn get_full_sutta_uid(&self, partial_uid: &str) -> Option<String> {
        use crate::db::appdata_schema::suttas::dsl::*;

        // If UID already contains '/', check if it exists and return it
        if partial_uid.contains('/') {
            let result = self.do_read(|db_conn| {
                suttas
                    .filter(uid.eq(partial_uid))
                    .select(uid)
                    .first::<String>(db_conn)
                    .optional()
            });

            return match result {
                Ok(found_uid) => found_uid,
                Err(e) => {
                    error(&format!("Error checking sutta UID '{}': {}", partial_uid, e));
                    None
                }
            };
        }

        // First, try to find the Pali Mahasangiti version "{partial_uid}/pli/ms"
        let pli_ms_uid = format!("{}/pli/ms", partial_uid);
        let pli_result = self.do_read(|db_conn| {
            suttas
                .filter(uid.eq(&pli_ms_uid))
                .select(uid)
                .first::<String>(db_conn)
                .optional()
        });

        match pli_result {
            Ok(Some(found_uid)) => return Some(found_uid),
            Ok(None) => {
                // Pali MS not found, try LIKE query for any translation
            },
            Err(e) => {
                error(&format!("Error checking Pali MS UID '{}': {}", pli_ms_uid, e));
            }
        }

        // If Pali MS not found, find the first matching UID with LIKE
        let pattern = format!("{}/%", partial_uid);
        let result = self.do_read(|db_conn| {
            suttas
                .filter(uid.like(pattern))
                .select(uid)
                .first::<String>(db_conn)
                .optional()
        });

        match result {
            Ok(found_uid) => found_uid,
            Err(e) => {
                error(&format!("Error finding sutta UID for '{}': {}", partial_uid, e));
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
            table_name: String,
            sutta_title: String,
            sutta_ref: String,
        }

        let res_sorted_data: Vec<TranslationData> = sort_suttas(res)
            .into_iter().map(|s| TranslationData {
                item_uid: s.uid,
                table_name: "suttas".to_string(),
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

    // === Book-related queries ===

    pub fn get_book_by_uid(&self, book_uid: &str) -> Result<Option<Book>> {
        use crate::db::appdata_schema::books::dsl::*;

        self.do_read(|db_conn| {
            books
                .filter(uid.eq(book_uid))
                .select(Book::as_select())
                .first(db_conn)
                .optional()
        })
    }

    pub fn get_book_spine_item(&self, spine_item_uid_param: &str) -> Result<Option<BookSpineItem>> {
        use crate::db::appdata_schema::book_spine_items::dsl::*;

        self.do_read(|db_conn| {
            book_spine_items
                .filter(spine_item_uid.eq(spine_item_uid_param))
                .select(BookSpineItem::as_select())
                .first(db_conn)
                .optional()
        })
    }

    pub fn get_book_resource(&self, book_uid_param: &str, resource_path_param: &str) -> Result<Option<BookResource>> {
        use crate::db::appdata_schema::book_resources::dsl::*;

        self.do_read(|db_conn| {
            book_resources
                .filter(book_uid.eq(book_uid_param))
                .filter(resource_path.eq(resource_path_param))
                .select(BookResource::as_select())
                .first(db_conn)
                .optional()
        })
    }

    pub fn get_all_books(&self) -> Result<Vec<Book>> {
        use crate::db::appdata_schema::books::dsl::*;

        self.do_read(|db_conn| {
            books
                .select(Book::as_select())
                .order(title.asc())
                .load(db_conn)
        })
    }

    pub fn get_spine_items_for_book(&self, book_uid_param: &str) -> Result<Vec<BookSpineItem>> {
        use crate::db::appdata_schema::book_spine_items::dsl::*;

        self.do_read(|db_conn| {
            book_spine_items
                .filter(book_uid.eq(book_uid_param))
                .order(spine_index.asc())
                .select(BookSpineItem::as_select())
                .load(db_conn)
        })
    }

    pub fn get_book_spine_item_by_path(&self, book_uid_param: &str, resource_path_param: &str) -> Result<Option<BookSpineItem>> {
        use crate::db::appdata_schema::book_spine_items::dsl::*;

        self.do_read(|db_conn| {
            book_spine_items
                .filter(book_uid.eq(book_uid_param))
                .filter(resource_path.eq(resource_path_param))
                .select(BookSpineItem::as_select())
                .first(db_conn)
                .optional()
        })
    }

    pub fn get_prev_book_spine_item(&self, spine_item_uid_param: &str) -> Result<Option<BookSpineItem>> {
        use crate::db::appdata_schema::book_spine_items::dsl::*;

        // First get the current spine item to obtain book_uid and spine_index
        let current_item = self.get_book_spine_item(spine_item_uid_param)?;

        match current_item {
            Some(item) => {
                // Query for spine item with same book_uid and spine_index - 1
                self.do_read(|db_conn| {
                    book_spine_items
                        .filter(book_uid.eq(&item.book_uid))
                        .filter(spine_index.eq(item.spine_index - 1))
                        .select(BookSpineItem::as_select())
                        .first(db_conn)
                        .optional()
                })
            }
            None => Ok(None), // Current item not found, return None
        }
    }

    pub fn get_next_book_spine_item(&self, spine_item_uid_param: &str) -> Result<Option<BookSpineItem>> {
        use crate::db::appdata_schema::book_spine_items::dsl::*;

        // First get the current spine item to obtain book_uid and spine_index
        let current_item = self.get_book_spine_item(spine_item_uid_param)?;

        match current_item {
            Some(item) => {
                // Query for spine item with same book_uid and spine_index + 1
                self.do_read(|db_conn| {
                    book_spine_items
                        .filter(book_uid.eq(&item.book_uid))
                        .filter(spine_index.eq(item.spine_index + 1))
                        .select(BookSpineItem::as_select())
                        .first(db_conn)
                        .optional()
                })
            }
            None => Ok(None), // Current item not found, return None
        }
    }

    pub fn get_prev_sutta(&self, sutta_uid_param: &str) -> Result<Option<Sutta>> {
        use crate::db::appdata_schema::suttas::dsl::*;

        // Get the current sutta
        let current_sutta = match self.get_sutta(sutta_uid_param) {
            Some(s) => s,
            None => return Ok(None),
        };

        // Check if sutta has range information
        let (range_group, range_start) = match (&current_sutta.sutta_range_group, current_sutta.sutta_range_start) {
            (Some(g), Some(s)) => (g.clone(), s),
            _ => return Ok(None), // No range info, can't navigate
        };

        // Calculate the previous range end: current start - 1
        let prev_end = range_start - 1;

        if prev_end < 1 {
            // We're at the first sutta in this group, check for previous numbered group
            return self.get_last_sutta_in_prev_group(&range_group, &current_sutta.language, &current_sutta.source_uid);
        }

        // Query for all suttas in same group with range_end <= prev_end
        // Order by range_end DESC to get the closest previous sutta
        let candidates: Vec<Sutta> = self.do_read(|db_conn| {
            suttas
                .filter(sutta_range_group.eq(&range_group))
                .filter(sutta_range_end.is_not_null())
                .filter(sutta_range_end.le(prev_end))
                .order(sutta_range_end.desc())
                .limit(100)  // Get multiple candidates to allow filtering by language/source
                .select(Sutta::as_select())
                .load(db_conn)
        })?;

        // Prioritize: same source_uid > same language > "en" > "pli"
        Ok(self.prioritize_sutta_by_language_and_source(
            candidates,
            &current_sutta.language,
            &current_sutta.source_uid,
        ))
    }

    pub fn get_next_sutta(&self, sutta_uid_param: &str) -> Result<Option<Sutta>> {
        use crate::db::appdata_schema::suttas::dsl::*;

        // Get the current sutta
        let current_sutta = match self.get_sutta(sutta_uid_param) {
            Some(s) => s,
            None => return Ok(None),
        };

        // Check if sutta has range information
        let (range_group, range_end) = match (&current_sutta.sutta_range_group, current_sutta.sutta_range_end) {
            (Some(g), Some(e)) => (g.clone(), e),
            _ => return Ok(None), // No range info, can't navigate
        };

        // Calculate the next range start: current end + 1
        let next_start = range_end + 1;

        // Query for all suttas in same group with range_start >= next_start
        // Order by range_start ASC to get the closest next sutta
        let candidates: Vec<Sutta> = self.do_read(|db_conn| {
            suttas
                .filter(sutta_range_group.eq(&range_group))
                .filter(sutta_range_start.is_not_null())
                .filter(sutta_range_start.ge(next_start))
                .order(sutta_range_start.asc())
                .limit(100)  // Get multiple candidates to allow filtering by language/source
                .select(Sutta::as_select())
                .load(db_conn)
        })?;

        // Prioritize: same source_uid > same language > "en" > "pli"
        let next_in_group = self.prioritize_sutta_by_language_and_source(
            candidates,
            &current_sutta.language,
            &current_sutta.source_uid,
        );

        if next_in_group.is_some() {
            return Ok(next_in_group);
        }

        // No next sutta in current group, check for next numbered group
        self.get_first_sutta_in_next_group(&range_group, &current_sutta.language, &current_sutta.source_uid)
    }

    fn prioritize_sutta_by_language_and_source(
        &self,
        candidates: Vec<Sutta>,
        current_language: &str,
        current_source: &Option<String>,
    ) -> Option<Sutta> {
        if candidates.is_empty() {
            return None;
        }

        // Group candidates by their range (to handle multiple translations of same sutta)
        // We want to find the first available sutta number, then choose best translation
        let first_range = candidates[0].sutta_range_start;
        let same_range: Vec<Sutta> = candidates
            .into_iter()
            .filter(|s| s.sutta_range_start == first_range)
            .collect();

        // Priority 1: Same source_uid and same language
        if let Some(source) = current_source {
            if let Some(sutta) = same_range.iter().find(|s| {
                s.language == current_language && s.source_uid.as_ref() == Some(source)
            }) {
                return Some(sutta.clone());
            }
        }

        // Priority 2: Same language, any source
        if let Some(sutta) = same_range.iter().find(|s| s.language == current_language) {
            return Some(sutta.clone());
        }

        // Priority 3: English, any source
        if let Some(sutta) = same_range.iter().find(|s| s.language == "en") {
            return Some(sutta.clone());
        }

        // Priority 4: Pali, any source
        if let Some(sutta) = same_range.iter().find(|s| s.language == "pli") {
            return Some(sutta.clone());
        }

        // Fallback: return first candidate
        same_range.into_iter().next()
    }

    fn get_last_sutta_in_prev_group(
        &self,
        current_group: &str,
        current_language: &str,
        current_source: &Option<String>,
    ) -> Result<Option<Sutta>> {
        use crate::db::appdata_schema::suttas::dsl::*;

        // Extract base collection and number from group (e.g., "an3" -> "an", 3)
        let (base, num) = match self.extract_group_number(current_group) {
            Some((b, n)) if n > 1 => (b, n),
            _ => return Ok(None), // Not a numbered group or already at 1
        };

        // Try previous numbered group (e.g., "an3" -> "an2")
        let prev_group = format!("{}{}", base, num - 1);

        // Get the last sutta in the previous group (highest range_end)
        let candidates: Vec<Sutta> = self.do_read(|db_conn| {
            suttas
                .filter(sutta_range_group.eq(&prev_group))
                .filter(sutta_range_end.is_not_null())
                .order(sutta_range_end.desc())
                .limit(100)
                .select(Sutta::as_select())
                .load(db_conn)
        })?;

        Ok(self.prioritize_sutta_by_language_and_source(
            candidates,
            current_language,
            current_source,
        ))
    }

    fn get_first_sutta_in_next_group(
        &self,
        current_group: &str,
        current_language: &str,
        current_source: &Option<String>,
    ) -> Result<Option<Sutta>> {
        use crate::db::appdata_schema::suttas::dsl::*;

        // Extract base collection and number from group (e.g., "an3" -> "an", 3)
        let (base, num) = match self.extract_group_number(current_group) {
            Some((b, n)) => (b, n),
            None => return Ok(None), // Not a numbered group
        };

        // Try next numbered group (e.g., "an3" -> "an4")
        let next_group = format!("{}{}", base, num + 1);

        // Get the first sutta in the next group (lowest range_start)
        let candidates: Vec<Sutta> = self.do_read(|db_conn| {
            suttas
                .filter(sutta_range_group.eq(&next_group))
                .filter(sutta_range_start.is_not_null())
                .order(sutta_range_start.asc())
                .limit(100)
                .select(Sutta::as_select())
                .load(db_conn)
        })?;

        Ok(self.prioritize_sutta_by_language_and_source(
            candidates,
            current_language,
            current_source,
        ))
    }

    fn extract_group_number(&self, group: &str) -> Option<(String, i32)> {
        // Extract base collection and number from group
        // Examples: "an3" -> ("an", 3), "sn30" -> ("sn", 30), "mn" -> None
        let re = regex::Regex::new(r"^([a-z-]+)(\d+)$").ok()?;
        let caps = re.captures(group)?;

        let base = caps.get(1)?.as_str().to_string();
        let num = caps.get(2)?.as_str().parse::<i32>().ok()?;

        Some((base, num))
    }

    pub fn delete_book_by_uid(&self, book_uid_param: &str) -> Result<()> {
        use crate::db::appdata_schema::books::dsl::*;

        self.do_write(|db_conn| {
            diesel::delete(books.filter(uid.eq(book_uid_param)))
                .execute(db_conn)
                .map(|_| ())
        })
    }

    pub fn update_book_metadata(&self, book_uid_param: &str, title_param: &str, author_param: &str, enable_embedded_css_param: bool) -> Result<()> {
        use crate::db::appdata_schema::books::dsl::*;

        self.do_write(|db_conn| {
            diesel::update(books.filter(uid.eq(book_uid_param)))
                .set((
                    title.eq(Some(title_param)),
                    author.eq(if author_param.is_empty() { None } else { Some(author_param) }),
                    enable_embedded_css.eq(enable_embedded_css_param),
                ))
                .execute(db_conn)
                .map(|_| ())
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

