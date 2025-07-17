pragma ComponentBehavior: Bound

import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15

import com.profoundlabs.simsapa

Dialog {
    id: root
    title: "Select Storage Location"
    modal: true

    anchors.centerIn: parent
    width: 500
    /* height: 400 */

    property alias storageManager: sm
    property int selectedIndex: -1

    readonly property int font_point_size: 12
    readonly property TextMetrics tm1: TextMetrics { text: "#"; font.pointSize: root.font_point_size }

    StorageManager { id: sm }
    ListModel { id: storage_locations_model }

    Component.onCompleted: {
        var s = sm.get_storage_locations_json();
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
            text: "Choose a storage location:"
            font.pointSize: root.font_point_size
            Layout.fillWidth: true
        }

        ScrollView {
            Layout.fillWidth: true
            Layout.fillHeight: true

            ListView {
                id: storageListView
                model: storage_locations_model
                delegate: storage_list_delegate
                clip: true

                readonly property int item_height: root.tm1.height + 10
                Layout.preferredHeight: item_height * storage_locations_model.count
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

                    Frame {
                        anchors.fill: parent

                        background: Rectangle {
                            anchors.fill: parent
                            radius: 5
                            border.width: 1
                            border.color: storageRadioButton.checked ? "#1976d2" : "#ddd"
                            color: storageRadioButton.checked ? "#e3f2fd" : "transparent"
                        }

                        MouseArea {
                            anchors.fill: parent
                            onClicked: {
                                root.selectedIndex = list_item.index;
                            }
                        }

                        RowLayout {
                            id: storageItem
                            anchors.verticalCenter: parent.verticalCenter
                            anchors.fill: parent
                            spacing: 4

                            RadioButton {
                                id: storageRadioButton
                                checked: root.selectedIndex === list_item.index
                                onClicked: {
                                    root.selectedIndex = list_item.index;
                                }
                            }

                            Label {
                                text: list_item.label
                                font.pointSize: root.font_point_size
                                font.bold: true
                            }

                            Label {
                                text: root.megabytes_to_gb(list_item.megabytes_available) + " of " + root.megabytes_to_gb(list_item.megabytes_total) + " GB free"
                                font.pointSize: root.font_point_size - 2
                            }

                            Label {
                                id: internalLabel
                                visible: list_item.is_internal
                                text: "(Internal)"
                                font.pointSize: root.font_point_size - 2
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
                highlighted: true

                onClicked: {
                    if (root.selectedIndex >= 0) {
                        var idx = root.selectedIndex;
                        sm.save_storage_path(storage_locations_model.get(idx).path,
                                             storage_locations_model.get(idx).is_internal);
                        root.accept()
                    }
                }
            }
        }
    }
}
