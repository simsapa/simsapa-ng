// Smoke test for the row-count helpers backing GET /health (Task 5.2).
// The /health handler itself lives in the bridges crate (cxx-qt, not
// unit-testable), so this pins that the count queries run against the real
// schema and return the populated shipped DBs' non-zero counts.

mod helpers;
use helpers as h;

use serial_test::serial;
use simsapa_backend::get_app_data;

#[test]
#[serial]
fn health_count_helpers_return_nonzero() {
    h::app_data_setup();
    let app_data = get_app_data();

    let suttas = app_data.dbm.appdata.count_suttas().expect("count_suttas should succeed");
    let dict_words = app_data.dbm.dictionaries.count_dict_words().expect("count_dict_words should succeed");
    let dpd_headwords = app_data.dbm.dpd.count_dpd_headwords().expect("count_dpd_headwords should succeed");

    assert!(suttas > 0, "expected suttas > 0, got {suttas}");
    assert!(dict_words > 0, "expected dict_words > 0, got {dict_words}");
    assert!(dpd_headwords > 0, "expected dpd_headwords > 0, got {dpd_headwords}");
}
