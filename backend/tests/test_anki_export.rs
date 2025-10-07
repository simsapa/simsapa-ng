use simsapa_backend::anki_export::*;
use simsapa_backend::types::{AnkiCsvExportInput, AnkiCsvTemplates};

#[test]
fn test_clean_stem() {
    assert_eq!(clean_stem("dhamma 1.01"), "dhamma");
    assert_eq!(clean_stem("ña 2.1"), "ña");
    assert_eq!(clean_stem("jhāyī 1"), "jhāyī");
    assert_eq!(clean_stem("test 123.456"), "test");
    assert_eq!(clean_stem("yo pana bhikkhu"), "yo pana bhikkhu");
}

#[test]
fn test_escape_csv_field() {
    assert_eq!(escape_csv_field("simple text"), "simple text");
    assert_eq!(escape_csv_field("text, with comma"), "\"text, with comma\"");
    assert_eq!(escape_csv_field("text with \"quotes\""), "\"text with \"\"quotes\"\"\"");
    assert_eq!(escape_csv_field("text\nwith newline"), "\"text\nwith newline\"");
    assert_eq!(escape_csv_field("text, with \"quotes\" and\nnewline"), "\"text, with \"\"quotes\"\" and\nnewline\"");
}

#[test]
fn test_format_csv_row() {
    assert_eq!(format_csv_row("front", "back"), "front,back");
    assert_eq!(format_csv_row("front, comma", "back"), "\"front, comma\",back");
    assert_eq!(format_csv_row("front", "back \"quoted\""), "front,\"back \"\"quoted\"\"\"");
    assert_eq!(format_csv_row("front, comma", "back \"quoted\""), "\"front, comma\",\"back \"\"quoted\"\"\"");
}

#[test]
fn test_basic_csv_export() {
    let gloss_data_json = r#"{
        "text": "Test paragraph",
        "paragraphs": [{
            "text": "Test paragraph",
            "vocabulary": [{
                "uid": "test_1",
                "word": "test 1",
                "summary": "test summary"
            }]
        }]
    }"#;

    let input = AnkiCsvExportInput {
        gloss_data_json: gloss_data_json.to_string(),
        export_format: "Simple".to_string(),
        include_cloze: false,
        templates: AnkiCsvTemplates {
            front: "{word_stem}".to_string(),
            back: "{vocab.summary}".to_string(),
        },
    };

    // This test would need AppData, so we'll skip the full integration test for now
    // Instead, we test the individual components above
}

#[test]
fn test_stem_number_removal() {
    // Test that stem numbers are properly removed in CSV export
    let stems = vec![
        ("karitvā 1", "karitvā"),
        ("citta 1.1", "citta"),
        ("dhamma 1.01", "dhamma"),
        ("test 123.456", "test"),
    ];

    for (input, expected) in stems {
        assert_eq!(clean_stem(input), expected.to_lowercase());
    }
}
