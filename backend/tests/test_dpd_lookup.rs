use std::fs;
use std::path::PathBuf;
use std::collections::{HashMap, BTreeMap};

use simsapa_backend::get_app_data;
use simsapa_backend::helpers::{extract_words, normalize_query_text};
use simsapa_backend::db::dpd::LookupResult;

mod helpers;
use helpers as h;

#[test]
fn test_dpd_lookup_list() {
    h::app_data_setup();
    let app_data = get_app_data();

    let query = "olokitasaññāṇeneva";
    let result = app_data.dbm.dpd.dpd_lookup_list(query);

    let expected: Vec<String> = r#"
<b>olokita</b> (pp) looked at; observed; viewed (by) <b>[ava + √lok + ita]</b> <i>pp of oloketi</i>
<b>saññāṇa 1</b> (nt) marking; signing <b>[saṁ + √ñā + aṇa]</b> <i>nt, act, from sañjānāti</i>
<b>saññāṇa 2</b> (nt) mental noting <b>[saṁ + √ñā + aṇa]</b> <i>nt, act, from sañjānāti</i>
<b>eva 1</b> (ind) only; just; merely; exclusively <i>ind, emph</i>
<b>eva 2</b> (ind) still <i>ind</i>
<b>eva 3</b> (ind) even; too; as well <i>ind, adv</i>
<b>eva 4</b> (ind) indeed, really, certainly, absolutely <i>ind</i>
<b>eva 5</b> (ind) as soon as <i>ind, emph</i>
<b>iva</b> (ind) like; as <i>ind</i>
<b>sañña</b> (adj) perceiving; having perception; regarding (as) <b>[saṁ + √ñā + ā + a], [saññā + a]</b> <i>adj, in comps, from saññā</i>
<b>saññā 1</b> (fem) perception; conception; recognition; third of the five aggregates <b>[saṁ + √ñā + ā]</b> <i>fem, abstr, from sañjānāti</i>
<b>saññā 2</b> (fem) name; label; concept; idea; notion; impression; representation; symbol <b>[saṁ + √ñā + ā]</b> <i>fem, abstr, from sañjānāti</i>
<b>saññā 3</b> (fem) sign; signal; gesture <b>[saṁ + √ñā + ā]</b> <i>fem, from sañjānāti</i>
<b>saññā 5</b> (fem) (gram) grammatical term; technical term; definition <b>[saṁ + √ñā + ā]</b> <i>fem, abstr, from sañjānāti</i>
<b>saññā 4</b> (fem) consciousness; awareness <b>[saṁ + √ñā + ā]</b> <i>fem, abstr, from sañjānāti</i>
<b>ṇa</b> (masc) (gram) indicatory letter ṇ; sign indicating that vuddhi takes place <i>masc, prefix</i>
<b>asañña 1</b> (adj) impercipient; without consciousness; senseless <b>[na > a + saṁ + √ñā + ā + a], [asaññā + a]</b> <i>adj, from na saññā</i>
<b>asañña 2</b> (adj) without perception; with no recognition <b>[na > a + saṁ + √ñā + ā + a], [asaññā + a]</b> <i>adj, from na saññā</i>
<b>asaññā 1</b> (fem) non-perception; non-conception; non-recognition <b>[na > a + saṁ + √ñā + ā]</b> <i>fem, abstr, from na sañjānāti</i>
<b>asaññā 2</b> (fem) name of a type of lightning <b>[na > a + saṁ + √ñā + ā]</b> <i>fem, from na sañjānāti</i>
"#.trim().split("\n").map(|i| i.to_string()).collect();

    assert_eq!(result.len(), expected.len());

    for (idx, result_i) in result.iter().enumerate() {
        assert_eq!(result_i.to_string(), expected[idx].to_string());
    }
}

#[test]
fn test_dpd_lookup_generate_json() {
    h::app_data_setup();
    let app_data = get_app_data();

    let mut texts: HashMap<&str, &str> = HashMap::new();
    texts.insert("dpd_lookup"                         , "Katamañca, bhikkhave, samādhindriyaṁ? Idha, bhikkhave, ariyasāvako vossaggārammaṇaṁ karitvā labhati samādhiṁ, labhati cittassa ekaggataṁ. So vivicceva kāmehi vivicca akusalehi dhammehi savitakkaṁ savicāraṁ vivekajaṁ pītisukhaṁ paṭhamaṁ jhānaṁ upasampajja viharati. / Saddhassa hi, sāriputta, ariyasāvakassa āraddhavīriyassa upaṭṭhitassatino etaṁ pāṭikaṅkhaṁ yaṁ vossaggārammaṇaṁ karitvā labhissati samādhiṁ, labhissati cittassa ekaggataṁ. Yo hissa, sāriputta, samādhi tadassa samādhindriyaṁ.");
    texts.insert("yam-janna"                          , "yaṁ jaññā — ‘sakkomi ajjeva gantun’ti.");
    texts.insert("anumattesu-vajjesu"                 , "aṇumattesu vajjesu bhayadassāvino, samādāya sikkhatha sikkhāpadesū’ti");
    texts.insert("anasavanca-vo"                      , "“Anāsavañca vo, bhikkhave, desessāmi anāsavagāmiñca maggaṁ. Taṁ suṇātha. Katamañca, bhikkhave, anāsavaṁ …pe….");
    texts.insert("suriyassa-bhikkhave"                , "“Sūriyassa, bhikkhave, udayato");
    texts.insert("yatha-asankhatam"                   , "(Yathā asaṅkhataṁ tathā vitthāretabbaṁ.)");
    texts.insert("parens-48.10-katamanca-bhikkhave"   , "(SN 48.10) Katamañca, bhikkhave, samādhindriyaṁ?");
    texts.insert("brackets-48.10-katamanca-bhikkhave" , "[SN 48:10] Katamañca, bhikkhave, samādhindriyaṁ?");
    texts.insert("te-jananti"                         , "Te jānanti atthaññe āvāsikā bhikkhū");

    texts.insert("idha-nandati", r#"
18.

idha nandati pecca nandati, katapuñño ubhayattha nandati.

‘‘puññaṁ me kata’’nti nandati, bhiyyo nandati suggatiṁ gato..
"#);

    texts.insert("gataddhino", r#"
Gataddhino visokassa,
vippamuttassa sabbadhi;
Sabbaganthappahīnassa,
pariḷāho na vijjati.
"#);

    for (file_name, quote) in texts.into_iter() {
        // Use a BTreeMap for consistent key sorting across test runs.
        let mut lookup_data: BTreeMap<String, Vec<LookupResult>> = BTreeMap::new();

        for word in extract_words(quote) {
            if word.len() <= 1 {
                continue;
            }
            let word = normalize_query_text(Some(word.to_string()));
            let res = app_data.dbm.dpd.dpd_lookup(&word, false, true).unwrap();
            lookup_data.insert(word, LookupResult::from_search_results(&res));
        }

        let json = serde_json::to_string_pretty(&lookup_data).expect("Can't encode JSON");

        let path = PathBuf::from(format!("tests/data/{}.json", file_name));
        // fs::write(&path, json.clone()).expect("Unable to write file!");

        let expected_json = fs::read_to_string(&path).expect("Failed to read file");

        assert_eq!(json, expected_json);
    }
}
