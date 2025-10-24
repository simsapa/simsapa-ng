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

    /// Rebuild the application database from local assets and create asset release archives.
    Bootstrap {
        /// Write a new .env file even if one already exists
        #[arg(long, default_value_t = false)]
        write_new_dotenv: bool,
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

    // Don't initialize app data for bootstrap command since it needs to create directories first
    match &cli.command {
        Commands::Bootstrap { .. } => {
            // Skip app data initialization for bootstrap
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
            bootstrap_old::bootstrap(write_new_dotenv)
                .map_err(|e| e.to_string())
        }
    };

    if let Err(e) = command_result {
        eprintln!("Error executing command: {}", e);
        exit(1);
    }
}
