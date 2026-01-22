use indexmap::IndexMap;
use serde::{Serialize, Deserialize};

use crate::logger::error;

static PROVIDERS_JSON: &str = include_str!("../../assets/providers.json");
pub static LANGUAGES_JSON: &str = include_str!("../../assets/languages.json");
pub static SUTTA_REFERENCE_CONVERTER_JSON: &str = include_str!("../../assets/sutta-reference-converter.json");
pub static CIPS_GENERAL_INDEX_JSON: &str = include_str!("../../assets/general-index.json");
static KEYBINDINGS_JSON: &str = include_str!("../../assets/keybindings.json");

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
    pub anki_template_front: String,
    pub anki_template_back: String,
    pub anki_template_cloze_front: String,
    pub anki_template_cloze_back: String,
    pub anki_export_format: AnkiExportFormat,
    pub anki_include_cloze: bool,
    pub search_as_you_type: bool,
    pub open_find_in_sutta_results: bool,
    pub show_bottom_footnotes: bool,
    pub first_time_start: bool,
    pub sutta_language_filter_key: String,
    pub mobile_top_bar_margin: MobileTopBarMargin,
    /// Whether to show update notifications on startup
    pub notify_about_simsapa_updates: bool,
    /// Release channel for updates (e.g., "main", "development", "simsapa-ng")
    /// None means use default "simsapa-ng"
    pub release_channel: Option<String>,
    /// Custom keyboard shortcuts for application actions
    pub app_keybindings: AppKeybindings,
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
            theme_name: ThemeName::Light,
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

            anki_template_front: r#"<style>
.word \{
color: #CF6303;
font-size: 1.2em;
}
.snippet \{ color: #504949; }
</style>
<div>
<p class="word">{word_stem}</p>
<p class="snippet">{context_snippet}</p>
</div>"#.to_string(),

            anki_template_back: r#"<style>
.gram \{ color: #BA9903; font-weight: bold; }
.constr \{ color: #890339; font-weight: bold; }
.constr-desc \{ color: #6E505E; }
table \{ text-align: left; }
table tr \{ text-align: left; }
table tr td \{ text-align: left; padding: 0.1em 0.5em; }
</style>
<div>
<p>{vocab.summary}</p>
<table style="padding-top: 0.5em;">
<tr>
    <td class="gram">Meaning:</td>
    <td>{dpd.meaning_1}</td>
</tr>
<tr>
    <td class="gram">Grammar:</td>
    <td>{dpd.grammar}</td>
</tr>
<tr>
    <td class="gram">Pos:</td>
    <td>{dpd.pos}</td>
</tr>
<tr>
    <td class="constr">Root:</td>
    <td class="constr-desc">
        {{ if dpd.root_key }}
            {root.root_clean}･{root.root_group} {root.root_sign} ({root.root_meaning})
        {{ endif }}
    </td>
</tr>
<tr>
    <td class="constr">Construction:</td>
    <td class="constr-desc">{dpd.construction}</td>
</tr>
</table>
</div>"#.to_string(),

            anki_template_cloze_front: "<div>{context_snippet}</div>".to_string(),
            anki_template_cloze_back: "<div>{vocab.summary}</div>".to_string(),
            anki_export_format: AnkiExportFormat::Templated,
            anki_include_cloze: true,
            search_as_you_type: true,
            open_find_in_sutta_results: true,
            show_bottom_footnotes: true,
            first_time_start: true,
            sutta_language_filter_key: String::new(),
            mobile_top_bar_margin: MobileTopBarMargin::default(),
            notify_about_simsapa_updates: true,
            release_channel: None,
            app_keybindings: AppKeybindings::default(),
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

    pub fn is_mobile_top_bar_margin_system(&self) -> bool {
        matches!(self.mobile_top_bar_margin, MobileTopBarMargin::SystemValue)
    }

    pub fn get_mobile_top_bar_margin_custom_value(&self) -> u32 {
        match self.mobile_top_bar_margin {
            MobileTopBarMargin::CustomValue(value) => value,
            MobileTopBarMargin::SystemValue => 0,
        }
    }

    pub fn set_mobile_top_bar_margin_system(&mut self) {
        self.mobile_top_bar_margin = MobileTopBarMargin::SystemValue;
    }

    pub fn set_mobile_top_bar_margin_custom(&mut self, value: u32) {
        self.mobile_top_bar_margin = MobileTopBarMargin::CustomValue(value);
    }
}

/// Anki CSV export format type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnkiExportFormat {
    Simple,
    Templated,
    DataCsv,
}

/// Mobile top bar margin setting
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Default)]
pub enum MobileTopBarMargin {
    #[default]
    SystemValue,
    CustomValue(u32),
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

/// Definition of a keybinding action loaded from JSON (used for defaults and UI display)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeybindingDefinition {
    /// Unique identifier for the action (e.g., "focus_search")
    pub id: String,
    /// Human-readable name for display (e.g., "Focus Search Input")
    pub name: String,
    /// Description of what the action does (for UI help text)
    pub description: String,
    /// Default keyboard shortcuts for this action
    pub shortcuts: Vec<String>,
}

/// Returns the keybinding definitions loaded from the embedded JSON
fn get_keybinding_definitions() -> Vec<KeybindingDefinition> {
    match serde_json::from_str::<Vec<KeybindingDefinition>>(KEYBINDINGS_JSON) {
        Ok(definitions) => definitions,
        Err(e) => {
            error(&format!("Failed to parse keybindings JSON: {}", e));
            vec![]
        }
    }
}

/// Stores custom keyboard shortcuts for application actions.
/// Maps action IDs to lists of key sequences (e.g., "settings" -> ["Ctrl+,"])
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppKeybindings {
    /// Mapping of action ID to list of keyboard shortcuts
    pub bindings: IndexMap<String, Vec<String>>,
}

impl Default for AppKeybindings {
    fn default() -> Self {
        let definitions = get_keybinding_definitions();
        let mut bindings = IndexMap::new();

        for def in definitions {
            bindings.insert(def.id, def.shortcuts);
        }

        AppKeybindings { bindings }
    }
}

impl AppKeybindings {
    /// Returns a mapping of action IDs to human-readable names for UI display.
    pub fn get_action_names() -> IndexMap<String, String> {
        let definitions = get_keybinding_definitions();
        let mut names = IndexMap::new();

        for def in definitions {
            names.insert(def.id, def.name);
        }

        names
    }

    /// Returns a mapping of action IDs to descriptions for UI display.
    pub fn get_action_descriptions() -> IndexMap<String, String> {
        let definitions = get_keybinding_definitions();
        let mut descriptions = IndexMap::new();

        for def in definitions {
            descriptions.insert(def.id, def.description);
        }

        descriptions
    }
}
