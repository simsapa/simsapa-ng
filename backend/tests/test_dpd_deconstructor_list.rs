use simsapa_backend::db;

mod helpers;
use helpers as h;

#[test]
fn test_dpd_deconstructor_list() {
    h::appdata_db_setup();
    let dbm = db::get_dbm();

    let query = "olokitasaññāṇeneva";
    let result = dbm.dpd.dpd_deconstructor_list(query);

    let expected: Vec<String> = r#"
olokita + saññāṇena + eva
olokita + saññāṇena + iva
olokita + saññā + ṇena + eva
olokitā + asaññā + ṇena + eva
"#.trim().split("\n").map(|i| i.to_string()).collect();

    assert_eq!(result.len(), expected.len());

    for (idx, result_i) in result.iter().enumerate() {
        assert_eq!(result_i.to_string(), expected[idx].to_string());
    }
}
