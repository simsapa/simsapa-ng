use diesel::prelude::*;
use crate::schema_dictionaries::*;
// use chrono::NaiveDateTime;

#[derive(Debug, Clone, Queryable, Selectable, Identifiable, PartialEq)]
#[diesel(table_name = dictionaries)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Dictionary {
    pub id: i32,
    pub label: String,
    pub title: String,
    pub dict_type: String,
    pub creator: Option<String>,
    pub description: Option<String>,
    pub feedback_email: Option<String>,
    pub feedback_url: Option<String>,
    pub version: Option<String>,
    // pub created_at: chrono::NaiveDateTime,
    // pub updated_at: Option<chrono::NaiveDateTime>,
}

#[derive(Insertable)]
#[diesel(table_name = dictionaries)]
pub struct NewDictionary<'a> {
    pub label: &'a str,
    pub title: &'a str,
    pub dict_type: &'a str,
    pub creator: Option<&'a str>,
    pub description: Option<&'a str>,
    pub feedback_email: Option<&'a str>,
    pub feedback_url: Option<&'a str>,
    pub version: Option<&'a str>,
}

impl Default for NewDictionary<'_> {
    fn default() -> Self {
        Self {
            label: "",
            title: "",
            dict_type: "",
            creator: None,
            description: None,
            feedback_email: None,
            feedback_url: None,
            version: None,
        }
    }
}

#[derive(Debug, Clone, Queryable, Selectable, Identifiable, PartialEq, Associations)]
#[diesel(table_name = dict_words)]
#[diesel(belongs_to(Dictionary, foreign_key = dictionary_id))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct DictWord {
    pub id: i32,
    pub dictionary_id: i32,
    pub dict_label: String,
    pub uid: String,
    pub word: String,
    pub word_ascii: String,
    pub language: Option<String>,
    pub source_uid: Option<String>,
    pub word_nom_sg: Option<String>,
    pub inflections: Option<String>,
    pub phonetic: Option<String>,
    pub transliteration: Option<String>,
    pub meaning_order: Option<i32>,
    pub definition_plain: Option<String>,
    pub definition_html: Option<String>,
    pub summary: Option<String>,
    pub synonyms: Option<String>,
    pub antonyms: Option<String>,
    pub homonyms: Option<String>,
    pub also_written_as: Option<String>,
    pub see_also: Option<String>,
    // pub created_at: chrono::NaiveDateTime,
    // pub updated_at: Option<chrono::NaiveDateTime>,
    // pub indexed_at: Option<chrono::NaiveDateTime>,
}

impl DictWord {
    pub fn word(&self) -> String {
        self.word.clone()
    }
}

// Hold owned Strings for improving batch insert.
#[derive(Insertable)]
#[diesel(table_name = dict_words)]
pub struct NewDictWord {
    pub dictionary_id: i32,
    pub dict_label: String,
    pub uid: String,
    pub word: String,
    pub word_ascii: String,
    pub language: Option<String>,
    pub source_uid: Option<String>,
    pub word_nom_sg: Option<String>,
    pub inflections: Option<String>,
    pub phonetic: Option<String>,
    pub transliteration: Option<String>,
    pub meaning_order: Option<i32>,
    pub definition_plain: Option<String>,
    pub definition_html: Option<String>,
    pub summary: Option<String>,
    pub synonyms: Option<String>,
    pub antonyms: Option<String>,
    pub homonyms: Option<String>,
    pub also_written_as: Option<String>,
    pub see_also: Option<String>,
}
