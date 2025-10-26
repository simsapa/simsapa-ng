mod bootstrap;
mod bootstrap_old;

use std::path::{Path, PathBuf};
use std::process::exit;

use clap::{Parser, Subcommand, ValueEnum};
use dotenvy::dotenv;
use anyhow::Result;

use simsapa_backend::{db, init_app_data, get_app_data, get_create_simsapa_dir};
use simsapa_backend::types::{SearchArea, SearchMode, SearchParams, SearchResult};
use simsapa_backend::query_task::SearchQueryTask;
use simsapa_backend::stardict_parse::import_stardict_as_new;
use simsapa_backend::db::appdata_models::Sutta;

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

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
    },

    /// Rebuild the application database using the legacy bootstrap implementation.
    BootstrapOld {
        /// Write a new .env file even if one already exists
        #[arg(long, default_value_t = false)]
        write_new_dotenv: bool,
    },

    /// Export Dhammapada Tipitaka.net suttas from legacy database
    DhammapadaTipitakaNetExport {
        /// Path to the legacy appdata.sqlite3 database
        #[arg(value_name = "LEGACY_DB_PATH")]
        legacy_db_path: PathBuf,

        /// Path to the output SQLite database file
        #[arg(value_name = "OUTPUT_DB_PATH")]
        output_db_path: PathBuf,
    }
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
        Commands::Bootstrap { .. } | Commands::BootstrapOld { .. } | Commands::DhammapadaTipitakaNetExport { .. } => {
            // Skip app data initialization for bootstrap and export commands
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

        Commands::Bootstrap { write_new_dotenv } => {
            bootstrap::bootstrap(write_new_dotenv)
                .map_err(|e| e.to_string())
        }

        Commands::BootstrapOld { write_new_dotenv } => {
            bootstrap_old::bootstrap(write_new_dotenv)
                .map_err(|e| e.to_string())
        }

        Commands::DhammapadaTipitakaNetExport { legacy_db_path, output_db_path } => {
            export_dhammapada_tipitaka_net(&legacy_db_path, &output_db_path)
                .map_err(|e| e.to_string())
        }
    };

    if let Err(e) = command_result {
        eprintln!("Error executing command: {}", e);
        exit(1);
    }
}
