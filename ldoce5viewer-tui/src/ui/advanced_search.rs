//! Advanced search overlay widget.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Widget},
};

use crate::app::{App, AppMode, FilterNode};

pub struct AdvancedSearchOverlay<'a> {
    pub app: &'a App,
}

impl<'a> Widget for AdvancedSearchOverlay<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.app.mode != AppMode::AdvancedSearch {
            return;
        }

        // Center a 70% wide, 80% tall overlay
        let w = (area.width * 70 / 100).max(40);
        let h = (area.height * 80 / 100).max(10);
        let x = area.x + (area.width.saturating_sub(w)) / 2;
        let y = area.y + (area.height.saturating_sub(h)) / 2;
        let overlay = Rect::new(x, y, w, h);

        // Clear the background
        Clear.render(overlay, buf);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(" Advanced Search  [Esc to close]  [Enter to search] ");
        let inner = block.inner(overlay);
        block.render(overlay, buf);

        // Split: phrase input + filter tree
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(1)])
            .split(inner);

        // Phrase input
        let phrase_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" Phrase ");
        Paragraph::new(Line::from(vec![Span::raw(&*self.app.adv_phrase)]))
            .block(phrase_block)
            .render(chunks[0], buf);

        // Filter tree
        let tree_items = collect_tree_items(&self.app.adv_filter_tree, 0);
        let list = List::new(tree_items)
            .highlight_style(Style::default().bg(Color::Blue).fg(Color::White));
        list.render(chunks[1], buf);
    }
}

fn collect_tree_items(nodes: &[FilterNode], depth: usize) -> Vec<ListItem<'static>> {
    let mut items = Vec::new();
    for node in nodes {
        let indent = "  ".repeat(depth);
        let checkbox = if node.checked { "[x] " } else { "[ ] " };
        let label = format!("{indent}{checkbox}{}", node.label);
        let style = if depth == 0 {
            Style::default().add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        items.push(ListItem::new(Line::from(Span::styled(label, style))));
        if !node.children.is_empty() {
            items.extend(collect_tree_items(&node.children, depth + 1));
        }
    }
    items
}
