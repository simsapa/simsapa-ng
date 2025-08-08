pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
// import QtQuick.Dialogs

import com.profoundlabs.simsapa

Item {
    id: root

    required property string window_id
    required property bool is_dark
    readonly property bool is_mobile: Qt.platform.os === "android" || Qt.platform.os === "ios"
    readonly property bool is_desktop: !root.is_mobile
    readonly property bool is_qml_preview: Qt.application.name === "Qml Runtime"

    readonly property int vocab_font_point_size: 10
    readonly property TextMetrics vocab_tm1: TextMetrics { text: "#"; font.pointSize: root.vocab_font_point_size }

    property alias gloss_text_input_area: gloss_text_input

    required property var handle_open_dict_tab_fn

    property string text_color: root.is_dark ? "#F0F0F0" : "#000000"
    property string bg_color: root.is_dark ? "#23272E" : "#FAE6B2"
    property string bg_color_lighter: root.is_dark ? "#2E333D" : "#FBEDC7"
    property string bg_color_darker: root.is_dark ? "#1C2025" : "#F8DA8E"

    property string border_color: root.is_dark ? "#0a0a0a" : "#ccc"

    SuttaBridge { id: sb }

    // Current session data
    property string current_session_id: ""
    property string current_text: ""

    // Common words to filter out
    property var common_words: []

    // Global deduplication option
    property bool no_duplicates_globally: true

    // Track globally shown stem words
    property var global_shown_stems: ({})

    // Stores recent glossing sessions
    ListModel { id: history_model }

    // Gloss data per paragraph
    ListModel { id: paragraph_model }

    Component.onCompleted: {
        load_history();
        load_common_words();
        if (root.is_qml_preview) {
            qml_preview_state();
        }
    }

    Timer {
        id: delayed_click
        interval: 100
        running: false
        repeat: false
        onTriggered: update_all_glosses_btn.click()
    }

    function qml_preview_state() {
        let text = `Katamañca, bhikkhave, samādhindriyaṁ? Idha, bhikkhave, ariyasāvako vossaggārammaṇaṁ karitvā labhati samādhiṁ, labhati cittassa ekaggataṁ.

So vivicceva kāmehi vivicca akusalehi dhammehi savitakkaṁ savicāraṁ vivekajaṁ pītisukhaṁ paṭhamaṁ jhānaṁ upasampajja viharati.`;

        gloss_text_input.text = text;
        delayed_click.start();
    }

    function load_common_words() {
        var saved_words = sb.get_common_words_json();
        if (saved_words) {
            try {
                root.common_words = JSON.parse(saved_words);
            } catch (e) {
                console.error("Failed to parse common words:", e);
            }
        }
    }

    function save_common_words() {
        sb.save_common_words_json(JSON.stringify(root.common_words));
    }

    function load_history() {
        history_model.clear()
        var history_json = sb.get_gloss_history_json();
        if (history_json) {
            try {
                var data = JSON.parse(history_json);
                for (var i = 0; i < data.length; i++) {
                    history_model.append({
                        db_id: data[i].id,
                        modified_time: data[i].modified,
                        gloss_data: data[i].data
                    });
                }
            } catch (e) {
                console.error("Failed to parse history:", e);
            }
        }
    }

    function save_session() {
        var gloss_data = {
            text: root.current_text,
            paragraphs: [],
            no_duplicates_globally: root.no_duplicates_globally,
        };

        for (var i = 0; i < paragraph_model.count; i++) {
            var paragraph = paragraph_model.get(i);
            var words = [];

            // Extract words data from the paragraph's words_json property
            if (paragraph.words_json) {
                words = JSON.parse(paragraph.words_json);
            }

            gloss_data.paragraphs.push({
                text: paragraph.text,
                words: words,
            });
        }

        if (root.current_session_id) {
            sb.update_gloss_session(root.current_session_id, JSON.stringify(gloss_data));
        } else {
            root.current_session_id = sb.save_new_gloss_session(JSON.stringify(gloss_data));
        }

        root.load_history();
    }

    function extract_words(text: string): list<string> {
        // NOTE: QML regex text.match(/\b\w+\b/g) doesn't match unicode and
        // splits Pāli words: samādhi → ['sam', 'dhi']
        // Hence simply splitting on space.
        // Need to filter empty strings, spiltting "" with " " results in [""].
        return text.replace('\n', ' ').split(' ').filter(i => i.length != 0) || [];
    }

    function extract_words_with_context(text: string): list<var> {
        var sentences = text.match(/[^.!?]+[.!?]+/g) || [text];
        var words_with_context = [];

        for (var i = 0; i < sentences.length; i++) {
            var sentence = sentences[i].trim();
            var words = root.extract_words(sentence);

            for (var j = 0; j < words.length; j++) {
                words_with_context.push({
                    word: words[j],
                    sentence: sentence,
                    position: sentence.indexOf(words[j]),
                });
            }
        }

        return words_with_context;
    }

    // Clean stem by removing disambiguating numbers
    // (e.g., "ña 2.1" → "ña", "jhāyī 1" → "jhāyī")
    function clean_stem(stem: string): string {
        return stem.replace(/\s+\d+(\.\d+)?$/, '').toLowerCase();
    }

    function is_common_word(stem: string): bool {
        return root.common_words.includes(clean_stem(stem));
    }

    function process_word(word: string, lookup_results, sentence: string): var {
        var word_data = {
            original_word: word,
            results: lookup_results,
            selected_index: 0,
            stem: lookup_results[0].word,
            example_sentence: sentence || "",
        };
        return word_data;
    }

    function process_word_for_glossing(word_info, paragraph_shown_stems, global_stems, check_global) {
        var lookup_result = sb.dpd_lookup_json(word_info.word.toLowerCase());
        var results = [];
        try {
            results = JSON.parse(lookup_result);
        } catch (e) {
            console.error("Failed to parse lookup result:", e);
            return null;
        }

        // Skip if no results
        if (!results || results.length === 0) {
            return null;
        }

        // Get the stem from the first result
        var stem = results[0].word;
        var stem_clean = root.clean_stem(stem);

        // Skip common words
        if (root.is_common_word(stem)) {
            return null;
        }

        // Skip if already shown in this paragraph
        if (paragraph_shown_stems[stem_clean]) {
            return null;
        }

        // Skip if global deduplication is on and already shown
        if (check_global && global_stems[stem_clean]) {
            return null;
        }

        // Mark as shown
        paragraph_shown_stems[stem_clean] = true;
        if (check_global) {
            global_stems[stem_clean] = true;
        }

        return process_word(word_info.word, results, word_info.sentence);
    }

    // FIXME extract_words_with_context()
    // function process_paragraph_for_glossing(paragraph_text, paragraph_shown_stems, global_stems, check_global) {
    //     var words_with_context = root.extract_words_with_context(paragraph_text);
    //     var glossed_words = [];

    //     for (var i = 0; i < words_with_context.length; i++) {
    //         var processed_word = root.process_word_for_glossing(
    //             words_with_context[i],
    //             paragraph_shown_stems,
    //             global_stems,
    //             check_global,
    //         );

    //         if (processed_word) {
    //             glossed_words.push(processed_word);
    //         }
    //     }

    //     return glossed_words;
    // }

    function process_paragraph_for_glossing(paragraph_text, paragraph_shown_stems, global_stems, check_global) {
        var words = root.extract_words(paragraph_text);
        // console.log(words);
        var glossed_words = [];

        for (var i = 0; i < words.length; i++) {
            var processed_word = root.process_word_for_glossing(
                { word: words[i], sentence: "" },
                paragraph_shown_stems,
                global_stems,
                check_global,
            );

            if (processed_word) {
                glossed_words.push(processed_word);
            }
        }

        return glossed_words;
    }

    // Get previous paragraph stems for global deduplication
    function get_previous_paragraph_stems(up_to_index) {
        var previous_stems = {};

        for (var p = 0; p < up_to_index; p++) {
            var prev_para = paragraph_model.get(p);
            var prev_words = JSON.parse(prev_para.words_data);
            for (var w = 0; w < prev_words.length; w++) {
                previous_stems[root.clean_stem(prev_words[w].stem)] = true;
            }
        }

        return previous_stems;
    }

    function update_all_glosses() {
        var paragraphs = gloss_text_input.text.split('\n\n').filter(p => p.trim() !== '');
        root.current_text = gloss_text_input.text;
        paragraph_model.clear();
        root.global_shown_stems = {};

        for (var i = 0; i < paragraphs.length; i++) {
            var paragraph_shown_stems = {};
            var glossed_words = root.process_paragraph_for_glossing(
                paragraphs[i],
                paragraph_shown_stems,
                root.global_shown_stems,
                root.no_duplicates_globally,
            )

            paragraph_model.append({
                text: paragraphs[i],
                words_data: JSON.stringify(glossed_words),
            });
        }

        root.save_session();
    }

    function update_paragraph_gloss(index) {
        var paragraph = paragraph_model.get(index);
        var paragraph_shown_stems = {};

        // If global deduplication, collect stems from previous paragraphs
        var previous_stems = root.no_duplicates_globally ? root.get_previous_paragraph_stems(index) : {};

        var glossed_words = root.process_paragraph_for_glossing(
            paragraph.text,
            paragraph_shown_stems,
            previous_stems,
            root.no_duplicates_globally,
        );

        paragraph_model.setProperty(index, "words_data", JSON.stringify(glossed_words));
        root.save_session();
    }

    function load_session(db_id, gloss_data_json) {
        // FIXME
    }

    function update_word_selection(paragraphIndex, wordIndex, selectedIndex) {
        var paragraph = paragraph_model.get(paragraphIndex);
        var words = JSON.parse(paragraph.words_data);
        words[wordIndex].selectedIndex = selectedIndex;

        // Update stem for the new selection (keep original with numbers for display)
        words[wordIndex].stem = words[wordIndex].results[selectedIndex].word;

        /* FIXME binding loop paragraph_model.setProperty(paragraphIndex, "words_data", JSON.stringify(words)); */
        root.save_session();
    }

    function update_paragraph_text(index, new_text) {
        paragraph_model.setProperty(index, "text", new_text);
        root.save_session();
    }

    TabBar {
        id: tabBar
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right

        TabButton {
            text: "Gloss"
        }

        TabButton {
            text: "History"
        }
    }

    StackLayout {
        anchors.top: tabBar.bottom
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        currentIndex: tabBar.currentIndex

        // Gloss Tab
        ScrollView {
            contentWidth: availableWidth

            background: Rectangle {
                anchors.fill: parent
                border.width: 0
                color: root.bg_color
            }

            ColumnLayout {
                width: parent.width
                spacing: 20

                GroupBox {
                    id: main_gloss_input_group
                    Layout.fillWidth: true
                    Layout.margins: 10

                    background: Rectangle {
                        anchors.fill: parent
                        border.width: 1
                        border.color: root.border_color
                        radius: 5
                        color: root.bg_color_darker
                    }

                    ColumnLayout {
                        anchors.fill: parent

                        ScrollView {
                            Layout.fillWidth: true
                            Layout.preferredHeight: 200

                            TextArea {
                                id: gloss_text_input
                                font.pointSize: 12
                                placeholderText: "Enter paragraphs to gloss ..."
                                selectByMouse: true
                                wrapMode: TextEdit.WordWrap
                            }
                        }

                        RowLayout {
                            Layout.fillWidth: true

                            CheckBox {
                                id: globalDedupeCheckBox
                                text: "No duplicates"
                                checked: root.no_duplicates_globally
                                onCheckedChanged: {
                                    root.no_duplicates_globally = globalDedupeCheckBox.checked;
                                    if (paragraph_model.count > 0) {
                                        root.update_all_glosses();
                                    }
                                }
                            }

                            Item { Layout.fillWidth: true }

                            Button {
                                text: "Export as Anki CSV"
                                enabled: paragraph_model.count > 0
                                // onClicked: root.exportAnkiCSV() FIXME
                            }

                            Button {
                                text: "Common Words..."
                                // onClicked: commonWordsDialog.open() FIXME
                            }

                            Button {
                                id: update_all_glosses_btn
                                text: "Update All Glosses"
                                onClicked: root.update_all_glosses()
                            }
                        }
                    }
                }

                // Paragraph glosses
                Repeater {
                    model: paragraph_model
                    delegate: paragraph_gloss_component
                }

                Item {
                    Layout.fillHeight: true
                }
            }
        }

        // History Tab
        ScrollView {
            Label { text: "History" }
            // ListView {
            //     anchors.fill: parent
            //     model: history_model
            //     spacing: 10
            //     delegate: historyItemDelegate
            // }
        }
    }

    Component {
        id: paragraph_gloss_component

        ColumnLayout {
            id: paragraph_item
            /* anchors.fill: parent */

            required property int index
            required property string text
            required property string words_data

            GroupBox {
                Layout.fillWidth: true
                Layout.margins: 10
                title: "Paragraph " + (paragraph_item.index + 1)

                background: Rectangle {
                    anchors.fill: parent
                    color: root.bg_color_darker
                    border.width: 1
                    border.color: root.border_color
                    radius: 5
                }

                ColumnLayout {
                    anchors.fill: parent

                    ScrollView {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 100

                        TextArea {
                            text: paragraph_item.text
                            font.pointSize: 12
                            selectByMouse: true
                            wrapMode: TextEdit.WordWrap
                            onTextChanged: {
                                if (text !== paragraph_item.text) {
                                    root.update_paragraph_text(paragraph_item.index, text);
                                }
                            }
                        }
                    }

                    Button {
                        text: "Update Gloss"
                        Layout.alignment: Qt.AlignRight
                        onClicked: root.update_paragraph_gloss(paragraph_item.index)
                    }
                }
            }

            ColumnLayout {
                spacing: 10
                Layout.margins: 10

                Item {
                    id: vocabulary_gloss
                    Layout.fillWidth: true
                    Layout.preferredHeight: Math.min(600, wordListView.contentHeight + 40)

                    ListView {
                        id: wordListView
                        anchors.fill: parent
                        clip: true
                        spacing: 5

                        model: {
                            try {
                                return JSON.parse(paragraph_item.words_data);
                            } catch (e) {
                                return [];
                            }
                        }

                        delegate: wordItemDelegate

                        property int paragraphIndex: paragraph_item.index
                    }
                }

                Component {
                    id: wordItemDelegate

                    ItemDelegate {
                        id: wordItem
                        width: parent ? parent.width : 0
                        height: root.vocab_tm1.height * 3

                        required property int index
                        required property var modelData

                        property int paragraphIndex: wordListView.paragraphIndex


                        Frame {
                            id: mainContent
                            anchors.fill: parent
                            padding: 4

                            background: Rectangle {
                                border.width: 0
                                color: (wordItem.index % 2 === 0 ?  root.bg_color_lighter : root.bg_color)
                            }

                            RowLayout {
                                anchors.fill: parent
                                spacing: 10

                                ComboBox {
                                    id: word_select
                                    Layout.alignment: Qt.AlignTop
                                    Layout.preferredWidth: wordItem.width * 0.2
                                    visible: wordItem.modelData.results && wordItem.modelData.results.length > 1
                                    model: wordItem.modelData.results
                                    textRole: "word"
                                    font.bold: true
                                    font.pointSize: root.vocab_font_point_size
                                    currentIndex: wordItem.modelData.selectedIndex || 0
                                    onCurrentIndexChanged: {
                                        if (currentIndex !== wordItem.modelData.selectedIndex) {
                                            root.update_word_selection(wordItem.paragraphIndex,
                                                                        wordItem.index,
                                                                        currentIndex);
                                        }
                                    }
                                }

                                Text {
                                    Layout.preferredWidth: wordItem.width * 0.2
                                    Layout.fillHeight: true
                                    verticalAlignment: Text.AlignTop
                                    visible: !wordItem.modelData.results || wordItem.modelData.results.length <= 1
                                    text: wordItem.modelData.results && wordItem.modelData.results.length > 0 ?
                                                wordItem.modelData.results[0].word : wordItem.modelData.original_word
                                    color: root.text_color
                                    font.bold: true
                                    font.pointSize: root.vocab_font_point_size
                                    wrapMode: TextEdit.WordWrap
                                }

                                RowLayout {
                                    Layout.preferredWidth: wordItem.width * 0.8
                                    Layout.fillHeight: true

                                    Text {
                                        Layout.fillHeight: true
                                        Layout.fillWidth: true
                                        verticalAlignment: Text.AlignTop
                                        text: {
                                            if (wordItem.modelData.results && wordItem.modelData.results.length > 0) {
                                                var idx = word_select.currentIndex || 0;
                                                return wordItem.modelData.results[idx].summary || "No summary";
                                            }
                                            return "No summary";
                                        }
                                        color: root.text_color
                                        font.pointSize: root.vocab_font_point_size
                                        wrapMode: TextEdit.WordWrap
                                        textFormat: Text.RichText
                                    }

                                    Button {
                                        icon.source: "icons/32x32/bxs_book_content.png"
                                        Layout.preferredHeight: word_select.height
                                        Layout.preferredWidth: word_select.height
                                        Layout.alignment: Qt.AlignTop
                                        onClicked: {
                                            var idx = word_select.currentIndex || 0;
                                            let word = wordItem.modelData.results && wordItem.modelData.results.length > 0 ?
                                                wordItem.modelData.results[idx].word : wordItem.modelData.original_word;
                                            root.handle_open_dict_tab_fn(word + "/dpd"); // qmllint disable use-proper-function
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
