//! Diesel models for XML fragments database

use diesel::prelude::*;
use crate::tipitaka_xml_parser::fragments_schema::{xml_fragments, nikaya_structures};

/// Insertable nikaya structure model
#[derive(Insertable)]
#[diesel(table_name = nikaya_structures)]
pub struct NewNikayaStructure<'a> {
    pub nikaya: &'a str,
    pub levels: &'a str,
}

/// Queryable nikaya structure model
#[derive(Queryable, Selectable)]
#[diesel(table_name = nikaya_structures)]
pub struct NikayaStructureRecord {
    pub id: i32,
    pub nikaya: String,
    pub levels: String,
    pub created_at: Option<chrono::NaiveDateTime>,
}

/// Insertable XML fragment model
#[derive(Insertable)]
#[diesel(table_name = xml_fragments)]
pub struct NewXmlFragment<'a> {
    pub cst_file: &'a str,
    pub frag_idx: i32,
    pub frag_type: &'a str,
    pub frag_review: Option<&'a str>,
    pub nikaya: &'a str,
    pub cst_code: Option<&'a str>,
    pub sc_code: Option<&'a str>,
    pub content: &'a str,
    pub cst_vagga: Option<&'a str>,
    pub cst_sutta: Option<&'a str>,
    pub cst_paranum: Option<&'a str>,
    pub sc_sutta: Option<&'a str>,
    pub start_line: i32,
    pub start_char: i32,
    pub end_line: i32,
    pub end_char: i32,
    pub group_levels: &'a str,
}

/// Queryable XML fragment model
#[derive(Queryable, Selectable)]
#[diesel(table_name = xml_fragments)]
pub struct XmlFragmentRecord {
    pub id: i32,
    pub cst_file: String,
    pub frag_idx: i32,
    pub frag_type: String,
    pub frag_review: Option<String>,
    pub nikaya: String,
    pub cst_code: Option<String>,
    pub sc_code: Option<String>,
    pub content: String,
    pub cst_vagga: Option<String>,
    pub cst_sutta: Option<String>,
    pub cst_paranum: Option<String>,
    pub sc_sutta: Option<String>,
    pub start_line: i32,
    pub start_char: i32,
    pub end_line: i32,
    pub end_char: i32,
    pub group_levels: String,
    pub created_at: Option<chrono::NaiveDateTime>,
}
