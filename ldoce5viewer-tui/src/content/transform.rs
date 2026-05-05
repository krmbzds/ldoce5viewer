//! XML → ratatui styled-text transformer.
//!
//! Converts LDOCE5 XML content into a `ContentPage` (a `Vec<Block>`) that
//! can be rendered by the `ContentView` TUI widget.
//!
//! Handles all ten content types documented in the original Python code:
//! Entry, Thesaurus, Collocations, WordSets, Phrases, Examples,
//! WordFamilies, Etymologies, Activator (concept + section).

use quick_xml::events::Event;
use quick_xml::Reader;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use unicode_width::UnicodeWidthStr;

use crate::content::types::ContentType;

// --------------------------------------------------------------------------
// Rich-text model
// --------------------------------------------------------------------------

/// An inline element within a `Block`.
#[derive(Debug, Clone)]
pub enum Inline {
    /// Styled plain text.
    Text(String, Style),
    /// Headword text (semantic variant) — used to identify the main headword
    /// and to render it specially in the title of the content view.
    Headword(String),
    /// A small prefix for a block (sense number, bullet, etc) rendered in a
    /// short fixed-width column to the left of the block text.
    Prefix(String, Style),
    /// An audio playback button:  `♪ <title>`.
    AudioButton { path: String, title: String },
    /// An image placeholder.
    Image { filename: String },
    /// A cross-reference / link.
    Link { text: String, target: String },
    /// A line break within a block.
    LineBreak,
    /// A frequency/corpus badge: S1, W2, AC etc.  Rendered as `[text]`.
    Badge { text: String },
    /// A signpost label — rendered with a visual box to indicate topic.
    Signpost { text: String },
}

/// A block of content (analogous to an HTML `<div>` / `<p>`).
#[derive(Debug, Clone)]
pub struct Block {
    /// Indentation level (each level = 2 spaces).
    pub indent: u8,
    /// Inlines that make up this block's content.
    pub inlines: Vec<Inline>,
    /// Whether this block comes from a collocations/collo-related section. When
    /// true we post-process the block by splitting it into multiple lines so
    /// each collocation / example appears on its own line.
    pub is_collo: bool,
    /// Whether this block is a section box heading (COLLOCATIONS, THESAURUS,
    /// GRAMMAR, etc.) — rendered with a dark header bar.
    pub is_heading: bool,
}

impl Default for Block {
    fn default() -> Self {
        Block::new(0)
    }
}

impl Block {
    fn new(indent: u8) -> Self {
        Block {
            indent,
            inlines: Vec::new(),
            is_collo: false,
            is_heading: false,
        }
    }

    /// Append frequency-badge text — each FREQ element gets its own Badge
    /// inline so that "[S1] [W2]" are rendered as two separate pills.
    fn push_badge(&mut self, text: &str) {
        // Trim and ignore empty/whitespace-only badges
        let t = text.trim();
        if t.is_empty() {
            return;
        }
        self.inlines.push(Inline::Badge { text: t.to_owned() });
    }

    /// Append signpost text, merging with an adjacent Signpost if present.
    fn push_signpost(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        if let Some(Inline::Signpost { text: existing }) = self.inlines.last_mut() {
            existing.push_str(text);
        } else {
            self.inlines.push(Inline::Signpost {
                text: text.to_owned(),
            });
        }
    }

    fn push_text(&mut self, text: &str, style: Style) {
        if text.is_empty() {
            return;
        }
        // If the last inline is text with the same style, append to it (inserting a
        // space when needed).
        if let Some(last) = self.inlines.last_mut() {
            if let Inline::Text(last_text, last_style) = last {
                if *last_style == style {
                    let need_space = last_text
                        .chars()
                        .rev()
                        .next()
                        .map(|c| c.is_alphanumeric())
                        .unwrap_or(false)
                        && text
                            .chars()
                            .next()
                            .map(|c| c.is_alphanumeric())
                            .unwrap_or(false);
                    if need_space {
                        last_text.push(' ');
                    }
                    last_text.push_str(text);
                    return;
                }
            }
        }
        self.inlines.push(Inline::Text(text.to_owned(), style));
    }

    fn push_headword(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        if let Some(last) = self.inlines.last_mut() {
            if let Inline::Headword(last_text) = last {
                // choose to insert a space when joining two alphanumeric tokens
                let need_space = last_text
                    .chars()
                    .rev()
                    .next()
                    .map(|c| c.is_alphanumeric())
                    .unwrap_or(false)
                    && text
                        .chars()
                        .next()
                        .map(|c| c.is_alphanumeric())
                        .unwrap_or(false);
                if need_space {
                    last_text.push(' ');
                }
                last_text.push_str(text);
                return;
            }
        }
        self.inlines.push(Inline::Headword(text.to_owned()));
    }
}

/// A fully rendered page, ready to be turned into ratatui `Text`.
pub type ContentPage = Vec<Block>;

// --------------------------------------------------------------------------
// Style constants (Dracula Pro palette)
// --------------------------------------------------------------------------

fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb(r, g, b)
}

fn style_default() -> Style {
    Style::default().fg(rgb(248, 248, 242)) // white
}
fn style_headword() -> Style {
    Style::default()
        .fg(rgb(153, 255, 238)) // bright_cyan #99FFEE
        .add_modifier(Modifier::BOLD)
}
fn style_pos() -> Style {
    Style::default().fg(rgb(255, 255, 128)) // yellow #FFFF80
}
fn style_def() -> Style {
    Style::default().fg(rgb(248, 248, 242)) // fg
}
fn style_example() -> Style {
    Style::default()
        .fg(rgb(138, 255, 128)) // green #8AFF80
        .add_modifier(Modifier::ITALIC)
}
fn style_ref() -> Style {
    Style::default()
        .fg(rgb(128, 255, 234)) // cyan #80FFEA
        .add_modifier(Modifier::UNDERLINED)
}
fn style_label() -> Style {
    Style::default().fg(rgb(255, 128, 191)) // pink #FF80BF
}
fn style_heading() -> Style {
    Style::default()
        .fg(rgb(162, 255, 153)) // bright_green #A2FF99
        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
}
fn style_audio() -> Style {
    Style::default().fg(rgb(128, 255, 234)) // cyan
}
fn style_dim() -> Style {
    Style::default()
        .fg(rgb(121, 112, 169)) // comment purple
        .add_modifier(Modifier::DIM)
}
/// Badge styling: provide separate styles for the bracket (border) and the
/// inner pill so we can visually separate the letter(s) with a small "border".
fn style_badge_border() -> Style {
    // light gray bracket color
    Style::default()
        .fg(rgb(180, 180, 190))
        .add_modifier(Modifier::DIM)
}

fn style_badge_inner() -> Style {
    // Muted purple background with light foreground for contrast.
    Style::default()
        .fg(rgb(248, 248, 242)) // white text
        .bg(rgb(121, 112, 169)) // #7970A9 muted purple
        .add_modifier(Modifier::BOLD)
}
/// Style for signpost labels (e.g. "DRIVING", "CAR JOURNEY").
fn style_signpost() -> Style {
    Style::default()
        .fg(rgb(248, 248, 242)) // white text
        .bg(rgb(63, 115, 115)) // dark teal background (#3f7373)
        .add_modifier(Modifier::BOLD)
}
/// Style for the text of section box headings (COLLOCATIONS, THESAURUS…).
fn style_section_heading() -> Style {
    // Use the requested background color (#7970A9) with light foreground.
    Style::default()
        .fg(rgb(248, 248, 242)) // light foreground (white)
        .bg(rgb(121, 112, 169)) // #7970A9
        .add_modifier(Modifier::BOLD)
}

// --------------------------------------------------------------------------
// to_ratatui_text  (ContentPage → ratatui Text)
// --------------------------------------------------------------------------

/// Convert a `ContentPage` into a ratatui `Text` object.
pub fn to_ratatui_text(page: &[Block]) -> Text<'static> {
    let mut lines: Vec<Line> = Vec::new();

    // Compute maximum prefix width across the whole page (in terminal cells)
    // so circled numbers and bullets align vertically.
    let max_prefix = page
        .iter()
        .filter_map(|b| {
            if let Some(Inline::Prefix(s, _)) = b.inlines.get(0) {
                Some(UnicodeWidthStr::width(s.as_str()))
            } else {
                None
            }
        })
        .max()
        .unwrap_or(0usize);

    // Also compute maximum indentation (in spaces) used by any block. We'll
    // reserve a fixed prefix column at this horizontal position so circled
    // numbers are vertically aligned regardless of nesting.
    let max_indent_spaces = page
        .iter()
        .map(|b| (b.indent as usize) * 2)
        .max()
        .unwrap_or(0usize);

    // Prefix column configuration: fixed left margin, a fixed inner width
    // (based on the widest prefix), and one separator cell.
    let prefix_col_margin: usize = 2;
    let prefix_col_width: usize = std::cmp::max(1, max_prefix);
    let prefix_col_total: usize = prefix_col_margin + prefix_col_width + 1;

    for block in page.iter() {
        let indent_str = if block.indent > 0 {
            " ".repeat(block.indent as usize * 2)
        } else {
            String::new()
        };

        // Detect explicit Prefix inline at the start (if present).
        let mut start_idx = 0usize;
        let mut prefix_opt: Option<(String, Style)> = None;
        if let Some(Inline::Prefix(s, st)) = block.inlines.get(0) {
            prefix_opt = Some((s.clone(), *st));
            start_idx = 1;
        }

        // prefix column totals are handled via `prefix_col_*` variables above.

        // For section heading blocks (COLLOCATIONS, THESAURUS etc.) render the
        // heading inline but aligned with the following text. Use a subtler
        // background and avoid inserting an extra blank line above the header.
        if block.is_heading {
            // Render heading aligned with the content column after the prefix column.
            let mut current: Vec<Span> = Vec::new();
            // Prefix column (left margin + prefix cell)
            current.push(Span::raw(" ".repeat(prefix_col_margin)));
            if let Some((ps, pst)) = &prefix_opt {
                let cur_w = UnicodeWidthStr::width(ps.as_str());
                if prefix_col_width > cur_w {
                    current.push(Span::raw(" ".repeat(prefix_col_width - cur_w)));
                }
                current.push(Span::styled(ps.clone(), *pst));
                // separation space between prefix column and content
                current.push(Span::raw(" "));
            } else {
                // empty prefix cell + separation
                current.push(Span::raw(" ".repeat(prefix_col_width + 1)));
            }
            // Now the block's own indentation
            if !indent_str.is_empty() {
                current.push(Span::raw(indent_str.clone()));
            }

            // Collect all inline text for the heading
            let heading_text: String = block
                .inlines
                .iter()
                .filter_map(|i| match i {
                    Inline::Text(t, _) => Some(t.as_str()),
                    Inline::Headword(t) => Some(t.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("")
                .trim()
                .to_string();

            if !heading_text.is_empty() {
                // Use uppercase and subtle padding for clarity
                let label = format!(" {} ", heading_text.to_uppercase());
                current.push(Span::styled(label, style_section_heading()));
                lines.push(Line::from(current));
            }
            continue;
        }

        // Build the first line (prefix column + indentation + content)
        let mut current: Vec<Span> = Vec::new();
        // Prefix column (left margin + prefix cell)
        current.push(Span::raw(" ".repeat(prefix_col_margin)));
        if let Some((ps, pst)) = &prefix_opt {
            let cur_w = UnicodeWidthStr::width(ps.as_str());
            if prefix_col_width > cur_w {
                current.push(Span::raw(" ".repeat(prefix_col_width - cur_w)));
            }
            current.push(Span::styled(ps.clone(), *pst));
            // separation space between prefix column and content
            current.push(Span::raw(" "));
        } else {
            // empty prefix cell + separation
            current.push(Span::raw(" ".repeat(prefix_col_width + 1)));
        }
        // Now the block's own indentation (content starts after prefix column)
        if !indent_str.is_empty() {
            current.push(Span::raw(indent_str.clone()));
        }

        for inline in block.inlines.iter().skip(start_idx) {
            match inline {
                Inline::Text(text, style) => {
                    current.push(Span::styled(text.clone(), *style));
                }
                Inline::Headword(text) => {
                    current.push(Span::styled(text.clone(), style_headword()));
                }
                Inline::AudioButton { title, .. } => {
                    let emoji = match title.as_str() {
                        "British" => "🇬🇧",
                        "American" => "🇺🇸",
                        _ => "▶",
                    };
                    current.push(Span::styled(format!(" {emoji} "), style_audio()));
                }
                Inline::Image { .. } => {
                    // Skip images in TUI
                }
                Inline::Link { text, .. } => {
                    current.push(Span::styled(text.clone(), style_ref()));
                }
                Inline::LineBreak => {
                    lines.push(Line::from(std::mem::take(&mut current)));
                    // start a new current line that aligns under the text start
                    current = Vec::new();
                    // prefix column empty (margin + cell + sep)
                    current.push(Span::raw(
                        " ".repeat(prefix_col_margin + prefix_col_width + 1),
                    ));
                    // Now add the block's indentation so wrapped lines align
                    if !indent_str.is_empty() {
                        current.push(Span::raw(indent_str.clone()));
                    }
                }
                Inline::Prefix(_, _) => {
                    // Shouldn't appear here — we handled leading Prefix above.
                }
                Inline::Badge { text } => {
                    // Render as a pill with a subtle bracket "border" and a
                    // colored inner background. Trim and skip empty badges.
                    let t = text.trim();
                    if t.is_empty() {
                        continue;
                    }
                    // leading space for separation
                    current.push(Span::raw(" "));
                    // left bracket (border)
                    current.push(Span::styled("[", style_badge_border()));
                    // inner pill with padding
                    current.push(Span::styled(format!(" {} ", t), style_badge_inner()));
                    // right bracket (border)
                    current.push(Span::styled("]", style_badge_border()));
                    // trailing space
                    current.push(Span::raw(" "));
                }
                Inline::Signpost { text } => {
                    // Render as ■ LABEL ■ with teal background style
                    current.push(Span::styled(
                        format!(" ■ {} ■ ", text.trim().to_uppercase()),
                        style_signpost(),
                    ));
                }
            }
        }

        if !current.is_empty() {
            lines.push(Line::from(current));
        }
    }

    Text::from(lines)
}

// --------------------------------------------------------------------------
// XML walking helpers
// --------------------------------------------------------------------------

struct XmlWalker<'a> {
    xml: &'a [u8],
}

/// A lightweight XML event emitter that turns quick-xml events into a flat
/// sequence of (tag, is_open, text, attrs) tuples for simple tree walking.
#[derive(Debug)]
enum XmlNode {
    Open {
        tag: String,
        attrs: Vec<(String, String)>,
    },
    Close {
        tag: String,
    },
    Text(String),
}

fn parse_xml(xml: &[u8]) -> Vec<XmlNode> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);
    let mut nodes = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let tag = String::from_utf8_lossy(e.local_name().into_inner()).into_owned();
                let mut attrs = Vec::new();
                for attr in e.attributes().flatten() {
                    let key =
                        String::from_utf8_lossy(attr.key.local_name().into_inner()).into_owned();
                    let val = attr
                        .unescape_value()
                        .map(|v| v.into_owned())
                        .unwrap_or_default();
                    attrs.push((key, val));
                }
                nodes.push(XmlNode::Open { tag, attrs });
            }
            Ok(Event::End(e)) => {
                let tag = String::from_utf8_lossy(e.local_name().into_inner()).into_owned();
                nodes.push(XmlNode::Close { tag });
            }
            Ok(Event::Empty(e)) => {
                let tag = String::from_utf8_lossy(e.local_name().into_inner()).into_owned();
                let mut attrs = Vec::new();
                for attr in e.attributes().flatten() {
                    let key =
                        String::from_utf8_lossy(attr.key.local_name().into_inner()).into_owned();
                    let val = attr
                        .unescape_value()
                        .map(|v| v.into_owned())
                        .unwrap_or_default();
                    attrs.push((key, val));
                }
                nodes.push(XmlNode::Open {
                    tag: tag.clone(),
                    attrs,
                });
                nodes.push(XmlNode::Close { tag });
            }
            Ok(Event::Text(e)) => {
                let text = e.unescape().map(|t| t.into_owned()).unwrap_or_default();
                if !text.is_empty() {
                    nodes.push(XmlNode::Text(text));
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    nodes
}

fn attr_get(attrs: &[(String, String)], key: &str) -> Option<String> {
    attrs.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
}

/// Strip LDOCE5 inline audio-cue markers from a text node.
///
/// LDOCE5 XML embeds example-level audio cues as `¿[Play]`, `¿[British]`,
/// `¿[American]` etc. — literal text nodes starting with U+00BF (`¿`) followed
/// by a `[…]` tag.  These have no displayable meaning in the TUI and must be
/// removed before the text is added to a content block.
fn strip_audio_markers(text: &str) -> std::borrow::Cow<str> {
    const ZAP: char = '\u{00BF}'; // inverted question mark used as LDOCE5 marker
    if !text.contains(ZAP) {
        return std::borrow::Cow::Borrowed(text);
    }
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == ZAP {
            // Consume the optional [...] payload that follows
            if chars.peek() == Some(&'[') {
                for c in chars.by_ref() {
                    if c == ']' {
                        break;
                    }
                }
            }
        } else {
            result.push(ch);
        }
    }
    std::borrow::Cow::Owned(result)
}

/// Convert a sense number string to circled Unicode characters.
/// Handles 1–20 using Unicode circled digit characters.
fn to_circled_number(s: &str) -> String {
    let trimmed = s.trim();
    match trimmed.parse::<u32>() {
        Ok(n @ 1..=20) => {
            // Unicode circled digits: ① = U+2460, ② = U+2461, …, ⑳ = U+2473
            let base = 0x2460u32;
            char::from_u32(base + n - 1)
                .map(|c| c.to_string())
                .unwrap_or_else(|| s.to_owned())
        }
        _ => s.to_owned(),
    }
}

/// Transform a dictionary entry XML blob into a `ContentPage`.
pub fn transform_entry(xml: &[u8]) -> ContentPage {
    let nodes = parse_xml(xml);
    let mut page: ContentPage = Vec::new();
    let mut stack: Vec<(String, Style, u8)> = Vec::new(); // (tag, inherited_style, indent)
    let mut current_block = Block::new(0);
    let mut depth = 0u8;

    // Collect all blocks into page, pushing new block when we encounter
    // block-level elements.
    fn flush(page: &mut ContentPage, block: &mut Block) {
        if !block.inlines.is_empty() {
            page.push(std::mem::replace(block, Block::new(0)));
        }
    }

    let block_tags: std::collections::HashSet<&str> = [
        "Entry",
        "Head",
        "Sense",
        "Subsense",
        "EXAMPLE",
        "GramExa",
        "ColloExa",
        "Deriv",
        "RunOn",
        "PhrVbEntry",
        "GramBox",
        "Exponent",
        "Section",
        "SECHEADING",
        "SpokenSect",
        "ThesBox",
        "ColloBox",
        "F2NBox",
        "Crossref",
        "Hint",
        "ColloGram",
    ]
    .iter()
    .copied()
    .collect();

    let skip_tags: std::collections::HashSet<&str> =
        ["ACTIV", "INFLX", "SE_EntryAssets", "EntryAsset"]
            .iter()
            .copied()
            .collect();

    for node in &nodes {
        match node {
            XmlNode::Open { tag, attrs } => {
                if skip_tags.contains(tag.as_str()) {
                    stack.push((tag.clone(), style_default(), depth));
                    depth += 1;
                    continue;
                }

                let style = style_for_tag(tag, attrs);
                let indent = if block_tags
                    .iter()
                    .any(|t| t.eq_ignore_ascii_case(tag.as_str()))
                {
                    depth
                } else {
                    current_block.indent
                };

                // Use a more specific tag name for sensenum spans so we can detect them in text nodes
                let effective_tag =
                    if tag == "span" && attr_get(attrs, "class").as_deref() == Some("sensenum") {
                        "sensenum".to_owned()
                    } else {
                        tag.clone()
                    };

                if block_tags
                    .iter()
                    .any(|t| t.eq_ignore_ascii_case(tag.as_str()))
                {
                    flush(&mut page, &mut current_block);
                    current_block = Block::new(depth.min(6));
                    // If the tag relates to collocations, mark the block so we
                    // post-process it later (split into per-example lines).
                    if tag.to_lowercase().contains("collo") {
                        current_block.is_collo = true;
                    }
                    // SECHEADING is a block-level heading tag
                    if tag.eq_ignore_ascii_case("SECHEADING") {
                        current_block.is_heading = true;
                    }
                    depth += 1;
                }

                // Handle special elements
                match tag.as_str() {
                    "Audio" => {
                        let topic = attr_get(attrs, "topic").unwrap_or_default();
                        let res = attr_get(attrs, "resource")
                            .unwrap_or_default()
                            .to_lowercase();
                        let filename = topic.split('/').last().unwrap_or("").to_owned();
                        let path = format!("/{res}/{filename}");
                        let title = match res.as_str() {
                            "gb_hwd_pron" => "British".to_owned(),
                            "us_hwd_pron" => "American".to_owned(),
                            _ => "Play".to_owned(),
                        };
                        current_block
                            .inlines
                            .push(Inline::AudioButton { path, title });
                    }
                    "ILLUSTRATION" => {
                        let thumb = attr_get(attrs, "thumb").unwrap_or_default();
                        let filename = thumb.split('/').last().unwrap_or("").to_owned();
                        current_block.inlines.push(Inline::Image { filename });
                    }
                    "Ref" => {
                        let topic = attr_get(attrs, "topic").unwrap_or_default();
                        let target = format!("/fs/{topic}");
                        stack.push((tag.clone(), style, depth));
                        depth += 1;
                        // The link text comes from the element's text children;
                        // we collect it on Close.
                        current_block.inlines.push(Inline::Link {
                            text: String::new(),
                            target,
                        });
                        continue;
                    }
                    "br" => {
                        current_block.inlines.push(Inline::LineBreak);
                    }
                    "span" => {
                        // If this is an example-bullet marker, start a new block so
                        // the example appears on its own line with correct
                        // indentation. Be resilient: class attribute may contain
                        // multiple classes or different casing, so check contains.
                        if let Some(cls) = attr_get(attrs, "class") {
                            let cls = cls.to_lowercase();
                            if cls.split_whitespace().any(|c| c == "exabullet") {
                                // flush any currently-building block and start a new
                                // block at the parent indentation level.
                                flush(&mut page, &mut current_block);
                                let indent = depth.saturating_sub(1).min(6);
                                current_block = Block::new(indent);
                                // If we are in a collocation context, mark the new
                                // block so it will be split per-example later.
                                if stack
                                    .iter()
                                    .any(|(t, _, _)| t.to_lowercase().contains("collo"))
                                {
                                    current_block.is_collo = true;
                                }
                            }
                        }
                    }
                    _ => {}
                }

                stack.push((effective_tag, style, depth));
                if !block_tags
                    .iter()
                    .any(|t| t.eq_ignore_ascii_case(tag.as_str()))
                {
                    depth += 1;
                }
            }

            XmlNode::Close { tag } => {
                // A "span" close may correspond to a "sensenum" entry on the stack
                let search_tag =
                    if tag == "span" && stack.iter().rev().any(|(t, _, _)| t == "sensenum") {
                        "sensenum"
                    } else {
                        tag.as_str()
                    };
                if let Some(pos) = stack.iter().rposition(|(t, _, _)| t == search_tag) {
                    let (_, _, d) = stack.remove(pos);
                    depth = d;

                    if block_tags
                        .iter()
                        .any(|t| t.eq_ignore_ascii_case(tag.as_str()))
                    {
                        flush(&mut page, &mut current_block);
                        current_block = Block::new(depth.saturating_sub(1).min(6));
                    }
                }
            }

            XmlNode::Text(text) => {
                // Strip LDOCE5 inline audio-cue markers (¿[Play], ¿[British], …)
                let filtered = strip_audio_markers(text);
                let text = filtered.as_ref();

                // Find the innermost non-skip style
                let style = stack
                    .iter()
                    .rev()
                    .find(|(t, _, _)| !skip_tags.contains(t.as_str()))
                    .map(|(_, s, _)| *s)
                    .unwrap_or_default();

                // If the last inline is a Link with empty text, fill it
                if let Some(Inline::Link { text: lt, .. }) = current_block.inlines.last_mut() {
                    if lt.is_empty() {
                        *lt = text.to_owned();
                        continue;
                    }
                }

                // Treat text as a headword only when directly inside HWD/BASE
                // and NOT inside an INFLX subtree (which contains inflected forms
                // we do not want merged into the entry title).
                let inside_inflx = stack
                    .iter()
                    .rev()
                    .any(|(t, _, _)| t == "INFLX" || t == "SE_EntryAssets");
                let is_headword = !inside_inflx
                    && stack
                        .iter()
                        .rev()
                        .any(|(t, _, _)| t == "HWD" || t == "BASE");

                // Check if we're inside a sensenum span
                let is_sensenum = stack.iter().rev().any(|(t, _, _)| t == "sensenum");

                // Check for frequency/corpus badge (FREQ, AC elements)
                let is_freq = stack.iter().rev().any(|(t, _, _)| t == "FREQ" || t == "AC");

                // Check for signpost element
                let is_signpost = stack.iter().rev().any(|(t, _, _)| t == "SIGNPOST");

                // Check for section heading (HEADING element inside a box)
                let is_heading_elem = stack.iter().rev().any(|(t, _, _)| t == "HEADING");

                if is_headword {
                    current_block.push_headword(text);
                } else if is_sensenum {
                    // Convert plain number to circled unicode character
                    let circled = to_circled_number(text);
                    // Store circled number without extra surrounding spaces; renderer will
                    // place it in the fixed prefix column.
                    current_block.push_text(&format!("{}", circled.trim()), style);
                } else if is_freq {
                    // Frequency/corpus badge: render as [S1], [W2], [AC] etc.
                    current_block.push_badge(text);
                } else if is_signpost {
                    // Signpost label: render with visual boxing
                    current_block.push_signpost(text);
                } else if is_heading_elem {
                    // Section box heading (COLLOCATIONS, THESAURUS, …)
                    current_block.is_heading = true;
                    current_block.push_text(text, style_heading());
                } else {
                    current_block.push_text(text, style);
                }
            }
        }
    }
    flush(&mut page, &mut current_block);

    // Don't perform heuristic splitting here. Use the XML tags (EXAMPLE,
    // ColloExa, GramExa, etc.) and the span.exabullet handling above to create
    // separate blocks when the source XML intends them. This avoids splitting
    // valid sentences like "The U.S. Constitution".

    // Post-process only: convert leading circled numbers or bullets into
    // `Inline::Prefix` so the renderer can place them in a prefix column.
    fn is_circled(c: char) -> bool {
        let u = c as u32;
        (0x2460..=0x2473).contains(&u)
    }
    fn is_bullet(c: char) -> bool {
        matches!(c, '•' | '●' | '\u{2022}' | '\u{25AA}' | '-' | '–' | '—')
    }

    for b in page.iter_mut() {
        if b.inlines.is_empty() {
            continue;
        }
        // Look at first inline; if it begins with a circled digit or bullet,
        // split it out as a Prefix inline.
        if let Inline::Text(s, st) = &mut b.inlines[0] {
            // find first non-whitespace char index (byte index)
            let mut byte_idx = None;
            for (i, ch) in s.char_indices() {
                if !ch.is_whitespace() {
                    byte_idx = Some((i, ch));
                    break;
                }
            }
            if let Some((i, ch)) = byte_idx {
                if is_circled(ch) {
                    // prefix is the circled character (store without trailing space)
                    let prefix = ch.to_string();
                    // compute rest of string after the circled char
                    let next = i + ch.len_utf8();
                    let rest = s[next..].trim_start().to_string();
                    if rest.is_empty() {
                        b.inlines.remove(0);
                    } else {
                        *s = rest;
                    }
                    // insert prefix with a sensible style
                    b.inlines.insert(0, Inline::Prefix(prefix, style_pos()));
                } else if is_bullet(ch) {
                    // replace bullet with a triangular play bullet used on the site
                    let prefix = "▶".to_string();
                    // compute rest
                    let next = i + ch.len_utf8();
                    let rest = s[next..].trim_start().to_string();
                    if rest.is_empty() {
                        b.inlines.remove(0);
                    } else {
                        *s = rest;
                    }
                    b.inlines.insert(0, Inline::Prefix(prefix, style_dim()));
                }
            }
        }
    }

    // Return the page as-is; do not heuristically split.
    page
}

fn style_for_tag(tag: &str, attrs: &[(String, String)]) -> Style {
    match tag {
        "HWD" | "BASE" => style_headword(),
        "POS" => style_pos(),
        "DEF" => style_def(),
        "EXAMPLE" | "GramExa" | "ColloExa" => style_example(),
        "Ref" | "NonDV" => style_ref(),
        "FIELD" | "REGISTERLAB" | "ACTIV" => style_label(),
        // Frequency badges (S1, W1, etc.) — bright green bold so they stand out
        "FREQ" => Style::default()
            .fg(rgb(162, 255, 153))
            .add_modifier(Modifier::BOLD),
        // Grammar labels like [countable] — use yellow
        "GRAM" => Style::default().fg(rgb(255, 255, 128)),
        // Pronunciation text — yellow so it's distinct from definition text
        "PRON" => Style::default().fg(rgb(255, 255, 128)),
        // Main section heading (COLLOCATIONS, THESAURUS, …) — bright green bold
        "HEADING" => Style::default()
            .fg(rgb(162, 255, 153))
            .add_modifier(Modifier::BOLD),
        "SECHEADING" => style_heading(),
        // Signpost labels in entries (e.g. "■ CAR JOURNEY")
        "SIGNPOST" => Style::default()
            .fg(rgb(255, 255, 128))
            .add_modifier(Modifier::BOLD),
        // Collocation-specific tags
        "coll-head" => Style::default().add_modifier(Modifier::BOLD),
        "coll-note" => Style::default().fg(rgb(121, 112, 169)),
        // COLLO marks the specific collocating word inside an example
        "COLLO" => Style::default().add_modifier(Modifier::BOLD),
        "span" => match attr_get(attrs, "class").as_deref() {
            Some("sensenum") => Style::default()
                .fg(rgb(255, 255, 128))
                .add_modifier(Modifier::BOLD),
            Some("heading") => style_heading(),
            Some("def") => style_def(),
            Some("exabullet") => style_dim(),
            _ => style_default(),
        },
        _ => style_default(),
    }
}

// --------------------------------------------------------------------------
// Thesaurus transformer
// --------------------------------------------------------------------------

pub fn transform_thesaurus(xml_chunks: &[&[u8]]) -> ContentPage {
    let mut page = Vec::new();
    for xml in xml_chunks {
        let nodes = parse_xml(xml);
        // Extract SECHEADING and Exponent/exp-head/EXP text
        let mut in_secheading = false;
        let mut in_exp_head = false;
        let mut current_heading = String::new();

        for node in &nodes {
            match node {
                XmlNode::Open { tag, .. } => match tag.as_str() {
                    "SECHEADING" => {
                        in_secheading = true;
                        current_heading.clear();
                    }
                    "exp-head" => {
                        in_exp_head = true;
                    }
                    _ => {}
                },
                XmlNode::Close { tag } => match tag.as_str() {
                    "SECHEADING" => {
                        in_secheading = false;
                        let mut b = Block::new(0);
                        b.push_text(&current_heading, style_heading());
                        page.push(b);
                    }
                    "exp-head" => {
                        in_exp_head = false;
                    }
                    _ => {}
                },
                XmlNode::Text(t) => {
                    if in_secheading {
                        current_heading.push_str(t);
                    } else if in_exp_head {
                        let mut b = Block::new(1);
                        b.push_text(t, style_default());
                        page.push(b);
                    }
                }
            }
        }
    }
    page
}

// --------------------------------------------------------------------------
// Collocations transformer
// --------------------------------------------------------------------------

pub fn transform_collocations(xml: &[u8]) -> ContentPage {
    let mut page = Vec::new();
    let nodes = parse_xml(xml);
    let mut heading_depth = 0u32;
    let mut current_heading = String::new();
    let mut in_heading = false;

    for node in &nodes {
        match node {
            XmlNode::Open { tag, .. } => match tag.as_str() {
                "HEADING" => {
                    heading_depth = 1;
                    in_heading = true;
                    current_heading.clear();
                }
                "SECHEADING" => {
                    heading_depth = 2;
                    in_heading = true;
                    current_heading.clear();
                }
                "coll-head" => {
                    in_heading = true;
                    current_heading.clear();
                }
                _ => {}
            },
            XmlNode::Close { tag } => match tag.as_str() {
                "HEADING" | "SECHEADING" | "coll-head" => {
                    in_heading = false;
                    let style = if heading_depth <= 1 {
                        style_heading()
                    } else {
                        style_pos()
                    };
                    let mut b = Block::new((heading_depth.saturating_sub(1)) as u8);
                    b.push_text(&current_heading, style);
                    page.push(b);
                    heading_depth = 0;
                }
                _ => {}
            },
            XmlNode::Text(t) => {
                if in_heading {
                    current_heading.push_str(t);
                }
            }
        }
    }
    page
}

// --------------------------------------------------------------------------
// Word Families transformer
// --------------------------------------------------------------------------

pub fn transform_word_families(xml: &[u8]) -> ContentPage {
    let mut page = Vec::new();
    let nodes = parse_xml(xml);
    let mut in_pos = false;
    let mut pos_text = String::new();
    let mut in_ref_hwd = false;
    let mut ref_hwd = String::new();
    let mut group_block: Option<Block> = None;

    for node in &nodes {
        match node {
            XmlNode::Open { tag, .. } => match tag.as_str() {
                "group" => {
                    group_block = Some(Block::new(0));
                }
                "pos" => {
                    in_pos = true;
                    pos_text.clear();
                }
                "Ref" => {
                    in_ref_hwd = true;
                    ref_hwd.clear();
                }
                _ => {}
            },
            XmlNode::Close { tag } => match tag.as_str() {
                "group" => {
                    if let Some(b) = group_block.take() {
                        if !b.inlines.is_empty() {
                            page.push(b);
                        }
                    }
                }
                "pos" => {
                    if let Some(b) = &mut group_block {
                        b.push_text(&pos_text, style_heading());
                        b.push_text(" ", style_default());
                    }
                    in_pos = false;
                }
                "Ref" => {
                    if let Some(b) = &mut group_block {
                        b.push_text(&ref_hwd, style_ref());
                        b.push_text("  ", style_default());
                    }
                    in_ref_hwd = false;
                }
                _ => {}
            },
            XmlNode::Text(t) => {
                if in_pos {
                    pos_text.push_str(t);
                } else if in_ref_hwd {
                    ref_hwd.push_str(t);
                }
            }
        }
    }
    page
}

// --------------------------------------------------------------------------
// Phrases transformer
// --------------------------------------------------------------------------

pub fn transform_phrases(xml: &[u8]) -> ContentPage {
    let mut page = Vec::new();
    let nodes = parse_xml(xml);
    let mut in_ref_text = false;
    let mut ref_text = String::new();
    let mut in_exa = false;
    let mut exa_text = String::new();

    for node in &nodes {
        match node {
            XmlNode::Open { tag, attrs } => match tag.as_str() {
                "Ref" => {
                    in_ref_text = true;
                    ref_text.clear();
                }
                "exa" => {
                    in_exa = true;
                    exa_text.clear();
                }
                _ => {}
            },
            XmlNode::Close { tag } => match tag.as_str() {
                "Ref" => {
                    in_ref_text = false;
                    let mut b = Block::new(0);
                    b.push_text(&ref_text, style_heading());
                    page.push(b);
                }
                "exa" => {
                    in_exa = false;
                    let mut b = Block::new(1);
                    b.push_text("• ", style_dim());
                    b.push_text(&exa_text, style_example());
                    page.push(b);
                }
                _ => {}
            },
            XmlNode::Text(t) => {
                if in_ref_text {
                    ref_text.push_str(t);
                } else if in_exa {
                    exa_text.push_str(t);
                }
            }
        }
    }
    page
}

// --------------------------------------------------------------------------
// Examples transformer
// --------------------------------------------------------------------------

pub fn transform_examples(xml: &[u8]) -> ContentPage {
    let mut page = Vec::new();
    let nodes = parse_xml(xml);
    let mut in_hwd = false;
    let mut in_pos = false;
    let mut in_exa = false;
    let mut hwd_text = String::new();
    let mut pos_text = String::new();
    let mut exa_text = String::new();

    for node in &nodes {
        match node {
            XmlNode::Open { tag, .. } => match tag.as_str() {
                "hwd" => {
                    in_hwd = true;
                    hwd_text.clear();
                }
                "pos" => {
                    in_pos = true;
                    pos_text.clear();
                }
                "exa" => {
                    in_exa = true;
                    exa_text.clear();
                }
                _ => {}
            },
            XmlNode::Close { tag } => match tag.as_str() {
                "hwd" => {
                    in_hwd = false;
                }
                "pos" => {
                    in_pos = false;
                    let mut b = Block::new(0);
                    b.push_text(&hwd_text, style_headword());
                    b.push_text(" ", style_default());
                    b.push_text(&pos_text, style_pos());
                    page.push(b);
                }
                "exa" => {
                    in_exa = false;
                    let mut b = Block::new(1);
                    b.push_text("• ", style_dim());
                    b.push_text(&exa_text, style_example());
                    page.push(b);
                }
                _ => {}
            },
            XmlNode::Text(t) => {
                if in_hwd {
                    hwd_text.push_str(t);
                } else if in_pos {
                    pos_text.push_str(t);
                } else if in_exa {
                    exa_text.push_str(t);
                }
            }
        }
    }
    page
}

// --------------------------------------------------------------------------
// Etymologies transformer
// --------------------------------------------------------------------------

pub fn transform_etymologies(xml: &[u8]) -> ContentPage {
    let mut page = Vec::new();
    let mut b = Block::new(0);
    // Just collect all text
    for node in &parse_xml(xml) {
        if let XmlNode::Text(t) = node {
            b.push_text(t, style_default());
        }
    }
    if !b.inlines.is_empty() {
        page.push(b);
    }
    page
}

// --------------------------------------------------------------------------
// Word Sets transformer
// --------------------------------------------------------------------------

pub fn transform_word_sets(xml_chunks: &[&[u8]]) -> ContentPage {
    let mut page = Vec::new();
    for xml in xml_chunks {
        let nodes = parse_xml(xml);
        let mut in_name = false;
        let mut in_number = false;
        let mut in_hwd = false;
        let mut in_pos = false;
        let mut name_text = String::new();
        let mut number_text = String::new();
        let mut hwd_text = String::new();
        let mut pos_text = String::new();

        for node in &nodes {
            match node {
                XmlNode::Open { tag, .. } => match tag.as_str() {
                    "name" => {
                        in_name = true;
                        name_text.clear();
                    }
                    "number" => {
                        in_number = true;
                        number_text.clear();
                    }
                    "hwd" => {
                        in_hwd = true;
                        hwd_text.clear();
                    }
                    "pos" => {
                        in_pos = true;
                        pos_text.clear();
                    }
                    _ => {}
                },
                XmlNode::Close { tag } => match tag.as_str() {
                    "number" => {
                        in_number = false;
                        let mut b = Block::new(0);
                        b.push_text(&name_text, style_heading());
                        b.push_text(" (", style_default());
                        b.push_text(&number_text, style_pos());
                        b.push_text(")", style_default());
                        page.push(b);
                    }
                    "name" => {
                        in_name = false;
                    }
                    "pos" => {
                        in_pos = false;
                        let mut b = Block::new(1);
                        b.push_text(&hwd_text, style_ref());
                        b.push_text(" ", style_default());
                        b.push_text(&pos_text, style_pos());
                        page.push(b);
                    }
                    "hwd" => {
                        in_hwd = false;
                    }
                    _ => {}
                },
                XmlNode::Text(t) => {
                    if in_name {
                        name_text.push_str(t);
                    }
                    if in_number {
                        number_text.push_str(t);
                    }
                    if in_hwd {
                        hwd_text.push_str(t);
                    }
                    if in_pos {
                        pos_text.push_str(t);
                    }
                }
            }
        }
    }
    page
}

// --------------------------------------------------------------------------
// Activator transformer (two-pane: concept + section)
// --------------------------------------------------------------------------

pub fn transform_activator(concept_xml: &[u8], section_xml: &[u8], _sid: &str) -> ContentPage {
    let mut page = Vec::new();

    // Concept pane (left/top)
    let concept_nodes = parse_xml(concept_xml);
    let mut in_hwd = false;
    let mut in_section = false;
    let mut section_text = String::new();
    let mut hwd_text = String::new();

    for node in &concept_nodes {
        match node {
            XmlNode::Open { tag, .. } => match tag.as_str() {
                "HWD" => {
                    in_hwd = true;
                    hwd_text.clear();
                }
                "Section" => {
                    in_section = true;
                    section_text.clear();
                }
                _ => {}
            },
            XmlNode::Close { tag } => match tag.as_str() {
                "HWD" => {
                    in_hwd = false;
                    let mut b = Block::new(0);
                    b.push_text(&hwd_text, style_heading());
                    page.push(b);
                }
                "Section" => {
                    in_section = false;
                    let mut b = Block::new(1);
                    b.push_text("▶ ", style_dim());
                    b.push_text(&section_text, style_ref());
                    page.push(b);
                }
                _ => {}
            },
            XmlNode::Text(t) => {
                if in_hwd {
                    hwd_text.push_str(t);
                }
                if in_section {
                    section_text.push_str(t);
                }
            }
        }
    }

    // Section pane (right/bottom)
    let section_nodes = parse_xml(section_xml);
    let mut in_secdef = false;
    let mut secdef_text = String::new();

    for node in &section_nodes {
        match node {
            XmlNode::Open { tag, .. } => match tag.as_str() {
                "SECDEF" => {
                    in_secdef = true;
                    secdef_text.clear();
                }
                _ => {}
            },
            XmlNode::Close { tag } => match tag.as_str() {
                "SECDEF" => {
                    in_secdef = false;
                    let mut b = Block::new(0);
                    b.push_text(&secdef_text, style_heading());
                    page.push(b);
                }
                _ => {}
            },
            XmlNode::Text(t) => {
                if in_secdef {
                    secdef_text.push_str(t);
                }
            }
        }
    }

    // Walk the section XML for Exponent content (same as entry walk)
    page.extend(transform_entry(section_xml));
    page
}

// --------------------------------------------------------------------------
// Dispatch
// --------------------------------------------------------------------------

/// Transform XML bytes for a given `ContentType` into a `ContentPage`.
pub fn transform(content_type: ContentType, xml: &[u8]) -> ContentPage {
    match content_type {
        ContentType::Entry => transform_entry(xml),
        ContentType::Etymologies => transform_etymologies(xml),
        ContentType::Phrases => transform_phrases(xml),
        ContentType::Examples => transform_examples(xml),
        _ => {
            // Fallback: just show all text
            let mut b = Block::new(0);
            for node in &parse_xml(xml) {
                if let XmlNode::Text(t) = node {
                    b.push_text(t, style_default());
                }
            }
            vec![b]
        }
    }
}

// --------------------------------------------------------------------------
// Tests
// --------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn entry_xml(hwd: &str, pos: &str, def: &str, example: &str) -> Vec<u8> {
        format!(
            r#"<Entry>
  <Head>
    <HWD><BASE>{hwd}</BASE></HWD>
    <POS>{pos}</POS>
    <Audio resource="GB_HWD_PRON" topic="gb_hwd_pron/{hwd}.mp3"/>
    <Audio resource="US_HWD_PRON" topic="us_hwd_pron/{hwd}.mp3"/>
  </Head>
  <Sense>
    <DEF>{def}</DEF>
    <EXAMPLE><span class="exabullet">●</span>{example}</EXAMPLE>
  </Sense>
</Entry>"#
        )
        .into_bytes()
    }

    #[test]
    fn test_transform_entry_produces_blocks() {
        let xml = entry_xml(
            "run",
            "verb",
            "to move quickly on foot",
            "She ran to the door.",
        );
        let page = transform_entry(&xml);
        assert!(!page.is_empty(), "page should not be empty");
    }

    #[test]
    fn test_transform_entry_headword_present() {
        let xml = entry_xml("run", "verb", "to move quickly", "He runs daily.");
        let page = transform_entry(&xml);
        let all_text: String = page
            .iter()
            .flat_map(|b| b.inlines.iter())
            .filter_map(|i| match i {
                Inline::Text(t, _) => Some(t.as_str()),
                Inline::Headword(t) => Some(t.as_str()),
                _ => None,
            })
            .collect();
        assert!(all_text.contains("run"), "headword 'run' not found");
    }

    #[test]
    fn test_transform_entry_audio_buttons() {
        let xml = entry_xml(
            "able",
            "adjective",
            "having the skill",
            "She is able to swim.",
        );
        let page = transform_entry(&xml);
        let audio_count = page
            .iter()
            .flat_map(|b| b.inlines.iter())
            .filter(|i| matches!(i, Inline::AudioButton { .. }))
            .count();
        assert!(
            audio_count >= 2,
            "expected at least 2 audio buttons, got {audio_count}"
        );
    }

    #[test]
    fn test_transform_entry_example_text() {
        let xml = entry_xml("walk", "verb", "to move on foot", "She walked to school.");
        let page = transform_entry(&xml);
        let all_text: String = page
            .iter()
            .flat_map(|b| b.inlines.iter())
            .filter_map(|i| match i {
                Inline::Text(t, _) => Some(t.as_str()),
                Inline::Headword(t) => Some(t.as_str()),
                _ => None,
            })
            .collect();
        assert!(all_text.contains("walked"), "example text not found");
    }

    #[test]
    fn test_to_ratatui_text_non_empty() {
        let xml = entry_xml("test", "noun", "a procedure for evaluation", "Run a test.");
        let page = transform_entry(&xml);
        let text = to_ratatui_text(&page);
        assert!(!text.lines.is_empty());
    }

    #[test]
    fn test_transform_examples() {
        let xml = br#"<exa-root>
          <exa-head><hwd>run</hwd><pos>verb</pos></exa-head>
          <exa-body>
            <exa>She ran fast.</exa>
            <exa>He ran away.</exa>
          </exa-body>
        </exa-root>"#;
        let page = transform_examples(xml);
        let all_text: String = page
            .iter()
            .flat_map(|b| b.inlines.iter())
            .filter_map(|i| match i {
                Inline::Text(t, _) => Some(t.as_str()),
                Inline::Headword(t) => Some(t.as_str()),
                _ => None,
            })
            .collect();
        assert!(all_text.contains("run"));
        assert!(all_text.contains("ran"));
    }

    #[test]
    fn test_transform_etymologies() {
        let xml = b"<etym>From Latin <i>currere</i>, to run.</etym>";
        let page = transform_etymologies(xml);
        let all_text: String = page
            .iter()
            .flat_map(|b| b.inlines.iter())
            .filter_map(|i| match i {
                Inline::Text(t, _) => Some(t.as_str()),
                Inline::Headword(t) => Some(t.as_str()),
                _ => None,
            })
            .collect();
        assert!(all_text.contains("Latin"));
    }

    #[test]
    fn test_transform_phrases() {
        let xml = br#"<phrases>
          <phrase>
            <phrase-head><Ref topic="fs/1.2.3">at a run</Ref></phrase-head>
            <phrase-body>
              <exa>She left at a run.</exa>
            </phrase-body>
          </phrase>
        </phrases>"#;
        let page = transform_phrases(xml);
        assert!(!page.is_empty());
    }

    #[test]
    fn test_style_for_headword_tag() {
        let s = style_for_tag("HWD", &[]);
        assert_eq!(s, style_headword());
    }

    #[test]
    fn test_empty_xml() {
        let page = transform_entry(b"");
        // Should not panic; may return an empty page
        let _ = page;
    }

    #[test]
    fn test_freq_badge_rendering() {
        // FREQ element should produce a Badge inline
        let xml = br#"<Entry id="e.1.1.1">
          <Head><HWD><BASE>car</BASE></HWD><FREQ>S1</FREQ><FREQ>W1</FREQ></Head>
          <Sense id="s.1.1.1.1"><DEF>a road vehicle with an engine</DEF></Sense>
        </Entry>"#;
        let page = transform_entry(xml);
        let badges: Vec<&str> = page
            .iter()
            .flat_map(|b| b.inlines.iter())
            .filter_map(|i| match i {
                Inline::Badge { text } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert!(
            !badges.is_empty(),
            "expected Badge inlines for FREQ elements, got none"
        );
        assert!(
            badges.iter().any(|b| *b == "S1"),
            "expected badge 'S1', got {:?}",
            badges
        );
        assert!(
            badges.iter().any(|b| *b == "W1"),
            "expected badge 'W1', got {:?}",
            badges
        );
    }

    #[test]
    fn test_badge_ratatui_rendering() {
        // A page with a Badge should render as text containing '[S1]'
        let xml = br#"<Entry id="e.1.1.1">
          <Head><HWD><BASE>car</BASE></HWD><FREQ>S1</FREQ></Head>
          <Sense id="s.1.1.1.1"><DEF>a road vehicle</DEF></Sense>
        </Entry>"#;
        let page = transform_entry(xml);
        let text = to_ratatui_text(&page);
        let all: String = text
            .lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.as_ref())
            .collect();
        assert!(all.contains("[S1]"), "rendered text should contain '[S1]'");
    }

    #[test]
    fn test_signpost_rendering() {
        // SIGNPOST element should produce a Signpost inline with visual boxing
        let xml = br#"<Entry id="e.1.1.1">
          <Head><HWD><BASE>car</BASE></HWD></Head>
          <Sense id="s.1.1.1.1">
            <SIGNPOST>DRIVING</SIGNPOST>
            <DEF>to operate a car</DEF>
          </Sense>
        </Entry>"#;
        let page = transform_entry(xml);
        let signposts: Vec<&str> = page
            .iter()
            .flat_map(|b| b.inlines.iter())
            .filter_map(|i| match i {
                Inline::Signpost { text } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert!(
            !signposts.is_empty(),
            "expected Signpost inlines for SIGNPOST elements"
        );
        assert!(
            signposts.iter().any(|s| s.contains("DRIVING")),
            "expected signpost containing 'DRIVING', got {:?}",
            signposts
        );
    }

    #[test]
    fn test_section_heading_rendering() {
        // HEADING element inside a ColloBox should produce an is_heading block
        let xml = br#"<Entry id="e.1.1.1">
          <Head><HWD><BASE>car</BASE></HWD></Head>
          <ColloBox><HEADING>COLLOCATIONS</HEADING>
            <ColloGram><COLLOC id="c.1.1.1.1">drive a car</COLLOC></ColloGram>
          </ColloBox>
        </Entry>"#;
        let page = transform_entry(xml);
        let heading_blocks: Vec<_> = page.iter().filter(|b| b.is_heading).collect();
        assert!(
            !heading_blocks.is_empty(),
            "expected at least one is_heading block for HEADING element"
        );
        // The rendered output should contain the ╔═ heading bar
        let text = to_ratatui_text(&page);
        let all: String = text
            .lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.as_ref())
            .collect();
        assert!(
            all.contains("COLLOCATIONS"),
            "rendered heading bar should contain section title"
        );
    }
}
