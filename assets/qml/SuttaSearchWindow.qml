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

    property bool webview_visible: root.is_desktop || (!mobile_menu.visible && !color_theme_dialog.visible && !storage_dialog.visible && !about_dialog.visible && !models_dialog.visible && !anki_export_dialog.visible && !gloss_tab.commonWordsDialog.visible)

    Logger { id: logger }

    Connections {
        target: SuttaBridge

        function onUpdateWindowTitle(item_uid: string, sutta_ref: string, sutta_title: string) {
            /* logger.log("onUpdateWindowTitle():", item_uid, sutta_ref, sutta_title); */
            const current_key = sutta_html_view_layout.current_key;
            if (sutta_html_view_layout.items_map[current_key].get_data_value('item_uid') === item_uid) {
                root.update_window_title(item_uid, sutta_ref, sutta_title);
            }
        }

        function onResultsPageReady(results_json: string) {
            let d = JSON.parse(results_json);
            fulltext_results.set_search_result_page(d);
            root.is_loading = false;
        }
    }

    function update_window_title(item_uid: string, sutta_ref: string, sutta_title: string) {
        let title_parts = [sutta_ref, sutta_title, item_uid].filter(i => i !== "");
        let title = title_parts.join(" ");
        root.setTitle(`${title} - Simsapa`);
    }

    function apply_theme() {
        root.is_dark = SuttaBridge.get_theme_name() === "dark";
        var theme_json = SuttaBridge.get_saved_theme();
        /* logger.log("Theme JSON:\n---\n", theme_json, "\n---\n"); */
        if (theme_json.length === 0 || theme_json === "{}") {
            logger.error("Couldn't get theme JSON.")
            return;
        }

        try {
            var d = JSON.parse(theme_json);

            for (var color_group_key in d) {
                /* logger.log(color_group_key); // active, inactive, disabled */
                if (!root.palette.hasOwnProperty(color_group_key) || root.palette[color_group_key] === undefined) {
                    logger.error("Member not found on root.palette:", color_group_key);
                    continue;
                }
                var color_group = d[color_group_key];
                for (var color_role_key in color_group) {
                    /* logger.log(color_role_key); // window, windowText, etc. */
                    /* logger.log(color_group[color_role_key]); // #EFEFEF, #000000, etc. */
                    if (!root.palette[color_group_key].hasOwnProperty(color_role_key) || root.palette[color_group_key][color_role_key] === undefined) {
                        logger.error("Member not found on root.palette:", color_group_key, color_role_key);
                        continue;
                    }
                    try {
                        root.palette[color_group_key][color_role_key] = color_group[color_role_key];
                    } catch (e) {
                        logger.error("Could not set palette property:", color_group_key, color_role_key, e);
                    }
                }
            }
        } catch (e) {
            logger.error("Failed to parse theme JSON:", e);
        }
    }

    ListModel { id: tabs_pinned_model }
    ListModel { id: tabs_results_model }
    ListModel { id: tabs_translations_model }

    function new_tab_data(fulltext_results_data: var, pinned = false, focus_on_new = false, id_key = null, web_item_key = ""): var {
        /* logger.log("new_tab_data()", fulltext_results_data, pinned, focus_on_new); */
        if (!id_key) {
            id_key = root.generate_key();
        }
        // Generate the tabs with empty web_item_key. An item_key and associated webview
        // will be created when the tab is first focused.
        //
        // NOTE: same attributes as on TabButton.
        /* logger.log("item_uid", fulltext_results_data.item_uid); */
        /* logger.log("sutta_title", fulltext_results_data.sutta_title); */
        let tab_data = {
            item_uid:    fulltext_results_data.item_uid || "",
            table_name:  fulltext_results_data.table_name || "",
            sutta_title: fulltext_results_data.sutta_title || "",
            sutta_ref:   fulltext_results_data.sutta_ref || "",
            pinned: pinned,
            focus_on_new: focus_on_new,
            id_key: id_key,
            web_item_key: web_item_key,
        };
        return tab_data;
    }

    function blank_sutta_tab_data(): var {
        return root.new_tab_data({item_uid: "Sutta", sutta_title: "", sutta_ref: ""});
    }

    // Timer for incremental search debounce
    Timer {
        id: search_timer
        interval: 400 // milliseconds
        repeat: false
        onTriggered: {
            if (search_as_you_type.checked) {
                root.handle_query(search_bar_input.search_input.text, 4);
            }
        }
    }

    function handle_query(query_text_orig: string, min_length=4) {
        if (query_text_orig === 'uid:')
            return;

        let params = root.get_search_params_from_ui();
        let search_area = search_bar_input.search_area_dropdown.currentText;

        let query_text = SuttaBridge.query_text_to_uid_field_query(query_text_orig);

        if (query_text.startsWith('uid:')) {
            params['mode'] = 'UidMatch';
            min_length = 7; // e.g. uid:mn8
        }

        if (query_text.length < min_length)
            return;

        // Not aborting, show the user that the app started processsing
        // TODO self._show_search_stopwatch_icon()

        // self.start_loading_animation()

        // self._last_query_time = datetime.now()

        // self._queries.start_search_query_workers()
        root.start_search_query_workers(
            query_text,
            search_area,
            /* self._last_query_time, */
            /* partial(self._search_query_finished), */
            params,
        )
    }

    function start_search_query_workers(
        query_text: string,
        search_area: string,
        params: var,
    ) {
        // FIXME: page number
        root.results_page(query_text, 0, search_area, params);

        // if len(results) > 0 and hits == 1 and results[0]['uid'] is not None:
        //     self._show_sutta_by_uid(results[0]['uid'])

        // elif self.query_in_tab:
        //     self._render_results_in_active_tab(hits)
    }

    function results_page(query_text: string, page_num: int, search_area: string, params: var) {
        root.is_loading = true;
        let params_json = JSON.stringify(params);
        SuttaBridge.results_page(query_text, page_num, search_area, params_json);
    }

    function new_results_page(page_num) {
        let query = search_bar_input.search_input.text;
        let search_area = search_bar_input.search_area_dropdown.currentText;
        let params = root.get_search_params_from_ui();
        root.results_page(query, page_num, search_area, params);
    }

    function get_search_params_from_ui(): var {
        // Extract params from the state of UI such as SearchBarInput and SearchBarOptions.

        // class SearchParams(TypedDict):
        //     mode: SearchMode
        //     page_len: Optional[int]
        //     lang: Optional[str]
        //     lang_include: bool
        //     source: Optional[str]
        //     source_include: bool
        //     enable_regex: bool
        //     fuzzy_distance: int

        return {
            mode: search_bar_options.search_mode_dropdown.currentText,
            page_len: 10,
            lang: null,
            lang_include: false,
            source: null,
            source_include: false,
            enable_regex: false,
            fuzzy_distance: 0,
        };
    }

    function set_summary_query(query_text: string) {
        word_summary_wrap.visible = true;
        word_summary.set_query(query_text);
    }

    function gloss_text(query_text: string) {
        show_sidebar_btn.checked = true;
        rightside_tabs.setCurrentIndex(2); // gloss tab
        gloss_tab.gloss_text_input.text = query_text;
        gloss_tab.start_background_all_glosses();
    }

    function new_prompt(prompt: string) {
        show_sidebar_btn.checked = true;
        rightside_tabs.setCurrentIndex(3); // prompts tab
        prompts_tab.new_prompt(prompt);
    }

    function run_sutta_menu_action(action: string, query_text: string) {
        /* logger.log("run_sutta_menu_action():", action, query_text.slice(0, 30)); */

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

        case "summarize-sutta":
            var prompt = `Summarize the following sutta text:

${query_text}`;
            root.new_prompt(prompt);
            break;

        case "translate-selection":
            var prompt = `Translate the following passage:

${query_text}`;
            root.new_prompt(prompt);
            break;

        case "analyse-selection":
            var prompt = `Analyse the following passage and provide a word-by-word breakdown as a list:

${query_text}`;
            root.new_prompt(prompt);
            break;

        case "copy-link-sutta":
        case "copy-sutta-url":
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
    function add_results_tab(fulltext_results_data: var, focus_on_new = true, new_tab = false): int {
        /* logger.log("add_results_tab()", "item_uid", fulltext_results_data.item_uid, "sutta_title", fulltext_results_data.sutta_title); */
        if (new_tab || tabs_results_model.count == 0) {
            /* logger.log("Adding a new results tab", "tabs_results_model.count", tabs_results_model.count); */
            let tab_data = root.new_tab_data(fulltext_results_data, false, focus_on_new);
            if (tabs_results_model.count == 0) {
                tab_data.id_key = "ResultsTab_0";
            }
            if (tab_data.web_item_key == "") {
                tab_data.web_item_key = root.generate_key();
                sutta_html_view_layout.add_item(tab_data, false);
            }
            tabs_results_model.append(tab_data);
            return tabs_results_model.count-1;
        } else {
            /* logger.log("Updating existing results tab"); */
            // Not creating a new tab, update the existing one at idx 0.
            let tab_data = root.new_tab_data(
                fulltext_results_data,
                false,
                focus_on_new,
                tabs_results_model.get(0).id_key,
                tabs_results_model.get(0).web_item_key);

            tabs_results_model.set(0, tab_data);

            // Update the existing webview component with new properties
            let comp = sutta_html_view_layout.get_item(tab_data.web_item_key);
            if (comp) {
                // Update all properties
                let data = {
                    item_uid: tab_data.item_uid,
                    table_name: tab_data.table_name,
                    sutta_ref: tab_data.sutta_ref,
                    sutta_title: tab_data.sutta_title,
                };
                comp.data_json = JSON.stringify(data);
            }

            if (tab_data.item_uid !== "Sutta" && tab_data.item_uid !== "Word") {
                SuttaBridge.emit_update_window_title(tab_data.item_uid, tab_data.sutta_ref, tab_data.sutta_title);
            }

            return 0;
        }
    }

    function focus_on_tab_with_id_key(id_key: string) {
        /* logger.log("focus_on_tab_with_id_key()", id_key); */
        let tab = root.get_tab_with_id_key(id_key);
        if (tab) {
            tab.click();
        } else {
            logger.error("Error: Tab not found with id_key: " + id_key);
        }
    }

    Component.onCompleted: {
        /* logger.log("SuttaSearchWindow: Component.onCompleted()"); */
        if (root.is_qml_preview) {
            return;
        } else {
            root.apply_theme();
            SuttaBridge.load_db();
            SuttaBridge.appdata_first_query();
            SuttaBridge.dpd_first_query();
        }

        // Add the default blank tab. The corresponding webview is created when it is focused.
        if (tabs_results_model.count == 0) {
            /* logger.log("tabs_results_model.count", tabs_results_model.count); */
            root.add_results_tab(root.blank_sutta_tab_data());
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
        show_sidebar_btn.checked = true;
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
                    id: search_as_you_type
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

            CMenuItem {
                action: Action {
                    id: action_find_in_page
                    text: "Find in Page..."
                    shortcut: Shortcut {
                        sequences: ["Ctrl+F"]
                        context: Qt.WindowShortcut
                        onActivated: action_find_in_page.trigger()
                    }
                    onTriggered: {
                        let html_view = sutta_html_view_layout.get_current_item();
                        html_view.show_find_bar();
                    }
                }
            }

            CMenuItem {
                action: Action {
                    id: action_find_next_in_page
                    text: "Find Next in Page..."
                    shortcut: Shortcut {
                        sequences: ["Ctrl+N"]
                        context: Qt.WindowShortcut
                        onActivated: action_find_next_in_page.trigger()
                    }
                    onTriggered: {
                        let html_view = sutta_html_view_layout.get_current_item();
                        html_view.find_next();
                    }
                }
            }

            CMenuItem {
                action: Action {
                    id: action_find_previous_in_page
                    text: "Find Previous in Page..."
                    shortcut: Shortcut {
                        sequences: ["Ctrl+P"]
                        context: Qt.WindowShortcut
                        onActivated: action_find_previous_in_page.trigger()
                    }
                    onTriggered: {
                        let html_view = sutta_html_view_layout.get_current_item();
                        html_view.find_previous();
                    }
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
                    onTriggered: {
                        SuttaBridge.open_sutta_search_window()
                    }
                }
            }

            // CMenuItem {
            //     action: Action {
            //         id: action_sutta_study
            //         text: "&Sutta Study"
            //         icon.source: "icons/32x32/bxs_book_bookmark.png"
            //         shortcut: Shortcut {
            //             sequences: ["Ctrl+F5"]
            //             context: Qt.WindowShortcut
            //             onActivated: action_sutta_study.trigger()
            //         }
            //         /* onTriggered: TODO */
            //     }
            // }

            // CMenuItem {
            //     action: Action {
            //         id: action_dictionary_search
            //         text: "&Dictionary Search"
            //         icon.source: "icons/32x32/bxs_book_content.png"
            //         shortcut: Shortcut {
            //             sequences: ["F6"]
            //             context: Qt.WindowShortcut
            //             onActivated: action_dictionary_search.trigger()
            //         }
            //         /* onTriggered: TODO */
            //     }
            // }

            // CMenuItem {
            //     action: Action {
            //         id: action_show_word_lookup
            //         text: "Show Word Lookup"
            //         checkable: true
            //         checked: false
            //         shortcut: Shortcut {
            //             sequences: ["Ctrl+F6"]
            //             context: Qt.WindowShortcut
            //             onActivated: action_show_word_lookup.trigger()
            //         }
            //         /* onTriggered: TODO */
            //     }
            // }
        }

        Menu {
            id: gloss_menu
            title: "&Gloss"

            CMenuItem {
                action: Action {
                    id: action_common_words
                    text: "&Common Words..."
                    onTriggered: gloss_tab.commonWordsDialog.open()
                }
            }

            CMenuItem {
                action: Action {
                    id: action_anki_export
                    text: "&Anki Export..."
                    onTriggered: anki_export_dialog.show()
                }
            }
        }

        Menu {
            id: prompts_menu
            title: "&Prompts"

            CMenuItem {
                action: Action {
                    id: action_models
                    text: "AI &Models..."
                    onTriggered: models_dialog.show()
                }
            }

            CMenuItem {
                action: Action {
                    id: action_system_prompts
                    text: "&System Prompts..."
                    onTriggered: system_prompts_dialog.show()
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
        menu_list: [file_menu, edit_menu, view_menu, find_menu, windows_menu, gloss_menu, prompts_menu, help_menu]
    }

    AboutDialog { id: about_dialog }

    SystemPromptsDialog { id: system_prompts_dialog }
    ModelsDialog { id: models_dialog }
    AnkiExportDialog { id: anki_export_dialog }

    ColorThemeDialog {
        id: color_theme_dialog
        current_theme: SuttaBridge.get_theme_name()
        onThemeChanged: function(theme_name) {
            SuttaBridge.set_theme_name(theme_name);
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
                db_loaded: SuttaBridge.db_loaded
                handle_query_fn: root.handle_query
                search_timer: search_timer
                search_as_you_type: search_as_you_type
                is_loading: root.is_loading
            }

            // FIXME combine SearchBarOptions with SearchBarInput
            SearchBarOptions {
                id: search_bar_options
                search_area_text: search_bar_input.search_area_dropdown.currentText
                visible: (root.width - 550) > 550 // FIXME: make search bar optionally visible in a second row on small screens
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
                            // NOTE: 20 : 40 or 15 : 30 is too wide, interferes with dragging the scroll bar
                            width: root.is_desktop ? 10 : 20
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
                            anchors.left: parent.left
                            anchors.right: parent.right

                            function tab_focus_changed(tab: SuttaTabButton, tab_model: ListModel) {
                                /* logger.log("tab_focus_changed()", tab.index, "item_uid:", tab.item_uid, "web_item_key:", tab.web_item_key); */
                                if (!tab.focus) return;
                                // If this tab doesn't have a webview associated yet, create it.
                                if (tab.web_item_key == "") {
                                    let key = root.generate_key();
                                    tab.web_item_key = key;

                                    // Update the key in both the model and the tab's property
                                    if (tab_model.count > tab.index) {
                                        let tab_data = tab_model.get(tab.index);
                                        tab_data.web_item_key = key;
                                        tab_model.set(tab.index, tab_data);
                                        tab.web_item_key = key;
                                        sutta_html_view_layout.add_item(tab_data);
                                    } else {
                                        logger.error("Out of bounds error:", "tab_model.count", tab_model.count, "tab_model.get(tab.index)", tab.index);
                                    }
                                }
                                // show the sutta tab
                                sutta_html_view_layout.current_key = tab.web_item_key;
                            }

                            function remove_tab_and_webview(tab: SuttaTabButton, tab_model: ListModel) {
                                /* logger.log("remove_tab_and_webview()", tab.index, tab.item_uid, tab.web_item_key); */
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
                                            let old_tab_data = tabs_pinned_model.get(pinned_tab_btn.index);
                                            let new_tab_data = root.new_tab_data(old_tab_data, false, true, root.generate_key(), old_tab_data.web_item_key);
                                            tabs_results_model.append(new_tab_data);
                                            tabs_pinned_model.remove(pinned_tab_btn.index)
                                            root.focus_on_tab_with_id_key(new_tab_data.id_key);
                                        }
                                        onCloseClicked: suttas_tab_bar.remove_tab_and_webview(pinned_tab_btn, tabs_pinned_model)
                                        onFocusChanged: suttas_tab_bar.tab_focus_changed(pinned_tab_btn, tabs_pinned_model)
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
                                            let old_tab_data = tabs_results_model.get(results_tab_btn.index);
                                            // New pinned tab will get focus.
                                            let new_tab_data = root.new_tab_data(old_tab_data, true, true, root.generate_key(), old_tab_data.web_item_key);
                                            tabs_pinned_model.append(new_tab_data);
                                            // Remove the tab data, but webview remains associated with the pinned item.
                                            tabs_results_model.remove(results_tab_btn.index);
                                            root.add_results_tab(root.blank_sutta_tab_data(), false);
                                            // TODO: If this is before add_results_tab(), the new results tab gets focus, despite of focus_on_new = false.
                                            root.focus_on_tab_with_id_key(new_tab_data.id_key);
                                        }
                                        onCloseClicked: {
                                            if (tabs_results_model.count == 1) {
                                                // If this is the only tab, don't remove it, just set it to blank
                                                results_tab_btn.item_uid = "Sutta";
                                                tabs_results_model.set(0, root.blank_sutta_tab_data());
                                            } else {
                                                suttas_tab_bar.remove_tab_and_webview(results_tab_btn, tabs_results_model);
                                            }
                                        }
                                        onFocusChanged: suttas_tab_bar.tab_focus_changed(results_tab_btn, tabs_results_model)
                                        onItem_uidChanged: {
                                            if (results_tab_btn.web_item_key !== "" && sutta_html_view_layout.has_item(results_tab_btn.web_item_key)) {
                                                let i = sutta_html_view_layout.get_item(results_tab_btn.web_item_key);
                                                i.set_data_value('item_uid', results_tab_btn.item_uid);
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
                                            let old_tab_data = tabs_translations_model.get(translations_tab_btn.index);
                                            let new_tab_data = root.new_tab_data(old_tab_data, true, true, root.generate_key(), old_tab_data.web_item_key);
                                            tabs_pinned_model.append(new_tab_data);
                                            tabs_translations_model.remove(translations_tab_btn.index);
                                            root.focus_on_tab_with_id_key(new_tab_data.id_key);
                                        }
                                        onCloseClicked: suttas_tab_bar.remove_tab_and_webview(translations_tab_btn, tabs_translations_model)
                                        onFocusChanged: suttas_tab_bar.tab_focus_changed(translations_tab_btn, tabs_translations_model)
                                    }
                                }

                                Item { Layout.fillWidth: true }
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
                                    height: root.is_desktop ? 15 : 30
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
                                    search_as_you_type_checked: search_as_you_type.checked
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
                                icon.source: "icons/32x32/material-symbols--list-alt-outline.png"
                                padding: 5
                            }

                            TabButton {
                                text: "Prompts"
                                id: prompts_tab_btn
                                icon.source: "icons/32x32/grommet-icons--chat.png"
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
                                    /* logger.log("update_item()"); */
                                    let result_data = fulltext_results.current_result_data();
                                    // E.g. in the case when fulltext_list.currentIndex is set to -1 such as when update_page() shows a new page of results.
                                    if (!result_data) {
                                        return;
                                    }
                                    let tab_data = root.new_tab_data(result_data);
                                    let tab_idx = root.add_results_tab(tab_data, true);
                                    // NOTE: It will not find the tab first time while window objects are still
                                    // constructed, but succeeds later on.
                                    root.focus_on_tab_with_id_key("ResultsTab_0");

                                    // Only add translation tabs for sutta results, not dictionary results
                                    if (tab_data.table_name && tab_data.table_name !== "dict_words" && tab_data.table_name !== "dpd_headwords") {
                                        // Add translations tabs for the sutta
                                        // Remove existing webviews for translation tabs
                                        for (let i=0; i < tabs_translations_model.count; i++) {
                                            let tr_tab_data = tabs_translations_model.get(i);
                                            if (tr_tab_data.web_item_key !== "") {
                                                sutta_html_view_layout.delete_item(tr_tab_data.web_item_key);
                                            }
                                        }
                                        tabs_translations_model.clear();

                                        let translations_data = JSON.parse(SuttaBridge.get_translations_data_json_for_sutta_uid(tab_data.item_uid));

                                        for (let i=0; i < translations_data.length; i++) {
                                            let tr_tab_data = root.new_tab_data(translations_data[i], false, false);
                                            tabs_translations_model.append(tr_tab_data);
                                        }
                                    } else {
                                        // For dictionary results, clear translation tabs
                                        for (let i=0; i < tabs_translations_model.count; i++) {
                                            let tr_tab_data = tabs_translations_model.get(i);
                                            if (tr_tab_data.web_item_key !== "") {
                                                sutta_html_view_layout.delete_item(tr_tab_data.web_item_key);
                                            }
                                        }
                                        tabs_translations_model.clear();
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
                                Layout.fillWidth: rightside_tabs.currentIndex === 1
                                Layout.fillHeight: rightside_tabs.currentIndex === 1
                                Layout.preferredWidth: rightside_tabs.currentIndex === 1 ? parent.width : 0
                                Layout.preferredHeight: rightside_tabs.currentIndex === 1 ? parent.height : 0
                                visible: root.webview_visible && rightside_tabs.currentIndex === 1
                            }

                            GlossTab {
                                id: gloss_tab
                                window_id: root.window_id
                                is_dark: root.is_dark
                                ai_models_auto_retry: models_dialog.auto_retry.checked
                                Layout.fillWidth: true
                                Layout.fillHeight: true
                                handle_open_dict_tab_fn: root.open_dict_tab
                            }

                            Connections {
                                target: gloss_tab
                                function onRequestWordSummary(word) {
                                    root.set_summary_query(word);
                                    word_summary.search_btn.click();
                                }
                            }

                            PromptsTab {
                                id: prompts_tab
                                window_id: root.window_id
                                is_dark: root.is_dark
                                ai_models_auto_retry: models_dialog.auto_retry.checked
                                Layout.fillWidth: true
                                Layout.fillHeight: true
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
