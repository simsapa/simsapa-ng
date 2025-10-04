use serde::Serialize;
use tinytemplate::TinyTemplate;

use crate::{get_app_globals, is_mobile};

static PAGE_HTML: &'static str = include_str!("../../assets/templates/page.html");
static FIND_HTML: &'static str = include_str!("../../assets/templates/find.html");
static TEXT_RESIZE_HTML: &'static str = include_str!("../../assets/templates/text_resize.html");
static MENU_HTML: &'static str = include_str!("../../assets/templates/menu.html");
static ICONS_HTML: &'static str = include_str!("../../assets/templates/icons.html");

static SUTTAS_CSS: &'static str = include_str!("../../assets/css/suttas.css");
static SUTTAS_JS: &'static str = include_str!("../../assets/js/suttas.js");

#[derive(Serialize)]
struct TmplContext {
    css_head: String,
    api_url: String,
    js_head: String,
    js_body: String,
    find_html: String,
    text_resize_html: String,
    menu_html: String,
    icons_html: String,
    content: String,
    body_class: String,
}

impl Default for TmplContext {
    fn default() -> Self {
        let g = get_app_globals();
        TmplContext {
            css_head: "".to_string(),
            api_url: g.api_url.clone(),
            js_head: "".to_string(),
            js_body: "".to_string(),
            find_html: FIND_HTML.replace("{api_url}", &g.api_url).to_string(),
            text_resize_html: TEXT_RESIZE_HTML.replace("{api_url}", &g.api_url).to_string(),
            menu_html: MENU_HTML.replace("{api_url}", &g.api_url).to_string(),
            icons_html: ICONS_HTML.to_string(),
            content: "".to_string(),
            body_class: "".to_string(),
        }
    }
}

pub fn sutta_html_page(content: &str,
                       api_url: Option<String>,
                       css_extra: Option<String>,
                       js_extra: Option<String>,
                       body_class: Option<String>) -> String {

    let mut tt = TinyTemplate::new();
    tt.set_default_formatter(&tinytemplate::format_unescaped);
    tt.add_template("page_html", PAGE_HTML).expect("Template error in page.html!");

    let mut ctx = TmplContext::default();

    if let Some(s) = body_class {
        ctx.body_class = String::from(s.clone());
    }

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

    js.push_str(&format!(" const IS_MOBILE = {};", is_mobile()));

    if let Some(js_extra) = &js_extra {
        js.push_str(js_extra);
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

pub fn blank_html_page(body_class: Option<String>) -> String {
    let mut tt = TinyTemplate::new();
    tt.set_default_formatter(&tinytemplate::format_unescaped);
    tt.add_template("page_html", PAGE_HTML).expect("Template error in page.html!");

    let mut ctx = TmplContext::default();
    ctx.find_html = "".to_string();
    ctx.text_resize_html = "".to_string();
    ctx.menu_html = "".to_string();
    ctx.icons_html = "".to_string();

    if let Some(s) = body_class {
        ctx.body_class = String::from(s.clone());
    }

    let mut css = String::new();

    css.push_str(&SUTTAS_CSS.to_string().replace("http://localhost:8000", &ctx.api_url));

    ctx.css_head = css;

    match tt.render("page_html", &ctx) {
        Ok(html) => html,
        Err(_) => String::new(),
    }
}
