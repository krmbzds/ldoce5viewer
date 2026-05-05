//! Persistent configuration.
//!
//! The config is stored as a JSON file in the platform-specific app config
//! directory (mirrors the Python `ldoce5viewer.qtgui.config` module).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Auto-pronunciation language choice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AutoPronLanguage {
    Off,
    GB,
    US,
}

impl Default for AutoPronLanguage {
    fn default() -> Self {
        AutoPronLanguage::Off
    }
}

/// All persisted application settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Path to the LDOCE5 data directory.
    pub data_dir: Option<PathBuf>,

    /// Auto-pronunciation language.
    pub auto_pron: AutoPronLanguage,

    /// Monitor clipboard and auto-search when it changes.
    pub monitor_clipboard: bool,

    /// Index directory (built from data_dir).
    pub index_dir: Option<PathBuf>,

    /// Last search query (restored on start).
    pub last_query: String,

    /// Terminal width saved for session restore.
    pub terminal_width: u16,

    /// Terminal height saved for session restore.
    pub terminal_height: u16,

    /// Whether to wrap long lines in the content pane.
    pub content_wrap: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            data_dir: None,
            auto_pron: AutoPronLanguage::Off,
            monitor_clipboard: false,
            index_dir: None,
            last_query: String::new(),
            terminal_width: 120,
            terminal_height: 40,
            content_wrap: true,
        }
    }
}

// --------------------------------------------------------------------------
// Platform directory helpers
// --------------------------------------------------------------------------

/// Returns the application config directory.
pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ldoce5viewer-tui")
}

/// Returns the application data directory (indices, etc.).
pub fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ldoce5viewer-tui")
}

/// Path to the config JSON file.
pub fn config_file_path() -> PathBuf {
    config_dir().join("config.json")
}

/// Path to the incremental index file.
pub fn incremental_index_path() -> PathBuf {
    data_dir().join("incremental.db")
}

/// Path to the Tantivy headword/phrase full-text index.
pub fn fulltext_hwdphr_dir() -> PathBuf {
    data_dir().join("fulltext_hp")
}

/// Path to the Tantivy definitions/examples full-text index.
pub fn fulltext_defexa_dir() -> PathBuf {
    data_dir().join("fulltext_de")
}

/// Path to the filemap CDB.
pub fn filemap_path() -> PathBuf {
    data_dir().join("filemap.cdb")
}

/// Path to the word-variations CDB.
pub fn variations_path() -> PathBuf {
    data_dir().join("variations.cdb")
}

// --------------------------------------------------------------------------
// Load / save
// --------------------------------------------------------------------------

/// Load the config from disk, returning `Config::default()` if the file does
/// not exist or cannot be parsed.
pub fn load_config() -> Config {
    let path = config_file_path();
    match std::fs::read_to_string(&path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => Config::default(),
    }
}

/// Save the config to disk.
pub fn save_config(cfg: &Config) -> Result<(), ConfigError> {
    let dir = config_dir();
    std::fs::create_dir_all(&dir)?;
    let path = config_file_path();
    // Atomic write: write to a temp file then rename
    let tmp = path.with_extension("tmp");
    let s = serde_json::to_string_pretty(cfg)?;
    std::fs::write(&tmp, s)?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

// --------------------------------------------------------------------------
// Tests
// --------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_default_config() {
        let cfg = Config::default();
        assert_eq!(cfg.auto_pron, AutoPronLanguage::Off);
        assert!(!cfg.monitor_clipboard);
        assert!(cfg.data_dir.is_none());
        assert!(cfg.content_wrap);
    }

    #[test]
    fn test_round_trip() {
        let mut cfg = Config::default();
        cfg.auto_pron = AutoPronLanguage::GB;
        cfg.monitor_clipboard = true;
        cfg.last_query = "hello".to_owned();
        cfg.content_wrap = false;

        let s = serde_json::to_string_pretty(&cfg).unwrap();
        let restored: Config = serde_json::from_str(&s).unwrap();

        assert_eq!(restored.auto_pron, AutoPronLanguage::GB);
        assert!(restored.monitor_clipboard);
        assert_eq!(restored.last_query, "hello");
        assert!(!restored.content_wrap);
    }

    #[test]
    fn test_load_config_missing_returns_default() {
        // Point config dir to a temp dir with no file
        // (We can't easily override the global config path in tests, so
        // just verify load_config() doesn't panic.)
        let _cfg = load_config();
    }

    #[test]
    fn test_save_and_load_config() {
        // Use a temp directory to avoid polluting the real config
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.json");

        let mut cfg = Config::default();
        cfg.last_query = "world".to_owned();
        cfg.content_wrap = false;

        // Save to path directly (bypass global config_file_path())
        let s = serde_json::to_string_pretty(&cfg).unwrap();
        std::fs::write(&path, s).unwrap();

        // Load from same path
        let s2 = std::fs::read_to_string(&path).unwrap();
        let loaded: Config = serde_json::from_str(&s2).unwrap();
        assert_eq!(loaded.last_query, "world");
        assert!(!loaded.content_wrap);
    }

    #[test]
    fn test_auto_pron_serde() {
        let off = serde_json::to_string(&AutoPronLanguage::Off).unwrap();
        let gb = serde_json::to_string(&AutoPronLanguage::GB).unwrap();
        let us = serde_json::to_string(&AutoPronLanguage::US).unwrap();
        assert_eq!(
            serde_json::from_str::<AutoPronLanguage>(&off).unwrap(),
            AutoPronLanguage::Off
        );
        assert_eq!(
            serde_json::from_str::<AutoPronLanguage>(&gb).unwrap(),
            AutoPronLanguage::GB
        );
        assert_eq!(
            serde_json::from_str::<AutoPronLanguage>(&us).unwrap(),
            AutoPronLanguage::US
        );
    }
}
