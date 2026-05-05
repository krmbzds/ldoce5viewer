//! Scrollable rich-text content view widget.
//!
//! Renders a `ContentPage` (a `Vec<Block>`) as a scrollable pane, applying
//! syntax highlighting and zoom.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block as RatatuiBlock, Borders, Paragraph, Widget, Wrap},
};

use crate::app::{App, AppMode};
use crate::content::{to_ratatui_text, Inline};

pub struct ContentView<'a> {
    pub app: &'a App,
}

impl<'a> Widget for ContentView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let is_active = !matches!(self.app.mode, AppMode::Searching);
        let border_style = if is_active {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        // Prefer a human-friendly headword title from the rendered page when
        // available; fall back to the final path component.
        let title = if let Some(page) = &self.app.content_page {
            if let Some(hw) = extract_headword(page) {
                format!(" {} ", hw)
            } else if let Some(path) = &self.app.current_path {
                format!(" {} ", path_to_title(path))
            } else {
                " Content ".to_owned()
            }
        } else if let Some(path) = &self.app.current_path {
            format!(" {} ", path_to_title(path))
        } else {
            " Content ".to_owned()
        };

        let block = RatatuiBlock::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(Span::styled(title, Style::default().add_modifier(Modifier::BOLD)));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height == 0 || inner.width == 0 { return; }

        if let Some(page) = &self.app.content_page {
            // Render using the pre-built Text; avoid remapping blocks → lines which
            // previously caused clipping and incorrect scrolling. We clamp the
            // requested scroll to the number of rendered lines so scrolling behaves
            // sensibly even when the content wraps to multiple lines.
            let text = to_ratatui_text(page);
            let total_lines = text.lines.len();
            let scroll_y = if total_lines == 0 { 0 } else { self.app.content_scroll.min(total_lines.saturating_sub(1)) } as u16;

            Paragraph::new(text)
                .wrap(Wrap { trim: false })
                .scroll((scroll_y, 0))
                .render(inner, buf);
        } else {
            // Show a help message when no content is loaded
            let help = Text::from(vec![
                Line::from(""),
                Line::from(vec![Span::styled(
                    "  Type in the search box to find dictionary entries.",
                    Style::default().fg(Color::DarkGray),
                )]),
                Line::from(""),
                Line::from(vec![Span::styled(
                    "  Keyboard shortcuts:",
                    Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD),
                )]),
                Line::from(vec![Span::styled(
                    "    /          Focus search box",
                    Style::default().fg(Color::DarkGray),
                )]),
                Line::from(vec![Span::styled(
                    "    j / k      Select next / previous result",
                    Style::default().fg(Color::DarkGray),
                )]),
                Line::from(vec![Span::styled(
                    "    Enter      Load selected entry",
                    Style::default().fg(Color::DarkGray),
                )]),
                Line::from(vec![Span::styled(
                    "    Ctrl+B     Navigate back",
                    Style::default().fg(Color::DarkGray),
                )]),
                Line::from(vec![Span::styled(
                    "    Ctrl+F     Navigate forward",
                    Style::default().fg(Color::DarkGray),
                )]),
                Line::from(vec![Span::styled(
                    "    /          Find in page (when content focused)",
                    Style::default().fg(Color::DarkGray),
                )]),
                Line::from(vec![Span::styled(
                    "    Ctrl+G / Ctrl+U  Play GB / US pronunciation",
                    Style::default().fg(Color::DarkGray),
                )]),
                Line::from(vec![Span::styled(
                    "    + / -      Zoom in / out",
                    Style::default().fg(Color::DarkGray),
                )]),
                Line::from(vec![Span::styled(
                    "    q          Quit",
                    Style::default().fg(Color::DarkGray),
                )]),
            ]);
            Paragraph::new(help).render(inner, buf);
        }
    }
}

/// Build a `Text` with find-match highlights applied.
fn build_ratatui_text<'t>(
    page: &'t [crate::content::transform::Block],
    find_query: &str,
    find_matches: &[usize],
) -> Text<'static> {
    let base = to_ratatui_text(page);
    if find_query.is_empty() {
        return base;
    }

    let q = find_query.to_lowercase();
    let match_set: std::collections::HashSet<usize> = find_matches.iter().copied().collect();

    // Re-walk the page blocks, highlighting any block that contains a match
    let mut lines: Vec<Line> = Vec::new();
    for (block_idx, block) in page.iter().enumerate() {
        let block_text: String = block
        .inlines
        .iter()
        .filter_map(|i| match i {
            Inline::Text(t, _) => Some(t.as_str()),
            Inline::Headword(t) => Some(t.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("");

        if match_set.contains(&block_idx) && !q.is_empty() {
            // Highlight all occurrences of `q` within the block
            let lower = block_text.to_lowercase();
            let mut spans: Vec<Span> = Vec::new();
            if block.indent > 0 {
                spans.push(Span::raw(" ".repeat(block.indent as usize * 2)));
            }
            let mut prev = 0;
            let mut start = 0;
            while let Some(pos) = lower[start..].find(&q) {
                let abs = start + pos;
                if abs > prev {
                    spans.push(Span::raw(block_text[prev..abs].to_owned()));
                }
                spans.push(Span::styled(
                    block_text[abs..abs + q.len()].to_owned(),
                    Style::default().bg(Color::Yellow).fg(Color::Black),
                ));
                prev  = abs + q.len();
                start = abs + q.len();
            }
            if prev < block_text.len() {
                spans.push(Span::raw(block_text[prev..].to_owned()));
            }
            lines.push(Line::from(spans));
        } else {
            // Use the pre-rendered line from base Text
            // We use the index into base.lines (one block → one or more lines)
            // Approximate: one block = one line for line count purposes
            if let Some(line) = base.lines.get(block_idx) {
                lines.push(line.clone());
            }
        }
    }

    Text::from(lines)
}

fn path_to_title(path: &str) -> &str {
    path.trim_start_matches('/').split('/').last().unwrap_or(path)
}

fn extract_headword(page: &[crate::content::transform::Block]) -> Option<String> {
    // Find the first Inline::Headword and return it.
    for block in page {
        for inline in &block.inlines {
            if let crate::content::transform::Inline::Headword(text) = inline {
                let s = text.trim();
                if !s.is_empty() { return Some(s.to_string()); }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_to_title() {
        assert_eq!(path_to_title("/fs/3.4.6.2"), "3.4.6.2");
        assert_eq!(path_to_title("entry"),       "entry");
        assert_eq!(path_to_title(""),            "");
    }
}
