use std::collections::HashSet;

use diesel::prelude::*;
use chrono::NaiveDateTime;
use anyhow::{Context, Result};

use crate::db::dictionaries_models::*;
use crate::db::DatabaseHandle;
use crate::logger::error;

pub type DictionariesDbHandle = DatabaseHandle;

impl DictionariesDbHandle {
    pub fn get_word(&self, word_uid: &str) -> Option<DictWord> {
        use crate::db::dictionaries_schema::dict_words::dsl::*;

        let dict_word = self.do_read(|db_conn| {
            dict_words
                .filter(uid.eq(word_uid))
                .select(DictWord::as_select())
                .first(db_conn)
                .optional()
        });

        match dict_word {
            Ok(x) => x,
            Err(e) => {
                error(&format!("{}", e));
                None
            }
        }
    }

    /// Get distinct language values from dict_words table
    /// Returns a sorted Vec<String> with NULL values filtered out
    pub fn get_distinct_languages(&self) -> Vec<String> {
        use crate::db::dictionaries_schema::dict_words::dsl::*;

        let result = self.do_read(|db_conn| {
            dict_words
                .select(language)
                .filter(language.is_not_null())
                .distinct()
                .load::<Option<String>>(db_conn)
        });

        match result {
            Ok(langs) => {
                let mut unique_langs: Vec<String> = langs
                    .into_iter()
                    .flatten() // Filter out None values
                    .filter(|lang| !lang.is_empty()) // Filter out empty strings
                    .collect();
                unique_langs.sort();
                unique_langs
            }
            Err(e) => {
                error(&format!("get_distinct_languages(): {}", e));
                Vec::new()
            }
        }
    }

    /// Get distinct dict_label (dictionary source) values from dict_words table
    /// Returns a sorted Vec<String>
    pub fn get_distinct_sources(&self) -> Vec<String> {
        use crate::db::dictionaries_schema::dict_words::dsl::*;

        let result = self.do_read(|db_conn| {
            dict_words
                .select(dict_label)
                .distinct()
                .load::<String>(db_conn)
        });

        match result {
            Ok(mut sources) => {
                // Filter out empty strings and sort
                sources.retain(|s| !s.is_empty());
                sources.sort();
                sources
            }
            Err(e) => {
                error(&format!("get_distinct_sources(): {}", e));
                Vec::new()
            }
        }
    }

    pub fn create_dictionary(&self, new_dict: NewDictionary) -> Result<Dictionary> {
        use crate::db::dictionaries_schema::dictionaries;

        self.do_write(|db_conn| {
            diesel::insert_into(dictionaries::table)
                .values(&new_dict)
                .returning(Dictionary::as_returning())
                .get_result(db_conn)
        }).with_context(|| format!("Insert failed for dictionary: {}", new_dict.label))
    }

    pub fn delete_dictionary_by_label(&self, dict_label_val: &str) -> Result<usize> {
        use crate::db::dictionaries_schema::dictionaries::dsl::*;

        self.do_write(|db_conn| {
            diesel::delete(dictionaries.filter(label.eq(dict_label_val))).execute(db_conn)
        })
    }

    pub fn create_dict_word(&self, new_dict_word: &NewDictWord) -> Result<DictWord> {
        use crate::db::dictionaries_schema::dict_words;

        self.do_write(|db_conn| {
            diesel::insert_into(dict_words::table)
                .values(new_dict_word)
                .returning(DictWord::as_returning())
                .get_result(db_conn)
        })
    }

    /// List user-imported dictionaries, ordered by label.
    pub fn list_user_dictionaries(&self) -> Result<Vec<Dictionary>> {
        use crate::db::dictionaries_schema::dictionaries::dsl::*;

        self.do_read(|db_conn| {
            dictionaries
                .filter(is_user_imported.eq(true))
                .order(label.asc())
                .select(Dictionary::as_select())
                .load::<Dictionary>(db_conn)
        }).context("list_user_dictionaries failed")
    }

    /// Count `dict_words` rows belonging to a given dictionary.
    pub fn count_words_for_dictionary(&self, dict_id: i32) -> Result<i64> {
        use crate::db::dictionaries_schema::dict_words::dsl::*;

        self.do_read(|db_conn| {
            dict_words
                .filter(dictionary_id.eq(dict_id))
                .count()
                .get_result::<i64>(db_conn)
        }).context("count_words_for_dictionary failed")
    }

    /// Return user-imported `dictionaries` rows whose `indexed_at IS NULL`.
    pub fn list_dictionaries_needing_index(&self) -> Result<Vec<Dictionary>> {
        use crate::db::dictionaries_schema::dictionaries::dsl::*;

        self.do_read(|db_conn| {
            dictionaries
                .filter(is_user_imported.eq(true))
                .filter(indexed_at.is_null())
                .order(label.asc())
                .select(Dictionary::as_select())
                .load::<Dictionary>(db_conn)
        }).context("list_dictionaries_needing_index failed")
    }

    /// Rename a user-imported dictionary's label. In a single transaction:
    ///   - update `dictionaries.label`
    ///   - update `dict_words.dict_label`
    ///   - rewrite `dict_words.uid` from `<word>/<old_label>` to `<word>/<new_label>`
    ///   - set `dictionaries.indexed_at = NULL`
    pub fn rename_dictionary_label(&self, old_label: &str, new_label: &str) -> Result<()> {
        use crate::db::dictionaries_schema::dictionaries;
        use crate::db::dictionaries_schema::dict_words;

        self.do_write(|db_conn| {
            db_conn.transaction::<_, diesel::result::Error, _>(|tx| {
                // Update dictionaries.label and clear indexed_at.
                diesel::update(
                    dictionaries::table.filter(dictionaries::label.eq(old_label))
                )
                    .set((
                        dictionaries::label.eq(new_label),
                        dictionaries::indexed_at.eq::<Option<NaiveDateTime>>(None),
                    ))
                    .execute(tx)?;

                // Update dict_words.dict_label.
                diesel::update(
                    dict_words::table.filter(dict_words::dict_label.eq(old_label))
                )
                    .set(dict_words::dict_label.eq(new_label))
                    .execute(tx)?;

                // Rewrite dict_words.uid from <word>/<old_label> to <word>/<new_label>.
                // SQLite's REPLACE on the suffix is unsafe in general, so use a
                // computed expression that strips the old suffix and appends new.
                let suffix_old = format!("/{}", old_label);
                let suffix_new = format!("/{}", new_label);
                let sql = "UPDATE dict_words \
                           SET uid = substr(uid, 1, length(uid) - length(?1)) || ?2 \
                           WHERE uid LIKE ?3";
                let like_pat = format!("%/{}", old_label);
                diesel::sql_query(sql)
                    .bind::<diesel::sql_types::Text, _>(&suffix_old)
                    .bind::<diesel::sql_types::Text, _>(&suffix_new)
                    .bind::<diesel::sql_types::Text, _>(&like_pat)
                    .execute(tx)?;

                Ok(())
            })
        }).with_context(|| format!("rename_dictionary_label({} -> {}) failed", old_label, new_label))
    }

    /// Set `dictionaries.indexed_at` for one row.
    pub fn set_indexed_at(&self, dict_id: i32, ts: NaiveDateTime) -> Result<()> {
        use crate::db::dictionaries_schema::dictionaries::dsl::*;

        self.do_write(|db_conn| {
            diesel::update(dictionaries.filter(id.eq(dict_id)))
                .set(indexed_at.eq(Some(ts)))
                .execute(db_conn)
                .map(|_| ())
        }).context("set_indexed_at failed")
    }

    /// Set of distinct `dict_words.source_uid` values (column `dict_label`)
    /// belonging to non-user-imported dictionaries.
    /// This is the canonical "shipped/built-in label set" for label-collision
    /// validation (PRD §8a). Some shipped sources (e.g. bold-definitions) use a
    /// per-row `ref_code` as `dict_label`, so we MUST compute this from
    /// `dict_words` and not from `dictionaries.label`.
    pub fn list_shipped_source_uids(&self) -> Result<HashSet<String>> {
        use crate::db::dictionaries_schema::dict_words;
        use crate::db::dictionaries_schema::dictionaries;

        let rows: Vec<String> = self.do_read(|db_conn| {
            dict_words::table
                .inner_join(dictionaries::table.on(dict_words::dictionary_id.eq(dictionaries::id)))
                .filter(dictionaries::is_user_imported.eq(false))
                .select(dict_words::dict_label)
                .distinct()
                .load::<String>(db_conn)
        }).context("list_shipped_source_uids failed")?;

        Ok(rows.into_iter().collect())
    }

    /// Returns true if `label` collides with any shipped/built-in `source_uid`.
    pub fn is_label_taken_by_shipped(&self, label: &str) -> Result<bool> {
        Ok(self.list_shipped_source_uids()?.contains(label))
    }

    /// Find or create DPD Dictionary record with label 'dpd'
    pub fn find_or_create_dpd_dictionary(&self) -> Result<Dictionary> {
        use crate::db::dictionaries_schema::dictionaries::dsl::*;

        let db_conn = &mut self.get_conn()?;

        if let Ok(x) = dictionaries
            .select(Dictionary::as_select())
            .filter(label.eq("dpd"))
            .first(db_conn) { return Ok(x) }

        // If not returned yet, create a new record
        let new_dict = NewDictionary {
            label: "dpd",
            title: "Digital Pāḷi Dictionary",
            dict_type: "sql", // FIXME dict_type = DictTypeName.Sql.value,
            ..Default::default()
        };

        self.create_dictionary(new_dict)
    }
}

pub fn create_dict_words_batch(
    db_conn: &mut SqliteConnection,
    new_words: &[NewDictWord],
) -> Result<usize, diesel::result::Error> {
    use crate::db::dictionaries_schema::dict_words;
    diesel::insert_into(dict_words::table)
        .values(new_words)
        .execute(db_conn)
}
