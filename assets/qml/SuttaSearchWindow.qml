pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import QtQuick.Window

import com.profoundlabs.simsapa

ApplicationWindow {
    id: root
    title: "Sutta Search - Simsapa"
    width: is_mobile ? Screen.desktopAvailableWidth : 1300
    height: is_mobile ? Screen.desktopAvailableHeight : 900
    visible: true
    color: palette.window

    // TODO: implment find_bar
    // onCurrent_web_viewChanged: {
    //     find_bar.reset();
    // }

    property string window_id

    readonly property bool is_mobile: Qt.platform.os === "android" || Qt.platform.os === "ios"
    /* readonly property bool is_mobile: true // for qml preview */
    readonly property bool is_desktop: !root.is_mobile
    // Make sure is_wide is not triggered on iPad portrait mode
    readonly property bool is_wide: is_desktop ? (root.width > 600) : (root.width > 800)
    readonly property bool is_tall: root.height > 800
    readonly property bool is_mac: Qt.platform.os == "osx"
    readonly property bool is_qml_preview: Qt.application.name === "Qml Runtime"

    property bool is_dark: false

    property bool is_loading: false

    property bool webview_visible: root.is_desktop || (!mobile_menu.activeFocus && !color_theme_dialog.visible && !storage_dialog.visible)

    SuttaBridge {
        id: sb
        Component.onCompleted: {
            if (root.is_qml_preview) return;
            root.apply_theme();
            sb.load_db();
            sb.appdata_first_query();
            sb.dpd_first_query();
        }
    }

    function apply_theme() {
        root.is_dark = sb.get_theme_name() === "dark";
        var theme_json = sb.get_saved_theme();
        /* console.log("Theme JSON:\n---\n", theme_json, "\n---\n"); */
        if (theme_json.length === 0 || theme_json === "{}") {
            console.error("Couldn't get theme JSON.")
            return;
        }

        try {
            var d = JSON.parse(theme_json);

            for (var color_group_key in d) {
                /* console.log(color_group_key); // active, inactive, disabled */
                if (!root.palette.hasOwnProperty(color_group_key) || root.palette[color_group_key] === undefined) {
                    console.error("Member not found on root.palette:", color_group_key);
                    continue;
                }
                var color_group = d[color_group_key];
                for (var color_role_key in color_group) {
                    /* console.log(color_role_key); // window, windowText, etc. */
                    /* console.log(color_group[color_role_key]); // #EFEFEF, #000000, etc. */
                    if (!root.palette[color_group_key].hasOwnProperty(color_role_key) || root.palette[color_group_key][color_role_key] === undefined) {
                        console.error("Member not found on root.palette:", color_group_key, color_role_key);
                        continue;
                    }
                    try {
                        root.palette[color_group_key][color_role_key] = color_group[color_role_key];
                    } catch (e) {
                        console.error("Could not set palette property:", color_group_key, color_role_key, e);
                    }
                }
            }
        } catch (e) {
            console.error("Failed to parse theme JSON:", e);
        }
    }

    ListModel { id: tabs_pinned_model }
    ListModel { id: tabs_results_model }
    ListModel { id: tabs_translations_model }

    function new_tab_data(title, pinned, focus_on_new, id_key = null, web_item_key = "") {
        if (!id_key) {
            id_key = root.generate_key();
        }
        // Generate the tabs with empty web_item_key. An item_key and associated webview
        // will be created when the tab is first focused.
        return { title: title, pinned: pinned, focus_on_new: focus_on_new, id_key: id_key, web_item_key: web_item_key}
    }

    // Timer for incremental search debounce
    Timer {
        id: debounce_timer
        interval: 400 // milliseconds
        repeat: false
        onTriggered: {
            if (incremental_search.checked && search_bar_input.search_input.text.length >= 4) {
                root.run_new_query(search_bar_input.search_input.text);
            }
        }
    }

    function run_new_query(query) {
        root.results_page(query, 0);
    }

    function results_page(query, page_num) {
        root.is_loading = true;
        Qt.callLater(function() {
            let json_res = sb.results_page(query, page_num);
            let d = JSON.parse(json_res);
            fulltext_results.set_search_result_page(d);
            root.is_loading = false;
        });
    }

    function new_results_page(page_num) {
        let query = search_bar_input.search_input.text;
        root.results_page(query, page_num);
    }

    function set_summary_query(query_text: string) {
        word_summary_wrap.visible = true;
        word_summary.set_query(query_text);
    }

    function gloss_text(query_text: string) {
        show_sidebar_btn.checked = true;
        tab_stack.currentIndex = 2; // gloss tab
        gloss_tab.gloss_text_input_area.text = query_text;
        gloss_tab.update_all_glosses();
    }

    function run_sutta_menu_action(action: string, query_text: string) {
        console.log("run_sutta_menu_action():", action, query_text.slice(0, 30));

        switch (action) {
        case "copy-selection":
            clip.copy_text(query_text);
            sutta_html_view_layout.show_transient_message(`Copied: ${query_text.slice(0, 30)} ...`);
            break;

        case "lookup-selection":
            sutta_html_view_layout.show_transient_message(`Lookup: ${query_text.slice(0, 30)} ...`);
            root.set_summary_query(query_text);
            break;

        case "gloss-selection":
            sutta_html_view_layout.show_transient_message(`Gloss: ${query_text.slice(0, 30)} ...`);
            root.gloss_text(query_text);
            break;

        case "copy-link-sutta":
        case "copy-sutta-url":
        case "summarize-sutta":
        case "analyse-selection":
            let msg = `TODO: ${action}`;
            sutta_html_view_layout.show_transient_message(msg);
            break;
        }
    }

    property int key_counter: 0

    function generate_key(): string {
        root.key_counter++;
        return `key_${root.key_counter}`;
    }

    // Returns the index of the tab in the model.
    function add_results_tab(sutta_uid: string, focus_on_new = true, new_tab = false): int {
        if (new_tab || tabs_results_model.count == 0) {
            let data = root.new_tab_data(sutta_uid, false, focus_on_new);
            if (tabs_results_model.count == 0) {
                data.id_key = "ResultsTab_0";
            }
            if (data.web_item_key == "") {
                data.web_item_key = root.generate_key();
                sutta_html_view_layout.add_item(data.web_item_key, data.title, false);
            }
            tabs_results_model.append(data);
            return tabs_results_model.count-1;
        } else {
            // Not creating a new tab, update the existing one at idx 0.
            tabs_results_model.setProperty(0, "title", sutta_uid);
            let tab = root.get_tab_with_id_key(tabs_results_model.get(0).id_key);
            tab.title = sutta_uid;
            return 0;
        }
    }

    function focus_on_tab_with_id_key(id_key: string) {
        let tab = root.get_tab_with_id_key(id_key);
        if (tab) {
            tab.click();
        } else {
            console.log("Error: Tab not found with id_key: " + id_key);
        }
    }

    Component.onCompleted: {
        // Add the default blank tab. The corresponding webview is created when it is focused.
        if (tabs_results_model.count == 0) {
            root.add_results_tab("Sutta");
        }

        if (root.is_qml_preview) {
            root.qml_preview_state();
        }
    }

    function qml_preview_state() {
        gloss_tab_btn.click();
    }

    function set_query(text: string) {
        search_bar_input.search_input.text = text;
    }

    function get_tab_with_web_item_key(web_item_key) {
        var tab = null;
        for (var i=0; i < tabs_row.children.length; i++) {
            if (tabs_row.children[i].web_item_key !== undefined && tabs_row.children[i].web_item_key == web_item_key) {
                tab = tabs_row.children[i];
                break;
            }
        }
        return tab;
    }

    function get_tab_with_id_key(id_key) {
        var tab = null;
        for (var i=0; i < tabs_row.children.length; i++) {
            if (tabs_row.children[i].id_key !== undefined && tabs_row.children[i].id_key == id_key) {
                tab = tabs_row.children[i];
                break;
            }
        }
        return tab;
    }

    function open_dict_tab(uid: string) {
        rightside_tabs.setCurrentIndex(1) // idx 1 = Dictionary
        dictionary_tab.word_uid = uid;
    }

    StorageDialog { id: storage_dialog }

    menuBar: MenuBar {
        visible: root.is_desktop
        // NOTE: A Menu > CMenuItem should always have an Action. This property
        // is expected when constructing the mobile_menu Drawer.
        Menu {
            id: file_menu
            title: "&File"

            CMenuItem {
                action: Action {
                    text: "&Close Window"
                    onTriggered: root.close()
                }
            }

            // CMenuItem {
            //     action: Action {
            //         text: "Select Storage..."
            //         onTriggered: storage_dialog.open()
            //     }
            // }

            CMenuItem {
                action: Action {
                    text: "&Quit Simsapa"
                    icon.source: "icons/32x32/fa_times-circle.png"
                    id: action_quit
                    shortcut: Shortcut {
                        sequences: ["Ctrl+Q"]
                        context: Qt.WindowShortcut
                        onActivated: action_quit.trigger()
                    }
                    onTriggered: Qt.quit()
                }
            }
        }

        Menu {
            id: edit_menu
            title: "&Edit"

            CMenuItem {
                action: Action {
                    id: action_focus_search
                    text: "Focus Search Input"
                    shortcut: Shortcut {
                        sequences: ["Ctrl+L"]
                        context: Qt.WindowShortcut
                        onActivated: action_focus_search.trigger()
                    }
                    onTriggered: {
                        search_bar_input.search_input.forceActiveFocus();
                        search_bar_input.search_input.selectAll();
                    }
                }
            }
        }

        Menu {
            id: view_menu
            title: "&View"

            CMenuItem {
                action: Action {
                    text: "Color Theme..."
                    onTriggered: color_theme_dialog.open()
                }

                // action: Action {
                //     id: web_reload
                //     text: "Reload Page"
                //     // TODO: implement reload
                //     // onTriggered: {
                //     //     if (root.current_web_view)
                //     //         root.current_web_view.reload();
                //     // }
                // }
            }
        }

        Menu {
            id: find_menu
            title: "&Find"

            CMenuItem {
                action: Action {
                    id: incremental_search
                    text: "Search As You Type"
                    checkable: true
                    checked: true
                }
            }

            CMenuItem {
                action: Action {
                    id: select_previous_result
                    text: "Previous Result"
                    shortcut: Shortcut {
                        sequences: ["Ctrl+Up", "Ctrl+K"]
                        context: Qt.WindowShortcut
                        onActivated: select_previous_result.trigger()
                    }
                    onTriggered: fulltext_results.select_previous_result()
                }
            }

            CMenuItem {
                action: Action {
                    id: select_next_result
                    text: "Next Result"
                    shortcut: Shortcut {
                        sequences: ["Ctrl+Down", "Ctrl+J"]
                        context: Qt.WindowShortcut
                        onActivated: select_next_result.trigger()
                    }
                    onTriggered: fulltext_results.select_next_result()
                }
            }
        }

        Menu {
            id: windows_menu
            title: "&Windows"

            CMenuItem {
                action: Action {
                    id: action_sutta_search
                    text: "&Sutta Search"
                    icon.source: "icons/32x32/bxs_book_bookmark.png"
                    shortcut: Shortcut {
                        sequences: ["F5"]
                        context: Qt.WindowShortcut
                        onActivated: action_sutta_search.trigger()
                    }
                    /* onTriggered: TODO */
                }
            }

            CMenuItem {
                action: Action {
                    id: action_sutta_study
                    text: "&Sutta Study"
                    icon.source: "icons/32x32/bxs_book_bookmark.png"
                    shortcut: Shortcut {
                        sequences: ["Ctrl+F5"]
                        context: Qt.WindowShortcut
                        onActivated: action_sutta_study.trigger()
                    }
                    /* onTriggered: TODO */
                }
            }

            CMenuItem {
                action: Action {
                    id: action_dictionary_search
                    text: "&Dictionary Search"
                    icon.source: "icons/32x32/bxs_book_content.png"
                    shortcut: Shortcut {
                        sequences: ["F6"]
                        context: Qt.WindowShortcut
                        onActivated: action_dictionary_search.trigger()
                    }
                    /* onTriggered: TODO */
                }
            }

            CMenuItem {
                action: Action {
                    id: action_show_word_lookup
                    text: "Show Word Lookup"
                    checkable: true
                    checked: false
                    shortcut: Shortcut {
                        sequences: ["Ctrl+F6"]
                        context: Qt.WindowShortcut
                        onActivated: action_show_word_lookup.trigger()
                    }
                    /* onTriggered: TODO */
                }
            }
        }

        Menu {
            id: help_menu
            title: "&Help"
            CMenuItem {
                action: Action {
                    text: "&About"
                    onTriggered: about_dialog.show()
                }
            }
        }
    }

    DrawerMenu {
        id: mobile_menu
        window_width: root.width
        window_height: root.height
        menu_list: [file_menu, edit_menu, view_menu, find_menu, windows_menu, help_menu]
    }

    AboutDialog { id: about_dialog }

    ColorThemeDialog {
        id: color_theme_dialog
        current_theme: sb.get_theme_name()
        onThemeChanged: function(theme_name) {
            sb.set_theme_name(theme_name);
            root.apply_theme();
        }
    }

    ColumnLayout {
        anchors.fill: parent

        RowLayout {
            Button {
                id: show_menu
                visible: root.is_mobile
                icon.source: "icons/32x32/mdi--menu.png"
                Layout.preferredHeight: 40
                Layout.preferredWidth: 40
                ToolTip.visible: hovered
                ToolTip.text: "Show Menu"
                onClicked: mobile_menu.open()
            }

            SearchBarInput {
                id: search_bar_input
                is_wide: root.is_wide
                db_loaded: sb.db_loaded
                run_new_query_fn: root.run_new_query
                debounce_timer: debounce_timer
                incremental_search: incremental_search
            }

            SearchBarOptions {
                id: search_bar_options
                visible: (root.width - 550) > 550
            }

            Button {
                id: show_sidebar_btn
                icon.source: "icons/32x32/bxs_book_content.png"
                Layout.preferredHeight: 40
                Layout.preferredWidth: 40
                checkable: true
                checked: true
                ToolTip.visible: hovered
                ToolTip.text: "Show Sidebar"
            }

        }

        // Main horizontal layout
        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true

            // Left side layout
            ColumnLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true

                SplitView {
                    id: panel_split
                    orientation: Qt.Horizontal
                    Layout.fillWidth: true
                    Layout.fillHeight: true

                    handle: Rectangle {
                        id: panel_split_handle
                        implicitWidth: root.is_desktop ? 2 : 4
                        implicitHeight: panel_split.height
                        color: SplitHandle.pressed ? panel_split.palette.dark : (SplitHandle.hovered ? panel_split.palette.midlight : panel_split.palette.mid)
                        containmentMask: Item {
                            x: (panel_split_handle.width - width) / 2
                            width: root.is_desktop ? 20 : 40
                            height: panel_split_handle.height
                        }
                    }

                    Item {
                        id: suttas_tab_container
                        SplitView.preferredWidth: show_sidebar_btn.checked ? (root.is_wide ? (parent.width * 0.5) : 0) : parent.width
                        visible: show_sidebar_btn.checked ? (root.is_wide ? true : false) : true
                        /* Layout.alignment: Qt.AlignTop */

                        TabBar {
                            id: suttas_tab_bar
                            anchors.top: parent.top
                            /* anchors.left: parent.left */
                            /* anchors.right: parent.right */

                            function tab_focus_changed(tab: SuttaTabButton, tab_model: ListModel) {
                                if (!tab.focus) return;
                                // If this tab doesn't have a webview associated yet, create it.
                                if (tab.web_item_key == "") {
                                    let key = root.generate_key();

                                    // Update the key in both the model and the tab's property
                                    let data = tab_model.get(tab.index);
                                    data.web_item_key = key;
                                    tab_model.set(tab.index, data);

                                    tab.web_item_key = key;

                                    sutta_html_view_layout.add_item(tab.web_item_key, tab.title);
                                }
                                sutta_html_view_layout.current_key = tab.web_item_key;
                            }

                            function remove_tab_and_webview(tab: SuttaTabButton, tab_model: ListModel) {
                                // Remove the tab and webview, focus the next or the previous
                                let old_idx = tab.index;
                                let old_web_item_key = tab_model.get(old_idx).web_item_key;

                                var focus_tab_data = null;

                                if (tab_model.count == 1) {
                                    // If this is the last item in the model, focus back on the 0 idx item of results
                                    focus_tab_data = tabs_results_model.get(0);
                                } else if (tab.activeFocusOnTab) {
                                    // FIXME: This check doesn't work. The tab gains focus because of the click on the close button?

                                    // If tab being removed has focus, move on to the next tab, or the previous.
                                    // If the tab is not focused, the user is closing another (unfocused) tab, and we don't need to manipulate tab focus.
                                    let focus_idx;
                                    if (tab_model.count-1 > old_idx) {
                                        // If there is a next one
                                        focus_idx = old_idx+1;
                                    } else {
                                        focus_idx = old_idx-1;
                                    }
                                    focus_tab_data = tab_model.get(focus_idx);
                                }

                                // Focus on the other tab, change the html view, delete this webview
                                if (focus_tab_data) {
                                    root.focus_on_tab_with_id_key(focus_tab_data.id_key);
                                    // Show the other tab's webview
                                    sutta_html_view_layout.current_key = focus_tab_data.web_item_key;
                                }

                                // If the tab has never been focused, its web_item_key is "" and there is no associated webview.
                                if (old_web_item_key !== "") {
                                    // Remove the webview of this tab
                                    sutta_html_view_layout.delete_item(old_web_item_key);
                                }

                                // Remove this tab item
                                tab_model.remove(tab.index);
                            }

                            contentItem: RowLayout {
                                id: tabs_row
                                spacing: 0

                                Repeater {
                                    id: tabs_pinned
                                    model: tabs_pinned_model
                                    delegate: SuttaTabButton {
                                        id: pinned_tab_btn
                                        onPinToggled: function (pinned) {
                                            // NOTE: Don't convert this to a method function, it causes a binding loop on the 'checked' property.
                                            if (pinned) return;
                                            // Unpin and move back to results group
                                            let data = tabs_pinned_model.get(pinned_tab_btn.index);
                                            data.pinned = false;
                                            tabs_results_model.append(data);
                                            root.focus_on_tab_with_id_key(data.id_key);
                                            tabs_pinned_model.remove(pinned_tab_btn.index)
                                        }
                                        onCloseClicked: suttas_tab_bar.remove_tab_and_webview(pinned_tab_btn, tabs_pinned_model)
                                        onFocusChanged: suttas_tab_bar.tab_focus_changed(pinned_tab_btn, tabs_results_model)
                                    }
                                }

                                Item { Layout.preferredWidth: 5 }

                                Repeater {
                                    id: tabs_results
                                    model: tabs_results_model
                                    delegate: SuttaTabButton {
                                        id: results_tab_btn
                                        onPinToggled: function (pinned) {
                                            // NOTE: Don't convert this to a method function, it causes a binding loop on the 'checked' property.
                                            if (!pinned) return;
                                            // Pin and move to pinned group
                                            let d = tabs_results_model.get(results_tab_btn.index);
                                            // New pinned tab will get focus.
                                            let data = root.new_tab_data(d.title, true, true, root.generate_key(), d.web_item_key);
                                            tabs_pinned_model.append(data);
                                            // Remove the tab data, but webview remains associated with the pinned item.
                                            tabs_results_model.remove(results_tab_btn.index);
                                            root.add_results_tab("Sutta", false);
                                        }
                                        onCloseClicked: {
                                            if (tabs_results_model.count == 1) {
                                                // If this is the only tab, don't remove it, just set it to blank
                                                results_tab_btn.title = "Sutta";
                                                tabs_results_model.setProperty(0, "title", "Sutta");
                                            } else {
                                                suttas_tab_bar.remove_tab_and_webview(results_tab_btn, tabs_results_model);
                                            }
                                        }
                                        onFocusChanged: suttas_tab_bar.tab_focus_changed(results_tab_btn, tabs_results_model)
                                        onTitleChanged: {
                                            if (results_tab_btn.web_item_key !== "" && sutta_html_view_layout.has_item(results_tab_btn.web_item_key)) {
                                                let i = sutta_html_view_layout.get_item(results_tab_btn.web_item_key);
                                                i.sutta_uid = results_tab_btn.title;
                                                // The title changes when an item in FulltextResults is selected,
                                                // so focus on this tab.
                                                results_tab_btn.click();
                                            }
                                        }
                                    }
                                }

                                Item { Layout.preferredWidth: 5 }

                                Repeater {
                                    id: tabs_translations
                                    model: tabs_translations_model
                                    delegate: SuttaTabButton {
                                        id: translations_tab_btn
                                        onPinToggled: function (pinned) {
                                            // NOTE: Don't convert this to a method function, it causes a binding loop on the 'checked' property.
                                            if (!pinned) return;
                                            // Pin and move to pinned group
                                            let data = tabs_translations_model.get(translations_tab_btn.index);
                                            data.pinned = true;
                                            tabs_pinned_model.append(data);
                                            root.focus_on_tab_with_id_key(data.id_key);
                                            tabs_translations_model.remove(translations_tab_btn.index);
                                        }
                                        onCloseClicked: suttas_tab_bar.remove_tab_and_webview(translations_tab_btn, tabs_translations_model)
                                        onFocusChanged: suttas_tab_bar.tab_focus_changed(translations_tab_btn, tabs_translations_model)
                                    }
                                }
                            }
                        }

                        SplitView {
                            id: sutta_split
                            orientation: Qt.Vertical

                            anchors.top: suttas_tab_bar.bottom
                            anchors.bottom: suttas_tab_container.bottom
                            anchors.left: suttas_tab_container.left
                            anchors.right: suttas_tab_container.right

                            handle: Rectangle {
                                id: sutta_split_handle
                                implicitHeight: root.is_desktop ? 2 : 4
                                implicitWidth: sutta_split.width
                                color: SplitHandle.pressed ? sutta_split.palette.dark : (SplitHandle.hovered ? sutta_split.palette.midlight : sutta_split.palette.mid)
                                containmentMask: Item {
                                    y: (sutta_split_handle.height - height) / 2
                                    height: root.is_desktop ? 20 : 40
                                    width: sutta_split_handle.width
                                }
                            }

                            Item {
                                SplitView.preferredHeight: root.is_tall ? parent.height*0.7 : parent.height*0.5
                                SplitView.preferredWidth: parent.width

                                SuttaStackLayout {
                                    id: sutta_html_view_layout
                                    anchors.fill: parent
                                    window_id: root.window_id
                                    is_dark: root.is_dark
                                    // Hide the webview when the drawer menu or a dialog is open. The mobile webview
                                    // is always on top, obscuring other items.
                                    visible: root.webview_visible
                                }
                            }

                            Item {
                                id: word_summary_wrap
                                SplitView.preferredHeight: root.is_tall ? parent.height*0.3 : parent.height*0.5
                                SplitView.preferredWidth: parent.width
                                visible: false

                                function handle_summary_close() {
                                    word_summary_wrap.visible = false;
                                }

                                WordSummary {
                                    id: word_summary
                                    anchors.fill: parent
                                    is_dark: root.is_dark
                                    window_height: root.height
                                    handle_summary_close_fn: word_summary_wrap.handle_summary_close
                                    handle_open_dict_tab_fn: root.open_dict_tab
                                    incremental_search_checked: incremental_search.checked
                                }
                            }
                        }
                    }

                    Item {
                        SplitView.preferredWidth: show_sidebar_btn.checked ? (root.is_wide ? (parent.width * 0.5) : parent.width) : 0
                        visible: show_sidebar_btn.checked

                        // Right side tabs
                        TabBar {
                            id: rightside_tabs
                            anchors.top: parent.top
                            anchors.left: parent.left
                            anchors.right: parent.right

                            TabButton {
                                text: "Results"
                                id: fulltext_results_tab_btn
                                icon.source: "icons/32x32/bx_search_alt_2.png"
                                padding: 5
                            }

                            TabButton {
                                text: "Dictionary"
                                id: dictionary_tab_btn
                                icon.source: "icons/32x32/bxs_book_content.png"
                                padding: 5
                            }

                            TabButton {
                                text: "Gloss"
                                id: gloss_tab_btn
                                icon.source: "icons/32x32/fa_clock-rotate-left-solid.png"
                                padding: 5
                            }

                            // TabButton {
                            //     text: "History"
                            //     id: history_tab_btn
                            //     icon.source: "icons/32x32/fa_clock-rotate-left-solid.png"
                            //     padding: 5
                            // }
                        }

                        // Tab content areas
                        StackLayout {
                            id: tab_stack
                            currentIndex: rightside_tabs.currentIndex
                            anchors.top: rightside_tabs.bottom
                            anchors.topMargin: 5
                            width: parent.width
                            anchors.bottom: parent.bottom

                            FulltextResults {
                                id: fulltext_results
                                is_loading: root.is_loading
                                is_dark: root.is_dark
                                new_results_page_fn: root.new_results_page

                                function update_item() {
                                    let uid = fulltext_results.current_uid();
                                    let tab_idx = root.add_results_tab(uid, true);
                                    // NOTE: It will not find the tab first time while window objects are still
                                    // constructed, but succeeds later on.
                                    root.focus_on_tab_with_id_key("ResultsTab_0");

                                    // Add translations tabs

                                    // Remove existing webviews for translation tabs
                                    for (let i=0; i < tabs_translations_model.count; i++) {
                                        let data = tabs_translations_model.get(i);
                                        if (data.web_item_key !== "") {
                                            sutta_html_view_layout.delete_item(data.web_item_key);
                                        }
                                    }
                                    tabs_translations_model.clear();

                                    let translations_uids = sb.get_translations_for_sutta_uid(uid);

                                    for (let i=0; i < translations_uids.length; i++) {
                                        let uid = translations_uids[i];
                                        let data = root.new_tab_data(uid, false, false);
                                        tabs_translations_model.append(data);
                                    }

                                    if (!root.is_wide) {
                                        show_sidebar_btn.checked = false;
                                    }
                                }

                                onCurrentIndexChanged: fulltext_results.update_item()
                            }

                            DictionaryTab {
                                id: dictionary_tab
                                window_id: root.window_id
                                is_dark: root.is_dark
                                word_uid: ""
                                Layout.fillWidth: true
                                Layout.fillHeight: true
                                visible: root.webview_visible
                            }

                            GlossTab {
                                id: gloss_tab
                                window_id: root.window_id
                                is_dark: root.is_dark
                                Layout.fillWidth: true
                                Layout.fillHeight: true
                                handle_open_dict_tab_fn: root.open_dict_tab
                            }

                            // History Tab
                            // ColumnLayout {
                            //     id: recent_tab
                            //     ListView { id: recent_list }
                            // }
                        }
                    }
                }
            }
        }

        // Invisible helper for clipboard
        TextEdit {
            id: clip
            visible: false
            function copy_text(text) {
                clip.text = text;
                clip.selectAll();
                clip.copy();
            }
        }
    }
}
