pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls

Dialog {
    id: control

    required property var tabs_pinned_model
    required property var tabs_results_model
    required property var tabs_translations_model
    required property var nav_history
    required property bool is_wide
    required property bool is_tall

    signal tabSelected(string id_key)
    signal historyItemSelected(string item_uid, string table_name, string sutta_ref, string sutta_title)
    signal clearAllTabs()
    signal clearHistory()

    // title: "Tabs and History"
    modal: true

    x: (parent.width - width) / 2
    y: (parent.height - height) / 2
    width: Math.min(parent.width * 0.9, 600)
    height: control.is_tall ? Math.min(parent.height * 0.7, 400) : Math.min(parent.height * 0.9, 800)

    // Track which column is active: "tabs" or "history"
    property string active_column: "tabs"

    footer: DialogButtonBox {
        Button {
            text: "Open"
            enabled: {
                if (control.active_column === "tabs") {
                    return tab_list_view.currentIndex >= 0;
                } else {
                    return history_list_view.currentIndex >= 0;
                }
            }
            DialogButtonBox.buttonRole: DialogButtonBox.AcceptRole
            onClicked: control.open_selected_item()
        }
        Button {
            text: "Close"
            DialogButtonBox.buttonRole: DialogButtonBox.RejectRole
            onClicked: control.close()
        }
    }

    Shortcut {
        sequences: ["Up", "K"]
        enabled: control.visible
        onActivated: {
            let view = control.active_column === "tabs" ? tab_list_view : history_list_view;
            if (view.currentIndex > 0) {
                view.currentIndex--;
            } else {
                view.currentIndex = view.count - 1;
            }
        }
    }

    Shortcut {
        sequences: ["Down", "J"]
        enabled: control.visible
        onActivated: {
            let view = control.active_column === "tabs" ? tab_list_view : history_list_view;
            if (view.currentIndex < view.count - 1) {
                view.currentIndex++;
            } else {
                view.currentIndex = 0;
            }
        }
    }

    Shortcut {
        sequences: ["Home", "G"]
        enabled: control.visible
        onActivated: {
            let view = control.active_column === "tabs" ? tab_list_view : history_list_view;
            view.currentIndex = 0;
        }
    }

    Shortcut {
        sequences: ["End", "Shift+G"]
        enabled: control.visible
        onActivated: {
            let view = control.active_column === "tabs" ? tab_list_view : history_list_view;
            view.currentIndex = view.count - 1;
        }
    }

    Shortcut {
        sequences: ["Left", "H"]
        enabled: control.visible
        onActivated: control.active_column = "tabs"
    }

    Shortcut {
        sequences: ["Right", "L"]
        enabled: control.visible
        onActivated: control.active_column = "history"
    }

    Shortcut {
        sequence: "Return"
        enabled: control.visible
        onActivated: control.open_selected_item()
    }

    Shortcut {
        sequence: "Enter"
        enabled: control.visible
        onActivated: control.open_selected_item()
    }

    contentItem: GridLayout {
        columns: control.is_wide ? 3 : 1
        columnSpacing: 8
        rowSpacing: 8

        // Tabs section
        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.preferredWidth: control.is_wide ? 1 : -1
            Layout.preferredHeight: control.is_wide ? -1 : 1

            RowLayout {
                Layout.fillWidth: true

                Label {
                    text: "Tabs"
                    font.bold: true
                    font.underline: control.active_column === "tabs"
                    Layout.fillWidth: true
                }

                Button {
                    text: "Clear"
                    flat: true
                    font.pointSize: 9
                    enabled: combined_tabs_model.count > 0
                    onClicked: {
                        control.clearAllTabs();
                        control.populate_model();
                    }
                }
            }

            ScrollView {
                Layout.fillWidth: true
                Layout.fillHeight: true

                ListView {
                    id: tab_list_view
                    clip: true
                    currentIndex: 0
                    highlightFollowsCurrentItem: true

                    model: ListModel {
                        id: combined_tabs_model
                    }

                    highlight: Rectangle {
                        color: control.palette.highlight
                        opacity: 0.3
                        visible: control.active_column === "tabs"
                    }

                    delegate: ItemDelegate {
                        id: tab_item_delegate
                        required property int index
                        required property string item_uid
                        required property string table_name
                        required property string sutta_title
                        required property string sutta_ref
                        required property string id_key
                        required property string group_label

                        width: ListView.view.width
                        highlighted: ListView.isCurrentItem && control.active_column === "tabs"

                        contentItem: RowLayout {
                            spacing: 8

                            Label {
                                text: tab_item_delegate.group_label
                                font.bold: true
                                Layout.preferredWidth: 60
                                color: {
                                    if (tab_item_delegate.highlighted) return "white";
                                    if (tab_item_delegate.group_label === "Pinned") return control.palette.link;
                                    if (tab_item_delegate.group_label === "Results") return control.palette.text;
                                    if (tab_item_delegate.group_label === "Trans") return "#2e7d32";
                                    return control.palette.text;
                                }
                            }

                            Label {
                                text: {
                                    if (tab_item_delegate.table_name && tab_item_delegate.table_name === "dpd_headwords") {
                                        // "cakka-1/dpd" (spaces replaced with hyphens)
                                        return `${tab_item_delegate.sutta_title.replace(/ /g, "-")}/dpd`;
                                    } else {
                                        return tab_item_delegate.item_uid;
                                    }
                                }
                                Layout.fillWidth: true
                                elide: Text.ElideRight
                                color: tab_item_delegate.highlighted ? "white" : control.palette.text
                            }
                        }

                        onClicked: {
                            control.active_column = "tabs";
                            tab_list_view.currentIndex = tab_item_delegate.index;
                        }

                        onDoubleClicked: {
                            control.active_column = "tabs";
                            tab_list_view.currentIndex = tab_item_delegate.index;
                            control.open_selected_item();
                        }
                    }

                    Component.onCompleted: control.populate_model()
                }
            }
        }

        // Separator
        Rectangle {
            Layout.fillHeight: control.is_wide
            Layout.fillWidth: !control.is_wide
            Layout.preferredWidth: control.is_wide ? 1 : -1
            Layout.preferredHeight: control.is_wide ? -1 : 1
            color: control.palette.mid
        }

        // History section
        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.preferredWidth: control.is_wide ? 1 : -1
            Layout.preferredHeight: control.is_wide ? -1 : 1

            RowLayout {
                Layout.fillWidth: true

                Label {
                    text: "History"
                    font.bold: true
                    font.underline: control.active_column === "history"
                    Layout.fillWidth: true
                }

                Button {
                    text: "Clear"
                    flat: true
                    font.pointSize: 9
                    enabled: history_list_model.count > 0
                    onClicked: {
                        control.clearHistory();
                        control.populate_history_model();
                    }
                }
            }

            ScrollView {
                Layout.fillWidth: true
                Layout.fillHeight: true

                ListView {
                    id: history_list_view
                    clip: true
                    currentIndex: 0
                    highlightFollowsCurrentItem: true

                    model: ListModel {
                        id: history_list_model
                    }

                    highlight: Rectangle {
                        color: control.palette.highlight
                        opacity: 0.3
                        visible: control.active_column === "history"
                    }

                    delegate: ItemDelegate {
                        id: history_item_delegate
                        required property int index
                        required property string item_uid
                        required property string table_name
                        required property string sutta_title
                        required property string sutta_ref

                        width: ListView.view.width
                        highlighted: ListView.isCurrentItem && control.active_column === "history"

                        contentItem: Label {
                            text: {
                                if (history_item_delegate.table_name && history_item_delegate.table_name === "dpd_headwords") {
                                    // "cakka-1/dpd" (spaces replaced with hyphens)
                                    return `${history_item_delegate.sutta_title.replace(/ /g, "-")}/dpd`;
                                } else {
                                    return history_item_delegate.item_uid;
                                }
                            }
                            elide: Text.ElideRight
                            color: history_item_delegate.highlighted ? "white" : control.palette.text
                        }

                        onClicked: {
                            control.active_column = "history";
                            history_list_view.currentIndex = history_item_delegate.index;
                        }

                        onDoubleClicked: {
                            control.active_column = "history";
                            history_list_view.currentIndex = history_item_delegate.index;
                            control.open_selected_item();
                        }
                    }
                }
            }
        }
    }

    function open_selected_item() {
        if (control.active_column === "tabs") {
            if (tab_list_view.currentIndex >= 0) {
                let item = combined_tabs_model.get(tab_list_view.currentIndex);
                if (item) {
                    control.tabSelected(item.id_key);
                    control.close();
                }
            }
        } else {
            if (history_list_view.currentIndex >= 0) {
                let item = history_list_model.get(history_list_view.currentIndex);
                if (item) {
                    control.historyItemSelected(item.item_uid, item.table_name, item.sutta_ref, item.sutta_title);
                    control.close();
                }
            }
        }
    }

    function is_blank_tab(item_uid) {
        return !item_uid || item_uid === "Sutta" || item_uid === "Word";
    }

    function populate_model() {
        combined_tabs_model.clear();

        // Add pinned tabs
        for (let i = 0; i < tabs_pinned_model.count; i++) {
            let tab_data = tabs_pinned_model.get(i);
            if (control.is_blank_tab(tab_data.item_uid)) continue;
            combined_tabs_model.append({
                item_uid: tab_data.item_uid,
                table_name: tab_data.table_name,
                sutta_title: tab_data.sutta_title,
                sutta_ref: tab_data.sutta_ref,
                id_key: tab_data.id_key,
                group_label: "Pinned"
            });
        }

        // Add results tabs
        for (let i = 0; i < tabs_results_model.count; i++) {
            let tab_data = tabs_results_model.get(i);
            if (control.is_blank_tab(tab_data.item_uid)) continue;
            combined_tabs_model.append({
                item_uid: tab_data.item_uid,
                table_name: tab_data.table_name,
                sutta_title: tab_data.sutta_title,
                sutta_ref: tab_data.sutta_ref,
                id_key: tab_data.id_key,
                group_label: "Results"
            });
        }

        // Add translation tabs
        for (let i = 0; i < tabs_translations_model.count; i++) {
            let tab_data = tabs_translations_model.get(i);
            if (control.is_blank_tab(tab_data.item_uid)) continue;
            combined_tabs_model.append({
                item_uid: tab_data.item_uid,
                table_name: tab_data.table_name,
                sutta_title: tab_data.sutta_title,
                sutta_ref: tab_data.sutta_ref,
                id_key: tab_data.id_key,
                group_label: "Trans"
            });
        }
    }

    function populate_history_model() {
        history_list_model.clear();

        // Iterate nav_history in reverse order (most recent first)
        for (let i = control.nav_history.length - 1; i >= 0; i--) {
            let entry = control.nav_history[i];
            history_list_model.append({
                item_uid: entry.item_uid || "",
                table_name: entry.table_name || "",
                sutta_ref: entry.sutta_ref || "",
                sutta_title: entry.sutta_title || "",
            });
        }
    }

    onAboutToShow: {
        populate_model();
        populate_history_model();
        active_column = "tabs";
        tab_list_view.currentIndex = 0;
        tab_list_view.positionViewAtBeginning();
        history_list_view.currentIndex = 0;
        history_list_view.positionViewAtBeginning();
    }
}
