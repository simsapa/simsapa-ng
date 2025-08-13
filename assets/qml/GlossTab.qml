pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs

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
    PromptManager { id: pm }

    Connections {
        target: pm

        function onPromptResponse (paragraph_idx: int, translation_idx: int, model_name: string, response: string) {
            let paragraph = paragraph_model.get(paragraph_idx);
            let translations = [];
            if (paragraph.translations_json) {
                try {
                    translations = JSON.parse(paragraph.translations_json);
                } catch (e) {
                    console.error("Failed to parse paragraph.translations_json:", e);
                }
            } else {
                console.error(`Missing paragraph.translations_json for paragraph_idx ${paragraph_idx}, translation_idx ${translation_idx}`);
            }
            let item = {
                model_name: model_name,
                status: "completed",
                response: response,
            };
            translations[translation_idx] = item;
            let translations_json = JSON.stringify(translations);
            paragraph_model.setProperty(paragraph_idx, "translations_json", translations_json);
            paragraph_model_export.setProperty(paragraph_idx, "translations_json", translations_json);
        }
    }

    property string translation_prompt_template: `Translate the following Pāli passage to English, keeping in mind the provided dictionary definitions.

Pāli passage:

<<PALI_PASSAGE>>

Dictionary definitions:

<<DICTIONARY_DEFINITIONS>>

Respond with only the translation of the Pāli passage.
`

    property list<var> translation_models_init: [
        { model_name: "tngtech/deepseek-r1t2-chimera:free", enabled: true },
        { model_name: "deepseek/deepseek-r1-0528:free", enabled: false },
        { model_name: "deepseek/deepseek-chat-v3-0324:free", enabled: false },
        { model_name: "google/gemini-2.0-flash-exp:free", enabled: false },
        { model_name: "google/gemma-3-12b-it:free", enabled: true },
        { model_name: "google/gemma-3-27b-it:free", enabled: false },
        { model_name: "openai/gpt-oss-20b:free", enabled: false },
        { model_name: "meta-llama/llama-3.3-70b-instruct:free", enabled: false },
        { model_name: "meta-llama/llama-3.1-405b-instruct:free", enabled: true },
        { model_name: "mistralai/mistral-small-3.2-24b-instruct:free", enabled: true },
    ]

    ListModel { id: translation_models }

    function load_translation_models() {
        for (var i = 0; i < root.translation_models_init.length; i++) {
            var item = root.translation_models_init[i];
            translation_models.append(item);
        }
    }

    // Current session data
    property string current_session_id: ""
    property string current_text: ""

    // Common words to filter out
    property var common_words: []

    // Global deduplication option
    property bool no_duplicates_globally: true
    property bool skip_common: true

    // Track globally shown stem words
    property var global_shown_stems: ({})

    // Stores recent glossing sessions
    ListModel { id: history_model }

    // Gloss data per paragraph
    ListModel { id: paragraph_model }
    // Saving changes here to avoid binding loop with paragraph_model
    ListModel { id: paragraph_model_export }

    Component.onCompleted: {
        load_history();
        load_common_words();
        load_translation_models();
        if (root.is_qml_preview) {
            qml_preview_state();
        }
    }

    FolderDialog {
        id: export_folder_dialog
        acceptLabel: "Export to Folder"
        onAccepted: root.export_dialog_accepted()
    }

    function export_dialog_accepted() {
        if (export_btn.currentIndex === 0) return;
        let save_file_name = null
        let save_content = null;

        if (export_btn.currentValue === "HTML") {
            save_file_name = "gloss_export.html";
            save_content = root.gloss_as_html();

        } else if (export_btn.currentValue === "Markdown") {
            save_file_name = "gloss_export.md";
            save_content = root.gloss_as_markdown();

        } else if (export_btn.currentValue === "Org-Mode") {
            save_file_name = "gloss_export.org";
            save_content = root.gloss_as_orgmode();
        }

        let save_fn = function() {
            let ok = sb.save_file(export_folder_dialog.selectedFolder, save_file_name, save_content);
            if (ok) {
                msg_dialog_ok.text = "Export completed."
                msg_dialog_ok.open();
            } else {
                msg_dialog_ok.text = "Export failed."
                msg_dialog_ok.open();
            }
        };

        if (save_file_name) {
            let exists = sb.check_file_exists_in_folder(export_folder_dialog.selectedFolder, save_file_name);
            if (exists) {
                msg_dialog_cancel_ok.text = `${save_file_name} exists. Overwrite?`;
                msg_dialog_cancel_ok.accept_fn = save_fn;
                msg_dialog_cancel_ok.open();
            } else {
                save_fn();
            }
        }

        // set the button back to default
        export_btn.currentIndex = 0;
    }

    MessageDialog {
        id: msg_dialog_ok
        buttons: MessageDialog.Ok
    }

    MessageDialog {
        id: msg_dialog_cancel_ok
        buttons: MessageDialog.Cancel | MessageDialog.Ok
        property var accept_fn: {}
        onAccepted: accept_fn() // qmllint disable use-proper-function
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
            if (paragraph.words_data_json) {
                words = JSON.parse(paragraph.words_data_json);
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
        return text.replace(/\n/g, ' ').split(' ').filter(i => i.length != 0) || [];
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
        var lookup_results_json = sb.dpd_lookup_json(word_info.word.toLowerCase());
        var results = [];
        try {
            results = JSON.parse(lookup_results_json);
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
        if (root.skip_common && root.is_common_word(stem)) {
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
            var prev_words = JSON.parse(prev_para.words_data_json);
            for (var w = 0; w < prev_words.length; w++) {
                previous_stems[root.clean_stem(prev_words[w].stem)] = true;
            }
        }

        return previous_stems;
    }

    function dictionary_definitions(glossed_words: var): string {
        let out = "";
        for (var i = 0; i < glossed_words.length; i++) {
            var w = glossed_words[i];
            var summary = summary_strip_html(w.results[w.selected_index].summary);
            var def = `- ${w.original_word}: stem '${clean_stem(w.stem)}', ${summary}\n`;
            out += def;
        }
        return out;
    }

    function update_all_glosses() {
        var paragraphs = gloss_text_input.text.split('\n\n').filter(p => p.trim() !== '');
        root.current_text = gloss_text_input.text;
        paragraph_model.clear();
        paragraph_model_export.clear();
        root.global_shown_stems = {};

        let translations_json = "[]";

        if (root.is_qml_preview) {
            let translations = [
                { model_name: "deepseek/deepseek-r1-0528:free",
                  status: "completed",
                  response: `
And what, bhikkhus, is concentration?

And what, bhikkhus, is concentration?

And what, bhikkhus, is concentration?

And what, bhikkhus, is concentration?

And what, bhikkhus, is concentration?

And what, bhikkhus, is concentration?

And what, bhikkhus, is concentration?
`,
                },
                { model_name: "google/gemini-2.0-flash-exp:free", status: "waiting", response: "" },
                { model_name: "google/gemma-3-27b-it:free", status: "completed", response: "And what, bhikkhus, is collectedness?" },
                { model_name: "meta-llama/llama-3.3-70b-instruct:free", status: "completed", response: "And what, bhikkhus, is the faculty of concentration?" },
            ];
            translations_json = JSON.stringify(translations);
        }

        for (var i = 0; i < paragraphs.length; i++) {
            var paragraph_shown_stems = {};
            var glossed_words = root.process_paragraph_for_glossing(
                paragraphs[i],
                paragraph_shown_stems,
                root.global_shown_stems,
                root.no_duplicates_globally,
            );

            var item = {
                text: paragraphs[i],
                words_data_json: JSON.stringify(glossed_words),
                translations_json: translations_json,
            };
            paragraph_model.append(item);
            paragraph_model_export.append(item);
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

        var glossed_words_json = JSON.stringify(glossed_words);
        paragraph_model.setProperty(index, "words_data_json", glossed_words_json);
        paragraph_model_export.setProperty(index, "words_data_json", glossed_words_json);
        root.save_session();
    }

    function load_session(db_id, gloss_data_json) {
        // FIXME
    }

    function update_word_selection(paragraph_idx: int, word_idx: int, selected_idx: int) {
        var paragraph = paragraph_model.get(paragraph_idx);
        var words = JSON.parse(paragraph.words_data_json);
        words[word_idx].selected_index = selected_idx;

        // Update stem for the new selection (keep original with numbers for display)
        words[word_idx].stem = words[word_idx].results[selected_idx].word;

        // Not saving to paragraph_model to avoid binding loop
        paragraph_model_export.setProperty(paragraph_idx, "words_data_json", JSON.stringify(words));
        root.save_session();
    }

    function update_paragraph_text(index, new_text) {
        paragraph_model.setProperty(index, "text", new_text);
        paragraph_model_export.setProperty(index, "text", new_text);
        root.save_session();
    }

    function summary_strip_html(text: string): string {
        text = text
            .replace(/<i>/g, "")
            .replace(/<\/i>/g, "")
            .replace(/<b>/g, "")
            .replace(/<\/b>/g, "");
        return text;
    }

    function summary_html_to_md(text: string): string {
        text = text
            .replace(/\*/g, "&ast;") // escape asterisks in the text
            .replace(/<i>/g, "*")
            .replace(/<\/i>/g, "*")
            .replace(/<b>/g, "**")
            .replace(/<\/b>/g, "**");
        return text;
    }

    function summary_html_to_orgmode(text: string): string {
        text = text
            .replace(/\*/g, "&ast;") // escape asterisks in the text
            .replace(/<i>/g, "/")
            .replace(/<\/i>/g, "/")
            .replace(/<b>/g, "*")
            .replace(/<\/b>/g, "*");
        return text;
    }

    function gloss_export_data(): var {
        // paragraph_model_export:
        // {
        //     text: paragraphs[i],
        //     words_data_json: JSON.stringify(glossed_words),
        // }
        //
        // words_data:
        // {
        //     original_word: word,
        //     results: lookup_results,
        //     selected_index: 0,
        //     stem: lookup_results[0].word,
        //     example_sentence: sentence || "",
        // }
        //
        // results (Vec<LookupResult>):
        // {
        //     uid: String,
        //     word: String,
        //     summary: String,
        // }
        //
        // Returns:
        // {
        //     text: "...",
        //     paragraphs: [
        //         {
        //             text: "...",
        //             vocabulary: [
        //                 {
        //                     word: "...",
        //                     summary: "...",
        //                 }
        //             ]
        //         }
        //     ]
        // }

        let gloss_data = {
            text: gloss_text_input.text.trim(),
            paragraphs: [],
        };

        for (var i = 0; i < paragraph_model_export.count; i++) {
            var paragraph = paragraph_model_export.get(i);

            var para_data = {
                text: paragraph.text.trim(),
                vocabulary: [],
            };

            var words_data = JSON.parse(paragraph.words_data_json);
            if (!words_data || words_data.length == 0) continue;

            for (var j = 0; j < words_data.length; j++) {
                var w_data = words_data[j];
                if (!w_data.results || w_data.results.length == 0) continue;

                // Add one line of word vocabulary info.
                // For each word, export only the selected result.
                para_data.vocabulary.push(w_data.results[w_data.selected_index]);
            }

            gloss_data.paragraphs.push(para_data);
        }

        return gloss_data;
    }

    function gloss_as_html(): string {
        let gloss_data = root.gloss_export_data();

        let main_text = "\n<blockquote>\n" + gloss_data.text.replace(/\n/g, "<br>\n") + "\n</blockquote>\n";

        let out = `
<!doctype html>
<html>
<head>
    <meta charset="utf-8">
    <meta http-equiv="x-ua-compatible" content="ie=edge">
    <title>Gloss Export</title>
    <meta name="viewport" content="width=device-width, initial-scale=1">
</head>
<body>
<h1>Gloss Export</h1>

${main_text}

<h2>Paragraphs</h2>
`;

        for (var i = 0; i < gloss_data.paragraphs.length; i++) {
            var paragraph = gloss_data.paragraphs[i];
            let para_text = "\n<blockquote>\n" + paragraph.text.replace(/\n/g, "<br>\n") + "\n</blockquote>\n";

            var table_rows = "";
            for (var j = 0; j < paragraph.vocabulary.length; j++) {
                var res = paragraph.vocabulary[j];
                table_rows += `<tr><td> <b>${res.word}</b> </td><td> ${res.summary} </td></tr>\n`;
            }

            out += `
${para_text}

<table><tbody>
${table_rows}
</tbody></table>
`;

        }

        out += "\n</body>\n</html>";
        return out.trim().replace(/\n\n\n+/g, "\n\n");
    }

    function gloss_as_markdown(): string {
        let gloss_data = root.gloss_export_data();

        // The main gloss text in a quote
        let main_text = "\n> " + gloss_data.text.replace(/\n/g, "\n> ");

        let out = `
# Gloss Export

${main_text}

## Paragraphs
`;

        for (var i = 0; i < gloss_data.paragraphs.length; i++) {
            var paragraph = gloss_data.paragraphs[i];
            // Add each paragraph text in a quote
            var para_text = "\n> " + paragraph.text.replace(/\n/g, "\n> ");

            var table_rows = "";
            for (var j = 0; j < paragraph.vocabulary.length; j++) {
                var res = paragraph.vocabulary[j];
                var summary = root.summary_html_to_md(res.summary);
                table_rows += `| **${res.word}** | ${summary} |\n`;
            }

            // Add the table header for syntax recognition, but leave empty to save space when rendered.
            out += `
${para_text}

|    |    |
|----|----|
${table_rows}
`;

        }

        // only two new lines for paragraph breaks
        return out.trim().replace(/\n\n\n+/g, "\n\n");
    }

    function gloss_as_orgmode(): string {
        let gloss_data = root.gloss_export_data();

        let main_text = "\n#+begin_quote\n" + gloss_data.text + "\n#+end_quote\n";

        let out = `
* Gloss Export

${main_text}

** Paragraphs
`;

        for (var i = 0; i < gloss_data.paragraphs.length; i++) {
            var paragraph = gloss_data.paragraphs[i];
            let para_text = "\n#+begin_quote\n" + paragraph.text + "\n#+end_quote\n";

            var table_rows = "";
            for (var j = 0; j < paragraph.vocabulary.length; j++) {
                var res = paragraph.vocabulary[j];
                var summary = root.summary_html_to_orgmode(res.summary);
                table_rows += `| *${res.word}* | ${summary} |\n`;
            }

            out += `
${para_text}

${table_rows}
`;

        }

        return out.trim().replace(/\n\n\n+/g, "\n\n");
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

                            CheckBox {
                                id: skip_common_check
                                text: "Skip common"
                                checked: root.skip_common
                                onCheckedChanged: {
                                    root.skip_common = skip_common_check.checked;
                                    if (paragraph_model.count > 0) {
                                        root.update_all_glosses();
                                    }
                                }
                            }

                            Item { Layout.fillWidth: true }

                            ComboBox {
                                id: export_btn
                                model: ["Export As...", "HTML", "Markdown", "Org-Mode"]
                                enabled: paragraph_model.count > 0
                                onCurrentIndexChanged: {
                                    if (export_btn.currentIndex !== 0) {
                                        export_folder_dialog.open();
                                    }
                                }
                            }

                            Button {
                                text: "Common Words..."
                                onClicked: commonWordsDialog.open()
                            }

                            Button {
                                id: update_all_glosses_btn
                                text: "Update All Glosses"
                                onClicked: root.update_all_glosses()
                            }
                        }
                    }
                }

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
            required property string words_data_json
            required property string translations_json

            property bool is_collapsed: collapse_btn.checked

            RowLayout {
                Layout.leftMargin: 10

                Button {
                    id: collapse_btn
                    checkable: true
                    checked: false
                    icon.source: checked ? "icons/32x32/fa_plus-solid.png" : "icons/32x32/fa_minus-solid.png"
                    Layout.alignment: Qt.AlignLeft
                    Layout.preferredWidth: collapse_btn.height
                }

                Label {
                    text: "Paragraph " + (paragraph_item.index + 1)
                    font.pointSize: root.vocab_font_point_size
                }
            }

            ColumnLayout {
                visible: !collapse_btn.checked

                GroupBox {
                    Layout.fillWidth: true
                    Layout.margins: 10

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

                        RowLayout {
                            Layout.alignment: Qt.AlignRight

                            Button {
                                text: "AI-Translate"
                                Layout.alignment: Qt.AlignRight
                                onClicked: {
                                    let paragraph = paragraph_model_export.get(paragraph_item.index);

                                    let glossed_words = JSON.parse(paragraph.words_data_json);

                                    let prompt = root.translation_prompt_template
                                        .replace("<<PALI_PASSAGE>>", paragraph_item.text)
                                        .replace("<<DICTIONARY_DEFINITIONS>>", root.dictionary_definitions(glossed_words));

                                    let translations = [];

                                    for (var i = 0; i < translation_models.count; i++) {
                                        var item = translation_models.get(i);
                                        if (item.enabled) {
                                            pm.prompt_request(paragraph_item.index, i, item.model_name, prompt);
                                            translations.push({
                                                model_name: item.model_name,
                                                status: "waiting",
                                                response: "",
                                            });
                                        }
                                    }

                                    let translations_json = JSON.stringify(translations);
                                    paragraph_model.setProperty(paragraph_item.index, "translations_json", translations_json);
                                    paragraph_model_export.setProperty(paragraph_item.index, "translations_json", translations_json);
                                }
                            }

                            Button {
                                text: "Update Gloss"
                                Layout.alignment: Qt.AlignRight
                                onClicked: root.update_paragraph_gloss(paragraph_item.index)
                            }
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
                                    return JSON.parse(paragraph_item.words_data_json);
                                } catch (e) {
                                    return [];
                                }
                            }

                            delegate: wordItemDelegate

                            property int paragraph_index: paragraph_item.index
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

                            property int paragraph_index: wordListView.paragraph_index


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
                                        currentIndex: wordItem.modelData.selected_index || 0
                                        onCurrentIndexChanged: {
                                            if (currentIndex !== wordItem.modelData.selected_index) {
                                                root.update_word_selection(wordItem.paragraph_index,
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

                Repeater {
                    model: {
                        try {
                            return JSON.parse(paragraph_item.translations_json);
                        } catch (e) {
                            return [];
                        }
                    }
                    delegate: translation_delegate
                }

                Component {
                    id: translation_delegate
                    Rectangle {
                        id: tr_item
                        required property int index
                        required property string model_name
                        required property string status
                        required property string response

                        Layout.minimumHeight: (!tr_item_collapse_btn.checked && status === "completed") ? 150 : 50
                        // FIXME length and status check shouldn't be necessary, but duplicate items are showing up
                        visible: tr_item.model_name.length != 0 && status === "completed"

                        ColumnLayout {
                            GroupBox {
                                Layout.preferredWidth: vocabulary_gloss.width
                                Layout.fillWidth: true
                                Layout.margins: 10

                                ColumnLayout {
                                    anchors.fill: parent
                                    id: tr_col

                                    RowLayout {
                                        Button {
                                            id: tr_item_collapse_btn
                                            checkable: true
                                            checked: false
                                            icon.source: checked ? "icons/32x32/fa_plus-solid.png" : "icons/32x32/fa_minus-solid.png"
                                            Layout.alignment: Qt.AlignLeft
                                            Layout.preferredWidth: tr_item_collapse_btn.height
                                        }

                                        Text {
                                            text: `(${tr_item.model_name}: ${tr_item.status})`
                                            font.pointSize: 10
                                            font.bold: true
                                        }
                                    }

                                    ScrollView {
                                        Layout.fillWidth: true
                                        Layout.preferredHeight: 100
                                        visible: !tr_item_collapse_btn.checked && tr_item.status === "completed"

                                        ScrollBar.vertical.policy: ScrollBar.AlwaysOn

                                        TextArea {
                                            text: "<p>" + tr_item.response.trim().replace(/\n/g, "<br>") + "</p>"
                                            font.pointSize: root.vocab_font_point_size
                                            selectByMouse: true
                                            readOnly: true
                                            textFormat: Text.RichText
                                            wrapMode: TextEdit.WordWrap
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

    Dialog {
        id: commonWordsDialog
        title: "Edit Common Words"
        width: 400
        height: 500
        anchors.centerIn: parent

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 10

            Label {
                text: "Enter common words (one per line):"
            }

            ScrollView {
                Layout.fillWidth: true
                Layout.fillHeight: true

                TextArea {
                    id: commonWordsTextArea
                    selectByMouse: true
                    text: root.common_words.join('\n')
                }
            }

            RowLayout {
                Layout.fillWidth: true

                Button {
                    text: "Cancel"
                    onClicked: commonWordsDialog.close()
                }

                Button {
                    text: "Save"
                    onClicked: {
                        var words = commonWordsTextArea.text.split('\n')
                            .map(w => w.trim().toLowerCase())
                            .filter(w => w.length > 0);
                        root.common_words = words;
                        sb.save_common_words_json(JSON.stringify(root.common_words));
                        commonWordsDialog.close();
                        root.update_all_glosses();
                    }
                }
            }
        }
    }
}
