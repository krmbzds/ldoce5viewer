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
    // Parse the lightweight markup in `item.label` into text+style parts, then
    // split into tokens, collapse adjacent duplicate tokens (case-insensitive),
    // and re-merge tokens into styled spans for rendering.
    let parts = parse_label_to_spans(&item.label);

    // Flatten parts into per-token (token, style) pairs
    let mut tokens: Vec<(String, Style)> = Vec::new();
    for (text, style) in parts {
        for tok in text.split_whitespace() {
            tokens.push((tok.to_string(), style));
        }
    }

    // Collapse adjacent duplicate tokens (case-insensitive)
    let mut deduped: Vec<(String, Style)> = Vec::new();
    for (tok, style) in tokens {
        if deduped.last().map(|(p, _)| p.eq_ignore_ascii_case(&tok)).unwrap_or(false) {
            continue;
        }
        deduped.push((tok, style));
    }

    // Merge consecutive tokens with the same style into single spans
    let mut spans_out: Vec<Span<'static>> = Vec::new();
    if !deduped.is_empty() {
        let mut cur_style = deduped[0].1;
        let mut cur_text = deduped[0].0.clone();
        for (tok, style) in deduped.into_iter().skip(1) {
            if style == cur_style {
                cur_text.push(' ');
                cur_text.push_str(&tok);
            } else {
                spans_out.push(Span::styled(cur_text.clone(), cur_style));
                cur_style = style;
                cur_text = tok;
            }
        }
        spans_out.push(Span::styled(cur_text, cur_style));
    }

    if spans_out.is_empty() {
        spans_out.push(Span::raw(""));
    }

    ListItem::new(Line::from(spans_out))
}

fn collapse_adjacent_duplicates(s: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    for tok in s.split_whitespace() {
        if out.last().map(|p| p.eq_ignore_ascii_case(tok)).unwrap_or(false) {
            // skip duplicate
            continue;
        }
        out.push(tok.to_string());
    }
    out.join(" ")
}

fn parse_label_to_spans(s: &str) -> Vec<(String, Style)> {
    // Simple tag -> style mapping. Only a small subset of tags used by the
    // label generator are handled here; unknown tags are ignored.
    fn style_for_tag(tag: &str) -> Style {
        match tag {
            "h" | "H" | "hw" => Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            "v" => Style::default().fg(Color::Magenta),
            "p" => Style::default().fg(Color::Yellow),
            "f" => Style::default().add_modifier(Modifier::DIM),
            "s" => Style::default().add_modifier(Modifier::DIM),
            _ => Style::default(),
        }
    }

    let mut parts: Vec<(String, Style)> = Vec::new();
    let mut style_stack: Vec<Style> = Vec::new();
    let mut buf = String::new();
    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '<' {
            // flush buffer
            if !buf.is_empty() {
                let current_style = style_stack.last().copied().unwrap_or_default();
                parts.push((buf.clone(), current_style));
                buf.clear();
            }
            // parse tag
            let mut tagname = String::new();
            let mut is_close = false;
            if let Some(&c) = chars.peek() {
                if c == '/' { is_close = true; chars.next(); }
            }
            while let Some(&c) = chars.peek() {
                chars.next();
                if c == '>' { break; }
                if c.is_whitespace() { break; }
                tagname.push(c);
            }
            // skip until '>' if not already
            while let Some(&c) = chars.peek() {
                if c == '>' { chars.next(); break; }
                chars.next();
            }
            let tagname = tagname.trim();
            if is_close {
                // pop matching style (best-effort)
                // If stack empty, ignore
                if !style_stack.is_empty() { style_stack.pop(); }
            } else {
                // push style for this tag
                let st = style_for_tag(tagname);
                style_stack.push(st);
            }
        } else {
            buf.push(ch);
        }
    }
    if !buf.is_empty() {
        let current_style = style_stack.last().copied().unwrap_or_default();
        parts.push((buf, current_style));
    }
    parts
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
