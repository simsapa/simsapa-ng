mod helpers;

use serial_test::serial;
use simsapa_backend::anki_export::*;
use simsapa_backend::types::{AnkiCsvExportInput, AnkiCsvTemplates};

fn create_test_gloss_data() -> String {
    r#"{
        "text": "Idha, bhikkhave, ariyasāvako vossaggārammaṇaṁ karitvā labhati samādhiṁ, labhati cittassa ekaggataṁ.",
        "paragraphs": [{
            "text": "Idha, bhikkhave, ariyasāvako vossaggārammaṇaṁ karitvā labhati samādhiṁ, labhati cittassa ekaggataṁ.",
            "vocabulary": [
                {
                    "uid": "20502/dpd",
                    "word": "karitvā 1",
                    "summary": "<i>(abs)</i> having done, having made",
                    "context_snippet": "vossaggārammaṇaṁ <b>karitvā</b> labhati samādhiṁ"
                },
                {
                    "uid": "26555/dpd",
                    "word": "citta 1.1",
                    "summary": "<b>citta 1.1</b> <i>(nt)</i> mind, heart",
                    "context_snippet": "labhati <b>cittassa</b> ekaggataṁ"
                }
            ]
        }]
    }"#.to_string()
}

fn create_multi_paragraph_gloss_data() -> String {
    r#"{
        "text": "Paragraph one karitvā.\n\nParagraph two cittassa.",
        "paragraphs": [
            {
                "text": "Paragraph one karitvā.",
                "vocabulary": [{
                    "uid": "20502/dpd",
                    "word": "karitvā 1",
                    "summary": "<i>(abs)</i> having done, having made",
                    "context_snippet": "Paragraph one <b>karitvā</b>."
                }]
            },
            {
                "text": "Paragraph two cittassa.",
                "vocabulary": [{
                    "uid": "26555/dpd",
                    "word": "citta 1.1",
                    "summary": "<b>citta 1.1</b> <i>(nt)</i> mind, heart",
                    "context_snippet": "Paragraph two <b>cittassa</b>."
                }]
            }
        ]
    }"#.to_string()
}

fn create_escaping_test_data() -> String {
    r#"{
        "text": "Test paragraph",
        "paragraphs": [{
            "text": "Test paragraph",
            "vocabulary": [{
                "uid": "test_1",
                "word": "test 1",
                "summary": "Test \"with quotes\", commas, and\nnewlines",
                "context_snippet": "Test <b>paragraph</b>"
            }]
        }]
    }"#.to_string()
}

#[test]
fn test_clean_stem() {
    assert_eq!(clean_stem("dhamma 1.01"), "dhamma");
    assert_eq!(clean_stem("ña 2.1"), "ña");
    assert_eq!(clean_stem("jhāyī 1"), "jhāyī");
    assert_eq!(clean_stem("test 123.456"), "test");
    assert_eq!(clean_stem("yo pana bhikkhu"), "yo pana bhikkhu");
    assert_eq!(clean_stem("karitvā 1"), "karitvā");
    assert_eq!(clean_stem("citta 1.1"), "citta");
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
fn test_convert_context_to_cloze() {
    assert_eq!(
        convert_context_to_cloze("vossaggārammaṇaṁ <b>karitvā</b> labhati samādhiṁ"),
        "vossaggārammaṇaṁ {{c1::karitvā}} labhati samādhiṁ"
    );
    
    assert_eq!(
        convert_context_to_cloze("labhati <b>cittassa</b> ekaggataṁ"),
        "labhati {{c1::cittassa}} ekaggataṁ"
    );
    
    assert_eq!(
        convert_context_to_cloze("no bold text here"),
        "no bold text here"
    );
}

#[test]
#[serial]
fn test_anki_csv_simple_format() {
    helpers::app_data_setup();
    let app_data = simsapa_backend::get_app_data();

    let input = AnkiCsvExportInput {
        gloss_data_json: create_test_gloss_data(),
        export_format: "Simple".to_string(),
        include_cloze: false,
        templates: AnkiCsvTemplates {
            front: "{word_stem}".to_string(),
            back: "{vocab.summary}".to_string(),
            cloze_front: "".to_string(),
            cloze_back: "".to_string(),
        },
    };

    let result = export_anki_csv(input, &app_data).expect("Export should succeed");
    
    assert!(result.success, "Export should succeed");
    assert_eq!(result.files.len(), 1, "Should have one file");
    
    let csv = &result.files[0].content;
    assert!(!csv.is_empty(), "CSV should not be empty");
    assert!(csv.contains("karitvā"), "CSV should contain karitvā");
    assert!(csv.contains("citta"), "CSV should contain citta");
    assert!(csv.contains("having done, having made"), "CSV should contain definition");
    
    let lines: Vec<&str> = csv.lines().collect();
    assert!(lines.len() >= 2, "Should have at least 2 CSV rows");
    
    let first_line = lines[0];
    assert!(first_line.contains(","), "CSV rows should have comma separator");
}

#[test]
#[serial]
fn test_anki_csv_cloze_format() {
    helpers::app_data_setup();
    let app_data = simsapa_backend::get_app_data();

    let input = AnkiCsvExportInput {
        gloss_data_json: create_test_gloss_data(),
        export_format: "Simple".to_string(),
        include_cloze: true,
        templates: AnkiCsvTemplates {
            front: "{word_stem}".to_string(),
            back: "{vocab.summary}".to_string(),
            cloze_front: "".to_string(),
            cloze_back: "".to_string(),
        },
    };

    let result = export_anki_csv(input, &app_data).expect("Export should succeed");
    
    assert!(result.success, "Export should succeed");
    assert_eq!(result.files.len(), 2, "Should have two files (normal and cloze)");
    
    let cloze_csv = &result.files[1].content;
    assert!(!cloze_csv.is_empty(), "Cloze CSV should not be empty");
    assert!(cloze_csv.contains("{{c1::"), "Cloze format should have {{c1:: marker");
    assert!(cloze_csv.contains("}}"), "Cloze format should have }} marker");
    
    let lines: Vec<&str> = cloze_csv.lines().collect();
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        if line.contains("{{c1::") {
            assert!(line.contains("}}"), "Cloze markers should be paired");
        }
    }
}

#[test]
#[serial]
fn test_anki_csv_templated_format() {
    helpers::app_data_setup();
    let app_data = simsapa_backend::get_app_data();

    let input = AnkiCsvExportInput {
        gloss_data_json: create_test_gloss_data(),
        export_format: "Templated".to_string(),
        include_cloze: false,
        templates: AnkiCsvTemplates {
            front: "Stem: {word_stem}".to_string(),
            back: "Summary: {vocab.summary}".to_string(),
            cloze_front: "".to_string(),
            cloze_back: "".to_string(),
        },
    };

    let result = export_anki_csv(input, &app_data).expect("Export should succeed");
    
    assert!(result.success, "Export should succeed");
    assert_eq!(result.files.len(), 1, "Should have one file");
    
    let csv = &result.files[0].content;
    assert!(!csv.is_empty(), "CSV should not be empty");
}

#[test]
#[serial]
fn test_anki_csv_data_format() {
    helpers::app_data_setup();
    let app_data = simsapa_backend::get_app_data();

    let input = AnkiCsvExportInput {
        gloss_data_json: create_test_gloss_data(),
        export_format: "DataCsv".to_string(),
        include_cloze: false,
        templates: AnkiCsvTemplates {
            front: "{word_stem}".to_string(),
            back: "{vocab.summary}".to_string(),
            cloze_front: "".to_string(),
            cloze_back: "".to_string(),
        },
    };

    let result = export_anki_csv(input, &app_data).expect("Export should succeed");
    
    assert!(result.success, "Export should succeed");
    assert_eq!(result.files.len(), 1, "Should have one file");
    
    let csv = &result.files[0].content;
    assert!(!csv.is_empty(), "CSV should not be empty");
    
    let lines: Vec<&str> = csv.lines().collect();
    assert!(lines.len() >= 2, "Data CSV should have header + data rows");
    
    let header = lines[0];
    assert!(header.contains("word_stem"), "Header should have word_stem");
    assert!(header.contains("context_snippet"), "Header should have context_snippet");
    assert!(header.contains("uid"), "Header should have uid");
    assert!(header.contains("summary"), "Header should have summary");
}

#[test]
#[serial]
fn test_anki_csv_escaping() {
    helpers::app_data_setup();
    let app_data = simsapa_backend::get_app_data();

    let input = AnkiCsvExportInput {
        gloss_data_json: create_escaping_test_data(),
        export_format: "Simple".to_string(),
        include_cloze: false,
        templates: AnkiCsvTemplates {
            front: "{word_stem}".to_string(),
            back: "{vocab.summary}".to_string(),
            cloze_front: "".to_string(),
            cloze_back: "".to_string(),
        },
    };

    let result = export_anki_csv(input, &app_data).expect("Export should succeed");
    
    let csv = &result.files[0].content;
    assert!(csv.contains("\"\""), "Quotes should be escaped as double quotes");
    assert!(csv.contains("\""), "Fields with special chars should be quoted");
}

#[test]
#[serial]
fn test_anki_csv_multiple_paragraphs() {
    helpers::app_data_setup();
    let app_data = simsapa_backend::get_app_data();

    let input = AnkiCsvExportInput {
        gloss_data_json: create_multi_paragraph_gloss_data(),
        export_format: "Simple".to_string(),
        include_cloze: false,
        templates: AnkiCsvTemplates {
            front: "{word_stem}".to_string(),
            back: "{vocab.summary}".to_string(),
            cloze_front: "".to_string(),
            cloze_back: "".to_string(),
        },
    };

    let result = export_anki_csv(input, &app_data).expect("Export should succeed");
    
    let csv = &result.files[0].content;
    assert!(csv.contains("karitvā"), "Should include vocab from paragraph 1");
    assert!(csv.contains("citta"), "Should include vocab from paragraph 2");
    
    let lines: Vec<&str> = csv.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(lines.len() >= 2, "Should have rows from both paragraphs");
}

#[test]
#[serial]
fn test_anki_csv_clean_stem_in_export() {
    helpers::app_data_setup();
    let app_data = simsapa_backend::get_app_data();

    let input = AnkiCsvExportInput {
        gloss_data_json: create_test_gloss_data(),
        export_format: "Simple".to_string(),
        include_cloze: false,
        templates: AnkiCsvTemplates {
            front: "{word_stem}".to_string(),
            back: "{vocab.summary}".to_string(),
            cloze_front: "".to_string(),
            cloze_back: "".to_string(),
        },
    };

    let result = export_anki_csv(input, &app_data).expect("Export should succeed");
    
    let csv = &result.files[0].content;
    let lines: Vec<&str> = csv.lines().collect();
    
    let stem_number_regex = regex::Regex::new(r"\s+\d+(\.\d+)?$").unwrap();
    
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        if !fields.is_empty() && !fields[0].trim().is_empty() {
            let first_field = fields[0].trim_matches('"');
            assert!(!stem_number_regex.is_match(first_field), 
                    "Stem numbers should be removed from word_stem: {}", first_field);
        }
    }
}

#[test]
#[serial]
fn test_context_snippet_in_templated_export() {
    helpers::app_data_setup();
    let app_data = simsapa_backend::get_app_data();

    let input = AnkiCsvExportInput {
        gloss_data_json: create_test_gloss_data(),
        export_format: "Templated".to_string(),
        include_cloze: false,
        templates: AnkiCsvTemplates {
            front: "{word_stem}".to_string(),
            back: "{context_snippet}".to_string(),
            cloze_front: "".to_string(),
            cloze_back: "".to_string(),
        },
    };

    let result = export_anki_csv(input, &app_data).expect("Export should succeed");
    
    let csv = &result.files[0].content;
    assert!(csv.contains("<b>karitvā</b>"), "Context snippet should contain bolded word");
    assert!(csv.contains("<b>cittassa</b>"), "Context snippet should contain bolded word");
    assert!(csv.contains("vossaggārammaṇaṁ"), "Context snippet should contain surrounding context");
}

#[test]
#[serial]
fn test_context_snippet_in_data_csv() {
    helpers::app_data_setup();
    let app_data = simsapa_backend::get_app_data();

    let input = AnkiCsvExportInput {
        gloss_data_json: create_test_gloss_data(),
        export_format: "DataCsv".to_string(),
        include_cloze: false,
        templates: AnkiCsvTemplates {
            front: "{word_stem}".to_string(),
            back: "{vocab.summary}".to_string(),
            cloze_front: "".to_string(),
            cloze_back: "".to_string(),
        },
    };

    let result = export_anki_csv(input, &app_data).expect("Export should succeed");
    
    let csv = &result.files[0].content;
    let lines: Vec<&str> = csv.lines().collect();
    
    assert!(lines.len() >= 2, "Should have header + data rows");
    
    let header = lines[0];
    assert!(header.contains("context_snippet"), "Header should have context_snippet column");
    
    let data_line = lines[1];
    assert!(data_line.contains("<b>karitvā</b>"), "Data should contain context snippet with bolded word");
}

#[test]
#[serial]
fn test_templated_cloze_format() {
    helpers::app_data_setup();
    let app_data = simsapa_backend::get_app_data();

    let input = AnkiCsvExportInput {
        gloss_data_json: create_test_gloss_data(),
        export_format: "Templated".to_string(),
        include_cloze: true,
        templates: AnkiCsvTemplates {
            front: "{context_snippet}".to_string(),
            back: "{vocab.summary}".to_string(),
            cloze_front: "{context_snippet}".to_string(),
            cloze_back: "{vocab.summary}".to_string(),
        },
    };

    let result = export_anki_csv(input, &app_data).expect("Export should succeed");
    
    assert_eq!(result.files.len(), 2, "Should have normal and cloze files");
    
    let normal_csv = &result.files[0].content;
    assert!(normal_csv.contains("<b>karitvā</b>"), "Normal file should have bold tags");
    assert!(normal_csv.contains("<b>cittassa</b>"), "Normal file should have bold tags");
    
    let cloze_csv = &result.files[1].content;
    assert!(cloze_csv.contains("{{c1::karitvā}}"), "Cloze file should have cloze deletions");
    assert!(cloze_csv.contains("{{c1::cittassa}}"), "Cloze file should have cloze deletions");
    assert!(!cloze_csv.contains("<b>karitvā</b>"), "Cloze file should not have bold tags");
    assert!(!cloze_csv.contains("<b>cittassa</b>"), "Cloze file should not have bold tags");
    
    let lines: Vec<&str> = cloze_csv.lines().collect();
    for line in &lines {
        if line.contains("vossaggārammaṇaṁ") {
            assert!(line.contains("{{c1::karitvā}}"), "Context should have cloze deletion for karitvā");
            assert!(line.contains("vossaggārammaṇaṁ"), "Context should have surrounding words");
        }
    }
}

#[test]
#[serial]
fn test_custom_cloze_templates() {
    helpers::app_data_setup();
    let app_data = simsapa_backend::get_app_data();

    let input = AnkiCsvExportInput {
        gloss_data_json: create_test_gloss_data(),
        export_format: "Templated".to_string(),
        include_cloze: true,
        templates: AnkiCsvTemplates {
            front: "{word_stem}".to_string(),
            back: "{vocab.summary}".to_string(),
            cloze_front: "<div class='cloze-front'>{context_snippet}</div>".to_string(),
            cloze_back: "<div class='cloze-back'><b>{dpd.pos}</b> {vocab.summary}</div>".to_string(),
        },
    };

    let result = export_anki_csv(input, &app_data).expect("Export should succeed");
    
    assert_eq!(result.files.len(), 2, "Should have normal and cloze files");
    
    let normal_csv = &result.files[0].content;
    assert!(normal_csv.contains("karitvā"), "Normal file should contain word stem");
    assert!(!normal_csv.contains("<div class='cloze-front'>"), "Normal file should not use cloze template");
    
    let cloze_csv = &result.files[1].content;
    assert!(cloze_csv.contains("<div class='cloze-front'>"), "Cloze file should use custom cloze front template");
    assert!(cloze_csv.contains("<div class='cloze-back'>"), "Cloze file should use custom cloze back template");
    assert!(cloze_csv.contains("{{c1::karitvā}}"), "Cloze file should have cloze deletions");
    assert!(cloze_csv.contains("{{c1::cittassa}}"), "Cloze file should have cloze deletions");
    
    // Verify DPD data is accessible and rendered
    assert!(cloze_csv.contains("<b>abs</b>"), "Cloze back should contain POS from DPD (abs for karitvā)");
    assert!(cloze_csv.contains("<b>nt</b>"), "Cloze back should contain POS from DPD (nt for citta)");
}

#[test]
#[serial]
fn test_cloze_template_fallback() {
    helpers::app_data_setup();
    let app_data = simsapa_backend::get_app_data();

    // Test with empty cloze templates - should fall back to normal templates
    let input = AnkiCsvExportInput {
        gloss_data_json: create_test_gloss_data(),
        export_format: "Templated".to_string(),
        include_cloze: true,
        templates: AnkiCsvTemplates {
            front: "<p>{context_snippet}</p>".to_string(),
            back: "<p>{vocab.summary}</p>".to_string(),
            cloze_front: "".to_string(),
            cloze_back: "".to_string(),
        },
    };

    let result = export_anki_csv(input, &app_data).expect("Export should succeed");
    
    assert_eq!(result.files.len(), 2, "Should have normal and cloze files");
    
    let normal_csv = &result.files[0].content;
    let cloze_csv = &result.files[1].content;
    
    // Both should use the same template structure
    assert!(normal_csv.contains("<p>"), "Normal should use front template");
    assert!(normal_csv.contains("<b>"), "Normal should have bold tags in context");
    
    assert!(cloze_csv.contains("<p>"), "Cloze should fall back to front template");
    assert!(cloze_csv.contains("{{c1::"), "Cloze should have cloze deletions instead of bold tags");
    assert!(!cloze_csv.contains("<b>karitvā</b>"), "Cloze should not have bold tags");
}

#[test]
#[serial]
fn test_cloze_template_with_context_snippet() {
    helpers::app_data_setup();
    let app_data = simsapa_backend::get_app_data();

    let input = AnkiCsvExportInput {
        gloss_data_json: create_test_gloss_data(),
        export_format: "Templated".to_string(),
        include_cloze: true,
        templates: AnkiCsvTemplates {
            front: "{word_stem}".to_string(),
            back: "{vocab.summary}".to_string(),
            cloze_front: "{context_snippet}".to_string(),
            cloze_back: "{vocab.summary}".to_string(),
        },
    };

    let result = export_anki_csv(input, &app_data).expect("Export should succeed");
    
    let cloze_csv = &result.files[1].content;
    
    // Verify that context snippet in cloze front has cloze deletions
    assert!(cloze_csv.contains("{{c1::karitvā}}"), "Context should have cloze deletion");
    assert!(cloze_csv.contains("vossaggārammaṇaṁ"), "Context should have surrounding text");
    assert!(!cloze_csv.contains("<b>karitvā</b>"), "Context should not have bold tags");
    
    // Verify the back still has the summary
    assert!(cloze_csv.contains("having done, having made"), "Back should have summary");
}

#[test]
#[serial]
fn test_cloze_template_variables() {
    helpers::app_data_setup();
    let app_data = simsapa_backend::get_app_data();

    let input = AnkiCsvExportInput {
        gloss_data_json: create_test_gloss_data(),
        export_format: "Templated".to_string(),
        include_cloze: true,
        templates: AnkiCsvTemplates {
            front: "{word_stem}".to_string(),
            back: "{vocab.summary}".to_string(),
            cloze_front: "Word: {word_stem}<br>Context: {context_snippet}".to_string(),
            cloze_back: "Stem: {word_stem}<br>Meaning: {vocab.summary}".to_string(),
        },
    };

    let result = export_anki_csv(input, &app_data).expect("Export should succeed");
    
    let cloze_csv = &result.files[1].content;
    
    // Verify all template variables are rendered
    assert!(cloze_csv.contains("Word:"), "Cloze front should have 'Word:' prefix");
    assert!(cloze_csv.contains("Context:"), "Cloze front should have 'Context:' prefix");
    assert!(cloze_csv.contains("Stem:"), "Cloze back should have 'Stem:' prefix");
    assert!(cloze_csv.contains("Meaning:"), "Cloze back should have 'Meaning:' prefix");
    assert!(cloze_csv.contains("<br>"), "Templates should render HTML");
    
    // Verify word stem appears in both front and back
    assert!(cloze_csv.contains("karitvā"), "Should contain word stem");
    
    // Verify context has cloze deletion
    assert!(cloze_csv.contains("{{c1::"), "Context should have cloze deletions");
}

#[test]
fn test_stem_number_removal() {
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
