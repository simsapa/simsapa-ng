use serde::{Serialize, Deserialize};

use crate::logger::error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppSettings {
    pub sutta_font_size: usize,
    pub sutta_max_width: usize,
    pub show_bookmarks: bool,
    pub show_translation_and_pali_line_by_line: bool,
    pub show_all_variant_readings: bool,
    pub show_glosses: bool,
    pub theme_name: ThemeName,
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
