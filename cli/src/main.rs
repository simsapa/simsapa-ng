mod bootstrap;
mod bootstrap_old;
mod tipitaka_xml_parser_tsv;
mod tipitaka_xml_parser;

use std::path::{Path, PathBuf};
use std::process::exit;

use clap::{Parser, Subcommand, ValueEnum};
use dotenvy::dotenv;
use anyhow::Result;

use simsapa_backend::{db, init_app_data, get_app_data, get_create_simsapa_dir, logger};
use simsapa_backend::types::{SearchArea, SearchMode, SearchParams, SearchResult};
use simsapa_backend::query_task::SearchQueryTask;
use simsapa_backend::stardict_parse::import_stardict_as_new;
use simsapa_backend::db::appdata_models::Sutta;

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use indexmap::IndexMap;

fn get_query_results(query: &str, area: SearchArea) -> Vec<SearchResult> {
    let app_data = get_app_data();

    let params = SearchParams {
        mode: SearchMode::ContainsMatch,
        page_len: None,
        lang: Some("en".to_string()),
        lang_include: true,
        source: None,
        source_include: true,
        enable_regex: false,
        fuzzy_distance: 0,
    };

    let mut query_task = SearchQueryTask::new(
        &app_data.dbm,
        "en".to_string(),
        query.to_string(),
        params,
        area,
    );

    let results = match query_task.results_page(0) {
        Ok(x) => x,
        Err(s) => {
            panic!("{}", s);
        }
    };

    results
}

fn query_suttas(
    query: &str,
    print_titles: bool,
    print_count: bool,
) -> Result<(), String> {

    let results = get_query_results(query, SearchArea::Suttas);

    if print_titles {
        for i in results.iter() {
            println!("{}: {}", i.uid, i.title);
        }
    }
    if print_count {
        println!("{}", results.len());
    }

    Ok(())
}

fn query_words(
    query: &str,
    print_titles: bool,
    print_count: bool,
) -> Result<(), String> {
    let results = get_query_results(query, SearchArea::Dictionary);

    if print_titles {
        for i in results.iter() {
            println!("{}: {}", i.uid, i.title);
        }
    }
    if print_count {
        println!("{}", results.len());
    }

    Ok(())
}

/// Simulates importing a dictionary into a specific database file.
fn import_stardict_dictionary(new_dict_label: &str,
                              unzipped_dir: &Path,
                              limit: Option<usize>)
                              -> Result<(), String> {
    import_stardict_as_new(unzipped_dir, "pli", new_dict_label, true, true, limit)?;
    Ok(())
}

/// Export Dhammapada Tipitaka.net suttas from legacy database
fn export_dhammapada_tipitaka_net(legacy_db_path: &Path, output_db_path: &Path) -> Result<(), String> {
    use simsapa_backend::db::appdata_schema::suttas;

    println!("Exporting Dhammapada Tipitaka.net suttas from legacy database...");
    println!("Legacy DB: {:?}", legacy_db_path);
    println!("Output DB: {:?}", output_db_path);

    // Check if legacy database exists
    if !legacy_db_path.exists() {
        return Err(format!("Legacy database not found: {:?}", legacy_db_path));
    }

    // Connect to legacy database
    let mut legacy_conn = SqliteConnection::establish(legacy_db_path.to_str().unwrap())
        .map_err(|e| format!("Failed to connect to legacy database: {}", e))?;

    // Query suttas with uid LIKE '%/daw'
    let daw_suttas: Vec<Sutta> = suttas::table
        .filter(suttas::uid.like("%/daw"))
        .order(suttas::uid.asc())
        .load(&mut legacy_conn)
        .map_err(|e| format!("Failed to query suttas: {}", e))?;

    println!("Found {} suttas with uid ending in '/daw'", daw_suttas.len());

    // Verify exactly 26 rows
    if daw_suttas.len() != 26 {
        return Err(format!("Expected exactly 26 suttas, found {}", daw_suttas.len()));
    }

    // Delete output database if it exists
    if output_db_path.exists() {
        std::fs::remove_file(output_db_path)
            .map_err(|e| format!("Failed to delete existing output database: {}", e))?;
    }

    // Create output database
    let mut output_conn = SqliteConnection::establish(output_db_path.to_str().unwrap())
        .map_err(|e| format!("Failed to create output database: {}", e))?;

    // Run migrations on output database
    println!("Creating database schema...");
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("../backend/migrations/appdata");
    output_conn.run_pending_migrations(MIGRATIONS)
        .map_err(|e| format!("Failed to run migrations: {}", e))?;

    // Insert suttas into output database (excluding id field)
    println!("Inserting suttas into output database...");
    for sutta in &daw_suttas {
        diesel::insert_into(suttas::table)
            .values((
                suttas::uid.eq(&sutta.uid),
                suttas::sutta_ref.eq(&sutta.sutta_ref),
                suttas::nikaya.eq(&sutta.nikaya),
                suttas::language.eq(&sutta.language),
                suttas::group_path.eq(&sutta.group_path),
                suttas::group_index.eq(&sutta.group_index),
                suttas::order_index.eq(&sutta.order_index),
                suttas::sutta_range_group.eq(&sutta.sutta_range_group),
                suttas::sutta_range_start.eq(&sutta.sutta_range_start),
                suttas::sutta_range_end.eq(&sutta.sutta_range_end),
                suttas::title.eq(&sutta.title),
                suttas::title_ascii.eq(&sutta.title_ascii),
                suttas::title_pali.eq(&sutta.title_pali),
                suttas::title_trans.eq(&sutta.title_trans),
                suttas::description.eq(&sutta.description),
                suttas::content_plain.eq(&sutta.content_plain),
                suttas::content_html.eq(&sutta.content_html),
                suttas::content_json.eq(&sutta.content_json),
                suttas::content_json_tmpl.eq(&sutta.content_json_tmpl),
                suttas::source_uid.eq(&sutta.source_uid),
                suttas::source_info.eq(&sutta.source_info),
                suttas::source_language.eq(&sutta.source_language),
                suttas::message.eq(&sutta.message),
                suttas::copyright.eq(&sutta.copyright),
                suttas::license.eq(&sutta.license),
            ))
            .execute(&mut output_conn)
            .map_err(|e| format!("Failed to insert sutta {}: {}", sutta.uid, e))?;
    }

    println!("✓ Successfully exported {} suttas to {:?}", daw_suttas.len(), output_db_path);

    // Print UIDs for verification
    println!("\nExported suttas:");
    for sutta in &daw_suttas {
        println!("  - {}", sutta.uid);
    }

    Ok(())
}

/// List available languages in SuttaCentral ArangoDB
fn suttacentral_import_languages_list() -> Result<(), String> {
    use bootstrap::suttacentral::{connect_to_arangodb, get_sorted_languages_list};

    // Connect to ArangoDB
    let db = connect_to_arangodb()
        .map_err(|e| format!("Failed to connect to ArangoDB: {}", e))?;

    // Get sorted languages list
    let languages = get_sorted_languages_list(&db)
        .map_err(|e| format!("Failed to get languages list: {}", e))?;

    // Print the languages
    println!("Available languages in SuttaCentral ArangoDB:");
    println!("(excluding: en, pli, san, hu)\n");
    for lang in &languages {
        println!("  {}", lang);
    }
    println!("\nTotal: {} languages", languages.len());

    Ok(())
}

/// Generate statistics for an appdata.sqlite3 database
fn appdata_stats(db_path: &Path, output_folder: Option<&Path>, write_stats: bool) -> Result<(), String> {
    use std::fs;

    // Define helper structs for raw SQL queries
    #[derive(Debug, QueryableByName)]
    struct CountResult {
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        count: i64,
    }

    #[derive(Debug, QueryableByName)]
    struct LanguageCount {
        #[diesel(sql_type = diesel::sql_types::Text)]
        language: String,
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        count: i64,
    }

    // Check if database exists
    if !db_path.exists() {
        return Err(format!("Database file not found: {:?}", db_path));
    }

    // Connect to the database
    let mut conn = SqliteConnection::establish(db_path.to_str().unwrap())
        .map_err(|e| format!("Failed to connect to database: {}", e))?;

    let mut stats: IndexMap<String, String> = IndexMap::new();

    // Helper function to check if a table exists
    let table_exists = |conn: &mut SqliteConnection, table_name: &str| -> bool {
        let query = format!(
            "SELECT COUNT(*) as count FROM sqlite_master WHERE type='table' AND name='{}'",
            table_name
        );
        diesel::sql_query(&query)
            .get_result::<CountResult>(conn)
            .map(|r| r.count > 0)
            .unwrap_or(false)
    };

    // Helper function to get row count for a table
    let get_row_count = |conn: &mut SqliteConnection, table_name: &str| -> Result<i64, String> {
        if !table_exists(conn, table_name) {
            return Ok(0);
        }

        let query = format!("SELECT COUNT(*) as count FROM {}", table_name);
        let result: Result<i64, diesel::result::Error> = diesel::sql_query(&query)
            .get_result::<CountResult>(conn)
            .map(|r| r.count);

        result.map_err(|e| format!("Failed to query {}: {}", table_name, e))
    };

    // Total rows in main tables
    for table in &["suttas", "sutta_variants", "sutta_glosses", "sutta_comments"] {
        let count = get_row_count(&mut conn, table)?;
        stats.insert(format!("Total rows in {}", table), count.to_string());
    }

    // Total rows in suttas_fts
    let fts_count = get_row_count(&mut conn, "suttas_fts")?;
    stats.insert("Total rows in suttas_fts".to_string(), fts_count.to_string());

    // Count distinct source_uid values
    if table_exists(&mut conn, "suttas") {
        let query = "SELECT COUNT(DISTINCT source_uid) as count FROM suttas WHERE source_uid IS NOT NULL";
        let source_uid_count: i64 = diesel::sql_query(query)
            .get_result::<CountResult>(&mut conn)
            .map(|r| r.count)
            .unwrap_or(0);
        stats.insert("Number of source_uid variants".to_string(), source_uid_count.to_string());

        // Count distinct nikaya values
        let query = "SELECT COUNT(DISTINCT nikaya) as count FROM suttas WHERE nikaya IS NOT NULL";
        let nikaya_count: i64 = diesel::sql_query(query)
            .get_result::<CountResult>(&mut conn)
            .map(|r| r.count)
            .unwrap_or(0);
        stats.insert("Number of nikaya variants".to_string(), nikaya_count.to_string());

        // Count distinct language values
        let query = "SELECT COUNT(DISTINCT language) as count FROM suttas WHERE language IS NOT NULL";
        let lang_count: i64 = diesel::sql_query(query)
            .get_result::<CountResult>(&mut conn)
            .map(|r| r.count)
            .unwrap_or(0);
        stats.insert("Number of language variants".to_string(), lang_count.to_string());

        // Count suttas with source_uid 'ms'
        let query = "SELECT COUNT(*) as count FROM suttas WHERE source_uid = 'ms'";
        let ms_count: i64 = diesel::sql_query(query)
            .get_result::<CountResult>(&mut conn)
            .map(|r| r.count)
            .unwrap_or(0);
        stats.insert("Suttas with source_uid 'ms'".to_string(), ms_count.to_string());

        // Count suttas with source_uid 'cst4'
        let query = "SELECT COUNT(*) as count FROM suttas WHERE source_uid = 'cst4'";
        let cst4_count: i64 = diesel::sql_query(query)
            .get_result::<CountResult>(&mut conn)
            .map(|r| r.count)
            .unwrap_or(0);
        stats.insert("Suttas with source_uid 'cst4'".to_string(), cst4_count.to_string());

        // Count rows per language
        let query = "SELECT language, COUNT(*) as count FROM suttas GROUP BY language ORDER BY count DESC";

        let lang_counts: Vec<LanguageCount> = diesel::sql_query(query)
            .load(&mut conn)
            .unwrap_or_default();

        for lc in lang_counts {
            stats.insert(format!("Rows for language '{}'", lc.language), lc.count.to_string());
        }

        // Count suttas from dhammatalks.org
        let query = "SELECT COUNT(*) as count FROM suttas WHERE content_html LIKE '%<div class=\"dhammatalks_org\">%'";
        let dhammatalks_count: i64 = diesel::sql_query(query)
            .get_result::<CountResult>(&mut conn)
            .map(|r| r.count)
            .unwrap_or(0);
        stats.insert("Suttas from dhammatalks.org".to_string(), dhammatalks_count.to_string());

        // Count suttas from tipitaka.net
        let query = "SELECT COUNT(*) as count FROM suttas WHERE content_html LIKE '%<div class=\"tipitaka_net\">%'";
        let tipitaka_net_count: i64 = diesel::sql_query(query)
            .get_result::<CountResult>(&mut conn)
            .map(|r| r.count)
            .unwrap_or(0);
        stats.insert("Suttas from tipitaka.net".to_string(), tipitaka_net_count.to_string());

        // Count suttas from Nyanadipa
        let query = "SELECT COUNT(*) as count FROM suttas WHERE source_uid = 'nyanadipa'";
        let nyanadipa_count: i64 = diesel::sql_query(query)
            .get_result::<CountResult>(&mut conn)
            .map(|r| r.count)
            .unwrap_or(0);
        stats.insert("Suttas from Nyanadipa".to_string(), nyanadipa_count.to_string());

        // Count suttas from Ajahn Munindo
        let query = "SELECT COUNT(*) as count FROM suttas WHERE source_uid = 'munindo'";
        let munindo_count: i64 = diesel::sql_query(query)
            .get_result::<CountResult>(&mut conn)
            .map(|r| r.count)
            .unwrap_or(0);
        stats.insert("Suttas from Ajahn Munindo".to_string(), munindo_count.to_string());

        // Count suttas from a-buddha-ujja.hu (Hungarian)
        let query = "SELECT COUNT(*) as count FROM suttas WHERE language = 'hu'";
        let buddha_ujja_count: i64 = diesel::sql_query(query)
            .get_result::<CountResult>(&mut conn)
            .map(|r| r.count)
            .unwrap_or(0);
        stats.insert("Suttas from a-buddha-ujja.hu".to_string(), buddha_ujja_count.to_string());
    }

    // Print as Markdown table
    println!("\n## Appdata Statistics\n");
    println!("| Statistic | Value |");
    println!("|-----------|-------|");
    for (key, value) in &stats {
        println!("| {} | {} |", key, value);
    }

    // Write to files if --write-stats is enabled
    if write_stats {
        // Determine the output folder
        let target_folder = match output_folder {
            Some(folder) => folder.to_path_buf(),
            None => {
                // Use the database file's parent directory
                db_path.parent()
                    .ok_or_else(|| format!("Could not determine parent directory of database: {:?}", db_path))?
                    .to_path_buf()
            }
        };

        // Create folder if it doesn't exist
        if !target_folder.exists() {
            fs::create_dir_all(&target_folder)
                .map_err(|e| format!("Failed to create output folder: {}", e))?;
        }

        // Write Markdown file
        let md_path = target_folder.join("appdata_stats.md");
        let mut md_content = String::from("# Appdata Statistics\n\n");
        md_content.push_str("| Statistic | Value |\n");
        md_content.push_str("|-----------|-------|\n");
        for (key, value) in &stats {
            md_content.push_str(&format!("| {} | {} |\n", key, value));
        }

        fs::write(&md_path, md_content)
            .map_err(|e| format!("Failed to write Markdown file: {}", e))?;
        logger::info(&format!("Wrote Markdown file: {:?}", md_path));

        // Write JSON file
        let json_path = target_folder.join("appdata_stats.json");
        let json_content = serde_json::to_string_pretty(&stats)
            .map_err(|e| format!("Failed to serialize JSON: {}", e))?;

        fs::write(&json_path, json_content)
            .map_err(|e| format!("Failed to write JSON file: {}", e))?;
        logger::info(&format!("Wrote JSON file: {:?}", json_path));
    }

    Ok(())
}

/// Parse Tipitaka XML files and import into database
fn parse_tipitaka_xml(
    input_path: &Path,
    output_db_path: &Path,
    verbose: bool,
    dry_run: bool,
) -> Result<(), String> {
    use tipitaka_xml_parser_tsv::encoding::read_xml_file;
    use tipitaka_xml_parser_tsv::xml_parser::parse_xml;
    use std::fs;
    use diesel::sqlite::SqliteConnection;
    use diesel::Connection;

    println!("Tipitaka XML Parser");
    println!("==================\n");

    // Collect XML files to process
    let xml_files: Vec<PathBuf> = if input_path.is_file() {
        println!("Processing single file: {:?}\n", input_path);
        vec![input_path.to_path_buf()]
    } else if input_path.is_dir() {
        println!("Processing folder: {:?}", input_path);
        let files: Vec<PathBuf> = fs::read_dir(input_path)
            .map_err(|e| format!("Failed to read directory: {}", e))?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext == "xml")
                    .unwrap_or(false)
            })
            .collect();

        println!("Found {} XML files\n", files.len());
        files
    } else {
        return Err(format!("Input path does not exist: {:?}", input_path));
    };

    if xml_files.is_empty() {
        return Err("No XML files found to process".to_string());
    }

    // Initialize database if not dry run
    if !dry_run {
        if output_db_path.exists() {
            println!("Output database already exists: {:?}", output_db_path);
            println!("WARNING: This will add to existing database\n");
        } else {
            println!("Creating new database: {:?}", output_db_path);

            // Create database and run migrations
            let mut conn = SqliteConnection::establish(output_db_path.to_str().unwrap())
                .map_err(|e| format!("Failed to create database: {}", e))?;

            use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
            const MIGRATIONS: EmbeddedMigrations = embed_migrations!("../backend/migrations/appdata");
            conn.run_pending_migrations(MIGRATIONS)
                .map_err(|e| format!("Failed to run migrations: {}", e))?;

            println!("✓ Database created and initialized\n");
        }
    } else {
        println!("DRY RUN MODE - No database will be created\n");
    }

    // Load CST mapping
    let tsv_path = Path::new("assets/cst-vs-sc.tsv");
    if !tsv_path.exists() {
        return Err(format!("CST mapping file not found: {:?}. Current dir: {:?}", tsv_path, std::env::current_dir()));
    }

    use tipitaka_xml_parser_tsv::TipitakaImporterUsingTSV;
    let importer = TipitakaImporterUsingTSV::new(tsv_path, verbose)
        .map_err(|e| format!("Failed to create importer: {}", e))?;

    // Get database connection if not dry run
    let mut conn_opt = if !dry_run {
        Some(SqliteConnection::establish(output_db_path.to_str().unwrap())
            .map_err(|e| format!("Failed to connect to database: {}", e))?)
    } else {
        None
    };

    // Process each XML file
    let mut total_suttas = 0;
    let mut total_inserted = 0;
    let mut total_books = 0;
    let mut total_vaggas = 0;
    let mut errors = 0;

    for (idx, xml_file) in xml_files.iter().enumerate() {
        println!("[{}/{}] Processing: {:?}", idx + 1, xml_files.len(), xml_file.file_name().unwrap_or_default());

        let stats = if let Some(ref mut conn) = conn_opt {
            // Full processing with database insertion
            match importer.process_file(xml_file, conn) {
                Ok(stats) => stats,
                Err(e) => {
                    eprintln!("  ✗ Error processing file: {}", e);
                    errors += 1;
                    continue;
                }
            }
        } else {
            // Dry run mode
            match importer.process_file_dry_run(xml_file) {
                Ok(stats) => stats,
                Err(e) => {
                    eprintln!("  ✗ Error processing file: {}", e);
                    errors += 1;
                    continue;
                }
            }
        };

        // Display results
        println!("  Nikaya: {}", stats.nikaya);
        println!("  Books: {}, Vaggas: {}, Suttas: {}", stats.books, stats.vaggas, stats.suttas_total);

        if !dry_run {
            println!("  Inserted: {}, Failed: {}", stats.suttas_inserted, stats.suttas_failed);
        }

        total_suttas += stats.suttas_total;
        total_inserted += stats.suttas_inserted;
        total_books += stats.books;
        total_vaggas += stats.vaggas;

        println!("  ✓ Processing complete\n");
    }

    // Summary
    println!("\n===================");
    println!("Summary");
    println!("===================");
    println!("Files processed: {}", xml_files.len());
    println!("Total books: {}", total_books);
    println!("Total vaggas: {}", total_vaggas);
    println!("Total suttas: {}", total_suttas);

    if !dry_run {
        println!("Successfully inserted: {}", total_inserted);
        println!("Failed: {}", total_suttas - total_inserted);
        println!("\n✓ Import complete! Database: {:?}", output_db_path);
    } else {
        println!("\nDRY RUN - No database operations performed");
    }

    println!("Errors: {}", errors);

    Ok(())
}

/// Parse Tipitaka XML files with fragment-based parser
fn parse_tipitaka_xml_new(
    input_path: &Path,
    _output_db_path: &Path,
    fragments_db: Option<&Path>,
    verbose: bool,
    dry_run: bool,
) -> Result<(), String> {
    use tipitaka_xml_parser_tsv::encoding::read_xml_file;
    use tipitaka_xml_parser::{detect_nikaya_structure, parse_into_fragments, export_fragments_to_db};
    use std::fs;

    println!("Tipitaka XML Parser (Fragment-Based)");
    println!("====================================\n");

    // Collect XML files to process
    let xml_files: Vec<PathBuf> = if input_path.is_file() {
        println!("Processing single file: {:?}\n", input_path);
        vec![input_path.to_path_buf()]
    } else if input_path.is_dir() {
        println!("Processing folder: {:?}", input_path);
        let files: Vec<PathBuf> = fs::read_dir(input_path)
            .map_err(|e| format!("Failed to read directory: {}", e))?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext == "xml")
                    .unwrap_or(false)
            })
            .collect();

        println!("Found {} XML files\n", files.len());
        files
    } else {
        return Err(format!("Input path does not exist: {:?}", input_path));
    };

    if xml_files.is_empty() {
        return Err("No XML files found to process".to_string());
    }

    if dry_run {
        println!("DRY RUN MODE - No database operations will be performed\n");
    }

    // Process each XML file
    let mut total_fragments = 0;
    let mut total_files_processed = 0;
    let mut errors = 0;

    for (idx, xml_file) in xml_files.iter().enumerate() {
        println!("[{}/{}] Processing: {:?}", idx + 1, xml_files.len(), 
                 xml_file.file_name().unwrap_or_default());

        // Read XML file
        let xml_content = match read_xml_file(xml_file) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("  ✗ Error reading file: {}", e);
                errors += 1;
                continue;
            }
        };

        // Phase 1: Detect nikaya structure
        let mut nikaya_structure = match detect_nikaya_structure(&xml_content) {
            Ok(structure) => structure,
            Err(e) => {
                eprintln!("  ✗ Error detecting nikaya: {}", e);
                errors += 1;
                continue;
            }
        };

        // Set the XML filename
        if let Some(filename) = xml_file.file_name().and_then(|n| n.to_str()) {
            nikaya_structure = nikaya_structure.with_xml_filename(filename.to_string());
        }

        if verbose {
            println!("  Detected nikaya: {} ({} levels)", 
                     nikaya_structure.nikaya, nikaya_structure.levels.len());
        }

        // Phase 2: Parse into fragments
        let fragments = match parse_into_fragments(&xml_content, &nikaya_structure) {
            Ok(frags) => frags,
            Err(e) => {
                eprintln!("  ✗ Error parsing fragments: {}", e);
                errors += 1;
                continue;
            }
        };

        println!("  Parsed {} fragments", fragments.len());
        
        // Count fragment types
        let header_count = fragments.iter()
            .filter(|f| matches!(f.fragment_type, tipitaka_xml_parser::FragmentType::Header))
            .count();
        let sutta_count = fragments.iter()
            .filter(|f| matches!(f.fragment_type, tipitaka_xml_parser::FragmentType::Sutta))
            .count();
        
        println!("    Headers: {}, Suttas: {}", header_count, sutta_count);

        // Export to fragments database if specified
        if let Some(frag_db_path) = fragments_db {
            if !dry_run {
                match export_fragments_to_db(&fragments, &nikaya_structure, frag_db_path) {
                    Ok(count) => {
                        if verbose {
                            println!("  ✓ Exported {} fragments to {:?}", count, frag_db_path);
                        }
                    }
                    Err(e) => {
                        eprintln!("  ✗ Error exporting fragments: {}", e);
                        errors += 1;
                        continue;
                    }
                }
            } else {
                println!("  (Dry run: would export {} fragments to {:?})", 
                         fragments.len(), frag_db_path);
            }
        }

        // TODO: Phase 3: Build suttas and insert into output_db_path
        // This is a stub - database_inserter.rs not yet implemented
        if verbose {
            println!("  (Sutta database insertion not yet implemented)");
        }

        total_fragments += fragments.len();
        total_files_processed += 1;
        println!("  ✓ Processing complete\n");
    }

    // Summary
    println!("\n===================");
    println!("Summary");
    println!("===================");
    println!("Files processed: {}", total_files_processed);
    println!("Total fragments: {}", total_fragments);
    println!("Errors: {}", errors);

    if let Some(frag_db_path) = fragments_db {
        if !dry_run {
            println!("\n✓ Fragments exported to: {:?}", frag_db_path);
        }
    }

    if dry_run {
        println!("\nDRY RUN - No database operations performed");
    }

    Ok(())
}

#[derive(Parser, Debug)]
#[command(author, version, about = "Simsapa CLI", long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// Optional path to the main Simsapa directory.
    /// If not provided, the SIMSAPA_DIR environment variable will be used.
    #[arg(long, global = true, value_name = "DIRECTORY_PATH", env = "SIMSAPA_DIR")]
    simsapa_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Query suttas or dictionary words
    #[command(arg_required_else_help = true)]
    Query {
        /// Type of query to perform
        #[arg(value_enum)]
        query_type: QueryType,

        /// The search query string
        query: String,

        /// Print the titles/keys of the results
        #[arg(long, default_value_t = true)]
        print_titles: bool,

        /// Print the count of the results
        #[arg(long, default_value_t = true)]
        print_count: bool,
    },

    /// Import a StarDict dictionary
    #[command(arg_required_else_help = true)]
    ImportStardictDictionary {
        // FIXME: Make the label optional, infer it from the file names in the stardict folder.
        /// A unique label for the dictionary (e.g., "pts, dpd, etc.")
        #[arg(value_name = "LABEL")]
        dict_label: String,

        /// Path to the StarDict dictionary directory (containing .ifo, .idx, .dict[.dz])
        #[arg(value_name = "DIRECTORY_PATH")]
        path: PathBuf,

        /// Limit imported items
        // #[arg(value_name = "DIRECTORY_PATH")]
        limit: Option<usize>,
    },

    /// Import a newly downloaded or generated DPD SQLite database for use in Simsapa
    /// by migrating the db schema and moving the file to Simsapa's local assets folder.
    /// The input db is modified and migrated before moving.
    #[command(arg_required_else_help = true)]
    ImportMigrateDpd {
        /// Path to the DPD SQLite database to migrate and import
        #[arg(value_name = "DIRECTORY_PATH")]
        dpd_input_path: PathBuf,

        /// Specify the path to move the migrated DPD SQLite database to,
        /// if you don't want it to be moved to Simsapa's local assets folder.
        #[arg(value_name = "DIRECTORY_PATH")]
        dpd_output_path: Option<PathBuf>,
    },

    /// Rebuild the application database from local assets and create asset release archives (new modular implementation).
    Bootstrap {
        /// Write a new .env file even if one already exists
        #[arg(long, default_value_t = false)]
        write_new_dotenv: bool,

        /// Skip DPD database initialization and bootstrap
        #[arg(long, default_value_t = false)]
        skip_dpd: bool,
    },

    /// Rebuild the application database using the legacy bootstrap implementation.
    BootstrapOld {
        /// Write a new .env file even if one already exists
        #[arg(long, default_value_t = false)]
        write_new_dotenv: bool,

        /// Skip DPD database initialization and bootstrap
        #[arg(long, default_value_t = false)]
        skip_dpd: bool,
    },

    /// Export Dhammapada Tipitaka.net suttas from legacy database
    DhammapadaTipitakaNetExport {
        /// Path to the legacy appdata.sqlite3 database
        #[arg(value_name = "LEGACY_DB_PATH")]
        legacy_db_path: PathBuf,

        /// Path to the output SQLite database file
        #[arg(value_name = "OUTPUT_DB_PATH")]
        output_db_path: PathBuf,
    },

    /// Generate statistics for an appdata.sqlite3 database
    #[command(arg_required_else_help = true)]
    AppdataStats {
        /// Path to the appdata.sqlite3 database file
        #[arg(value_name = "DB_PATH")]
        db_path: PathBuf,

        /// Optional folder path to write the stats as Markdown and JSON files
        #[arg(value_name = "OUTPUT_FOLDER")]
        output_folder: Option<PathBuf>,

        /// Write stats to files (uses output_folder if specified, otherwise database folder)
        #[arg(long, default_value_t = false)]
        write_stats: bool,
    },

    /// List available languages in SuttaCentral ArangoDB
    SuttacentralImportLanguagesList,

    /// Parse VRI CST Tipitaka XML files using TSV data and import into SQLite database
    #[command(arg_required_else_help = true)]
    ParseTipitakaXmlUsingTSV {
        /// Path to a single XML file or folder containing XML files
        #[arg(value_name = "INPUT_PATH")]
        input_path: PathBuf,

        /// Path to the output SQLite database file
        #[arg(value_name = "OUTPUT_DB_PATH")]
        output_db_path: PathBuf,

        /// Show verbose output during parsing
        #[arg(long, default_value_t = false)]
        verbose: bool,

        /// Parse without inserting into database (dry run)
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },

    /// Parse VRI CST Tipitaka XML files with fragment-based parser
    #[command(arg_required_else_help = true)]
    ParseTipitakaXml {
        /// Path to a single XML file or folder containing XML files
        #[arg(value_name = "INPUT_PATH")]
        input_path: PathBuf,

        /// Path to the output SQLite database file for sutta import (stub - not yet implemented)
        #[arg(value_name = "OUTPUT_DB_PATH")]
        output_db_path: PathBuf,

        /// Optional path to SQLite database for exporting fragments
        #[arg(long, value_name = "FRAGMENTS_DB_PATH")]
        fragments_db: Option<PathBuf>,

        /// Show verbose output during parsing
        #[arg(long, default_value_t = false)]
        verbose: bool,

        /// Parse without inserting into database (dry run)
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },

    /// Convert Tipitaka XML file to UTF-8 (normalizes line endings to LF)
    #[command(arg_required_else_help = true)]
    TipitakaXmlToUtf8 {
        /// Path to the input XML file
        #[arg(value_name = "INPUT_XML_PATH")]
        input_xml_path: PathBuf,

        /// Path to write the UTF-8 encoded output
        #[arg(value_name = "OUTPUT_PATH")]
        output_path: PathBuf,
    },
}

/// Enum for the different types of queries available.
#[derive(ValueEnum, Clone, Debug, PartialEq, Eq)]
enum QueryType {
    Suttas,
    Words,
}

fn main() {
    // Attempt to load .env file. This might define SIMSAPA_DIR if it's not
    // already in the environment. Clap will pick it up via `env = "SIMSAPA_DIR"`.
    if dotenv().is_err() {
        println!("Info: No .env file found or failed to load.");
    }

    let cli = Cli::parse();

    // Don't initialize app data for bootstrap commands since they need to create directories first
    match &cli.command {
        Commands::Bootstrap { .. } | Commands::BootstrapOld { .. } | Commands::DhammapadaTipitakaNetExport { .. } | Commands::AppdataStats { .. } | Commands::SuttacentralImportLanguagesList | Commands::TipitakaXmlToUtf8 { .. } => {
            // Skip app data initialization for bootstrap, export, stats, and suttacentral commands
        }
        _ => {
            init_app_data();
        }
    }

    // Determine Base Simsapa Directory
    // Precedence:
    // - given with --simsapa-dir
    // - set with env var SIMSAPA_DIR
    // - get_create_simsapa_dir()
    let simsapa_dir = match cli.simsapa_dir {
        Some(path) => path,
        None => {
            let simsapa_dir = get_create_simsapa_dir();
            match simsapa_dir {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Failed to get Simsapa directory: {}", e);
                    eprintln!("Use the --simsapa-dir option or set the SIMSAPA_DIR environment variable.");
                    exit(1);
                }
            }
        }
    };

    if !simsapa_dir.is_dir() {
        eprintln!("Error: Directory does not exist or is not a directory: {:?}", simsapa_dir);
        exit(1);
    }

    // === Execute the requested command ===

    let command_result = match cli.command {
        Commands::Query { query_type, query, print_titles, print_count } => {
            match query_type {
                QueryType::Suttas => {
                    query_suttas(&query, print_titles, print_count)
                }
                QueryType::Words => {
                    query_words(&query, print_titles, print_count)
                }
            }
        }

        Commands::ImportStardictDictionary { dict_label, path, limit } => {
             if !path.exists() {
                  Err(format!("Dictionary source path does not exist: {:?}", path))
             } else if !path.is_dir() {
                 Err(format!("Warning: Provided dictionary source path is a file, not a directory. Unzip the StarDict files to a directory."))
             } else {
                 import_stardict_dictionary(&dict_label, &path, limit)
             }
        }

        Commands::ImportMigrateDpd { dpd_input_path, dpd_output_path } => {
             if !dpd_input_path.exists() {
                 Err(format!("DPD input path does not exist: {:?}", dpd_input_path))
             } else {
                 db::dpd::import_migrate_dpd(&dpd_input_path, dpd_output_path)
             }
        }

        Commands::Bootstrap { write_new_dotenv, skip_dpd } => {
            bootstrap::bootstrap(write_new_dotenv, skip_dpd)
                .map_err(|e| e.to_string())
        }

        Commands::BootstrapOld { write_new_dotenv, skip_dpd } => {
            bootstrap_old::bootstrap(write_new_dotenv, skip_dpd)
                .map_err(|e| e.to_string())
        }

        Commands::DhammapadaTipitakaNetExport { legacy_db_path, output_db_path } => {
            export_dhammapada_tipitaka_net(&legacy_db_path, &output_db_path)
                .map_err(|e| e.to_string())
        }

        Commands::AppdataStats { db_path, output_folder, write_stats } => {
            appdata_stats(&db_path, output_folder.as_deref(), write_stats)
        }

        Commands::SuttacentralImportLanguagesList => {
            suttacentral_import_languages_list()
        }

        Commands::ParseTipitakaXmlUsingTSV { input_path, output_db_path, verbose, dry_run } => {
            parse_tipitaka_xml(&input_path, &output_db_path, verbose, dry_run)
        }

        Commands::ParseTipitakaXml { input_path, output_db_path, fragments_db, verbose, dry_run } => {
            parse_tipitaka_xml_new(&input_path, &output_db_path, fragments_db.as_deref(), verbose, dry_run)
        }

        Commands::TipitakaXmlToUtf8 { input_xml_path, output_path } => {
            use std::fs;
            use tipitaka_xml_parser_tsv::encoding::read_xml_file;

            if !input_xml_path.exists() {
                Err(format!("Input XML file does not exist: {:?}", input_xml_path))
            } else if !input_xml_path.is_file() {
                Err(format!("Input path is not a file: {:?}", input_xml_path))
            } else {
                match read_xml_file(&input_xml_path) {
                    Ok(input_text) => {
                        let output_text = input_text.replace(r#"encoding="UTF-16""#, r#"encoding="UTF-8""#);
                        match fs::write(&output_path, output_text) {
                            Ok(()) => {
                                println!("✓ Wrote UTF-8 file to {:?}", output_path);
                                Ok(())
                            }
                            Err(e) => Err(format!("Failed to write output file {:?}: {}", output_path, e)),
                        }
                    }
                    Err(e) => Err(format!("Failed to read XML file: {}", e)),
                }
            }
        }
    };

    if let Err(e) = command_result {
        eprintln!("Error executing command: {}", e);
        exit(1);
    }
}
