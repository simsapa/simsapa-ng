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
use std::time::{Duration, Instant};

use serial_test::serial;

use simsapa_backend::get_app_data;
use simsapa_backend::query_task::SearchQueryTask;
use simsapa_backend::types::{SearchArea, SearchMode, SearchParams};

const PAGE_LEN: usize = 10;

/// Per-call upper bound. Cold-cache page-0 calls in the full-fetch path
/// (uid_suffix set) materialize up to `SAFETY_LIMIT_TANTIVY` candidate rows
/// — each one a tantivy doc-store read + snippet generation. On a broad
/// query like 'vinnana' that saturates the cap, this dominates wall-clock
/// time. 15s is generous enough that real regressions (orders-of-magnitude
/// slower) still trip the test, while accommodating cold disk caches in CI.
/// Subsequent pages of the same task are served from `cached_full_fetch`
/// and are effectively free.
const PER_PAGE_BUDGET: Duration = Duration::from_secs(15);

/// Upper bound for paging through a full filtered result set. The cached
/// full-fetch path means only page 0 pays the heavy fetch; subsequent pages
/// are pure slicing, so this is essentially `PER_PAGE_BUDGET` with headroom.
const FILTERED_PAGINATION_BUDGET: Duration = Duration::from_secs(30);

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
        assert!(
            dt < PER_PAGE_BUDGET,
            "page {page} took {:?} (> {:?}) for query {:?} prefix={:?} suffix={:?}",
            dt,
            PER_PAGE_BUDGET,
            query,
            uid_prefix,
            uid_suffix,
        );
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
    assert!(
        run.elapsed < FILTERED_PAGINATION_BUDGET,
        "{label}: full filtered pagination took {:?} (> {:?})",
        run.elapsed,
        FILTERED_PAGINATION_BUDGET,
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
    assert!(
        page_0_dt < PER_PAGE_BUDGET,
        "baseline Suttas Fulltext `vinnana` page 0 took {:?} (> {:?})",
        page_0_dt,
        PER_PAGE_BUDGET,
    );

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
    assert!(
        page_0_dt < PER_PAGE_BUDGET,
        "baseline Dictionary Fulltext `sutthu` page 0 took {:?} (> {:?})",
        page_0_dt,
        PER_PAGE_BUDGET,
    );

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
    assert!(
        page_0_dt < PER_PAGE_BUDGET,
        "baseline Library Fulltext `food` page 0 took {:?} (> {:?})",
        page_0_dt,
        PER_PAGE_BUDGET,
    );

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
}
