//! Integration tests for the XML → ratatui content transformer.

use ldoce5viewer_tui::content::{
    transform::{to_ratatui_text, transform_entry, transform_examples, transform_etymologies, transform_phrases, Block, Inline},
    types::{ContentId, ContentType},
};

// --------------------------------------------------------------------------
// Helpers
// --------------------------------------------------------------------------

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

fn all_text(page: &[Block]) -> String {
    page.iter()
        .flat_map(|b| b.inlines.iter())
        .filter_map(|i| if let Inline::Text(t, _) = i { Some(t.as_str()) } else { None })
        .collect()
}

fn count_audio(page: &[Block]) -> usize {
    page.iter()
        .flat_map(|b| b.inlines.iter())
        .filter(|i| matches!(i, Inline::AudioButton { .. }))
        .count()
}

// --------------------------------------------------------------------------
// Entry transformer tests
// --------------------------------------------------------------------------

#[test]
fn test_entry_headword_found() {
    let xml = entry_xml("run", "verb", "to move quickly", "She ran every day.");
    let page = transform_entry(&xml);
    assert!(all_text(&page).contains("run"), "headword 'run' missing from page");
}

#[test]
fn test_entry_pos_found() {
    let xml = entry_xml("happy", "adjective", "feeling glad", "I am very happy.");
    let page = transform_entry(&xml);
    assert!(all_text(&page).contains("adjective"), "POS missing");
}

#[test]
fn test_entry_definition_found() {
    let xml = entry_xml("walk", "verb", "to move on foot at a normal speed", "I walked to school.");
    let page = transform_entry(&xml);
    assert!(all_text(&page).contains("move on foot"), "definition text missing");
}

#[test]
fn test_entry_example_found() {
    let xml = entry_xml("jump", "verb", "to push up into the air", "She jumped over the puddle.");
    let page = transform_entry(&xml);
    assert!(all_text(&page).contains("puddle"), "example sentence missing");
}

#[test]
fn test_entry_audio_buttons_gb_us() {
    let xml = entry_xml("able", "adjective", "having the skill", "She is able to help.");
    let page = transform_entry(&xml);
    assert!(count_audio(&page) >= 2, "expected ≥2 audio buttons (GB + US)");
}

#[test]
fn test_entry_non_empty() {
    let xml = entry_xml("test", "noun", "a procedure", "Run the test.");
    let page = transform_entry(&xml);
    assert!(!page.is_empty());
}

#[test]
fn test_entry_empty_xml_does_not_panic() {
    let page = transform_entry(b"");
    let _ = page; // must not panic
}

#[test]
fn test_entry_illustration_placeholder() {
    let xml = b"<Entry><ILLUSTRATION thumb='pictures/apple.jpg'/></Entry>";
    let page = transform_entry(xml);
    let has_img = page.iter()
        .flat_map(|b| b.inlines.iter())
        .any(|i| matches!(i, Inline::Image { .. }));
    assert!(has_img, "image inline expected");
}

#[test]
fn test_entry_link_target() {
    let xml = br#"<Entry><Sense><Ref topic="fs/1.2.3">See also</Ref></Sense></Entry>"#;
    let page = transform_entry(xml);
    let has_link = page.iter()
        .flat_map(|b| b.inlines.iter())
        .any(|i| matches!(i, Inline::Link { target, .. } if target.contains("1.2.3")));
    assert!(has_link, "link with correct target expected");
}

// --------------------------------------------------------------------------
// Examples transformer tests
// --------------------------------------------------------------------------

#[test]
fn test_examples_headword_present() {
    let xml = br#"<exa-root>
        <exa-head><hwd>swim</hwd><pos>verb</pos></exa-head>
        <exa-body><exa>She swam every morning.</exa></exa-body>
    </exa-root>"#;
    let page = transform_examples(xml);
    assert!(all_text(&page).contains("swim"), "headword missing");
    assert!(all_text(&page).contains("swam"), "example missing");
}

// --------------------------------------------------------------------------
// Etymologies transformer tests
// --------------------------------------------------------------------------

#[test]
fn test_etymologies_text_present() {
    let xml = b"<etym>From Old English <i>rinnand</i>.</etym>";
    let page = transform_etymologies(xml);
    assert!(all_text(&page).contains("Old English"), "etymology text missing");
}

// --------------------------------------------------------------------------
// Phrases transformer tests
// --------------------------------------------------------------------------

#[test]
fn test_phrases_produces_blocks() {
    let xml = br#"<phrases>
        <phrase>
            <phrase-head><Ref topic="fs/1.2.3">at a run</Ref></phrase-head>
            <phrase-body><exa>She left at a run.</exa></phrase-body>
        </phrase>
    </phrases>"#;
    let page = transform_phrases(xml);
    assert!(!page.is_empty(), "phrases page should have content");
}

// --------------------------------------------------------------------------
// to_ratatui_text
// --------------------------------------------------------------------------

#[test]
fn test_to_ratatui_text_line_count() {
    let xml = entry_xml("cat", "noun", "a small furry animal", "The cat sat on the mat.");
    let page = transform_entry(&xml);
    let text = to_ratatui_text(&page);
    assert!(!text.lines.is_empty(), "ratatui Text must have at least one line");
}

#[test]
fn test_to_ratatui_text_empty_page() {
    let text = to_ratatui_text(&[]);
    assert!(text.lines.is_empty());
}

// --------------------------------------------------------------------------
// ContentId
// --------------------------------------------------------------------------

#[test]
fn test_content_id_entry() {
    let cid = ContentId::from_path("/fs/3.4.6.2").unwrap();
    assert_eq!(cid.content_type, ContentType::Entry);
    assert_eq!(cid.id, "3.4.6.2");
    assert!(cid.anchor.is_none());
}

#[test]
fn test_content_id_anchor() {
    let cid = ContentId::from_path("/fs/3.4.6.2#s2").unwrap();
    assert_eq!(cid.anchor, Some("s2".to_owned()));
}

#[test]
fn test_content_id_audio() {
    let cid = ContentId::from_path("/gb_hwd_pron/run_g.mp3").unwrap();
    assert_eq!(cid.content_type, ContentType::AudioPronunciation);
    assert_eq!(cid.id, "run_g.mp3");
}

#[test]
fn test_content_id_unknown_returns_none() {
    assert!(ContentId::from_path("/unknown/abc").is_none());
}
