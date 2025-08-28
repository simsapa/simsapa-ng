use std::collections::BTreeMap;

use serde::{Serialize, Deserialize};

use crate::logger::error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelEntry {
    pub model_name: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub sutta_font_size: usize,
    pub sutta_max_width: usize,
    pub show_bookmarks: bool,
    pub show_translation_and_pali_line_by_line: bool,
    pub show_all_variant_readings: bool,
    pub show_glosses: bool,
    pub theme_name: ThemeName,
    pub api_keys: BTreeMap<String, String>,
    pub system_prompts: BTreeMap<String, String>,
    pub models: Vec<ModelEntry>,
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            sutta_font_size: 22,
            sutta_max_width: 75,
            show_bookmarks: true,
            show_translation_and_pali_line_by_line: true,
            show_all_variant_readings: false,
            show_glosses: false,
            theme_name: ThemeName::System,
            api_keys: BTreeMap::new(),
            system_prompts: {
                let mut prompts = BTreeMap::new();
                prompts.insert("Gloss Tab: AI-Translation".to_string(),
                    r#"
Translate the following Pāli passage to English, keeping in mind the provided dictionary definitions.

Pāli passage:

<<PALI_PASSAGE>>

Dictionary definitions:

<<DICTIONARY_DEFINITIONS>>

Respond with only the translation of the Pāli passage.
Respond with GFM-Markdown formatted text.
"#.trim().to_string());

                prompts.insert("Prompts Tab: System Prompt".to_string(),
                    r#"
You are a helpful assistant for studying the suttas of the Theravāda Pāli Tipitaka and the Pāli language.
Respond with concise answers and respond only with the information requested in the task.
Respond with GFM-Markdown formatted text.
"#.trim().to_string());

                prompts
            },
            models: vec![
                ModelEntry { model_name: "tngtech/deepseek-r1t2-chimera:free".to_string(), enabled: true },
                ModelEntry { model_name: "deepseek/deepseek-r1-0528:free".to_string(), enabled: false },
                ModelEntry { model_name: "deepseek/deepseek-chat-v3-0324:free".to_string(), enabled: false },
                ModelEntry { model_name: "google/gemini-2.0-flash-exp:free".to_string(), enabled: false },
                ModelEntry { model_name: "google/gemma-3-27b-it:free".to_string(), enabled: true },
                ModelEntry { model_name: "openai/gpt-oss-20b:free".to_string(), enabled: false },
                ModelEntry { model_name: "meta-llama/llama-3.3-70b-instruct:free".to_string(), enabled: false },
                ModelEntry { model_name: "meta-llama/llama-3.1-405b-instruct:free".to_string(), enabled: true },
                ModelEntry { model_name: "mistralai/mistral-small-3.2-24b-instruct:free".to_string(), enabled: true },
            ],
        }
    }
}

impl AppSettings {
    pub fn theme_name_as_string(&self) -> String {
        match self.theme_name {
            ThemeName::System => "system".to_string(),
            ThemeName::Light => "light".to_string(),
            ThemeName::Dark => "dark".to_string(),
        }
    }

    pub fn set_theme_name_from_str(&mut self, theme_name: &str) {
        let theme_name = match theme_name.to_lowercase().as_str() {
            "system" => ThemeName::System,
            "light" => ThemeName::Light,
            "dark" => ThemeName::Dark,
            _ => {
                error(&format!("Can't recognize theme name: {}", theme_name));
                return;
            }
        };
        self.theme_name = theme_name;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeName {
    #[serde(rename = "system")]
    System,
    #[serde(rename = "light")]
    Light,
    #[serde(rename = "dark")]
    Dark,
}
