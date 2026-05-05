//! Main entry point for `ldoce5viewer-tui`.
//!
//! Sets up the terminal, runs the event loop, and tears down the terminal on
//! exit.

mod app;
mod audio;
mod config;
mod content;
mod data;
mod images;
mod search;
mod ui;

use std::io;
use std::time::Duration;

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
        MouseButton, MouseEvent, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use app::{App, AppMode};
use config::{load_config, save_config, AutoPronLanguage};
use content::{ContentId, ContentType};
use search::{FulltextSearcher, IncrementalSearcher};

// --------------------------------------------------------------------------
// Terminal setup / teardown
// --------------------------------------------------------------------------

fn setup_terminal() -> io::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

fn teardown_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) {
    let _ = disable_raw_mode();
    let _ = execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    );
    let _ = terminal.show_cursor();
}

// --------------------------------------------------------------------------
// Entry point
// --------------------------------------------------------------------------

fn main() -> anyhow::Result<()> {
    let cfg = load_config();
    let mut app = App::new(cfg);

    // Attempt to open the search indices
    let incr_path = config::incremental_index_path();
    if incr_path.exists() {
        match IncrementalSearcher::open(&incr_path) {
            Ok(s) => {
                app.incr_searcher = Some(s);
            }
            Err(e) => {
                app.status = format!("Incremental index error: {e}");
            }
        }
    }

    let hp_dir = config::fulltext_hwdphr_dir();
    if hp_dir.exists() {
        match FulltextSearcher::open(&hp_dir) {
            Ok(s) => {
                app.fts_hp = Some(s);
            }
            Err(e) => {
                app.status = format!("Fulltext HP index error: {e}");
            }
        }
    }

    let de_dir = config::fulltext_defexa_dir();
    if de_dir.exists() {
        match FulltextSearcher::open(&de_dir) {
            Ok(s) => {
                app.fts_de = Some(s);
            }
            Err(e) => {
                app.status = format!("Fulltext DE index error: {e}");
            }
        }
    }

    // Determine LDOCE5 data directory in this priority order:
    // 1) LDOCE5_DATA_DIR environment variable (if set and valid)
    // 2) value stored in `config.json` (already loaded into `app.config`)
    // 3) auto-discover common installation locations for the current OS
    let mut data_dir_found = false;

    if let Ok(dir) = std::env::var("LDOCE5_DATA_DIR") {
        let pb = std::path::PathBuf::from(dir);
        if data::is_ldoce5_dir(&pb) {
            app.config.data_dir = Some(pb);
            data_dir_found = true;
        } else {
            app.status = format!(
                "LDOCE5_DATA_DIR is set but \"{}\" doesn't look like a valid LDOCE5 data directory",
                pb.display()
            );
        }
    }

    if app.config.data_dir.is_none() {
        if let Some(pb) = data::discover_ldoce5_dir() {
            app.config.data_dir = Some(pb);
            data_dir_found = true;
        }
    }

    if !data_dir_found && app.config.data_dir.is_none() {
        app.status =
            "No LDOCE5 data directory configured. Set LDOCE5_DATA_DIR env or edit config.json."
                .to_owned();
    }

    let mut terminal = setup_terminal()?;

    let result = run_loop(&mut terminal, &mut app);

    teardown_terminal(&mut terminal);

    // Save config on exit
    let _ = save_config(&app.config);

    result
}

// --------------------------------------------------------------------------
// Main event loop
// --------------------------------------------------------------------------

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> anyhow::Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if app.mode == AppMode::Quit {
            break;
        }

        if event::poll(Duration::from_millis(50))? {
            let term_size = {
                let s = terminal.size().unwrap_or_default();
                ratatui::layout::Rect::new(0, 0, s.width, s.height)
            };
            match event::read()? {
                Event::Key(key) => handle_key(app, key),
                Event::Resize(_w, _h) => { /* terminal will redraw next iteration */ }
                Event::Mouse(m) => handle_mouse(app, m, term_size),
                _ => {}
            }
        }
    }
    Ok(())
}

// --------------------------------------------------------------------------
// Key handling
// --------------------------------------------------------------------------

fn handle_key(app: &mut App, key: KeyEvent) {
    match app.mode {
        AppMode::Searching => handle_key_searching(app, key),
        AppMode::Normal => handle_key_normal(app, key),
        AppMode::ContentFocused => handle_key_content(app, key),
        AppMode::FindInPage => handle_key_find(app, key),
        AppMode::AdvancedSearch => handle_key_advsearch(app, key),
        AppMode::BuildingIndex => handle_key_building(app, key),
        AppMode::Quit => {}
    }
}

// ── Searching mode ──────────────────────────────────────────────────────────

fn handle_key_searching(app: &mut App, key: KeyEvent) {
    match (key.modifiers, key.code) {
        (KeyModifiers::NONE, KeyCode::Esc) => {
            app.mode = AppMode::Normal;
        }
        (KeyModifiers::NONE, KeyCode::Tab) => {
            // Tab: Search → Normal (result list)
            app.mode = AppMode::Normal;
        }
        (KeyModifiers::NONE, KeyCode::Enter) => {
            app.mode = AppMode::Normal;
            run_incremental_search(app);
            auto_preview(app);
        }
        (KeyModifiers::NONE, KeyCode::Backspace) => {
            app.backspace();
            run_incremental_search(app);
            auto_preview(app);
        }
        (KeyModifiers::NONE, KeyCode::Left) => app.cursor_left(),
        (KeyModifiers::NONE, KeyCode::Right) => app.cursor_right(),
        (KeyModifiers::NONE, KeyCode::Home) => app.cursor_home(),
        (KeyModifiers::NONE, KeyCode::End) => app.cursor_end(),
        (KeyModifiers::NONE, KeyCode::Down) => {
            app.mode = AppMode::Normal;
            app.select_next();
            auto_preview(app);
        }
        (KeyModifiers::CONTROL, KeyCode::Char('c'))
        | (KeyModifiers::CONTROL, KeyCode::Char('q')) => {
            app.mode = AppMode::Quit;
        }
        (KeyModifiers::CONTROL, KeyCode::Char('a')) => {
            app.mode = AppMode::AdvancedSearch;
        }
        (_, KeyCode::Char(c)) => {
            app.insert_char(c);
            run_incremental_search(app);
            auto_preview(app);
        }
        _ => {}
    }
}

// ── Normal mode ──────────────────────────────────────────────────────────────

fn handle_key_normal(app: &mut App, key: KeyEvent) {
    match (key.modifiers, key.code) {
        // Quit
        (KeyModifiers::NONE, KeyCode::Char('q')) | (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
            app.mode = AppMode::Quit;
        }
        // Tab: Normal (result list) → ContentFocused
        (KeyModifiers::NONE, KeyCode::Tab) => {
            app.mode = AppMode::ContentFocused;
        }
        // Focus search
        (KeyModifiers::NONE, KeyCode::Char('/')) | (KeyModifiers::NONE, KeyCode::Char('i')) => {
            app.mode = AppMode::Searching;
        }
        // Vim-style result navigation with auto-preview
        (KeyModifiers::NONE, KeyCode::Char('j'))
        | (KeyModifiers::NONE, KeyCode::Down)
        | (KeyModifiers::CONTROL, KeyCode::Char('j'))
        | (KeyModifiers::CONTROL, KeyCode::Down) => {
            app.select_next();
            auto_preview(app);
        }
        (KeyModifiers::NONE, KeyCode::Char('k'))
        | (KeyModifiers::NONE, KeyCode::Up)
        | (KeyModifiers::CONTROL, KeyCode::Char('k'))
        | (KeyModifiers::CONTROL, KeyCode::Up) => {
            app.select_prev();
            auto_preview(app);
        }
        // Load entry explicitly
        (KeyModifiers::NONE, KeyCode::Enter) | (KeyModifiers::NONE, KeyCode::Char('l')) => {
            if let Some(idx) = app.selected_row {
                if let Some(item) = app.results.get(idx).cloned() {
                    load_entry(app, &item.path);
                    app.mode = AppMode::ContentFocused;
                }
            }
        }
        // Navigation – Ctrl+O (back), Ctrl+I (forward)
        (KeyModifiers::CONTROL, KeyCode::Char('o')) | (KeyModifiers::ALT, KeyCode::Left) => {
            app.navigate_back();
            if let Some(path) = app.current_path.clone() {
                load_entry(app, &path);
            }
        }
        (KeyModifiers::CONTROL, KeyCode::Char('i')) | (KeyModifiers::ALT, KeyCode::Right) => {
            app.navigate_forward();
            if let Some(path) = app.current_path.clone() {
                load_entry(app, &path);
            }
        }
        // Content scroll (when result pane focused but content already loaded)
        (KeyModifiers::NONE, KeyCode::PageDown) => app.scroll_down(20),
        (KeyModifiers::NONE, KeyCode::PageUp) => app.scroll_up(20),
        (KeyModifiers::NONE, KeyCode::Char(' ')) => app.scroll_down(10),
        (KeyModifiers::SHIFT, KeyCode::Char(' ')) => app.scroll_up(10),
        (KeyModifiers::NONE, KeyCode::Home) => app.scroll_to_top(),
        (KeyModifiers::NONE, KeyCode::End) => app.scroll_to_bottom(),

        // Find in page
        (KeyModifiers::CONTROL, KeyCode::Char('p')) => {
            app.mode = AppMode::FindInPage;
        }

        // Advanced search
        (KeyModifiers::CONTROL, KeyCode::Char('g')) => {
            if let Some(path) = app.current_path.clone() {
                if let Some(cid) = content::ContentId::from_path(&path) {
                    let filename = format!("{}.mp3", cid.id.replace('.', "_"));
                    play_audio_file(app, "gb_hwd_pron", &filename);
                }
            }
        }
        // Audio: US – play American pronunciation for current entry
        (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
            if let Some(path) = app.current_path.clone() {
                if let Some(cid) = content::ContentId::from_path(&path) {
                    let filename = format!("{}.mp3", cid.id.replace('.', "_"));
                    play_audio_file(app, "us_hwd_pron", &filename);
                }
            }
        }

        // Advanced search
        (KeyModifiers::CONTROL, KeyCode::Char('a')) => {
            app.mode = AppMode::AdvancedSearch;
        }

        _ => {}
    }
}

// ── Content-focused mode ─────────────────────────────────────────────────────

fn handle_key_content(app: &mut App, key: KeyEvent) {
    match (key.modifiers, key.code) {
        // Quit
        (KeyModifiers::NONE, KeyCode::Char('q')) | (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
            app.mode = AppMode::Quit;
        }
        // Esc / Tab: ContentFocused → Normal
        (KeyModifiers::NONE, KeyCode::Esc) => {
            app.mode = AppMode::Normal;
        }
        // Tab: ContentFocused → Searching
        (KeyModifiers::NONE, KeyCode::Tab) => {
            app.mode = AppMode::Searching;
        }
        // Focus search
        (KeyModifiers::NONE, KeyCode::Char('i')) => {
            app.mode = AppMode::Searching;
        }
        // Find in page
        (KeyModifiers::NONE, KeyCode::Char('/')) => {
            app.mode = AppMode::FindInPage;
        }
        // Scroll down
        (KeyModifiers::NONE, KeyCode::Char('j')) | (KeyModifiers::NONE, KeyCode::Down) => {
            app.scroll_down(1)
        }
        // Scroll up
        (KeyModifiers::NONE, KeyCode::Char('k')) | (KeyModifiers::NONE, KeyCode::Up) => {
            app.scroll_up(1)
        }
        // Horizontal scroll (only meaningful when wrap is off)
        (KeyModifiers::NONE, KeyCode::Char('h')) | (KeyModifiers::NONE, KeyCode::Left) => {
            app.scroll_left(4)
        }
        (KeyModifiers::NONE, KeyCode::Char('l')) | (KeyModifiers::NONE, KeyCode::Right) => {
            app.scroll_right(4)
        }
        // go to top
        (KeyModifiers::NONE, KeyCode::Char('g')) | (KeyModifiers::NONE, KeyCode::Home) => {
            app.scroll_to_top()
        }
        // go to bottom
        (KeyModifiers::NONE, KeyCode::Char('G')) | (KeyModifiers::NONE, KeyCode::End) => {
            app.scroll_to_bottom()
        }
        // Half page down
        (KeyModifiers::CONTROL, KeyCode::Char('d')) | (KeyModifiers::NONE, KeyCode::PageDown) => {
            app.scroll_down(15)
        }
        // Half page up
        (KeyModifiers::CONTROL, KeyCode::Char('u')) | (KeyModifiers::NONE, KeyCode::PageUp) => {
            app.scroll_up(15)
        }
        // Space scroll down
        (KeyModifiers::NONE, KeyCode::Char(' ')) => app.scroll_down(10),
        (KeyModifiers::SHIFT, KeyCode::Char(' ')) => app.scroll_up(10),
        // Toggle line wrapping
        (KeyModifiers::NONE, KeyCode::Char('w')) => app.toggle_wrap(),
        // Enter: play nearest audio button
        (KeyModifiers::NONE, KeyCode::Enter) => {
            let near = app.content_scroll;
            let path_parts = app
                .audio_buttons
                .iter()
                .min_by_key(|(idx, _, _, _)| (*idx as isize - near as isize).unsigned_abs())
                .and_then(|(_, _, path, _)| {
                    let trimmed = path.trim_start_matches('/');
                    let mut parts = trimmed.splitn(2, '/');
                    let archive = parts.next()?.to_owned();
                    let name = parts.next()?.to_owned();
                    Some((archive, name))
                });
            if let Some((archive, name)) = path_parts {
                let filename = name;
                play_audio_file(app, &archive, &filename);
            }
        }
        // Navigation – Ctrl+O (back), Ctrl+I (forward)
        (KeyModifiers::CONTROL, KeyCode::Char('o')) | (KeyModifiers::ALT, KeyCode::Left) => {
            app.navigate_back();
            if let Some(path) = app.current_path.clone() {
                load_entry(app, &path);
            }
        }
        (KeyModifiers::CONTROL, KeyCode::Char('i')) | (KeyModifiers::ALT, KeyCode::Right) => {
            app.navigate_forward();
            if let Some(path) = app.current_path.clone() {
                load_entry(app, &path);
            }
        }
        _ => {}
    }
}

// ── Find-in-page mode ────────────────────────────────────────────────────────

fn handle_key_find(app: &mut App, key: KeyEvent) {
    match (key.modifiers, key.code) {
        (KeyModifiers::NONE, KeyCode::Esc) => {
            app.find_text.clear();
            app.find_matches.clear();
            app.mode = AppMode::ContentFocused;
        }
        (KeyModifiers::NONE, KeyCode::Enter) | (KeyModifiers::NONE, KeyCode::Down) => {
            app.find_next();
        }
        (KeyModifiers::NONE, KeyCode::Up) => {
            app.find_prev();
        }
        (KeyModifiers::NONE, KeyCode::Backspace) => {
            app.find_text.pop();
            let q = app.find_text.clone();
            app.find_in_page(&q);
        }
        (_, KeyCode::Char(c)) => {
            app.find_text.push(c);
            let q = app.find_text.clone();
            app.find_in_page(&q);
        }
        _ => {}
    }
}

// ── Advanced search mode ─────────────────────────────────────────────────────

fn handle_key_advsearch(app: &mut App, key: KeyEvent) {
    match (key.modifiers, key.code) {
        (KeyModifiers::NONE, KeyCode::Esc) => {
            app.mode = AppMode::Normal;
        }
        (KeyModifiers::NONE, KeyCode::Enter) => {
            let filter = app.adv_make_filter_string();
            let phrase = app.adv_phrase.clone();
            run_fulltext_search(app, &phrase, &filter);
            app.mode = AppMode::Normal;
        }
        (KeyModifiers::NONE, KeyCode::Backspace) => {
            app.adv_phrase.pop();
        }
        (_, KeyCode::Char(c)) => {
            app.adv_phrase.push(c);
        }
        _ => {}
    }
}

// ── Building index mode ───────────────────────────────────────────────────────

fn handle_key_building(app: &mut App, key: KeyEvent) {
    match (key.modifiers, key.code) {
        (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
            app.mode = AppMode::Quit;
        }
        _ => {}
    }
}

// --------------------------------------------------------------------------
// Search helpers
// --------------------------------------------------------------------------

fn run_incremental_search(app: &mut App) {
    let query = app.search_text.clone();
    app.incr_results.clear();
    app.fts_results.clear();

    if let Some(searcher) = &app.incr_searcher {
        let results = searcher.search(&query, 200);
        app.incr_results = results
            .into_iter()
            .map(|r| content::SearchResultItem {
                label: r.label,
                path: r.path,
                sortkey: r.plain,
                prio: r.prio,
                snippet: None,
            })
            .collect();
    }

    // Sort so that exact matches come first, then by sortkey + prio
    let norm_query = search::normalize_index_key(&query);
    app.incr_results.sort_by(|a, b| {
        let a_exact = a.sortkey == norm_query;
        let b_exact = b.sortkey == norm_query;
        b_exact
            .cmp(&a_exact)
            .then_with(|| a.sortkey.cmp(&b.sortkey))
            .then_with(|| a.prio.cmp(&b.prio))
    });

    app.rebuild_results();
    app.check_spell();

    // Auto-select best match
    if !app.results.is_empty() {
        app.select_by_prefix(&query);
    }
}

fn run_fulltext_search(app: &mut App, phrase: &str, filter: &str) {
    app.fts_results.clear();
    let filter_opt = if filter.is_empty() {
        None
    } else {
        Some(filter)
    };

    if let Some(searcher) = &app.fts_hp {
        if let Ok(results) = searcher.search(
            Some(phrase),
            &["hm", "hp", "pl", "p"],
            filter_opt,
            Some(500),
        ) {
            for r in results {
                app.fts_results.push(content::SearchResultItem {
                    label: r.label,
                    path: r.path,
                    sortkey: r.sortkey,
                    prio: r.prio as u8,
                    snippet: r.highlight,
                });
            }
        }
    }

    app.rebuild_results();
}

// --------------------------------------------------------------------------
// Entry loading
// --------------------------------------------------------------------------

fn load_entry(app: &mut App, path: &str) {
    use data::{list_files, ArchiveReader};

    let data_dir = match &app.config.data_dir {
        Some(d) => d.clone(),
        None => {
            app.status = "No LDOCE5 data directory configured.".to_owned();
            return;
        }
    };

    let cid = match ContentId::from_path(path) {
        Some(c) => c,
        None => {
            app.status = format!("Unknown path: {path}");
            return;
        }
    };

    // Determine the archive name from the content type
    let archive = match cid.content_type {
        ContentType::Entry => "fs",
        ContentType::Thesaurus => "thesaurus",
        ContentType::Collocations => "collocations",
        ContentType::WordSets => "word_sets",
        ContentType::Phrases => "phrases",
        ContentType::Examples => "examples",
        ContentType::WordFamilies => "word_families",
        ContentType::Etymologies => "etymologies",
        ContentType::Activator => "activator",
        ContentType::Picture => "picture",
        _ => {
            app.status = format!(
                "Content type {:?} not directly loadable via path",
                cid.content_type
            );
            return;
        }
    };

    // Use the filemap (CDB) to look up the archive location
    let filemap_path = config::filemap_path();
    if !filemap_path.exists() {
        app.status = "Index not built yet. Run the index builder first.".to_owned();
        return;
    }

    let reader = match data::CDBReader::open(&filemap_path) {
        Ok(r) => r,
        Err(e) => {
            app.status = format!("Filemap error: {e}");
            return;
        }
    };

    // The filemap keys are the first 10 bytes of MD5(archive + ":" + name)
    // Compute the same lookup key here (mirrors Python FilemapReader.lookup)
    let md = md5::compute(format!("{}:{}", archive, cid.id));
    let key = &md[0..10];
    let val = match reader.get(key) {
        Some(v) => v,
        None => {
            app.status = format!("Entry not found: {}", cid.id);
            return;
        }
    };

    // The CDB stores the binary location tuple (cmp_offset, cmp_size, orig_offset, orig_size)
    // encoded as either <IIII> (16 bytes) or <IHHH> (10 bytes). Decode it here and read
    // the corresponding file block directly from the archive.
    let (cmpo, cmps, orgo, orgs) = match val.len() {
        16 => {
            let a: [u8; 4] = val[0..4].try_into().unwrap();
            let b: [u8; 4] = val[4..8].try_into().unwrap();
            let c: [u8; 4] = val[8..12].try_into().unwrap();
            let d: [u8; 4] = val[12..16].try_into().unwrap();
            (
                u32::from_le_bytes(a) as u64,
                u32::from_le_bytes(b) as u64,
                u32::from_le_bytes(c) as u64,
                u32::from_le_bytes(d) as u64,
            )
        }
        10 => {
            let a: [u8; 4] = val[0..4].try_into().unwrap();
            let b: [u8; 2] = val[4..6].try_into().unwrap();
            let c: [u8; 2] = val[6..8].try_into().unwrap();
            let d: [u8; 2] = val[8..10].try_into().unwrap();
            (
                u32::from_le_bytes(a) as u64,
                u16::from_le_bytes(b) as u64,
                u16::from_le_bytes(c) as u64,
                u16::from_le_bytes(d) as u64,
            )
        }
        _ => {
            app.status = "Malformed filemap entry.".to_owned();
            return;
        }
    };

    let mut arch_reader = match ArchiveReader::new(&data_dir, archive) {
        Ok(r) => r,
        Err(e) => {
            app.status = format!("Archive reader error: {e}");
            return;
        }
    };

    let xml_bytes = match arch_reader.read((cmpo, cmps, orgo, orgs)) {
        Ok(b) => b,
        Err(e) => {
            app.status = format!("Read error: {e}");
            return;
        }
    };

    // Transform
    let page = content::transform(cid.content_type, &xml_bytes);
    app.content_page = Some(page);
    app.rebuild_audio_buttons();
    app.navigate_to(path);
    app.status = String::new();

    // Trigger auto-pronunciation
    let auto_pron = app.config.auto_pron.clone();
    if auto_pron != AutoPronLanguage::Off {
        let pron_archive = match auto_pron {
            AutoPronLanguage::GB => "gb_hwd_pron",
            AutoPronLanguage::US => "us_hwd_pron",
            AutoPronLanguage::Off => unreachable!(),
        };
        let pron_key = format!("{}/{}.mp3", pron_archive, sanitize_for_pron(&cid.id));
        app.auto_pron_pending = Some(pron_key);
    }
}

fn sanitize_for_pron(id: &str) -> String {
    // id is like "3.4.6.2", convert to something like a filename
    id.replace('.', "_")
}

/// Auto-preview the currently selected result in the content view.
fn auto_preview(app: &mut App) {
    if let Some(idx) = app.selected_row {
        if let Some(item) = app.results.get(idx).cloned() {
            load_entry(app, &item.path);
        }
    }
}

// --------------------------------------------------------------------------
// Mouse handling
// --------------------------------------------------------------------------

fn handle_mouse(app: &mut App, m: MouseEvent, term_size: ratatui::layout::Rect) {
    // Recompute layout for hit-testing
    let layout = ui::layout::compute_layout(term_size, app);

    let col = m.column;
    let row = m.row;

    match m.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            if rect_contains(layout.search, col, row) {
                app.mode = AppMode::Searching;
            } else if rect_contains(layout.results, col, row) {
                // Click on result list → select and load that row
                app.mode = AppMode::Normal;
                // -1 for the top border of the block
                let inner_row = row.saturating_sub(layout.results.y + 1) as usize;
                if inner_row < app.results.len() {
                    app.selected_row = Some(inner_row);
                    auto_preview(app);
                }
            } else if rect_contains(layout.content, col, row) {
                // Click on content → try to play the audio button closest to the clicked position
                app.mode = AppMode::ContentFocused;
                let inner_row = row.saturating_sub(layout.content.y + 1) as usize;
                // Account for content border (1 col) when computing column offset
                let inner_col = col.saturating_sub(layout.content.x + 1);
                let target_line = app.content_scroll + inner_row;
                // Collect audio buttons on or near the target line
                let path_parts = {
                    let buttons = &app.audio_buttons;
                    // First try: button on exact target_line whose column range contains the click
                    let exact = buttons.iter().find(|(idx, col_start, _, _)| {
                        *idx == target_line && inner_col >= *col_start
                    });
                    // Second try: nearest button by block index, then column
                    let nearest = buttons
                        .iter()
                        .filter(|(idx, _, _, _)| {
                            (*idx as isize - target_line as isize).unsigned_abs() <= 2
                        })
                        .min_by_key(|(idx, col_start, _, _)| {
                            let row_dist = (*idx as isize - target_line as isize).unsigned_abs();
                            let col_dist =
                                (*col_start as isize - inner_col as isize).unsigned_abs();
                            row_dist * 1000 + col_dist
                        });
                    exact.or(nearest).and_then(|(_, _, path, _)| {
                        let trimmed = path.trim_start_matches('/');
                        let mut parts = trimmed.splitn(2, '/');
                        let archive = parts.next()?.to_owned();
                        let name = parts.next()?.to_owned();
                        Some((archive, name))
                    })
                };
                if let Some((archive, name)) = path_parts {
                    play_audio_file(app, &archive, &name);
                }
            }
        }
        MouseEventKind::ScrollDown => {
            if rect_contains(layout.results, col, row) {
                app.select_next();
                auto_preview(app);
            } else if rect_contains(layout.content, col, row) {
                app.scroll_down(3);
            }
        }
        MouseEventKind::ScrollUp => {
            if rect_contains(layout.results, col, row) {
                app.select_prev();
                auto_preview(app);
            } else if rect_contains(layout.content, col, row) {
                app.scroll_up(3);
            }
        }
        _ => {}
    }
}

fn rect_contains(rect: ratatui::layout::Rect, col: u16, row: u16) -> bool {
    col >= rect.x && col < rect.x + rect.width && row >= rect.y && row < rect.y + rect.height
}

fn play_audio_file(app: &mut App, archive: &str, filename: &str) {
    let data_dir = match &app.config.data_dir {
        Some(d) => d.clone(),
        None => return,
    };

    let entries = match data::list_files(&data_dir, archive) {
        Ok(e) => e,
        Err(_) => return,
    };

    let entry = match entries.iter().find(|e| e.name == filename) {
        Some(e) => e.clone(),
        None => return,
    };

    let mut reader = match data::ArchiveReader::new(&data_dir, archive) {
        Ok(r) => r,
        Err(_) => return,
    };

    if let Ok(data) = reader.read(entry.location) {
        app.play_audio(data);
    }
}
