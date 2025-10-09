use serde::{Serialize, Deserialize};
use serde_json::Value;
use tinytemplate::TinyTemplate;
use anyhow::{Result, anyhow};

use crate::types::{AnkiCsvExportInput, AnkiCsvExportResult, AnkiCsvFile};
use crate::app_data::AppData;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct VocabItem {
    uid: String,
    word: String,
    summary: String,
    context_snippet: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ParagraphData {
    text: String,
    vocabulary: Vec<VocabItem>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct GlossData {
    text: String,
    paragraphs: Vec<ParagraphData>,
}

#[derive(Serialize, Debug)]
struct TemplateContext {
    word_stem: String,
    context_snippet: String,
    original_word: String,
    clean_word: String,
    vocab: VocabContextData,
    dpd: serde_json::Map<String, Value>,
    root: serde_json::Map<String, Value>,
}

#[derive(Serialize, Debug)]
struct VocabContextData {
    uid: String,
    word: String,
    summary: String,
}

pub fn clean_stem(stem: &str) -> String {
    let re = regex::Regex::new(r"\s+\d+(\.\d+)?$").unwrap();
    re.replace(stem, "").to_string().to_lowercase()
}

pub fn escape_csv_field(field: &str) -> String {
    let escaped = field.replace('"', "\"\"");
    if escaped.contains(',') || escaped.contains('\n') || escaped.contains('"') {
        format!("\"{}\"", escaped)
    } else {
        escaped
    }
}

pub fn format_csv_row(front: &str, back: &str) -> String {
    format!("{},{}", escape_csv_field(front), escape_csv_field(back))
}

pub fn convert_context_to_cloze(context_snippet: &str) -> String {
    let re = regex::Regex::new(r"<b>(.*?)</b>").unwrap();
    re.replace_all(context_snippet, "{{c1::$1}}").to_string()
}

fn render_template(template_str: &str, context: &TemplateContext) -> Result<String> {
    let mut tt = TinyTemplate::new();
    tt.set_default_formatter(&tinytemplate::format_unescaped);
    tt.add_template("tmpl", template_str)?;
    Ok(tt.render("tmpl", context)?)
}

fn build_template_context(
    vocab: &VocabItem,
    dpd_data: &serde_json::Map<String, Value>,
    root_data: &serde_json::Map<String, Value>,
    context_snippet: &str,
) -> TemplateContext {
    let word_stem_value = clean_stem(&vocab.word);

    let mut filtered_dpd = dpd_data.clone();
    filtered_dpd.remove("inflections");
    filtered_dpd.remove("inflections_api_ca_eva_iti");
    filtered_dpd.remove("inflections_sinhala");
    filtered_dpd.remove("inflections_devanagari");
    filtered_dpd.remove("inflections_thai");
    filtered_dpd.remove("inflections_html");
    filtered_dpd.remove("freq_data");
    filtered_dpd.remove("freq_html");
    filtered_dpd.remove("ebt_count");

    let mut filtered_root = root_data.clone();
    filtered_root.remove("matrix_test");
    filtered_root.remove("root_info");
    filtered_root.remove("root_matrix");

    TemplateContext {
        word_stem: word_stem_value.clone(),
        context_snippet: context_snippet.to_string(),
        original_word: word_stem_value.clone(),
        clean_word: word_stem_value,
        vocab: VocabContextData {
            uid: vocab.uid.clone(),
            word: vocab.word.clone(),
            summary: vocab.summary.clone(),
        },
        dpd: filtered_dpd,
        root: filtered_root,
    }
}

pub fn export_anki_csv(
    input: AnkiCsvExportInput,
    app_data: &AppData,
) -> Result<AnkiCsvExportResult> {
    let gloss_data: GlossData = serde_json::from_str(&input.gloss_data_json)?;

    let mut files = Vec::new();

    let filename = format!("gloss_export_anki_{}.csv", &input.export_format.to_lowercase());
    let filename_cloze = format!("gloss_export_anki_{}_cloze.csv", &input.export_format.to_lowercase());

    match input.export_format.as_str() {
        "Simple" => {
            let simple_content = generate_simple_csv(&gloss_data, &input, app_data)?;
            files.push(AnkiCsvFile {
                filename,
                content: simple_content,
            });

            if input.include_cloze {
                let cloze_content = generate_simple_cloze_csv(&gloss_data, &input, app_data)?;
                files.push(AnkiCsvFile {
                    filename: filename_cloze,
                    content: cloze_content,
                });
            }
        }
        "Templated" => {
            let templated_content = generate_templated_csv(&gloss_data, &input.templates.front, &input.templates.back, app_data, false)?;
            files.push(AnkiCsvFile {
                filename,
                content: templated_content,
            });

            if input.include_cloze {
                let cloze_front = if input.templates.cloze_front.is_empty() {
                    &input.templates.front
                } else {
                    &input.templates.cloze_front
                };
                let cloze_back = if input.templates.cloze_back.is_empty() {
                    &input.templates.back
                } else {
                    &input.templates.cloze_back
                };
                let templated_cloze_content = generate_templated_csv(&gloss_data, cloze_front, cloze_back, app_data, true)?;
                files.push(AnkiCsvFile {
                    filename: filename_cloze,
                    content: templated_cloze_content,
                });
            }
        }
        "DataCsv" => {
            let data_content = generate_data_csv(&gloss_data, &input, app_data)?;
            files.push(AnkiCsvFile {
                filename,
                content: data_content,
            });
        }
        _ => {
            return Err(anyhow!("Unknown export format: {}", input.export_format));
        }
    }

    Ok(AnkiCsvExportResult {
        success: true,
        files,
        error: None,
    })
}

fn generate_simple_csv(
    gloss_data: &GlossData,
    _input: &AnkiCsvExportInput,
    _app_data: &AppData,
) -> Result<String> {
    let mut csv_lines = Vec::new();

    for paragraph in &gloss_data.paragraphs {
        for vocab in &paragraph.vocabulary {
            let word_stem = clean_stem(&vocab.word);
            let front = format!("<div><p>{}</p><p>{}</p></div>", word_stem, vocab.context_snippet);
            let back = vocab.summary.clone();
            csv_lines.push(format_csv_row(&front, &back));
        }
    }

    Ok(csv_lines.join("\n"))
}

fn generate_simple_cloze_csv(
    gloss_data: &GlossData,
    _input: &AnkiCsvExportInput,
    _app_data: &AppData,
) -> Result<String> {
    let mut csv_lines = Vec::new();

    for paragraph in &gloss_data.paragraphs {
        for vocab in &paragraph.vocabulary {
            let front = convert_context_to_cloze(&vocab.context_snippet);
            let back = vocab.summary.clone();
            csv_lines.push(format_csv_row(&front, &back));
        }
    }

    Ok(csv_lines.join("\n"))
}

fn generate_templated_csv(
    gloss_data: &GlossData,
    front_template: &str,
    back_template: &str,
    app_data: &AppData,
    is_cloze: bool,
) -> Result<String> {
    let mut csv_lines = Vec::new();

    for paragraph in &gloss_data.paragraphs {
        for vocab in &paragraph.vocabulary {
            let dpd_data = match app_data.get_dpd_headword_by_uid(&vocab.uid) {
                Some(json) => serde_json::from_str::<serde_json::Map<String, Value>>(&json).unwrap_or_default(),
                None => serde_json::Map::new(),
            };

            let root_key = dpd_data.get("root_key")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let root_data = if !root_key.is_empty() {
                match app_data.get_dpd_root_by_root_key(root_key) {
                    Some(json) => serde_json::from_str::<serde_json::Map<String, Value>>(&json).unwrap_or_default(),
                    None => serde_json::Map::new(),
                }
            } else {
                serde_json::Map::new()
            };

            let context_snippet = if is_cloze {
                convert_context_to_cloze(&vocab.context_snippet)
            } else {
                vocab.context_snippet.clone()
            };

            let context = build_template_context(vocab, &dpd_data, &root_data, &context_snippet);

            let front = render_template(front_template, &context)?;
            let back = render_template(back_template, &context)?;

            csv_lines.push(format_csv_row(&front, &back));
        }
    }

    Ok(csv_lines.join("\n"))
}

fn generate_data_csv(
    gloss_data: &GlossData,
    _input: &AnkiCsvExportInput,
    app_data: &AppData,
) -> Result<String> {
    let mut csv_lines = Vec::new();

    let header = vec![
        "word_stem",
        "context_snippet",
        "word",
        "uid",
        "id",
        "lemma_1",
        "lemma_2",
        "pos",
        "grammar",
        "derived_from",
        "neg",
        "verb",
        "trans",
        "plus_case",
        "meaning_1",
        "meaning_lit",
        "meaning_2",
        "non_ia",
        "sanskrit",
        "root_key",
        "root_sign",
        "root_base",
        "family_root_fk",
        "family_word_fk",
        "family_compound_fk",
        "family_idioms_fk",
        "family_set_fk",
        "construction",
        "derivative",
        "suffix",
        "phonetic",
        "compound_type",
        "compound_construction",
        "non_root_in_comps",
        "source_1",
        "sutta_1",
        "example_1",
        "source_2",
        "sutta_2",
        "example_2",
        "antonym",
        "synonym",
        "variant",
        "var_phonetic",
        "var_text",
        "commentary",
        "notes",
        "cognate",
        "link",
        "origin",
        "stem",
        "pattern",
        "dictionary_id",
        "word_ascii",
        "lemma_clean",
        "summary",
    ];
    csv_lines.push(header.join(","));

    for paragraph in &gloss_data.paragraphs {
        for vocab in &paragraph.vocabulary {
            let word_stem = clean_stem(&vocab.word);

            let dpd_json = app_data.get_dpd_headword_by_uid(&vocab.uid).unwrap_or_else(|| "{}".to_string());
            let dpd_data: serde_json::Map<String, Value> = serde_json::from_str(&dpd_json).unwrap_or_default();

            let get_field = |key: &str| -> String {
                dpd_data.get(key)
                    .and_then(|v| {
                        if v.is_string() {
                            v.as_str().map(|s| s.to_string())
                        } else if v.is_i64() {
                            Some(v.as_i64().unwrap().to_string())
                        } else if v.is_u64() {
                            Some(v.as_u64().unwrap().to_string())
                        } else if v.is_f64() {
                            Some(v.as_f64().unwrap().to_string())
                        } else if v.is_boolean() {
                            Some(v.as_bool().unwrap().to_string())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| "".to_string())
            };

            let row = vec![
                word_stem,
                vocab.context_snippet.clone(),
                vocab.word.clone(),
                vocab.uid.clone(),
                get_field("id"),
                get_field("lemma_1"),
                get_field("lemma_2"),
                get_field("pos"),
                get_field("grammar"),
                get_field("derived_from"),
                get_field("neg"),
                get_field("verb"),
                get_field("trans"),
                get_field("plus_case"),
                get_field("meaning_1"),
                get_field("meaning_lit"),
                get_field("meaning_2"),
                get_field("non_ia"),
                get_field("sanskrit"),
                get_field("root_key"),
                get_field("root_sign"),
                get_field("root_base"),
                get_field("family_root_fk"),
                get_field("family_word_fk"),
                get_field("family_compound_fk"),
                get_field("family_idioms_fk"),
                get_field("family_set_fk"),
                get_field("construction"),
                get_field("derivative"),
                get_field("suffix"),
                get_field("phonetic"),
                get_field("compound_type"),
                get_field("compound_construction"),
                get_field("non_root_in_comps"),
                get_field("source_1"),
                get_field("sutta_1"),
                get_field("example_1"),
                get_field("source_2"),
                get_field("sutta_2"),
                get_field("example_2"),
                get_field("antonym"),
                get_field("synonym"),
                get_field("variant"),
                get_field("var_phonetic"),
                get_field("var_text"),
                get_field("commentary"),
                get_field("notes"),
                get_field("cognate"),
                get_field("link"),
                get_field("origin"),
                get_field("stem"),
                get_field("pattern"),
                get_field("dictionary_id"),
                get_field("word_ascii"),
                get_field("lemma_clean"),
                vocab.summary.clone(),
            ];

            let escaped_row: Vec<String> = row.iter().map(|f| escape_csv_field(f)).collect();
            csv_lines.push(escaped_row.join(","));
        }
    }

    Ok(csv_lines.join("\n"))
}

pub fn render_anki_preview(
    sample_data_json: &str,
    front_template: &str,
    back_template: &str,
    _app_data: &AppData,
) -> Result<String> {
    let sample_data: serde_json::Map<String, Value> = serde_json::from_str(sample_data_json)?;

    let vocab_obj = sample_data.get("vocab")
        .and_then(|v| v.as_object())
        .ok_or_else(|| anyhow!("Missing vocab field in sample data"))?;

    let dpd_data = sample_data.get("dpd")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();

    let root_data = sample_data.get("root")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();

    let uid = vocab_obj.get("uid")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let word = vocab_obj.get("word")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let summary = vocab_obj.get("summary")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let context_snippet = sample_data.get("context_snippet")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let vocab = VocabItem {
        uid: uid.to_string(),
        word: word.to_string(),
        summary: summary.to_string(),
        context_snippet: context_snippet.to_string(),
    };

    let context = build_template_context(&vocab, &dpd_data, &root_data, context_snippet);

    let front_rendered = render_template(front_template, &context)
        .unwrap_or_else(|e| format!("<span style='color: red;'>Error: {}</span>", e));
    let back_rendered = render_template(back_template, &context)
        .unwrap_or_else(|e| format!("<span style='color: red;'>Error: {}</span>", e));

    let preview_html = format!(
        "<h4>Front:</h4>\
         <div style='background: #fff; padding: 10px; border: 1px solid #ccc; margin-bottom: 10px;'>{}</div>\
         <h4>Back:</h4>\
         <div style='background: #fff; padding: 10px; border: 1px solid #ccc;'>{}</div>",
        front_rendered,
        back_rendered
    );

    Ok(preview_html)
}
