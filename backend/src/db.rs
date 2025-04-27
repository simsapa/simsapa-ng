use crate::models::*;
use crate::schema::suttas::dsl::*;
use diesel::prelude::*;
use dotenvy::dotenv;
use std::env;
use std::path::PathBuf;
use std::fs;
use regex::Regex;

use crate::get_create_simsapa_app_root;

pub fn establish_connection() -> SqliteConnection {
    dotenv().ok();

    let db_path = match env::var("DATABASE_PATH") {
        Ok(s) => PathBuf::from(s),
        Err(_) => {
            if let Ok(p) = get_create_simsapa_app_root() {
                PathBuf::from(p).join("appdata.sqlite3")
            } else {
                PathBuf::from("appdata.sqlite3")
            }
        }
    };

    if !db_path.exists() {
        panic!("File not found: {}", db_path.as_os_str().to_str().expect("os_str Error!"));
    }

    let abs_path = fs::canonicalize(db_path.clone()).unwrap_or(db_path);
    let database_url = format!("sqlite://{}", abs_path.as_os_str().to_str().expect("os_str Error!"));

    SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

// pub fn create_sutta(conn: &mut SqliteConnection,
//                     sutta_uid: &str,
//                     sutta_sutta_ref: &str,
//                     sutta_title: &str,
//                     sutta_content_html: &str) -> Sutta {
//     use crate::schema::suttas;

//     let new_sutta = NewSutta {
//         uid: sutta_uid,
//         sutta_ref: sutta_sutta_ref,
//         title: sutta_title,
//         content_html: sutta_content_html,
//     };

//     diesel::insert_into(suttas::table)
//         .values(&new_sutta)
//         .returning(Sutta::as_returning())
//         .get_result(conn)
//         .expect("Error saving new sutta")
// }

// pub fn populate_suttas() {
//     let connection = &mut establish_connection();
//     let post = create_sutta(connection, "New Post", "Amazing words in orders of terrible!");
//     println!("\nSaved post '{}' with id {}", post.title, post.id);
// }

// pub fn publish_post() {
//     use crate::schema::posts::dsl::{posts, published};

//     let post_id: i32 = 2;
//     let connection = &mut establish_connection();

//     let post = diesel::update(posts.find(post_id))
//         .set(published.eq(true))
//         .returning(Post::as_returning())
//         .get_result(connection)
//         .unwrap();

//     println!("Published post {}", post.title);
// }

pub fn get_sutta(sutta_uid: &str) -> Option<Sutta> {
    use crate::schema::suttas::dsl::suttas;

    let conn = &mut establish_connection();

    let sutta = suttas
        .filter(uid.eq(sutta_uid))
        .select(Sutta::as_select())
        .first(conn)
        .optional();

    match sutta {
        Ok(x) => x,
        Err(e) => {
            eprintln!("{}", e);
            None
        },
    }
}

pub fn delete_sutta() {
    use crate::schema::suttas::dsl::*;

    let pattern = "unwholesome";

    let connection = &mut establish_connection();
    let num_deleted = diesel::delete(suttas.filter(content_html.like(pattern)))
        .execute(connection)
        .expect("Error deleting suttas");

    println!("Deleted {} suttas", num_deleted);
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


pub fn get_translations_for_sutta_uid(sutta_uid: &str) -> Vec<String> {
    // See sutta_search_window_state.py::_add_related_tabs()

    // Capture the reference before the first '/'
    let re = Regex::new(r"^([^/]+)/.*").expect("Invalid regex");
    let uid_ref = re.replace(&sutta_uid, "$1").to_string();

    use crate::schema::suttas::dsl::suttas;

    let conn = &mut establish_connection();

    let mut res: Vec<Sutta> = Vec::new();

    if let Ok(a) = suttas
        .select(Sutta::as_select())
        .filter(uid.ne(sutta_uid))
        .filter(uid.like(format!("{}/%", uid_ref)))
        .load(conn) {
            res.extend(a);
    }

    let res_sorted_uids: Vec<String> = sort_suttas(res)
        .into_iter().map(|s| s.uid).collect();

    res_sorted_uids
}
