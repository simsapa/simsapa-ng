import QtQuick
import QtQuick.Layouts
import QtQuick.Controls

import com.profoundlabs.simsapa

TabButton {
    id: control

    required property int index

    // NOTE: same attributes as returned from new_tab_data().
    required property string item_uid
    required property string table_name
    required property string sutta_title
    required property string sutta_ref
    required property bool pinned
    required property bool focus_on_new
    required property string id_key
    required property string web_item_key

    signal pinToggled(bool pinned)
    signal closeClicked()

    Logger { id: logger }

    // Alias to allow triggering close from outside (e.g., keyboard shortcut)
    property alias close_btn: close_btn

    /* implicitWidth: Math.min(200, Math.max(150, implicitContentWidth + 30)) */

    contentItem: RowLayout {
        Button {
            id: pin_btn
            checkable: true
            checked: control.pinned
            icon.source: checked ? "icons/32x32/material-symbols-light--push-pin.png" : "icons/32x32/material-symbols-light--push-pin-outline.png"
            Layout.preferredWidth: 24
            flat: true
            onCheckedChanged: control.pinToggled(checked)
        }

        Label {
            text: {
                if (control.table_name && control.table_name === "dpd_headwords") {
                    // "25671/dpd" -> "cakka 1/dpd"
                    return `${control.sutta_title}/dpd`;
                } else {
                    return control.item_uid;
                }
            }
            elide: Text.ElideRight
            /* Layout.fillWidth: true */
        }

        Button {
            id: close_btn
            icon.source: "icons/32x32/mdi--close.png"
            Layout.preferredWidth: 24
            flat: true
            visible: control.id_key != "ResultsTab_0"
            onClicked: control.closeClicked()
        }
    }

    onClicked: {
        logger.log("SuttaTabButton clicked - index:", control.index, "item_uid:", control.item_uid, "checked:", control.checked);
    }

    onCheckedChanged: {
        logger.log("SuttaTabButton checked changed to:", control.checked, "index:", control.index, "item_uid:", control.item_uid);
    }

    onFocusChanged: {
        logger.log("SuttaTabButton focus changed to:", control.focus, "index:", control.index, "item_uid:", control.item_uid, "web_item_key:", control.web_item_key);
    }

    Component.onCompleted: {
        logger.log("SuttaTabButton completed - index:", control.index, "item_uid:", control.item_uid, "focus_on_new:", control.focus_on_new);
        if (control.focus_on_new) {
            logger.log("  -> Clicking tab due to focus_on_new");
            control.click();
        }
    }
}
