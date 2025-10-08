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

    SignalSpy {
        id: allParagraphsSpy
        target: SuttaBridge
        signalName: "allParagraphsGlossReady"
    }

    TestCase {
        name: "TestGlossTabAnkiCsvExport"
        when: windowShown

        function cleanup() {
            gloss_tab.global_shown_stems = {};
            gloss_tab.no_duplicates_globally = true;
            allParagraphsSpy.clear();
        }

        function processTextBackground(text) {
            allParagraphsSpy.clear();
            gloss_tab.gloss_text_input.text = text;
            gloss_tab.start_background_all_glosses();

            var signalReceived = allParagraphsSpy.wait(1000);
            if (!signalReceived) {
                var paragraphs = text.split('\n\n').filter(p => p.trim() !== '');
                var mockResults = {
                    success: true,
                    paragraphs: [],
                    updated_global_stems: {},
                    global_unrecognized_words: []
                };

                for (var i = 0; i < paragraphs.length; i++) {
                    var mockWordsData = [];
                    if (paragraphs[i].includes("karitvā")) {
                        mockWordsData.push({
                            original_word: "karitvā",
                            results: [
                                { uid: "karitvā_1", word: "karitvā 1", summary: "<i>(ind)</i> having done, having made" }
                            ],
                            selected_index: 0,
                            stem: "karitvā 1",
                            example_sentence: "vossaggārammaṇaṁ <b>karitvā</b> labhati samādhiṁ"
                        });
                    }
                    if (paragraphs[i].includes("citta")) {
                        mockWordsData.push({
                            original_word: "cittassa",
                            results: [
                                { uid: "citta_1", word: "citta 1.1", summary: "<b>citta 1.1</b> <i>(nt)</i> mind, heart" }
                            ],
                            selected_index: 0,
                            stem: "citta 1.1",
                            example_sentence: "labhati <b>cittassa</b> ekaggataṁ"
                        });
                    }

                    mockResults.paragraphs.push({
                        paragraph_index: i,
                        words_data: mockWordsData,
                        unrecognized_words: []
                    });
                }

                gloss_tab.handle_all_paragraphs_results(mockResults);
            }
        }

        function test_anki_csv_simple_format() {
            var paragraph = "Idha, bhikkhave, ariyasāvako vossaggārammaṇaṁ karitvā labhati samādhiṁ, labhati cittassa ekaggataṁ.";
            processTextBackground(paragraph);

            var csv = gloss_tab.gloss_as_anki_csv("simple");
            
            verify(csv.length > 0, "CSV should not be empty");
            verify(csv.includes("karitvā"), "CSV should contain karitvā");
            verify(csv.includes("citta"), "CSV should contain citta");
            verify(csv.includes("having done, having made"), "CSV should contain definition");
            
            var lines = csv.split("\n");
            verify(lines.length >= 2, "Should have at least 2 CSV rows");
            
            var firstLine = lines[0];
            verify(firstLine.includes(","), "CSV rows should have comma separator");
            
            var parts = firstLine.split(",");
            verify(parts.length === 2, "Basic format should have front,back fields");
        }

        function test_anki_csv_cloze_format() {
            var paragraph = "Idha, bhikkhave, ariyasāvako vossaggārammaṇaṁ karitvā labhati samādhiṁ.";
            processTextBackground(paragraph);

            var csv = gloss_tab.gloss_as_anki_csv("cloze");
            
            verify(csv.length > 0, "CSV should not be empty");
            verify(csv.includes("{{c1::"), "Cloze format should have {{c1:: marker");
            verify(csv.includes("}}"), "Cloze format should have }} marker");
            
            var lines = csv.split("\n");
            for (var i = 0; i < lines.length; i++) {
                if (lines[i].trim() === "") continue;
                if (lines[i].includes("{{c1::")) {
                    verify(lines[i].includes("}}"), "Cloze markers should be paired");
                }
            }
        }

        function test_anki_csv_templated_format() {
            var paragraph = "Idha, bhikkhave, ariyasāvako vossaggārammaṇaṁ karitvā labhati samādhiṁ.";
            processTextBackground(paragraph);

            SuttaBridge.set_anki_template_front("Stem: {word_stem}");
            SuttaBridge.set_anki_template_back("Summary: {vocab.summary}");

            var csv = gloss_tab.gloss_as_anki_csv("templated");
            
            verify(csv.length > 0, "CSV should not be empty");
        }

        function test_anki_csv_data_format() {
            var paragraph = "Idha, bhikkhave, ariyasāvako vossaggārammaṇaṁ karitvā labhati samādhiṁ.";
            processTextBackground(paragraph);

            var csv = gloss_tab.gloss_as_anki_csv("data");
            
            verify(csv.length > 0, "CSV should not be empty");
            
            var lines = csv.split("\n");
            verify(lines.length >= 2, "Data CSV should have header + data rows");
            
            var header = lines[0];
            verify(header.includes("word_stem"), "Header should have word_stem");
            verify(header.includes("context_snippet"), "Header should have context_snippet");
            verify(header.includes("uid"), "Header should have uid");
            verify(header.includes("summary"), "Header should have summary");
        }

        function test_anki_csv_escaping() {
            gloss_tab.paragraph_model.clear();
            gloss_tab.paragraph_model.append({
                text: "Test paragraph",
                words_data_json: JSON.stringify([{
                    original_word: "test",
                    results: [{
                        uid: "test_1",
                        word: "test 1",
                        summary: 'Test "with quotes", commas, and\nnewlines'
                    }],
                    selected_index: 0,
                    stem: "test 1",
                    example_sentence: ""
                }]),
                translations_json: "[]",
                selected_ai_tab: 0
            });

            var csv = gloss_tab.gloss_as_anki_csv("simple");
            
            verify(csv.includes('""'), "Quotes should be escaped as double quotes");
            verify(csv.includes('"'), "Fields with special chars should be quoted");
        }

        function test_anki_csv_multiple_paragraphs() {
            var text = "Paragraph one karitvā.\n\nParagraph two cittassa.";
            processTextBackground(text);

            verify(gloss_tab.paragraph_model.count === 2, "Should have 2 paragraphs");

            var csv = gloss_tab.gloss_as_anki_csv("simple");
            
            verify(csv.includes("karitvā"), "Should include vocab from paragraph 1");
            verify(csv.includes("citta"), "Should include vocab from paragraph 2");
            
            var lines = csv.split("\n").filter(l => l.trim() !== "");
            verify(lines.length >= 2, "Should have rows from both paragraphs");
        }

        function test_anki_csv_clean_stem_in_export() {
            var paragraph = "Idha, bhikkhave, ariyasāvako vossaggārammaṇaṁ karitvā labhati samādhiṁ.";
            processTextBackground(paragraph);

            var csv = gloss_tab.gloss_as_anki_csv("simple");
            
            var lines = csv.split("\n");
            for (var i = 0; i < lines.length; i++) {
                if (lines[i].trim() === "") continue;
                var fields = lines[i].split(",");
                if (fields.length > 0 && fields[0].trim() !== "") {
                    verify(!fields[0].match(/\s+\d+(\.\d+)?$/), "Stem numbers should be removed from word_stem: " + fields[0]);
                }
            }
        }

        function test_anki_csv_with_context_snippet() {
            var paragraph = "Idha, bhikkhave, ariyasāvako vossaggārammaṇaṁ karitvā labhati samādhiṁ, labhati cittassa ekaggataṁ.";
            processTextBackground(paragraph);

            var csv = gloss_tab.gloss_as_anki_csv("simple");
            
            verify(csv.includes("<b>"), "Context snippet should contain bold markers for target word");
            verify(csv.includes("</b>"), "Context snippet should have closing bold tag");
        }
    }
}
