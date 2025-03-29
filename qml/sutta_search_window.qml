import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import QtQuick.Window
import QtWebView

import com.profound_labs.simsapa 1.0

Item {
    anchors.fill: parent
    // title: qsTr("Simsapa Dhamma Reader - Sutta Search")
    id: aw
    /* width: 1300 */
    /* height: 900 */
    visible: true
    // color: palette.window

    SuttaBridge {
        id: sb
    }

    function load_url(url) {
        webEngineView.url = url;
    }

    function set_query(text) {
        search_input.text = text;
    }

    function show_sutta(query) {
        if (query.length < 4) {
            return;
        }
        var html = sb.get_sutta_html(query);
        webEngineView.loadHtml(html);
    }

    ColumnLayout {
        anchors.fill: parent

        ToolBar {
            id: search_bar
            Layout.fillWidth: true

            RowLayout {
                Layout.fillWidth: true

                TextField {
                    id: search_input
                    Layout.fillWidth: true
                    width: 80
                    focus: true
                    font.pointSize: 12
                    placeholderText: qsTr("Search for suttas...")
                    /* Binding on text { */
                    /*     when: webEngineView */
                    /*     value: webEngineView.url */
                    /* } */
                    /* onAccepted: webEngineView.url = Utils.fromUserInput(text) */
                    selectByMouse: true
                }

                ToolButton {
                    id: search_btn
                    icon.source: "qrc:/icons/search"
                    onClicked: show_sutta(search_input.text)
                    activeFocusOnTab: !aw.platformIsMac
                }
            }
        }

        WebView {
            id: webEngineView
            /* focus: true */
            Layout.fillWidth: true
            Layout.fillHeight: true
            url: "http://localhost:4848/"
        }
    }
}
