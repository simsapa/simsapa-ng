//! Shared, range-based snippet highlighter.
//!
//! This is the only code that writes `<span class='match'>` markup. Producers
//! (the Fulltext `render_snippet` and the Contains `db_*_to_result` handlers)
//! choose *which* byte ranges to highlight and call [`wrap_ranges`], which emits
//! exactly one span per merged range — **non-nested by construction**. See
//! `docs/search-snippet-highlight-pipeline.md` for the full design (per-mode
//! semantics, the central-pass fallback, focal-only highlighting).

use std::ops::Range;

/// Sort and coalesce overlapping or adjacent ranges into a minimal disjoint
/// set. Empty (`start >= end`) ranges are dropped. Adjacent ranges
/// (`a.end == b.start`) are merged so two touching matches become one span.
pub fn merge_ranges(mut ranges: Vec<Range<usize>>) -> Vec<Range<usize>> {
    ranges.retain(|r| r.start < r.end);
    if ranges.is_empty() {
        return ranges;
    }
    ranges.sort_by(|a, b| a.start.cmp(&b.start).then(a.end.cmp(&b.end)));

    let mut merged: Vec<Range<usize>> = Vec::with_capacity(ranges.len());
    let mut cur = ranges[0].clone();
    for r in ranges.into_iter().skip(1) {
        if r.start <= cur.end {
            // Overlapping or adjacent — extend the current range.
            if r.end > cur.end {
                cur.end = r.end;
            }
        } else {
            merged.push(cur);
            cur = r;
        }
    }
    merged.push(cur);
    merged
}

/// Wrap each (merged) range in a single `<span class='match'>…</span>`. Ranges
/// are byte ranges into `text` and must fall on char boundaries (the producers
/// derive them from char-boundary-safe sources — tantivy's `highlighted()`,
/// [`literal_ranges`], and `fragment_around_offset`). The input is merged here
/// defensively, so the output can never contain nested match spans.
pub fn wrap_ranges(text: &str, ranges: &[Range<usize>]) -> String {
    let merged = merge_ranges(ranges.to_vec());
    if merged.is_empty() {
        return text.to_string();
    }

    let mut out = String::with_capacity(text.len() + merged.len() * 28);
    let mut last = 0usize;
    for r in merged {
        // Defensive bounds/boundary check: skip a range we can't slice safely
        // rather than panic.
        if r.start < last
            || r.end > text.len()
            || !text.is_char_boundary(r.start)
            || !text.is_char_boundary(r.end)
        {
            continue;
        }
        out.push_str(&text[last..r.start]);
        out.push_str("<span class='match'>");
        out.push_str(&text[r.start..r.end]);
        out.push_str("</span>");
        last = r.end;
    }
    out.push_str(&text[last..]);
    out
}

/// All non-overlapping, case-insensitive byte ranges of `term` in `text`.
///
/// `text` and `term` are expected to already be normalized (lowercase) by the
/// caller; matching is still done case-insensitively for safety. Byte offsets
/// from the lowercased haystack are treated as valid in the original `text`,
/// the same convention used by `fragment_around_text` — this holds because the
/// search corpus is lowercase-normalized Pāli/Latin where lowercasing preserves
/// byte length.
pub fn literal_ranges(text: &str, term: &str) -> Vec<Range<usize>> {
    let term = term.trim();
    if term.is_empty() {
        return Vec::new();
    }
    let hay = text.to_lowercase();
    let needle = term.to_lowercase();
    // If lowercasing changed the byte length, offsets would no longer map back
    // to `text`; bail out rather than produce wrong ranges.
    if hay.len() != text.len() || needle.is_empty() {
        return Vec::new();
    }

    let mut ranges = Vec::new();
    let mut start = 0usize;
    while let Some(pos) = hay[start..].find(&needle) {
        let s = start + pos;
        let e = s + needle.len();
        if text.is_char_boundary(s) && text.is_char_boundary(e) {
            ranges.push(s..e);
        }
        start = e;
    }
    ranges
}

/// A single focal range `[offset, offset+len)`, clamped to `text` bounds and
/// char boundaries. Used by all-snippets mode to highlight only the occurrence
/// a given snippet is for. Returns an empty vec if the offset is out of range.
pub fn focal_range(text: &str, offset: usize, len: usize) -> Vec<Range<usize>> {
    if offset >= text.len() || !text.is_char_boundary(offset) {
        return Vec::new();
    }
    let mut end = (offset + len).min(text.len());
    while end > offset && !text.is_char_boundary(end) {
        end -= 1;
    }
    if end <= offset {
        return Vec::new();
    }
    vec![offset..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_ranges_overlapping_adjacent_disjoint() {
        // Disjoint stay separate.
        assert_eq!(merge_ranges(vec![0..2, 5..7]), vec![0..2, 5..7]);
        // Overlapping coalesce.
        assert_eq!(merge_ranges(vec![0..4, 2..6]), vec![0..6]);
        // Adjacent coalesce.
        assert_eq!(merge_ranges(vec![0..2, 2..4]), vec![0..4]);
        // Unsorted input, with an empty range dropped.
        assert_eq!(merge_ranges(vec![5..7, 3..3, 0..2]), vec![0..2, 5..7]);
        // Nested coalesce to the outer.
        assert_eq!(merge_ranges(vec![0..10, 3..5]), vec![0..10]);
    }

    #[test]
    fn wrap_ranges_is_non_nested() {
        let s = wrap_ranges("abcdef", &[0..2, 1..3]);
        // Overlap merged into one span — never nested.
        assert_eq!(s, "<span class='match'>abc</span>def");
        assert!(!s.contains("class='match'><span"));
        assert_eq!(s.matches("class='match'").count(), 1);
    }

    #[test]
    fn literal_ranges_all_occurrences() {
        let text = "pajahati na upādiyati pajahati ṭhito";
        let r = literal_ranges(text, "pajahati");
        assert_eq!(r.len(), 2);
        let wrapped = wrap_ranges(text, &r);
        assert_eq!(wrapped.matches("class='match'").count(), 2);
        assert!(!wrapped.contains("class='match'><span"));
    }

    #[test]
    fn literal_ranges_does_not_match_inflection() {
        // Contains semantics: 'pajahati' must not match 'pajahitvā'.
        let text = "pajahitvā ṭhito";
        assert!(literal_ranges(text, "pajahati").is_empty());
    }

    #[test]
    fn focal_range_clamps() {
        let text = "pajahati ṭhito";
        let r = focal_range(text, 0, 8);
        assert_eq!(r, vec![0..8]);
        // Out of range → empty.
        assert!(focal_range(text, text.len(), 3).is_empty());
    }
}
