use std::collections::HashMap;
use std::path::Path;

use diesel::prelude::*;

use stardict::{self, Ifo, WordDefinition};

use crate::get_app_data;
use crate::db;
use crate::db::dictionaries_models::NewDictWord;
use crate::helpers as h;
use crate::logger::{info, warn, error};

/// Stages emitted by the SQL-only StarDict import pipeline.
///
/// The startup re-indexing pass owns all FTS5 / Tantivy writes, so this enum
/// intentionally has no `IndexingFts5` / `IndexingTantivy` variants.
#[derive(Debug, Clone)]
pub enum StardictImportProgress {
    Extracting,
    Parsing,
    InsertingWords { done: usize, total: usize },
    Done,
    Failed { msg: String },
}

/// Read the optional `description=` line from a StarDict `.ifo` file.
///
/// Returns the trimmed value when present and non-empty; otherwise `None`.
/// The underlying `stardict` crate already parses the `.ifo` description
/// field (see `stardict::Ifo::description`), so this is a thin convenience
/// wrapper used by the runtime importer.
pub fn read_ifo_description(unzipped_dir: &Path, physical_stem: &str) -> Option<String> {
    let ifo_path = unzipped_dir.join(format!("{}.ifo", physical_stem));
    let ifo = Ifo::new(ifo_path).ok()?;
    let trimmed = ifo.description.trim();
    if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
}

/// Replaces bword:// links with ssp:// links.
fn parse_bword_links_to_ssp(definition: &str) -> String {
    // QueryType.words
    let words_route_path = "words";
    definition
        .replace("bword://localhost/", &format!("ssp://{}/", words_route_path))
        .replace("bword://", &format!("ssp://{}/", words_route_path))
}

/// Holds the parsed data extracted from WordDefinition segments.
#[derive(Debug, Default)]
struct DictEntry {
    word: String,
    definition_plain: Option<String>,
    definition_html: Option<String>,
    synonyms: Vec<String>,
}

fn db_entries(x: &DictEntry,
              dictionary_id: i32,
              dictionary_label: &str,
              lang: &str) -> NewDictWord {
    // TODO should we check for conflicting uids? generate with meaning count?
    // NOTE: sanitizing dict word uid causes duplicates because of punctuation, and DPD stardict fails to import to db.
    // let uid = h::word_uid(&x.word, dictionary_label);
    let uid = format!("{}/{}", &x.word.trim(), dictionary_label);

    // add a Latinized lowercase synonym
    let mut syn = x.synonyms.clone();
    let latin = h::latinize(&x.word).to_lowercase();
    if !syn.contains(&latin) {
        syn.push(latin);
    }

    NewDictWord {
        // copy values
        word: x.word.clone(),
        word_ascii: h::pali_to_ascii(Some(&x.word)),
        language: Some(lang.to_string()),
        definition_plain: x.definition_plain.clone(),
        definition_html: x.definition_html.clone(),
        synonyms: Some(syn.join(", ")),
        // add missing data
        uid,
        dict_label: dictionary_label.to_string(),
        dictionary_id,
        word_nom_sg: None,
        inflections: None,
        phonetic: None,
        transliteration: None,
        meaning_order: None,
        summary: None,
        antonyms: None,
        homonyms: None,
        also_written_as: None,
        see_also: None,
    }
}

/// Parses a WordDefinition, extracting plain text and HTML definitions
/// based on segment types ('m' and 'h').
///
/// It processes the *first* segment of type 'm' or 'h' found.
fn parse_word(word_def: &WordDefinition) -> DictEntry {
    let mut parsed_data = DictEntry {
        // Apply consistent_niggahita to the main word itself
        word: h::consistent_niggahita(Some(word_def.word.clone())),
        ..Default::default()
    };

    // Iterate through segments to find the first relevant one ('m' or 'h')
    for segment in &word_def.segments {
        let clean_text = h::consistent_niggahita(Some(segment.text.clone()));

        match segment.types.as_str() {
            "m" => {
                // Found plain text definition (already owned String)
                parsed_data.definition_plain = Some(clean_text);
                // Stop after finding the first 'm' segment
                // TODO: Check if there are more
                break;
            }
            "h" => {
                // Found HTML definition (parse returns owned String)
                let html_def = parse_bword_links_to_ssp(&clean_text);
                parsed_data.definition_html = Some(html_def);
                // Also generate a plain text version
                parsed_data.definition_plain = Some(h::compact_rich_text(&clean_text));
                // Stop after finding the first 'h' segment
                // TODO: Check if there are more
                break;
            }
            _ => {
                warn(&format!(
                    "Segment type '{}' is not handled for word '{}'",
                    segment.types, parsed_data.word
                ));
            }
        }
    }

    if parsed_data.definition_plain.is_none() && parsed_data.definition_html.is_none() {
        warn(&format!(
            "No 'm' or 'h' type definition found for word: {}",
            parsed_data.word
        ));
    }

    // TODO: handle synonyms
    parsed_data.synonyms = Vec::new();

    parsed_data
}

fn parse_dict(dict: &mut stardict::StarDictStd,
              dictionary_id: i32,
              new_dict_label: &str,
              lang: &str,
              limit: Option<usize>) -> Vec<NewDictWord> {
    let mut words_to_insert: Vec<NewDictWord> = Vec::with_capacity(dict.idx.items.len());

    let max_n = if let Some(n) = limit { n } else { dict.idx.items.len() };
    let mut n: usize = 0;

    // NOTE: items is a HashMap, so entries are not sorted
    for (word, idx_entry) in &dict.idx.items {
        if n >= max_n {
            break;
        } else {
            n += 1;
        }
        let def_result = dict.dict.get_definition(idx_entry, &dict.ifo);

        match def_result {
            Ok(Some(def)) => {
                // Parse the word definition using the new logic (returns owned data)
                let dict_entry = parse_word(&def);
                words_to_insert.push(db_entries(&dict_entry, dictionary_id, new_dict_label, lang));
            }
            Ok(None) => {
                warn(&format!("No definitions found for index entry: {}", word));
            }
            Err(e) => {
                error(&format!("Failed to get definition for {}: {}", word, e));
                // Decide whether to stop or continue; here we continue
            }
        }
    }

    // Disambiguate colliding uids. Two stardict index entries can produce the
    // same uid when they differ only in characters normalized away by
    // `consistent_niggahita` (ṃ/ṁ/ŋ) or by `.trim()` in db_entries(). The
    // 2026 DPD goldendict bundle introduced such collisions; older bundles
    // had none.
    //
    // Convention: append ` N` before the `/{label}` suffix for the 2nd, 3rd,
    // … occurrence — same shape DPD itself uses for multi-meaning headwords
    // (e.g. `dhamma 1.01/dpd`), and matched by the user-query disambiguator
    // at helpers.rs:85.
    let label_suffix = format!("/{}", new_dict_label);
    let mut seen: HashMap<String, u32> = HashMap::with_capacity(words_to_insert.len());
    let mut collisions: u32 = 0;
    for nw in words_to_insert.iter_mut() {
        let count = seen.entry(nw.uid.clone()).or_insert(0);
        *count += 1;
        if *count > 1 {
            collisions += 1;
            let base = nw.uid.strip_suffix(&label_suffix).unwrap_or(&nw.uid).to_string();
            let new_uid = format!("{} {}{}", base, *count, label_suffix);
            warn(&format!(
                "Stardict uid collision: '{}' (occurrence {}) -> '{}'",
                nw.uid, *count, new_uid
            ));
            nw.uid = new_uid;
        }
    }
    if collisions > 0 {
        info(&format!(
            "Disambiguated {} colliding stardict uid(s) for '{}'",
            collisions, new_dict_label
        ));
    }

    words_to_insert
}

/// SQL-only StarDict import.
///
/// - Parses the `.ifo` + index, builds `dict_words`, inserts them in chunks.
/// - Does NOT touch FTS5 / Tantivy. Index writes are owned by the startup
///   reconciliation pass (see `dict_index_reconcile`).
/// - When `description` is `Some`, that value is stored on the new
///   `dictionaries` row. When `description` is `None` and `is_user_imported`
///   is true, the value is taken from the `.ifo` `description=` field if
///   present (trimmed); otherwise NULL.
/// - `progress` receives stage updates; pass `|_| {}` for no-op.
/// - `indexed_at` is always set to `NULL` so the next startup picks the
///   dictionary up for re-indexing.
/// `physical_stem` is the basename of the `.ifo`/`.idx`/`.dict` files inside
/// `unzipped_dir` (i.e. how the StarDict archive names them). `new_dict_label`
/// is the logical label stored on the `dictionaries` row and used as the
/// `{word}/{label}` uid suffix; the two may differ when the user picks a custom
/// label at import time.
pub fn import_stardict_as_new(
    unzipped_dir: &Path,
    lang: &str,
    physical_stem: &str,
    new_dict_label: &str,
    _ignore_synonyms: bool,
    delete_if_exists: bool,
    limit: Option<usize>,
    is_user_imported: bool,
    description: Option<&str>,
    progress: &dyn Fn(StardictImportProgress),
) -> Result<i32, String> {
    use crate::db::dictionaries_models::NewDictionary;

    let app_data = get_app_data();

    progress(StardictImportProgress::Parsing);

    let ifo_path = unzipped_dir.join(format!("{}.ifo", physical_stem));
    let ifo = match Ifo::new(ifo_path.clone()) {
        Ok(x) => x,
        Err(e) => {
            let msg = format!("Error parsing ifo: {}, {}", ifo_path.to_string_lossy(), e);
            progress(StardictImportProgress::Failed { msg: msg.clone() });
            return Err(msg);
        }
    };

    if delete_if_exists {
        // Delete dictionary. Associated words are dropped due to cascade.
        match app_data.dbm.dictionaries.delete_dictionary_by_label(new_dict_label) {
            Ok(n) => info(&format!("Deleted {} dictionary.", n)),
            Err(e) => {
                let msg = format!("Error deleting: {}", e);
                progress(StardictImportProgress::Failed { msg: msg.clone() });
                return Err(msg);
            }
        };
    }

    // Resolve description: caller-supplied wins; else fall back to the
    // `.ifo` description for user-imported dictionaries.
    let ifo_description: Option<String> = if description.is_some() {
        None
    } else if is_user_imported {
        let t = ifo.description.trim();
        if t.is_empty() { None } else { Some(t.to_string()) }
    } else {
        None
    };

    // Add the dictionary, store id
    let lang_opt = if lang.is_empty() { None } else { Some(lang) };
    let new_dict = NewDictionary {
        label: new_dict_label,
        title: &ifo.bookname,
        dict_type: "stardict",
        creator: None, // TODO: parse from ifo
        description: description.or(ifo_description.as_deref()),
        feedback_email: None,
        feedback_url: None,
        version: None,
        is_user_imported,
        language: if is_user_imported { lang_opt } else { None },
        indexed_at: None,
    };

    let dictionary_id = match app_data.dbm.dictionaries.create_dictionary(new_dict) {
        Ok(x) => x.id,
        Err(e) => {
            let msg = format!("{}", e);
            progress(StardictImportProgress::Failed { msg: msg.clone() });
            return Err(msg);
        }
    };

    // Add dict_word entries, assign dictionary_id to make them related

    // Load the dictionary data
    let mut dict = match stardict::no_cache(ifo_path) {
        Ok(x) => x,
        Err(e) => {
            // Roll back the dictionaries row we just inserted so the DB stays clean.
            let _ = app_data.dbm.dictionaries.delete_dictionary_by_label(new_dict_label);
            let msg = format!("Error loading stardict dictionary: {}", e);
            progress(StardictImportProgress::Failed { msg: msg.clone() });
            return Err(msg);
        }
    };

    info(&format!("Importing {}, {} total entries ...", &ifo.bookname, dict.idx.items.len()));

    let words_to_insert = parse_dict(&mut dict, dictionary_id, new_dict_label, lang, limit);
    let total = words_to_insert.len();

    info(&format!("Inserting {} words into the database via batch...", total));
    progress(StardictImportProgress::InsertingWords { done: 0, total });

    let _lock = app_data.dbm.dictionaries.write_lock.lock();
    let db_conn = &mut app_data.dbm.dictionaries.get_conn().map_err(|e| {
        let msg = format!("{}", e);
        progress(StardictImportProgress::Failed { msg: msg.clone() });
        msg
    })?;

    let chunk_size = 5000;
    let mut inserted: usize = 0;
    let insert_result = db_conn.transaction::<_, diesel::result::Error, _>(|transaction_conn| {
        for chunk in words_to_insert.chunks(chunk_size) {
            let batch_result = db::dictionaries::create_dict_words_batch(transaction_conn, chunk);
            if let Err(err) = batch_result {
                error(&format!("Batch insertion failed for chunk. Error: {}", err));
                return Err(err);
            }
            inserted += chunk.len();
            // Note: progress callback is called outside of transaction lifetime
            // safety — but it's a `Fn`, so we just emit the count we've committed
            // in this transaction so far. The callback must not touch the DB.
            progress(StardictImportProgress::InsertingWords { done: inserted, total });
        }
        Ok(())
    });

    match insert_result {
        Ok(_) => {},
        Err(e) => {
            // Transaction automatically rolled back on error.
            // Also drop the parent dictionaries row so we don't leak an empty entry.
            drop(_lock);
            let _ = app_data.dbm.dictionaries.delete_dictionary_by_label(new_dict_label);
            let msg = format!("Batch insertion failed: {}", e);
            progress(StardictImportProgress::Failed { msg: msg.clone() });
            return Err(msg);
        }
    }

    info(&format!("Import finished for '{}'.", &ifo.bookname));
    progress(StardictImportProgress::Done);

    Ok(dictionary_id)
}
