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
    // split into tokens and apply deduplication.
    let parts = parse_label_to_spans(&item.label);

    // Flatten parts into per-token (token, style) pairs
    let mut tokens: Vec<(String, Style)> = Vec::new();
    for (text, style) in parts {
        for tok in text.split_whitespace() {
            tokens.push((tok.to_string(), style));
        }
    }

    // Remove a plain-text prefix that duplicates the beginning of the styled
    // (markup) portion.  LDOCE5 labels are often stored as
    //   "car alarm <hw>car alarm</hw> <p>noun</p>"
    // i.e. the plaintext headword appears before its own markup.  After
    // token-splitting the default-styled "car alarm" comes before the hw-styled
    // "car alarm", and the result list shows "car alarm car alarm noun".
    let tokens = remove_plain_prefix(tokens);

    // Collapse adjacent duplicate tokens (case-insensitive) as a secondary
    // safety net for any remaining repetition.
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

/// Remove a leading run of default-styled tokens that is immediately repeated
/// in the non-default-styled tokens that follow.
///
/// Example: `["car"(plain), "alarm"(plain), "car"(hw), "alarm"(hw), "noun"(pos)]`
/// → `["car"(hw), "alarm"(hw), "noun"(pos)]`
fn remove_plain_prefix(mut tokens: Vec<(String, Style)>) -> Vec<(String, Style)> {
    if tokens.len() < 2 {
        return tokens;
    }
    let default_style = Style::default();
    // Count leading default-styled tokens
    let prefix_end = tokens
        .iter()
        .take_while(|(_, s)| *s == default_style)
        .count();
    if prefix_end == 0 || tokens.len() < prefix_end * 2 {
        return tokens;
    }
    // Check whether tokens[0..prefix_end] matches tokens[prefix_end..prefix_end*2]
    // (case-insensitive word comparison)
    let matches = tokens[..prefix_end]
        .iter()
        .zip(tokens[prefix_end..prefix_end * 2].iter())
        .all(|((t1, _), (t2, _))| t1.eq_ignore_ascii_case(t2));
    if matches {
        tokens.drain(..prefix_end);
    }
    tokens
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_plain_prefix_compound() {
        // Simulates label "car alarm <hw>car alarm</hw> <pos>noun</pos>"
        let hw_style  = Style::default().fg(ratatui::style::Color::Cyan)
            .add_modifier(ratatui::style::Modifier::BOLD);
        let pos_style = Style::default().fg(ratatui::style::Color::Yellow);
        let tokens = vec![
            ("car".to_string(),   Style::default()),
            ("alarm".to_string(), Style::default()),
            ("car".to_string(),   hw_style),
            ("alarm".to_string(), hw_style),
            ("noun".to_string(),  pos_style),
        ];
        let result = remove_plain_prefix(tokens);
        assert_eq!(result.len(), 3, "plain prefix should be removed");
        assert_eq!(result[0].0, "car");
        assert_eq!(result[0].1, hw_style);
    }

    #[test]
    fn test_remove_plain_prefix_single_no_change() {
        // Single-word label with no markup prefix — must stay unchanged
        let tokens = vec![("Caracas".to_string(), Style::default())];
        let result = remove_plain_prefix(tokens.clone());
        assert_eq!(result, tokens);
    }

    #[test]
    fn test_remove_plain_prefix_no_plain_prefix() {
        // Label that starts directly with a styled token — no change
        let hw_style = Style::default().fg(ratatui::style::Color::Cyan)
            .add_modifier(ratatui::style::Modifier::BOLD);
        let tokens = vec![
            ("run".to_string(), hw_style),
            ("verb".to_string(), Style::default().fg(ratatui::style::Color::Yellow)),
        ];
        let result = remove_plain_prefix(tokens.clone());
        assert_eq!(result, tokens);
    }
}
