import QtQuick
import QtQuick.Layouts
import QtQuick.Controls

import com.profoundlabs.simsapa

Frame {
    id: root
    Layout.fillWidth: true
    Layout.minimumHeight: root.icon_size

    Logger { id: logger }

    readonly property bool is_mobile: Qt.platform.os === "android" || Qt.platform.os === "ios"
    readonly property bool is_desktop: !root.is_mobile

    required property int window_width
    required property bool is_wide
    required property bool is_tall
    required property bool db_loaded
    required property bool searcher_ready
    required property var handle_query_fn
    required property Timer search_timer
    required property DrawerMenu mobile_menu
    required property bool search_as_you_type_checked
    required property bool is_loading
    required property bool has_query_error

    required property int icon_size

    property alias search_input: search_input
    property alias search_mode_dropdown: search_mode_dropdown
    property alias language_filter_dropdown: language_filter_dropdown
    property alias advanced_options_btn: advanced_options_btn

    // Search area state: "Suttas", "Dictionary", or "Library"
    property string search_area: "Suttas"
    readonly property var search_area_list: ["Suttas", "Dictionary", "Library"]

    function set_search_area(area: string) {
        if (search_area_list.indexOf(area) !== -1) {
            search_area = area;
        }
    }

    function cycle_search_area() {
        const current_index = search_area_list.indexOf(search_area);
        const next_index = (current_index + 1) % search_area_list.length;
        search_area = search_area_list[next_index];
    }

    background: Rectangle {
        color: "transparent"
        border.color: "transparent"
        border.width: 0
    }

    // Build the language dropdown model for the given area from the distinct
    // language values in the database. The same on-demand distinct-value query
    // is used for every area (Suttas, Dictionary, Library) for consistency.
    // Index 0 is the "Language"/"Lang" sentinel meaning "no language filter".
    // Index restoration is owned by the dropdown's restore_for_current_area().
    function load_language_labels_for_area(area: string) {
        let lang_labels;
        if (area === "Suttas") {
            lang_labels = SuttaBridge.get_sutta_language_labels();
        } else if (area === "Library") {
            lang_labels = SuttaBridge.get_library_language_labels();
        } else {
            // Dictionary: filter by the languages present in the dictionaries DB.
            lang_labels = SuttaBridge.get_dict_language_labels();
        }
        // Shorter first label for narrow screens.
        const first_label = root.is_wide ? "Language" : "Lang";
        language_filter_dropdown.model = [first_label].concat(lang_labels);
    }

    // EXACTLY ONE query per area switch.
    //
    // Both dropdowns restore their per-area mode/language independently via
    // their own `Connections { onSearch_areaChanged }` (pure restores — no
    // query). The single query is then fired by `area_query_coordinator`, a
    // Connections declared AFTER both dropdowns so it connects (and therefore
    // fires) last: after the ComboBox `model` bindings have re-evaluated and
    // after both restores have run. This guarantees the query reads the correct
    // mode + language and never fires twice — a second query would cost real
    // compute and can cause slowdown.
    //
    // Connection-order note: a root *inline* onSearch_areaChanged would connect
    // before the child dropdowns' `model` bindings and fire too early, so the
    // coordinator must be a Connections object placed after the dropdowns.
    //
    // On initial load the dropdowns restore in their own Component.onCompleted
    // (children complete before the parent), so root.Component.onCompleted can
    // fire the one initial query.
    Component.onCompleted: {
        // Keyboard diagnostics: log the detected platform once at startup so we
        // can confirm whether a Chromebook (Android app) is treated as mobile.
        logger.info("SearchBarInput: Qt.platform.os=" + Qt.platform.os
            + " is_mobile=" + root.is_mobile + " is_desktop=" + root.is_desktop
            + " search_input.focus=" + search_input.focus
            + " inputMethod.visible=" + Qt.inputMethod.visible);
        root.handle_query_fn(search_input.text); // qmllint disable use-proper-function
    }

    function user_typed() {
        // TODO self._show_search_normal_icon()
        if (root.search_as_you_type_checked) root.search_timer.restart();
    }

    Flow {
        id: search_bar_layout
        width: parent.width
        spacing: 5

        RowLayout {
            id: search_input_layout
            /* On wide screens, constrain the search input to 600px width so the
             * options can sit beside it. On narrow screens, let it take full
             * width, which will push the options to wrap below. */
            width: root.is_wide ? 600 : parent.width

            // Open/close the drawer menu on mobile
            Button {
                id: show_menu
                visible: root.is_mobile
                icon.source: "icons/32x32/mdi--menu.png"
                Layout.preferredHeight: root.icon_size
                Layout.preferredWidth: root.icon_size
                ToolTip.visible: hovered
                ToolTip.text: "Show Menu"
                onClicked: root.mobile_menu.open()
            }

            // === Search Input ====
            TextField {
                id: search_input
                enabled: root.db_loaded && root.searcher_ready
                Layout.fillWidth: true
                Layout.preferredWidth: root.is_wide ? 500 : 250
                Layout.preferredHeight: root.icon_size

                // Auto-focus on desktop so the user can type immediately. On
                // mobile do NOT pre-grab focus: with `focus: true` the field
                // already holds active focus once the DB finishes loading, so
                // the first physical tap is not a focus transition and Android
                // never raises the soft keyboard (needing a second tap).
                // Leaving it unfocused makes the first tap a real focus change.
                focus: root.is_desktop
                // Pāli queries are lowercase; stop the soft keyboard from
                // auto-capitalising the first letter (Sentence case).
                inputMethodHints: Qt.ImhNoAutoUppercase | Qt.ImhPreferLowercase
                // Make the soft keyboard's action key a "Search" button. On
                // Android this maps to IME_ACTION_SEARCH, which is consistent
                // across taps (otherwise the first focus can show a "Next"
                // arrow that does not emit `accepted`) and fires `onAccepted`
                // when pressed, starting the query.
                EnterKey.type: Qt.EnterKeySearch
                font.pointSize: root.is_mobile ? 14 : 12
                placeholderText: {
                    if (!root.db_loaded || !root.searcher_ready) return "Loading...";
                    if (root.search_area === "Dictionary") return "Search in dictionary";
                    if (root.search_area === "Library") return "Search in library";
                    return "Search in suttas";
                }

                onAccepted: search_btn.clicked()
                onTextChanged: root.user_typed()
                selectByMouse: true

                // Keyboard diagnostics: report focus transitions so we can see
                // whether tapping the field actually moves active focus to it
                // (the precondition for the IME to be raised).
                onActiveFocusChanged: logger.info("search_input: activeFocus="
                    + search_input.activeFocus + " inputMethod.visible="
                    + Qt.inputMethod.visible)

                // Reliably raise the Android/ChromeOS soft keyboard on the
                // first tap. See docs/android-soft-keyboard.md.
                MobileKeyboardHelper {}
            }

            Button {
                id: search_btn
                icon.source: root.has_query_error ? "icons/32x32/fa_triangle-exclamation-solid.png" : (root.is_loading ? "icons/32x32/fa_stopwatch-solid.png" : "icons/32x32/bx_search_alt_2.png")
                enabled: search_input.text.length > 0
                onClicked: root.handle_query_fn(search_input.text, 1) // qmllint disable use-proper-function
                Layout.preferredHeight: root.icon_size
                Layout.preferredWidth: root.icon_size
            }
        }

        RowLayout {
            id: search_options_layout

            Button {
                id: advanced_options_btn
                checkable: true
                icon.source: "icons/32x32/system-uicons--settings.png"
                Layout.preferredHeight: root.icon_size
                Layout.preferredWidth: root.icon_size
                ToolTip.visible: hovered
                ToolTip.text: "Advanced search options"
            }

            // Search area buttons (S = Suttas, D = Dictionary, L = Library)
            Row {
                id: search_area_buttons
                spacing: 0

                Button {
                    id: btn_suttas
                    text: "S"
                    checked: root.search_area === "Suttas"
                    checkable: true
                    autoExclusive: true
                    implicitWidth: root.icon_size
                    implicitHeight: root.icon_size
                    ToolTip.visible: hovered
                    ToolTip.text: "Suttas"
                    onClicked: root.search_area = "Suttas"
                }

                Button {
                    id: btn_dictionary
                    text: "D"
                    checked: root.search_area === "Dictionary"
                    checkable: true
                    autoExclusive: true
                    implicitWidth: root.icon_size
                    implicitHeight: root.icon_size
                    ToolTip.visible: hovered
                    ToolTip.text: "Dictionary"
                    onClicked: root.search_area = "Dictionary"
                }

                Button {
                    id: btn_library
                    text: "L"
                    checked: root.search_area === "Library"
                    checkable: true
                    autoExclusive: true
                    implicitWidth: root.icon_size
                    implicitHeight: root.icon_size
                    ToolTip.visible: hovered
                    ToolTip.text: "Library"
                    onClicked: root.search_area = "Library"
                }
            }

            ComboBox {
                id: search_mode_dropdown
                Layout.preferredHeight: root.icon_size
                Layout.preferredWidth: root.is_wide ? 120 : 80

                readonly property var search_mode_label_wide: {
                    "Suttas": [
                        "Fulltext Match",
                        "Contains Match",
                        "Title Match",
                    ],
                    "Library": [
                        "Fulltext Match",
                        "Contains Match",
                        "Title Match",
                    ],
                    "Dictionary": [
                        "Combined",
                        "DPD Lookup",
                        "Fulltext Match",
                        "Contains Match",
                        "Headword Match",
                    ],
                }

                // For narrow screen, show shorter label texts.
                // Value reading uses get_text(), which will return the longer label text,
                // which is used for the JSON search parameters.
                readonly property var search_mode_label_narrow: {
                    "Suttas": [
                        "Fulltext",
                        "Contains",
                        "Title",
                    ],
                    "Library": [
                        "Fulltext",
                        "Contains",
                        "Title",
                    ],
                    "Dictionary": [
                        "Combined",
                        "Lookup",
                        "Fulltext",
                        "Contains",
                        "Headword",
                    ],
                }

                // When true, suppress side-effects (persistence + query) of
                // currentIndex changes caused by programmatic restores rather
                // than by the user.
                property bool suppress_persist: false

                // Tracks the area whose saved mode is currently applied, so
                // is_wide-driven model swaps don't get treated as area changes.
                property string applied_area: ""

                model: {
                    if (root.is_wide) {
                        return search_mode_label_wide[root.search_area];
                    } else {
                        return search_mode_label_narrow[root.search_area];
                    }
                }

                // Pure restore (no query). The single query for an area switch
                // is fired last by root's area_query_coordinator, after BOTH
                // dropdowns have restored, so it uses the freshly restored mode
                // + language and never fires twice.
                function restore_for_current_area() {
                    const wide_list = search_mode_label_wide[root.search_area];
                    const saved_mode = SuttaBridge.get_last_search_mode(root.search_area);
                    let idx = wide_list.indexOf(saved_mode);
                    if (idx === -1) idx = 0;
                    suppress_persist = true;
                    currentIndex = idx;
                    suppress_persist = false;
                    applied_area = root.search_area;
                }

                Component.onCompleted: restore_for_current_area()

                Connections {
                    target: root
                    // Driven by area change (not by model change), because
                    // Suttas and Library share identical model labels — a
                    // Suttas↔Library switch produces no model-change signal,
                    // and a Dictionary→shorter-area switch may emit
                    // model-change after ComboBox auto-clips currentIndex.
                    function onSearch_areaChanged() {
                        search_mode_dropdown.restore_for_current_area();
                    }
                }

                onCurrentIndexChanged: {
                    if (suppress_persist) return;
                    // Mid-transition between search areas: the model just
                    // rebound and ComboBox auto-clipped currentIndex into the
                    // new (shorter) list before the area-restore could run.
                    // Ignore — the restore in onModelChanged will set the
                    // correct index for the new area.
                    if (applied_area !== root.search_area) return;
                    SuttaBridge.set_last_search_mode(root.search_area, get_text());
                    root.handle_query_fn(search_input.text); // qmllint disable use-proper-function
                }

                function get_text(): string {
                    // Return the value using the wide values which is expected for JSON search parameters.
                    return search_mode_label_wide[root.search_area][currentIndex];
                }
            }

            // Button {
            //     id: language_include_btn
            //     checkable: true
            //     icon.source: "icons/32x32/fa_plus-solid.png"
            //     Layout.preferredHeight: root.icon_size
            //     Layout.preferredWidth: root.icon_size
            //     ToolTip.visible: hovered
            //     ToolTip.text: "+ means 'must include', - means 'must exclude'"
            // }

            ComboBox {
                id: language_filter_dropdown
                Layout.preferredHeight: root.icon_size
                Layout.preferredWidth: root.is_wide ? 120 : 80
                // The model is rebuilt per area by load_language_labels_for_area;
                // index 0 ("Language"/"Lang") is the no-filter sentinel.
                model: root.is_wide ? ["Language",] : ["Lang",]
                enabled: root.search_area === "Suttas" || root.search_area === "Library" || root.search_area === "Dictionary"

                // When true, suppress side-effects (persistence + query) of
                // currentIndex changes caused by programmatic restores rather
                // than by the user. Mirrors search_mode_dropdown.
                property bool suppress_persist: false

                // Tracks the area whose saved language is currently applied, so
                // is_wide-driven model swaps don't get treated as area changes.
                property string applied_area: ""

                // Rebuild the model for the current area and restore the
                // per-area saved language key (defaulting to index 0 = no
                // filter). The language key is persisted separately per area,
                // exactly like the search mode (see set_language_filter_key).
                function restore_for_current_area() {
                    root.load_language_labels_for_area(root.search_area);
                    const saved_key = SuttaBridge.get_language_filter_key(root.search_area);
                    let idx = 0;
                    if (saved_key && saved_key !== "Language" && saved_key !== "Lang") {
                        const found = model.indexOf(saved_key);
                        if (found !== -1) idx = found;
                    }
                    suppress_persist = true;
                    currentIndex = idx;
                    suppress_persist = false;
                    applied_area = root.search_area;
                }

                Component.onCompleted: restore_for_current_area()

                Connections {
                    target: root
                    // Area change: rebuild labels + restore the saved language
                    // for the new area. Pure restore — the single query is
                    // fired afterwards by root's area_query_coordinator.
                    function onSearch_areaChanged() {
                        language_filter_dropdown.restore_for_current_area();
                    }
                    // is_wide toggles the first-label width ("Language"↔"Lang")
                    // and rebuilds the model; restore preserves the selection
                    // from the persisted per-area key. No query is fired (this
                    // is only a relabel).
                    function onIs_wideChanged() {
                        language_filter_dropdown.restore_for_current_area();
                    }
                }

                onCurrentIndexChanged: {
                    if (suppress_persist) return;
                    if (!enabled) return;
                    // Mid-transition between search areas: the model just
                    // rebound and ComboBox auto-clipped currentIndex before the
                    // area-restore could run. Ignore — restore_for_current_area
                    // will set the correct index for the new area.
                    if (applied_area !== root.search_area) return;
                    // No-op guard: a deferred ComboBox model reconciliation can
                    // re-assert the already-restored index after restore ended
                    // (suppress_persist is false by then). If the value already
                    // matches the persisted per-area key, skip — otherwise we'd
                    // fire a redundant query on top of the area_query_coordinator.
                    const new_key = get_text();
                    let saved = SuttaBridge.get_language_filter_key(root.search_area);
                    if (!saved) saved = "Language";
                    if (new_key === saved) return;
                    SuttaBridge.set_language_filter_key(root.search_area, new_key);
                    // Re-run search (handle_query will check text min length)
                    root.handle_query_fn(search_input.text); // qmllint disable use-proper-function
                }

                function get_text(): string {
                    // Always return "Language" for index 0, because it is a fixed keyword for
                    // "no language filter is selected".
                    if (currentIndex === 0) {
                        return "Language";
                    } else {
                        return model[currentIndex];
                    }
                }
            }

            // Fires the single per-area-switch query. Declared AFTER both
            // dropdowns so its connection to root.search_areaChanged is made
            // last — it therefore runs after the dropdowns' model bindings have
            // re-evaluated and after both restore_for_current_area() calls, so
            // the one query reads the freshly restored mode + language. This is
            // what prevents a second (wasteful) query on an area switch.
            Connections {
                id: area_query_coordinator
                target: root
                function onSearch_areaChanged() {
                    root.handle_query_fn(search_input.text); // qmllint disable use-proper-function
                }
            }

            // Button {
            //     id: source_include_btn
            //     checkable: true
            //     icon.source: "icons/32x32/fa_plus-solid.png"
            //     Layout.preferredHeight: root.icon_size
            //     Layout.preferredWidth: root.icon_size
            //     ToolTip.visible: hovered
            //     ToolTip.text: "+ means 'must include', - means 'must exclude'"
            // }

            // ComboBox {
            //     id: source_filter_dropdown
            //     Layout.preferredHeight: root.icon_size
            //     model: [
            //         "Sources",
            //         "ms",
            //         "cst",
            //     ]
            // }
        }
    }
}
