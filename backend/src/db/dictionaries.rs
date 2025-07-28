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
            },
        }
    }

    pub fn create_dictionary(&self,
                             new_dict: NewDictionary) -> Result<Dictionary> {
        use crate::db::dictionaries_schema::dictionaries;

        self.do_write(|db_conn| {
            diesel::insert_into(dictionaries::table)
                .values(&new_dict)
                .returning(Dictionary::as_returning())
                .get_result(db_conn)
        }).with_context(|| format!("Insert failed for dictionary: {}", new_dict.label))
    }

    pub fn delete_dictionary_by_label(&self,
                                      dict_label_val: &str) -> Result<usize> {
        use crate::db::dictionaries_schema::dictionaries::dsl::*;

        self.do_write(|db_conn| {
            diesel::delete(dictionaries.filter(label.eq(dict_label_val)))
                .execute(db_conn)
        })
    }

    pub fn create_dict_word(&self,
                            new_dict_word: &NewDictWord) -> Result<DictWord> {
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

        match dictionaries
            .select(Dictionary::as_select())
            .filter(label.eq("dpd"))
            .first(db_conn) {
                Ok(x) => return Ok(x),
                Err(_) => {},
            }

        // If not returned yet, create a new record
        let new_dict = NewDictionary {
            label: "dpd",
            title: "Digital Pāḷi Dictionary",
            dict_type: "sql", // FIXME dict_type = DictTypeName.Sql.value,
            .. Default::default()
        };

        self.create_dictionary(new_dict)
    }
}

pub fn create_dict_words_batch(db_conn: &mut SqliteConnection,
                               new_words: &[NewDictWord]) -> Result<usize, diesel::result::Error> {
    use crate::db::dictionaries_schema::dict_words;
    diesel::insert_into(dict_words::table)
        .values(new_words)
        .execute(db_conn)
}

