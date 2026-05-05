use std::path::Path;
use anyhow::{Result, Context};
use diesel::prelude::*;
use diesel::sql_types::{Text, Nullable, Integer};
use diesel::SqliteConnection;

use simsapa_backend::logger;
use simsapa_backend::db::dictionaries_models::{NewDictionary, NewDictWord};
use simsapa_backend::db::dictionaries_schema::dictionaries;
use simsapa_backend::helpers::compact_rich_text;
use crate::bootstrap::create_database_connection;

#[derive(QueryableByName)]
struct SourceDictWord {
    #[diesel(sql_type = Text)]
    uid: String,
    #[diesel(sql_type = Text)]
    word: String,
    #[diesel(sql_type = Text)]
    word_ascii: String,
    #[diesel(sql_type = Nullable<Text>)]
    language: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    word_nom_sg: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    inflections: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    phonetic: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    transliteration: Option<String>,
    #[diesel(sql_type = Nullable<Integer>)]
    meaning_order: Option<i32>,
    #[diesel(sql_type = Nullable<Text>)]
    definition_html: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    summary: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    synonyms: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    antonyms: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    homonyms: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    also_written_as: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    see_also: Option<String>,
}

pub fn dppn_bootstrap(bootstrap_assets_dir: &Path, assets_dir: &Path) -> Result<()> {
    logger::info("=== dppn_bootstrap() ===");

    let dppn_source_path = bootstrap_assets_dir.join("dppn-anandajoti/dppn.sqlite3");
    let dict_db_path = assets_dir.join("dictionaries.sqlite3");

    if !dppn_source_path.exists() {
        return Err(anyhow::anyhow!(
            "DPPN source database not found at: {}",
            dppn_source_path.display()
        ));
    }

    // Connect to the dictionaries database
    let mut dict_conn = create_database_connection(&dict_db_path)
        .context("Failed to connect to dictionaries.sqlite3")?;

    // Create the DPPN dictionary entry
    let new_dict = NewDictionary {
        label: "dppn",
        title: "Dictionary of Pāli Proper Names (Revised 2025)",
        dict_type: "sql",
        creator: Some("Ānandajoti Bhikkhu"),
        language: Some("en"),
        is_user_imported: false,
        indexed_at: None,
        ..Default::default()
    };

    let dict_id: i32 = diesel::insert_into(dictionaries::table)
        .values(&new_dict)
        .returning(dictionaries::id)
        .get_result(&mut dict_conn)
        .context("Failed to insert DPPN dictionary entry")?;

    logger::info(&format!("Created DPPN dictionary entry with id={}", dict_id));

    // Connect to the source DPPN database
    let source_db_url = dppn_source_path.to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid DPPN source path"))?;
    let mut source_conn = SqliteConnection::establish(source_db_url)
        .context("Failed to connect to DPPN source database")?;

    // Query all dict_words from the source
    let source_words: Vec<SourceDictWord> = diesel::sql_query(
        "SELECT uid, word, word_ascii, language, word_nom_sg, inflections,
                phonetic, transliteration, meaning_order, definition_html,
                summary, synonyms, antonyms, homonyms,
                also_written_as, see_also
         FROM dict_words"
    )
        .get_results(&mut source_conn)
        .context("Failed to query DPPN dict_words")?;

    logger::info(&format!("Found {} entries in DPPN source", source_words.len()));

    // Batch insert into dictionaries.sqlite3
    let batch_size = 1000;
    let mut total_inserted = 0;

    for chunk in source_words.chunks(batch_size) {
        let new_words: Vec<NewDictWord> = chunk.iter().map(|w| {
            NewDictWord {
                dictionary_id: dict_id,
                dict_label: "dppn".to_string(),
                uid: w.uid.clone(),
                word: w.word.clone(),
                word_ascii: w.word_ascii.clone(),
                language: w.language.clone(),
                word_nom_sg: w.word_nom_sg.clone(),
                inflections: w.inflections.clone(),
                phonetic: w.phonetic.clone(),
                transliteration: w.transliteration.clone(),
                meaning_order: w.meaning_order,
                definition_plain: w.definition_html.as_ref().map(|html| compact_rich_text(html)),
                definition_html: w.definition_html.clone(),
                summary: w.summary.clone(),
                synonyms: w.synonyms.clone(),
                antonyms: w.antonyms.clone(),
                homonyms: w.homonyms.clone(),
                also_written_as: w.also_written_as.clone(),
                see_also: w.see_also.clone(),
            }
        }).collect();

        let inserted = diesel::insert_into(simsapa_backend::db::dictionaries_schema::dict_words::table)
            .values(&new_words)
            .execute(&mut dict_conn)
            .context("Failed to batch insert DPPN dict_words")?;

        total_inserted += inserted;

        if total_inserted % 5000 == 0 {
            logger::info(&format!("Inserted {} DPPN entries...", total_inserted));
        }
    }

    logger::info(&format!("Successfully imported {} DPPN entries", total_inserted));

    Ok(())
}
