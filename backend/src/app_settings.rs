use indexmap::IndexMap;
use serde::{Serialize, Deserialize};

use crate::logger::error;

static PROVIDERS_JSON: &str = include_str!("../../assets/providers.json");

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelEntry {
    pub model_name: String,
    pub enabled: bool,
    pub removable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provider {
    pub name: ProviderName,
    pub description: String,
    pub enabled: bool,
    /// e.g. OPENROUTER_API_KEY, DEEPSEEK_API_KEY, etc. which may be present as env variables.
    pub api_key_env_var_name: String,
    pub api_key_value: Option<String>,
    pub models: Vec<ModelEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderName {
    Gemini,
    OpenRouter,
    Anthropic,
    OpenAI,
    DeepSeek,
    #[serde(rename = "xAI")]
    XAI,
    Mistral,
    HuggingFace,
    Perplexity,
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
    pub api_keys: IndexMap<String, String>,
    pub system_prompts: IndexMap<String, String>,
    pub providers: Vec<Provider>,
    pub ai_models_auto_retry: bool,
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
            api_keys: IndexMap::new(),
            system_prompts: {
                let mut prompts = IndexMap::new();
                prompts.insert("Gloss Tab: System Prompt".to_string(),
                    r#"
You are a helpful assistant for studying the suttas of the Theravāda Pāli Tipitaka and the Pāli language.
Respond with concise answers and respond only with the information requested in the task.
Respond with GFM-Markdown formatted text.
"#.trim().to_string());

                prompts.insert("Gloss Tab: AI Translation with Vocabulary".to_string(),
                    r#"
Translate the following Pāli passage to English, keeping in mind the provided dictionary definitions.

Pāli passage:

<<PALI_PASSAGE>>

Dictionary definitions:

<<DICTIONARY_DEFINITIONS>>

Respond with only the translation of the Pāli passage.
Respond with GFM-Markdown formatted text.
"#.trim().to_string());

                prompts.insert("Gloss Tab: AI Translation without Vocabulary".to_string(),
                    r#"
Translate the following Pāli passage to English.

Pāli passage:

<<PALI_PASSAGE>>

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
            providers: {
                match serde_json::from_str::<Vec<Provider>>(PROVIDERS_JSON) {
                    Ok(providers) => providers,
                    Err(e) => {
                        error(&format!("Failed to parse providers JSON: {}", e));
                        vec![]
                    }
                }
            },
            ai_models_auto_retry: false,
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
