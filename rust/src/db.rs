use crate::models::*;
use crate::schema::suttas::dsl::*;
use diesel::prelude::*;
use dotenvy::dotenv;
use std::env;
use std::path::PathBuf;
use std::fs;

use crate::api::ffi::get_app_assets_path;

pub fn establish_connection() -> SqliteConnection {
    dotenv().ok();

    let db_path = match env::var("DATABASE_PATH") {
        Ok(s) => PathBuf::from(s),
        Err(_) => {
            PathBuf::from(get_app_assets_path().to_string()).join("appdata.sqlite3")
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

pub fn create_sutta(conn: &mut SqliteConnection,
                    sutta_uid: &str,
                    sutta_sutta_ref: &str,
                    sutta_title: &str,
                    sutta_content_html: &str) -> Sutta {
    use crate::schema::suttas;

    let new_sutta = NewSutta {
        uid: sutta_uid,
        sutta_ref: sutta_sutta_ref,
        title: sutta_title,
        content_html: sutta_content_html,
    };

    diesel::insert_into(suttas::table)
        .values(&new_sutta)
        .returning(Sutta::as_returning())
        .get_result(conn)
        .expect("Error saving new sutta")
}

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
        Err(_) => None,
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
