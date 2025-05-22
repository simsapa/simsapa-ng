use diesel::prelude::*;
use crate::db::appdata_schema::*;
// use chrono::NaiveDateTime;

#[derive(Debug, Clone, Queryable, Selectable, Identifiable, PartialEq)]
#[diesel(table_name = app_settings)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct AppSetting {
    pub id: i32,
    #[diesel(column_name = "key")]
    pub key: String,
    pub value: Option<String>,
    // pub created_at: NaiveDateTime,
    // pub updated_at: Option<NaiveDateTime>,
}

#[derive(Insertable)]
#[diesel(table_name = app_settings)]
pub struct NewAppSetting<'a> {
    #[diesel(column_name = "key")]
    pub key: &'a str,
    pub value: Option<&'a str>,
}

// Queryable struct for reading records
#[derive(Debug, Clone, Queryable, Selectable, Identifiable, PartialEq)]
#[diesel(table_name = suttas)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Sutta {
    pub id: i32,
    pub uid: String,
    pub sutta_ref: String,
    pub nikaya: String,
    pub language: String,
    pub group_path: Option<String>,
    pub group_index: Option<i32>,
    pub order_index: Option<i32>,
    pub sutta_range_group: Option<String>,
    pub sutta_range_start: Option<i32>,
    pub sutta_range_end: Option<i32>,
    pub title: Option<String>,
    pub title_ascii: Option<String>,
    pub title_pali: Option<String>,
    pub title_trans: Option<String>,
    pub description: Option<String>,
    pub content_plain: Option<String>,
    pub content_html: Option<String>,
    pub content_json: Option<String>,
    pub content_json_tmpl: Option<String>,
    pub source_uid: Option<String>,
    pub source_info: Option<String>,
    pub source_language: Option<String>,
    pub message: Option<String>,
    pub copyright: Option<String>,
    pub license: Option<String>,
    // pub created_at: NaiveDateTime,
    // pub updated_at: Option<NaiveDateTime>,
    // pub indexed_at: Option<NaiveDateTime>,
}

// Insertable struct for creating new records
#[derive(Insertable)]
#[diesel(table_name = suttas)]
pub struct NewSutta<'a> {
    pub uid: &'a str,
    pub sutta_ref: &'a str,
    pub nikaya: &'a str,
    pub language: &'a str,
    pub group_path: Option<&'a str>,
    pub group_index: Option<i32>,
    pub order_index: Option<i32>,
    pub sutta_range_group: Option<&'a str>,
    pub sutta_range_start: Option<i32>,
    pub sutta_range_end: Option<i32>,
    pub title: Option<&'a str>,
    pub title_ascii: Option<&'a str>,
    pub title_pali: Option<&'a str>,
    pub title_trans: Option<&'a str>,
    pub description: Option<&'a str>,
    pub content_plain: Option<&'a str>,
    pub content_html: Option<&'a str>,
    pub content_json: Option<&'a str>,
    pub content_json_tmpl: Option<&'a str>,
    pub source_uid: Option<&'a str>,
    pub source_info: Option<&'a str>,
    pub source_language: Option<&'a str>,
    pub message: Option<&'a str>,
    pub copyright: Option<&'a str>,
    pub license: Option<&'a str>,
}

#[derive(Debug, Clone, Queryable, Selectable, Identifiable, PartialEq, Associations)]
#[diesel(belongs_to(Sutta, foreign_key = sutta_id))]
#[diesel(table_name = sutta_variants)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct SuttaVariant {
    pub id: i32,
    pub sutta_id: i32,
    pub sutta_uid: String,
    pub language: Option<String>,
    pub source_uid: Option<String>,
    pub content_json: Option<String>,
    // pub created_at: NaiveDateTime,
    // pub updated_at: Option<NaiveDateTime>,
}

#[derive(Insertable)]
#[diesel(table_name = sutta_variants)]
pub struct NewSuttaVariant<'a> {
    pub sutta_id: i32,
    pub sutta_uid: &'a str,
    pub language: Option<&'a str>,
    pub source_uid: Option<&'a str>,
    pub content_json: Option<&'a str>,
}

#[derive(Debug, Clone, Queryable, Selectable, Identifiable, PartialEq, Associations)]
#[diesel(belongs_to(Sutta, foreign_key = sutta_id))]
#[diesel(table_name = sutta_comments)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct SuttaComment {
    pub id: i32,
    pub sutta_id: i32,
    pub sutta_uid: String,
    pub language: Option<String>,
    pub source_uid: Option<String>,
    pub content_json: Option<String>,
    // pub created_at: NaiveDateTime,
    // pub updated_at: Option<NaiveDateTime>,
}

#[derive(Insertable)]
#[diesel(table_name = sutta_comments)]
pub struct NewSuttaComment<'a> {
    pub sutta_id: i32,
    pub sutta_uid: &'a str,
    pub language: Option<&'a str>,
    pub source_uid: Option<&'a str>,
    pub content_json: Option<&'a str>,
}

#[derive(Debug, Clone, Queryable, Selectable, Identifiable, PartialEq, Associations)]
#[diesel(belongs_to(Sutta, foreign_key = sutta_id))]
#[diesel(table_name = sutta_glosses)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct SuttaGloss {
    pub id: i32,
    pub sutta_id: i32,
    pub sutta_uid: String,
    pub language: Option<String>,
    pub source_uid: Option<String>,
    pub content_json: Option<String>,
    // pub created_at: NaiveDateTime,
    // pub updated_at: Option<NaiveDateTime>,
}

#[derive(Insertable)]
#[diesel(table_name = sutta_glosses)]
pub struct NewSuttaGloss<'a> {
    pub sutta_id: i32,
    pub sutta_uid: &'a str,
    pub language: Option<&'a str>,
    pub source_uid: Option<&'a str>,
    pub content_json: Option<&'a str>,
}
