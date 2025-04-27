import QtQuick

Item {
    function search(query: string) {
        console.log(query);
    }

    function get_sutta_html(query: string): string {
        var html = "<!doctype><html><body><h1>%1</h1></body></html>".arg(query);
        return html;
    }

    function get_translations_for_sutta_uid(sutta_uid: string): list<string> {
        // See sutta_search_window_state.py _add_related_tabs()
        let uid_ref = sutta_uid.replace('^([^/]+)/.*', '$1');
        let translations = [
            `${uid_ref}/en/thanissaro`,
            `${uid_ref}/en/bodhi`,
            `${uid_ref}/en/sujato`,
        ];
        return translations;
    }
}
