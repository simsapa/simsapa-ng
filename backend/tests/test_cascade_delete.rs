use diesel::prelude::*;
use serial_test::serial;
use simsapa_backend::get_app_data;

mod helpers;
use helpers as h;

#[test]
#[serial]
fn test_cascade_delete_on_sutta_removal() {
    use simsapa_backend::db::appdata_schema::*;
    
    h::app_data_setup();
    let app_data = get_app_data();
    
    // Get a sutta with some variants/comments/glosses
    let test_sutta = app_data.dbm.appdata.do_read(|db_conn| {
        suttas::table
            .filter(suttas::language.eq("en"))
            .select(suttas::id)
            .first::<i32>(db_conn)
            .optional()
    }).expect("Query failed");
    
    let test_sutta_id = match test_sutta {
        Some(id) => id,
        None => {
            println!("No English suttas found, skipping test");
            return;
        }
    };
    
    // Count child records BEFORE deletion
    let variants_count_before = app_data.dbm.appdata.do_read(|db_conn| {
        sutta_variants::table
            .filter(sutta_variants::sutta_id.eq(test_sutta_id))
            .count()
            .get_result::<i64>(db_conn)
    }).expect("Query failed");
    
    let comments_count_before = app_data.dbm.appdata.do_read(|db_conn| {
        sutta_comments::table
            .filter(sutta_comments::sutta_id.eq(test_sutta_id))
            .count()
            .get_result::<i64>(db_conn)
    }).expect("Query failed");
    
    let glosses_count_before = app_data.dbm.appdata.do_read(|db_conn| {
        sutta_glosses::table
            .filter(sutta_glosses::sutta_id.eq(test_sutta_id))
            .count()
            .get_result::<i64>(db_conn)
    }).expect("Query failed");
    
    println!("Before deletion - Variants: {}, Comments: {}, Glosses: {}", 
             variants_count_before, comments_count_before, glosses_count_before);
    
    // Delete the sutta (not by language, by ID to be precise)
    let deleted = app_data.dbm.appdata.do_write(|db_conn| {
        diesel::delete(suttas::table.filter(suttas::id.eq(test_sutta_id)))
            .execute(db_conn)
    }).expect("Delete failed");
    
    println!("Deleted {} sutta", deleted);
    
    // Count child records AFTER deletion - should be 0 if CASCADE works
    let variants_count_after = app_data.dbm.appdata.do_read(|db_conn| {
        sutta_variants::table
            .filter(sutta_variants::sutta_id.eq(test_sutta_id))
            .count()
            .get_result::<i64>(db_conn)
    }).expect("Query failed");
    
    let comments_count_after = app_data.dbm.appdata.do_read(|db_conn| {
        sutta_comments::table
            .filter(sutta_comments::sutta_id.eq(test_sutta_id))
            .count()
            .get_result::<i64>(db_conn)
    }).expect("Query failed");
    
    let glosses_count_after = app_data.dbm.appdata.do_read(|db_conn| {
        sutta_glosses::table
            .filter(sutta_glosses::sutta_id.eq(test_sutta_id))
            .count()
            .get_result::<i64>(db_conn)
    }).expect("Query failed");
    
    println!("After deletion - Variants: {}, Comments: {}, Glosses: {}", 
             variants_count_after, comments_count_after, glosses_count_after);
    
    // Assert CASCADE DELETE worked
    assert_eq!(variants_count_after, 0, "CASCADE DELETE should have removed all variants");
    assert_eq!(comments_count_after, 0, "CASCADE DELETE should have removed all comments");
    assert_eq!(glosses_count_after, 0, "CASCADE DELETE should have removed all glosses");
}
