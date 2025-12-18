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
    // NOTE: on desktop, 650 width threshold is when the show_sidebar_btn starts to touch the SearchBarInput search_input
    readonly property bool is_wide: is_desktop ? (root.width > 650) : (root.width > 800)
    readonly property bool is_tall: root.height > 800
    readonly property bool is_mac: Qt.platform.os == "osx"
    readonly property bool is_qml_preview: Qt.application.name === "Qml Runtime"

    readonly property int icon_size: is_tall ? 40 : 30

    // Add extra top margin on mobile to account for status bar
    // Get the actual status bar height from the system and add base margin
    property int top_bar_margin: is_mobile ? 24 : 0

    property bool is_dark: false

    property bool is_loading: false

    property bool webview_visible: root.is_desktop || (!mobile_menu.visible && !color_theme_dialog.visible && !storage_dialog.visible && !about_dialog.visible && !models_dialog.visible && !anki_export_dialog.visible && !gloss_tab.commonWordsDialog.visible && !tab_list_dialog.visible && !mobile_top_margin_dialog.visible)

    property string last_query_text: ""
    property string last_search_area: ""
    property string pending_find_query: ""

    Logger { id: logger }

    Connections {
        target: SuttaBridge

        function onUpdateWindowTitle(item_uid: string, sutta_ref: string, sutta_title: string) {
            /* logger.log("onUpdateWindowTitle():", item_uid, sutta_ref, sutta_title); */
            const current_key = sutta_html_view_layout.current_key;
            // Check if the item exists in items_map before accessing it
            if (current_key && sutta_html_view_layout.items_map[current_key] &&
                sutta_html_view_layout.items_map[current_key].get_data_value('item_uid') === item_uid) {
                root.update_window_title(item_uid, sutta_ref, sutta_title);
            }
        }

        function onResultsPageReady(results_json: string) {
            let d = JSON.parse(results_json);
            fulltext_results.set_search_result_page(d);
            root.is_loading = false;
        }

        function onShowChapterFromLibrary(window_id: string, result_data_json: string) {
            // Only handle this signal if it's for this window or if window_id is empty
            if (window_id === "" || window_id === root.window_id) {
                root.show_result_in_html_view_with_json(result_data_json);
            }
        }
    }

    function update_window_title(item_uid: string, sutta_ref: string, sutta_title: string) {
        let title_parts = [sutta_ref, sutta_title, item_uid].filter(i => i !== "");
        let title = title_parts.join(" ");
        root.setTitle(`${title} - Simsapa`);
    }

    function update_top_bar_margin() {
        root.top_bar_margin = root.is_mobile ? SuttaBridge.get_mobile_top_bar_margin() : 0;
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
            anchor:      fulltext_results_data.anchor || "",
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
        let search_area = search_bar_input.search_area_dropdown.get_text();

        // Determine if query_text_orig is a sutta/book reference
        // For Suttas and Library, convert references to uid: format
        // For Dictionary, skip conversion to allow normal word searches
        let query_text = query_text_orig;
        if (search_area === "Suttas" || search_area === "Library") {
            // query_text_to_uid_field_query() should return the query as normal (e.g. 'heard') if not recognized as a uid
            query_text = SuttaBridge.query_text_to_uid_field_query(query_text_orig);
        }

        if (query_text.startsWith('uid:')) {
            params['mode'] = 'Uid Match';
            min_length = 7; // e.g. uid:mn8, uid:bmc
        }

        if (query_text.length < min_length)
            return;

        // Not aborting, show the user that the app started processsing
        // TODO self._show_search_stopwatch_icon()

        // self.start_loading_animation()

        // self._last_query_time = datetime.now()

        // When the user continues searching, show the results panel
        // Force the checked state to update by toggling if needed
        if (!show_sidebar_btn.checked) {
            show_sidebar_btn.checked = true;
        }

        // Activate the Results tab
        rightside_tabs.setCurrentIndex(0);

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
        root.last_query_text = query_text;
        root.last_search_area = search_area;

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
        let search_area = search_bar_input.search_area_dropdown.get_text();
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

        const mode = search_bar_input.search_mode_dropdown.get_text();
        const search_area = search_bar_input.search_area_dropdown.get_text();
        let lang = search_bar_input.language_filter_dropdown.get_text();
        // Dictionary currently only uses English language from DPD.
        if (search_area === "Dictionary") {
            lang = null;
        }

        return {
            mode: mode,
            page_len: 10,
            lang: lang,
            lang_include: true,
            source: null,
            source_include: true,
            enable_regex: false,
            fuzzy_distance: 0,
        };
    }

    function set_summary_query(query_text: string) {
        word_summary_wrap.visible = true;
        word_summary.set_query(query_text);
        word_summary.search_btn.click();
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

    function show_result_in_html_view_with_json(result_data_json: string, new_tab) {
        if (new_tab === undefined) new_tab = false;
        let result_data = JSON.parse(result_data_json);
        root.show_result_in_html_view(result_data, new_tab);
    }

    function show_result_in_html_view(result_data: var, new_tab) {
        if (new_tab === undefined) new_tab = false;
        let tab_data = root.new_tab_data(result_data);
        let tab_idx = root.add_results_tab(tab_data, true, new_tab);
        // NOTE: It will not find the tab first time while window objects are still
        // constructed, but succeeds later on.
        // Focus on the tab that was just created/updated
        if (new_tab && tab_idx >= 0 && tab_idx < tabs_results_model.count) {
            // For a new tab, focus on the newly created tab
            let created_tab_data = tabs_results_model.get(tab_idx);
            root.focus_on_tab_with_id_key(created_tab_data.id_key);
        } else {
            // For updating existing tab, focus on ResultsTab_0
            root.focus_on_tab_with_id_key("ResultsTab_0");
        }

        // Update TocTab if this is a book chapter
        if (tab_data.table_name === "book_spine_items" && tab_data.item_uid) {
            toc_tab.update_for_spine_item(tab_data.item_uid);
        }

        // Only add translation tabs for sutta results, not dictionary or library results
        if (tab_data.table_name && tab_data.table_name !== "dict_words" && tab_data.table_name !== "dpd_headwords" && tab_data.table_name !== "book_spine_items") {
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

            if (action_open_find_in_sutta_results.checked &&
                root.last_search_area === "Suttas" &&
                root.last_query_text.length > 0) {
                let query_as_uid = SuttaBridge.query_text_to_uid_field_query(root.last_query_text);
                if (!query_as_uid.startsWith('uid:')) {
                    root.pending_find_query = root.last_query_text;
                }
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
                // Show the item if this is the first tab being added (focus_on_new will be true for the blank initial tab)
                sutta_html_view_layout.add_item(tab_data, tab_data.focus_on_new);
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
                    anchor: tab_data.anchor || "",
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
            SuttaBridge.dictionary_first_query();
            SuttaBridge.userdata_first_query();

            // Update top_bar_margin after app data is initialized
            // This will automatically update all child dialogs via property bindings
            root.update_top_bar_margin();

            search_as_you_type.checked = SuttaBridge.get_search_as_you_type();
            action_open_find_in_sutta_results.checked = SuttaBridge.get_open_find_in_sutta_results();
        }

        // Add the default blank tab. The corresponding webview is created when it is focused.
        //
        // When opened in narrow view, the right panel with results are shown.
        // In narrow view, don't add a blank tab, because its webview is going to cover the entire screen.
        if (tabs_results_model.count == 0 && root.is_wide) {
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

    function open_find_in_sutta_with_query(query: string) {
        let html_view = sutta_html_view_layout.get_current_item();
        if (html_view) {
            html_view.active_focus();
            let escaped_query = query.replace(/\\/g, '\\\\').replace(/`/g, '\\`').replace(/\$/g, '\\$');
            let js = `document.SSP.find.setSearchTerm(\`${escaped_query}\`);`;
            html_view.item.web.runJavaScript(js);
        }
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
                    id: action_close_window
                    text: "&Close Window"
                    shortcut: Shortcut {
                        sequences: ["Alt+F4"]
                        context: Qt.WindowShortcut
                        onActivated: action_close_window.trigger()
                    }
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
            }

            CMenuItem {
                enabled: root.is_mobile // Using 'visible' leaves an empty space on desktop
                action: Action {
                    text: "Mobile Top Margin..."
                    onTriggered: mobile_top_margin_dialog.open()
                }
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
                    onToggled: SuttaBridge.set_search_as_you_type(checked)
                }
            }

            CMenuItem {
                action: Action {
                    id: action_open_find_in_sutta_results
                    text: "Open Find in Sutta Results"
                    checkable: true
                    checked: true
                    onToggled: SuttaBridge.set_open_find_in_sutta_results(checked)
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

            CMenuItem {
                action: Action {
                    id: action_close_tab
                    text: "Close Tab"
                    shortcut: Shortcut {
                        sequences: ["Ctrl+W"]
                        context: Qt.WindowShortcut
                        onActivated: action_close_tab.trigger()
                    }
                    onTriggered: {
                        suttas_tab_bar.close_current_tab();
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

            CMenuItem {
                action: Action {
                    id: action_sutta_languages
                    text: "Sutta Languages..."
                    onTriggered: {
                        SuttaBridge.open_sutta_languages_window()
                    }
                }
            }

            CMenuItem {
                action: Action {
                    id: action_library
                    text: "Library..."
                    onTriggered: {
                        SuttaBridge.open_library_window()
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

    AboutDialog {
        id: about_dialog
        top_bar_margin: root.top_bar_margin
    }

    SystemPromptsDialog {
        id: system_prompts_dialog
        top_bar_margin: root.top_bar_margin
    }

    ModelsDialog {
        id: models_dialog
        top_bar_margin: root.top_bar_margin
    }

    AnkiExportDialog {
        id: anki_export_dialog
        top_bar_margin: root.top_bar_margin
    }

    ColorThemeDialog {
        id: color_theme_dialog
        current_theme: SuttaBridge.get_theme_name()
        onThemeChanged: function(theme_name) {
            SuttaBridge.set_theme_name(theme_name);
            root.apply_theme();
        }
    }

    MobileTopMarginDialog {
        id: mobile_top_margin_dialog
        onMarginChanged: {
            // Update the top_bar_margin property
            root.update_top_bar_margin();
        }
    }

    DatabaseValidationDialog {
        id: database_validation_dialog
        top_bar_margin: root.top_bar_margin
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.topMargin: root.top_bar_margin

        RowLayout {
            Button {
                id: show_menu
                Layout.alignment: Qt.AlignTop
                Layout.leftMargin: 10
                Layout.rightMargin: 0
                Layout.topMargin: 9
                visible: root.is_mobile
                icon.source: "icons/32x32/mdi--menu.png"
                Layout.preferredHeight: root.icon_size
                Layout.preferredWidth: root.icon_size
                ToolTip.visible: hovered
                ToolTip.text: "Show Menu"
                onClicked: mobile_menu.open()
            }

            SearchBarInput {
                id: search_bar_input
                Layout.alignment: Qt.AlignTop
                is_wide: root.is_wide
                is_tall: root.is_tall
                icon_size: root.icon_size
                db_loaded: SuttaBridge.db_loaded
                handle_query_fn: root.handle_query
                search_timer: search_timer
                search_as_you_type: search_as_you_type
                is_loading: root.is_loading
            }

            Button {
                id: show_sidebar_btn
                Layout.alignment: Qt.AlignTop
                Layout.leftMargin: 0
                Layout.rightMargin: 10
                Layout.topMargin: 9
                icon.source: "icons/32x32/bxs_book_content.png"
                Layout.preferredHeight: root.icon_size
                Layout.preferredWidth: root.icon_size
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

                        RowLayout {
                            id: suttas_tab_bar_container
                            anchors.top: parent.top
                            anchors.left: parent.left
                            anchors.right: parent.right
                            height: suttas_tab_bar.height
                            spacing: 0

                            TabBar {
                                id: suttas_tab_bar
                                Layout.fillWidth: true
                                Layout.fillHeight: true
                                Layout.alignment: Qt.AlignBottom

                                function tab_checked_changed(tab: SuttaTabButton, tab_model: ListModel) {
                                    // Called when a tab's checked state changes (user clicked on a tab)
                                    // Parameters: tab index, item_uid, web_item_key, checked state, current_key

                                    // Only proceed if the tab is checked (selected)
                                    // When switching tabs: previous tab becomes unchecked (early return here),
                                    // new tab becomes checked (continue processing)
                                    if (!tab.checked) {
                                        return;
                                    }

                                    // Prevent redundant processing if this tab's webview is already showing
                                    // This can happen if the same tab is clicked multiple times
                                    if (tab.web_item_key !== "" && sutta_html_view_layout.current_key === tab.web_item_key) {
                                        return;
                                    }

                                    // If this tab doesn't have a webview associated yet, create one
                                    // This happens on first click of a tab (web_item_key is empty string)
                                    if (tab.web_item_key == "") {
                                        // Generate unique key for this webview
                                        let key = root.generate_key();
                                        tab.web_item_key = key;

                                        // Update the key in both the model and the tab's property
                                        if (tab_model.count > tab.index) {
                                            let tab_data = tab_model.get(tab.index);
                                            tab_data.web_item_key = key;
                                            tab_model.set(tab.index, tab_data);
                                            tab.web_item_key = key;
                                            // Add the webview but don't show it yet (show_item = false)
                                            // We'll set current_key explicitly below to ensure it happens after the item is fully added
                                            sutta_html_view_layout.add_item(tab_data, false);
                                            // New webview created and added to layout
                                        } else {
                                            logger.error("Out of bounds error: tab_model.count " + tab_model.count + " tab.index " + tab.index);
                                        }
                                    }
                                    // If tab already has a web_item_key, we're switching to an existing webview

                                    // Show the tab's webview by setting current_key
                                    // This is called after add_item to ensure the item is fully created
                                    sutta_html_view_layout.current_key = tab.web_item_key;

                                    // Scroll the checked tab into view in the tab bar
                                    suttas_tab_bar.scroll_tab_into_view(tab);
                                    // Tab switch completed: webview shown, tab scrolled into view
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

                                function scroll_tab_into_view(tab: SuttaTabButton) {
                                    if (!tab) return;

                                    // Get tab position relative to the flickable
                                    let tab_x = tab.x;
                                    let tab_width = tab.width;

                                    // Calculate the visible area in the flickable
                                    let visible_left = tabs_flickable.contentX;
                                    let visible_right = visible_left + tabs_flickable.width;

                                    // Check if tab is fully visible
                                    if (tab_x < visible_left) {
                                        // Tab is to the left of visible area, scroll left
                                        tabs_flickable.contentX = tab_x;
                                    } else if (tab_x + tab_width > visible_right) {
                                        // Tab is to the right of visible area, scroll right
                                        tabs_flickable.contentX = tab_x + tab_width - tabs_flickable.width;
                                    }
                                }

                                function close_current_tab() {
                                    // Find the currently checked tab across all repeaters (pinned, results, translations)
                                    // and trigger its close action

                                    // Check pinned tabs
                                    for (let i = 0; i < tabs_pinned.count; i++) {
                                        let tab = tabs_pinned.itemAt(i);
                                        if (tab && tab.checked) {
                                            // Pinned tabs can always be closed normally
                                            tab.close_btn.clicked();
                                            return;
                                        }
                                    }

                                    // Check results tabs
                                    for (let i = 0; i < tabs_results.count; i++) {
                                        let tab = tabs_results.itemAt(i);
                                        if (tab && tab.checked) {
                                            // Special handling for ResultsTab_0
                                            if (tab.id_key === "ResultsTab_0") {
                                                if (tab.item_uid === "Sutta") {
                                                    // If ResultsTab_0 has placeholder content, close the window
                                                    root.close();
                                                } else {
                                                    // Replace with blank content, preserving id_key and web_item_key
                                                    let old_tab_data = tabs_results_model.get(0);
                                                    let blank_data = root.new_tab_data(
                                                        {item_uid: "Sutta", sutta_title: "", sutta_ref: ""},
                                                        false,
                                                        false,
                                                        old_tab_data.id_key,
                                                        old_tab_data.web_item_key
                                                    );
                                                    tabs_results_model.set(0, blank_data);
                                                }
                                            } else {
                                                // For non-ResultsTab_0 tabs, trigger the close button
                                                tab.close_btn.clicked();
                                            }
                                            return;
                                        }
                                    }

                                    // Check translations tabs
                                    for (let i = 0; i < tabs_translations.count; i++) {
                                        let tab = tabs_translations.itemAt(i);
                                        if (tab && tab.checked) {
                                            tab.close_btn.clicked();
                                            return;
                                        }
                                    }
                                }

                                contentItem: Flickable {
                                    id: tabs_flickable
                                    clip: true
                                    contentWidth: tabs_row.implicitWidth
                                    contentHeight: tabs_row.implicitHeight

                                    flickableDirection: Flickable.HorizontalFlick
                                    boundsBehavior: Flickable.StopAtBounds

                                    RowLayout {
                                        id: tabs_row
                                        spacing: 0
                                        height: parent.height

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
                                                // Handle tab selection via checked state (works on both Linux and macOS)
                                                onCheckedChanged: suttas_tab_bar.tab_checked_changed(pinned_tab_btn, tabs_pinned_model)
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
                                                    // Add a blank tab to replace the pinned one (new_tab=true to avoid overwriting existing tabs)
                                                    root.add_results_tab(root.blank_sutta_tab_data(), false, true);
                                                    // TODO: If this is before add_results_tab(), the new results tab gets focus, despite of focus_on_new = false.
                                                    root.focus_on_tab_with_id_key(new_tab_data.id_key);
                                                }
                                                onCloseClicked: {
                                                    if (tabs_results_model.count == 1) {
                                                        // If this is the only tab, don't remove it, just set it to blank
                                                        // Preserve id_key and web_item_key
                                                        let old_tab_data = tabs_results_model.get(0);
                                                        let blank_data = root.new_tab_data(
                                                            {item_uid: "Sutta", sutta_title: "", sutta_ref: ""},
                                                            false,
                                                            false,
                                                            old_tab_data.id_key,
                                                            old_tab_data.web_item_key
                                                        );
                                                        tabs_results_model.set(0, blank_data);
                                                    } else {
                                                        suttas_tab_bar.remove_tab_and_webview(results_tab_btn, tabs_results_model);
                                                    }
                                                }
                                                // Handle tab selection via checked state (works on both Linux and macOS)
                                                onCheckedChanged: suttas_tab_bar.tab_checked_changed(results_tab_btn, tabs_results_model)
                                                onItem_uidChanged: {
                                                    // Only update if the new item_uid is not a placeholder (blank tab)
                                                    if (results_tab_btn.item_uid !== "Sutta" && results_tab_btn.item_uid !== "Word" && results_tab_btn.web_item_key !== "" && sutta_html_view_layout.has_item(results_tab_btn.web_item_key)) {
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
                                                // Handle tab selection via checked state (works on both Linux and macOS)
                                                onCheckedChanged: suttas_tab_bar.tab_checked_changed(translations_tab_btn, tabs_translations_model)
                                            }
                                        }

                                        Item { Layout.fillWidth: true }
                                    }
                                }
                            }

                            Button {
                                id: tab_overflow_btn
                                icon.source: "icons/32x32/mdi--menu.png"
                                Layout.preferredWidth: 28
                                Layout.preferredHeight: 28 // 32 x 32 creates a gap under the tabs
                                flat: true
                                visible: tabs_flickable.contentWidth > tabs_flickable.width
                                onClicked: tab_list_dialog.open()

                                background: Rectangle {
                                    color: "transparent"
                                    border.color: tab_overflow_btn.palette.mid
                                    border.width: 1
                                    radius: 2
                                }
                            }
                        }

                        TabListDialog {
                            id: tab_list_dialog

                            tabs_pinned_model: tabs_pinned_model
                            tabs_results_model: tabs_results_model
                            tabs_translations_model: tabs_translations_model

                            onTabSelected: function(id_key) {
                                root.focus_on_tab_with_id_key(id_key);
                            }
                        }

                        SplitView {
                            id: sutta_split
                            orientation: Qt.Vertical

                            anchors.top: suttas_tab_bar_container.bottom
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
                                    // Also respect the parent container's visibility
                                    visible: root.webview_visible && suttas_tab_container.visible

                                    onPage_loaded: {
                                        if (root.pending_find_query.length > 0) {
                                            root.open_find_in_sutta_with_query(root.pending_find_query);
                                            root.pending_find_query = "";
                                        }
                                    }
                                }
                            }

                            Item {
                                id: word_summary_wrap
                                SplitView.preferredHeight: root.is_tall ? parent.height*0.3 : parent.height*0.5
                                SplitView.preferredWidth: parent.width
                                visible: false

                                function handle_summary_close() {
                                    word_summary_wrap.visible = false;
                                    let html_view = sutta_html_view_layout.get_current_item();
                                    if (html_view) {
                                        html_view.item.web.runJavaScript("if (typeof window.word_summary_closed === 'function') { window.word_summary_closed(); }");
                                    }
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

                            TabButton {
                                text: "TOC"
                                id: toc_tab_btn
                                icon.source: "icons/32x32/bxs_book_bookmark.png"
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
                                    root.show_result_in_html_view(result_data);
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

                            TocTab {
                                id: toc_tab
                                window_id: root.window_id
                                is_dark: root.is_dark
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
