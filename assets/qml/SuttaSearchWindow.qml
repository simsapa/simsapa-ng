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

    onClosing: function(close) {
        if (root.is_mobile) {
            close.accepted = false;
            show_sidebar_btn.checked = false;
            tab_list_dialog.open();
        }
        // Desktop: close.accepted defaults to true, normal close behavior
    }

    property string window_id

    readonly property bool is_mobile: Qt.platform.os === "android" || Qt.platform.os === "ios"
    /* readonly property bool is_mobile: true // for qml preview */
    readonly property bool is_desktop: !root.is_mobile

    // QML uses device-independent pixels (dp), not physical pixels.
    // logical pixels = physical pixels / devicePixelRatio
    //
    // Qt uses devicePixelRatio (DPR) to scale between physical and logical pixels.
    // On most modern Android phones, the DPR is typically 2.75, 3.0, or 3.5.
    //
    // Example of two screens, with a DPR of 3.0:
    // 
    // Screen 1 — is_tall is false (height <= 800):
    // Physical: 1080 x 2340
    // 2340 / 3.0 = 780 logical px → 780 < 800 → false
    // 
    // Screen 2 — is_tall is true (height > 800):
    // Physical: 1080 x 2408
    // 2408 / 3.0 = 802.6 logical px → 802 > 800 → true

    // Make sure is_wide is not triggered on iPad portrait mode
    // NOTE: on desktop, 650 width threshold is when the show_sidebar_btn starts to touch the SearchBarInput search_input
    readonly property bool is_wide: is_desktop ? (root.width > 650) : (root.width > 800)
    readonly property bool is_tall: root.height > 810
    readonly property bool is_mac: Qt.platform.os == "osx"
    readonly property bool is_qml_preview: Qt.application.name === "Qml Runtime"

    readonly property int icon_size: is_tall ? 40 : 30

    // Add extra top margin on mobile to account for status bar
    // Get the actual status bar height from the system and add base margin
    property int top_bar_margin: is_mobile ? 24 : 0

    property bool is_dark: false
    property bool is_reading_mode: false

    property bool is_loading: false
    property bool has_query_error: false

    property bool webview_visible: root.is_desktop || (!mobile_menu.visible && !about_dialog.visible && !models_dialog.visible && !anki_export_dialog.visible && !gloss_tab.commonWordsDialog.visible && !tab_list_dialog.visible && !database_validation_dialog.visible && !app_settings_window.visible)

    property string last_query_text: ""
    property string last_search_area: ""
    property var last_params: null
    property string pending_find_query: ""
    property real pending_bookmark_scroll: 0.0

    // === Navigation history stack (mobile back button) ===
    property var nav_history: []
    property bool nav_history_paused: false
    property bool is_restoring_session: false
    function nav_history_push(entry) {
        if (root.nav_history_paused) return;
        // Don't record blank/placeholder tabs in history
        if (!entry.item_uid || entry.item_uid === "Sutta" || entry.item_uid === "Word") return;
        // Skip consecutive duplicates
        if (root.nav_history.length > 0 && root.nav_history[root.nav_history.length - 1].item_uid === entry.item_uid) return;
        root.nav_history.push(entry);
    }

    function nav_history_current() {
        if (root.nav_history.length === 0) return null;
        return root.nav_history[root.nav_history.length - 1];
    }

    function get_current_scroll_position(callback) {
        let html_view = sutta_html_view_layout.get_current_item();
        if (html_view && html_view.item && html_view.item.web) {
            html_view.item.web.runJavaScript("window.scrollY", function(result) {
                callback(result || 0);
            });
        } else {
            callback(0);
        }
    }

    function get_tab_model_name_for_id_key(id_key) {
        for (let i = 0; i < tabs_pinned_model.count; i++) {
            if (tabs_pinned_model.get(i).id_key === id_key) return "pinned";
        }
        for (let i = 0; i < tabs_results_model.count; i++) {
            if (tabs_results_model.get(i).id_key === id_key) return "results";
        }
        for (let i = 0; i < tabs_translations_model.count; i++) {
            if (tabs_translations_model.get(i).id_key === id_key) return "translations";
        }
        return "results";
    }

    function build_nav_entry(type, id_key, tab_data, scroll_position) {
        return {
            type: type,
            tab_model: root.get_tab_model_name_for_id_key(id_key),
            id_key: id_key,
            scroll_position: scroll_position || 0,
            item_uid: tab_data.item_uid || "",
            table_name: tab_data.table_name || "",
            sutta_ref: tab_data.sutta_ref || "",
            sutta_title: tab_data.sutta_title || "",
        };
    }

    function get_model_by_name(model_name) {
        if (model_name === "pinned") return tabs_pinned_model;
        if (model_name === "translations") return tabs_translations_model;
        return tabs_results_model;
    }

    function restore_scroll_position(scroll_pos) {
        if (scroll_pos > 0) {
            let html_view = sutta_html_view_layout.get_current_item();
            if (html_view && html_view.item && html_view.item.web) {
                let js = `setTimeout(function() { window.scrollTo(0, ${scroll_pos}); }, 200);`;
                html_view.item.web.runJavaScript(js);
            }
        }
    }

    function open_history_item(item_uid, table_name, sutta_ref, sutta_title) {
        // Check if the item is already open in a tab
        let models = [tabs_pinned_model, tabs_results_model, tabs_translations_model];
        for (let m = 0; m < models.length; m++) {
            for (let i = 0; i < models[m].count; i++) {
                if (models[m].get(i).item_uid === item_uid) {
                    root.focus_on_tab_with_id_key(models[m].get(i).id_key);
                    return;
                }
            }
        }

        let result_data = {
            item_uid: item_uid,
            table_name: table_name,
            sutta_ref: sutta_ref,
            sutta_title: sutta_title,
        };
        root.show_result_in_html_view(result_data, true);
    }

    // Keybindings loaded from settings
    property var keybindings: ({})

    // Load keybindings from settings
    function load_keybindings() {
        root.keybindings = JSON.parse(SuttaBridge.get_keybindings_json());
    }

    // Get shortcut sequences for an action, returns empty array if not found
    function get_sequences(action_id: string): var {
        return root.keybindings[action_id] || [];
    }

    Logger { id: logger }

    Connections {
        target: SuttaBridge

        function onUpdateWindowTitle(item_uid: string, sutta_ref: string, sutta_title: string) {
            /* logger.info("onUpdateWindowTitle():", item_uid, sutta_ref, sutta_title); */
            const current_key = sutta_html_view_layout.current_key;
            // Check if the item exists in items_map before accessing it
            if (current_key && sutta_html_view_layout.items_map[current_key] &&
                sutta_html_view_layout.items_map[current_key].get_data_value('item_uid') === item_uid) {
                root.update_window_title(item_uid, sutta_ref, sutta_title);
            }
        }

        function onResultsPageReady(results_json: string) {
            let d = JSON.parse(results_json);
            root.is_loading = false;

            // On parse error, preserve existing results and only update error state
            if (d.error) {
                root.has_query_error = true;
                query_tab.update_debug("", d.error);
                return;
            }

            fulltext_results.set_search_result_page(d);
        }

        function onDebugQueryReady(debug_json: string) {
            try {
                let d = JSON.parse(debug_json);
                let debug_text = d.debug_text || "";
                let error_text = d.error || "";
                query_tab.update_debug(debug_text, error_text);
                root.has_query_error = (error_text !== "");
            } catch (e) {
                query_tab.update_debug("", "Failed to parse debug response");
                root.has_query_error = true;
            }
        }

        function onShowChapterFromLibrary(window_id: string, result_data_json: string) {
            // Only handle this signal if it's for this window or if window_id is empty
            if (window_id === "" || window_id === root.window_id) {
                root.show_result_in_html_view_with_json(result_data_json);
            }
        }

        function onShowSuttaFromReferenceSearch(window_id: string, result_data_json: string) {
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
        /* logger.info("Theme JSON:\n---\n", theme_json, "\n---\n"); */
        if (theme_json.length === 0 || theme_json === "{}") {
            logger.error("Couldn't get theme JSON.")
            return;
        }

        try {
            var d = JSON.parse(theme_json);

            for (var color_group_key in d) {
                /* logger.info(color_group_key); // active, inactive, disabled */
                if (!root.palette.hasOwnProperty(color_group_key) || root.palette[color_group_key] === undefined) {
                    logger.error("Member not found on root.palette:", color_group_key);
                    continue;
                }
                var color_group = d[color_group_key];
                for (var color_role_key in color_group) {
                    /* logger.info(color_role_key); // window, windowText, etc. */
                    /* logger.info(color_group[color_role_key]); // #EFEFEF, #000000, etc. */
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
        /* logger.info("new_tab_data()", fulltext_results_data, pinned, focus_on_new); */
        if (!id_key) {
            id_key = root.generate_key();
        }
        // Generate the tabs with empty web_item_key. An item_key and associated webview
        // will be created when the tab is first focused.
        //
        // NOTE: same attributes as on TabButton.
        /* logger.info("item_uid", fulltext_results_data.item_uid); */
        /* logger.info("sutta_title", fulltext_results_data.sutta_title); */
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

    function get_open_items_json(): string {
        let items = [];

        function collect_from_model(model, tab_group) {
            for (let i = 0; i < model.count; i++) {
                let tab = model.get(i);
                if (tab.item_uid && tab.item_uid !== "Sutta" && tab.item_uid.length > 0) {
                    items.push({
                        item_uid: tab.item_uid,
                        table_name: tab.table_name || "suttas",
                        title: tab.sutta_title || "",
                        tab_group: tab_group,
                    });
                }
            }
        }

        collect_from_model(tabs_pinned_model, "pinned");
        collect_from_model(tabs_results_model, "results");
        collect_from_model(tabs_translations_model, "translations");

        return JSON.stringify(items);
    }

    function get_session_data_json(): string {
        let items = [];
        let sort_order = 0;

        function collect_from_model(model, tab_group) {
            for (let i = 0; i < model.count; i++) {
                let tab = model.get(i);
                if (tab.item_uid && tab.item_uid !== "Sutta" && tab.item_uid.length > 0) {
                    items.push({
                        item_uid: tab.item_uid,
                        table_name: tab.table_name || "suttas",
                        title: tab.sutta_title || "",
                        tab_group: tab_group,
                        scroll_position: 0.0,
                        find_query: "",
                        find_match_index: 0,
                        sort_order: sort_order,
                    });
                    sort_order++;
                }
            }
        }

        collect_from_model(tabs_pinned_model, "pinned");
        collect_from_model(tabs_results_model, "results");
        collect_from_model(tabs_translations_model, "translations");

        let session = {
            name: root.window_id || "window",
            items: items,
        };

        return JSON.stringify(session);
    }

    function save_last_session(windows_json: string) {
        SuttaBridge.save_last_session(windows_json);
    }

    function restore_last_session(session_json: string) {
        let session = JSON.parse(session_json);
        let items = session.items || [];
        root.is_restoring_session = true;
        for (let i = 0; i < items.length; i++) {
            let focus = (i === 0);
            root.open_bookmark_in_tab_group(items[i], focus);
        }
        root.is_restoring_session = false;
    }

    function get_restore_last_session_setting(): bool {
        return SuttaBridge.get_restore_last_session();
    }

    function get_last_session_json_from_bridge(): string {
        return SuttaBridge.get_last_session_json();
    }

    // Timer for incremental search debounce
    Timer {
        id: search_timer
        interval: 400 // milliseconds
        repeat: false
        onTriggered: {
            if (app_settings_window.search_as_you_type) {
                root.handle_query(search_bar_input.search_input.text, 4);
            }
        }
    }

    // Timer for debug query debounce
    Timer {
        id: debug_query_timer
        interval: 400
        repeat: false
        onTriggered: root.trigger_debug_query()
    }

    function trigger_debug_query() {
        let query_text = search_bar_input.search_input.text;
        if (query_text.length === 0) {
            return;
        }
        let search_area = search_bar_input.search_area;
        let params = root.get_search_params_from_ui();
        let params_json = JSON.stringify(params);
        SuttaBridge.debug_query(query_text, search_area, params_json);
    }

    Connections {
        target: search_bar_input.search_input
        function onTextChanged() {
            root.has_query_error = false;
            debug_query_timer.restart();
        }
    }

    function handle_query(query_text_orig: string, min_length=4) {
        if (query_text_orig === 'uid:')
            return;

        let params = root.get_search_params_from_ui();
        let search_area = search_bar_input.search_area;

        // Determine if query_text_orig is a sutta/book/dictionary reference
        // query_text_to_uid_field_query() returns the query as normal (e.g. 'heard') if not recognized as a uid
        // For all search areas, check if it matches a uid pattern:
        // - Suttas/Library: sutta refs like 'SN 56.11', book UIDs like 'bmc.0'
        // - Dictionary: dictionary UIDs like 'dhamma 1.01', '34626/dpd'
        let query_text = SuttaBridge.query_text_to_uid_field_query(query_text_orig);

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
        root.last_params = params;

        // FIXME: page number
        root.results_page(query_text, 0, search_area, params);

        // Also trigger debug query on explicit search
        root.trigger_debug_query();

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
        // Use the last query text, search area, and params instead of reading from the UI,
        // because the input might have been cleared or changed, and the search mode needs to match
        let query = root.last_query_text;
        let search_area = root.last_search_area;
        let params = root.last_params || root.get_search_params_from_ui();
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
        const search_area = search_bar_input.search_area;
        let lang = search_bar_input.language_filter_dropdown.get_text();
        // Dictionary currently only uses English language from DPD.
        if (search_area === "Dictionary") {
            lang = null;
        }

        const nikaya_prefix = search_bar_input.nikaya_prefix;
        const uid_prefix = search_bar_input.uid_prefix;

        return {
            mode: mode,
            page_len: 10,
            lang: lang,
            lang_include: true,
            source: null,
            source_include: true,
            enable_regex: false,
            fuzzy_distance: 0,
            include_cst_mula: SuttaBridge.get_include_cst_mula_in_search_results(),
            include_cst_commentary: SuttaBridge.get_include_cst_commentary_in_search_results(),
            nikaya_prefix: nikaya_prefix.length > 0 ? nikaya_prefix : null,
            uid_prefix: uid_prefix.length > 0 ? uid_prefix : null,
            include_ms_mula: SuttaBridge.get_include_ms_mula_in_search_results(),
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

    // Open a related sutta (commentary, sub-commentary, or root text) for the current tab.
    // relation: "att" (commentary), "tik" (sub-commentary), "mula" (root text)
    function open_related_sutta(relation: string) {
        const current_key = sutta_html_view_layout.current_key;
        if (!current_key || !sutta_html_view_layout.items_map[current_key]) {
            sutta_html_view_layout.show_transient_message("No sutta currently loaded");
            return;
        }

        const item_uid = sutta_html_view_layout.items_map[current_key].get_data_value('item_uid');
        const table_name = sutta_html_view_layout.items_map[current_key].get_data_value('table_name');

        if (!item_uid || table_name !== "suttas") {
            sutta_html_view_layout.show_transient_message("Not a sutta tab");
            return;
        }

        const result_json = SuttaBridge.find_related_sutta_json(item_uid, relation);
        const result = JSON.parse(result_json);

        if (result.found) {
            // Determine which model the current tab is in, and the index
            let target_model = null;
            let insert_index = -1;
            let is_pinned = false;

            for (let i = 0; i < tabs_pinned_model.count; i++) {
                if (tabs_pinned_model.get(i).web_item_key === current_key) {
                    target_model = tabs_pinned_model;
                    insert_index = i + 1;
                    is_pinned = true;
                    break;
                }
            }

            if (!target_model) {
                for (let i = 0; i < tabs_results_model.count; i++) {
                    if (tabs_results_model.get(i).web_item_key === current_key) {
                        target_model = tabs_results_model;
                        insert_index = i + 1;
                        break;
                    }
                }
            }

            if (!target_model) {
                for (let i = 0; i < tabs_translations_model.count; i++) {
                    if (tabs_translations_model.get(i).web_item_key === current_key) {
                        target_model = tabs_translations_model;
                        insert_index = i + 1;
                        break;
                    }
                }
            }

            if (!target_model) {
                // Fallback: add as a new results tab
                target_model = tabs_results_model;
                insert_index = tabs_results_model.count;
            }

            let tab_data = root.new_tab_data(result, is_pinned, true);
            // Generate web_item_key and create webview immediately since we'll focus it
            tab_data.web_item_key = root.generate_key();
            sutta_html_view_layout.add_item(tab_data, true);
            target_model.insert(insert_index, tab_data);
            root.focus_on_tab_with_id_key(tab_data.id_key);

            // Record navigation history
            let model_name = is_pinned ? "pinned"
                : (target_model === tabs_translations_model) ? "translations"
                : "results";
            root.nav_history_push({
                type: "tab_switch",
                tab_model: model_name,
                id_key: tab_data.id_key,
                scroll_position: 0,
                item_uid: tab_data.item_uid || "",
                table_name: tab_data.table_name || "",
                sutta_ref: tab_data.sutta_ref || "",
                sutta_title: tab_data.sutta_title || "",
            });
        } else {
            // Not found - show dialog offering to search by title
            related_sutta_not_found_dialog.search_title = result.sutta_title;
            related_sutta_not_found_dialog.open();
        }
    }

    function run_sutta_menu_action(action: string, query_text: string) {
        /* logger.info("run_sutta_menu_action():", action, query_text.slice(0, 30)); */

        switch (action) {
        case "load-translations":
            // Get the current tab's item_uid
            const current_key = sutta_html_view_layout.current_key;
            if (!current_key || !sutta_html_view_layout.items_map[current_key]) {
                sutta_html_view_layout.show_transient_message("No sutta currently loaded");
                break;
            }

            const item_uid = sutta_html_view_layout.items_map[current_key].get_data_value('item_uid');
            const table_name = sutta_html_view_layout.items_map[current_key].get_data_value('table_name');

            // Only load translations for sutta results, not dictionary or library results
            if (table_name === "dict_words" || table_name === "dpd_headwords" || table_name === "book_spine_items") {
                sutta_html_view_layout.show_transient_message("Translations not available");
                break;
            }

            if (!item_uid) {
                sutta_html_view_layout.show_transient_message("No sutta UID found");
                break;
            }

            // If the current tab is in the translations group, move it to results group first
            // to prevent it from being cleared when loading new translations
            for (let i = 0; i < tabs_translations_model.count; i++) {
                let tr_tab_data = tabs_translations_model.get(i);
                if (tr_tab_data.web_item_key === current_key) {
                    // Move tab from translations to results group
                    let new_tab_data = root.new_tab_data(tr_tab_data, false, true, root.generate_key(), tr_tab_data.web_item_key);
                    tabs_results_model.append(new_tab_data);
                    tabs_translations_model.remove(i);
                    root.focus_on_tab_with_id_key(new_tab_data.id_key);
                    break;
                }
            }

            const num_translations = root.load_translations_for_sutta(item_uid);
            sutta_html_view_layout.show_transient_message(`Loaded ${num_translations} translation(s)`);
            break;

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

        case "open-commentary-text":
            root.open_related_sutta("att");
            break;

        case "open-sub-commentary-text":
            root.open_related_sutta("tik");
            break;

        case "open-root-text":
            root.open_related_sutta("mula");
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

    // Clear existing translation tabs and load translations for the given sutta UID.
    // Returns the number of translations loaded.
    function load_translations_for_sutta(item_uid: string): int {
        // Remove existing webviews for translation tabs
        for (let i = 0; i < tabs_translations_model.count; i++) {
            let tr_tab_data = tabs_translations_model.get(i);
            if (tr_tab_data.web_item_key !== "") {
                sutta_html_view_layout.delete_item(tr_tab_data.web_item_key);
            }
        }
        tabs_translations_model.clear();

        let translations_data = JSON.parse(SuttaBridge.get_translations_data_json_for_sutta_uid(item_uid));

        for (let i = 0; i < translations_data.length; i++) {
            let tr_tab_data = root.new_tab_data(translations_data[i], false, false);
            tabs_translations_model.append(tr_tab_data);
        }

        return translations_data.length;
    }

    // Clear all translation tabs without loading new ones.
    function clear_translation_tabs() {
        for (let i = 0; i < tabs_translations_model.count; i++) {
            let tr_tab_data = tabs_translations_model.get(i);
            if (tr_tab_data.web_item_key !== "") {
                sutta_html_view_layout.delete_item(tr_tab_data.web_item_key);
            }
        }
        tabs_translations_model.clear();
    }

    function clear_all_tabs() {
        root.nav_history_paused = true;

        // Close pinned tabs from end to start to avoid index shifting
        for (let i = tabs_pinned.count - 1; i >= 0; i--) {
            tabs_pinned.itemAt(i).closeClicked();
        }

        // Close translation tabs from end to start
        for (let i = tabs_translations.count - 1; i >= 0; i--) {
            tabs_translations.itemAt(i).closeClicked();
        }

        // Close results tabs from end to start
        // The last remaining results tab's onCloseClicked resets it to blank "Sutta"
        for (let i = tabs_results.count - 1; i >= 0; i--) {
            tabs_results.itemAt(i).closeClicked();
        }

        root.nav_history_paused = false;
    }

    function show_result_in_html_view_with_json(result_data_json: string, new_tab) {
        if (new_tab === undefined) new_tab = false;
        let result_data = JSON.parse(result_data_json);
        root.show_result_in_html_view(result_data, new_tab);
    }

    function show_result_in_html_view(result_data: var, new_tab) {
        if (new_tab === undefined) new_tab = false;
        logger.debug("SHOW_RESULT: show_result_in_html_view() called - item_uid: " + result_data.item_uid + " new_tab: " + new_tab);
        let tab_data = root.new_tab_data(result_data);
        logger.debug("SHOW_RESULT: Created tab_data - id_key: " + tab_data.id_key + " web_item_key: " + tab_data.web_item_key);
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

        // Record navigation history (content replacement)
        let focus_id_key = new_tab ? tabs_results_model.get(tab_idx).id_key : "ResultsTab_0";
        root.nav_history_push(root.build_nav_entry("content_replace", focus_id_key, tab_data, 0));

        // Update TocTab if this is a book chapter
        if (tab_data.table_name === "book_spine_items" && tab_data.item_uid) {
            toc_tab.update_for_spine_item(tab_data.item_uid);
        }

        // Only add translation tabs for sutta results, not dictionary or library results.
        // During session restore, skip loading translations since they are restored directly from saved items.
        if (tab_data.table_name && tab_data.table_name !== "dict_words" && tab_data.table_name !== "dpd_headwords" && tab_data.table_name !== "book_spine_items") {
            if (!root.is_restoring_session) {
                root.load_translations_for_sutta(tab_data.item_uid);
            }

            // Only open find bar with search query if:
            // 1. User preference is enabled
            // 2. This is from a search result (not a sutta link, i.e. new_tab is false)
            // 3. Last search was in Suttas
            // 4. There is a query text available
            if (app_settings_window.open_find_in_sutta_results &&
                !new_tab &&
                root.last_search_area === "Suttas" &&
                root.last_query_text.length > 0) {
                let query_as_uid = SuttaBridge.query_text_to_uid_field_query(root.last_query_text);
                if (!query_as_uid.startsWith('uid:')) {
                    root.pending_find_query = root.last_query_text;
                }
            }
        } else {
            // For dictionary results, clear translation tabs
            root.clear_translation_tabs();
        }

        if (!root.is_wide) {
            show_sidebar_btn.checked = false;
        }
    }

    // Returns the index of the tab in the model.
    function add_results_tab(fulltext_results_data: var, focus_on_new = true, new_tab = false): int {
        logger.debug("ADD_RESULTS_TAB: add_results_tab() called - item_uid: " + fulltext_results_data.item_uid + " focus_on_new: " + focus_on_new + " new_tab: " + new_tab);
        logger.debug("ADD_RESULTS_TAB: tabs_results_model.count: " + tabs_results_model.count);
        if (new_tab || tabs_results_model.count == 0) {
            logger.debug("ADD_RESULTS_TAB: Adding a new results tab");
            let tab_data = root.new_tab_data(fulltext_results_data, false, focus_on_new);
            if (tabs_results_model.count == 0) {
                logger.debug("ADD_RESULTS_TAB: First tab, setting id_key to ResultsTab_0");
                tab_data.id_key = "ResultsTab_0";
            }
            logger.debug("ADD_RESULTS_TAB: tab_data - id_key: " + tab_data.id_key + " web_item_key: " + tab_data.web_item_key);
            // Only create webview if we're going to show it immediately (focus_on_new is true)
            // Otherwise leave web_item_key empty and let tab_checked_changed create it when tab is clicked
            if (tab_data.web_item_key == "" && tab_data.focus_on_new) {
                logger.debug("ADD_RESULTS_TAB: web_item_key is empty and focus_on_new is true, generating new key");
                tab_data.web_item_key = root.generate_key();
                logger.debug("ADD_RESULTS_TAB: Generated web_item_key: " + tab_data.web_item_key + ", calling add_item with show_item: " + tab_data.focus_on_new);
                // Show the item since focus_on_new is true
                sutta_html_view_layout.add_item(tab_data, true);
            } else if (tab_data.web_item_key == "") {
                logger.debug("ADD_RESULTS_TAB: web_item_key is empty but focus_on_new is false, leaving empty for lazy creation");
            }
            tabs_results_model.append(tab_data);
            logger.debug("ADD_RESULTS_TAB: Tab appended. New tabs_results_model.count: " + tabs_results_model.count);
            return tabs_results_model.count-1;
        } else {
            logger.debug("ADD_RESULTS_TAB: Updating existing results tab at index 0");
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
        /* logger.info("focus_on_tab_with_id_key()", id_key); */
        let tab = root.get_tab_with_id_key(id_key);
        if (tab) {
            tab.click();
        } else {
            logger.error("Error: Tab not found with id_key: " + id_key);
        }
    }

    function check_search_index_on_startup() {
        let status_json = SuttaBridge.check_search_index_status();
        try {
            let status = JSON.parse(status_json);
            if (!status.exists) {
                search_index_notification.status_text = "Search index not found. Fulltext search will not work until you build it.\n\nUse File > Rebuild Search Index to create one.";
                search_index_notification.open();
            } else if (!status.current) {
                search_index_notification.status_text = "Search index is outdated. Re-indexing is recommended for best results.\n\nUse File > Rebuild Search Index to update it.";
                search_index_notification.open();
            }
        } catch (e) {
            logger.warn("Failed to parse search index status: " + e);
        }
    }

    Component.onCompleted: {
        /* logger.info("SuttaSearchWindow: Component.onCompleted()"); */
        if (root.is_qml_preview) {
            return;
        } else {
            root.apply_theme();
            root.load_keybindings();
            SuttaBridge.load_db();
            SuttaBridge.appdata_first_query();
            SuttaBridge.dpd_first_query();
            SuttaBridge.dictionary_first_query();

            // Update top_bar_margin after app data is initialized
            // This will automatically update all child dialogs via property bindings
            root.update_top_bar_margin();

            // Start delayed update check timer
            update_check_timer.start();

            // Check search index status
            root.check_search_index_on_startup();
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

        // Push initial history entry for the first view
        if (tabs_results_model.count > 0) {
            let tab_data = tabs_results_model.get(0);
            root.nav_history_push(root.build_nav_entry("tab_switch", tab_data.id_key, tab_data, 0));
        }
    }

    function qml_preview_state() {
        gloss_tab_btn.click();
    }

    function set_query(text: string) {
        search_bar_input.search_input.text = text;
    }

    function run_lookup_query(query_text: string) {
        // Set search area to Dictionary
        search_bar_input.set_search_area("Dictionary");
        // Set the query text
        search_bar_input.search_input.text = query_text;
        // Run the search with min_length 1 to allow single character queries
        root.handle_query(query_text, 1);
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

    // Open a bookmark item in the correct tab group, with optional scroll/find restoration.
    // item_data: {item_uid, table_name, title, tab_group, scroll_position, find_query, find_match_index}
    // focus: whether to focus on the newly created tab
    function open_bookmark_in_tab_group(item_data: var, focus: bool) {
        let result_data = {
            item_uid: item_data.item_uid,
            table_name: item_data.table_name || "suttas",
            sutta_title: item_data.title || "",
            sutta_ref: "",
        };

        let tab_group = item_data.tab_group || "results";

        if (tab_group === "pinned") {
            let tab_data = root.new_tab_data(result_data, true, focus);
            if (focus) {
                tab_data.web_item_key = root.generate_key();
                sutta_html_view_layout.add_item(tab_data, true);
            }
            tabs_pinned_model.append(tab_data);
            if (focus) {
                root.focus_on_tab_with_id_key(tab_data.id_key);
                root.nav_history_push(root.build_nav_entry("tab_switch", tab_data.id_key, tab_data, 0));
            }
            if (tab_data.table_name === "book_spine_items" && tab_data.item_uid) {
                toc_tab.update_for_spine_item(tab_data.item_uid);
            }
        } else if (tab_group === "translations") {
            let tab_data = root.new_tab_data(result_data, false, focus);
            if (focus) {
                tab_data.web_item_key = root.generate_key();
                sutta_html_view_layout.add_item(tab_data, true);
            }
            tabs_translations_model.append(tab_data);
            if (focus) {
                root.focus_on_tab_with_id_key(tab_data.id_key);
                root.nav_history_push(root.build_nav_entry("tab_switch", tab_data.id_key, tab_data, 0));
            }
            if (tab_data.table_name === "book_spine_items" && tab_data.item_uid) {
                toc_tab.update_for_spine_item(tab_data.item_uid);
            }
        } else {
            // "results" — use the standard function which handles first-tab logic
            root.show_result_in_html_view(result_data, true);
        }

        // Schedule scroll position restoration if needed
        let scroll_pos = item_data.scroll_position || 0.0;
        if (scroll_pos > 0.0) {
            root.pending_bookmark_scroll = scroll_pos;
        }

        // Schedule find query restoration if needed
        let find_q = item_data.find_query || "";
        if (find_q.length > 0) {
            root.pending_find_query = find_q;
        }
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

    function toggle_search_ui_visibility(visible: bool) {
        root.is_reading_mode = !visible;
    }

    function get_visible_html_tabs(): var {
        // Returns an array of visible tab objects from all three tab groups
        var visible_tabs = [];
        for (var i = 0; i < tabs_row.children.length; i++) {
            var child = tabs_row.children[i];
            // Check if it's a tab button (has id_key) and is visible
            if (child.id_key !== undefined && child.visible) {
                visible_tabs.push(child);
            }
        }
        return visible_tabs;
    }

    function get_current_html_tab_index(visible_tabs: var): int {
        // Find the index of the currently checked tab in the visible_tabs array
        for (var i = 0; i < visible_tabs.length; i++) {
            if (visible_tabs[i].checked) {
                return i;
            }
        }
        return -1;
    }

    function activate_next_html_tab() {
        var visible_tabs = get_visible_html_tabs();
        if (visible_tabs.length === 0) return;
        var current_idx = get_current_html_tab_index(visible_tabs);
        var next_idx = (current_idx + 1) % visible_tabs.length;
        visible_tabs[next_idx].click();
    }

    function activate_previous_html_tab() {
        var visible_tabs = get_visible_html_tabs();
        if (visible_tabs.length === 0) return;
        var current_idx = get_current_html_tab_index(visible_tabs);
        var prev_idx = (current_idx - 1 + visible_tabs.length) % visible_tabs.length;
        visible_tabs[prev_idx].click();
    }

    function activate_next_sidebar_tab() {
        var count = rightside_tabs.count;
        if (count === 0) return;
        var next_idx = (rightside_tabs.currentIndex + 1) % count;
        rightside_tabs.setCurrentIndex(next_idx);
    }

    function activate_previous_sidebar_tab() {
        var count = rightside_tabs.count;
        if (count === 0) return;
        var prev_idx = (rightside_tabs.currentIndex - 1 + count) % count;
        rightside_tabs.setCurrentIndex(prev_idx);
    }

    onIs_reading_modeChanged: {
        search_ui_row.visible = !root.is_reading_mode;
        // On a narrow screen, the sidebar was already hidden when the user
        // enabled reading mode from the html button, and turning reader mode
        // off would show the sidebar for them instead of the html view.
        if (root.is_wide) {
            show_sidebar_btn.checked = !root.is_reading_mode;
        }
    }

    menuBar: MenuBar {
        visible: root.is_desktop
        // NOTE: A Menu > CMenuItem should always have an Action. This property
        // is expected when constructing the mobile_menu Drawer.
        Menu {
            id: file_menu
            title: "&File"

            CMenuItem {
                action: Action {
                    id: action_settings
                    text: "&Settings..."
                    shortcut: Shortcut {
                        sequences: root.get_sequences("settings")
                        context: Qt.WindowShortcut
                        onActivated: action_settings.trigger()
                    }
                    onTriggered: app_settings_window.show()
                }
            }

            CMenuItem {
                action: Action {
                    id: action_close_window
                    text: "&Close Window"
                    shortcut: Shortcut {
                        sequences: root.get_sequences("close_window")
                        context: Qt.WindowShortcut
                        onActivated: action_close_window.trigger()
                    }
                    onTriggered: root.close()
                }
            }

            CMenuItem {
                action: Action {
                    text: "&Quit Simsapa"
                    icon.source: "icons/32x32/fa_times-circle.png"
                    id: action_quit
                    shortcut: Shortcut {
                        sequences: root.get_sequences("quit_app")
                        context: Qt.WindowShortcut
                        onActivated: action_quit.trigger()
                    }
                    onTriggered: Qt.quit()
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
                        sequences: root.get_sequences("sutta_search")
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

                CMenuItem {
                action: Action {
                    id: action_reference_search
                    text: "&Reference Search..."
                    onTriggered: {
                        SuttaBridge.open_reference_search_window()
                    }
                }
            }

            CMenuItem {
                action: Action {
                    id: action_topic_index
                    text: "&Topic Index..."
                    onTriggered: {
                        SuttaBridge.open_topic_index_window()
                    }
                }
            }

            CMenuItem {
                action: Action {
                    id: action_chanting_practice
                    text: "&Chanting Practice..."
                    onTriggered: {
                        SuttaBridge.open_chanting_practice_window(root.window_id)
                    }
                }
            }
        }

        Menu {
            id: find_menu
            title: "&Find"

            CMenuItem {
                action: Action {
                    id: action_focus_search
                    text: "Focus Search Input"
                    shortcut: Shortcut {
                        sequences: root.get_sequences("focus_search")
                        context: Qt.WindowShortcut
                        onActivated: action_focus_search.trigger()
                    }
                    onTriggered: {
                        search_bar_input.search_input.forceActiveFocus();
                        search_bar_input.search_input.selectAll();
                    }
                }
            }

            CMenuItem {
                action: Action {
                    id: action_next_search_area
                    text: "Next Search Area"
                    shortcut: Shortcut {
                        sequences: root.get_sequences("next_search_area")
                        context: Qt.WindowShortcut
                        onActivated: action_next_search_area.trigger()
                    }
                    onTriggered: {
                        search_bar_input.cycle_search_area();
                    }
                }
            }

            CMenuItem {
                action: Action {
                    id: select_previous_result
                    text: "Previous Result"
                    shortcut: Shortcut {
                        sequences: root.get_sequences("prev_result")
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
                        sequences: root.get_sequences("next_result")
                        context: Qt.WindowShortcut
                        onActivated: select_next_result.trigger()
                    }
                    onTriggered: fulltext_results.select_next_result()
                }
            }

            CMenuItem {
                action: Action {
                    id: action_find_in_page
                    text: "Find in Page"
                    shortcut: Shortcut {
                        sequences: root.get_sequences("find_in_page")
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
                    text: "Find Next in Page"
                    shortcut: Shortcut {
                        sequences: root.get_sequences("find_next")
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
                    text: "Find Previous in Page"
                    shortcut: Shortcut {
                        sequences: root.get_sequences("find_prev")
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
            id: tabs_menu
            title: "&Tabs"

            CMenuItem {
                action: Action {
                    id: action_toggle_reading_mode
                    text: "Reading Mode"
                    shortcut: Shortcut {
                        sequences: root.get_sequences("toggle_reading_mode")
                        context: Qt.WindowShortcut
                        onActivated: action_toggle_reading_mode.trigger()
                    }
                    onTriggered: {
                        root.is_reading_mode = !root.is_reading_mode;
                    }
                }
            }

            CMenuItem {
                action: Action {
                    id: action_close_tab
                    text: "Close Tab"
                    shortcut: Shortcut {
                        sequences: root.get_sequences("close_tab")
                        context: Qt.WindowShortcut
                        onActivated: action_close_tab.trigger()
                    }
                    onTriggered: {
                        suttas_tab_bar.close_current_tab();
                    }
                }
            }

            CMenuItem {
                action: Action {
                    id: action_pin_tab
                    text: "Pin Tab"
                    shortcut: Shortcut {
                        sequences: root.get_sequences("pin_tab")
                        context: Qt.WindowShortcut
                        onActivated: action_pin_tab.trigger()
                    }
                    onTriggered: {
                        suttas_tab_bar.toggle_pin_current_tab();
                    }
                }
            }

            MenuSeparator {}

            CMenuItem {
                action: Action {
                    id: action_toggle_tab_list
                    text: "Toggle Tab List"
                    shortcut: Shortcut {
                        sequences: root.get_sequences("toggle_tab_list")
                        context: Qt.WindowShortcut
                        onActivated: action_toggle_tab_list.trigger()
                    }
                    onTriggered: {
                        if (tab_list_dialog.visible) {
                            tab_list_dialog.close();
                        } else {
                            tab_list_dialog.open();
                        }
                    }
                }
            }

            CMenuItem {
                action: Action {
                    id: action_previous_tab
                    text: "Previous Tab"
                    shortcut: Shortcut {
                        sequences: root.get_sequences("prev_tab")
                        context: Qt.WindowShortcut
                        onActivated: action_previous_tab.trigger()
                    }
                    onTriggered: root.activate_previous_html_tab()
                }
            }

            CMenuItem {
                action: Action {
                    id: action_next_tab
                    text: "Next Tab"
                    shortcut: Shortcut {
                        sequences: root.get_sequences("next_tab")
                        context: Qt.WindowShortcut
                        onActivated: action_next_tab.trigger()
                    }
                    onTriggered: root.activate_next_html_tab()
                }
            }

            CMenuItem {
                action: Action {
                    id: action_previous_sidebar_tab
                    text: "Previous Sidebar Tab"
                    shortcut: Shortcut {
                        sequences: root.get_sequences("prev_sidebar_tab")
                        context: Qt.WindowShortcut
                        onActivated: action_previous_sidebar_tab.trigger()
                    }
                    onTriggered: root.activate_previous_sidebar_tab()
                }
            }

            CMenuItem {
                action: Action {
                    id: action_next_sidebar_tab
                    text: "Next Sidebar Tab"
                    shortcut: Shortcut {
                        sequences: root.get_sequences("next_sidebar_tab")
                        context: Qt.WindowShortcut
                        onActivated: action_next_sidebar_tab.trigger()
                    }
                    onTriggered: root.activate_next_sidebar_tab()
                }
            }

            MenuSeparator {}

            CMenuItem {
                action: Action {
                    id: action_scroll_up
                    text: "Scroll Up"
                    shortcut: Shortcut {
                        sequences: root.get_sequences("scroll_up")
                        context: Qt.WindowShortcut
                        enabled: !tab_list_dialog.visible
                        onActivated: action_scroll_up.trigger()
                    }
                    onTriggered: {
                        let html_view = sutta_html_view_layout.get_current_item();
                        if (html_view) html_view.scroll_small_up();
                    }
                }
            }

            CMenuItem {
                action: Action {
                    id: action_scroll_down
                    text: "Scroll Down"
                    shortcut: Shortcut {
                        sequences: root.get_sequences("scroll_down")
                        context: Qt.WindowShortcut
                        enabled: !tab_list_dialog.visible
                        onActivated: action_scroll_down.trigger()
                    }
                    onTriggered: {
                        let html_view = sutta_html_view_layout.get_current_item();
                        if (html_view) html_view.scroll_small_down();
                    }
                }
            }

            CMenuItem {
                action: Action {
                    id: action_scroll_half_page_up
                    text: "Scroll Half Page Up"
                    shortcut: Shortcut {
                        sequences: root.get_sequences("scroll_half_page_up")
                        context: Qt.WindowShortcut
                        onActivated: action_scroll_half_page_up.trigger()
                    }
                    onTriggered: {
                        let html_view = sutta_html_view_layout.get_current_item();
                        if (html_view) html_view.scroll_half_page_up();
                    }
                }
            }

            CMenuItem {
                action: Action {
                    id: action_scroll_half_page_down
                    text: "Scroll Half Page Down"
                    shortcut: Shortcut {
                        sequences: root.get_sequences("scroll_half_page_down")
                        context: Qt.WindowShortcut
                        onActivated: action_scroll_half_page_down.trigger()
                    }
                    onTriggered: {
                        let html_view = sutta_html_view_layout.get_current_item();
                        if (html_view) html_view.scroll_half_page_down();
                    }
                }
            }

            CMenuItem {
                action: Action {
                    id: action_scroll_page_up
                    text: "Scroll Page Up"
                    shortcut: Shortcut {
                        sequences: root.get_sequences("scroll_page_up")
                        context: Qt.WindowShortcut
                        onActivated: action_scroll_page_up.trigger()
                    }
                    onTriggered: {
                        let html_view = sutta_html_view_layout.get_current_item();
                        if (html_view) html_view.scroll_page_up();
                    }
                }
            }

            CMenuItem {
                action: Action {
                    id: action_scroll_page_down
                    text: "Scroll Page Down"
                    shortcut: Shortcut {
                        sequences: root.get_sequences("scroll_page_down")
                        context: Qt.WindowShortcut
                        onActivated: action_scroll_page_down.trigger()
                    }
                    onTriggered: {
                        let html_view = sutta_html_view_layout.get_current_item();
                        if (html_view) html_view.scroll_page_down();
                    }
                }
            }

            CMenuItem {
                action: Action {
                    id: action_scroll_to_top
                    text: "Scroll to Top"
                    shortcut: Shortcut {
                        sequences: root.get_sequences("scroll_to_top")
                        context: Qt.WindowShortcut
                        enabled: !tab_list_dialog.visible
                        onActivated: action_scroll_to_top.trigger()
                    }
                    onTriggered: {
                        let html_view = sutta_html_view_layout.get_current_item();
                        if (html_view) html_view.scroll_to_top();
                    }
                }
            }

            CMenuItem {
                action: Action {
                    id: action_scroll_to_bottom
                    text: "Scroll to Bottom"
                    shortcut: Shortcut {
                        sequences: root.get_sequences("scroll_to_bottom")
                        context: Qt.WindowShortcut
                        enabled: !tab_list_dialog.visible
                        onActivated: action_scroll_to_bottom.trigger()
                    }
                    onTriggered: {
                        let html_view = sutta_html_view_layout.get_current_item();
                        if (html_view) html_view.scroll_to_bottom();
                    }
                }
            }
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
                    id: action_check_for_updates
                    text: "Check for Simsapa Updates..."
                    onTriggered: SuttaBridge.check_for_updates(true, Screen.desktopAvailableWidth + " x " + Screen.desktopAvailableHeight, "determine")
                }
            }

            CMenuItem {
                action: Action {
                    id: action_dhamma_text_sources
                    text: "Dhamma Text Sources"
                    onTriggered: dhamma_text_sources_dialog.show()
                }
            }

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
        // NOTE: No need for find_menu and tabs_menu on mobile, they are keyboard actions
        menu_list: [file_menu, windows_menu, gloss_menu, prompts_menu, help_menu]
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

    DatabaseValidationDialog {
        id: database_validation_dialog
        top_bar_margin: root.top_bar_margin
    }

    DhammaTextSourcesDialog {
        id: dhamma_text_sources_dialog
        top_bar_margin: root.top_bar_margin
    }

    UpdateNotificationDialog {
        id: update_notification_dialog
        top_bar_margin: root.top_bar_margin
    }

    Dialog {
        id: search_index_notification
        title: "Search Index"
        anchors.centerIn: parent
        modal: true
        standardButtons: Dialog.Ok
        width: 400

        property string status_text: ""

        ColumnLayout {
            spacing: 10
            width: parent.width

            Label {
                text: search_index_notification.status_text
                wrapMode: Text.WordWrap
                Layout.fillWidth: true
            }
        }
    }

    Dialog {
        id: related_sutta_not_found_dialog
        title: "Related Text Not Found"
        anchors.centerIn: parent
        modal: true
        standardButtons: Dialog.Ok | Dialog.Cancel
        width: 450

        property string search_title: ""

        onAccepted: {
            // Do NOT touch search_bar_input.search_area or search_input.text here.
            // Changing search_area triggers onSearch_areaChanged → load_language_labels_for_area
            // → language_filter_dropdown.currentIndex = 0 → onCurrentIndexChanged
            // → handle_query with old text and wrong UI params. That async search
            // can complete after ours and overwrite the results with MS Mūla records.
            //
            // Build params from scratch: pli language, CST Commentary enabled,
            // MS Mūla and CST Mūla disabled.
            let params = {
                mode: "Contains Match",
                page_len: 10,
                lang: "pli",
                lang_include: true,
                source: null,
                source_include: true,
                enable_regex: false,
                fuzzy_distance: 0,
                include_cst_mula: false,
                include_cst_commentary: true,
                nikaya_prefix: null,
                uid_prefix: null,
                include_ms_mula: false,
            };

            if (!show_sidebar_btn.checked) {
                show_sidebar_btn.checked = true;
            }
            rightside_tabs.setCurrentIndex(0);

            root.start_search_query_workers(
                related_sutta_not_found_dialog.search_title,
                "Suttas",
                params,
            );
        }

        ColumnLayout {
            spacing: 10
            width: parent.width

            Label {
                text: "The sutta couldn't be found by its uid. Mapping the SuttaCentral sutta references to the correct sections in the CST files is work-in-progress.\n\nSearch with the title to find the un-mapped CST xml file?\n\nThis will run a Contains Match query with Mūla texts disabled."
                wrapMode: Text.WordWrap
                Layout.fillWidth: true
            }
        }
    }

    AppSettingsWindow {
        id: app_settings_window
        top_bar_margin: root.top_bar_margin
        database_validation_dialog: database_validation_dialog
        onThemeChanged: function(theme_name) {
            SuttaBridge.set_theme_name(theme_name);
            root.apply_theme();
        }
        onMarginChanged: {
            root.update_top_bar_margin();
        }
        onKeybindingsChanged: {
            root.load_keybindings();
        }
    }

    // Timer for delayed update check on startup (500ms delay)
    // The equivalent of windows.py init_timer which runs _init_tasks()
    Timer {
        id: update_check_timer
        interval: 500
        repeat: false
        onTriggered: {
            if (!SuttaBridge.get_updates_checked()) {
                SuttaBridge.set_updates_checked(true);
                if (SuttaBridge.get_notify_about_simsapa_updates()) {
                    SuttaBridge.check_for_updates(false, Screen.desktopAvailableWidth + " x " + Screen.desktopAvailableHeight, "determine");
                }
            }
        }
    }

    // Connections for update check signals
    Connections {
        target: SuttaBridge

        function onAppUpdateAvailable(update_info_json: string) {
            update_notification_dialog.show_app_update(update_info_json);
        }

        function onDbUpdateAvailable(update_info_json: string) {
            update_notification_dialog.show_db_update(update_info_json);
        }

        function onLocalDbObsolete(update_info_json: string) {
            update_notification_dialog.show_obsolete_warning(update_info_json);
        }

        function onNoUpdatesAvailable() {
            update_notification_dialog.show_no_updates();
        }

        function onUpdateCheckError(error_message: string) {
            // Log error but don't show dialog on automatic startup check
            // For manual checks, the user will see "no updates" if check succeeds
            logger.info("Update check error:", error_message);
        }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.topMargin: root.top_bar_margin

        RowLayout {
            id: search_ui_row

            SearchBarInput {
                id: search_bar_input
                Layout.alignment: Qt.AlignTop
                is_wide: root.is_wide
                is_tall: root.is_tall
                icon_size: root.icon_size
                db_loaded: SuttaBridge.db_loaded
                handle_query_fn: root.handle_query
                search_timer: search_timer
                mobile_menu: mobile_menu
                search_as_you_type_checked: app_settings_window.search_as_you_type
                is_loading: root.is_loading
                has_query_error: root.has_query_error
                onAdvanced_options_changed: {
                    if (search_input.text.length > 0) {
                        root.handle_query(search_input.text);
                    }
                }
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

                                    logger.debug("TAB_CHECK: Tab checked changed. Tab id_key: " + tab.id_key + " item_uid: " + tab.item_uid + " web_item_key: " + tab.web_item_key + " index: " + tab.index);
                                    logger.debug("TAB_CHECK: Current SuttaStackLayout.current_key: " + sutta_html_view_layout.current_key);

                                    // Prevent redundant processing if this tab's webview is already showing
                                    // This can happen if the same tab is clicked multiple times
                                    if (tab.web_item_key !== "" && sutta_html_view_layout.current_key === tab.web_item_key) {
                                        logger.debug("TAB_CHECK: Tab webview already showing, skipping");
                                        return;
                                    }

                                    // If this tab doesn't have a webview associated yet, create one
                                    // This happens on first click of a tab (web_item_key is empty string)
                                    if (tab.web_item_key == "") {
                                        logger.debug("TAB_CHECK: Tab has no webview, creating new one");
                                        // Generate unique key for this webview
                                        let key = root.generate_key();
                                        tab.web_item_key = key;
                                        logger.debug("TAB_CHECK: Generated new web_item_key: " + key);

                                        // Update the key in both the model and the tab's property
                                        if (tab_model.count > tab.index) {
                                            let tab_data = tab_model.get(tab.index);
                                            tab_data.web_item_key = key;
                                            tab_model.set(tab.index, tab_data);
                                            tab.web_item_key = key;
                                            logger.debug("TAB_CHECK: Updated model with new web_item_key, calling add_item");
                                            // Add the webview but don't show it yet (show_item = false)
                                            // We'll set current_key explicitly below to ensure it happens after the item is fully added
                                            sutta_html_view_layout.add_item(tab_data, false);
                                            logger.debug("TAB_CHECK: add_item completed");
                                            // New webview created and added to layout
                                        } else {
                                            logger.error("Out of bounds error: tab_model.count " + tab_model.count + " tab.index " + tab.index);
                                        }
                                    } else {
                                        logger.debug("TAB_CHECK: Tab already has web_item_key, switching to existing webview");
                                    }
                                    // If tab already has a web_item_key, we're switching to an existing webview

                                    // Show the tab's webview by setting current_key
                                    // This is called after add_item to ensure the item is fully created
                                    logger.debug("TAB_CHECK: Setting current_key to: " + tab.web_item_key);
                                    sutta_html_view_layout.current_key = tab.web_item_key;
                                    logger.debug("TAB_CHECK: current_key set. New value: " + sutta_html_view_layout.current_key);

                                    // Scroll the checked tab into view in the tab bar
                                    suttas_tab_bar.scroll_tab_into_view(tab);

                                    // Record navigation history
                                    let model_name = (tab_model === tabs_pinned_model) ? "pinned"
                                        : (tab_model === tabs_translations_model) ? "translations"
                                        : "results";
                                    let tab_data_entry = tab_model.get(tab.index);
                                    root.nav_history_push(root.build_nav_entry("tab_switch", tab_data_entry.id_key, tab_data_entry, 0));

                                    // Update TocTab if switching to a book chapter tab
                                    if (tab_data_entry.table_name === "book_spine_items" && tab_data_entry.item_uid) {
                                        toc_tab.update_for_spine_item(tab_data_entry.item_uid);
                                    }

                                    // Tab switch completed: webview shown, tab scrolled into view
                                    logger.debug("TAB_CHECK: Tab switch completed");
                                }

                                function remove_tab_and_webview(tab: SuttaTabButton, tab_model: ListModel) {
                                    /* logger.info("remove_tab_and_webview()", tab.index, tab.item_uid, tab.web_item_key); */
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

                                function toggle_pin_current_tab() {
                                    // Find the currently checked tab and toggle its pin state

                                    // Check pinned tabs (will unpin)
                                    for (let i = 0; i < tabs_pinned.count; i++) {
                                        let tab = tabs_pinned.itemAt(i);
                                        if (tab && tab.checked) {
                                            tab.pin_btn.toggle();
                                            return;
                                        }
                                    }

                                    // Check results tabs (will pin)
                                    for (let i = 0; i < tabs_results.count; i++) {
                                        let tab = tabs_results.itemAt(i);
                                        if (tab && tab.checked) {
                                            tab.pin_btn.toggle();
                                            return;
                                        }
                                    }

                                    // Check translations tabs (will pin)
                                    for (let i = 0; i < tabs_translations.count; i++) {
                                        let tab = tabs_translations.itemAt(i);
                                        if (tab && tab.checked) {
                                            tab.pin_btn.toggle();
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
                                                    logger.debug("UNPIN: Starting unpin from pinned group. Tab index: " + pinned_tab_btn.index + " item_uid: " + pinned_tab_btn.item_uid + " id_key: " + pinned_tab_btn.id_key + " web_item_key: " + pinned_tab_btn.web_item_key);
                                                    logger.debug("UNPIN: Model counts before - pinned: " + tabs_pinned_model.count + " results: " + tabs_results_model.count + " translations: " + tabs_translations_model.count);

                                                    // If unpinning a blank tab, just remove it from pinned group.
                                                    // The hidden blank tab in results group will become visible automatically.
                                                    let is_blank_tab = (pinned_tab_btn.item_uid === "Sutta" || pinned_tab_btn.item_uid === "Word");
                                                    if (is_blank_tab) {
                                                        logger.debug("UNPIN: Blank tab, just removing from pinned group");
                                                        tabs_pinned_model.remove(pinned_tab_btn.index);
                                                        // Focus on the blank tab in results (ResultsTab_0)
                                                        root.focus_on_tab_with_id_key("ResultsTab_0");
                                                        logger.debug("UNPIN: Completed unpin of blank tab");
                                                        return;
                                                    }

                                                    let old_tab_data = tabs_pinned_model.get(pinned_tab_btn.index);
                                                    let new_tab_data = root.new_tab_data(old_tab_data, false, true, root.generate_key(), old_tab_data.web_item_key);
                                                    logger.debug("UNPIN: Created new tab data - old_id_key: " + old_tab_data.id_key + " new_id_key: " + new_tab_data.id_key + " web_item_key: " + new_tab_data.web_item_key);
                                                    tabs_results_model.append(new_tab_data);
                                                    tabs_pinned_model.remove(pinned_tab_btn.index)
                                                    logger.debug("UNPIN: Model counts after - pinned: " + tabs_pinned_model.count + " results: " + tabs_results_model.count + " translations: " + tabs_translations_model.count);
                                                    root.focus_on_tab_with_id_key(new_tab_data.id_key);
                                                    logger.debug("UNPIN: Completed unpin operation");
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
                                                // Blank tabs (item_uid is "Sutta" or "Word") should only be visible
                                                // when pinned and translations groups are empty, so the tab bar is not empty.
                                                visible: {
                                                    let is_blank_tab = (item_uid === "Sutta" || item_uid === "Word");
                                                    if (is_blank_tab) {
                                                        return tabs_pinned_model.count === 0 && tabs_translations_model.count === 0;
                                                    }
                                                    return true;
                                                }
                                                onPinToggled: function (pinned) {
                                                    // NOTE: Don't convert this to a method function, it causes a binding loop on the 'checked' property.
                                                    if (!pinned) return;
                                                    // Pin and move to pinned group
                                                    logger.debug("PIN from results: Starting pin. Tab index: " + results_tab_btn.index + " item_uid: " + results_tab_btn.item_uid + " id_key: " + results_tab_btn.id_key + " web_item_key: " + results_tab_btn.web_item_key);
                                                    logger.debug("PIN from results: Model counts before - pinned: " + tabs_pinned_model.count + " results: " + tabs_results_model.count + " translations: " + tabs_translations_model.count);
                                                    logger.debug("PIN from results: Current SuttaStackLayout.current_key: " + sutta_html_view_layout.current_key);
                                                    let old_tab_data = tabs_results_model.get(results_tab_btn.index);
                                                    // New pinned tab will get focus.
                                                    let new_tab_data = root.new_tab_data(old_tab_data, true, true, root.generate_key(), old_tab_data.web_item_key);
                                                    logger.debug("PIN from results: Created new tab data - old_id_key: " + old_tab_data.id_key + " new_id_key: " + new_tab_data.id_key + " web_item_key: " + new_tab_data.web_item_key);
                                                    tabs_pinned_model.append(new_tab_data);
                                                    logger.debug("PIN from results: Appended to pinned model. New pinned count: " + tabs_pinned_model.count);
                                                    // Remove the tab data, but webview remains associated with the pinned item.
                                                    tabs_results_model.remove(results_tab_btn.index);
                                                    logger.debug("PIN from results: Removed from results model. New results count: " + tabs_results_model.count);
                                                    // Only add a blank tab if we just removed the last tab from results group
                                                    if (tabs_results_model.count === 0) {
                                                        logger.debug("PIN from results: Results model is empty, adding blank tab");
                                                        root.add_results_tab(root.blank_sutta_tab_data(), false, true);
                                                        logger.debug("PIN from results: Blank tab added. Results count: " + tabs_results_model.count);
                                                    }
                                                    // TODO: If this is before add_results_tab(), the new results tab gets focus, despite of focus_on_new = false.
                                                    logger.debug("PIN from results: About to focus on new pinned tab with id_key: " + new_tab_data.id_key);
                                                    root.focus_on_tab_with_id_key(new_tab_data.id_key);
                                                    logger.debug("PIN from results: Completed pin operation. Final current_key: " + sutta_html_view_layout.current_key);
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

                                                        // Update the webview to show blank content
                                                        let comp = sutta_html_view_layout.get_item(old_tab_data.web_item_key);
                                                        if (comp) {
                                                            let data = {
                                                                item_uid: "Sutta",
                                                                table_name: "",
                                                                sutta_ref: "",
                                                                sutta_title: "",
                                                                anchor: "",
                                                            };
                                                            comp.data_json = JSON.stringify(data);
                                                        }
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
                                                    logger.debug("PIN from translations: Starting pin. Tab index: " + translations_tab_btn.index + " item_uid: " + translations_tab_btn.item_uid + " id_key: " + translations_tab_btn.id_key + " web_item_key: " + translations_tab_btn.web_item_key);
                                                    logger.debug("PIN from translations: Model counts before - pinned: " + tabs_pinned_model.count + " results: " + tabs_results_model.count + " translations: " + tabs_translations_model.count);
                                                    logger.debug("PIN from translations: Current SuttaStackLayout.current_key: " + sutta_html_view_layout.current_key);
                                                    let old_tab_data = tabs_translations_model.get(translations_tab_btn.index);
                                                    let new_tab_data = root.new_tab_data(old_tab_data, true, true, root.generate_key(), old_tab_data.web_item_key);
                                                    logger.debug("PIN from translations: Created new tab data - old_id_key: " + old_tab_data.id_key + " new_id_key: " + new_tab_data.id_key + " web_item_key: " + new_tab_data.web_item_key);
                                                    tabs_pinned_model.append(new_tab_data);
                                                    logger.debug("PIN from translations: Appended to pinned model. New pinned count: " + tabs_pinned_model.count);
                                                    tabs_translations_model.remove(translations_tab_btn.index);
                                                    logger.debug("PIN from translations: Removed from translations model. New translations count: " + tabs_translations_model.count);
                                                    logger.debug("PIN from translations: About to focus on new pinned tab with id_key: " + new_tab_data.id_key);
                                                    root.focus_on_tab_with_id_key(new_tab_data.id_key);
                                                    logger.debug("PIN from translations: Completed pin operation. Final current_key: " + sutta_html_view_layout.current_key);
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
                                id: tab_list_btn
                                icon.source: "icons/32x32/mdi--menu.png"
                                Layout.preferredWidth: 36
                                Layout.preferredHeight: 28 // 32 x 32 creates a gap under the tabs
                                flat: true
                                onClicked: tab_list_dialog.open()

                                background: Rectangle {
                                    color: "transparent"
                                    border.color: tab_list_btn.palette.mid
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
                            nav_history: root.nav_history
                            is_wide: root.is_wide
                            is_tall: root.is_tall

                            onTabSelected: function(id_key) {
                                root.focus_on_tab_with_id_key(id_key);
                            }

                            onHistoryItemSelected: function(item_uid, table_name, sutta_ref, sutta_title) {
                                root.open_history_item(item_uid, table_name, sutta_ref, sutta_title);
                            }

                            onClearAllTabs: root.clear_all_tabs()
                            onClearHistory: root.nav_history = []
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
                                    is_reading_mode: root.is_reading_mode
                                    // Hide the webview when the drawer menu or a dialog is open. The mobile webview
                                    // is always on top, obscuring other items.
                                    // Also respect the parent container's visibility
                                    visible: root.webview_visible && suttas_tab_container.visible

                                    onPage_loaded: {
                                        // Restore scroll position from bookmark
                                        if (root.pending_bookmark_scroll > 0.0) {
                                            let scroll_ratio = root.pending_bookmark_scroll;
                                            root.pending_bookmark_scroll = 0.0;
                                            let html_view = sutta_html_view_layout.get_current_item();
                                            if (html_view) {
                                                // Delay slightly to let content render
                                                let js = `setTimeout(function() { window.scrollTo(0, ${scroll_ratio} * document.documentElement.scrollHeight); }, 200);`;
                                                html_view.item.web.runJavaScript(js);
                                            }
                                        }

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
                                    search_as_you_type_checked: app_settings_window.search_as_you_type
                                }
                            }
                        }
                    }

                    Item {
                        id: sidebar_panel
                        SplitView.preferredWidth: show_sidebar_btn.checked ? (root.is_wide ? (parent.width * 0.5) : parent.width) : 0
                        visible: show_sidebar_btn.checked

                        // Show only icons when the sidebar is too narrow for tab titles
                        readonly property bool narrow_tabs: !root.is_wide

                        // Right side tabs
                        TabBar {
                            id: rightside_tabs
                            anchors.top: parent.top
                            anchors.left: parent.left
                            anchors.right: parent.right

                            onCurrentIndexChanged: {
                                // Refresh bookmarks tab when it becomes visible (index 5)
                                if (currentIndex === 5) {
                                    bookmarks_tab.load_open_items();
                                    bookmarks_tab.load_bookmarks();
                                }
                            }

                            TabButton {
                                text: "Results"
                                id: fulltext_results_tab_btn
                                icon.source: "icons/32x32/bx_search_alt_2.png"
                                display: sidebar_panel.narrow_tabs ? AbstractButton.IconOnly : AbstractButton.TextBesideIcon
                                padding: 5
                            }

                            TabButton {
                                text: "Dictionary"
                                id: dictionary_tab_btn
                                icon.source: "icons/32x32/bxs_book_content.png"
                                display: sidebar_panel.narrow_tabs ? AbstractButton.IconOnly : AbstractButton.TextBesideIcon
                                padding: 5
                            }

                            TabButton {
                                text: "Gloss"
                                id: gloss_tab_btn
                                icon.source: "icons/32x32/material-symbols--list-alt-outline.png"
                                display: sidebar_panel.narrow_tabs ? AbstractButton.IconOnly : AbstractButton.TextBesideIcon
                                padding: 5
                            }

                            TabButton {
                                text: "Prompts"
                                id: prompts_tab_btn
                                icon.source: "icons/32x32/grommet-icons--chat.png"
                                display: sidebar_panel.narrow_tabs ? AbstractButton.IconOnly : AbstractButton.TextBesideIcon
                                padding: 5
                            }

                            TabButton {
                                text: "TOC"
                                id: toc_tab_btn
                                icon.source: "icons/32x32/bxs_book_bookmark.png"
                                display: sidebar_panel.narrow_tabs ? AbstractButton.IconOnly : AbstractButton.TextBesideIcon
                                padding: 5
                            }

                            TabButton {
                                text: "Bookmarks"
                                id: bookmarks_tab_btn
                                icon.source: "icons/32x32/fa_bookmark-solid.png"
                                display: sidebar_panel.narrow_tabs ? AbstractButton.IconOnly : AbstractButton.TextBesideIcon
                                padding: 5
                            }

                            // TODO: WIP work in progress, commented out for production build
                            // TabButton {
                            //     text: "Query"
                            //     id: query_tab_btn
                            //     icon.source: root.has_query_error ? "icons/32x32/fa_triangle-exclamation-solid.png" : "icons/32x32/fa_circle-info-solid.png"
                            //     padding: 5
                            // }

                            // TabButton {
                            //     text: "History"
                            //     id: history_tab_btn
                            //     icon.source: "icons/32x32/fa_clock-rotate-left-solid.png"
                            //     padding: 5
                            // }
                        }

                        // Tab title shown when tab bar is icon-only (narrow sidebar)
                        Label {
                            id: tab_title_label
                            anchors.top: rightside_tabs.bottom
                            anchors.left: parent.left
                            anchors.right: parent.right
                            anchors.topMargin: 4
                            anchors.leftMargin: 8
                            visible: sidebar_panel.narrow_tabs
                            text: rightside_tabs.currentItem ? rightside_tabs.currentItem.text : ""
                            font.bold: true
                            font.pointSize: 12
                        }

                        // Tab content areas
                        StackLayout {
                            id: tab_stack
                            currentIndex: rightside_tabs.currentIndex
                            anchors.top: sidebar_panel.narrow_tabs ? tab_title_label.bottom : rightside_tabs.bottom
                            anchors.topMargin: 5
                            width: parent.width
                            anchors.bottom: parent.bottom

                            FulltextResults {
                                id: fulltext_results
                                is_loading: root.is_loading
                                is_dark: root.is_dark
                                new_results_page_fn: root.new_results_page

                                function update_item() {
                                    /* logger.info("update_item()"); */
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

                            BookmarksTab {
                                id: bookmarks_tab
                                is_dark: root.is_dark
                                get_open_items_fn: root.get_open_items_json
                                Layout.fillWidth: true
                                Layout.fillHeight: true

                                onOpen_bookmark_item: function(item_data) {
                                    root.open_bookmark_in_tab_group(item_data, true);
                                }

                                onOpen_all_folder_items: function(items) {
                                    for (let i = 0; i < items.length; i++) {
                                        // Focus only the first item in each tab group
                                        let focus = (i === 0);
                                        root.open_bookmark_in_tab_group(items[i], focus);
                                    }
                                }
                            }

                            QueryTab {
                                id: query_tab
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
