//! Three-pane layout helper.
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │  Search [__________________]               [ADV]         │  row 0 (3 lines)
//! ├──────────────────┬──────────────────────────────────────┤
//! │  Results         │  Content                              │  row 1 (fill)
//! │                  │                                       │
//! ├──────────────────┴──────────────────────────────────────┤
//! │  [FIND bar]  (shown in FindInPage mode only)             │  row 2 (3 lines, optional)
//! ├─────────────────────────────────────────────────────────┤
//! │  Status bar                                             │  row 3 (1 line)
//! └─────────────────────────────────────────────────────────┘
//! ```

use ratatui::layout::{Constraint, Direction, Layout, Rect};

use crate::app::{App, AppMode};

#[derive(Debug, Clone)]
pub struct AppLayout {
    pub search: Rect,
    pub results: Rect,
    pub content: Rect,
    pub findbar: Option<Rect>,
    pub status: Rect,
}

pub fn compute_layout(area: Rect, app: &App) -> AppLayout {
    let find_height: u16 = if app.mode == AppMode::FindInPage {
        3
    } else {
        0
    };
    let search_height: u16 = if app.spell_suggestions.is_empty() {
        3
    } else {
        4
    };

    // Vertical split: [search | main | findbar | status]
    let v_constraints = if find_height > 0 {
        vec![
            Constraint::Length(search_height),
            Constraint::Min(5),
            Constraint::Length(find_height),
            Constraint::Length(1),
        ]
    } else {
        vec![
            Constraint::Length(search_height),
            Constraint::Min(5),
            Constraint::Length(1),
        ]
    };

    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(v_constraints)
        .split(area);

    // Horizontal split for the main row: [results | content]
    let result_width = (area.width / 3).max(20).min(50);
    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(result_width), Constraint::Min(30)])
        .split(v_chunks[1]);

    let (findbar, status) = if find_height > 0 {
        (Some(v_chunks[2]), v_chunks[3])
    } else {
        (None, v_chunks[2])
    };

    AppLayout {
        search: v_chunks[0],
        results: h_chunks[0],
        content: h_chunks[1],
        findbar,
        status,
    }
}
