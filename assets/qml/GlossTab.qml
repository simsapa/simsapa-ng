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
    property alias commonWordsDialog: commonWordsDialog

    property var handle_open_dict_tab_fn

    // Unrecognized words tracking
    property var global_unrecognized_words: []
    property var paragraph_unrecognized_words: ({})

    property string text_color: root.is_dark ? "#F0F0F0" : "#000000"
    property string bg_color: root.is_dark ? "#23272E" : "#FAE6B2"
    property string bg_color_lighter: root.is_dark ? "#2E333D" : "#FBEDC7"
    property string bg_color_darker: root.is_dark ? "#1C2025" : "#F8DA8E"

    property string border_color: root.is_dark ? "#0a0a0a" : "#ccc"

    // Signals
    signal requestWordSummary(string word)

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

    // Background processing state tracking
    property bool is_processing_all: false
    property bool is_processing_single: false
    property bool is_exporting_anki: false
    property int exporting_note_count: 0

    // Signal connections for background gloss processing
    Connections {
        target: SuttaBridge

        function onAllParagraphsGlossReady(results_json: string) {
            logger.debug(`📥 onAllParagraphsGlossReady received: ${results_json.substring(0, 100)}...`);

            // Always reset processing state
            root.is_processing_all = false;

            try {
                let results = JSON.parse(results_json);
                if (results.success) {
                    root.handle_all_paragraphs_results(results);
                } else {
                    logger.error(`❌ Background processing failed: ${results.error}`);
                    // TODO: Show user-friendly error message
                }
            } catch (e) {
                logger.error("Failed to parse background processing results:", e);
                // TODO: Show user-friendly error message
            }
        }

        function onParagraphGlossReady(paragraph_index: int, results_json: string) {
            logger.debug(`📥 onParagraphGlossReady received for paragraph ${paragraph_index}: ${results_json.substring(0, 100)}...`);

            // Always reset processing state
            root.is_processing_single = false;

            try {
                let results = JSON.parse(results_json);
                if (results.success) {
                    root.handle_single_paragraph_results(paragraph_index, results);
                } else {
                    logger.error(`❌ Background processing failed for paragraph ${paragraph_index}: ${results.error}`);
                    // TODO: Show user-friendly error message
                }
            } catch (e) {
                logger.error("Failed to parse background processing results:", e);
                // TODO: Show user-friendly error message
            }
        }

        function onAnkiCsvExportReady(results_json: string) {
            logger.debug(`📥 onAnkiCsvExportReady received: ${results_json.substring(0, 100)}...`);

            // Always reset exporting state
            root.is_exporting_anki = false;

            try {
                let results = JSON.parse(results_json);
                if (results.success && results.files && results.files.length > 0) {
                    root.handle_anki_export_results(results);
                } else {
                    logger.error(`❌ Anki export failed: ${results.error || 'Unknown error'}`);
                    msg_dialog_ok.text = `Export failed: ${results.error || 'Unknown error'}`;
                    msg_dialog_ok.open();
                }
            } catch (e) {
                logger.error("Failed to parse Anki export results:", e);
                msg_dialog_ok.text = `Export failed: ${e}`;
                msg_dialog_ok.open();
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
        let save_file_name = null;
        let save_content = null;
        let is_anki_csv = false;

        if (export_btn.currentValue === "HTML") {
            save_file_name = "gloss_export.html";
            save_content = root.gloss_as_html();

        } else if (export_btn.currentValue === "Markdown") {
            save_file_name = "gloss_export.md";
            save_content = root.gloss_as_markdown();

        } else if (export_btn.currentValue === "Org-Mode") {
            save_file_name = "gloss_export.org";
            save_content = root.gloss_as_orgmode();

        } else if (export_btn.currentValue === "Anki CSV") {
            is_anki_csv = true;
        }

        let save_fn = function() {
            if (is_anki_csv) {
                root.start_anki_export_background(export_folder_dialog.selectedFolder);
            } else {
                let ok = SuttaBridge.save_file(export_folder_dialog.selectedFolder, save_file_name, save_content);
                if (ok) {
                    msg_dialog_ok.text = "Exported as: " + save_file_name;
                    msg_dialog_ok.open();
                } else {
                    msg_dialog_ok.text = "Export failed."
                    msg_dialog_ok.open();
                }
            }
        };

        if (is_anki_csv) {
            // AnkiExportFormat
            let export_format = SuttaBridge.get_anki_export_format().toLowerCase();
            let include_cloze = SuttaBridge.get_anki_include_cloze();

            let existing_save_files = [];

            // AnkiExportFormat
            if (export_format) {
                var name = `gloss_export_anki_${export_format}.csv`;
                var exists = SuttaBridge.check_file_exists_in_folder(export_folder_dialog.selectedFolder, name);
                if (exists) {
                    existing_save_files.push(name);
                }
                if (include_cloze) {
                    var name = `gloss_export_anki_${export_format}_cloze.csv`;
                    var exists = SuttaBridge.check_file_exists_in_folder(export_folder_dialog.selectedFolder, name);
                    if (exists) {
                        existing_save_files.push(name);
                    }
                }
            }

            if (existing_save_files.length > 0) {
                let file_names = existing_save_files.join(", ");
                msg_dialog_cancel_ok.text = `Already exists: ${file_names}. Overwrite?`;
                msg_dialog_cancel_ok.accept_fn = save_fn;
                msg_dialog_cancel_ok.open();
            } else {
                save_fn();
            }
        } else {
            if (save_file_name) {
                let exists = SuttaBridge.check_file_exists_in_folder(export_folder_dialog.selectedFolder, save_file_name);
                if (exists) {
                    msg_dialog_cancel_ok.text = `Already exists: ${save_file_name}. Overwrite?`;
                    msg_dialog_cancel_ok.accept_fn = save_fn;
                    msg_dialog_cancel_ok.open();
                } else {
                    save_fn();
                }
            }
        }

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

    MessageDialog {
        id: no_models_dialog
        title: "No AI Models"
        text: "There are no enabled models. See Prompts menu > AI Models"
        buttons: MessageDialog.Ok
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
                    // Determine the correct prompt template based on the original request
                    var template_key = translations[i].with_vocab ? "Gloss Tab: AI Translation with Vocabulary" : "Gloss Tab: AI Translation without Vocabulary";
                    var template = SuttaBridge.get_system_prompt(template_key);
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

    function handle_ai_translate_request(paragraph_index: int, with_vocab = true) {
        logger.log(`🚀 AI Translate button clicked for paragraph ${paragraph_index}, with_vocab=${with_vocab}`);

        root.load_translation_models();
        logger.log(`📋 Loaded ${translation_models.count} translation models`);

        if (translation_models.count === 0) {
            no_models_dialog.open();
            return;
        }

        let paragraph = paragraph_model.get(paragraph_index);

        // Load system prompt and translation template
        let system_prompt = SuttaBridge.get_system_prompt("Gloss Tab: System Prompt");
        let template_key = with_vocab ? "Gloss Tab: AI Translation with Vocabulary" : "Gloss Tab: AI Translation without Vocabulary";
        let template = SuttaBridge.get_system_prompt(template_key);
        let user_prompt = template
            .replace("<<PALI_PASSAGE>>", paragraph.text)
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
                pm.prompt_request(paragraph_index, translation_idx, provider_name, item.model_name, combined_prompt);
                translations.push({
                    model_name: item.model_name,
                    status: "waiting",
                    response: "",
                    request_id: request_id,
                    retry_count: 0,
                    last_updated: Date.now(),
                    user_selected: translation_idx === 0,
                    with_vocab: with_vocab
                });
            } else {
                logger.log(`⏭️  Skipping disabled model ${item.model_name}`);
            }
        }

        logger.log(`📊 Created ${translations.length} translation entries`);
        let translations_json = JSON.stringify(translations);
        paragraph_model.setProperty(paragraph_index, "translations_json", translations_json);
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

    function process_word_for_glossing(word_info, paragraph_shown_stems, global_stems, check_global) {
        var lookup_results_json = SuttaBridge.dpd_lookup_json(word_info.word.toLowerCase());
        var results = [];
        try {
            results = JSON.parse(lookup_results_json);
        } catch (e) {
            logger.error("Failed to parse lookup result:", e);
            return null;
        }

        // Skip if no results - but return info about unrecognized word
        if (!results || results.length === 0) {
            return { is_unrecognized: true, word: word_info.word };
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

    // Handle results from background processing of all paragraphs
    function handle_all_paragraphs_results(results) {
        logger.debug(`🔄 Processing results for ${results.paragraphs.length} paragraphs`);

        // Clear the paragraph model
        paragraph_model.clear();

        // Update global state
        root.global_shown_stems = results.updated_global_stems || {};
        root.global_unrecognized_words = results.global_unrecognized_words || [];

        // Process each paragraph result
        for (var i = 0; i < results.paragraphs.length; i++) {
            let paragraph_result = results.paragraphs[i];
            let paragraph_text = root.current_text.split('\n\n').filter(p => p.trim() !== '')[i] || "";

            // Update paragraph unrecognized words
            root.paragraph_unrecognized_words[paragraph_result.paragraph_index] = paragraph_result.unrecognized_words || [];

            // Create model item
            let model_item = {
                text: paragraph_text,
                words_data_json: JSON.stringify(paragraph_result.words_data),
                translations_json: "[]", // TODO: Preserve existing translations if any
                selected_ai_tab: 0
            };

            paragraph_model.append(model_item);
        }

        logger.debug(`✅ Successfully processed ${results.paragraphs.length} paragraphs`);
        root.save_session();
    }

    // Handle results from background processing of a single paragraph
    function handle_single_paragraph_results(paragraph_index, results) {
        logger.debug(`🔄 Processing results for paragraph ${paragraph_index}`);

        if (paragraph_index >= paragraph_model.count) {
            logger.error(`❌ Invalid paragraph index: ${paragraph_index}`);
            return;
        }

        // Update global state
        root.global_shown_stems = results.updated_global_stems || {};

        // Update paragraph unrecognized words
        root.paragraph_unrecognized_words[paragraph_index] = results.unrecognized_words || [];

        // Update global unrecognized words (merge with existing)
        let existing_global = root.global_unrecognized_words || [];
        let new_unrecognized = results.unrecognized_words || [];
        for (let word of new_unrecognized) {
            if (existing_global.indexOf(word) === -1) {
                existing_global.push(word);
            }
        }
        root.global_unrecognized_words = existing_global;

        // Update the paragraph model
        paragraph_model.setProperty(paragraph_index, "words_data_json", JSON.stringify(results.words_data));

        logger.debug(`✅ Successfully processed paragraph ${paragraph_index}`);
        root.save_session();
    }

    // Start background processing for all paragraphs
    function start_background_all_glosses() {
        if (root.is_processing_all) {
            logger.warn("Background processing already in progress");
            return;
        }

        let paragraphs = gloss_text_input.text.split('\n\n').filter(p => p.trim() !== '');
        if (paragraphs.length === 0) {
            logger.warn("No paragraphs to process");
            return;
        }

        logger.debug(`🚀 Starting background processing for ${paragraphs.length} paragraphs`);

        // Set processing state
        root.is_processing_all = true;
        root.current_text = gloss_text_input.text;

        // Reset global state
        root.global_shown_stems = {};
        root.global_unrecognized_words = [];
        root.paragraph_unrecognized_words = {};

        // Prepare input data structure
        let input_data = {
            paragraphs: paragraphs,
            options: {
                no_duplicates_globally: root.no_duplicates_globally,
                skip_common: root.skip_common,
                common_words: root.common_words,
                existing_global_stems: {},
                existing_paragraph_unrecognized: {},
                existing_global_unrecognized: []
            }
        };

        // Call background processing function
        SuttaBridge.process_all_paragraphs_background(JSON.stringify(input_data));
    }

    // Start background processing for a single paragraph
    function start_background_paragraph_gloss(paragraph_index) {
        if (root.is_processing_single) {
            logger.warn("Background processing already in progress");
            return;
        }

        let paragraph = paragraph_model.get(paragraph_index);
        if (!paragraph || !paragraph.text.trim()) {
            logger.warn(`No valid paragraph at index ${paragraph_index}`);
            return;
        }

        logger.debug(`🚀 Starting background processing for paragraph ${paragraph_index}`);

        // Set processing state
        root.is_processing_single = true;

        // Get existing global stems (from previous paragraphs if global deduplication is enabled)
        let existing_global_stems = root.no_duplicates_globally ? root.get_previous_paragraph_stems(paragraph_index) : {};

        // Prepare input data structure
        let input_data = {
            paragraph_text: paragraph.text,
            options: {
                no_duplicates_globally: root.no_duplicates_globally,
                skip_common: root.skip_common,
                common_words: root.common_words,
                existing_global_stems: existing_global_stems,
                existing_paragraph_unrecognized: root.paragraph_unrecognized_words,
                existing_global_unrecognized: root.global_unrecognized_words
            }
        };

        // Call background processing function
        SuttaBridge.process_paragraph_background(paragraph_index, JSON.stringify(input_data));
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

    function format_paragraph_html(paragraph: var, paragraph_number: int): string {
        let para_text = "\n<blockquote>\n" + paragraph.text.replace(/\n/g, "<br>\n") + "\n</blockquote>\n";

        var table_rows = "";
        for (var j = 0; j < paragraph.vocabulary.length; j++) {
            var res = paragraph.vocabulary[j];
            table_rows += `<tr><td> <b>${res.word}</b> </td><td> ${res.summary} </td></tr>\n`;
        }

        var ai_translations_section = "";
        if (paragraph.ai_translations && paragraph.ai_translations.length > 0) {
            ai_translations_section = "\n<h3>AI Translations</h3>\n";
            for (var k = 0; k < paragraph.ai_translations.length; k++) {
                var ai_trans = paragraph.ai_translations[k];
                var ai_trans_html = SuttaBridge.markdown_to_html(ai_trans.response || "");
                var model_display = ai_trans.model_name;
                var selected_indicator = ai_trans.is_selected ? " (selected)" : "";
                ai_translations_section += `<h4>${model_display}${selected_indicator}</h4>\n`;
                ai_translations_section += `<blockquote>${ai_trans_html}</blockquote>\n`;
            }
        }

        return `
<h2>Paragraph ${paragraph_number}</h2>

${para_text}

${ai_translations_section}

<h3>Vocabulary</h3>

<p><b>Dictionary definitions from DPD:</b></p>

<table><tbody>
${table_rows}
</tbody></table>
`;
    }

    function format_paragraph_markdown(paragraph: var, paragraph_number: int): string {
        var para_text = "\n> " + paragraph.text.replace(/\n/g, "\n> ");

        var table_rows = "";
        for (var j = 0; j < paragraph.vocabulary.length; j++) {
            var res = paragraph.vocabulary[j];
            var summary = root.summary_html_to_md(res.summary);
            table_rows += `| **${res.word}** | ${summary} |\n`;
        }

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

        return `
## Paragraph ${paragraph_number}

${para_text}

${ai_translations_section}

### Vocabulary

**Dictionary definitions from DPD:**

|    |    |
|----|----|
${table_rows}
`;
    }

    function format_paragraph_orgmode(paragraph: var, paragraph_number: int): string {
        let para_text = "\n#+begin_quote\n" + paragraph.text + "\n#+end_quote\n";

        var table_rows = "";
        for (var j = 0; j < paragraph.vocabulary.length; j++) {
            var res = paragraph.vocabulary[j];
            var summary = root.summary_html_to_orgmode(res.summary);
            table_rows += `| *${res.word}* | ${summary} |\n`;
        }

        var ai_translations_section = "";
        if (paragraph.ai_translations && paragraph.ai_translations.length > 0) {
            ai_translations_section = "\n*** AI Translations\n";
            for (var k = 0; k < paragraph.ai_translations.length; k++) {
                var ai_trans = paragraph.ai_translations[k];
                var ai_trans_md = ai_trans.response.split('\n').map(function(line) {
                    return line.replace(/^\* /, '- ');
                }).join('\n');
                var model_display = ai_trans.model_name;
                var selected_indicator = ai_trans.is_selected ? " (selected)" : "";
                ai_translations_section += `\n**** ${model_display}${selected_indicator}\n\n`;
                ai_translations_section += `#+begin_src markdown\n${ai_trans_md}\n#+end_src\n`;
            }
        }

        return `
** Paragraph ${paragraph_number}

${para_text}

${ai_translations_section}

*** Vocabulary

*Dictionary definitions from DPD:*

${table_rows}
`;
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
                        var vocab_item = Object.assign({}, w_data.results[selected_index]);
                        vocab_item.context_snippet = w_data.example_sentence || "";
                        para_data.vocabulary.push(vocab_item);
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
            out += root.format_paragraph_html(paragraph, i+1);
        }

        out += "\n</body>\n</html>";
        return out.trim().replace(/\n\n\n+/g, "\n\n");
    }

    function gloss_as_markdown(): string {
        let gloss_data = root.gloss_export_data();

        let main_text = "\n> " + gloss_data.text.replace(/\n/g, "\n> ");

        let out = `
# Gloss Export

${main_text}
`;

        for (var i = 0; i < gloss_data.paragraphs.length; i++) {
            var paragraph = gloss_data.paragraphs[i];
            out += root.format_paragraph_markdown(paragraph, i+1);
        }

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
            out += root.format_paragraph_orgmode(paragraph, i+1);
        }

        return out.trim().replace(/\n\n\n+/g, "\n\n");
    }

    function paragraph_gloss_as_html(paragraph_index: int): string {
        if (paragraph_index < 0 || paragraph_index >= paragraph_model.count) {
            logger.error("Invalid paragraph index:", paragraph_index);
            return "";
        }

        let gloss_data = root.gloss_export_data();
        if (paragraph_index >= gloss_data.paragraphs.length) {
            logger.error("Paragraph index out of range:", paragraph_index);
            return "";
        }

        var paragraph = gloss_data.paragraphs[paragraph_index];
        return root.format_paragraph_html(paragraph, paragraph_index + 1).trim().replace(/\n\n\n+/g, "\n\n");
    }

    function paragraph_gloss_as_markdown(paragraph_index: int): string {
        if (paragraph_index < 0 || paragraph_index >= paragraph_model.count) {
            logger.error("Invalid paragraph index:", paragraph_index);
            return "";
        }

        let gloss_data = root.gloss_export_data();
        if (paragraph_index >= gloss_data.paragraphs.length) {
            logger.error("Paragraph index out of range:", paragraph_index);
            return "";
        }

        var paragraph = gloss_data.paragraphs[paragraph_index];
        return root.format_paragraph_markdown(paragraph, paragraph_index + 1).trim().replace(/\n\n\n+/g, "\n\n");
    }

    function paragraph_gloss_as_orgmode(paragraph_index: int): string {
        if (paragraph_index < 0 || paragraph_index >= paragraph_model.count) {
            logger.error("Invalid paragraph index:", paragraph_index);
            return "";
        }

        let gloss_data = root.gloss_export_data();
        if (paragraph_index >= gloss_data.paragraphs.length) {
            logger.error("Paragraph index out of range:", paragraph_index);
            return "";
        }

        var paragraph = gloss_data.paragraphs[paragraph_index];
        return root.format_paragraph_orgmode(paragraph, paragraph_index + 1).trim().replace(/\n\n\n+/g, "\n\n");
    }

    function start_anki_export_background(folder_url) {
        if (root.is_exporting_anki) {
            logger.warn("Anki export already in progress");
            return;
        }

        let gloss_data = root.gloss_export_data();
        let note_count = 0;
        for (var i = 0; i < gloss_data.paragraphs.length; i++) {
            note_count += gloss_data.paragraphs[i].vocabulary.length;
        }

        root.exporting_note_count = note_count;
        root.is_exporting_anki = true;

        let export_format = SuttaBridge.get_anki_export_format();
        let include_cloze = SuttaBridge.get_anki_include_cloze();

        let input_data = {
            gloss_data_json: JSON.stringify(gloss_data),
            export_format: export_format,
            include_cloze: include_cloze,
            templates: {
                front: SuttaBridge.get_anki_template_front(),
                back: SuttaBridge.get_anki_template_back(),
                cloze_front: SuttaBridge.get_anki_template_cloze_front(),
                cloze_back: SuttaBridge.get_anki_template_cloze_back()
            },
            folder_url: folder_url.toString()
        };

        SuttaBridge.export_anki_csv_background(JSON.stringify(input_data));
    }

    function handle_anki_export_results(results) {
        logger.debug(`📦 Handling Anki export results: ${results.files.length} files`);

        let folder_url = export_folder_dialog.selectedFolder;
        let files_saved = [];

        for (var i = 0; i < results.files.length; i++) {
            let file = results.files[i];
            let ok = SuttaBridge.save_file(folder_url, file.filename, file.content);
            if (ok) {
                files_saved.push(file.filename);
            }
        }

        if (files_saved.length > 0) {
            msg_dialog_ok.text = "Exported as: " + files_saved.join(", ");
            msg_dialog_ok.open();
        } else {
            msg_dialog_ok.text = "Export failed: No files saved";
            msg_dialog_ok.open();
        }
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
                            contentWidth: availableWidth

                            TextArea {
                                id: gloss_text_input
                                width: parent.width
                                font.pointSize: 12
                                placeholderText: "Enter paragraphs to gloss ..."
                                selectByMouse: true
                                wrapMode: TextEdit.WordWrap
                            }
                        }

                        Flow {
                            Layout.fillWidth: true
                            spacing: 10

                            CheckBox {
                                id: globalDedupeCheckBox
                                text: "No duplicates"
                                checked: root.no_duplicates_globally
                                onCheckedChanged: {
                                    root.no_duplicates_globally = globalDedupeCheckBox.checked;
                                    if (paragraph_model.count > 0) {
                                        root.start_background_all_glosses();
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
                                        root.start_background_all_glosses();
                                    }
                                }
                            }

                            Text {
                                id: exporting_message
                                text: `Exporting ${root.exporting_note_count} notes...`
                                font.pointSize: root.vocab_font_point_size
                                color: "#4CAF50"
                                visible: root.is_exporting_anki
                                /* anchors.verticalCenter: parent.verticalCenter */
                            }

                            ComboBox {
                                id: export_btn
                                model: ["Export As...", "HTML", "Markdown", "Org-Mode", "Anki CSV"]
                                enabled: paragraph_model.count > 0 && !root.is_exporting_anki
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
                                enabled: !root.is_processing_all
                                icon.source: root.is_processing_all ? "icons/32x32/fa_stopwatch-solid.png" : ""
                                onClicked: root.start_background_all_glosses()
                            }
                        }
                    }
                }

                // Global unrecognized words list
                UnrecognizedWordsList {
                    Layout.fillWidth: true
                    Layout.leftMargin: 10
                    Layout.rightMargin: 10
                    word_list: root.global_unrecognized_words
                    prefix_text: "Click for deconstructor lookup:"
                    bg_color_darker: root.bg_color_darker
                    bg_color_lighter: root.bg_color_lighter
                    text_color: root.text_color
                    border_color: root.border_color
                    onWordClicked: function(word) {
                        root.requestWordSummary(word)
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
                            contentWidth: availableWidth

                            TextArea {
                                width: parent.width
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

                        Flow {
                            Layout.fillWidth: true
                            spacing: 10

                            Button {
                                id: ai_translate_btn
                                text: "AI Translate w/ Vocab"
                                onClicked: root.handle_ai_translate_request(paragraph_item.index, true)
                            }

                            Button {
                                id: ai_translate_no_vocab_btn
                                text: "w/o Vocab"
                                onClicked: root.handle_ai_translate_request(paragraph_item.index, false)
                            }

                            Button {
                                id: update_gloss_btn
                                text: "Update Gloss"
                                enabled: !root.is_processing_single
                                icon.source: root.is_processing_single ? "icons/32x32/fa_stopwatch-solid.png" : ""
                                onClicked: root.start_background_paragraph_gloss(paragraph_item.index)
                            }
                        }
                    }
                }

                AssistantResponses {
                    id: assistant_responses_component
                    title: {
                        try {
                            let translations = JSON.parse(paragraph_item.translations_json);
                            if (translations && translations.length > 0) {
                                // Check the first translation to determine if it was with or without vocab
                                let with_vocab = translations[0].with_vocab;
                                return with_vocab ? "AI Translations w/ Vocab:" : "AI Translations w/o Vocab:";
                            }
                            return "AI Translations:";
                        } catch (e) {
                            logger.error(`❌ Error parsing translations_json for title in paragraph ${paragraph_item.index}:`, e);
                            return "AI Translations:";
                        }
                    }
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

                    // Per-paragraph unrecognized words list
                    UnrecognizedWordsList {
                        Layout.fillWidth: true
                        word_list: root.paragraph_unrecognized_words[paragraph_item.index] || []
                        prefix_text: "Click for deconstructor lookup:"
                        bg_color_darker: root.bg_color_darker
                        bg_color_lighter: root.bg_color_lighter
                        text_color: root.text_color
                        border_color: root.border_color
                        onWordClicked: function(word) {
                            root.requestWordSummary(word)
                        }
                    }

                    TextEdit {
                        id: paragraph_clip
                        visible: false
                        function copy_text(text) {
                            paragraph_clip.text = text;
                            paragraph_clip.selectAll();
                            paragraph_clip.copy();
                        }
                    }

                    RowLayout {
                        spacing: 10
                        Layout.fillWidth: true

                        Text {
                            text: "Dictionary definitions from DPD:"
                            font.bold: true
                            font.pointSize: root.vocab_font_point_size
                            Layout.alignment: Qt.AlignLeft
                        }

                        Item { Layout.fillWidth: true }

                        Text {
                            id: copied_message
                            text: "Copied!"
                            font.pointSize: root.vocab_font_point_size
                            color: "#4CAF50"
                            visible: false
                            opacity: 0
                            Layout.leftMargin: 10
                        }

                        SequentialAnimation {
                            id: copied_message_animation

                            PropertyAction {
                                target: copied_message
                                property: "visible"
                                value: true
                            }

                            NumberAnimation {
                                target: copied_message
                                property: "opacity"
                                from: 0
                                to: 1.0
                                duration: 200
                            }

                            PauseAnimation {
                                duration: 1500
                            }

                            NumberAnimation {
                                target: copied_message
                                property: "opacity"
                                from: 1.0
                                to: 0
                                duration: 300
                            }

                            PropertyAction {
                                target: copied_message
                                property: "visible"
                                value: false
                            }
                        }

                        ComboBox {
                            id: copy_combobox
                            model: ["Copy As...", "HTML", "Markdown", "Org-Mode"]
                            currentIndex: 0
                            Layout.alignment: Qt.AlignRight

                            onCurrentIndexChanged: {
                                if (currentIndex === 0) {
                                    return;
                                }

                                var content = "";
                                if (currentIndex === 1) {
                                    content = root.paragraph_gloss_as_html(paragraph_item.index);
                                } else if (currentIndex === 2) {
                                    content = root.paragraph_gloss_as_markdown(paragraph_item.index);
                                } else if (currentIndex === 3) {
                                    content = root.paragraph_gloss_as_orgmode(paragraph_item.index);
                                }

                                if (content.length > 0) {
                                    paragraph_clip.copy_text(content);
                                    copied_message_animation.start();
                                }

                                copy_combobox.currentIndex = 0;
                            }
                        }
                    }

                    ColumnLayout {
                        id: vocabulary_gloss
                        Layout.fillWidth: true
                        spacing: 5

                        property int paragraph_index: paragraph_item.index

                        Repeater {
                            model: {
                                try {
                                    return JSON.parse(paragraph_item.words_data_json);
                                } catch (e) {
                                    return [];
                                }
                            }

                            delegate: wordItemDelegate
                        }
                    }

                    Component {
                        id: wordItemDelegate

                        ItemDelegate {
                            id: wordItem
                            Layout.fillWidth: true
                            implicitHeight: mainContent.implicitHeight

                            required property int index
                            required property var modelData

                            property int paragraph_index: vocabulary_gloss.paragraph_index


                            Frame {
                                id: mainContent
                                width: parent.width
                                padding: 4

                                background: Rectangle {
                                    border.width: 0
                                    color: (wordItem.index % 2 === 0 ?  root.bg_color_lighter : root.bg_color)
                                }

                                RowLayout {
                                    width: parent.width
                                    spacing: 10

                                    ComboBox {
                                        id: word_select
                                        Layout.alignment: Qt.AlignTop
                                        Layout.preferredWidth: wordItem.width * 0.2
                                        visible: {
                                            // Show ComboBox only when there are multiple lookup results to choose from
                                            return wordItem.modelData.results !== undefined &&
                                                   wordItem.modelData.results.length > 1;
                                        }
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
                                        visible: {
                                            // Show static text when there's no results or only one result available
                                            return wordItem.modelData.results === undefined ||
                                                   wordItem.modelData.results.length <= 1;
                                        }
                                        text: {
                                            // Display the first result's word if available, otherwise show original word
                                            if (wordItem.modelData.results !== undefined &&
                                                wordItem.modelData.results.length > 0) {
                                                return wordItem.modelData.results[0].word;
                                            }
                                            return wordItem.modelData.original_word;
                                        }
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
                                                let word;
                                                // Get the selected word from results if available, otherwise use original word
                                                if (wordItem.modelData.results !== undefined &&
                                                    wordItem.modelData.results.length > 0) {
                                                    word = wordItem.modelData.results[idx].word;
                                                } else {
                                                    word = wordItem.modelData.original_word;
                                                }
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

            GroupBox {
                Layout.fillWidth: true
                Layout.fillHeight: true

                background: Rectangle {
                    anchors.fill: parent
                    color: "white"
                    border.width: 1
                    border.color: "#ccc"
                    radius: 5
                }

                ScrollView {
                    anchors.fill: parent

                    TextArea {
                        id: commonWordsTextArea
                        selectByMouse: true
                        text: root.common_words.join('\n')
                        background: Rectangle {
                            color: "transparent"
                        }
                    }
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
                        root.start_background_all_glosses();
                    }
                }
            }
        }
    }
}
