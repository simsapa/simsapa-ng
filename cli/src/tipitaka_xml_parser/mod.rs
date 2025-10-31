//! Tipitaka XML Parser
//!
//! This module provides functionality for parsing Tipitaka XML files into structured
//! fragments that can be assembled into sutta database records.
//!
//! The parser uses a fragment-based architecture to handle different nikaya structures
//! and preserve line tracking information.

pub mod types;
pub mod nikaya_detector;
pub mod nikaya_structure;
pub mod fragment_parser;
pub mod fragment_exporter;
pub mod sutta_builder;
pub mod database_inserter;
pub mod integration;

// Re-export main types for convenience
pub use types::{FragmentType, GroupType, GroupLevel, XmlFragment};
pub use nikaya_structure::NikayaStructure;
pub use nikaya_detector::detect_nikaya_structure;
pub use fragment_parser::parse_into_fragments;
pub use fragment_exporter::export_fragments_to_db;
pub use sutta_builder::build_suttas;
pub use database_inserter::insert_suttas;
pub use integration::{process_xml_file, process_directory, ProcessingStats};
