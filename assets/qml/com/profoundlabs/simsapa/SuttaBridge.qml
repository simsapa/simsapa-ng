import QtQuick

Item {
    function search(query) {
        console.log(query);
    }

    function get_sutta_html(query) {
        var html = "<!doctype><html><body><h1>%1</h1></body></html>".arg(query);
        return html;
    }
}
