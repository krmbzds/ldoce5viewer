//! Scrollable result list widget.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Widget},
};

use crate::app::{App, AppMode};
use crate::content::SearchResultItem;

pub struct ResultList<'a> {
    pub app: &'a App,
}

impl<'a> Widget for ResultList<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let is_active = self.app.mode == AppMode::Normal;
        let border_style = if is_active {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(Span::styled(
                format!(" Results ({}) ", self.app.results.len()),
                Style::default().add_modifier(Modifier::BOLD),
            ));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height == 0 { return; }

        let items: Vec<ListItem> = self
            .app
            .results
            .iter()
            .enumerate()
            .map(|(i, item)| render_result_item(item, Some(i) == self.app.selected_row))
            .collect();

        // Build a ListState to control scroll
        let mut state = ListState::default();
        state.select(self.app.selected_row);

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            );

        ratatui::widgets::StatefulWidget::render(list, inner, buf, &mut state);
    }
}

fn render_result_item(item: &SearchResultItem, _selected: bool) -> ListItem<'static> {
    // Strip lightweight markup tags like `<hw>`, `<pos>`, `</hw>` etc.
    let label = strip_markup(&item.label);
    ListItem::new(Line::from(vec![Span::raw(label)]))
}

/// Remove `<tag>` and `</tag>` markup from a label string.
fn strip_markup(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for ch in s.chars() {
        match ch {
            '<' => { in_tag = true; }
            '>' => { in_tag = false; }
            _   => { if !in_tag { out.push(ch); } }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_markup() {
        assert_eq!(strip_markup("Hello <b>World</b>!"), "Hello World!");
        assert_eq!(strip_markup("<hw>run</hw>  <pos>verb</pos>"), "run  verb");
        assert_eq!(strip_markup("plain text"), "plain text");
    }
}
