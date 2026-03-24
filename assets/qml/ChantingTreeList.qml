pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls

ColumnLayout {
    id: root
    required property var collections_list
    required property int pointSize

    property string selected_uid: ""
    property string selected_type: "" // "collection", "chant", or "section"

    anchors.fill: parent
    spacing: 5

    signal section_clicked(string section_uid)
    signal selection_changed(string uid, string item_type)

    Repeater {
        model: root.collections_list

        delegate: ColumnLayout {
            id: collection_item
            Layout.fillWidth: true
            Layout.margins: 2
            spacing: 0

            required property var modelData
            property bool is_selected: root.selected_uid === modelData.uid && root.selected_type === "collection"
            property bool is_expanded: false

            // Collection header
            Frame {
                Layout.fillWidth: true

                background: Rectangle {
                    color: collection_item.is_selected ? palette.highlight : palette.base
                    border.color: palette.shadow
                    border.width: 1
                    radius: 4
                }

                contentItem: MouseArea {
                    implicitWidth: collection_row.implicitWidth
                    implicitHeight: collection_row.implicitHeight
                    cursorShape: Qt.PointingHandCursor

                    onClicked: {
                        root.selected_uid = collection_item.modelData.uid;
                        root.selected_type = "collection";
                        root.selection_changed(collection_item.modelData.uid, "collection");
                        collection_item.is_expanded = !collection_item.is_expanded;
                    }

                    RowLayout {
                        id: collection_row
                        anchors.fill: parent
                        spacing: 8

                        Label {
                            text: collection_item.is_expanded ? "▼" : "▶"
                            font.pointSize: root.pointSize - 2
                            color: palette.text
                        }

                        Rectangle {
                            Layout.preferredWidth: 12
                            Layout.preferredHeight: 12
                            radius: 2
                            color: "#4A90E2"
                        }

                        Label {
                            text: collection_item.modelData.title || "Untitled"
                            font.pointSize: root.pointSize
                            font.bold: true
                            color: collection_item.is_selected ? palette.highlightedText : palette.text
                            wrapMode: Text.WordWrap
                            Layout.fillWidth: true
                        }

                        Label {
                            visible: collection_item.modelData.chants && collection_item.modelData.chants.length > 0
                            text: "(" + (collection_item.modelData.chants ? collection_item.modelData.chants.length : 0) + ")"
                            font.pointSize: root.pointSize - 2
                            color: collection_item.is_selected ? palette.highlightedText : palette.mid
                        }
                    }
                }
            }

            // Chants list
            ColumnLayout {
                visible: collection_item.is_expanded
                Layout.fillWidth: true
                Layout.leftMargin: 24
                Layout.topMargin: 2
                spacing: 2

                Repeater {
                    model: collection_item.modelData.chants || []

                    delegate: ColumnLayout {
                        id: chant_item
                        Layout.fillWidth: true
                        spacing: 0

                        required property var modelData
                        property bool is_selected: root.selected_uid === modelData.uid && root.selected_type === "chant"
                        property bool is_expanded: false

                        // Chant header
                        Frame {
                            Layout.fillWidth: true

                            background: Rectangle {
                                color: chant_item.is_selected ? palette.highlight : palette.alternateBase
                                border.color: palette.shadow
                                border.width: 1
                                radius: 3
                            }

                            contentItem: MouseArea {
                                implicitWidth: chant_row.implicitWidth
                                implicitHeight: chant_row.implicitHeight
                                cursorShape: Qt.PointingHandCursor

                                onClicked: {
                                    root.selected_uid = chant_item.modelData.uid;
                                    root.selected_type = "chant";
                                    root.selection_changed(chant_item.modelData.uid, "chant");
                                    chant_item.is_expanded = !chant_item.is_expanded;
                                }

                                RowLayout {
                                    id: chant_row
                                    anchors.fill: parent
                                    spacing: 8

                                    Label {
                                        text: chant_item.is_expanded ? "▼" : "▶"
                                        font.pointSize: root.pointSize - 3
                                        color: palette.text
                                    }

                                    Rectangle {
                                        Layout.preferredWidth: 10
                                        Layout.preferredHeight: 10
                                        radius: 2
                                        color: "#7B68EE"
                                    }

                                    Label {
                                        text: chant_item.modelData.title || "Untitled"
                                        font.pointSize: root.pointSize - 1
                                        font.bold: true
                                        color: chant_item.is_selected ? palette.highlightedText : palette.text
                                        wrapMode: Text.WordWrap
                                        Layout.fillWidth: true
                                    }

                                    Label {
                                        visible: chant_item.modelData.sections && chant_item.modelData.sections.length > 0
                                        text: "(" + (chant_item.modelData.sections ? chant_item.modelData.sections.length : 0) + ")"
                                        font.pointSize: root.pointSize - 3
                                        color: chant_item.is_selected ? palette.highlightedText : palette.mid
                                    }
                                }
                            }
                        }

                        // Sections list
                        ColumnLayout {
                            visible: chant_item.is_expanded
                            Layout.fillWidth: true
                            Layout.leftMargin: 24
                            Layout.topMargin: 2
                            spacing: 1

                            Repeater {
                                model: chant_item.modelData.sections || []

                                delegate: Frame {
                                    id: section_item
                                    Layout.fillWidth: true

                                    required property var modelData
                                    property bool is_selected: root.selected_uid === modelData.uid && root.selected_type === "section"

                                    background: Rectangle {
                                        color: section_item.is_selected ? palette.highlight : "transparent"
                                        border.color: section_item.is_selected ? palette.shadow : "transparent"
                                        border.width: 1
                                        radius: 3
                                    }

                                    contentItem: MouseArea {
                                        implicitWidth: section_row.implicitWidth
                                        implicitHeight: section_row.implicitHeight
                                        cursorShape: Qt.PointingHandCursor

                                        onClicked: {
                                            root.selected_uid = section_item.modelData.uid;
                                            root.selected_type = "section";
                                            root.selection_changed(section_item.modelData.uid, "section");
                                        }

                                        onDoubleClicked: {
                                            root.section_clicked(section_item.modelData.uid);
                                        }

                                        RowLayout {
                                            id: section_row
                                            anchors.fill: parent
                                            spacing: 8

                                            Rectangle {
                                                Layout.preferredWidth: 8
                                                Layout.preferredHeight: 8
                                                radius: 4
                                                color: "#50C878"
                                            }

                                            Label {
                                                text: section_item.modelData.title || "Untitled"
                                                font.pointSize: root.pointSize - 2
                                                color: section_item.is_selected ? palette.highlightedText : palette.text
                                                wrapMode: Text.WordWrap
                                                Layout.fillWidth: true
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
