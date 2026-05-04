//! Search input pane widget.

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::app::{App, AppMode};

pub struct SearchPane<'a> {
    pub app: &'a App,
}

impl<'a> Widget for SearchPane<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let is_active = self.app.mode == AppMode::Searching
            || self.app.mode == AppMode::Normal;

        let border_style = if self.app.mode == AppMode::Searching {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(Span::styled(" Search ", Style::default().add_modifier(Modifier::BOLD)));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.width == 0 || inner.height == 0 {
            return;
        }

        // Build the line with cursor
        let text = &self.app.search_text;
        let cursor_pos = self.app.search_cursor;

        // Show up to `inner.width` chars around the cursor
        let display_width = inner.width as usize;
        // Simple scroll: show a window around the cursor
        let start_char = if cursor_pos + 1 > display_width {
            cursor_pos + 1 - display_width
        } else {
            0
        };

        let visible: String = text
            .chars()
            .skip(start_char)
            .take(display_width)
            .collect();

        let cursor_in_view = cursor_pos - start_char;

        let before: String = visible.chars().take(cursor_in_view).collect();
        let cursor_char: String = visible
            .chars()
            .nth(cursor_in_view)
            .map(|c| c.to_string())
            .unwrap_or_else(|| " ".to_owned());
        let after: String = visible
            .chars()
            .skip(cursor_in_view + 1)
            .collect();

        let cursor_style = if self.app.mode == AppMode::Searching {
            Style::default()
                .bg(Color::Cyan)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let line = Line::from(vec![
            Span::raw(before),
            Span::styled(cursor_char, cursor_style),
            Span::raw(after),
        ]);

        Paragraph::new(line)
            .alignment(Alignment::Left)
            .render(inner, buf);

        // Spell suggestions below the input (if any and if there's room)
        if !self.app.spell_suggestions.is_empty() && inner.height > 1 {
            let suggestions = self.app.spell_suggestions.join("  ·  ");
            let suggestion_line = Line::from(vec![
                Span::styled("Did you mean: ", Style::default().fg(Color::Yellow)),
                Span::styled(suggestions, Style::default().fg(Color::White).add_modifier(Modifier::ITALIC)),
            ]);
            let suggest_area = Rect {
                x: inner.x,
                y: inner.y + 1,
                width: inner.width,
                height: 1,
            };
            Paragraph::new(suggestion_line).render(suggest_area, buf);
        }
    }
}
