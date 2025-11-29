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
    standardButtons: Dialog.Close
    
    x: (parent.width - width) / 2
    y: (parent.height - height) / 2
    width: Math.min(parent.width * 0.6, 500)
    height: Math.min(parent.height * 0.7, 400)

    contentItem: ScrollView {
        ListView {
            id: tab_list_view
            clip: true
            
            model: ListModel {
                id: combined_tabs_model
            }
            
            delegate: ItemDelegate {
                id: item_delegate
                required property string item_uid
                required property string table_name
                required property string sutta_title
                required property string sutta_ref
                required property string id_key
                required property string group_label
                
                width: ListView.view.width
                
                contentItem: RowLayout {
                    spacing: 8
                    
                    Label {
                        text: item_delegate.group_label
                        font.bold: true
                        Layout.preferredWidth: 60
                        color: {
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
                    }
                }
                
                onClicked: {
                    control.tabSelected(id_key);
                    control.close();
                }
            }
            
            Component.onCompleted: control.populate_model()
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
