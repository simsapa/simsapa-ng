import QtQuick

Item {
    property bool db_loaded: false;

    function load_db() {
        console.log("load_db()");
    }

    function results_page(query: string, page_num: int): string {
        console.log(query);
        return "{}";
    }

    function get_sutta_html(window_id: string, query: string): string {
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

    function app_data_folder_path(): string {
        return "~/.local/share/simsapa-ng";
    }

    function is_app_data_folder_writable(): bool {
        return true;
    }

    function app_data_contents_html_table() {
        return `<table>
                    <tr>
                        <td>file</td>
                        <td>size</td>
                        <td>modified</td>
                    </tr>
                </table>`;
    }

    function app_data_contents_plain_table() {
        return `| file | size | modified |`;
    }

    function dpd_deconstructor_list(query: string): list<string> {
        return [
            "olokita + saññāṇena + eva",
            "olokita + saññāṇena + iva",
        ];
    }

    function get_theme_name(): string {
        return 'dark';
    }

    function get_theme(theme_name: string): string {
        return '{}';
    }

    function get_saved_theme(): string {
        return '{}';
    }
}
