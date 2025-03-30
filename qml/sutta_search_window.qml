import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import QtQuick.Window
import QtWebView

import com.profound_labs.simsapa 1.0

ApplicationWindow {
    id: aw
    title: qsTr("Simsapa Dhamma Reader - Sutta Search")
    width: 1300
    height: 900
    visible: true
    color: palette.window

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

    Action {
        id: action_focus_search
        shortcut: "Ctrl+L"
        onTriggered: {
            search_input.forceActiveFocus();
            search_input.selectAll();
        }
    }

    Action {
        id: action_quit
        shortcut: StandardKey.Quit
        onTriggered: Qt.quit()
    }

    Action {
        id: action_sutta_search
        shortcut: "F5"
        /* onTriggered: aw.close() */
    }

    Action {
        id: action_Sutta_Study
        shortcut: "Ctrl+F5"
        /* onTriggered: aw.close() */
    }

    Action {
        id: action_Dictionary_Search
        shortcut: "F6"
        /* onTriggered: aw.close() */
    }

    menuBar: MenuBar {
        Menu {
            title: "&File"
            MenuItem {
                text: "&Close Window"
                onTriggered: aw.close()
            }
            MenuItem {
                text: "&Quit Simsapa"
                icon.source: "qrc:/icons/close"
                action: action_quit
            }
        }

        Menu {
            title: "&Windows"
            MenuItem {
                text: "&Sutta Search"
                icon.source: "qrc:/icons/book"
                action: action_sutta_search
            }

            /* MenuItem { */
            /*     text: "Sutta Study" */
            /*     // book icon */
            /*     // Ctrl+F5 */
            /*     onTriggered: action_Sutta_Study() */
            /* } */
            /* MenuItem { */
            /*     text: "&Dictionary Search" */
            /*     // dict icon */
            /*     // F6 */
            /*     onTriggered: action_Dictionary_Search() */
            /* } */
        }
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
