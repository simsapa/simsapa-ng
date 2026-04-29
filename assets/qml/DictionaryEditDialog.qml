pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls

import com.profoundlabs.simsapa

Dialog {
    id: root

    property int dictionary_id: 0
    property string original_label: ""
    property int point_size: 12

    signal renamed(int dictionary_id, string new_label)
    signal failed(string message)

    title: "Edit Dictionary"
    modal: true
    standardButtons: Dialog.Cancel | Dialog.Ok
    width: 480
    anchors.centerIn: parent

    DictionaryManager { id: dict_manager }

    property string label_status: "available"

    function refresh_status() {
        const v = label_input.text;
        if (v === root.original_label || v.length === 0) {
            root.label_status = "available";
        } else {
            root.label_status = dict_manager.label_status(v);
        }
    }

    onOpened: {
        label_input.text = root.original_label;
        root.refresh_status();
        label_input.forceActiveFocus();
    }

    contentItem: ColumnLayout {
        spacing: 10

        Label {
            text: "Label:"
            font.pointSize: root.point_size
        }

        TextField {
            id: label_input
            Layout.fillWidth: true
            font.pointSize: root.point_size
            onTextChanged: root.refresh_status()
        }

        Label {
            visible: root.label_status === "invalid"
            text: "Label must be ASCII alphanumeric, '_' or '-' only."
            color: "red"
            font.pointSize: root.point_size - 1
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
        }

        Label {
            visible: root.label_status === "taken_shipped"
            text: "This name is reserved by a built-in dictionary."
            color: "red"
            font.pointSize: root.point_size - 1
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
        }

        Label {
            visible: root.label_status === "taken_user"
            text: "Another imported dictionary already uses this label."
            color: "red"
            font.pointSize: root.point_size - 1
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: warn_label.implicitHeight + 12
            color: "#fff8d8"
            border.color: "#d4b94d"
            radius: 3

            Label {
                id: warn_label
                anchors.fill: parent
                anchors.margins: 6
                text: "Renaming takes effect after the next app restart, when the affected entries are re-indexed in FTS5 and Tantivy. This may take some time for large dictionaries."
                wrapMode: Text.WordWrap
                font.pointSize: root.point_size - 1
                color: "#000000"
            }
        }
    }

    onAccepted: {
        const v = label_input.text;
        if (v === root.original_label) {
            return;
        }
        if (root.label_status !== "available") {
            return;
        }
        const result = dict_manager.rename_label(root.dictionary_id, v);
        if (result === "ok") {
            root.renamed(root.dictionary_id, v);
        } else {
            root.failed(result);
        }
    }
}
