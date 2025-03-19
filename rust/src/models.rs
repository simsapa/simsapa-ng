use diesel::prelude::*;
use crate::schema::suttas;

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::suttas)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Sutta {
    pub id: i32,
    pub uid: String,
    pub sutta_ref: String,
    pub title: String,
    pub content_html: String,
}

#[derive(Insertable)]
#[diesel(table_name = suttas)]
pub struct NewSutta<'a> {
    pub uid: &'a str,
    pub sutta_ref: &'a str,
    pub title: &'a str,
    pub content_html: &'a str,
}
