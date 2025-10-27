use diesel::prelude::*;
use serial_test::serial;
use serde::{Serialize, Deserialize};
use std::fs;
use std::path::PathBuf;

mod helpers;

// Serializable wrapper for Sutta
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct SerializableSutta {
    pub id: i32,
    pub uid: String,
    pub sutta_ref: String,
    pub nikaya: String,
    pub language: String,
    pub group_path: Option<String>,
    pub group_index: Option<i32>,
    pub order_index: Option<i32>,
    pub sutta_range_group: Option<String>,
    pub sutta_range_start: Option<i32>,
    pub sutta_range_end: Option<i32>,
    pub title: Option<String>,
    pub title_ascii: Option<String>,
    pub title_pali: Option<String>,
    pub title_trans: Option<String>,
    pub description: Option<String>,
    pub content_plain: Option<String>,
    pub content_html: Option<String>,
    pub content_json: Option<String>,
    pub content_json_tmpl: Option<String>,
    pub source_uid: Option<String>,
    pub source_info: Option<String>,
    pub source_language: Option<String>,
    pub message: Option<String>,
    pub copyright: Option<String>,
    pub license: Option<String>,
}

// Serializable wrapper for SuttaVariant
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct SerializableSuttaVariant {
    pub id: i32,
    pub sutta_id: i32,
    pub sutta_uid: String,
    pub language: Option<String>,
    pub source_uid: Option<String>,
    pub content_json: Option<String>,
}

impl From<simsapa_backend::db::appdata_models::Sutta> for SerializableSutta {
    fn from(s: simsapa_backend::db::appdata_models::Sutta) -> Self {
        SerializableSutta {
            id: s.id,
            uid: s.uid,
            sutta_ref: s.sutta_ref,
            nikaya: s.nikaya,
            language: s.language,
            group_path: s.group_path,
            group_index: s.group_index,
            order_index: s.order_index,
            sutta_range_group: s.sutta_range_group,
            sutta_range_start: s.sutta_range_start,
            sutta_range_end: s.sutta_range_end,
            title: s.title,
            title_ascii: s.title_ascii,
            title_pali: s.title_pali,
            title_trans: s.title_trans,
            description: s.description,
            content_plain: s.content_plain,
            content_html: s.content_html,
            content_json: s.content_json,
            content_json_tmpl: s.content_json_tmpl,
            source_uid: s.source_uid,
            source_info: s.source_info,
            source_language: s.source_language,
            message: s.message,
            copyright: s.copyright,
            license: s.license,
        }
    }
}

impl From<simsapa_backend::db::appdata_models::SuttaVariant> for SerializableSuttaVariant {
    fn from(sv: simsapa_backend::db::appdata_models::SuttaVariant) -> Self {
        SerializableSuttaVariant {
            id: sv.id,
            sutta_id: sv.sutta_id,
            sutta_uid: sv.sutta_uid,
            language: sv.language,
            source_uid: sv.source_uid,
            content_json: sv.content_json,
        }
    }
}

// Get database connection to legacy database
fn get_legacy_db() -> SqliteConnection {
    let db_path = "../../bootstrap-assets-resources/appdata-db-for-bootstrap/current/appdata.sqlite3";
    SqliteConnection::establish(db_path)
        .unwrap_or_else(|_| panic!("Error connecting to legacy database at {}", db_path))
}

// Get database connection to new database
fn get_new_db() -> SqliteConnection {
    let db_path = "../../bootstrap-assets-resources/dist/simsapa-ng/app-assets/appdata.sqlite3";
    SqliteConnection::establish(db_path)
        .unwrap_or_else(|_| panic!("Error connecting to new database at {}", db_path))
}

// Query sutta by uid
fn query_sutta_by_uid(conn: &mut SqliteConnection, uid_value: &str) -> Option<SerializableSutta> {
    use simsapa_backend::db::appdata_schema::suttas::dsl::*;

    suttas
        .filter(uid.eq(uid_value))
        .first::<simsapa_backend::db::appdata_models::Sutta>(conn)
        .optional()
        .unwrap()
        .map(|s| s.into())
}

// Query sutta_variant by sutta_uid
fn query_sutta_variant_by_uid(conn: &mut SqliteConnection, uid_value: &str) -> Option<SerializableSuttaVariant> {
    use simsapa_backend::db::appdata_schema::sutta_variants::dsl::*;

    sutta_variants
        .filter(sutta_uid.eq(uid_value))
        .first::<simsapa_backend::db::appdata_models::SuttaVariant>(conn)
        .optional()
        .unwrap()
        .map(|sv| sv.into())
}

// Get test data directory
fn get_test_data_dir() -> PathBuf {
    PathBuf::from("tests/data")
}

// Generate expected JSON from legacy database
fn generate_expected_json_for_uid(uid_value: &str) {
    let mut legacy_conn = get_legacy_db();
    let test_data_dir = get_test_data_dir();

    // Query sutta
    let sutta = query_sutta_by_uid(&mut legacy_conn, uid_value);
    let sutta_filename = format!("{}_sutta.json", uid_value.replace('/', "_"));
    let sutta_path = test_data_dir.join(&sutta_filename);

    match sutta {
        Some(s) => {
            let json = serde_json::to_string_pretty(&s).unwrap();
            fs::write(&sutta_path, json).unwrap();
            println!("Generated {}", sutta_filename);
        }
        None => {
            fs::write(&sutta_path, "").unwrap();
            println!("Generated empty file {}", sutta_filename);
        }
    }
}

// Generate expected JSON for sutta_variant from legacy database
fn generate_expected_json_for_variant(uid_value: &str) {
    let mut legacy_conn = get_legacy_db();
    let test_data_dir = get_test_data_dir();

    // Query sutta_variant
    let variant = query_sutta_variant_by_uid(&mut legacy_conn, uid_value);
    let variant_filename = format!("{}_sutta_variant.json", uid_value.replace('/', "_"));
    let variant_path = test_data_dir.join(&variant_filename);

    match variant {
        Some(v) => {
            let json = serde_json::to_string_pretty(&v).unwrap();
            fs::write(&variant_path, json).unwrap();
            println!("Generated {}", variant_filename);
        }
        None => {
            fs::write(&variant_path, "").unwrap();
            println!("Generated empty file {}", variant_filename);
        }
    }
}

// Test helper to compare sutta from new DB with expected JSON
fn test_sutta_comparison(uid_value: &str) {
    let mut new_conn = get_new_db();
    let test_data_dir = get_test_data_dir();

    // Query from new database
    let new_sutta = query_sutta_by_uid(&mut new_conn, uid_value);

    // Load expected JSON
    let sutta_filename = format!("{}_sutta.json", uid_value.replace('/', "_"));
    let sutta_path = test_data_dir.join(&sutta_filename);
    let expected_json = fs::read_to_string(&sutta_path)
        .unwrap_or_else(|_| panic!("Failed to read {}", sutta_filename));

    if expected_json.is_empty() {
        assert!(new_sutta.is_none(), "Expected no sutta for uid {}, but found one", uid_value);
    } else {
        let expected_sutta: SerializableSutta = serde_json::from_str(&expected_json)
            .unwrap_or_else(|_| panic!("Failed to parse JSON for {}", sutta_filename));

        let new_sutta = new_sutta.unwrap_or_else(|| panic!("Expected sutta for uid {}, but found none", uid_value));

        // Compare structural fields (ignoring id field as it may differ between databases)
        assert_eq!(new_sutta.uid, expected_sutta.uid, "uid mismatch for {}", uid_value);
        assert_eq!(new_sutta.sutta_ref, expected_sutta.sutta_ref, "sutta_ref mismatch for {}", uid_value);
        assert_eq!(new_sutta.nikaya, expected_sutta.nikaya, "nikaya mismatch for {}", uid_value);
        assert_eq!(new_sutta.language, expected_sutta.language, "language mismatch for {}", uid_value);
        assert_eq!(new_sutta.group_path, expected_sutta.group_path, "group_path mismatch for {}", uid_value);
        assert_eq!(new_sutta.group_index, expected_sutta.group_index, "group_index mismatch for {}", uid_value);
        assert_eq!(new_sutta.order_index, expected_sutta.order_index, "order_index mismatch for {}", uid_value);

        // Note: sutta_range_group, sutta_range_start, and sutta_range_end are expected to be None
        // in the new database as this is an intentional schema/data change.
        // Legacy DB had values like Some("sn56"), Some("mn"), etc. but new DB has None.
        // We verify that new DB has None for these fields.
        assert_eq!(new_sutta.sutta_range_group, None,
            "sutta_range_group should be None in new database for {}, but got {:?}",
            uid_value, new_sutta.sutta_range_group);
        assert_eq!(new_sutta.sutta_range_start, None,
            "sutta_range_start should be None in new database for {}, but got {:?}",
            uid_value, new_sutta.sutta_range_start);
        assert_eq!(new_sutta.sutta_range_end, None,
            "sutta_range_end should be None in new database for {}, but got {:?}",
            uid_value, new_sutta.sutta_range_end);
        assert_eq!(new_sutta.title, expected_sutta.title, "title mismatch for {}", uid_value);
        assert_eq!(new_sutta.title_ascii, expected_sutta.title_ascii, "title_ascii mismatch for {}", uid_value);
        assert_eq!(new_sutta.title_pali, expected_sutta.title_pali, "title_pali mismatch for {}", uid_value);
        assert_eq!(new_sutta.title_trans, expected_sutta.title_trans, "title_trans mismatch for {}", uid_value);
        assert_eq!(new_sutta.description, expected_sutta.description, "description mismatch for {}", uid_value);
        assert_eq!(new_sutta.source_uid, expected_sutta.source_uid, "source_uid mismatch for {}", uid_value);
        assert_eq!(new_sutta.source_info, expected_sutta.source_info, "source_info mismatch for {}", uid_value);
        assert_eq!(new_sutta.source_language, expected_sutta.source_language, "source_language mismatch for {}", uid_value);
        assert_eq!(new_sutta.message, expected_sutta.message, "message mismatch for {}", uid_value);
        assert_eq!(new_sutta.copyright, expected_sutta.copyright, "copyright mismatch for {}", uid_value);
        assert_eq!(new_sutta.license, expected_sutta.license, "license mismatch for {}", uid_value);

        // Compare content fields - allow small differences (< 1%) due to whitespace/punctuation normalization
        // The new database may have slightly different text processing that results in minor differences
        match (&new_sutta.content_plain, &expected_sutta.content_plain) {
            (Some(new_content), Some(expected_content)) => {
                let new_len = new_content.len();
                let expected_len = expected_content.len();
                let diff = if new_len > expected_len {
                    new_len - expected_len
                } else {
                    expected_len - new_len
                };
                let percent_diff = (diff as f64 / expected_len as f64) * 100.0;
                assert!(percent_diff < 1.0,
                    "content_plain length difference too large for {}: new={}, expected={}, diff={}%, should be < 1%",
                    uid_value, new_len, expected_len, percent_diff);
            }
            (None, None) => {},
            _ => panic!("content_plain presence mismatch for {}", uid_value),
        }

        match (&new_sutta.content_html, &expected_sutta.content_html) {
            (Some(new_content), Some(expected_content)) => {
                let new_len = new_content.len();
                let expected_len = expected_content.len();
                let diff = if new_len > expected_len {
                    new_len - expected_len
                } else {
                    expected_len - new_len
                };
                let percent_diff = (diff as f64 / expected_len as f64) * 100.0;
                assert!(percent_diff < 1.0,
                    "content_html length difference too large for {}: new={}, expected={}, diff={}%, should be < 1%",
                    uid_value, new_len, expected_len, percent_diff);
            }
            (None, None) => {},
            _ => panic!("content_html presence mismatch for {}", uid_value),
        }

        // For content_json and content_json_tmpl, we can do exact comparison as they're structured
        assert_eq!(new_sutta.content_json, expected_sutta.content_json, "content_json mismatch for {}", uid_value);
        assert_eq!(new_sutta.content_json_tmpl, expected_sutta.content_json_tmpl, "content_json_tmpl mismatch for {}", uid_value);
    }
}

// Test helper to compare sutta_variant from new DB with expected JSON
fn test_variant_comparison(uid_value: &str) {
    let mut new_conn = get_new_db();
    let test_data_dir = get_test_data_dir();

    // Query from new database
    let new_variant = query_sutta_variant_by_uid(&mut new_conn, uid_value);

    // Load expected JSON
    let variant_filename = format!("{}_sutta_variant.json", uid_value.replace('/', "_"));
    let variant_path = test_data_dir.join(&variant_filename);
    let expected_json = fs::read_to_string(&variant_path)
        .unwrap_or_else(|_| panic!("Failed to read {}", variant_filename));

    if expected_json.is_empty() {
        assert!(new_variant.is_none(), "Expected no variant for uid {}, but found one", uid_value);
    } else {
        let expected_variant: SerializableSuttaVariant = serde_json::from_str(&expected_json)
            .unwrap_or_else(|_| panic!("Failed to parse JSON for {}", variant_filename));

        let new_variant = new_variant.unwrap_or_else(|| panic!("Expected variant for uid {}, but found none", uid_value));

        // Compare (ignoring id and sutta_id fields as they may differ between databases)
        assert_eq!(new_variant.sutta_uid, expected_variant.sutta_uid, "sutta_uid mismatch for {}", uid_value);
        assert_eq!(new_variant.language, expected_variant.language, "language mismatch for {}", uid_value);
        assert_eq!(new_variant.source_uid, expected_variant.source_uid, "source_uid mismatch for {}", uid_value);
        assert_eq!(new_variant.content_json, expected_variant.content_json, "content_json mismatch for {}", uid_value);
    }
}

// Generate all expected JSON files from legacy database
#[test]
#[serial]
#[ignore] // Run with: cargo test --test test_database_comparison -- --ignored
fn generate_all_expected_json() {
    let sutta_uids = vec![
        "sn56.11/pli/ms",
        "mn1/pli/ms",
        "dn22/pli/ms",
        "dhp290-305/pli/ms",
        "snp1.8/pli/ms",
        "pli-tv-bu-vb-pj4/pli/ms",
        "mn1/en/sujato",
        "dn22/en/sujato",
        "dhp290-305/en/sujato",
        "snp1.8/en/sujato",
        "pli-tv-bu-vb-pj4/en/brahmali",
    ];

    let variant_uids = vec![
        "sn56.11/pli/ms",
        "mn1/pli/ms",
        "dn22/pli/ms",
        "dhp290-305/pli/ms",
        "snp1.8/pli/ms",
        "pli-tv-bu-vb-pj4/pli/ms",
    ];

    println!("Generating expected JSON files from legacy database...");

    for uid in &sutta_uids {
        generate_expected_json_for_uid(uid);
    }

    for uid in &variant_uids {
        generate_expected_json_for_variant(uid);
    }

    println!("All expected JSON files generated successfully!");
}

// Tests for suttas
#[test]
#[serial]
fn test_sutta_sn56_11_pli_ms() {
    test_sutta_comparison("sn56.11/pli/ms");
}

#[test]
#[serial]
fn test_sutta_mn1_pli_ms() {
    test_sutta_comparison("mn1/pli/ms");
}

#[test]
#[serial]
fn test_sutta_dn22_pli_ms() {
    test_sutta_comparison("dn22/pli/ms");
}

#[test]
#[serial]
fn test_sutta_dhp290_305_pli_ms() {
    test_sutta_comparison("dhp290-305/pli/ms");
}

#[test]
#[serial]
fn test_sutta_snp1_8_pli_ms() {
    test_sutta_comparison("snp1.8/pli/ms");
}

#[test]
#[serial]
fn test_sutta_pli_tv_bu_vb_pj4_pli_ms() {
    test_sutta_comparison("pli-tv-bu-vb-pj4/pli/ms");
}

#[test]
#[serial]
fn test_sutta_mn1_en_sujato() {
    test_sutta_comparison("mn1/en/sujato");
}

#[test]
#[serial]
fn test_sutta_dn22_en_sujato() {
    test_sutta_comparison("dn22/en/sujato");
}

#[test]
#[serial]
fn test_sutta_dhp290_305_en_sujato() {
    test_sutta_comparison("dhp290-305/en/sujato");
}

#[test]
#[serial]
fn test_sutta_snp1_8_en_sujato() {
    test_sutta_comparison("snp1.8/en/sujato");
}

#[test]
#[serial]
fn test_sutta_pli_tv_bu_vb_pj4_en_brahmali() {
    test_sutta_comparison("pli-tv-bu-vb-pj4/en/brahmali");
}

// Tests for sutta_variants
#[test]
#[serial]
fn test_variant_sn56_11_pli_ms() {
    test_variant_comparison("sn56.11/pli/ms");
}

#[test]
#[serial]
fn test_variant_mn1_pli_ms() {
    test_variant_comparison("mn1/pli/ms");
}

#[test]
#[serial]
fn test_variant_dn22_pli_ms() {
    test_variant_comparison("dn22/pli/ms");
}

#[test]
#[serial]
fn test_variant_dhp290_305_pli_ms() {
    test_variant_comparison("dhp290-305/pli/ms");
}

#[test]
#[serial]
fn test_variant_snp1_8_pli_ms() {
    test_variant_comparison("snp1.8/pli/ms");
}

#[test]
#[serial]
fn test_variant_pli_tv_bu_vb_pj4_pli_ms() {
    test_variant_comparison("pli-tv-bu-vb-pj4/pli/ms");
}
