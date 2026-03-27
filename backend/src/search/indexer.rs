use std::path::Path;

use anyhow::Result;
use diesel::prelude::*;
use tantivy::{doc, Directory, Index, IndexWriter};

use crate::db::DatabaseHandle;
use crate::db::appdata_models::Sutta;
use crate::db::dictionaries_models::DictWord;
use crate::logger::info;
use crate::AppGlobalPaths;

use super::schema::{build_dict_schema, build_sutta_schema};
use super::tokenizer::register_tokenizers;

/// Open or create a Tantivy index at the given directory path.
pub fn open_or_create_index(dir: &Path, schema: tantivy::schema::Schema, lang: &str) -> Result<Index> {
    match dir.try_exists() {
        Ok(true) => {}
        _ => {
            std::fs::create_dir_all(dir)?;
        }
    }

    let mmap_dir = tantivy::directory::MmapDirectory::open(dir)?;
    let index = Index::open_or_create(mmap_dir, schema)?;

    register_tokenizers(&index, lang);

    Ok(index)
}

/// Build fulltext index for suttas of a given language.
pub fn build_sutta_index(appdata_db: &DatabaseHandle, index_dir: &Path, lang: &str) -> Result<()> {
    use crate::db::appdata_schema::suttas::dsl::*;

    info(&format!("Building sutta index for language: {}", lang));

    let lang_index_dir = index_dir.join(lang);
    let schema = build_sutta_schema(lang);
    let index = open_or_create_index(&lang_index_dir, schema, lang)?;

    let mut writer: IndexWriter = index.writer(50_000_000)?;

    // Clear existing documents before rebuilding
    writer.delete_all_documents()?;
    writer.commit()?;

    let schema = index.schema();
    let uid_field = schema.get_field("uid").unwrap();
    let title_field = schema.get_field("title").unwrap();
    let language_field = schema.get_field("language").unwrap();
    let source_uid_field = schema.get_field("source_uid").unwrap();
    let sutta_ref_field = schema.get_field("sutta_ref").unwrap();
    let nikaya_field = schema.get_field("nikaya").unwrap();
    let content_field = schema.get_field("content").unwrap();
    let content_exact_field = schema.get_field("content_exact").unwrap();

    let lang_clone = lang.to_string();
    let sutta_list: Vec<Sutta> = appdata_db.do_read(|db_conn| {
        suttas
            .filter(language.eq(&lang_clone))
            .select(Sutta::as_select())
            .load(db_conn)
    })?;

    info(&format!("Indexing {} suttas for language {}", sutta_list.len(), lang));

    let mut indexed_count = 0;
    for sutta in &sutta_list {
        let plain = sutta.content_plain.as_deref().unwrap_or("");
        if plain.is_empty() {
            continue;
        }

        // Prepend sutta_ref, title, and title_pali to content for better matching
        let sref = &sutta.sutta_ref;
        let t = sutta.title.as_deref().unwrap_or("");
        let tp = sutta.title_pali.as_deref().unwrap_or("");
        let content_text = format!("{} {} {} {}", sref, t, tp, plain);

        let source = sutta.source_uid.as_deref().unwrap_or("");

        writer.add_document(doc!(
            uid_field => sutta.uid.as_str(),
            title_field => t,
            language_field => sutta.language.as_str(),
            source_uid_field => source,
            sutta_ref_field => sref.as_str(),
            nikaya_field => sutta.nikaya.as_str(),
            content_field => content_text.as_str(),
            content_exact_field => content_text.as_str(),
        ))?;

        indexed_count += 1;
    }

    // Finalize the index:
    // 1. commit() persists documents and makes them searchable.
    // 2. wait_merging_threads() blocks until background merges finish. Since it takes 'self',
    //    it consumes the writer and explicitly releases the INDEX_WRITER_LOCK.
    // 3. sync_directory() ensures the OS flushes directory metadata, preventing "file changed"
    //    errors during external archival (tar).
    writer.commit()?;
    writer.wait_merging_threads()?;
    index.directory().sync_directory()?;

    info(&format!("Sutta index committed: {} documents for language {}", indexed_count, lang));

    Ok(())
}

/// Build fulltext index for dictionary words of a given language.
pub fn build_dict_index(dict_db: &DatabaseHandle, index_dir: &Path, lang: &str) -> Result<()> {
    use crate::db::dictionaries_schema::dict_words::dsl::*;

    info(&format!("Building dict_words index for language: {}", lang));

    let lang_index_dir = index_dir.join(lang);
    let schema = build_dict_schema(lang);
    let index = open_or_create_index(&lang_index_dir, schema, lang)?;

    let mut writer: IndexWriter = index.writer(50_000_000)?;

    // Clear existing documents before rebuilding
    writer.delete_all_documents()?;
    writer.commit()?;

    let schema = index.schema();
    let uid_field = schema.get_field("uid").unwrap();
    let word_field = schema.get_field("word").unwrap();
    let synonyms_field = schema.get_field("synonyms").unwrap();
    let language_field = schema.get_field("language").unwrap();
    let source_uid_field = schema.get_field("source_uid").unwrap();
    let content_field = schema.get_field("content").unwrap();
    let content_exact_field = schema.get_field("content_exact").unwrap();

    let lang_clone = lang.to_string();
    let word_list: Vec<DictWord> = dict_db.do_read(|db_conn| {
        dict_words
            .filter(language.eq(&lang_clone))
            .select(DictWord::as_select())
            .load(db_conn)
    })?;

    info(&format!("Indexing {} dict_words for language {}", word_list.len(), lang));

    let mut indexed_count = 0;
    for dw in &word_list {
        let def = dw.definition_plain.as_deref().unwrap_or("");
        if def.is_empty() {
            continue;
        }

        // Prepend word and synonyms to content for better matching
        let w = &dw.word;
        let syn = dw.synonyms.as_deref().unwrap_or("");
        let content_text = format!("{} {} {}", w, syn, def);

        let lang_val = dw.language.as_deref().unwrap_or("");

        writer.add_document(doc!(
            uid_field => dw.uid.as_str(),
            word_field => w.as_str(),
            synonyms_field => syn,
            language_field => lang_val,
            source_uid_field => dw.dict_label.as_str(),
            content_field => content_text.as_str(),
            content_exact_field => content_text.as_str(),
        ))?;

        indexed_count += 1;
    }

    // Finalize the index:
    // See build_sutta_index for details on why wait_merging_threads and sync_directory are used.
    writer.commit()?;
    writer.wait_merging_threads()?;
    index.directory().sync_directory()?;

    info(&format!("Dict_words index committed: {} documents for language {}", indexed_count, lang));

    Ok(())
}

/// Get distinct sutta languages from the appdata database.
pub fn get_sutta_languages(appdata_db: &DatabaseHandle) -> Result<Vec<String>> {
    use crate::db::appdata_schema::suttas::dsl::*;

    let langs = appdata_db.do_read(|db_conn| {
        suttas
            .select(language)
            .distinct()
            .load::<String>(db_conn)
    })?;

    Ok(langs
        .into_iter()
        .filter(|l| !l.is_empty())
        .map(|l| l.to_lowercase())
        .collect())
}

/// Get distinct dict_word languages from the dictionaries database.
pub fn get_dict_word_languages(dict_db: &DatabaseHandle) -> Result<Vec<String>> {
    use crate::db::dictionaries_schema::dict_words::dsl::*;

    let langs = dict_db.do_read(|db_conn| {
        dict_words
            .select(language)
            .filter(language.is_not_null())
            .distinct()
            .load::<Option<String>>(db_conn)
    })?;

    Ok(langs
        .into_iter()
        .flatten()
        .filter(|l| !l.is_empty())
        .map(|l| l.to_lowercase())
        .collect())
}

/// Build all fulltext indexes for all languages found in both databases.
pub fn build_all_indexes(
    appdata_db: &DatabaseHandle,
    dict_db: &DatabaseHandle,
    paths: &AppGlobalPaths,
) -> Result<()> {
    info("Building all fulltext indexes...");

    let sutta_langs = get_sutta_languages(appdata_db)?;
    info(&format!("Sutta languages: {:?}", sutta_langs));

    for lang in &sutta_langs {
        build_sutta_index(appdata_db, &paths.suttas_index_dir, lang)?;
    }

    let dict_langs = get_dict_word_languages(dict_db)?;
    info(&format!("Dict_word languages: {:?}", dict_langs));

    for lang in &dict_langs {
        build_dict_index(dict_db, &paths.dict_words_index_dir, lang)?;
    }

    write_version_file(&paths.index_dir)?;

    info("All fulltext indexes built successfully.");
    Ok(())
}

// ---------------------------------------------------------------------------
// Index versioning
// ---------------------------------------------------------------------------

pub const INDEX_VERSION: &str = "1.0";

/// Write a VERSION file to the index directory.
pub fn write_version_file(index_dir: &Path) -> Result<()> {
    match index_dir.try_exists() {
        Ok(true) => {}
        _ => {
            std::fs::create_dir_all(index_dir)?;
        }
    }

    let version_path = index_dir.join("VERSION");
    std::fs::write(&version_path, INDEX_VERSION)?;

    info(&format!("Wrote index VERSION file: {}", version_path.display()));
    Ok(())
}

/// Read the VERSION file from the index directory.
pub fn read_version_file(index_dir: &Path) -> Result<String> {
    let version_path = index_dir.join("VERSION");
    let version = std::fs::read_to_string(version_path)?;
    Ok(version.trim().to_string())
}

/// Check if the index is current (VERSION file matches INDEX_VERSION).
pub fn is_index_current(index_dir: &Path) -> bool {
    match index_dir.try_exists() {
        Ok(true) => {}
        _ => return false,
    }

    match read_version_file(index_dir) {
        Ok(version) => version == INDEX_VERSION,
        Err(_) => false,
    }
}
