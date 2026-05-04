//! Content type definitions.

use std::fmt;

/// All content types understood by the viewer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContentType {
    /// Dictionary entry (the main content type)
    Entry,
    /// Thesaurus page
    Thesaurus,
    /// Collocations box
    Collocations,
    /// Word sets
    WordSets,
    /// Phrase bank
    Phrases,
    /// Example sentences
    Examples,
    /// Word families (morphological family)
    WordFamilies,
    /// Etymology
    Etymologies,
    /// Longman Activator (concept + section, two-pane)
    Activator,
    /// Full-text search results page
    SearchResults,
    /// GB/US pronunciation audio (raw bytes, not rendered as text)
    AudioPronunciation,
    /// Picture / illustration
    Picture,
    /// Sound effect
    Sound,
}

impl fmt::Display for ContentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ContentType::Entry              => "entry",
            ContentType::Thesaurus          => "thesaurus",
            ContentType::Collocations       => "collocations",
            ContentType::WordSets           => "word_sets",
            ContentType::Phrases            => "phrases",
            ContentType::Examples           => "examples",
            ContentType::WordFamilies       => "word_families",
            ContentType::Etymologies        => "etymologies",
            ContentType::Activator          => "activator",
            ContentType::SearchResults      => "search",
            ContentType::AudioPronunciation => "audio",
            ContentType::Picture            => "picture",
            ContentType::Sound              => "sound",
        };
        write!(f, "{s}")
    }
}

/// A parsed content address, mirroring the Python dict:// URL scheme.
///
/// Examples:
/// * `dict:///fs/3.4.6.2`              → Entry
/// * `dict:///thesaurus/3.4.6.2`       → Thesaurus
/// * `dict:///collocations/3.4.6.2`    → Collocations
/// * `audio:///gb_hwd_pron/able_g.mp3` → AudioPronunciation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentId {
    pub content_type: ContentType,
    /// The resource identifier (archive-relative key / filename)
    pub id:           String,
    /// Optional anchor within the page
    pub anchor:       Option<String>,
}

impl ContentId {
    pub fn new(ct: ContentType, id: &str) -> Self {
        ContentId { content_type: ct, id: id.to_owned(), anchor: None }
    }

    pub fn with_anchor(mut self, anchor: &str) -> Self {
        self.anchor = Some(anchor.to_owned());
        self
    }

    /// Parse a `dict://` or `audio://` URL path into a `ContentId`.
    ///
    /// Path format: `/<archive>/<key>`
    pub fn from_path(path: &str) -> Option<Self> {
        let path = path.trim_start_matches('/');
        let (archive, rest) = path.split_once('/')?;
        let (key, anchor) = match rest.split_once('#') {
            Some((k, a)) => (k, Some(a.to_owned())),
            None         => (rest, None),
        };
        let ct = match archive {
            "fs"                   => ContentType::Entry,
            "thesaurus"            => ContentType::Thesaurus,
            "collocations"         => ContentType::Collocations,
            "word_sets"            => ContentType::WordSets,
            "phrases"              => ContentType::Phrases,
            "examples"             => ContentType::Examples,
            "word_families"        => ContentType::WordFamilies,
            "etymologies"          => ContentType::Etymologies,
            "activator"            => ContentType::Activator,
            "gb_hwd_pron"
            | "us_hwd_pron"
            | "exa_pron"           => ContentType::AudioPronunciation,
            "picture"              => ContentType::Picture,
            "sound" | "sfx"        => ContentType::Sound,
            _                      => return None,
        };
        Some(ContentId { content_type: ct, id: key.to_owned(), anchor })
    }
}

/// A single search result item displayed in the result list.
#[derive(Debug, Clone)]
pub struct SearchResultItem {
    /// Display label (may contain lightweight markup tags)
    pub label:   String,
    /// dict:// path, e.g. `/fs/3.4.6.2`
    pub path:    String,
    /// Normalised sort key
    pub sortkey: String,
    /// Priority (lower = higher priority in display)
    pub prio:    u8,
    /// Optional highlighted snippet (from full-text search)
    pub snippet: Option<String>,
}

// --------------------------------------------------------------------------
// Tests
// --------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_path_entry() {
        let cid = ContentId::from_path("/fs/3.4.6.2").unwrap();
        assert_eq!(cid.content_type, ContentType::Entry);
        assert_eq!(cid.id, "3.4.6.2");
        assert!(cid.anchor.is_none());
    }

    #[test]
    fn test_from_path_with_anchor() {
        let cid = ContentId::from_path("/fs/3.4.6.2#s1").unwrap();
        assert_eq!(cid.id, "3.4.6.2");
        assert_eq!(cid.anchor, Some("s1".to_owned()));
    }

    #[test]
    fn test_from_path_audio() {
        let cid = ContentId::from_path("/gb_hwd_pron/able_g.mp3").unwrap();
        assert_eq!(cid.content_type, ContentType::AudioPronunciation);
        assert_eq!(cid.id, "able_g.mp3");
    }

    #[test]
    fn test_from_path_unknown() {
        assert!(ContentId::from_path("/unknowntype/foo").is_none());
    }

    #[test]
    fn test_content_type_display() {
        assert_eq!(ContentType::Entry.to_string(),       "entry");
        assert_eq!(ContentType::Thesaurus.to_string(),   "thesaurus");
        assert_eq!(ContentType::Activator.to_string(),   "activator");
    }
}
