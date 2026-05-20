//! Per-chunk-commit + cancellation integration test for the StarDict
//! importer.
//!
//! Builds a synthetic StarDict on disk with ~2500 entries (3 chunks at the
//! new chunk size of 1000), starts an import on a worker thread, flips the
//! cancel flag from the progress callback after the 2nd chunk lands, and
//! asserts that the rows already committed survive in the database.
//!
//! Covers PRD §4.3.4 / §7.3.3: "abort keeps partial entries".

use std::cell::RefCell;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serial_test::serial;
use simsapa_backend::dictionary_manager_core::import_user_zip;
use simsapa_backend::get_app_data;
use simsapa_backend::stardict_parse::StardictImportProgress;

mod helpers;
use helpers as h;

/// Write a minimal valid StarDict triplet (`.ifo` / `.idx` / `.dict`) into
/// `dir` with `count` entries, each entry's definition a fixed short
/// string. Returns the basename (stem) used for the files.
fn write_synthetic_stardict(dir: &Path, stem: &str, count: usize) -> std::io::Result<()> {
    // Each entry has the same definition body to keep the test small.
    let def_body = b"def";
    let def_size: u32 = def_body.len() as u32;

    // Build .dict (all definitions concatenated) and .idx in lockstep.
    let dict_path = dir.join(format!("{}.dict", stem));
    let idx_path = dir.join(format!("{}.idx", stem));

    let mut dict_file = fs::File::create(&dict_path)?;
    let mut idx_bytes: Vec<u8> = Vec::new();

    let mut offset: u32 = 0;
    for i in 0..count {
        // Unique headword per entry.
        let word = format!("word_{:06}", i);
        dict_file.write_all(def_body)?;

        idx_bytes.extend_from_slice(word.as_bytes());
        idx_bytes.push(0);
        idx_bytes.extend_from_slice(&offset.to_be_bytes());
        idx_bytes.extend_from_slice(&def_size.to_be_bytes());

        offset += def_size;
    }
    dict_file.flush()?;
    fs::write(&idx_path, &idx_bytes)?;

    // .ifo
    let ifo_path = dir.join(format!("{}.ifo", stem));
    let ifo_body = format!(
        "StarDict's dict ifo file\nversion=2.4.2\nwordcount={}\nidxfilesize={}\nbookname=Per-Chunk Test\nsametypesequence=m\n",
        count,
        idx_bytes.len(),
    );
    fs::write(&ifo_path, ifo_body.as_bytes())?;

    Ok(())
}

/// Zip a directory's contents into `out_zip`, files at archive root.
fn zip_dir_contents(src_dir: &Path, out_zip: &Path) -> std::io::Result<()> {
    let file = fs::File::create(out_zip)?;
    let mut zw = zip::ZipWriter::new(file);
    let opts: zip::write::FileOptions<'_, ()> =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for entry in fs::read_dir(src_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let name = entry.file_name().into_string().unwrap();
            zw.start_file(name, opts)?;
            let bytes = fs::read(&path)?;
            zw.write_all(&bytes)?;
        }
    }
    zw.finish()?;
    Ok(())
}

#[test]
#[serial]
fn abort_keeps_partial_rows_in_db() {
    h::app_data_setup();

    // ~2500 entries → 3 chunks at chunk_size 1000. Cancel after 2 ticks.
    let total_entries: usize = 2500;

    // Use a unique label per run. We INTENTIONALLY do not clean up at the
    // end of the test: this mirrors production abort semantics, where the
    // partial dict is left for the next reconcile to pick up rather than
    // rolled back. (Cleanup is now cheap — the `dict_words_fts` delete
    // trigger uses the FTS5 rowid, so per-row deletes are O(log n) rather
    // than the full FTS scans they were when `dict_word_id` was an
    // UNINDEXED column.)
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH).unwrap().as_millis();
    let label = format!("ssp_test_abort_partial_{}", millis);
    let label = label.as_str();

    let app_data = get_app_data();

    let tmp = tempfile::Builder::new()
        .prefix("simsapa-stardict-test-")
        .tempdir()
        .expect("tempdir");
    let stardict_dir = tmp.path().join("sd");
    fs::create_dir_all(&stardict_dir).unwrap();
    write_synthetic_stardict(&stardict_dir, "test", total_entries).expect("write stardict");

    let zip_path = tmp.path().join("test.zip");
    zip_dir_contents(&stardict_dir, &zip_path).expect("zip");

    let cancel = AtomicBool::new(false);
    let ticks: RefCell<Vec<usize>> = RefCell::new(Vec::new());
    let aborted_inserted: RefCell<Option<usize>> = RefCell::new(None);

    let outcome = import_user_zip(&zip_path, label, "en", &|p| {
        match p {
            StardictImportProgress::InsertingWords { done, total: _ } => {
                // The importer emits an initial `done: 0` before the first
                // chunk commits; only count post-chunk ticks here.
                if done > 0 {
                    ticks.borrow_mut().push(done);
                    // Flip the cancel flag after the 2nd committed-chunk tick.
                    if ticks.borrow().len() == 2 {
                        cancel.store(true, Ordering::Relaxed);
                    }
                }
            }
            StardictImportProgress::Aborted { inserted } => {
                *aborted_inserted.borrow_mut() = Some(inserted);
            }
            _ => {}
        }
    }, &cancel).expect("import_user_zip should return Ok on cancel");

    assert!(outcome.cancelled, "expected cancelled=true on abort");
    assert_eq!(outcome.inserted, 2000, "expected 2000 rows inserted before abort (2 chunks of 1000)");
    assert_eq!(*aborted_inserted.borrow(), Some(2000), "Aborted progress should report 2000 inserted");

    // Verify the parent row and partial children survive in the DB.
    let dict_id = outcome.dictionary_id;
    let row_count = app_data.dbm.dictionaries
        .count_words_for_dictionary(dict_id)
        .expect("count_words_for_dictionary");
    assert_eq!(row_count, 2000, "partial dict_words rows must persist");

    let dicts = app_data.dbm.dictionaries
        .list_dictionaries(Some(true))
        .expect("list_dictionaries");
    assert!(dicts.iter().any(|d| d.id == dict_id),
        "parent dictionaries row must persist so next-startup reconcile picks it up");
}
