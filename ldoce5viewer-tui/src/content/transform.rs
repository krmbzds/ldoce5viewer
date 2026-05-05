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
    /// An audio playback button:  `♪ <title>`.
    AudioButton { path: String, title: String },
    /// An image placeholder.
    Image { filename: String },
    /// A cross-reference / link.
    Link { text: String, target: String },
    /// A line break within a block.
    LineBreak,
}

/// A block of content (analogous to an HTML `<div>` / `<p>`).
#[derive(Debug, Clone)]
pub struct Block {
    /// Indentation level (each level = 2 spaces).
    pub indent: u8,
    /// Inlines that make up this block's content.
    pub inlines: Vec<Inline>,
}

impl Block {
    fn new(indent: u8) -> Self {
        Block { indent, inlines: Vec::new() }
    }

    fn push_text(&mut self, text: &str, style: Style) {
        if text.is_empty() { return; }
        // If the last inline is text with the same style, append to it (inserting a
        // space when needed).
        if let Some(last) = self.inlines.last_mut() {
            if let Inline::Text(last_text, last_style) = last {
                if *last_style == style {
                    let need_space = last_text.chars().rev().next().map(|c| c.is_alphanumeric()).unwrap_or(false)
                        && text.chars().next().map(|c| c.is_alphanumeric()).unwrap_or(false);
                    if need_space { last_text.push(' '); }
                    last_text.push_str(text);
                    return;
                }
            }
        }
        self.inlines.push(Inline::Text(text.to_owned(), style));
    }

    fn push_headword(&mut self, text: &str) {
        if text.is_empty() { return; }
        if let Some(last) = self.inlines.last_mut() {
            if let Inline::Headword(last_text) = last {
                // choose to insert a space when joining two alphanumeric tokens
                let need_space = last_text.chars().rev().next().map(|c| c.is_alphanumeric()).unwrap_or(false)
                    && text.chars().next().map(|c| c.is_alphanumeric()).unwrap_or(false);
                if need_space { last_text.push(' '); }
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
// Style constants
// --------------------------------------------------------------------------

fn style_default() -> Style { Style::default() }
fn style_headword() -> Style { Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD) }
fn style_pos()      -> Style { Style::default().fg(Color::Yellow) }
fn style_def()      -> Style { Style::default() }
fn style_example()  -> Style { Style::default().fg(Color::Green).add_modifier(Modifier::ITALIC) }
fn style_ref()      -> Style { Style::default().fg(Color::Blue).add_modifier(Modifier::UNDERLINED) }
fn style_label()    -> Style { Style::default().fg(Color::Magenta) }
fn style_heading()  -> Style { Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD | Modifier::UNDERLINED) }
fn style_audio()    -> Style { Style::default().fg(Color::Cyan) }
fn style_dim()      -> Style { Style::default().add_modifier(Modifier::DIM) }

// --------------------------------------------------------------------------
// to_ratatui_text  (ContentPage → ratatui Text)
// --------------------------------------------------------------------------

/// Convert a `ContentPage` into a ratatui `Text` object.
pub fn to_ratatui_text(page: &[Block]) -> Text<'static> {
    let lines: Vec<Line> = page.iter().flat_map(|block| {
        // Expand LineBreak inlines into multiple lines
        let mut current: Vec<Span> = Vec::new();
        if block.indent > 0 {
            current.push(Span::raw(" ".repeat(block.indent as usize * 2)));
        }
        let mut result_lines: Vec<Line> = Vec::new();

        for inline in &block.inlines {
            match inline {
                Inline::Text(text, style) => {
                    current.push(Span::styled(text.clone(), *style));
                }
                Inline::Headword(text) => {
                    current.push(Span::styled(text.clone(), style_headword()));
                }
                Inline::AudioButton { title, .. } => {
                    current.push(Span::styled(format!("♪[{title}]"), style_audio()));
                }
                Inline::Image { .. } => {
                    // Images cannot be rendered in a TUI; skip silently.
                }
                Inline::Link { text, .. } => {
                    current.push(Span::styled(text.clone(), style_ref()));
                }
                Inline::LineBreak => {
                    result_lines.push(Line::from(std::mem::take(&mut current)));
                    if block.indent > 0 {
                        current.push(Span::raw(" ".repeat(block.indent as usize * 2)));
                    }
                }
            }
        }
        if !current.is_empty() {
            result_lines.push(Line::from(current));
        }
        result_lines
    }).collect();

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
    Open  { tag: String, attrs: Vec<(String, String)> },
    Close { tag: String },
    Text  (String),
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
                    let key = String::from_utf8_lossy(attr.key.local_name().into_inner()).into_owned();
                    let val = attr.unescape_value().map(|v| v.into_owned()).unwrap_or_default();
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
                    let key = String::from_utf8_lossy(attr.key.local_name().into_inner()).into_owned();
                    let val = attr.unescape_value().map(|v| v.into_owned()).unwrap_or_default();
                    attrs.push((key, val));
                }
                nodes.push(XmlNode::Open  { tag: tag.clone(), attrs });
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
                    if c == ']' { break; }
                }
            }
        } else {
            result.push(ch);
        }
    }
    std::borrow::Cow::Owned(result)
}

// --------------------------------------------------------------------------
// Entry transformer
// --------------------------------------------------------------------------

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
        "Entry", "Head", "Sense", "Subsense", "EXAMPLE", "GramExa",
        "ColloExa", "Deriv", "RunOn", "PhrVbEntry", "GramBox", "Exponent",
        "Section", "SECHEADING", "SpokenSect", "ThesBox", "ColloBox",
        "F2NBox", "Crossref", "Hint", "ColloGram",
    ].iter().copied().collect();

    let skip_tags: std::collections::HashSet<&str> = [
        "ACTIV", "INFLX", "SE_EntryAssets", "EntryAsset",
    ].iter().copied().collect();

    for node in &nodes {
        match node {
            XmlNode::Open { tag, attrs } => {
                if skip_tags.contains(tag.as_str()) {
                    stack.push((tag.clone(), style_default(), depth));
                    depth += 1;
                    continue;
                }

                let style = style_for_tag(tag, attrs);
                let indent = if block_tags.contains(tag.as_str()) { depth } else { current_block.indent };

                if block_tags.contains(tag.as_str()) {
                    flush(&mut page, &mut current_block);
                    current_block = Block::new(depth.min(6));
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
                            _             => "Play".to_owned(),
                        };
                        current_block.inlines.push(Inline::AudioButton { path, title });
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
                    _ => {}
                }

                stack.push((tag.clone(), style, depth));
                if !block_tags.contains(tag.as_str()) {
                    depth += 1;
                }
            }

            XmlNode::Close { tag } => {
                if let Some(pos) = stack.iter().rposition(|(t, _, _)| t == tag) {
                    let (_, style, d) = stack.remove(pos);
                    depth = d;

                    if block_tags.contains(tag.as_str()) {
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
                let style = stack.iter().rev()
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
                let inside_inflx = stack.iter().rev()
                    .any(|(t, _, _)| t == "INFLX" || t == "SE_EntryAssets");
                let is_headword = !inside_inflx
                    && stack.iter().rev().any(|(t, _, _)| t == "HWD" || t == "BASE");
                if is_headword {
                    current_block.push_headword(text);
                } else {
                    current_block.push_text(text, style);
                }
            }
        }
    }
    flush(&mut page, &mut current_block);
    page
}

fn style_for_tag(tag: &str, attrs: &[(String, String)]) -> Style {
    match tag {
        "HWD" | "BASE"          => style_headword(),
        "POS"                   => style_pos(),
        "DEF"                   => style_def(),
        "EXAMPLE" | "GramExa"
        | "ColloExa"            => style_example(),
        "Ref" | "NonDV"         => style_ref(),
        "FIELD" | "REGISTERLAB"
        | "ACTIV"               => style_label(),
        // Frequency badges (S1, W1, etc.) — bright green bold so they stand out
        "FREQ"                  => Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        // Grammar labels like [countable] — cyan
        "GRAM"                  => Style::default().fg(Color::Cyan),
        // Pronunciation text — yellow so it's distinct from definition text
        "PRON"                  => Style::default().fg(Color::Yellow),
        // Main section heading (COLLOCATIONS, THESAURUS, …) — green bold
        "HEADING"               => Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        "SECHEADING"            => style_heading(),
        // Signpost labels in entries (e.g. "■ CAR JOURNEY")
        "SIGNPOST"              => Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        // Collocation-specific tags
        "coll-head"             => Style::default().add_modifier(Modifier::BOLD),
        "coll-note"             => Style::default().fg(Color::DarkGray),
        // COLLO marks the specific collocating word inside an example
        "COLLO"                 => Style::default().add_modifier(Modifier::BOLD),
        "span" => {
            match attr_get(attrs, "class").as_deref() {
                Some("sensenum")  => Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                Some("heading")   => style_heading(),
                Some("def")       => style_def(),
                Some("exabullet") => style_dim(),
                _                 => style_default(),
            }
        }
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
                    "SECHEADING" => { in_secheading = true; current_heading.clear(); }
                    "exp-head"   => { in_exp_head = true; }
                    _            => {}
                },
                XmlNode::Close { tag } => match tag.as_str() {
                    "SECHEADING" => {
                        in_secheading = false;
                        let mut b = Block::new(0);
                        b.push_text(&current_heading, style_heading());
                        page.push(b);
                    }
                    "exp-head"   => { in_exp_head = false; }
                    _            => {}
                },
                XmlNode::Text(t) => {
                    if in_secheading { current_heading.push_str(t); }
                    else if in_exp_head {
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
                "HEADING"    => { heading_depth = 1; in_heading = true; current_heading.clear(); }
                "SECHEADING" => { heading_depth = 2; in_heading = true; current_heading.clear(); }
                "coll-head"  => { in_heading = true; current_heading.clear(); }
                _            => {}
            },
            XmlNode::Close { tag } => match tag.as_str() {
                "HEADING" | "SECHEADING" | "coll-head" => {
                    in_heading = false;
                    let style = if heading_depth <= 1 { style_heading() } else { style_pos() };
                    let mut b = Block::new((heading_depth.saturating_sub(1)) as u8);
                    b.push_text(&current_heading, style);
                    page.push(b);
                    heading_depth = 0;
                }
                _ => {}
            },
            XmlNode::Text(t) => {
                if in_heading { current_heading.push_str(t); }
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
                "group"    => { group_block = Some(Block::new(0)); }
                "pos"      => { in_pos = true; pos_text.clear(); }
                "Ref"      => { in_ref_hwd = true; ref_hwd.clear(); }
                _          => {}
            },
            XmlNode::Close { tag } => match tag.as_str() {
                "group" => {
                    if let Some(b) = group_block.take() {
                        if !b.inlines.is_empty() { page.push(b); }
                    }
                }
                "pos"   => {
                    if let Some(b) = &mut group_block {
                        b.push_text(&pos_text, style_heading());
                        b.push_text(" ", style_default());
                    }
                    in_pos = false;
                }
                "Ref"   => {
                    if let Some(b) = &mut group_block {
                        b.push_text(&ref_hwd, style_ref());
                        b.push_text("  ", style_default());
                    }
                    in_ref_hwd = false;
                }
                _       => {}
            },
            XmlNode::Text(t) => {
                if in_pos { pos_text.push_str(t); }
                else if in_ref_hwd { ref_hwd.push_str(t); }
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
                "Ref"  => { in_ref_text = true; ref_text.clear(); }
                "exa"  => { in_exa = true; exa_text.clear(); }
                _      => {}
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
                _     => {}
            },
            XmlNode::Text(t) => {
                if in_ref_text { ref_text.push_str(t); }
                else if in_exa { exa_text.push_str(t); }
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
                "hwd" => { in_hwd = true; hwd_text.clear(); }
                "pos" => { in_pos = true; pos_text.clear(); }
                "exa" => { in_exa = true; exa_text.clear(); }
                _     => {}
            },
            XmlNode::Close { tag } => match tag.as_str() {
                "hwd" => { in_hwd = false; }
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
                _     => {}
            },
            XmlNode::Text(t) => {
                if in_hwd { hwd_text.push_str(t); }
                else if in_pos { pos_text.push_str(t); }
                else if in_exa { exa_text.push_str(t); }
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
    if !b.inlines.is_empty() { page.push(b); }
    page
}

// --------------------------------------------------------------------------
// Word Sets transformer
// --------------------------------------------------------------------------

pub fn transform_word_sets(xml_chunks: &[&[u8]]) -> ContentPage {
    let mut page = Vec::new();
    for xml in xml_chunks {
        let nodes = parse_xml(xml);
        let mut in_name   = false;
        let mut in_number = false;
        let mut in_hwd    = false;
        let mut in_pos    = false;
        let mut name_text   = String::new();
        let mut number_text = String::new();
        let mut hwd_text    = String::new();
        let mut pos_text    = String::new();

        for node in &nodes {
            match node {
                XmlNode::Open { tag, .. } => match tag.as_str() {
                    "name"   => { in_name   = true; name_text.clear(); }
                    "number" => { in_number = true; number_text.clear(); }
                    "hwd"    => { in_hwd    = true; hwd_text.clear(); }
                    "pos"    => { in_pos    = true; pos_text.clear(); }
                    _        => {}
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
                    "name"   => { in_name = false; }
                    "pos"    => {
                        in_pos = false;
                        let mut b = Block::new(1);
                        b.push_text(&hwd_text, style_ref());
                        b.push_text(" ", style_default());
                        b.push_text(&pos_text, style_pos());
                        page.push(b);
                    }
                    "hwd"    => { in_hwd = false; }
                    _        => {}
                },
                XmlNode::Text(t) => {
                    if in_name   { name_text.push_str(t); }
                    if in_number { number_text.push_str(t); }
                    if in_hwd    { hwd_text.push_str(t); }
                    if in_pos    { pos_text.push_str(t); }
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
                "HWD"     => { in_hwd = true; hwd_text.clear(); }
                "Section" => { in_section = true; section_text.clear(); }
                _         => {}
            },
            XmlNode::Close { tag } => match tag.as_str() {
                "HWD"     => {
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
                _         => {}
            },
            XmlNode::Text(t) => {
                if in_hwd     { hwd_text.push_str(t); }
                if in_section { section_text.push_str(t); }
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
                "SECDEF" => { in_secdef = true; secdef_text.clear(); }
                _        => {}
            },
            XmlNode::Close { tag } => match tag.as_str() {
                "SECDEF" => {
                    in_secdef = false;
                    let mut b = Block::new(0);
                    b.push_text(&secdef_text, style_heading());
                    page.push(b);
                }
                _        => {}
            },
            XmlNode::Text(t) => {
                if in_secdef { secdef_text.push_str(t); }
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
        ContentType::Entry       => transform_entry(xml),
        ContentType::Etymologies => transform_etymologies(xml),
        ContentType::Phrases     => transform_phrases(xml),
        ContentType::Examples    => transform_examples(xml),
        _                        => {
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
        ).into_bytes()
    }

    #[test]
    fn test_transform_entry_produces_blocks() {
        let xml = entry_xml("run", "verb", "to move quickly on foot", "She ran to the door.");
        let page = transform_entry(&xml);
        assert!(!page.is_empty(), "page should not be empty");
    }

    #[test]
    fn test_transform_entry_headword_present() {
        let xml = entry_xml("run", "verb", "to move quickly", "He runs daily.");
        let page = transform_entry(&xml);
        let all_text: String = page.iter()
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
        let xml = entry_xml("able", "adjective", "having the skill", "She is able to swim.");
        let page = transform_entry(&xml);
        let audio_count = page.iter()
            .flat_map(|b| b.inlines.iter())
            .filter(|i| matches!(i, Inline::AudioButton { .. }))
            .count();
        assert!(audio_count >= 2, "expected at least 2 audio buttons, got {audio_count}");
    }

    #[test]
    fn test_transform_entry_example_text() {
        let xml = entry_xml("walk", "verb", "to move on foot", "She walked to school.");
        let page = transform_entry(&xml);
        let all_text: String = page.iter()
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
        let all_text: String = page.iter()
            .flat_map(|b| b.inlines.iter())
            .filter_map(|i| match i { Inline::Text(t, _) => Some(t.as_str()), Inline::Headword(t) => Some(t.as_str()), _ => None })
            .collect();
        assert!(all_text.contains("run"));
        assert!(all_text.contains("ran"));
    }

    #[test]
    fn test_transform_etymologies() {
        let xml = b"<etym>From Latin <i>currere</i>, to run.</etym>";
        let page = transform_etymologies(xml);
        let all_text: String = page.iter()
            .flat_map(|b| b.inlines.iter())
            .filter_map(|i| match i { Inline::Text(t, _) => Some(t.as_str()), Inline::Headword(t) => Some(t.as_str()), _ => None })
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
}
