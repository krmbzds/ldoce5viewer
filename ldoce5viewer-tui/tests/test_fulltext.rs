//! Integration tests for the Tantivy full-text search engine.

use std::path::Path;
use tempfile::tempdir;

use ldoce5viewer_tui::search::fulltext::{FulltextMaker, FulltextSearcher};

// --------------------------------------------------------------------------
// Helper
// --------------------------------------------------------------------------

fn make_index(dir: &Path) -> FulltextMaker {
    FulltextMaker::new(dir).unwrap()
}

// --------------------------------------------------------------------------
// Tests
// --------------------------------------------------------------------------

#[test]
fn test_basic_search_and_recall() {
    let dir = tempdir().unwrap();
    let mut maker = make_index(dir.path());
    maker
        .add_item("hm", "run", "", "Run", "/fs/run", 0, "run")
        .unwrap();
    maker
        .add_item("hm", "walk", "", "Walk", "/fs/walk", 0, "walk")
        .unwrap();
    maker.commit().unwrap();

    let searcher = FulltextSearcher::open(dir.path()).unwrap();
    let results = searcher.search(Some("run"), &[], None, None).unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0].label, "Run");
    assert_eq!(results[0].path, "/fs/run");
}

#[test]
fn test_no_results_for_unknown_word() {
    let dir = tempdir().unwrap();
    let mut maker = make_index(dir.path());
    maker
        .add_item("hm", "apple", "", "Apple", "/fs/apple", 0, "apple")
        .unwrap();
    maker.commit().unwrap();

    let searcher = FulltextSearcher::open(dir.path()).unwrap();
    let results = searcher.search(Some("xyzzy"), &[], None, None).unwrap();
    assert!(results.is_empty());
}

#[test]
fn test_itemtype_filter_restricts_results() {
    let dir = tempdir().unwrap();
    let mut maker = make_index(dir.path());
    maker
        .add_item("hm", "run fast", "", "Run (hw)", "/fs/run_hw", 0, "run")
        .unwrap();
    maker
        .add_item(
            "e",
            "run quickly",
            "",
            "Run (example)",
            "/fs/run_exa",
            1,
            "run",
        )
        .unwrap();
    maker.commit().unwrap();

    let searcher = FulltextSearcher::open(dir.path()).unwrap();
    let results = searcher.search(Some("run"), &["hm"], None, None).unwrap();
    // Only headword type should be returned
    assert!(!results.is_empty());
    assert!(
        results.iter().all(|r| !r.path.contains("exa")),
        "example type should be filtered out: {results:?}"
    );
}

#[test]
fn test_multiple_itemtypes() {
    let dir = tempdir().unwrap();
    let mut maker = make_index(dir.path());
    maker
        .add_item("hm", "jump", "", "Jump (hw)", "/fs/jump_hw", 0, "jump")
        .unwrap();
    maker
        .add_item(
            "hp",
            "jump at",
            "",
            "Jump at (phrase)",
            "/fs/jump_ph",
            0,
            "jump",
        )
        .unwrap();
    maker
        .add_item("e", "she jumped", "", "Example", "/fs/jump_exa", 0, "jump")
        .unwrap();
    maker.commit().unwrap();

    let searcher = FulltextSearcher::open(dir.path()).unwrap();
    let results = searcher
        .search(Some("jump"), &["hm", "hp"], None, None)
        .unwrap();
    // Both hm and hp should be returned but not e
    let paths: Vec<&str> = results.iter().map(|r| r.path.as_str()).collect();
    assert!(!results.is_empty());
    assert!(
        paths.iter().all(|p| !p.contains("exa")),
        "examples should be filtered"
    );
}

#[test]
fn test_results_sorted_by_sortkey() {
    let dir = tempdir().unwrap();
    let mut maker = make_index(dir.path());
    maker
        .add_item("hm", "zoo", "", "Zoo", "/fs/zoo", 0, "zoo")
        .unwrap();
    maker
        .add_item(
            "hm",
            "aardvark",
            "",
            "Aardvark",
            "/fs/aardvark",
            0,
            "aardvark",
        )
        .unwrap();
    maker
        .add_item("hm", "middle", "", "Middle", "/fs/middle", 0, "middle")
        .unwrap();
    maker.commit().unwrap();

    let searcher = FulltextSearcher::open(dir.path()).unwrap();
    let results = searcher.search(None, &[], None, None).unwrap();
    assert!(results.len() >= 3);
    for w in results.windows(2) {
        assert!(w[0].sortkey <= w[1].sortkey, "not sorted by sortkey");
    }
}

#[test]
fn test_limit_applied() {
    let dir = tempdir().unwrap();
    let mut maker = make_index(dir.path());
    for i in 0..30u32 {
        maker
            .add_item(
                "hm",
                &format!("word{i}"),
                "",
                &format!("W{i}"),
                &format!("/fs/w{i}"),
                i as u64,
                &format!("word{i}"),
            )
            .unwrap();
    }
    maker.commit().unwrap();

    let searcher = FulltextSearcher::open(dir.path()).unwrap();
    let results = searcher.search(Some("word"), &[], None, Some(5)).unwrap();
    assert!(results.len() <= 5);
}

#[test]
fn test_open_nonexistent_returns_error() {
    let result = FulltextSearcher::open(Path::new("/does/not/exist"));
    assert!(result.is_err());
}

#[test]
fn test_prio_field_preserved() {
    let dir = tempdir().unwrap();
    let mut maker = make_index(dir.path());
    maker
        .add_item("hm", "test", "", "Test", "/fs/test", 42, "test")
        .unwrap();
    maker.commit().unwrap();

    let searcher = FulltextSearcher::open(dir.path()).unwrap();
    let results = searcher.search(Some("test"), &[], None, None).unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0].prio, 42);
}

#[test]
fn test_all_query_when_no_text_or_filter() {
    let dir = tempdir().unwrap();
    let mut maker = make_index(dir.path());
    maker
        .add_item("hm", "alpha", "", "Alpha", "/fs/alpha", 0, "alpha")
        .unwrap();
    maker
        .add_item("hm", "beta", "", "Beta", "/fs/beta", 0, "beta")
        .unwrap();
    maker.commit().unwrap();

    let searcher = FulltextSearcher::open(dir.path()).unwrap();
    let results = searcher.search(None, &[], None, None).unwrap();
    assert_eq!(results.len(), 2, "AllQuery should return all documents");
}
