//! UI module: widget composition and the top-level `draw()` function.

pub mod advanced_search;
pub mod content_view;
pub mod layout;
pub mod result_list;
pub mod search_pane;
pub mod status_bar;

use ratatui::Frame;

use crate::app::App;
use advanced_search::AdvancedSearchOverlay;
use content_view::ContentView;
use layout::compute_layout;
use result_list::ResultList;
use search_pane::SearchPane;
use status_bar::{FindBar, StatusBar};

/// Top-level draw function: renders all widgets into a single frame.
pub fn draw(f: &mut Frame, app: &App) {
    let layout = compute_layout(f.area(), app);

    f.render_widget(SearchPane { app }, layout.search);
    f.render_widget(ResultList { app }, layout.results);
    f.render_widget(ContentView { app }, layout.content);
    if let Some(fb_rect) = layout.findbar {
        f.render_widget(FindBar { app }, fb_rect);
    }
    f.render_widget(StatusBar { app }, layout.status);

    // Overlay (advanced search)
    f.render_widget(AdvancedSearchOverlay { app }, f.area());
}
