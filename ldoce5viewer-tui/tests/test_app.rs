//! Integration tests for the App state machine.

use ldoce5viewer_tui::{
    app::{App, AppMode, NavHistory},
    config::{AutoPronLanguage, Config},
    content::{
        transform::{Block, Inline},
        types::SearchResultItem,
    },
};
use ratatui::style::Style;

// --------------------------------------------------------------------------
// Helper
// --------------------------------------------------------------------------

fn make_app() -> App {
    App::new(Config::default())
}

fn make_result(path: &str, sortkey: &str) -> SearchResultItem {
    SearchResultItem {
        label:   path.to_owned(),
        path:    path.to_owned(),
        sortkey: sortkey.to_owned(),
        prio:    0,
        snippet: None,
    }
}

// --------------------------------------------------------------------------
// Mode transitions
// --------------------------------------------------------------------------

#[test]
fn test_initial_mode_is_searching() {
    let app = make_app();
    assert_eq!(app.mode, AppMode::Searching);
}

// --------------------------------------------------------------------------
// Search text editing
// --------------------------------------------------------------------------

#[test]
fn test_insert_chars() {
    let mut app = make_app();
    app.set_search_text("");
    for ch in "hello".chars() { app.insert_char(ch); }
    assert_eq!(app.search_text, "hello");
    assert_eq!(app.search_cursor, 5);
}

#[test]
fn test_backspace_removes_last_char() {
    let mut app = make_app();
    app.set_search_text("hello");
    app.backspace();
    assert_eq!(app.search_text, "hell");
}

#[test]
fn test_backspace_on_empty_does_not_panic() {
    let mut app = make_app();
    app.set_search_text("");
    app.backspace(); // should not panic
    assert_eq!(app.search_text, "");
}

#[test]
fn test_cursor_home_end() {
    let mut app = make_app();
    app.set_search_text("abcde");
    app.cursor_home();
    assert_eq!(app.search_cursor, 0);
    app.cursor_end();
    assert_eq!(app.search_cursor, 5);
}

#[test]
fn test_cursor_left_right_clamp() {
    let mut app = make_app();
    app.set_search_text("ab");
    app.cursor_home();
    app.cursor_left(); // should stay at 0
    assert_eq!(app.search_cursor, 0);
    app.cursor_right();
    app.cursor_right();
    app.cursor_right(); // should clamp at len=2
    assert_eq!(app.search_cursor, 2);
}

// --------------------------------------------------------------------------
// Result list selection
// --------------------------------------------------------------------------

#[test]
fn test_select_next_from_empty() {
    let mut app = make_app();
    app.results = vec![make_result("/a", "a"), make_result("/b", "b")];
    app.select_next();
    assert_eq!(app.selected_row, Some(0));
}

#[test]
fn test_select_next_increments() {
    let mut app = make_app();
    app.results = vec![make_result("/a", "a"), make_result("/b", "b"), make_result("/c", "c")];
    app.select_next();
    app.select_next();
    assert_eq!(app.selected_row, Some(1));
}

#[test]
fn test_select_prev_clamps_at_zero() {
    let mut app = make_app();
    app.results = vec![make_result("/a", "a"), make_result("/b", "b")];
    app.select_next(); // selected=0
    app.select_prev(); // should stay at 0
    assert_eq!(app.selected_row, Some(0));
}

#[test]
fn test_select_next_clamps_at_last() {
    let mut app = make_app();
    app.results = vec![make_result("/a", "a"), make_result("/b", "b")];
    app.select_next(); // 0
    app.select_next(); // 1
    app.select_next(); // still 1 (clamped)
    assert_eq!(app.selected_row, Some(1));
}

// --------------------------------------------------------------------------
// Zoom
// --------------------------------------------------------------------------

#[test]
fn test_zoom_default_factor_is_one() {
    let app = make_app();
    assert!((app.zoom_factor() - 1.0).abs() < 1e-4);
}

#[test]
fn test_zoom_in_out_reset() {
    let mut app = make_app();
    app.zoom_in();
    app.zoom_in();
    assert_eq!(app.zoom_power, 2);
    app.zoom_reset();
    assert_eq!(app.zoom_power, 0);
}

#[test]
fn test_zoom_clamped() {
    let mut app = make_app();
    for _ in 0..50 { app.zoom_in(); }
    assert_eq!(app.zoom_power, 20);
    for _ in 0..50 { app.zoom_out(); }
    assert_eq!(app.zoom_power, -10);
}

// --------------------------------------------------------------------------
// Navigation history
// --------------------------------------------------------------------------

#[test]
fn test_history_back_forward() {
    let mut h = NavHistory::new();
    h.push("/a", "a");
    h.push("/b", "b");
    h.push("/c", "c");
    assert!(h.can_go_back());
    assert!(!h.can_go_forward());
    let back = h.go_back().unwrap();
    assert_eq!(back.path, "/b");
    assert!(h.can_go_forward());
    let fwd = h.go_forward().unwrap();
    assert_eq!(fwd.path, "/c");
}

#[test]
fn test_history_push_clears_forward() {
    let mut h = NavHistory::new();
    h.push("/a", "a");
    h.push("/b", "b");
    h.go_back(); // now at /a
    h.push("/c", "c"); // discard /b
    assert!(!h.can_go_forward(), "forward history should be cleared after new push");
}

#[test]
fn test_history_dedup() {
    let mut h = NavHistory::new();
    h.push("/a", "a");
    h.push("/a", "a"); // duplicate
    assert_eq!(h.entries.len(), 1);
}

// --------------------------------------------------------------------------
// Rebuild results (deduplication)
// --------------------------------------------------------------------------

#[test]
fn test_rebuild_results_dedup() {
    let mut app = make_app();
    let r = make_result("/a", "a");
    app.incr_results = vec![r.clone()];
    app.fts_results  = vec![r.clone(), make_result("/b", "b")];
    app.rebuild_results();
    assert_eq!(app.results.len(), 2);
}

// --------------------------------------------------------------------------
// Find in page
// --------------------------------------------------------------------------

#[test]
fn test_find_in_page_matches() {
    let mut app = make_app();
    app.content_page = Some(vec![
        Block { indent: 0, inlines: vec![Inline::Text("hello world".into(), Style::default())] },
        Block { indent: 0, inlines: vec![Inline::Text("foo bar".into(), Style::default())] },
        Block { indent: 0, inlines: vec![Inline::Text("hello again".into(), Style::default())] },
    ]);
    app.find_in_page("hello");
    assert_eq!(app.find_matches, vec![0, 2], "two blocks contain 'hello'");
}

#[test]
fn test_find_next_wraps() {
    let mut app = make_app();
    app.content_page = Some(vec![
        Block { indent: 0, inlines: vec![Inline::Text("a".into(), Style::default())] },
        Block { indent: 0, inlines: vec![Inline::Text("a".into(), Style::default())] },
    ]);
    app.find_in_page("a");
    assert_eq!(app.find_matches.len(), 2);
    app.find_next(); // cursor = 1
    app.find_next(); // cursor = 0 (wraps)
    assert_eq!(app.find_cursor, 0);
}

// --------------------------------------------------------------------------
// Clipboard monitoring
// --------------------------------------------------------------------------

#[test]
fn test_clipboard_monitor_off() {
    let mut app = make_app();
    app.config.monitor_clipboard = false;
    app.set_search_text("original");
    app.handle_clipboard_change("new text");
    assert_eq!(app.search_text, "original");
}

#[test]
fn test_clipboard_monitor_on() {
    let mut app = make_app();
    app.config.monitor_clipboard = true;
    app.handle_clipboard_change("shortword");
    assert_eq!(app.search_text, "shortword");
}

// --------------------------------------------------------------------------
// Advanced search filter
// --------------------------------------------------------------------------

#[test]
fn test_adv_filter_empty_when_nothing_checked() {
    let app = make_app();
    assert_eq!(app.adv_make_filter_string(), "");
}

#[test]
fn test_adv_filter_includes_checked_codes() {
    let mut app = make_app();
    // Check the first leaf in the first group (code "233")
    if let Some(group) = app.adv_filter_tree.first_mut() {
        if let Some(child) = group.children.first_mut() {
            child.checked = true;
        }
    }
    let filter = app.adv_make_filter_string();
    assert!(filter.contains("asfilter:233"), "filter: {filter}");
}

// --------------------------------------------------------------------------
// Anki export
// --------------------------------------------------------------------------

#[test]
fn test_anki_export_info_none_when_no_content() {
    let app = make_app();
    assert!(app.anki_export_info().is_none());
}

#[test]
fn test_anki_export_info_returns_something_when_content_present() {
    let mut app = make_app();
    app.content_page = Some(vec![
        Block { indent: 0, inlines: vec![Inline::Text("able".into(), Style::default())] },
    ]);
    assert!(app.anki_export_info().is_some());
}
