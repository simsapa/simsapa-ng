import QtQuick
import QtWebView

import com.profoundlabs.simsapa

/*
 * Mobile WebView Visibility Management
 *
 * This component wraps QtWebView in an Item container to provide proper visibility control.
 *
 * On mobile platforms (Android/iOS), QtWebView uses native platform views (Android WebView,
 * WKWebView) that render in a separate layer above Qt Quick content. These native views don't
 * respect QML's visibility hierarchy, so simply setting visible: false on parent items doesn't
 * reliably hide them.
 *
 * Solution:
 * 1. Wrap WebView in an Item container that participates in QML's visibility hierarchy
 * 2. Explicitly bind WebView's visible property to the container's visibility
 * 3. Set enabled: false in addition to visible: false to stop native rendering
 *
 * This ensures the WebView is properly hidden when it should not be visible, preventing
 * blank yellow webviews from covering the screen.
 *
 * See docs/mobile-webview-visibility-management.md for detailed explanation.
 */

Item {
    id: root
    anchors.fill: parent

    property string window_id
    property bool is_dark
    property bool is_reading_mode

    property string data_json

    property string item_uid
    property string table_name
    property string sutta_ref
    property string sutta_title
    property string anchor

    property alias web: web

    signal page_loaded()

    Timer {
        id: scroll_timer
        interval: 300
        repeat: false
        onTriggered: root.scroll_to_anchor()
    }

    function set_properties_from_data_json() {
        if (!root.data_json || root.data_json.length === 0) {
            return;
        }
        try {
            let data = JSON.parse(root.data_json);
            root.item_uid = data.item_uid || "";
            root.table_name = data.table_name || "";
            root.sutta_ref = data.sutta_ref || "";
            root.sutta_title = data.sutta_title || "";
            root.anchor = data.anchor || "";
        } catch (e) {
            console.error("Failed to parse data_json:", e, "data_json:", root.data_json);
        }
    }

    function show_transient_message(msg: string) {
        let js = `var msg = \`${msg}\`; document.SSP.show_transient_message(msg, "transient-messages-top");`;
        web.runJavaScript(js);
    }

    function show_find_bar() {
        web.forceActiveFocus();
        web.runJavaScript(`document.SSP.find.show();`);
    }

    function find_next() {
        web.runJavaScript(`document.SSP.find.nextMatch();`);
    }

    function find_previous() {
        web.runJavaScript(`document.SSP.find.previousMatch();`);
    }

    function load_sutta_uid(uid) {
        if (uid == "Sutta") {
            // Initial blank page
            uid = "";
        }

        // For empty UID, use loadHtml to avoid 404 from API endpoint
        if (uid === "") {
            var html = SuttaBridge.get_sutta_html(root.window_id, "");
            web.loadHtml(html);
            return;
        }

        const api_url = SuttaBridge.get_api_url();
        let url = `${api_url}/get_sutta_html_by_uid/${root.window_id}/${uid}/`;
        if (root.anchor && root.anchor.length > 0) {
            // Ensure anchor has # prefix
            let anchor_fragment = root.anchor.startsWith('#') ? root.anchor : `#${root.anchor}`;
            url = `${url}${anchor_fragment}`;
        }
        web.url = url;
    }

    function load_word_uid(uid) {
        if (uid == "Word") {
            // Initial blank page
            uid = "";
        }
        if (root.table_name === "dpd_headwords") {
            // Results from DPD Lookup are in the form of
            // "item_uid": "25671/dpd", "table_name": "dpd_headwords", "sutta_title":"cakka 1"
            // SuttaBridge.get_word_html() needs the uid for dict_words table in dictionaries.sqlite3
            // where the form is "uid": "cakka 1/dpd"
            uid = `${root.sutta_title}/dpd`;
        }
        var html = SuttaBridge.get_word_html(root.window_id, uid);
        web.loadHtml(html);
    }

    function load_book_spine_uid(spine_item_uid) {
        // Check if this is a PDF book
        const api_url = SuttaBridge.get_api_url();
        if (SuttaBridge.is_spine_item_pdf(spine_item_uid)) {
            // Load PDF viewer with file parameter
            const book_uid = SuttaBridge.get_book_uid_for_spine_item(spine_item_uid);
            const pdf_url = `${api_url}/book_resources/${book_uid}/document.pdf`;
            web.url = `${api_url}/assets/pdf-viewer/web/viewer.html?file=${encodeURIComponent(pdf_url)}`;
        } else {
            // Regular book content
            // Append anchor to URL for native browser scrolling (works on all platforms)
            // On same-page reloads, clear and re-set URL to trigger scroll
            let url = `${api_url}/get_book_spine_item_html_by_uid/${root.window_id}/${spine_item_uid}/`;
            if (root.anchor && root.anchor.length > 0) {
                // Ensure anchor has # prefix
                let anchor_fragment = root.anchor.startsWith('#') ? root.anchor : `#${root.anchor}`;
                url = `${url}${anchor_fragment}`;
            }
            web.url = url;
        }
    }

    function scroll_to_anchor() {
        if (root.anchor && root.anchor.length > 0) {
            // Remove the leading # if present
            let anchor_id = root.anchor.startsWith('#') ? root.anchor.substring(1) : root.anchor;

            // Try to scroll to the element with the anchor ID
            let js = `
                (function() {
                    var element = document.getElementById('${anchor_id}');
                    if (element) {
                        element.scrollIntoView({ behavior: 'auto', block: 'start' });
                        return true;
                    }
                    // Also try with querySelector in case it's a more complex selector
                    element = document.querySelector('a[name="${anchor_id}"]');
                    if (element) {
                        element.scrollIntoView({ behavior: 'auto', block: 'start' });
                        return true;
                    }
                    // Try with the hash directly
                    element = document.querySelector('${root.anchor}');
                    if (element) {
                        element.scrollIntoView({ behavior: 'auto', block: 'start' });
                        return true;
                    }
                    return false;
                })();
            `;
            web.runJavaScript(js);
        }
    }

    Component.onCompleted: {
        root.set_properties_from_data_json();
        // Both "dict_words" and "dpd_headwords" should load dictionary content
        if (root.table_name === "dict_words" || root.table_name === "dpd_headwords") {
            root.load_word_uid(root.item_uid);
        } else if (root.table_name === "book_spine_items") {
            root.load_book_spine_uid(root.item_uid);
        } else {
            root.load_sutta_uid(root.item_uid);
        }
    }

    // Load the sutta or dictionary word when the Loader in SuttaHtmlView updates data_json
    onData_jsonChanged: function() {
        root.set_properties_from_data_json();
        // Both "dict_words" and "dpd_headwords" should load dictionary content
        if (root.table_name === "dict_words" || root.table_name === "dpd_headwords") {
            root.load_word_uid(root.item_uid);
        } else if (root.table_name === "book_spine_items") {
            root.load_book_spine_uid(root.item_uid);
        } else {
            root.load_sutta_uid(root.item_uid);
        }
    }

    onIs_darkChanged: function() {
        let js = "";
        if (root.is_dark) {
            js = `
document.body.classList.add('dark');
document.documentElement.classList.add('dark');
document.documentElement.style.colorScheme = 'dark';
`;
        } else {
            js = `
document.body.classList.remove('dark');
document.documentElement.classList.remove('dark');
document.documentElement.style.colorScheme = 'light';
`;
        }
        web.runJavaScript(js);
    }

    onIs_reading_modeChanged: function() {
        let js = "";
        if (root.is_reading_mode) {
            js = `
if (document.getElementById('readingModeButton')) {
    document.getElementById('readingModeButton').classList.add('active');
}
`;
        } else {
            js = `
if (document.getElementById('readingModeButton')) {
    document.getElementById('readingModeButton').classList.remove('active');
}
`;
        }
        web.runJavaScript(js);
    }

    Connections {
        target: SuttaBridge

        function onShowBottomFootnotesChanged() {
            const enabled = SuttaBridge.get_show_bottom_footnotes();
            const js = `
if (document.SSP) {
    document.SSP.show_bottom_footnotes = ${enabled};
    if (window.footnote_bottom_bar_refresh) {
        window.footnote_bottom_bar_refresh();
    }
}`;
            web.runJavaScript(js);
        }
    }

    WebView {
        id: web
        anchors.fill: parent
        visible: root.visible
        enabled: root.visible

        onLoadingChanged: function(loadRequest) {
            if (root.is_dark) {
                web.runJavaScript("document.documentElement.style.colorScheme = 'dark';");
            }
            if (root.is_reading_mode) {
                let js = `
if (document.getElementById('readingModeButton')) {
    document.getElementById('readingModeButton').classList.add('active');
}
`;
                web.runJavaScript(js);
            }
            // Set footnote bottom bar setting from database
            const show_footnotes = SuttaBridge.get_show_bottom_footnotes();
            web.runJavaScript(`
if (document.SSP) {
    document.SSP.show_bottom_footnotes = ${show_footnotes};
}
`);
            if (loadRequest.loadProgress === 100) {
                root.page_loaded();
                // Note: Anchor scrolling is now handled natively by including it in the URL
                //
                // The JavaScript fallback provides additional reliability in case:
                // - The anchor element has a different attribute (like `name` instead of `id`)
                // - There are timing issues with native scrolling
                // - The WebView needs a "nudge" to complete scrolling
                if (root.anchor && root.anchor.length > 0) {
                    scroll_timer.restart();
                }
            }
        }
    }
}
