//! Application state machine.
//!
//! `App` owns all runtime state and handles keyboard / mouse events, routing
//! them to the appropriate subsystem (search, navigation, audio, etc.).

use std::collections::VecDeque;
use std::path::PathBuf;

use crate::audio::AudioPlayer;
use crate::config::{AutoPronLanguage, Config};
use crate::content::{ContentId, ContentPage, ContentType, SearchResultItem};
use crate::search::{FulltextResult, FulltextSearcher, IncrementalResult, IncrementalSearcher};

// --------------------------------------------------------------------------
// Application mode
// --------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppMode {
    /// Normal operation: search box + result list + content view.
    Normal,
    /// Keyboard focus is on the search text box.
    Searching,
    /// Keyboard focus is on the content pane.
    ContentFocused,
    /// Ctrl+F activated: text search within the content view.
    FindInPage,
    /// Advanced search overlay is shown.
    AdvancedSearch,
    /// Index is being built in the background.
    BuildingIndex,
    /// Quit requested.
    Quit,
}

// --------------------------------------------------------------------------
// Navigation history (browser-style back/forward)
// --------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub path: String,
    pub query: String,
}

pub struct NavHistory {
    pub entries: VecDeque<HistoryEntry>,
    pub cursor: usize,
    /// Maximum number of entries to keep.
    max: usize,
}

impl NavHistory {
    pub fn new() -> Self {
        NavHistory {
            entries: VecDeque::new(),
            cursor: 0,
            max: 50,
        }
    }

    /// Push a new entry, discarding any forward history.
    pub fn push(&mut self, path: &str, query: &str) {
        // Discard forward history
        while self.entries.len() > self.cursor + 1 {
            self.entries.pop_back();
        }
        // Deduplicate consecutive identical entries
        if let Some(last) = self.entries.back() {
            if last.path == path {
                return;
            }
        }
        if self.entries.len() >= self.max {
            self.entries.pop_front();
            if self.cursor > 0 {
                self.cursor -= 1;
            }
        }
        self.entries.push_back(HistoryEntry {
            path: path.to_owned(),
            query: query.to_owned(),
        });
        self.cursor = self.entries.len() - 1;
    }

    pub fn can_go_back(&self) -> bool {
        self.cursor > 0
    }
    pub fn can_go_forward(&self) -> bool {
        self.cursor + 1 < self.entries.len()
    }

    pub fn go_back(&mut self) -> Option<&HistoryEntry> {
        if self.can_go_back() {
            self.cursor -= 1;
            self.entries.get(self.cursor)
        } else {
            None
        }
    }

    pub fn go_forward(&mut self) -> Option<&HistoryEntry> {
        if self.can_go_forward() {
            self.cursor += 1;
            self.entries.get(self.cursor)
        } else {
            None
        }
    }

    /// Returns up to 20 back items (most recent first).
    pub fn back_list(&self) -> Vec<&str> {
        (0..self.cursor)
            .rev()
            .take(20)
            .filter_map(|i| self.entries.get(i))
            .map(|e| e.path.as_str())
            .collect()
    }

    /// Returns up to 20 forward items.
    pub fn forward_list(&self) -> Vec<&str> {
        (self.cursor + 1..self.entries.len())
            .take(20)
            .filter_map(|i| self.entries.get(i))
            .map(|e| e.path.as_str())
            .collect()
    }
}

// --------------------------------------------------------------------------
// Advanced search filter node (mirrors Python advtree.py)
// --------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct FilterNode {
    pub label: String,
    pub code: Option<String>,
    pub children: Vec<FilterNode>,
    pub checked: bool,
}

impl FilterNode {
    fn new(label: &str, code: Option<&str>) -> Self {
        FilterNode {
            label: label.to_owned(),
            code: code.map(str::to_owned),
            children: Vec::new(),
            checked: false,
        }
    }

    fn with_children(mut self, children: Vec<FilterNode>) -> Self {
        self.children = children;
        self
    }

    /// Collect all checked leaf codes into a Tantivy filter expression.
    pub fn collect_filter(&self, out: &mut Vec<String>) {
        if let Some(code) = &self.code {
            if self.checked {
                out.push(format!("asfilter:{code}"));
            }
        }
        for child in &self.children {
            child.collect_filter(out);
        }
    }
}

/// Build the advanced search filter tree (mirrors Python advtree._DATA).
pub fn build_filter_tree() -> Vec<FilterNode> {
    vec![
        FilterNode::new("Most frequent spoken words", None).with_children(vec![
            FilterNode::new("1000", Some("233")),
            FilterNode::new("2000", Some("234")),
            FilterNode::new("3000", Some("235")),
        ]),
        FilterNode::new("Most frequent written words", None).with_children(vec![
            FilterNode::new("1000", Some("236")),
            FilterNode::new("2000", Some("237")),
            FilterNode::new("3000", Some("238")),
        ]),
        FilterNode::new("Multimedia", None).with_children(vec![
            FilterNode::new("Picture", Some("332")),
            FilterNode::new("Sound effect", Some("333")),
        ]),
        FilterNode::new("Part of speech", None).with_children(vec![
            FilterNode::new("adjective", Some("334")),
            FilterNode::new("adverb", Some("335")),
            FilterNode::new("auxiliary verb", Some("336")),
            FilterNode::new("conjunction", Some("337")),
            FilterNode::new("determiner", Some("338")),
            FilterNode::new("interjection", Some("339")),
            FilterNode::new("modal verb", Some("340")),
            FilterNode::new("noun", Some("341")),
            FilterNode::new("number", Some("342")),
            FilterNode::new("phrasal verb", Some("343")),
            FilterNode::new("preposition", Some("346")),
            FilterNode::new("verb", Some("349")),
        ]),
        FilterNode::new("Register", None).with_children(vec![
            FilterNode::new("formal", Some("351")),
            FilterNode::new("informal", Some("353")),
            FilterNode::new("spoken", Some("360")),
            FilterNode::new("written", Some("364")),
            FilterNode::new("technical", Some("362")),
            FilterNode::new("old-fashioned", Some("359")),
        ]),
    ]
}

// --------------------------------------------------------------------------
// Anki export
// --------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AnkiExportInfo {
    pub header: String,
    pub meaning: String,
    pub audio_paths: Vec<String>,
}

// --------------------------------------------------------------------------
// Main application state
// --------------------------------------------------------------------------

pub struct App {
    // ── Mode ──────────────────────────────────────────────────────────────
    pub mode: AppMode,

    // ── Search ────────────────────────────────────────────────────────────
    /// Current text in the search box.
    pub search_text: String,
    /// Cursor position within the search text.
    pub search_cursor: usize,
    /// Results from incremental search.
    pub incr_results: Vec<SearchResultItem>,
    /// Results from full-text search.
    pub fts_results: Vec<SearchResultItem>,
    /// Merged and deduplicated result list.
    pub results: Vec<SearchResultItem>,
    /// Currently highlighted row in the result list.
    pub selected_row: Option<usize>,
    /// How far the result list is scrolled.
    pub result_scroll: usize,

    // ── Find in page ──────────────────────────────────────────────────────
    pub find_text: String,
    pub find_matches: Vec<usize>, // line indices of matches
    pub find_cursor: usize,

    // ── Content view ──────────────────────────────────────────────────────
    pub content_page: Option<ContentPage>,
    pub content_scroll: usize,
    /// Horizontal scroll offset in the content view (columns, used when wrap is off).
    pub content_scroll_x: u16,
    /// The currently displayed content path.
    pub current_path: Option<String>,
    /// Audio buttons in the current page: (block_idx, col_start, path, title).
    pub audio_buttons: Vec<(usize, u16, String, String)>,

    // ── Zoom ──────────────────────────────────────────────────────────────
    /// Zoom level: each integer step multiplies the font size by 1.05.
    pub zoom_power: i32,

    // ── Navigation history ────────────────────────────────────────────────
    pub history: NavHistory,

    // ── Advanced search ───────────────────────────────────────────────────
    pub adv_phrase: String,
    pub adv_filter_tree: Vec<FilterNode>,
    /// Focused node index in the advanced search tree.
    pub adv_tree_cursor: usize,

    // ── Clipboard monitor ─────────────────────────────────────────────────
    pub clipboard_text: String,

    // ── Status message ────────────────────────────────────────────────────
    pub status: String,
    pub is_searching: bool,

    // ── Spelling suggestions ──────────────────────────────────────────────
    pub spell_suggestions: Vec<String>,

    // ── Config ────────────────────────────────────────────────────────────
    pub config: Config,

    // ── Searchers (lazy-loaded) ───────────────────────────────────────────
    pub incr_searcher: Option<IncrementalSearcher>,
    pub fts_hp: Option<FulltextSearcher>,
    pub fts_de: Option<FulltextSearcher>,

    // ── Audio ─────────────────────────────────────────────────────────────
    pub audio_player: Option<AudioPlayer>,

    // ── Pending auto-pronunciation ────────────────────────────────────────
    pub auto_pron_pending: Option<String>,
}

impl App {
    pub fn new(config: Config) -> Self {
        App {
            mode: AppMode::Searching,
            search_text: config.last_query.clone(),
            search_cursor: config.last_query.len(),
            incr_results: Vec::new(),
            fts_results: Vec::new(),
            results: Vec::new(),
            selected_row: None,
            result_scroll: 0,
            find_text: String::new(),
            find_matches: Vec::new(),
            find_cursor: 0,
            content_page: None,
            content_scroll: 0,
            content_scroll_x: 0,
            current_path: None,
            audio_buttons: Vec::new(),
            zoom_power: 0,
            history: NavHistory::new(),
            adv_phrase: String::new(),
            adv_filter_tree: build_filter_tree(),
            adv_tree_cursor: 0,
            clipboard_text: String::new(),
            status: String::new(),
            is_searching: false,
            spell_suggestions: Vec::new(),
            config,
            incr_searcher: None,
            fts_hp: None,
            fts_de: None,
            audio_player: AudioPlayer::new().ok(),
            auto_pron_pending: None,
        }
    }

    // ── Search box manipulation ──────────────────────────────────────────

    /// Insert `ch` at the current cursor position.
    pub fn insert_char(&mut self, ch: char) {
        let mut s = std::mem::take(&mut self.search_text);
        let byte_pos = char_to_byte_pos(&s, self.search_cursor);
        s.insert(byte_pos, ch);
        self.search_cursor += 1;
        self.search_text = s;
    }

    /// Delete the character immediately before the cursor.
    pub fn backspace(&mut self) {
        if self.search_cursor == 0 {
            return;
        }
        let mut s = std::mem::take(&mut self.search_text);
        let byte_pos = char_to_byte_pos(&s, self.search_cursor);
        // Find the byte position of the previous char
        let prev = s[..byte_pos]
            .char_indices()
            .last()
            .map(|(i, _)| i)
            .unwrap_or(0);
        s.remove(prev);
        self.search_cursor -= 1;
        self.search_text = s;
    }

    /// Replace the entire search text (e.g. from clipboard paste).
    pub fn set_search_text(&mut self, text: &str) {
        self.search_text = text.to_owned();
        self.search_cursor = self.search_text.chars().count();
    }

    /// Move search cursor left.
    pub fn cursor_left(&mut self) {
        if self.search_cursor > 0 {
            self.search_cursor -= 1;
        }
    }

    /// Move search cursor right.
    pub fn cursor_right(&mut self) {
        let len = self.search_text.chars().count();
        if self.search_cursor < len {
            self.search_cursor += 1;
        }
    }

    pub fn cursor_home(&mut self) {
        self.search_cursor = 0;
    }
    pub fn cursor_end(&mut self) {
        self.search_cursor = self.search_text.chars().count();
    }

    // ── Result list navigation ───────────────────────────────────────────

    pub fn select_next(&mut self) {
        let max = self.results.len().saturating_sub(1);
        self.selected_row = Some(self.selected_row.map(|r| (r + 1).min(max)).unwrap_or(0));
    }

    pub fn select_prev(&mut self) {
        self.selected_row = Some(self.selected_row.map(|r| r.saturating_sub(1)).unwrap_or(0));
    }

    /// Select the first result whose plain field starts with `prefix`.
    pub fn select_by_prefix(&mut self, prefix: &str) {
        let norm = crate::search::normalize_index_key(prefix);
        for (i, r) in self.results.iter().enumerate() {
            if crate::search::normalize_index_key(&r.sortkey).starts_with(&norm) {
                self.selected_row = Some(i);
                return;
            }
        }
        // fall back to first entry
        if !self.results.is_empty() {
            self.selected_row = Some(0);
        }
    }

    // ── Merge incremental + FTS results ─────────────────────────────────

    pub fn rebuild_results(&mut self) {
        use std::collections::HashSet;
        let mut seen: HashSet<String> = self.incr_results.iter().map(|r| r.path.clone()).collect();
        self.results = self.incr_results.clone();
        for r in &self.fts_results {
            if seen.insert(r.path.clone()) {
                self.results.push(r.clone());
            }
        }
    }

    // ── Content navigation ───────────────────────────────────────────────

    pub fn navigate_to(&mut self, path: &str) {
        self.current_path = Some(path.to_owned());
        self.history.push(path, &self.search_text);
        self.content_scroll = 0;
        self.auto_pron_pending = ContentId::from_path(path)
            .filter(|c| c.content_type == ContentType::Entry)
            .map(|c| c.id.clone());
        // Extract audio buttons from the page (called after content_page is set)
        self.rebuild_audio_buttons();
    }

    /// Extract audio buttons from the current content page.
    pub fn rebuild_audio_buttons(&mut self) {
        self.audio_buttons.clear();
        if let Some(page) = &self.content_page {
            for (block_idx, block) in page.iter().enumerate() {
                let mut col = block.indent as usize * 2;
                for inline in &block.inlines {
                    match inline {
                        crate::content::Inline::AudioButton { path, title } => {
                            let emoji = match title.as_str() {
                                "British" => "🇬🇧",
                                "American" => "🇺🇸",
                                _ => "▶",
                            };
                            let btn_width = 1 + emoji.chars().count() + 1; // " emoji "
                            self.audio_buttons.push((
                                block_idx,
                                col as u16,
                                path.clone(),
                                title.clone(),
                            ));
                            col += btn_width;
                        }
                        crate::content::Inline::Prefix(p, _) => {
                            col += p.chars().count();
                        }
                        crate::content::Inline::Text(t, _) => {
                            col += t.chars().count();
                        }
                        crate::content::Inline::Headword(t) => {
                            col += t.chars().count();
                        }
                        crate::content::Inline::Link { text, .. } => {
                            col += text.chars().count();
                        }
                        crate::content::Inline::LineBreak => {
                            col = block.indent as usize * 2;
                        }
                        crate::content::Inline::Image { .. } => {}
                        crate::content::Inline::Badge { text } => {
                            col += text.chars().count() + 3; // " [" + text + "]"
                        }
                        crate::content::Inline::Signpost { text } => {
                            col += text.chars().count() + 6; // " ■ " + text + " ■ "
                        }
                    }
                }
            }
        }
    }

    /// Play the audio button nearest to (or at) the given block index.
    pub fn play_nearest_audio(&mut self, near_block: usize) -> bool {
        if self.audio_buttons.is_empty() {
            return false;
        }
        // Find the closest audio button
        let best = self
            .audio_buttons
            .iter()
            .min_by_key(|(idx, _, _, _)| (*idx as isize - near_block as isize).unsigned_abs())
            .cloned();
        if let Some((_, _, path, _)) = best {
            let parts: Vec<&str> = path.trim_start_matches('/').splitn(2, '/').collect();
            if parts.len() == 2 {
                let archive = parts[0].to_owned();
                let filename = format!("{}.mp3", parts[1]);
                self._trigger_audio_file(&archive, &filename);
                return true;
            }
        }
        false
    }

    /// Internal helper – stores the archive/filename request for the main loop to play.
    /// (audio playing needs data_dir, so we emit a pending request)
    pub fn request_audio_play(&mut self, archive: String, filename: String) {
        self.auto_pron_pending = Some(format!("{}/{}", archive, filename));
    }

    pub fn navigate_back(&mut self) {
        if let Some(entry) = self.history.go_back() {
            let path = entry.path.clone();
            self.current_path = Some(path);
            self.content_scroll = 0;
        }
    }

    pub fn navigate_forward(&mut self) {
        if let Some(entry) = self.history.go_forward() {
            let path = entry.path.clone();
            self.current_path = Some(path);
            self.content_scroll = 0;
        }
    }

    // ── Content scroll ───────────────────────────────────────────────────

    pub fn scroll_down(&mut self, lines: usize) {
        let max = self
            .content_page
            .as_ref()
            .map(|p| p.len().saturating_sub(1))
            .unwrap_or(0);
        self.content_scroll = (self.content_scroll + lines).min(max);
    }

    pub fn scroll_up(&mut self, lines: usize) {
        self.content_scroll = self.content_scroll.saturating_sub(lines);
    }

    pub fn scroll_to_top(&mut self) {
        self.content_scroll = 0;
    }
    pub fn scroll_to_bottom(&mut self) {
        self.content_scroll = self
            .content_page
            .as_ref()
            .map(|p| p.len().saturating_sub(1))
            .unwrap_or(0);
    }

    pub fn scroll_left(&mut self, cols: u16) {
        self.content_scroll_x = self.content_scroll_x.saturating_sub(cols);
    }

    pub fn scroll_right(&mut self, cols: u16) {
        self.content_scroll_x = self.content_scroll_x.saturating_add(cols);
    }

    pub fn toggle_wrap(&mut self) {
        self.config.content_wrap = !self.config.content_wrap;
        // When wrapping is re-enabled, reset horizontal scroll
        if self.config.content_wrap {
            self.content_scroll_x = 0;
        }
    }

    // ── Zoom ─────────────────────────────────────────────────────────────
    // (zoom_power is kept for config compatibility but not actively used)
    pub fn zoom_in(&mut self) {
        self.zoom_power = (self.zoom_power + 1).min(20);
    }
    pub fn zoom_out(&mut self) {
        self.zoom_power = (self.zoom_power - 1).max(-10);
    }
    pub fn zoom_reset(&mut self) {
        self.zoom_power = 0;
    }

    /// Returns the effective zoom factor (1.05^zoom_power).
    pub fn zoom_factor(&self) -> f32 {
        1.05f32.powi(self.zoom_power)
    }

    fn _trigger_audio_file(&self, _archive: &str, _filename: &str) {
        // placeholder – actual play is handled via play_audio_file in main.rs
    }

    // ── Find in page ────────────────────────────────────────────────────

    pub fn find_in_page(&mut self, query: &str) {
        self.find_text = query.to_owned();
        self.find_matches.clear();
        self.find_cursor = 0;
        if let Some(page) = &self.content_page {
            let q = query.to_lowercase();
            for (i, block) in page.iter().enumerate() {
                let text: String = block
                    .inlines
                    .iter()
                    .filter_map(|il| match il {
                        crate::content::Inline::Text(t, _) => Some(t.as_str()),
                        crate::content::Inline::Headword(t) => Some(t.as_str()),
                        crate::content::Inline::Prefix(p, _) => Some(p.as_str()),
                        crate::content::Inline::Badge { text } => Some(text.as_str()),
                        crate::content::Inline::Signpost { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("");
                if text.to_lowercase().contains(&q) {
                    self.find_matches.push(i);
                }
            }
        }
        if let Some(&line) = self.find_matches.first() {
            self.content_scroll = line;
        }
    }

    pub fn find_next(&mut self) {
        if self.find_matches.is_empty() {
            return;
        }
        self.find_cursor = (self.find_cursor + 1) % self.find_matches.len();
        self.content_scroll = self.find_matches[self.find_cursor];
    }

    pub fn find_prev(&mut self) {
        if self.find_matches.is_empty() {
            return;
        }
        if self.find_cursor == 0 {
            self.find_cursor = self.find_matches.len() - 1;
        } else {
            self.find_cursor -= 1;
        }
        self.content_scroll = self.find_matches[self.find_cursor];
    }

    // ── Audio ────────────────────────────────────────────────────────────

    pub fn play_audio(&self, data: Vec<u8>) {
        if let Some(player) = &self.audio_player {
            let _ = player.play(data);
        }
    }

    pub fn stop_audio(&self) {
        if let Some(player) = &self.audio_player {
            player.stop();
        }
    }

    // ── Anki export ──────────────────────────────────────────────────────

    pub fn anki_export_info(&self) -> Option<AnkiExportInfo> {
        let page = self.content_page.as_ref()?;
        // Collect all text as a simple tab-separated header+meaning
        let mut header = String::new();
        let mut meaning = String::new();
        let mut in_head = false;
        let mut audio_paths = Vec::new();

        for block in page.iter().take(3) {
            for inline in &block.inlines {
                match inline {
                    crate::content::Inline::Text(t, _)
                    | crate::content::Inline::Headword(t)
                    | crate::content::Inline::Prefix(t, _) => {
                        if header.is_empty() {
                            header.push_str(t);
                        } else {
                            meaning.push_str(t);
                        }
                    }
                    crate::content::Inline::AudioButton { path, .. } => {
                        audio_paths.push(path.clone());
                    }
                    _ => {}
                }
            }
        }
        Some(AnkiExportInfo {
            header,
            meaning,
            audio_paths,
        })
    }

    // ── Clipboard ────────────────────────────────────────────────────────

    pub fn handle_clipboard_change(&mut self, text: &str) {
        if !self.config.monitor_clipboard {
            return;
        }
        let trimmed = text.trim();
        if trimmed.is_empty() || trimmed == self.clipboard_text {
            return;
        }
        self.clipboard_text = trimmed.to_owned();
        // Auto-search if the text looks like a word
        if trimmed.split_whitespace().count() <= 5 {
            self.set_search_text(trimmed);
        }
    }

    // ── Auto-pronunciation ───────────────────────────────────────────────

    pub fn trigger_auto_pron(&mut self, path: String) {
        match &self.config.auto_pron {
            AutoPronLanguage::Off => {}
            _ => {
                self.auto_pron_pending = Some(path);
            }
        }
    }

    // ── Advanced search ──────────────────────────────────────────────────

    pub fn adv_make_filter_string(&self) -> String {
        let mut codes = Vec::new();
        for node in &self.adv_filter_tree {
            node.collect_filter(&mut codes);
        }
        if codes.is_empty() {
            String::new()
        } else {
            codes.join(" OR ")
        }
    }

    // ── Spell correction ─────────────────────────────────────────────────

    pub fn check_spell(&mut self) {
        if self.results.is_empty() && self.search_text.split_whitespace().count() == 1 {
            if let Some(fts) = &self.fts_hp {
                self.spell_suggestions = fts.correct(&self.search_text, 5);
            }
        } else {
            self.spell_suggestions.clear();
        }
    }
}

// --------------------------------------------------------------------------
// Helpers
// --------------------------------------------------------------------------

fn char_to_byte_pos(s: &str, char_pos: usize) -> usize {
    s.char_indices()
        .nth(char_pos)
        .map(|(b, _)| b)
        .unwrap_or(s.len())
}

// --------------------------------------------------------------------------
// Tests
// --------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_app() -> App {
        App::new(Config::default())
    }

    #[test]
    fn test_initial_mode() {
        let app = make_app();
        assert_eq!(app.mode, AppMode::Searching);
    }

    #[test]
    fn test_insert_and_backspace() {
        let mut app = make_app();
        app.set_search_text("");
        app.insert_char('h');
        app.insert_char('i');
        assert_eq!(app.search_text, "hi");
        app.backspace();
        assert_eq!(app.search_text, "h");
        app.backspace();
        assert_eq!(app.search_text, "");
        app.backspace(); // Should not panic on empty
        assert_eq!(app.search_text, "");
    }

    #[test]
    fn test_set_search_text() {
        let mut app = make_app();
        app.set_search_text("hello world");
        assert_eq!(app.search_text, "hello world");
        assert_eq!(app.search_cursor, 11);
    }

    #[test]
    fn test_cursor_movement() {
        let mut app = make_app();
        app.set_search_text("abc");
        app.cursor_home();
        assert_eq!(app.search_cursor, 0);
        app.cursor_right();
        assert_eq!(app.search_cursor, 1);
        app.cursor_end();
        assert_eq!(app.search_cursor, 3);
        app.cursor_left();
        assert_eq!(app.search_cursor, 2);
    }

    #[test]
    fn test_select_next_prev() {
        let mut app = make_app();
        app.results = vec![
            SearchResultItem {
                label: "a".into(),
                path: "/a".into(),
                sortkey: "a".into(),
                prio: 0,
                snippet: None,
            },
            SearchResultItem {
                label: "b".into(),
                path: "/b".into(),
                sortkey: "b".into(),
                prio: 0,
                snippet: None,
            },
            SearchResultItem {
                label: "c".into(),
                path: "/c".into(),
                sortkey: "c".into(),
                prio: 0,
                snippet: None,
            },
        ];
        app.select_next();
        assert_eq!(app.selected_row, Some(0));
        app.select_next();
        assert_eq!(app.selected_row, Some(1));
        app.select_prev();
        assert_eq!(app.selected_row, Some(0));
        app.select_prev(); // Clamp at 0
        assert_eq!(app.selected_row, Some(0));
    }

    #[test]
    fn test_nav_history() {
        let mut h = NavHistory::new();
        h.push("/fs/apple", "apple");
        h.push("/fs/banana", "banana");
        assert!(h.can_go_back());
        assert!(!h.can_go_forward());
        let entry = h.go_back().unwrap();
        assert_eq!(entry.path, "/fs/apple");
        assert!(h.can_go_forward());
        let entry = h.go_forward().unwrap();
        assert_eq!(entry.path, "/fs/banana");
    }

    #[test]
    fn test_nav_history_dedup() {
        let mut h = NavHistory::new();
        h.push("/fs/apple", "apple");
        h.push("/fs/apple", "apple"); // duplicate
        assert_eq!(h.entries.len(), 1);
    }

    #[test]
    fn test_nav_history_back_list() {
        let mut h = NavHistory::new();
        h.push("/fs/a", "a");
        h.push("/fs/b", "b");
        h.push("/fs/c", "c");
        let list = h.back_list();
        assert_eq!(list, vec!["/fs/b", "/fs/a"]);
    }

    #[test]
    fn test_zoom() {
        let mut app = make_app();
        assert_eq!(app.zoom_power, 0);
        app.zoom_in();
        assert_eq!(app.zoom_power, 1);
        app.zoom_out();
        app.zoom_out();
        assert_eq!(app.zoom_power, -1);
        app.zoom_reset();
        assert_eq!(app.zoom_power, 0);
        assert!((app.zoom_factor() - 1.0).abs() < 1e-4);
    }

    #[test]
    fn test_zoom_clamp() {
        let mut app = make_app();
        for _ in 0..30 {
            app.zoom_in();
        }
        assert_eq!(app.zoom_power, 20);
        for _ in 0..30 {
            app.zoom_out();
        }
        assert_eq!(app.zoom_power, -10);
    }

    #[test]
    fn test_find_in_page() {
        use crate::content::transform::{Block, Inline};
        use ratatui::style::Style;
        let mut app = make_app();
        app.content_page = Some(vec![
            Block {
                indent: 0,
                inlines: vec![Inline::Text("hello world".into(), Style::default())],
                ..Block::default()
            },
            Block {
                indent: 0,
                inlines: vec![Inline::Text("foo bar".into(), Style::default())],
                ..Block::default()
            },
            Block {
                indent: 0,
                inlines: vec![Inline::Text("hello again".into(), Style::default())],
                ..Block::default()
            },
        ]);
        app.find_in_page("hello");
        assert_eq!(app.find_matches, vec![0, 2]);
        app.find_next();
        assert_eq!(app.find_cursor, 1);
        app.find_prev();
        assert_eq!(app.find_cursor, 0);
    }

    #[test]
    fn test_rebuild_results_dedup() {
        let mut app = make_app();
        let item_a = SearchResultItem {
            label: "A".into(),
            path: "/a".into(),
            sortkey: "a".into(),
            prio: 0,
            snippet: None,
        };
        let item_b = SearchResultItem {
            label: "B".into(),
            path: "/b".into(),
            sortkey: "b".into(),
            prio: 0,
            snippet: None,
        };
        app.incr_results = vec![item_a.clone()];
        // item_a also appears in fts, item_b is new
        app.fts_results = vec![item_a.clone(), item_b.clone()];
        app.rebuild_results();
        assert_eq!(app.results.len(), 2, "duplicates should be removed");
    }

    #[test]
    fn test_adv_make_filter_string_empty() {
        let app = make_app();
        assert_eq!(app.adv_make_filter_string(), "");
    }

    #[test]
    fn test_adv_make_filter_string_checked() {
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

    #[test]
    fn test_clipboard_monitor_disabled() {
        let mut app = make_app();
        app.config.monitor_clipboard = false;
        app.handle_clipboard_change("new text");
        assert_eq!(app.search_text, app.config.last_query);
    }

    #[test]
    fn test_clipboard_monitor_enabled() {
        let mut app = make_app();
        app.config.monitor_clipboard = true;
        app.handle_clipboard_change("apple");
        assert_eq!(app.search_text, "apple");
    }
}
