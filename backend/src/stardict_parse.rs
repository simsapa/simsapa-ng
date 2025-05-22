use std::path::Path;

use diesel::prelude::*;

use stardict::{self, Ifo, WordDefinition};

use crate::{db::DbManager, helpers as h, db::dictionaries_models::NewDictWord};

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
        source_uid: None,
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
                eprintln!(
                    "Segment type '{}' is not handled for word '{}'",
                    segment.types, parsed_data.word
                );
            }
        }
    }

    if parsed_data.definition_plain.is_none() && parsed_data.definition_html.is_none() {
        eprintln!(
            "[WARN] No 'm' or 'h' type definition found for word: {}",
            parsed_data.word
        );
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
        let def_result = dict.dict.get_definition(&idx_entry, &dict.ifo);

        match def_result {
            Ok(Some(def)) => {
                // Parse the word definition using the new logic (returns owned data)
                let dict_entry = parse_word(&def);
                words_to_insert.push(db_entries(&dict_entry, dictionary_id, new_dict_label, lang));
            }
            Ok(None) => {
                eprintln!("[WARN] No definitions found for index entry: {}", word);
            }
            Err(e) => {
                eprintln!("[ERROR] Failed to get definition for {}: {}", word, e);
                // Decide whether to stop or continue; here we continue
            }
        }
    }

    words_to_insert
}

pub fn import_stardict_as_new(dbm: &DbManager,
                              unzipped_dir: &Path,
                              lang: &str,
                              new_dict_label: &str,
                              _ignore_synonyms: bool,
                              delete_if_exists: bool,
                              limit: Option<usize>)
                              -> Result<(), String> {
    use crate::db::dictionaries_models::NewDictionary;
    use crate::db;

    let ifo_path = unzipped_dir.join(format!("{}.ifo", new_dict_label));
    let ifo = match Ifo::new(ifo_path.clone()) {
        Ok(x) => x,
        Err(e) => return Err(format!("Error parsing ifo: {}, {}", ifo_path.to_string_lossy(), e)),
    };

    if delete_if_exists {
        // Delete dictionary. Associated words are dropped due to cascade.
        match dbm.dictionaries.delete_dictionary_by_label(new_dict_label) {
            Ok(n) => println!("Deleted {} dictionary.", n),
            Err(e) => return Err(format!("Error deleting: {}", e)),
        };
    }

    // Add the dictionary, store id
    let new_dict = NewDictionary {
        label: new_dict_label,
        title: &ifo.bookname,
        dict_type: "stardict",
        creator: None, // TODO: parse from ifo
        description: None,
        feedback_email: None,
        feedback_url: None,
        version: None,
    };

    let dictionary_id = match dbm.dictionaries.create_dictionary(new_dict) {
        Ok(x) => x.id,
        Err(e) => return Err(format!("{}", e)),
    };

    // Add dict_word entries, assign dictionary_id to make them related

    // Load the dictionary data
    let mut dict = match stardict::no_cache(ifo_path) {
        Ok(x) => x,
        Err(e) => return Err(format!("Error loading stardict dictionary: {}", e)),
    };

    println!("Importing {}, {} total entries ...", &ifo.bookname, dict.idx.items.len());

    let words_to_insert = parse_dict(&mut dict, dictionary_id, new_dict_label, lang, limit);

    println!("Inserting {} ...", words_to_insert.len());

    println!("Inserting {} words into the database via batch...", words_to_insert.len());

    let _lock = dbm.dictionaries.write_lock.lock();
    let db_conn = &mut dbm.dictionaries.get_conn().map_err(|e| format!("{}", e))?;

    let insert_result = db_conn.transaction::<_, diesel::result::Error, _>(|transaction_conn| {
        let chunk_size = 5000;
        for chunk in words_to_insert.chunks(chunk_size) {
            println!("Inserting chunk of size {}", chunk.len());
            // Explicitly handle the result of the batch insert
            let batch_result = db::dictionaries::create_dict_words_batch(transaction_conn, chunk);
            if let Err(err) = batch_result {
                // If an error occurs, collect UIDs from the failed chunk
                eprintln!("Batch insertion failed for chunk. Error: {}", err);
                // let failed_uids: Vec<String> = chunk.iter()
                //                                     .map(|word| word.uid.clone())
                //                                     .collect();
                // eprintln!("UIDs in failing chunk: {:?}", failed_uids);
                // Return the error to roll back the transaction
                return Err(err);
            }
            // If Ok, loop continues
        }
        Ok(()) // Commit transaction if all chunks succeeded
    });

    match insert_result {
        Ok(_) => {},
        Err(e) => {
            // Transaction automatically rolled back on error
            return Err(format!("Batch insertion failed: {}", e));
        }
    }

    println!("Import finished for '{}'.", &ifo.bookname);

    Ok(())
}
