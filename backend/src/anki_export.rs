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

fn render_template(template_str: &str, context: &TemplateContext) -> Result<String> {
    let mut tt = TinyTemplate::new();
    tt.set_default_formatter(&tinytemplate::format_unescaped);
    tt.add_template("tmpl", template_str)?;
    Ok(tt.render("tmpl", context)?)
}

fn build_template_context(
    vocab: &VocabItem,
    dpd_data: &serde_json::Map<String, Value>,
    context_snippet: &str,
) -> TemplateContext {
    let word_stem_value = clean_stem(&vocab.word);

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
        dpd: dpd_data.clone(),
    }
}

pub fn export_anki_csv(
    input: AnkiCsvExportInput,
    app_data: &AppData,
) -> Result<AnkiCsvExportResult> {
    let gloss_data: GlossData = serde_json::from_str(&input.gloss_data_json)?;

    let mut files = Vec::new();

    match input.export_format.as_str() {
        "Simple" => {
            let basic_content = generate_basic_csv(&gloss_data, &input, app_data)?;
            files.push(AnkiCsvFile {
                filename: "gloss_export_anki_basic.csv".to_string(),
                content: basic_content,
            });

            if input.include_cloze {
                let cloze_content = generate_cloze_csv(&gloss_data, &input, app_data)?;
                files.push(AnkiCsvFile {
                    filename: "gloss_export_anki_cloze.csv".to_string(),
                    content: cloze_content,
                });
            }
        }
        "Templated" => {
            let templated_content = generate_templated_csv(&gloss_data, &input, app_data, false)?;
            files.push(AnkiCsvFile {
                filename: "gloss_export_anki_templated.csv".to_string(),
                content: templated_content,
            });

            if input.include_cloze {
                let templated_cloze_content = generate_templated_csv(&gloss_data, &input, app_data, true)?;
                files.push(AnkiCsvFile {
                    filename: "gloss_export_anki_templated_cloze.csv".to_string(),
                    content: templated_cloze_content,
                });
            }
        }
        "DataCsv" => {
            let data_content = generate_data_csv(&gloss_data, &input, app_data)?;
            files.push(AnkiCsvFile {
                filename: "gloss_export_anki_data.csv".to_string(),
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

fn generate_basic_csv(
    gloss_data: &GlossData,
    _input: &AnkiCsvExportInput,
    _app_data: &AppData,
) -> Result<String> {
    let mut csv_lines = Vec::new();

    for paragraph in &gloss_data.paragraphs {
        for vocab in &paragraph.vocabulary {
            let word_stem = clean_stem(&vocab.word);
            let front = format!("<div><p>{}</p></div>", word_stem);
            let back = vocab.summary.clone();
            csv_lines.push(format_csv_row(&front, &back));
        }
    }

    Ok(csv_lines.join("\n"))
}

fn generate_cloze_csv(
    gloss_data: &GlossData,
    _input: &AnkiCsvExportInput,
    _app_data: &AppData,
) -> Result<String> {
    let mut csv_lines = Vec::new();

    for paragraph in &gloss_data.paragraphs {
        for vocab in &paragraph.vocabulary {
            let word_stem = clean_stem(&vocab.word);
            let front = format!("{{{{c1::{}}}}}", word_stem);
            let back = format!("<div>{}</div>", vocab.summary);
            csv_lines.push(format_csv_row(&front, &back));
        }
    }

    Ok(csv_lines.join("\n"))
}

fn generate_templated_csv(
    gloss_data: &GlossData,
    input: &AnkiCsvExportInput,
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

            let context_snippet = if is_cloze {
                "".to_string()
            } else {
                "".to_string()
            };

            let context = build_template_context(vocab, &dpd_data, &context_snippet);

            let front = render_template(&input.templates.front, &context)?;
            let back = render_template(&input.templates.back, &context)?;

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
        "lemma_1",
        "lemma_2",
        "pos",
        "grammar",
        "derived_from",
        "meaning_1",
        "construction",
        "derivative",
        "example_1",
        "synonym",
        "antonym",
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
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string()
            };

            let row = vec![
                word_stem,
                "".to_string(),
                vocab.word.clone(),
                vocab.uid.clone(),
                get_field("lemma_1"),
                get_field("lemma_2"),
                get_field("pos"),
                get_field("grammar"),
                get_field("derived_from"),
                get_field("meaning_1"),
                get_field("construction"),
                get_field("derivative"),
                get_field("example_1"),
                get_field("synonym"),
                get_field("antonym"),
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
    };

    let context = build_template_context(&vocab, &dpd_data, context_snippet);

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
