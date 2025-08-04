pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

import com.profoundlabs.simsapa

Dialog {
    id: root
    title: "Select App Data Storage Location"
    modal: true

    anchors.centerIn: parent
    width: 500

    property alias storageManager: sm
    property int selectedIndex: -1

    readonly property int font_point_size: 12
    readonly property bool is_qml_preview: Qt.application.name === "Qml Runtime"

    StorageManager { id: sm }
    ListModel { id: storage_locations_model }

    Component.onCompleted: {
        if (root.is_qml_preview) return;
        var s = sm.get_app_data_storage_paths_json();
        var d = JSON.parse(s);
        for (var i = 0; i < d.length; i++) {
            var item = d[i];
            var data = {
                path: item.path,
                label: item.label,
                is_internal: item.is_internal,
                megabytes_total: item.megabytes_total,
                megabytes_available: item.megabytes_available,
            };

            if (item.is_internal) {
                storage_locations_model.insert(0, data);
                root.selectedIndex = 0;
            } else {
                storage_locations_model.append(data);
            }
        }
    }

    function megabytes_to_gb(megabytes: int): string {
        var gb = megabytes / 1024;
        return gb.toFixed(1);
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 10

        Label {
            text: "Choose a storage location for the app database and other assets:"
            wrapMode: Text.WordWrap
            font.pointSize: root.font_point_size
            Layout.fillWidth: true
        }

        ListView {
            id: storageListView
            model: storage_locations_model
            delegate: storage_list_delegate
            clip: true

            Layout.fillWidth: true
            Layout.fillHeight: true

            Layout.preferredHeight: (item_height+20) * storage_locations_model.count
            readonly property int item_height: 40
        }

        Component {
            id: storage_list_delegate
            ItemDelegate {
                id: list_item

                width: parent ? parent.width : 0
                height: storageListView.item_height

                required property int index
                required property string path
                required property string label
                required property bool is_internal
                required property int megabytes_total
                required property int megabytes_available

                Rectangle {
                    anchors.fill: parent
                    height: storageListView.item_height

                    radius: 5
                    border.width: 1
                    border.color: storageRadioButton.checked ? "#1976d2" : "#ddd"
                    color: storageRadioButton.checked ? "#e3f2fd" : "transparent"

                    MouseArea {
                        anchors.fill: parent
                        onClicked: {
                            root.selectedIndex = list_item.index;
                        }
                    }

                    Column {
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.verticalCenter: parent.verticalCenter
                        anchors.margins: 10

                        Row {
                            id: storageItem
                            anchors.fill: parent
                            spacing: 4

                            RadioButton {
                                id: storageRadioButton
                                checked: root.selectedIndex === list_item.index
                                onClicked: {
                                    root.selectedIndex = list_item.index;
                                }
                                anchors.verticalCenter: parent.verticalCenter
                            }

                            Text {
                                text: list_item.label
                                font.pointSize: root.font_point_size
                                font.bold: true
                                anchors.verticalCenter: parent.verticalCenter
                            }

                            Text {
                                text: root.megabytes_to_gb(list_item.megabytes_available) + " of " + root.megabytes_to_gb(list_item.megabytes_total) + " GB free"
                                font.pointSize: root.font_point_size - 2
                                anchors.verticalCenter: parent.verticalCenter
                            }

                            Text {
                                id: internalLabel
                                visible: list_item.is_internal
                                text: "(Internal)"
                                font.pointSize: root.font_point_size - 2
                                anchors.verticalCenter: parent.verticalCenter
                            }
                        }
                    }
                }
            }
        }

        RowLayout {
            Button {
                text: "Select"
                enabled: root.selectedIndex >= 0

                onClicked: {
                    if (root.selectedIndex >= 0) {
                        var idx = root.selectedIndex;
                        sm.save_storage_path(storage_locations_model.get(idx).path,
                                             storage_locations_model.get(idx).is_internal);
                        root.accept()
                    }
                }
            }

            Item { Layout.fillWidth: true }

            Button {
                text: "Cancel"
                highlighted: true
                onClicked: root.close()
            }
        }
    }
}
