use serde::{Serialize, Deserialize};

/// All values of the qpalette.h::ColorRole enum.
/// NOTE: window.palette reports it has no noRole member.
///
/// enum ColorRole { WindowText, Button, Light, Midlight, Dark, Mid,
///                 Text, BrightText, ButtonText, Base, Window, Shadow,
///                 Highlight, HighlightedText,
///                 Link, LinkVisited,
///                 AlternateBase,
///                 NoRole,
///                 ToolTipBase, ToolTipText,
///                 PlaceholderText,
///                 Accent,
///                 NColorRoles = Accent + 1,
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColorGroupValues {
    pub window: String,
    pub window_text: String,
    pub base: String,
    pub alternate_base: String,
    pub accent: String,
    pub text: String,

    pub button: String,
    pub button_text: String,

    pub bright_text: String,
    pub placeholder_text: String,

    pub highlight: String,
    pub highlighted_text: String,
    pub tool_tip_base: String,
    pub tool_tip_text: String,

    pub light: String,
    pub midlight: String,
    pub dark: String,
    pub mid: String,
    pub shadow: String,
    pub link: String,
    pub link_visited: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeColors {
    active: ColorGroupValues,
    inactive: ColorGroupValues,
    disabled: ColorGroupValues,
}

pub static THEME_COLORS_LIGHT_JSON: &str = include_str!("theme_colors_light.json");
pub static THEME_COLORS_DARK_JSON: &str = include_str!("theme_colors_dark.json");

impl ThemeColors {
    pub fn light_json() -> String {
        THEME_COLORS_LIGHT_JSON.to_string()
    }

    pub fn dark_json() -> String {
        THEME_COLORS_DARK_JSON.to_string()
    }
}
