use crate::db::appdata_models::*;

use std::collections::BTreeMap;
use anyhow::{anyhow, Context, Result};

use crate::helpers::{bilara_line_by_line_html, bilara_content_json_to_html};
use crate::html_content::html_page;
use crate::types::SuttaQuote;
use crate::app_data::AppData;

/// Renders the complete HTML page for a sutta.
///
/// See also: simsapa/simsapa/app/export_helpers.py::render_sutta_content()
pub fn render_sutta_content(
    app_data: &mut AppData,
    sutta: &Sutta,
    sutta_quote: Option<&SuttaQuote>,
    js_extra_pre: Option<String>,
) -> Result<String> {
    let content_html_body = if let Some(ref content_json_str) = sutta.content_json {
        if !content_json_str.is_empty() {
            // Check setting for line-by-line view
            let line_by_line = app_data.get_setting_or("show_translation_and_pali_line_by_line", true);

            // Attempt to fetch Pali sutta if needed
            let pali_sutta_result = if line_by_line && sutta.language != "pli" {
                 app_data.get_pali_for_translated(sutta)
            } else {
                Ok(None)
            };
            let pali_sutta = pali_sutta_result.context("Failed to get Pali sutta for translated version")?;

            if line_by_line && pali_sutta.is_some() {
                // Generate line-by-line HTML
                let pali_sutta = pali_sutta.unwrap();
                let translated_segments = app_data.sutta_to_segments_json(sutta, false)
                    .context("Failed to generate translated segments for line-by-line view")?;
                let pali_segments = app_data.sutta_to_segments_json(&pali_sutta, false)
                     .context("Failed to generate Pali segments for line-by-line view")?;

                let tmpl_str = sutta.content_json_tmpl.as_deref()
                    .ok_or_else(|| anyhow!("Sutta {} requires content_json_tmpl for line-by-line view", sutta.uid))?;
                // Parse template into BTreeMap as well
                let tmpl_json: BTreeMap<String, String> = serde_json::from_str(tmpl_str)
                    .with_context(|| format!("Failed to parse template JSON into BTreeMap for line-by-line view (Sutta: {})", sutta.uid))?;

                bilara_line_by_line_html(&translated_segments, &pali_segments, &tmpl_json)?
            } else {
                // Generate standard HTML view (using template within sutta_to_segments_json)
                 let segments_json = app_data.sutta_to_segments_json(sutta, true)
                     .context("Failed to generate segments for standard view")?;
                 bilara_content_json_to_html(&segments_json)?
            }
        } else {
             "<div class='suttacentral bilara-text'></div>".to_string()
        }

    } else if let Some(ref html) = sutta.content_html {
         if !html.is_empty() { html.clone() }
         else { "<div class='suttacentral bilara-text'></div>".to_string() }

    } else if let Some(ref plain) = sutta.content_plain {
         if !plain.is_empty() { format!("<div class='suttacentral bilara-text'><pre>{}</pre></div>", plain) }
         else { "<div class='suttacentral bilara-text'></div>".to_string() }

    } else {
        "<div class='suttacentral bilara-text'><p>No content.</p></div>".to_string()
    };

    // Get display settings
    let font_size: f64 = app_data.get_setting_or("sutta_font_size", 22.0);
    let max_width: f64 = app_data.get_setting_or("sutta_max_width", 75.0);

    // Format CSS and JS extras
    let css_extra = format!("html {{ font-size: {}px; }} body {{ max-width: {}ex; }}", font_size, max_width);

    let mut js_extra = format!("const SUTTA_UID = '{}';", sutta.uid);

    if let Some(js_pre) = js_extra_pre {
        js_extra = format!("{}; {}", js_pre, js_extra);
    }

    let show_bookmarks: bool = app_data.get_setting_or("show_bookmarks", true);
    js_extra.push_str(&format!(" const SHOW_BOOKMARKS = {};", show_bookmarks));

    if let Some(quote) = sutta_quote {
        // Escape the quote text for JavaScript string literal
        let escaped_text = quote.quote.replace('\\', "\\\\").replace('"', "\\\"");
        js_extra.push_str(&format!(r#" document.addEventListener("DOMContentLoaded", function(event) {{ highlight_and_scroll_to("{}"); }}); const SHOW_QUOTE = "{}";"#, escaped_text, escaped_text));
    }

    // Wrap content in the full HTML page structure
    let final_html = html_page(
        &content_html_body,
        Some(app_data.api_url.to_string()),
        Some(css_extra.to_string()),
        Some(js_extra.to_string()),
        app_data.get_theme_name(),
    );

    Ok(final_html)
}
