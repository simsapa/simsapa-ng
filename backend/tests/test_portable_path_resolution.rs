//! Tests for portable-mode `SIMSAPA_DIR` resolution.
//!
//! These exercise the pure helpers `resolve_simsapa_dir` and
//! `normalize_lexically` (no GUI, no real DB, no filesystem access), covering
//! exe-relative resolution and lexical `..` normalization.

use std::path::{Path, PathBuf};

use simsapa_backend::{normalize_lexically, resolve_simsapa_dir};

#[test]
fn relative_value_resolves_against_exe_dir() {
    let exe_dir = PathBuf::from("/home/user/Desktop/Simsapa");
    let resolved = resolve_simsapa_dir("../SimsapaData", Some(exe_dir));
    assert_eq!(resolved, PathBuf::from("/home/user/Desktop/SimsapaData"));
}

#[test]
fn parent_dir_segments_are_normalized_lexically() {
    // The `..` must be collapsed without canonicalize() (no `\\?\` prefix, no
    // filesystem access).
    let normalized = normalize_lexically(Path::new("/a/b/c/../../d"));
    assert_eq!(normalized, PathBuf::from("/a/d"));

    // A leading `..` with nothing to pop is preserved verbatim.
    let normalized = normalize_lexically(Path::new("../SimsapaData"));
    assert_eq!(normalized, PathBuf::from("../SimsapaData"));

    // `.` segments are dropped.
    let normalized = normalize_lexically(Path::new("/a/./b"));
    assert_eq!(normalized, PathBuf::from("/a/b"));
}

#[test]
fn absolute_value_is_returned_unchanged() {
    #[cfg(windows)]
    let abs = r"C:\Data\Simsapa";
    #[cfg(not(windows))]
    let abs = "/opt/simsapa/data";

    // Even with an exe_dir present, an absolute value must be used as-is.
    let resolved = resolve_simsapa_dir(abs, Some(PathBuf::from("/some/exe/dir")));
    assert_eq!(resolved, PathBuf::from(abs));
}

#[test]
fn forward_slash_relative_path_works() {
    // The portable installer writes forward slashes; they must resolve
    // correctly when joined onto the exe dir.
    let exe_dir = PathBuf::from("/home/user/Desktop/Simsapa");
    let resolved = resolve_simsapa_dir("../SimsapaData", Some(exe_dir));
    assert_eq!(resolved, PathBuf::from("/home/user/Desktop/SimsapaData"));
}

#[test]
fn relative_value_without_exe_dir_falls_back_to_raw() {
    let resolved = resolve_simsapa_dir("../SimsapaData", None);
    assert_eq!(resolved, PathBuf::from("../SimsapaData"));
}
