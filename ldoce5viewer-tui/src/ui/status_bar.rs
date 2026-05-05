//! Status bar widget.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::app::{App, AppMode};

pub struct StatusBar<'a> {
    pub app: &'a App,
}

impl<'a> Widget for StatusBar<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let left = build_left(self.app);
        let right = build_right(self.app);

        // Compute padding
        let lw: usize = left.iter().map(|s| s.content.len()).sum();
        let rw: usize = right.iter().map(|s| s.content.len()).sum();
        let pad = (area.width as usize)
            .saturating_sub(lw + rw);

        let mut spans = left;
        spans.push(Span::raw(" ".repeat(pad)));
        spans.extend(right);

        Paragraph::new(Line::from(spans))
            .style(Style::default().bg(Color::DarkGray))
            .render(area, buf);
    }
}

fn build_left(app: &App) -> Vec<Span<'static>> {
    let mode_label = match app.mode {
        AppMode::Searching      => " SEARCH ",
        AppMode::Normal         => " NORMAL ",
        AppMode::ContentFocused => " CONTENT",
        AppMode::FindInPage     => " FIND   ",
        AppMode::AdvancedSearch => " ADV    ",
        AppMode::BuildingIndex  => " BUILD  ",
        AppMode::Quit           => " QUIT   ",
    };
    let mode_style = Style::default()
        .bg(Color::Blue)
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);

    let mut spans = vec![Span::styled(mode_label.to_owned(), mode_style)];

    // Show "◀ ▶" nav indicators
    let back = if app.history.can_go_back()    { "◀ " } else { "  " };
    let fwd  = if app.history.can_go_forward() { "▶ " } else { "  " };
    spans.push(Span::styled(format!(" {back}{fwd}"), Style::default().fg(Color::White)));

    // Status message
    if !app.status.is_empty() {
        spans.push(Span::styled(
            format!("  {}", app.status),
            Style::default().fg(Color::Yellow),
        ));
    }

    // "Searching…" spinner
    if app.is_searching {
        spans.push(Span::styled("  ⟳ Searching…", Style::default().fg(Color::Cyan)));
    }

    spans
}

fn build_right(app: &App) -> Vec<Span<'static>> {
    let mut parts: Vec<Span> = Vec::new();

    // Auto-pron indicator
    let pron_label = match app.config.auto_pron {
        crate::config::AutoPronLanguage::Off => "",
        crate::config::AutoPronLanguage::GB  => "🔊GB ",
        crate::config::AutoPronLanguage::US  => "🔊US ",
    };
    if !pron_label.is_empty() {
        parts.push(Span::styled(pron_label.to_owned(), Style::default().fg(Color::Green)));
    }

    // Clipboard monitor indicator
    if app.config.monitor_clipboard {
        parts.push(Span::styled("📋 ", Style::default().fg(Color::Magenta)));
    }

    // Zoom level
    if app.zoom_power != 0 {
        let sign = if app.zoom_power > 0 { "+" } else { "" };
        parts.push(Span::styled(
            format!(" zoom:{sign}{} ", app.zoom_power),
            Style::default().fg(Color::White),
        ));
    }

    // Scroll position in content
    if let Some(page) = &app.content_page {
        let total = page.len().max(1);
        let pct = (app.content_scroll * 100) / total;
        parts.push(Span::styled(
            format!(" {pct}% "),
            Style::default().fg(Color::DarkGray),
        ));
    }

    parts
}

// --------------------------------------------------------------------------
// Find-bar widget (shown above the status bar when FindInPage mode active)
// --------------------------------------------------------------------------

pub struct FindBar<'a> {
    pub app: &'a App,
}

impl<'a> Widget for FindBar<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.app.mode != AppMode::FindInPage { return; }

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(" Find ");
        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height == 0 { return; }

        let match_count = self.app.find_matches.len();
        let current = if match_count == 0 { 0 } else { self.app.find_cursor + 1 };

        let mut spans = vec![
            Span::raw("Find: "),
            Span::styled(
                self.app.find_text.clone(),
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
        ];
        if match_count == 0 {
            spans.push(Span::styled("No matches", Style::default().fg(Color::Red)));
        } else {
            spans.push(Span::styled(
                format!("{current} / {match_count}"),
                Style::default().fg(Color::Green),
            ));
        }

        Paragraph::new(Line::from(spans)).render(inner, buf);
    }
}
