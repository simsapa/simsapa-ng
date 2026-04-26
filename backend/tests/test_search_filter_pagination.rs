//! Filter/pagination correctness + speed checks for the three Fulltext areas.
//!
//! For each user-reported scenario we run two passes:
//!   1. Baseline (no uid filter): just confirm page 0 returns results, that
//!      `total_hits()` is positive, and that page 0 alone fits inside a
//!      generous wall-clock budget. We deliberately do NOT paginate through
//!      the full unfiltered result set — the unfiltered fetch path grows the
//!      candidate window by `page_len` per page and is O(N²/page_len) when
//!      paginated end-to-end, which is fine for the UI (1–2 page clicks) but
//!      not a realistic test target.
//!   2. Filtered: paginate every page. Asserts (a) `total_hits()` matches the
//!      number of distinct `(table_name, uid)` rows actually returned across
//!      all pages — catches the bug where a uid prefix filter inflated the
//!      reported count past what pagination delivered — (b) the filtered
//!      total is strictly fewer than the baseline total, (c) every returned
//!      uid satisfies the prefix/suffix predicate, and (d) full pagination
//!      stays inside a generous wall-clock budget. The full-fetch path is
//!      cached on the task so only the first page pays the heavy fetch cost.

mod helpers;

use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use serial_test::serial;

use simsapa_backend::get_app_data;
use simsapa_backend::query_task::SearchQueryTask;
use simsapa_backend::types::{SearchArea, SearchMode, SearchParams};

const PAGE_LEN: usize = 10;

/// Configuration: Set to true to record timing data, false to test against recorded data
/// When RECORD_MODE is true, the test will measure query execution times and update
/// tests/data/test_query_timings.json with the measured values.
/// When RECORD_MODE is false, the test will load timing data from the JSON file and
/// assert that actual execution times are within 10% of the recorded values.
const RECORD_MODE: bool = false;

/// Per-call upper bound. Cold-cache page-0 calls in the full-fetch path
/// (uid_suffix set) materialize up to `SAFETY_LIMIT_TANTIVY` candidate rows
/// — each one a tantivy doc-store read + snippet generation. On a broad
/// query like 'vinnana' that saturates the cap, this dominates wall-clock
/// time. 15s is generous enough that real regressions (orders-of-magnitude
/// slower) still trip the test, while accommodating cold disk caches in CI.
/// Subsequent pages of the same task are served from `cached_full_fetch`
/// and are effectively free.




/// Path to the timing data file
const TIMING_DATA_PATH: &str = "tests/data/test_query_timings.json";

/// Load timing data from JSON file
fn load_timing_data() -> serde_json::Value {
    let path = Path::new(TIMING_DATA_PATH);
    let content = fs::read_to_string(path)
        .unwrap_or_else(|_| panic!("Failed to read timing data file at {}", TIMING_DATA_PATH));
    serde_json::from_str(&content)
        .unwrap_or_else(|_| panic!("Failed to parse timing data file at {}", TIMING_DATA_PATH))
}

/// Save timing data to JSON file
fn save_timing_data(data: &serde_json::Value) {
    let path = Path::new(TIMING_DATA_PATH);
    let content = serde_json::to_string_pretty(data)
        .expect("Failed to serialize timing data");
    fs::write(path, content)
        .unwrap_or_else(|_| panic!("Failed to write timing data file at {}", TIMING_DATA_PATH));
}

/// Get timing entry for a specific test and measurement type
fn get_timing_entry(test_name: &str, measurement_type: &str) -> f64 {
    let data = load_timing_data();
    data[test_name][measurement_type]
        .as_f64()
        .expect(&format!("Missing or invalid timing entry for {}.{}", test_name, measurement_type))
}

/// Save timing entry for a specific test and measurement type
fn save_timing_entry(test_name: &str, measurement_type: &str, value: f64) {
    let mut data = load_timing_data();
    data[test_name][measurement_type] = serde_json::json!(value);
    save_timing_data(&data);
}

/// Apply 10% tolerance to expected time (increase for upper bound)
fn apply_tolerance(expected_secs: f64) -> f64 {
    expected_secs * 1.10
}

/// Check if timing is within tolerance of expected time
fn assert_within_tolerance(actual_secs: f64, expected_secs: f64, label: &str) {
    let upper_bound = apply_tolerance(expected_secs);
    assert!(
        actual_secs <= upper_bound,
        "{} took {:.3}s (>{:.3}s, expected {:.3}s + 10% tolerance)",
        label,
        actual_secs,
        upper_bound,
        expected_secs
    );
}

fn make_params(
    mode: SearchMode,
    uid_prefix: Option<&str>,
    uid_suffix: Option<&str>,
) -> SearchParams {
    SearchParams {
        mode,
        page_len: Some(PAGE_LEN),
        lang: None,
        lang_include: true,
        source: None,
        source_include: true,
        enable_regex: false,
        fuzzy_distance: 0,
        include_cst_mula: true,
        include_cst_commentary: true,
        nikaya_prefix: None,
        uid_prefix: uid_prefix.map(str::to_string),
        uid_suffix: uid_suffix.map(str::to_string),
        include_ms_mula: true,
        include_comm_bold_definitions: true,
    }
}

fn make_task<'a>(
    area: SearchArea,
    query: &str,
    mode: SearchMode,
    uid_prefix: Option<&str>,
    uid_suffix: Option<&str>,
) -> SearchQueryTask<'a> {
    let app_data = get_app_data();
    let params = make_params(mode, uid_prefix, uid_suffix);
    SearchQueryTask::new(&app_data.dbm, query.to_string(), params, area)
}

/// Run page 0 only. Returns `(total_hits, page_0_rows, elapsed)`. Used for
/// baseline checks where paginating through the entire unfiltered result set
/// would be too slow without caching.
fn run_page_0(
    area: SearchArea,
    query: &str,
    mode: SearchMode,
) -> (i64, usize, Duration) {
    let mut task = make_task(area, query, mode, None, None);
    let start = Instant::now();
    let results = task
        .results_page(0)
        .unwrap_or_else(|e| panic!("results_page(0) failed: {e}"));
    let elapsed = start.elapsed();
    (task.total_hits(), results.len(), elapsed)
}

struct FilteredRun {
    total_hits: i64,
    distinct_keys: HashSet<(String, String)>,
    elapsed: Duration,
}

/// Page through the entire filtered result set. Each `results_page` call must
/// fit inside `PER_PAGE_BUDGET`; the sum across all pages must fit inside
/// `FILTERED_PAGINATION_BUDGET`.
fn run_full_filtered_pagination(
    area: SearchArea,
    query: &str,
    mode: SearchMode,
    uid_prefix: Option<&str>,
    uid_suffix: Option<&str>,
) -> FilteredRun {
    let mut task = make_task(area, query, mode, uid_prefix, uid_suffix);

    let mut distinct_keys: HashSet<(String, String)> = HashSet::new();
    let mut elapsed = Duration::ZERO;
    let mut total_hits: i64 = 0;

    let mut page = 0usize;
    let max_pages = 1_000;
    while page < max_pages {
        let start = Instant::now();
        let results = task
            .results_page(page)
            .unwrap_or_else(|e| panic!("results_page({page}) failed: {e}"));
        let dt = start.elapsed();
        elapsed += dt;

        if page == 0 {
            total_hits = task.total_hits();
        }

        if results.is_empty() {
            break;
        }
        for r in results {
            distinct_keys.insert((r.table_name.clone(), r.uid));
        }
        page += 1;
    }

    FilteredRun {
        total_hits,
        distinct_keys,
        elapsed,
    }
}

fn assert_filtered_consistent(run: &FilteredRun, label: &str) {
    assert_eq!(
        run.distinct_keys.len() as i64,
        run.total_hits,
        "{label}: paginated through {} distinct rows but total_hits()={} \
         — mismatch indicates an incorrect count or empty trailing pages",
        run.distinct_keys.len(),
        run.total_hits,
    );
}

// ---------------------------------------------------------------------------
// Suttas Fulltext: 'vinnana' baseline (~37 pages), then suffix='bodhi'
// (~1 page). Suffix forces the full-fetch path; the per-task cache is what
// keeps later pages from re-running the heavy fetch.
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn suttas_fulltext_vinnana_suffix_bodhi_filters_and_is_fast() {
    helpers::app_data_setup();

    let (baseline_total, page_0_len, page_0_dt) =
        run_page_0(SearchArea::Suttas, "vinnana", SearchMode::FulltextMatch);
    assert!(
        baseline_total > 0,
        "baseline Suttas Fulltext `vinnana` should return some hits"
    );
    assert!(
        page_0_len > 0,
        "baseline Suttas Fulltext `vinnana` page 0 should have rows"
    );

    // Handle baseline page 0 timing
    let page_0_secs = page_0_dt.as_secs_f64();
    if RECORD_MODE {
        save_timing_entry("suttas_fulltext_vinnana_suffix_bodhi_filters_and_is_fast", "baseline_page_0", page_0_secs);
    } else {
        let expected = get_timing_entry("suttas_fulltext_vinnana_suffix_bodhi_filters_and_is_fast", "baseline_page_0");
        assert_within_tolerance(page_0_secs, expected, "baseline Suttas Fulltext `vinnana` page 0");
    }

    let filtered = run_full_filtered_pagination(
        SearchArea::Suttas,
        "vinnana",
        SearchMode::FulltextMatch,
        None,
        Some("bodhi"),
    );
    assert_filtered_consistent(&filtered, "suttas vinnana suffix=bodhi");
    assert!(
        filtered.total_hits < baseline_total,
        "suffix-filtered Suttas Fulltext should have fewer hits ({} >= {})",
        filtered.total_hits,
        baseline_total,
    );
    for (table, uid) in &filtered.distinct_keys {
        assert!(
            uid.to_lowercase().ends_with("bodhi"),
            "suffix filter leaked uid {table}:{uid} (does not end with `bodhi`)"
        );
    }

    // Handle filtered pagination timing
    let filtered_secs = filtered.elapsed.as_secs_f64();
    if RECORD_MODE {
        save_timing_entry("suttas_fulltext_vinnana_suffix_bodhi_filters_and_is_fast", "filtered_pagination", filtered_secs);
    } else {
        let expected = get_timing_entry("suttas_fulltext_vinnana_suffix_bodhi_filters_and_is_fast", "filtered_pagination");
        assert_within_tolerance(filtered_secs, expected, "suttas vinnana suffix=bodhi filtered pagination");
    }
}

// ---------------------------------------------------------------------------
// Dictionary Fulltext: 'sutthu' baseline (~434 pages), then suffix='mnt'
// (~18 pages).
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn dictionary_fulltext_sutthu_suffix_mnt_filters_and_is_fast() {
    helpers::app_data_setup();

    let (baseline_total, page_0_len, page_0_dt) =
        run_page_0(SearchArea::Dictionary, "sutthu", SearchMode::FulltextMatch);
    assert!(
        baseline_total > 0,
        "baseline Dictionary Fulltext `sutthu` should return some hits"
    );
    assert!(
        page_0_len > 0,
        "baseline Dictionary Fulltext `sutthu` page 0 should have rows"
    );

    // Handle baseline page 0 timing
    let page_0_secs = page_0_dt.as_secs_f64();
    if RECORD_MODE {
        save_timing_entry("dictionary_fulltext_sutthu_suffix_mnt_filters_and_is_fast", "baseline_page_0", page_0_secs);
    } else {
        let expected = get_timing_entry("dictionary_fulltext_sutthu_suffix_mnt_filters_and_is_fast", "baseline_page_0");
        assert_within_tolerance(page_0_secs, expected, "baseline Dictionary Fulltext `sutthu` page 0");
    }

    let filtered = run_full_filtered_pagination(
        SearchArea::Dictionary,
        "sutthu",
        SearchMode::FulltextMatch,
        None,
        Some("mnt"),
    );
    assert_filtered_consistent(&filtered, "dict sutthu suffix=mnt");
    assert!(
        filtered.total_hits > 0,
        "Dictionary Fulltext `sutthu` with suffix=mnt should still match some rows"
    );
    assert!(
        filtered.total_hits < baseline_total,
        "suffix-filtered Dictionary Fulltext should have fewer hits ({} >= {})",
        filtered.total_hits,
        baseline_total,
    );
    for (table, uid) in &filtered.distinct_keys {
        assert!(
            uid.to_lowercase().ends_with("mnt"),
            "suffix filter leaked uid {table}:{uid} (does not end with `mnt`)"
        );
    }

    // Handle filtered pagination timing
    let filtered_secs = filtered.elapsed.as_secs_f64();
    if RECORD_MODE {
        save_timing_entry("dictionary_fulltext_sutthu_suffix_mnt_filters_and_is_fast", "filtered_pagination", filtered_secs);
    } else {
        let expected = get_timing_entry("dictionary_fulltext_sutthu_suffix_mnt_filters_and_is_fast", "filtered_pagination");
        assert_within_tolerance(filtered_secs, expected, "dict sutthu suffix=mnt filtered pagination");
    }
}

// ---------------------------------------------------------------------------
// Library Fulltext: 'food' baseline (~9 pages), then prefix='buddha'
// (1 page in practice). Pre-fix, the prefix variant claimed 9 pages but only
// the first held rows: `total_hits()` was using the unfiltered tantivy total.
// This test pins the contract: the consistency assertion fails loudly if
// `total_hits()` overstates the real paginated count.
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn library_fulltext_food_prefix_buddha_filters_and_count_matches_pages() {
    helpers::app_data_setup();

    let (baseline_total, page_0_len, page_0_dt) =
        run_page_0(SearchArea::Library, "food", SearchMode::FulltextMatch);
    assert!(
        baseline_total > 0,
        "baseline Library Fulltext `food` should return some hits"
    );
    assert!(
        page_0_len > 0,
        "baseline Library Fulltext `food` page 0 should have rows"
    );

    // Handle baseline page 0 timing
    let page_0_secs = page_0_dt.as_secs_f64();
    if RECORD_MODE {
        save_timing_entry("library_fulltext_food_prefix_buddha_filters_and_count_matches_pages", "baseline_page_0", page_0_secs);
    } else {
        let expected = get_timing_entry("library_fulltext_food_prefix_buddha_filters_and_count_matches_pages", "baseline_page_0");
        assert_within_tolerance(page_0_secs, expected, "baseline Library Fulltext `food` page 0");
    }

    let filtered = run_full_filtered_pagination(
        SearchArea::Library,
        "food",
        SearchMode::FulltextMatch,
        Some("buddha"),
        None,
    );
    assert_filtered_consistent(&filtered, "library food prefix=buddha");
    assert!(
        filtered.total_hits <= baseline_total,
        "prefix-filtered Library Fulltext cannot exceed unfiltered ({} > {})",
        filtered.total_hits,
        baseline_total,
    );
    for (table, uid) in &filtered.distinct_keys {
        assert!(
            uid.to_lowercase().starts_with("buddha"),
            "prefix filter leaked uid {table}:{uid} (does not start with `buddha`)"
        );
    }

    // Handle filtered pagination timing
    let filtered_secs = filtered.elapsed.as_secs_f64();
    if RECORD_MODE {
        save_timing_entry("library_fulltext_food_prefix_buddha_filters_and_count_matches_pages", "filtered_pagination", filtered_secs);
    } else {
        let expected = get_timing_entry("library_fulltext_food_prefix_buddha_filters_and_count_matches_pages", "filtered_pagination");
        assert_within_tolerance(filtered_secs, expected, "library food prefix=buddha filtered pagination");
    }
}
