import QtQuick
import QtWebEngine

import com.profoundlabs.simsapa

Item {
    id: root
    anchors.fill: parent

    property string window_id
    property bool is_dark

    property string data_json

    property string item_uid
    property string table_name
    property string sutta_ref
    property string sutta_title

    property alias web: web

    signal page_loaded()

    function set_properties_from_data_json() {
        let data = JSON.parse(root.data_json);
        root.item_uid = data.item_uid;
        root.table_name = data.table_name;
        root.sutta_ref = data.sutta_ref;
        root.sutta_title = data.sutta_title;
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
        var html = SuttaBridge.get_sutta_html(root.window_id, uid);
        web.loadHtml(html);
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
            web.url = `${api_url}/get_book_spine_item_html_by_uid/${root.window_id}/${spine_item_uid}/`;
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

    WebEngineView {
        id: web
        anchors.fill: parent
        visible: root.visible
        enabled: root.visible

        onLoadingChanged: function(loadRequest) {
            if (root.is_dark) {
                web.runJavaScript("document.documentElement.style.colorScheme = 'dark';");
            }
            if (loadRequest.status === WebEngineView.LoadSucceededStatus) {
                root.page_loaded();
            }
        }
    }
}
