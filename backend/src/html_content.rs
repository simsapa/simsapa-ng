use serde::Serialize;
use tinytemplate::TinyTemplate;

use crate::{get_app_globals, is_mobile};

static PAGE_HTML: &str = include_str!("../../assets/templates/page.html");
static FIND_HTML: &str = include_str!("../../assets/templates/find.html");
static TEXT_RESIZE_HTML: &str = include_str!("../../assets/templates/text_resize.html");
static READING_MODE_HTML: &str = include_str!("../../assets/templates/reading_mode.html");
pub static PREV_NEXT_CHAPTER_HTML: &str = include_str!("../../assets/templates/prev_next_chapter.html");
static MENU_HTML: &str = include_str!("../../assets/templates/menu.html");
static CONFIRM_MODAL_HTML: &str = include_str!("../../assets/templates/confirm_modal.html");
static ICONS_HTML: &str = include_str!("../../assets/templates/icons.html");

static SUTTAS_CSS: &str = include_str!("../../assets/css/suttas.css");
static SUTTAS_JS: &str = include_str!("../../assets/js/suttas.js");

#[derive(Serialize)]
struct TmplContext {
    css_head: String,
    api_url: String,
    js_head: String,
    js_body: String,
    reading_mode_html: String,
    prev_next_chapter_html: String,
    find_html: String,
    text_resize_html: String,
    menu_html: String,
    confirm_modal_html: String,
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
            reading_mode_html: READING_MODE_HTML.replace("{api_url}", &g.api_url).to_string(),
            prev_next_chapter_html: "".to_string(),  // Default to empty for suttas
            find_html: FIND_HTML.replace("{api_url}", &g.api_url).to_string(),
            text_resize_html: TEXT_RESIZE_HTML.replace("{api_url}", &g.api_url).to_string(),
            menu_html: MENU_HTML.replace("{api_url}", &g.api_url).to_string(),
            confirm_modal_html: CONFIRM_MODAL_HTML.to_string(),
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
    sutta_html_page_with_nav(content, api_url, css_extra, js_extra, body_class, None)
}

pub fn sutta_html_page_with_nav(content: &str,
                                 api_url: Option<String>,
                                 css_extra: Option<String>,
                                 js_extra: Option<String>,
                                 body_class: Option<String>,
                                 prev_next_chapter_html: Option<String>) -> String {

    let mut tt = TinyTemplate::new();
    tt.set_default_formatter(&tinytemplate::format_unescaped);
    tt.add_template("page_html", PAGE_HTML).expect("Template error in page.html!");

    let mut ctx = TmplContext::default();

    if let Some(s) = body_class {
        ctx.body_class = s.clone();
    }

    if let Some(nav_html) = prev_next_chapter_html {
        ctx.prev_next_chapter_html = nav_html;
    }

    let mut css = String::new();

    if let Some(s) = api_url {
        ctx.api_url = s.clone();
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

    tt.render("page_html", &ctx).unwrap_or_default()
}

pub fn blank_html_page(body_class: Option<String>) -> String {
    let mut tt = TinyTemplate::new();
    tt.set_default_formatter(&tinytemplate::format_unescaped);
    tt.add_template("page_html", PAGE_HTML).expect("Template error in page.html!");

    let mut ctx = TmplContext {
        reading_mode_html: "".to_string(),
        find_html: "".to_string(),
        text_resize_html: "".to_string(),
        menu_html: "".to_string(),
        confirm_modal_html: "".to_string(),
        icons_html: "".to_string(),
        body_class: body_class.unwrap_or_default(),
        ..Default::default()
    };

    let mut css = String::new();

    css.push_str(&SUTTAS_CSS.to_string().replace("http://localhost:8000", &ctx.api_url));

    ctx.css_head = css;

    tt.render("page_html", &ctx).unwrap_or_default()
}
