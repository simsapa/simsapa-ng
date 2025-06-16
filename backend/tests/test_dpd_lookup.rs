use simsapa_backend::get_app_data;

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
