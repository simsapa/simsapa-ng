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
    required property bool ai_models_auto_retry

    readonly property bool is_mobile: Qt.platform.os === "android" || Qt.platform.os === "ios"
    readonly property bool is_desktop: !root.is_mobile
    readonly property bool is_qml_preview: Qt.application.name === "Qml Runtime"

    readonly property int vocab_font_point_size: 10
    readonly property TextMetrics vocab_tm1: TextMetrics { text: "#"; font.pointSize: root.vocab_font_point_size }

    property alias gloss_text_input: gloss_text_input
    property alias paragraph_model: paragraph_model

    property var handle_open_dict_tab_fn

    property string text_color: root.is_dark ? "#F0F0F0" : "#000000"
    property string bg_color: root.is_dark ? "#23272E" : "#FAE6B2"
    property string bg_color_lighter: root.is_dark ? "#2E333D" : "#FBEDC7"
    property string bg_color_darker: root.is_dark ? "#1C2025" : "#F8DA8E"

    property string border_color: root.is_dark ? "#0a0a0a" : "#ccc"

    Logger { id: logger }
    PromptManager { id: pm }

    Connections {
        target: pm

        function onPromptResponse (paragraph_idx: int, translation_idx: int, model_name: string, response: string) {
            logger.debug(`🤖 onPromptResponse received: paragraph_idx=${paragraph_idx}, translation_idx=${translation_idx}, model_name=${model_name}`);
            logger.debug(`📝 Response content: "${response.substring(0, 100)}..."`);

            let paragraph = paragraph_model.get(paragraph_idx);
            if (!paragraph) {
                logger.error(`❌ No paragraph found at index ${paragraph_idx}`);
                return;
            }

            let translations = [];
            if (paragraph.translations_json) {
                try {
                    translations = JSON.parse(paragraph.translations_json);
                    logger.debug(`📚 Parsed ${translations.length} existing translations`);
                } catch (e) {
                    logger.error("Failed to parse paragraph.translations_json:", e);
                }
            } else {
                logger.error(`Missing paragraph.translations_json for paragraph_idx ${paragraph_idx}, translation_idx ${translation_idx}`);
            }

            if (translation_idx < translations.length) {
                let is_error = root.is_error_response(response);
                let current_retry_count = translations[translation_idx].retry_count || 0;

                logger.debug(`🔄 Updating translation at index ${translation_idx}: is_error=${is_error}, retry_count=${current_retry_count}`);

                // Update the existing translation entry
                translations[translation_idx].response = response;
                translations[translation_idx].status = is_error ? "error" : "completed";
                translations[translation_idx].last_updated = Date.now();

                logger.debug(`✅ Updated translation data:`, JSON.stringify(translations[translation_idx]));

                // Handle automatic retry for errors (up to 5 times)
                if (is_error && current_retry_count < 5 && root.ai_models_auto_retry && !root.is_rate_limit_error(response)) {
                    logger.debug(`🔁 Scheduling automatic retry for ${model_name}`);
                    // Schedule automatic retry
                    Qt.callLater(function() {
                        root.handle_retry_request(paragraph_idx, model_name, root.generate_request_id());
                    });
                } else if (is_error && root.is_rate_limit_error(response)) {
                    logger.debug(`⏸️  Skipping auto-retry for rate limit error: ${model_name}`);
                } else if (is_error && !root.ai_models_auto_retry) {
                    logger.debug(`⏸️  Auto-retry disabled, not retrying: ${model_name}`);
                }

                let translations_json = JSON.stringify(translations);
                paragraph_model.setProperty(paragraph_idx, "translations_json", translations_json);
                logger.debug(`💾 Saved translations_json to paragraph model`);
            } else {
                logger.error(`❌ translation_idx ${translation_idx} is out of bounds for ${translations.length} translations`);
            }
        }
    }

    property alias translation_models: translation_models

    ListModel { id: translation_models }

    function load_translation_models() {
        logger.debug(`🔄 Loading translation models from all providers...`);
        translation_models.clear();
        let providers_json = SuttaBridge.get_providers_json();
        logger.debug(`📥 Raw providers JSON: "${providers_json}"`);
        try {
            let providers_array = JSON.parse(providers_json);
            logger.debug(`📊 Parsing ${providers_array.length} providers`);
            for (var i = 0; i < providers_array.length; i++) {
                var provider = providers_array[i];
                logger.debug(`  Provider ${provider.name}: enabled=${provider.enabled}`);

                // Only load models from enabled providers
                if (provider.enabled) {
                    for (var j = 0; j < provider.models.length; j++) {
                        var model = provider.models[j];
                        logger.debug(`    [${j}] ${model.model_name}: enabled=${model.enabled}`);
                        translation_models.append({
                            model_name: model.model_name,
                            enabled: model.enabled,
                            removable: model.removable
                        });
                    }
                } else {
                    logger.debug(`    Skipping disabled provider ${provider.name}`);
                }
            }
            logger.debug(`🎯 Total models loaded: ${translation_models.count}`);
        } catch (e) {
            logger.error("Failed to parse providers JSON:", e);
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

    // Single paragraph model with nested word models
    ListModel {
        id: paragraph_model
        // Each paragraph item contains:
        // - text: string (paragraph text)
        // - words_data: Array (vocabulary words data)
        // - translations_json: string (keep JSON for external API data)
    }

    Component.onCompleted: {
        load_history();
        load_common_words();
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
            let ok = SuttaBridge.save_file(export_folder_dialog.selectedFolder, save_file_name, save_content);
            if (ok) {
                msg_dialog_ok.text = "Export completed."
                msg_dialog_ok.open();
            } else {
                msg_dialog_ok.text = "Export failed."
                msg_dialog_ok.open();
            }
        };

        if (save_file_name) {
            let exists = SuttaBridge.check_file_exists_in_folder(export_folder_dialog.selectedFolder, save_file_name);
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
        var saved_words = SuttaBridge.get_common_words_json();
        if (saved_words) {
            try {
                root.common_words = JSON.parse(saved_words);
            } catch (e) {
                logger.error("Failed to parse common words:", e);
            }
        }
    }

    function save_common_words() {
        SuttaBridge.save_common_words_json(JSON.stringify(root.common_words));
    }

    function generate_request_id() {
        return Date.now().toString() + "_" + Math.random().toString(36);
    }

    function is_error_response(response_text) {
        return response_text.includes("API Error:") ||
               response_text.includes("Error:") ||
               response_text.includes("Failed:");
    }

    function is_rate_limit_error(response_text) {
        return response_text.includes("API Error: Rate limit exceeded");
    }

    ScrollableHelper {
        id: scroll_helper
        target_scroll_view: main_scroll_view
    }

    function handle_retry_request(paragraph_idx, model_name, new_request_id) {
        var paragraph = paragraph_model.get(paragraph_idx);
        if (!paragraph || !paragraph.translations_json) return;

        try {
            var translations = JSON.parse(paragraph.translations_json);
            for (var i = 0; i < translations.length; i++) {
                if (translations[i].model_name === model_name) {
                    // Update the translation entry for retry
                    translations[i].request_id = new_request_id;
                    translations[i].status = "waiting";
                    translations[i].retry_count = (translations[i].retry_count || 0) + 1;
                    translations[i].last_updated = Date.now();

                    // Append retry message to response
                    var retry_msg = `\n\nRetrying... (${translations[i].retry_count}x)`;
                    if (translations[i].response && !translations[i].response.includes("Retrying...")) {
                        translations[i].response += retry_msg;
                    }

                    // Update the model
                    paragraph_model.setProperty(paragraph_idx, "translations_json", JSON.stringify(translations));

                    // Send new request with system prompt
                    let system_prompt = SuttaBridge.get_system_prompt("Gloss Tab: System Prompt");
                    var template = SuttaBridge.get_system_prompt("Gloss Tab: AI Translation");
                    var user_prompt = template
                        .replace("<<PALI_PASSAGE>>", paragraph.text)
                        .replace("<<DICTIONARY_DEFINITIONS>>", root.dictionary_definitions_from_paragraph(paragraph));

                    // Combine system prompt with user prompt
                    var combined_prompt = user_prompt;
                    if (system_prompt && system_prompt.trim() !== "") {
                        combined_prompt = system_prompt + "\n\n" + user_prompt;
                    }

                    let provider_name = SuttaBridge.get_provider_for_model(model_name);
                    pm.prompt_request(paragraph_idx, i, provider_name, model_name, combined_prompt);
                    break;
                }
            }
        } catch (e) {
            logger.error("Failed to handle retry request:", e);
        }
    }

    function update_tab_selection(paragraph_idx, tab_index, model_name) {
        // Just update the selected tab index without modifying translations_json to avoid binding loop
        var paragraph = paragraph_model.get(paragraph_idx);
        if (paragraph) {
            // Store the selected tab index directly in the paragraph item
            paragraph_model.setProperty(paragraph_idx, "selected_ai_tab", tab_index);
            // Don't modify translations_json here to avoid binding loops
            // The export functions will use selected_ai_tab to determine which translation is selected
        }
    }

    function load_history() {
        history_model.clear()
        var history_json = SuttaBridge.get_gloss_history_json();
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
                logger.error("Failed to parse history:", e);
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
            var words_data = [];

            if (paragraph.words_data_json) {
                try {
                    words_data = JSON.parse(paragraph.words_data_json);
                } catch (e) {
                    logger.error("Failed to parse words_data_json:", e);
                }
            }

            gloss_data.paragraphs.push({
                text: paragraph.text,
                words: words_data,
            });
        }

        if (root.current_session_id) {
            SuttaBridge.update_gloss_session(root.current_session_id, JSON.stringify(gloss_data));
        } else {
            root.current_session_id = SuttaBridge.save_new_gloss_session(JSON.stringify(gloss_data));
        }

        root.load_history();
    }

    function extract_words_with_context(text: string): list<var> {
        var sentences = text.match(/[^.!?]+[.!?]+/g) || [text];
        var words_with_context = [];

        for (var i = 0; i < sentences.length; i++) {
            var sentence = sentences[i].trim();
            var words = SuttaBridge.extract_words(sentence);

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

    function clean_word(word: string): string {
        // NOTE: QML \w doesn't include accented Pāli letters.
        return word
            .toLowerCase()
            .replace(/^[^\wāīūṃṁṅñṭḍṇḷṛṣś]+/, '')
            .replace(/[^\wāīūṃṁṅñṭḍṇḷṛṣś]+$/, '');
    }

    function is_common_word(stem: string): bool {
        return root.common_words.includes(clean_stem(stem));
    }

    function create_word_model_item(word: string, lookup_results, sentence: string): var {
        return {
            original_word: clean_word(word),
            results: lookup_results,
            selected_index: 0,
            stem: lookup_results[0].word,
            example_sentence: sentence || "",
        };
    }

    function create_paragraph_with_words_model(text: string): var {
        return {
            text: text,
            words_data: [],
            translations_json: "[]"
        };
    }

    function get_word_from_data(paragraph_idx: int, word_idx: int): var {
        var paragraph = paragraph_model.get(paragraph_idx);
        if (!paragraph || !paragraph.words_data_json) return null;

        try {
            var words_data = JSON.parse(paragraph.words_data_json);
            if (word_idx >= words_data.length) return null;
            return words_data[word_idx];
        } catch (e) {
            logger.error("Failed to parse words_data_json:", e);
            return null;
        }
    }

    function update_word_in_data(paragraph_idx: int, word_idx: int, property_name: string, value): void {
        var paragraph = paragraph_model.get(paragraph_idx);
        if (!paragraph || !paragraph.words_data_json) return;

        try {
            var words_data = JSON.parse(paragraph.words_data_json);
            if (word_idx < words_data.length) {
                words_data[word_idx][property_name] = value;
                paragraph_model.setProperty(paragraph_idx, "words_data_json", JSON.stringify(words_data));
            }
        } catch (e) {
            logger.error("Failed to parse words_data_json:", e);
        }
    }

    function process_word_for_glossing(word_info, paragraph_shown_stems, global_stems, check_global) {
        var lookup_results_json = SuttaBridge.dpd_lookup_json(word_info.word.toLowerCase());
        var results = [];
        try {
            results = JSON.parse(lookup_results_json);
        } catch (e) {
            logger.error("Failed to parse lookup result:", e);
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

        return create_word_model_item(word_info.word, results, word_info.sentence);
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

    function populate_paragraph_words_data(paragraph_item, paragraph_text, paragraph_shown_stems, global_stems, check_global) {
        paragraph_item.words_data = [];
        var words = SuttaBridge.extract_words(paragraph_text);

        for (var i = 0; i < words.length; i++) {
            var processed_word = root.process_word_for_glossing(
                { word: words[i], sentence: "" },
                paragraph_shown_stems,
                global_stems,
                check_global,
            );

            if (processed_word) {
                paragraph_item.words_data.push(processed_word);
            }
        }
    }

    function process_paragraph_for_glossing(paragraph_text, paragraph_shown_stems, global_stems, check_global) {
        var words = SuttaBridge.extract_words(paragraph_text);
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
            if (prev_para && prev_para.words_data_json) {
                try {
                    var words_data = JSON.parse(prev_para.words_data_json);
                    for (var w = 0; w < words_data.length; w++) {
                        var word_item = words_data[w];
                        previous_stems[root.clean_stem(word_item.stem)] = true;
                    }
                } catch (e) {
                    logger.error("Failed to parse words_data_json:", e);
                }
            }
        }

        return previous_stems;
    }

    function dictionary_definitions_from_paragraph(paragraph): string {
        if (!paragraph || !paragraph.words_data_json) return "";

        try {
            var words_data = JSON.parse(paragraph.words_data_json);
            let out = "";
            for (var i = 0; i < words_data.length; i++) {
                var w = words_data[i];
                if (!w || !w.results || !w.results.length) continue;

                var selected_idx = w.selected_index || 0;
                if (selected_idx >= w.results.length) selected_idx = 0;

                var summary = summary_strip_html(w.results[selected_idx].summary);
                var def = `- ${w.original_word}: stem '${clean_stem(w.stem)}', ${summary}\n`;
                out += def;
            }
            return out;
        } catch (e) {
            logger.error("Failed to parse words_data_json:", e);
            return "";
        }
    }

    function update_all_glosses() {
        var paragraphs = gloss_text_input.text.split('\n\n').filter(p => p.trim() !== '');
        root.current_text = gloss_text_input.text;
        paragraph_model.clear();
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

            // Create paragraph with words data array
            var paragraph_item = root.create_paragraph_with_words_model(paragraphs[i]);
            paragraph_item.translations_json = translations_json;

            // Populate the words data
            root.populate_paragraph_words_data(
                paragraph_item,
                paragraphs[i],
                paragraph_shown_stems,
                root.global_shown_stems,
                root.no_duplicates_globally,
            );

            // Convert words_data to JSON for storage in ListModel, as ListModel can't handle complex JS arrays
            var model_item = {
                text: paragraph_item.text,
                words_data_json: JSON.stringify(paragraph_item.words_data),
                translations_json: paragraph_item.translations_json,
                selected_ai_tab: 0
            };

            paragraph_model.append(model_item);
        }

        root.save_session();
    }

    function update_paragraph_gloss(index) {
        var paragraph = paragraph_model.get(index);
        if (!paragraph) return;

        var paragraph_shown_stems = {};

        // If global deduplication, collect stems from previous paragraphs
        var previous_stems = root.no_duplicates_globally ? root.get_previous_paragraph_stems(index) : {};

        // Create temporary paragraph item to populate words data
        var temp_paragraph = { words_data: [] };
        root.populate_paragraph_words_data(
            temp_paragraph,
            paragraph.text,
            paragraph_shown_stems,
            previous_stems,
            root.no_duplicates_globally
        );

        // Update the model with JSON
        paragraph_model.setProperty(index, "words_data_json", JSON.stringify(temp_paragraph.words_data));

        root.save_session();
    }

    function load_session(db_id, gloss_data_json) {
        try {
            var session_data = JSON.parse(gloss_data_json);

            paragraph_model.clear();
            root.current_text = session_data.text || "";
            root.no_duplicates_globally = session_data.no_duplicates_globally !== undefined ?
                                         session_data.no_duplicates_globally : true;

            gloss_text_input.text = root.current_text;

            // Load paragraphs
            if (session_data.paragraphs) {
                for (var i = 0; i < session_data.paragraphs.length; i++) {
                    var para_data = session_data.paragraphs[i];
                    var model_item = {
                        text: para_data.text || "",
                        words_data_json: JSON.stringify(para_data.words || []),
                        translations_json: "[]",
                        selected_ai_tab: 0
                    };

                    paragraph_model.append(model_item);
                }
            }

            root.current_session_id = db_id;
        } catch (e) {
            logger.error("Failed to load session:", e);
        }
    }

    function update_word_selection(paragraph_idx: int, word_idx: int, selected_idx: int) {
        if (paragraph_idx >= paragraph_model.count) return;

        var paragraph = paragraph_model.get(paragraph_idx);
        if (!paragraph || !paragraph.words_data_json) return;

        var words_data = JSON.parse(paragraph.words_data_json);
        if (word_idx >= words_data.length) return;

        var word_item = words_data[word_idx];
        if (!word_item || !word_item.results || selected_idx >= word_item.results.length) return;

        // Update selection index and stem directly
        words_data[word_idx].selected_index = selected_idx;
        words_data[word_idx].stem = word_item.results[selected_idx].word;

        // Update model with new JSON
        paragraph_model.setProperty(paragraph_idx, "words_data_json", JSON.stringify(words_data));

        root.save_session();
    }

    function update_paragraph_text(index, new_text) {
        paragraph_model.setProperty(index, "text", new_text);
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

        for (var i = 0; i < paragraph_model.count; i++) {
            var paragraph = paragraph_model.get(i);
            if (!paragraph) continue;

            var para_data = {
                text: paragraph.text ? paragraph.text.trim() : "",
                vocabulary: [],
                ai_translations: [],
            };

            if (paragraph.words_data_json) {
                try {
                    var words_data = JSON.parse(paragraph.words_data_json);
                    for (var j = 0; j < words_data.length; j++) {
                        var w_data = words_data[j];
                        if (!w_data || !w_data.results || w_data.results.length == 0) continue;

                        var selected_index = w_data.selected_index || 0;
                        if (selected_index >= w_data.results.length) selected_index = 0;

                        // Add one line of word vocabulary info.
                        // For each word, export only the selected result.
                        para_data.vocabulary.push(w_data.results[selected_index]);
                    }
                } catch (e) {
                    logger.error("Failed to parse words_data_json:", e);
                }
            }

            // Add AI translations if they exist
            if (paragraph.translations_json) {
                try {
                    var translations = JSON.parse(paragraph.translations_json);
                    var selected_tab_index = paragraph.selected_ai_tab || 0;
                    var selected_translation = null;
                    var other_translations = [];

                    for (var k = 0; k < translations.length; k++) {
                        var trans = translations[k];
                        if (trans.status === "completed" && trans.response && trans.response.trim()) {
                            var isSelected = (k === selected_tab_index);
                            if (isSelected) {
                                selected_translation = {
                                    model_name: trans.model_name,
                                    response: trans.response,
                                    is_selected: true
                                };
                            } else {
                                other_translations.push({
                                    model_name: trans.model_name,
                                    response: trans.response,
                                    is_selected: false
                                });
                            }
                        }
                    }

                    // Add selected translation first, then others
                    if (selected_translation) {
                        para_data.ai_translations.push(selected_translation);
                    }
                    para_data.ai_translations = para_data.ai_translations.concat(other_translations);

                } catch (e) {
                    logger.error("Failed to parse translations_json:", e);
                }
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
`;

        for (var i = 0; i < gloss_data.paragraphs.length; i++) {
            var paragraph = gloss_data.paragraphs[i];
            let para_text = "\n<blockquote>\n" + paragraph.text.replace(/\n/g, "<br>\n") + "\n</blockquote>\n";

            var table_rows = "";
            for (var j = 0; j < paragraph.vocabulary.length; j++) {
                var res = paragraph.vocabulary[j];
                table_rows += `<tr><td> <b>${res.word}</b> </td><td> ${res.summary} </td></tr>\n`;
            }

            // Add AI translations section if they exist
            var ai_translations_section = "";
            if (paragraph.ai_translations && paragraph.ai_translations.length > 0) {
                ai_translations_section = "\n<h3>AI Translations</h3>\n";
                for (var k = 0; k < paragraph.ai_translations.length; k++) {
                    var ai_trans = paragraph.ai_translations[k];
                    var model_display = ai_trans.model_name;
                    var selected_indicator = ai_trans.is_selected ? " (selected)" : "";
                    ai_translations_section += `<h4>${model_display}${selected_indicator}</h4>\n`;
                    ai_translations_section += `<blockquote>${ai_trans.response.replace(/\n/g, "<br>\n")}</blockquote>\n`;
                }
            }

            out += `
<h2>Paragraph ${i+1}</h2>

${para_text}

${ai_translations_section}

<h3>Vocabulary</h3>

<p><b>Dictionary definitions from DPD:</b></p>

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

            // Add AI translations section if they exist
            var ai_translations_section = "";
            if (paragraph.ai_translations && paragraph.ai_translations.length > 0) {
                ai_translations_section = "\n### AI Translations\n";
                for (var k = 0; k < paragraph.ai_translations.length; k++) {
                    var ai_trans = paragraph.ai_translations[k];
                    var model_display = ai_trans.model_name;
                    var selected_indicator = ai_trans.is_selected ? " (selected)" : "";
                    ai_translations_section += `\n#### ${model_display}${selected_indicator}\n\n`;
                    ai_translations_section += `> ${ai_trans.response.replace(/\n/g, "\n> ")}\n`;
                }
            }

            // Add the table header for syntax recognition, but leave empty to save space when rendered.
            out += `
## Paragraph ${i+1}

${para_text}

${ai_translations_section}

### Vocabulary

**Dictionary definitions from DPD:**

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

            // Add AI translations section if they exist
            var ai_translations_section = "";
            if (paragraph.ai_translations && paragraph.ai_translations.length > 0) {
                ai_translations_section = "\n*** AI Translations\n";
                for (var k = 0; k < paragraph.ai_translations.length; k++) {
                    var ai_trans = paragraph.ai_translations[k];
                    var model_display = ai_trans.model_name;
                    var selected_indicator = ai_trans.is_selected ? " (selected)" : "";
                    ai_translations_section += `\n**** ${model_display}${selected_indicator}\n\n`;
                    ai_translations_section += `#+begin_quote\n${ai_trans.response}\n#+end_quote\n`;
                }
            }

            out += `
** Paragraph ${i+1}

${para_text}

${ai_translations_section}

*** Vocabulary

*Dictionary definitions from DPD:*

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
            id: main_scroll_view
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
                    id: paragraph_repeater
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
            required property int selected_ai_tab

            property bool is_collapsed: collapse_btn.checked

            RowLayout {
                Layout.leftMargin: 10

                Button {
                    id: collapse_btn
                    checkable: true
                    checked: false
                    icon.source: checked ? "icons/32x32/material-symbols--expand-all.png" : "icons/32x32/material-symbols--collapse-all.png"
                    Layout.alignment: Qt.AlignLeft
                    Layout.preferredWidth: collapse_btn.height
                }

                Label {
                    text: "Paragraph " + (paragraph_item.index + 1)
                    font.bold: true
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
                                id: ai_translate_btn
                                text: "AI Translate"
                                Layout.alignment: Qt.AlignRight
                                onClicked: {
                                    logger.log(`🚀 AI Translate button clicked for paragraph ${paragraph_item.index}`);

                                    root.load_translation_models();
                                    logger.log(`📋 Loaded ${translation_models.count} translation models`);

                                    let paragraph = paragraph_model.get(paragraph_item.index);

                                    // Load system prompt and translation template
                                    let system_prompt = SuttaBridge.get_system_prompt("Gloss Tab: System Prompt");
                                    let template = SuttaBridge.get_system_prompt("Gloss Tab: AI Translation");
                                    let user_prompt = template
                                        .replace("<<PALI_PASSAGE>>", paragraph_item.text)
                                        .replace("<<DICTIONARY_DEFINITIONS>>", root.dictionary_definitions_from_paragraph(paragraph));

                                    // Combine system prompt with user prompt (simple approach)
                                    let combined_prompt = user_prompt;
                                    if (system_prompt && system_prompt.trim() !== "") {
                                        combined_prompt = system_prompt + "\n\n" + user_prompt;
                                    }

                                    logger.log(`📝 Generated prompt with system context: "${combined_prompt.substring(0, 200)}..."`);

                                    let translations = [];

                                    for (var i = 0; i < translation_models.count; i++) {
                                        var item = translation_models.get(i);
                                        if (item.enabled) {
                                            let request_id = root.generate_request_id();
                                            let translation_idx = translations.length; // Use the current translations array length as index
                                            logger.log(`🎯 Sending request to ${item.model_name} (model_idx=${i}, translation_idx=${translation_idx}, request_id=${request_id})`);
                                            let provider_name = SuttaBridge.get_provider_for_model(item.model_name);
                                            pm.prompt_request(paragraph_item.index, translation_idx, provider_name, item.model_name, combined_prompt);
                                            translations.push({
                                                model_name: item.model_name,
                                                status: "waiting",
                                                response: "",
                                                request_id: request_id,
                                                retry_count: 0,
                                                last_updated: Date.now(),
                                                user_selected: translation_idx === 0
                                            });
                                        } else {
                                            logger.log(`⏭️  Skipping disabled model ${item.model_name}`);
                                        }
                                    }

                                    logger.log(`📊 Created ${translations.length} translation entries`);
                                    let translations_json = JSON.stringify(translations);
                                    paragraph_model.setProperty(paragraph_item.index, "translations_json", translations_json);
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

                AssistantResponses {
                    id: assistant_responses_component
                    title: "AI Translations:"
                    is_dark: root.is_dark
                    Layout.fillWidth: true
                    translations_data: {
                        try {
                            return JSON.parse(paragraph_item.translations_json);
                        } catch (e) {
                            logger.error(`❌ Error parsing translations_json for paragraph ${paragraph_item.index}:`, e);
                            return [];
                        }
                    }
                    paragraph_text: paragraph_item.text
                    paragraph_index: paragraph_item.index
                    selected_tab_index: paragraph_item.selected_ai_tab || 0

                    onRetryRequest: function(model_name, request_id) {
                        root.handle_retry_request(paragraph_item.index, model_name, request_id);
                    }

                    onTabSelectionChanged: function(tab_index, model_name) {
                        root.update_tab_selection(paragraph_item.index, tab_index, model_name);
                    }
                }

                ColumnLayout {
                    spacing: 10
                    Layout.margins: 10

                    Text {
                        text:  "Dictionary definitions from DPD:"
                        font.bold: true
                        font.pointSize: root.vocab_font_point_size
                    }

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
                        SuttaBridge.save_common_words_json(JSON.stringify(root.common_words));
                        commonWordsDialog.close();
                        root.update_all_glosses();
                    }
                }
            }
        }
    }
}
