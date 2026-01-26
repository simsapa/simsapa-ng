pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls

Dialog {
    id: control

    required property var tabs_pinned_model
    required property var tabs_results_model
    required property var tabs_translations_model

    signal tabSelected(string id_key)

    title: "Select a Tab to Focus"
    modal: true

    x: (parent.width - width) / 2
    y: (parent.height - height) / 2
    width: Math.min(parent.width * 0.6, 500)
    height: Math.min(parent.height * 0.7, 400)

    footer: DialogButtonBox {
        Button {
            text: "Open"
            enabled: tab_list_view.currentIndex >= 0
            DialogButtonBox.buttonRole: DialogButtonBox.AcceptRole
            onClicked: control.open_selected_tab()
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
            if (tab_list_view.currentIndex > 0) {
                tab_list_view.currentIndex--;
            } else {
                tab_list_view.currentIndex = tab_list_view.count - 1;
            }
        }
    }

    Shortcut {
        sequences: ["Down", "J"]
        enabled: control.visible
        onActivated: {
            if (tab_list_view.currentIndex < tab_list_view.count - 1) {
                tab_list_view.currentIndex++;
            } else {
                tab_list_view.currentIndex = 0;
            }
        }
    }

    Shortcut {
        sequences: ["Home", "G"]
        enabled: control.visible
        onActivated: tab_list_view.currentIndex = 0
    }

    Shortcut {
        sequences: ["End", "Shift+G"]
        enabled: control.visible
        onActivated: tab_list_view.currentIndex = tab_list_view.count - 1
    }

    Shortcut {
        sequence: "Return"
        enabled: control.visible
        onActivated: control.open_selected_tab()
    }

    Shortcut {
        sequence: "Enter"
        enabled: control.visible
        onActivated: control.open_selected_tab()
    }

    contentItem: ScrollView {
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
            }

            delegate: ItemDelegate {
                id: item_delegate
                required property int index
                required property string item_uid
                required property string table_name
                required property string sutta_title
                required property string sutta_ref
                required property string id_key
                required property string group_label

                width: ListView.view.width
                highlighted: ListView.isCurrentItem

                contentItem: RowLayout {
                    spacing: 8

                    Label {
                        text: item_delegate.group_label
                        font.bold: true
                        Layout.preferredWidth: 60
                        color: {
                            if (item_delegate.highlighted) return "white";
                            if (item_delegate.group_label === "Pinned") return control.palette.link;
                            if (item_delegate.group_label === "Results") return control.palette.text;
                            if (item_delegate.group_label === "Trans") return "#2e7d32";
                            return control.palette.text;
                        }
                    }

                    Label {
                        text: {
                            if (item_delegate.table_name && item_delegate.table_name === "dpd_headwords") {
                                return `${item_delegate.sutta_title}/dpd`;
                            } else {
                                return item_delegate.item_uid;
                            }
                        }
                        Layout.fillWidth: true
                        elide: Text.ElideRight
                        color: item_delegate.highlighted ? "white" : control.palette.text
                    }
                }

                onClicked: {
                    tab_list_view.currentIndex = item_delegate.index;
                }

                onDoubleClicked: {
                    tab_list_view.currentIndex = item_delegate.index;
                    control.open_selected_tab();
                }
            }

            Component.onCompleted: control.populate_model()
        }
    }

    function open_selected_tab() {
        if (tab_list_view.currentIndex >= 0) {
            let item = combined_tabs_model.get(tab_list_view.currentIndex);
            if (item) {
                control.tabSelected(item.id_key);
                control.close();
            }
        }
    }
    
    function populate_model() {
        combined_tabs_model.clear();
        
        // Add pinned tabs
        for (let i = 0; i < tabs_pinned_model.count; i++) {
            let tab_data = tabs_pinned_model.get(i);
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
    
    onAboutToShow: populate_model()
}
