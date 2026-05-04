//! Integration tests for the incremental (prefix-search) index.

use std::path::Path;
use tempfile::tempdir;

use ldoce5viewer_tui::search::incremental::{IncrementalMaker, IncrementalSearcher, normalize_index_key};

// --------------------------------------------------------------------------
// Helper
// --------------------------------------------------------------------------

fn build_index(dir: &Path, items: &[(&str, &str, &str, &str, u8)]) -> std::path::PathBuf {
    let idx_path = dir.join("inc.dat");
    let tmp_path = dir.join("inc.tmp");
    let mut maker = IncrementalMaker::new(&idx_path, &tmp_path).unwrap();
    for &(plain, tc, label, path, prio) in items {
        maker.add_item(plain, tc, label, path, prio).unwrap();
    }
    maker.finalize().unwrap();
    idx_path
}

// --------------------------------------------------------------------------
// Tests
// --------------------------------------------------------------------------

#[test]
fn test_round_trip_prefix_match() {
    let dir = tempdir().unwrap();
    let idx = build_index(dir.path(), &[
        ("apple",       "hw", "Apple",       "/fs/apple",       0),
        ("application", "hw", "Application", "/fs/application", 0),
        ("apply",       "hw", "Apply",       "/fs/apply",       1),
        ("banana",      "hw", "Banana",      "/fs/banana",      0),
    ]);
    let searcher = IncrementalSearcher::open(&idx).unwrap();
    let results = searcher.search("appl", 100);
    let labels: Vec<&str> = results.iter().map(|r| r.label.as_str()).collect();
    assert!(labels.contains(&"Apple"),       "Apple should match 'appl'");
    assert!(labels.contains(&"Application"), "Application should match 'appl'");
    assert!(labels.contains(&"Apply"),       "Apply should match 'appl'");
    assert!(!labels.contains(&"Banana"),     "Banana should NOT match 'appl'");
}

#[test]
fn test_exact_match() {
    let dir = tempdir().unwrap();
    let idx = build_index(dir.path(), &[
        ("run",  "hw", "Run",  "/fs/run",  0),
        ("runs", "hw", "Runs", "/fs/runs", 0),
    ]);
    let searcher = IncrementalSearcher::open(&idx).unwrap();
    let results = searcher.search("run", 100);
    let labels: Vec<&str> = results.iter().map(|r| r.label.as_str()).collect();
    assert!(labels.contains(&"Run"),  "exact match 'run' missing");
    assert!(labels.contains(&"Runs"), "'runs' is a prefix match of 'run' and should be included");
}

#[test]
fn test_no_match() {
    let dir = tempdir().unwrap();
    let idx = build_index(dir.path(), &[("elephant", "hw", "Elephant", "/fs/e", 0)]);
    let searcher = IncrementalSearcher::open(&idx).unwrap();
    assert!(searcher.search("xyz", 100).is_empty());
}

#[test]
fn test_empty_query() {
    let dir = tempdir().unwrap();
    let idx = build_index(dir.path(), &[("word", "hw", "Word", "/fs/word", 0)]);
    let searcher = IncrementalSearcher::open(&idx).unwrap();
    assert!(searcher.search("", 100).is_empty());
}

#[test]
fn test_limit_respected() {
    let dir = tempdir().unwrap();
    let items: Vec<(String, String, String, String, u8)> = (0u32..50)
        .map(|i| (format!("word{i:03}"), "hw".to_string(), format!("Word{i}"), format!("/fs/w{i}"), 0))
        .collect();
    let refs: Vec<(&str, &str, &str, &str, u8)> = items
        .iter()
        .map(|(a, b, c, d, e)| (a.as_str(), b.as_str(), c.as_str(), d.as_str(), *e))
        .collect();
    let idx = build_index(dir.path(), &refs);
    let searcher = IncrementalSearcher::open(&idx).unwrap();
    let results = searcher.search("word", 10);
    assert_eq!(results.len(), 10, "limit should be respected");
}

#[test]
fn test_accent_normalisation() {
    let dir = tempdir().unwrap();
    let idx = build_index(dir.path(), &[("café", "hw", "Café", "/fs/cafe", 0)]);
    let searcher = IncrementalSearcher::open(&idx).unwrap();
    // Searching for "cafe" should match the accented entry because both
    // are normalised to "cafe"
    let results = searcher.search("cafe", 10);
    assert!(!results.is_empty(), "normalised 'cafe' should match 'café'");
}

#[test]
fn test_sort_order_prio() {
    let dir = tempdir().unwrap();
    // Same plain text, different priority
    let idx = build_index(dir.path(), &[
        ("run", "hw", "Run (prio=2)", "/fs/run2", 2),
        ("run", "hw", "Run (prio=0)", "/fs/run0", 0),
        ("run", "hw", "Run (prio=1)", "/fs/run1", 1),
    ]);
    let searcher = IncrementalSearcher::open(&idx).unwrap();
    let results = searcher.search("run", 10);
    // Should be sorted: prio=0 first, then 1, then 2
    let prios: Vec<u8> = results.iter().map(|r| r.prio).collect();
    for w in prios.windows(2) {
        assert!(w[0] <= w[1], "results not sorted by prio: {prios:?}");
    }
}

#[test]
fn test_result_fields() {
    let dir = tempdir().unwrap();
    let idx = build_index(dir.path(), &[
        ("example", "hw", "Example Label", "/dict/example", 3),
    ]);
    let searcher = IncrementalSearcher::open(&idx).unwrap();
    let results = searcher.search("example", 1);
    assert_eq!(results.len(), 1);
    let r = &results[0];
    assert_eq!(r.label, "Example Label");
    assert_eq!(r.path, "/dict/example");
    assert_eq!(r.prio, 3);
    assert_eq!(r.typecode, "hw");
}

#[test]
fn test_normalize_index_key() {
    assert_eq!(normalize_index_key("Hello"),    "hello");
    assert_eq!(normalize_index_key("  café  "), "cafe");
    assert_eq!(normalize_index_key(""),          "");
    assert_eq!(normalize_index_key("über"),      "uber");
    // Copyright sign maps to 'c'
    assert_eq!(normalize_index_key("\u{00a9}"),  "c");
    // Numbers preserved
    assert_eq!(normalize_index_key("mp3"),       "mp3");
}
