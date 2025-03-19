use serde::Serialize;
use tinytemplate::TinyTemplate;

use crate::API_URL;

static PAGE_HTML: &'static str = include_str!("../../assets/templates/page.html");
static ICONS_HTML: &'static str = include_str!("../../assets/templates/icons.html");
static SUTTAS_CSS: &'static str = include_str!("../../assets/css/suttas.css");
static SUTTAS_JS: &'static str = include_str!("../../assets/js/suttas.js");

#[derive(Serialize)]
struct TmplContext {
    css_head: String,
    api_url: String,
    js_head: String,
    js_body: String,
    icons_html: String,
    content: String,
}

impl Default for TmplContext {
    fn default() -> Self {
        TmplContext {
            css_head: "".to_string(),
            api_url: API_URL.to_string(),
            js_head: "".to_string(),
            js_body: "".to_string(),
            icons_html: ICONS_HTML.to_string(),
            content: "".to_string(),
        }
    }
}

pub fn html_page(content: &str,
                 api_url: Option<String>,
                 css_extra: Option<String>,
                 js_extra: Option<String>) -> String {

    let mut tt = TinyTemplate::new();
    tt.set_default_formatter(&tinytemplate::format_unescaped);
    tt.add_template("page_html", PAGE_HTML).expect("Template error in page.html!");

    let mut ctx = TmplContext::default();

    let mut css = String::new();

    if let Some(s) = api_url {
        ctx.api_url = String::from(s.clone());
    }
    css.push_str(&SUTTAS_CSS.to_string().replace("http://localhost:8000", &ctx.api_url));

    if let Some(s) = css_extra {
        css.push_str("\n\n");
        css.push_str(&s);
    }

    let mut js = String::new();

    if let Some(js_extra) = &js_extra {
        if !js_extra.contains("SHOW_BOOKMARKS") {
            js.push_str(" const SHOW_BOOKMARKS = false;");
        }
    } else {
        js.push_str(" const SHOW_BOOKMARKS = false;");
    }

    if let Some(js_extra) = &js_extra {
        if !js_extra.contains("SHOW_QUOTE") {
            js.push_str(" const SHOW_QUOTE = null;");
        }
    } else {
        js.push_str(" const SHOW_QUOTE = null;");
    }

    // In suttas.js we expect SHOW_BOOKMARKS to be already set.
    js.push_str(SUTTAS_JS);

    ctx.css_head = css;
    ctx.js_head = js;
    ctx.content = String::from(content);

    match tt.render("page_html", &ctx) {
        Ok(html) => html,
        Err(_) => String::new(),
    }
}
