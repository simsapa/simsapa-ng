import QtQuick
import QtTest

import com.profoundlabs.simsapa

Item {
    width: 800; height: 600

    GlossTab {
        id: gloss_tab
        window_id: "window_0"
        is_dark: false
        anchors.centerIn: parent
    }

    SuttaBridge { id: sb }

    Component.onCompleted: {
        console.log("Running Gloss Widget Tests...");
    }

    TestCase {
        name: "TestGlossTab"
        when: windowShown

        function cleanup() {
            // Reset any global state after each test
            gloss_tab.global_shown_stems = {};
            gloss_tab.no_duplicates_globally = true;
        }

        function test_extract_words() {
            var result = gloss_tab.extract_words("Hello world test");
            compare(result.length, 3);
            compare(result[0], "Hello");
            compare(result[1], "world");
            compare(result[2], "test");

            // Test punctuation
            result = gloss_tab.extract_words("Hello, world!");
            compare(result.length, 2);
            compare(result[0], "Hello,");
            compare(result[1], "world!");

            // Test empty string
            result = gloss_tab.extract_words("");
            compare(result.length, 0);

            // Unicode text
            result = gloss_tab.extract_words("Pāḷi ñāṇa");
            compare(result.length, 2);
            compare(result[0], "Pāḷi");
            compare(result[1], "ñāṇa");

            // Multiple spaces
            result = gloss_tab.extract_words("word1    word2");
            compare(result.length, 2);

            // Filter punctuation and non-words
            // result = gloss_tab.extract_words("(48.50) samādhi1 ... hey ho! !!"); FIXME
            // compare(result.length, 3);
            // compare(result[0], "samādhi1");
            // compare(result[1], "hey");
            // compare(result[2], "ho!");
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
            let json = sb.dpd_lookup_json("cittassa");
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
            // console.log(result_words);
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

        function test_clean_word() {
            compare(gloss_tab.clean_word("Hello"), "hello");
            compare(gloss_tab.clean_word("!!!Hello!!!"), "hello");
            compare(gloss_tab.clean_word(" Word123 "), "word123");
            compare(gloss_tab.clean_word("@#$test@#$"), "test");
            compare(gloss_tab.clean_word(""), "");
            compare(gloss_tab.clean_word("!!!"), "");
        }

        function test_clean_word_pali_examples() {
            let test_words = [
                "‘sakkomi",
                "gantun’",
                "sampannasīlā,",
                "(Yathā",
                "vitthāretabbaṁ.)",
                "anāsavaṁ …",
            ];

            let cleaned_words = [];

            for (var i = 0; i < test_words.length; i++) {
                cleaned_words.push(gloss_tab.clean_word(test_words[i]));
            }

            let expected_words = [
                "sakkomi",
                "gantun",
                "sampannasīlā",
                "yathā",
                "vitthāretabbaṁ",
                "anāsavaṁ",
            ];

            compare(cleaned_words.join(" "), expected_words.join(" "));
        }
    }
}
