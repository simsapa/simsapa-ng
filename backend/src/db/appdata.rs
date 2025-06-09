use diesel::prelude::*;
use regex::Regex;
// use anyhow::{Context, Result};

use crate::db::get_dbm;
use crate::db::appdata_models::*;
use crate::db::DatabaseHandle;
use crate::logger::{info, error};

pub type AppdataDbHandle = DatabaseHandle;

impl AppdataDbHandle {
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

    pub fn get_translations_for_sutta_uid(&self, sutta_uid: &str) -> Vec<String> {
        // See sutta_search_window_state.py::_add_related_tabs()

        // Capture the reference before the first '/'
        let re = Regex::new(r"^([^/]+)/.*").expect("Invalid regex");
        let uid_ref = re.replace(&sutta_uid, "$1").to_string();

        use crate::db::appdata_schema::suttas::dsl::*;

        let dbm = get_dbm();
        let _lock = dbm.appdata.write_lock.lock();
        let mut db_conn = dbm.appdata.get_conn().expect("get_translations(): No appdata conn");

        let mut res: Vec<Sutta> = Vec::new();

        if let Ok(a) = suttas
            .select(Sutta::as_select())
            .filter(uid.ne(sutta_uid))
            .filter(uid.like(format!("{}/%", uid_ref)))
            .load(&mut db_conn) {
                res.extend(a);
            }

        let res_sorted_uids: Vec<String> = sort_suttas(res)
            .into_iter().map(|s| s.uid).collect();

        res_sorted_uids
    }
}

pub fn delete_sutta() {
    use crate::db::appdata_schema::suttas::dsl::*;

    let pattern = "unwholesome";

    let dbm = get_dbm();
    let _lock = dbm.appdata.write_lock.lock();
    let db_conn = &mut dbm.appdata.get_conn().expect("Can't get db conn");

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

