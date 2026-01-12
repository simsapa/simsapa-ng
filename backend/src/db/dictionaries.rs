use diesel::prelude::*;
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
