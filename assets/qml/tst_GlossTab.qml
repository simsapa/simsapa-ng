import QtQuick
import QtTest

import com.profoundlabs.simsapa

Item {
    width: 800; height: 600

    GlossTab {
        id: gloss_tab
        window_id: "window_0"
        ai_models_auto_retry: false
        is_dark: false
        anchors.centerIn: parent
    }

    TestCase {
        name: "TestGlossTab"
        when: windowShown

        function cleanup() {
            // Reset any global state after each test
            gloss_tab.global_shown_stems = {};
            gloss_tab.no_duplicates_globally = true;
        }

        function test_extract_words_with_context() {
            var text = "This is a test. Another sentence here!";
            var result = gloss_tab.extract_words_with_context(text);

            compare(result.length, 7);

            // Check first word
            compare(result[0].word, "This");
            compare(result[0].sentence, "This is a test.");
            compare(result[0].position, 0);

            // Check word from second sentence
            compare(result[4].word, "Another");
            compare(result[4].sentence, "Another sentence here!");

            // Test single sentence without punctuation
            result = gloss_tab.extract_words_with_context("No punctuation here");
            compare(result.length, 3);
            compare(result[0].sentence, "No punctuation here");
        }

        function test_clean_stem() {
            compare(gloss_tab.clean_stem("dhamma 1.01"), "dhamma");
            compare(gloss_tab.clean_stem("ña 2.1"), "ña");
            compare(gloss_tab.clean_stem("jhāyī 1"), "jhāyī");
            compare(gloss_tab.clean_stem("test 123.456"), "test");
            compare(gloss_tab.clean_stem("yo pana bhikkhu"), "yo pana bhikkhu");
        }

        function test_has_common_words() {
            verify(gloss_tab.common_words.length > 5);
            verify(gloss_tab.is_common_word("dhamma 1.01"));
            verify(gloss_tab.is_common_word("Tena"));
            verify(!gloss_tab.is_common_word("anupekkhati"));
        }

        function test_dpd_lookup() {
            let json = SuttaBridge.dpd_lookup_json("cittassa");
            var results = JSON.parse(json);
            compare(results[0].word, "citta 1.1");
            compare(results[1].word, "citta 1.2");
        }

        function test_process_word_for_glossing_filter_common_words() {
            var word_info = { word: "dhammehi", sentence: "So vivicceva kāmehi vivicca akusalehi dhammehi savitakkaṁ savicāraṁ..." };
            var paragraph_stems = {};
            var global_stems = {};

            var result = gloss_tab.process_word_for_glossing(word_info, paragraph_stems, global_stems, false);
            compare(result, null); // Should return null for common word
        }

        function test_process_paragraph_for_glossing() {
            var paragraph = "Idha, bhikkhave, ariyasāvako vossaggārammaṇaṁ karitvā labhati samādhiṁ, labhati cittassa ekaggataṁ.";
            var paragraph_stems = {};
            var global_stems = {};
            var all_results = [];

            var result = gloss_tab.process_paragraph_for_glossing(paragraph,
                                                                  paragraph_stems,
                                                                  global_stems,
                                                                  true);
            all_results.push(...result);

            var result_words = result.map(i => i.original_word);
            // logger.log(result_words);
            // [ariyasāvako,vossaggārammaṇaṁ,karitvā,labhati,samādhiṁ,,cittassa,ekaggataṁ.]

            // Should skip common words and local duplicates
            compare(result.length, 7);
            compare(result[0].original_word, "ariyasāvako");
            compare(result[1].original_word, "vossaggārammaṇaṁ");
            verify(global_stems["labhati"] === true);

            // Test with global duplicates
            paragraph = "Saddhassa hi, sāriputta, ariyasāvakassa āraddhavīriyassa upaṭṭhitassatino etaṁ pāṭikaṅkhaṁ yaṁ vossaggārammaṇaṁ karitvā labhissati samādhiṁ, labhissati cittassa ekaggataṁ. Yo hissa, sāriputta, samādhi tadassa samādhindriyaṁ.";
            paragraph_stems = {};
            result = gloss_tab.process_paragraph_for_glossing(paragraph,
                                                              paragraph_stems,
                                                              global_stems,
                                                              true);
            all_results.push(...result);

            result_words = result.map(i => i.original_word);
            // [Saddhassa,āraddhavīriyassa,upaṭṭhitassatino,pāṭikaṅkhaṁ,labhissati,hissa,,tadassa,samādhindriyaṁ.]

            // labhissati should be skipped as inflected form of labhati seen before.
            // FIXME should skip labhissati, but currently dpd_lookup.json has a direct entry for it.
            /* verify(!result_words.includes("labhissati")); */

            compare(result.length, 8);

            // Check that ariyasāvaka is not duplicated
            var n = 0;
            for (var i = 0; i < all_results.length; i++) {
                var w = gloss_tab.clean_stem(all_results[i].stem);
                if (w === "ariyasāvaka") n++;
            }
            compare(n, 1);
        }

        function test_gloss_word_selection_and_export() {
            var paragraph = "Katamañca, bhikkhave, samādhindriyaṁ? Idha, bhikkhave, ariyasāvako vossaggārammaṇaṁ karitvā labhati samādhiṁ, labhati cittassa ekaggataṁ.";
            gloss_tab.gloss_text_input.text = paragraph;
            gloss_tab.update_all_glosses();

            // Test that we have words data
            verify(gloss_tab.paragraph_model.count > 0);
            var paragraph_data = gloss_tab.paragraph_model.get(0);
            verify(paragraph_data.words_data_json.length > 0);

            // 1st paragraph, 5th word 'karitvā 1', change to 4th selection 'karitvā 4'
            gloss_tab.update_word_selection(0, 4, 3);
            // 1st paragraph, 8th word 'citta 1.1', change to 3rd selection 'citta 1.3'
            gloss_tab.update_word_selection(0, 7, 2);
            var org_content = gloss_tab.gloss_as_orgmode();
            verify(org_content.includes("karitvā 4"));
            verify(org_content.includes("citta 1.3"));
        }

        function test_clean_word() {
            compare(gloss_tab.clean_word("Hello"), "hello");
            compare(gloss_tab.clean_word("!!!Hello!!!"), "hello");
            compare(gloss_tab.clean_word(" Word123 "), "word123");
            compare(gloss_tab.clean_word("@#$test@#$"), "test");
            compare(gloss_tab.clean_word(""), "");
            compare(gloss_tab.clean_word("!!!"), "");
        }

        function test_clean_word_pali_examples() {
            var test_words = [
                "‘sakkomi",
                "gantun’",
                "sampannasīlā,",
                "(Yathā",
                "vitthāretabbaṁ.)",
                "anāsavaṁ …",
            ];

            var cleaned_words = [];

            for (var i = 0; i < test_words.length; i++) {
                cleaned_words.push(gloss_tab.clean_word(test_words[i]));
            }

            var expected_words = [
                "sakkomi",
                "gantun",
                "sampannasīlā",
                "yathā",
                "vitthāretabbaṁ",
                "anāsavaṁ",
            ];

            compare(cleaned_words.join(" "), expected_words.join(" "));
        }

        function test_ai_translation_request_processing() {
            var paragraph = "Katamañca, bhikkhave, samādhindriyaṁ? Idha, bhikkhave, ariyasāvako vossaggārammaṇaṁ karitvā labhati samādhiṁ, labhati cittassa ekaggataṁ.";

            // Setup paragraph with text
            gloss_tab.gloss_text_input.text = paragraph;
            gloss_tab.update_all_glosses();

            verify(gloss_tab.paragraph_model.count > 0);
            var paragraph_data = gloss_tab.paragraph_model.get(0);

            // Test AI translation data structure
            verify(paragraph_data);
            compare(paragraph_data.text, paragraph);

            // Initially no translations
            var initial_translations = paragraph_data.translations_json || "[]";
            var parsed_initial = JSON.parse(initial_translations);
            compare(parsed_initial.length, 0);
        }

        function test_ai_translation_data_structure() {
            // Test sample translation data processing
            var sample_translations = [{
                model_name: "deepseek/deepseek-r1-0528:free",
                status: "completed",
                response: "What is the concentration faculty, monks?",
                request_id: "test_request_1",
                retry_count: 0,
                last_updated: Date.now(),
                user_selected: true
            }, {
                model_name: "google/gemma-3-12b-it:free",
                status: "waiting",
                response: "",
                request_id: "test_request_2",
                retry_count: 0,
                last_updated: Date.now(),
                user_selected: false
            }];

            // Add paragraph with translation data
            gloss_tab.paragraph_model.append({
                text: "Test paragraph",
                words_data_json: "[]",
                translations_json: JSON.stringify(sample_translations),
                selected_ai_tab: 0
            });

            verify(gloss_tab.paragraph_model.count > 0);
            var paragraph = gloss_tab.paragraph_model.get(gloss_tab.paragraph_model.count - 1);

            var translations = JSON.parse(paragraph.translations_json);
            compare(translations.length, 2);
            compare(translations[0].model_name, "deepseek/deepseek-r1-0528:free");
            compare(translations[0].status, "completed");
            compare(translations[1].status, "waiting");
            verify(translations[0].user_selected);
            verify(!translations[1].user_selected);
        }

        function test_error_response_detection() {
            // Test error detection functions
            verify(gloss_tab.is_error_response("API Error: Rate limit exceeded"));
            verify(gloss_tab.is_error_response("Error: Connection timeout"));
            verify(gloss_tab.is_error_response("Failed: Authentication failed"));
            verify(!gloss_tab.is_error_response("Normal translation response"));
            verify(!gloss_tab.is_error_response("Successfully translated text"));

            // Test rate limit specific detection
            verify(gloss_tab.is_rate_limit_error("API Error: Rate limit exceeded"));
            verify(!gloss_tab.is_rate_limit_error("API Error: Connection failed"));
            verify(!gloss_tab.is_rate_limit_error("Normal response"));
        }

        function test_request_id_generation() {
            var id1 = gloss_tab.generate_request_id();
            var id2 = gloss_tab.generate_request_id();

            // IDs should be unique
            verify(id1 !== id2);
            verify(id1.length > 10);
            verify(id1.includes("_"));

            // Should be timestamp + random
            var parts = id1.split("_");
            compare(parts.length, 2);
            verify(!isNaN(parseInt(parts[0]))); // timestamp part should be numeric
        }

        function test_export_with_ai_translations() {
            var paragraph = "Katamañca, bhikkhave, samādhindriyaṁ?";

            // Setup paragraph with glossing
            gloss_tab.gloss_text_input.text = paragraph;
            gloss_tab.update_all_glosses();

            verify(gloss_tab.paragraph_model.count > 0);

            // Add sample AI translations AFTER update_all_glosses to avoid being overridden
            var translations = [{
                model_name: "deepseek/deepseek-r1-0528:free",
                status: "completed",
                response: "What is the **concentration faculty**, monks?",
                request_id: "test_request_1",
                retry_count: 0,
                last_updated: Date.now(),
                user_selected: true
            }, {
                model_name: "google/gemma-3-12b-it:free",
                status: "completed",
                response: "What is the faculty of *concentration*, O monks?",
                request_id: "test_request_2",
                retry_count: 0,
                last_updated: Date.now(),
                user_selected: false
            }];

            gloss_tab.paragraph_model.setProperty(0, "translations_json", JSON.stringify(translations));

            // Test HTML export
            var html_content = gloss_tab.gloss_as_html();
            verify(html_content.length > 0);
            verify(html_content.includes("<html>"));
            verify(html_content.includes("Katamañca"));
            verify(html_content.includes("AI Translations"));
            verify(html_content.includes("deepseek/deepseek-r1-0528:free"));
            verify(html_content.includes("concentration faculty"));
            verify(html_content.includes("(selected)"));

            // Test Markdown export
            var markdown_content = gloss_tab.gloss_as_markdown();
            verify(markdown_content.length > 0);
            verify(markdown_content.includes("Katamañca"));
            verify(markdown_content.includes("### AI Translations"));
            verify(markdown_content.includes("#### deepseek/deepseek-r1-0528:free"));
            verify(markdown_content.includes("**concentration faculty**"));

            // Test Org-mode export
            var org_content = gloss_tab.gloss_as_orgmode();
            verify(org_content.length > 0);
            verify(org_content.includes("Katamañca"));
            verify(org_content.includes("*** AI Translations"));
            verify(org_content.includes("**** deepseek/deepseek-r1-0528:free"));
        }

        function test_translation_model_loading() {
            // Test model loading functionality
            gloss_tab.load_translation_models();

            // Should have loaded some models (depends on test environment)
            verify(gloss_tab.translation_models.count >= 0);

            // Check that models have required properties if any exist
            if (gloss_tab.translation_models.count > 0) {
                var first_model = gloss_tab.translation_models.get(0);
                verify(first_model.hasOwnProperty("model_name"));
                verify(first_model.hasOwnProperty("enabled"));
            }
        }

        function test_retry_request_handling() {
            // Setup paragraph with error translation
            var error_translations = [{
                model_name: "test/model:free",
                status: "error",
                response: "API Error: Connection timeout",
                request_id: "test_request_error",
                retry_count: 1,
                last_updated: Date.now(),
                user_selected: true
            }];

            gloss_tab.paragraph_model.append({
                text: "Test paragraph for retry",
                words_data_json: "[]",
                translations_json: JSON.stringify(error_translations),
                selected_ai_tab: 0
            });

            var paragraph_idx = gloss_tab.paragraph_model.count - 1;
            var paragraph = gloss_tab.paragraph_model.get(paragraph_idx);

            // Test retry request handling
            var new_request_id = gloss_tab.generate_request_id();
            gloss_tab.handle_retry_request(paragraph_idx, "test/model:free", new_request_id);

            // Check that request ID was updated
            paragraph = gloss_tab.paragraph_model.get(paragraph_idx);
            var updated_translations = JSON.parse(paragraph.translations_json);
            compare(updated_translations[0].request_id, new_request_id);
            compare(updated_translations[0].status, "waiting");
            compare(updated_translations[0].retry_count, 2);
        }

        function test_assistant_responses_integration() {
            var paragraph = "Test paragraph with AI responses";

            // Setup paragraph with AI translations
            var translations = [{
                model_name: "deepseek/deepseek-r1-0528:free",
                status: "completed",
                response: "First model response",
                request_id: "test_request_1",
                retry_count: 0,
                last_updated: Date.now(),
                user_selected: true
            }, {
                model_name: "google/gemma-3-12b-it:free",
                status: "error",
                response: "API Error: Rate limit exceeded",
                request_id: "test_request_2",
                retry_count: 1,
                last_updated: Date.now(),
                user_selected: false
            }];

            gloss_tab.paragraph_model.append({
                text: paragraph,
                words_data_json: "[]",
                translations_json: JSON.stringify(translations),
                selected_ai_tab: 0
            });

            verify(gloss_tab.paragraph_model.count > 0);
            var paragraph_data = gloss_tab.paragraph_model.get(gloss_tab.paragraph_model.count - 1);

            // Verify data is accessible for AssistantResponses component
            var parsed_translations = JSON.parse(paragraph_data.translations_json);
            compare(parsed_translations.length, 2);
            compare(parsed_translations[0].status, "completed");
            compare(parsed_translations[1].status, "error");

            // Test tab selection update
            gloss_tab.update_tab_selection(gloss_tab.paragraph_model.count - 1, 1, "google/gemma-3-12b-it:free");

            // Verify selection was updated via selected_ai_tab property
            paragraph_data = gloss_tab.paragraph_model.get(gloss_tab.paragraph_model.count - 1);
            compare(paragraph_data.selected_ai_tab, 1); // Should be set to index 1

            var updated_translations = JSON.parse(paragraph_data.translations_json);
            verify(updated_translations.length === 2);
            compare(updated_translations[0].status, "completed");
            compare(updated_translations[1].status, "error");
        }
    }
}
