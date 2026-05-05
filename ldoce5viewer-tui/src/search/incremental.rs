//! Incremental (prefix-based) search index.
//!
//! Ports the Python `ldoce5viewer.incremental` module verbatim.
//!
//! ## Binary file format
//!
//! ```text
//! Offset 0:  magic   u32le  = 0x28061691
//! Offset 4:  version u32le  = 1
//! Offset 8:  count   u32le  – number of items
//! Offset 12: first   u32le  – byte offset of the sort-index section
//! Offset 16: data records (variable length), packed end-to-end
//!
//! Each data record (8-byte header + payload):
//!   lenplain    u16le
//!   lentypecode u8
//!   lenlabel    u16le
//!   lenpath     u16le
//!   prio        u8
//!   [plain_utf8][typecode_utf8][label_utf8][path_ascii]
//!
//! Sort-index (at offset `first`):
//!   count × u32le  – file offsets of data records, sorted by (plain, prio)
//! ```

use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

use memmap2::Mmap;
use thiserror::Error;
use unicode_normalization::UnicodeNormalization;

const MAGIC: u32 = 0x28061691;
const DB_VERSION: u32 = 1;

// --------------------------------------------------------------------------
// Text normalisation (identical to Python `normalize_index_key`)
// --------------------------------------------------------------------------

/// Normalise a search key: lowercase, NFKD, keep only lowercase letters
/// and decimal digits.
pub fn normalize_index_key(key: &str) -> String {
    let lower = key.trim().to_lowercase().replace('\u{00a9}', "c");
    lower
        .nfkd()
        .filter(|c| c.is_lowercase() || c.is_ascii_digit())
        .collect()
}

// --------------------------------------------------------------------------
// Errors
// --------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum IncrementalError {
    #[error("Index too small")]
    TooSmall,
    #[error("Index has wrong magic number")]
    WrongMagic,
    #[error("Unsupported index version")]
    WrongVersion,
    #[error("Index is empty")]
    Empty,
    #[error("Index is broken: {0}")]
    Broken(String),
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}

// --------------------------------------------------------------------------
// Search result
// --------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct IncrementalResult {
    pub label: String,
    pub path: String,
    pub plain: String,
    pub prio: u8,
    pub typecode: String,
}

// --------------------------------------------------------------------------
// Searcher
// --------------------------------------------------------------------------

pub struct IncrementalSearcher {
    mmap: Mmap,
    count: u32,
    first: u32,
}

impl IncrementalSearcher {
    pub fn open(path: &Path) -> Result<Self, IncrementalError> {
        let file = std::fs::File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };
        if mmap.len() < 16 {
            return Err(IncrementalError::TooSmall);
        }
        let magic = u32::from_le_bytes(mmap[0..4].try_into().unwrap());
        let version = u32::from_le_bytes(mmap[4..8].try_into().unwrap());
        let count = u32::from_le_bytes(mmap[8..12].try_into().unwrap());
        let first = u32::from_le_bytes(mmap[12..16].try_into().unwrap());

        if magic != MAGIC {
            return Err(IncrementalError::WrongMagic);
        }
        if version != DB_VERSION {
            return Err(IncrementalError::WrongVersion);
        }
        if count == 0 || first == 0 {
            return Err(IncrementalError::Empty);
        }
        let expected_len = (first as usize) + (count as usize) * 4;
        if mmap.len() != expected_len {
            return Err(IncrementalError::Broken(format!(
                "expected {} bytes, got {}",
                expected_len,
                mmap.len()
            )));
        }
        Ok(IncrementalSearcher { mmap, count, first })
    }

    /// Search for records whose normalised `plain` field starts with `key`.
    /// Returns up to `limit` results.
    pub fn search(&self, key: &str, limit: usize) -> Vec<IncrementalResult> {
        let norm = normalize_index_key(key);
        if norm.is_empty() {
            return vec![];
        }
        let mm = &self.mmap;
        let count = self.count as usize;
        let first = self.first as usize;

        // ------------------------------------------------------------------
        // Binary search for the first index slot whose plain >= norm
        // ------------------------------------------------------------------
        let plain_at = |idx: usize| -> String {
            let p = first + idx * 4;
            let off = u32::from_le_bytes(mm[p..p + 4].try_into().unwrap()) as usize;
            let len_plain = u16::from_le_bytes(mm[off..off + 2].try_into().unwrap()) as usize;
            String::from_utf8_lossy(&mm[off + 8..off + 8 + len_plain]).into_owned()
        };

        let bisect_start = |key: &str| -> usize {
            let (mut a, mut b) = (0usize, count);
            while a != b {
                let c = (a + b) / 2;
                if key > plain_at(c).as_str() {
                    a = c + 1;
                } else {
                    b = c;
                }
            }
            a
        };

        let bisect_end = |key: &str, start: usize| -> usize {
            let (mut a, mut b) = (start, count);
            while a != b {
                let c = (a + b) / 2;
                let p = plain_at(c);
                if key < p.as_str() && !p.starts_with(key) {
                    b = c;
                } else {
                    a = c + 1;
                }
            }
            a
        };

        let start = bisect_start(&norm);
        if start == count {
            return vec![];
        }
        let end = bisect_end(&norm, start);
        let take = limit.min(end - start);

        let mut results = Vec::with_capacity(take);
        for i in start..start + take {
            let p = first + i * 4;
            let off = u32::from_le_bytes(mm[p..p + 4].try_into().unwrap()) as usize;
            if let Some(rec) = decode_record(mm, off) {
                results.push(rec);
            }
        }
        results
    }
}

fn decode_record(mm: &Mmap, off: usize) -> Option<IncrementalResult> {
    if off + 8 > mm.len() {
        return None;
    }
    let len_plain = u16::from_le_bytes(mm[off..off + 2].try_into().ok()?) as usize;
    let len_typecode = mm[off + 2] as usize;
    let len_label = u16::from_le_bytes(mm[off + 3..off + 5].try_into().ok()?) as usize;
    let len_path = u16::from_le_bytes(mm[off + 5..off + 7].try_into().ok()?) as usize;
    let prio = mm[off + 7];
    let data_off = off + 8;
    let data_end = data_off + len_plain + len_typecode + len_label + len_path;
    if data_end > mm.len() {
        return None;
    }
    let plain = String::from_utf8_lossy(&mm[data_off..data_off + len_plain]).into_owned();
    let tc_off = data_off + len_plain;
    let typecode = String::from_utf8_lossy(&mm[tc_off..tc_off + len_typecode]).into_owned();
    let lb_off = tc_off + len_typecode;
    let label = String::from_utf8_lossy(&mm[lb_off..lb_off + len_label]).into_owned();
    let pt_off = lb_off + len_label;
    let path = String::from_utf8_lossy(&mm[pt_off..pt_off + len_path]).into_owned();
    Some(IncrementalResult {
        label,
        path,
        plain,
        prio,
        typecode,
    })
}

// --------------------------------------------------------------------------
// Maker
// --------------------------------------------------------------------------

/// Writes an incremental search index.
pub struct IncrementalMaker {
    path: std::path::PathBuf,
    tmp_path: std::path::PathBuf,
    tmpf: std::fs::File,
    /// (file_offset, normalised_plain, prio)
    items: Vec<(u32, String, u8)>,
}

impl IncrementalMaker {
    pub fn new(path: &Path, tmp_path: &Path) -> io::Result<Self> {
        let tmpf = std::fs::File::create(tmp_path)?;
        Ok(IncrementalMaker {
            path: path.to_owned(),
            tmp_path: tmp_path.to_owned(),
            tmpf,
            items: Vec::new(),
        })
    }

    /// Add one item to the index.
    pub fn add_item(
        &mut self,
        plain: &str,
        typecode: &str,
        label: &str,
        path: &str,
        prio: u8,
    ) -> io::Result<()> {
        let plain_n = normalize_index_key(plain);
        let plain_e = plain_n.as_bytes();
        let typecode_e = typecode.as_bytes();
        let label_e = label.as_bytes();
        let path_e = path.as_bytes();

        if plain_e.len() > u16::MAX as usize
            || typecode_e.len() > u8::MAX as usize
            || label_e.len() > u16::MAX as usize
            || path_e.len() > u16::MAX as usize
        {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "field too long"));
        }

        let pos = self.tmpf.seek(SeekFrom::End(0))? as u32;

        // 8-byte header
        self.tmpf.write_all(&(plain_e.len() as u16).to_le_bytes())?;
        self.tmpf.write_all(&[typecode_e.len() as u8])?;
        self.tmpf.write_all(&(label_e.len() as u16).to_le_bytes())?;
        self.tmpf.write_all(&(path_e.len() as u16).to_le_bytes())?;
        self.tmpf.write_all(&[prio])?;
        // payload
        self.tmpf.write_all(plain_e)?;
        self.tmpf.write_all(typecode_e)?;
        self.tmpf.write_all(label_e)?;
        self.tmpf.write_all(path_e)?;

        self.items.push((pos, plain_n, prio));
        Ok(())
    }

    /// Sort and finalise the index file.
    pub fn finalize(mut self) -> io::Result<()> {
        use std::io::BufReader;
        let first = self.tmpf.seek(SeekFrom::End(0))? as u32;
        let num = self.items.len() as u32;

        // Sort by (normalised plain, prio)
        self.items.sort_by(|a, b| a.1.cmp(&b.1).then(a.2.cmp(&b.2)));

        // Write sorted offsets into tmp file
        for (off, _, _) in &self.items {
            self.tmpf.write_all(&off.to_le_bytes())?;
        }
        drop(self.tmpf);

        // Read tmp, write to final path (prepend 16-byte header)
        let tmp_bytes = std::fs::read(&self.tmp_path)?;

        // Adjust the stored record offsets: add the 16-byte file header size.
        // The offsets were written when the tmp file had no header; the final
        // file starts with a 16-byte header then the same data.
        let adjusted_first = first + 16;
        let mut out = std::fs::File::create(&self.path)?;
        out.write_all(&MAGIC.to_le_bytes())?;
        out.write_all(&DB_VERSION.to_le_bytes())?;
        out.write_all(&num.to_le_bytes())?;
        out.write_all(&adjusted_first.to_le_bytes())?;

        // Write data records (adjust each sort-index pointer too)
        let data_len = first as usize;
        out.write_all(&tmp_bytes[..data_len])?;

        // Write sort index (each stored offset += 16)
        for i in 0..num as usize {
            let p = data_len + i * 4;
            let old_off = u32::from_le_bytes(tmp_bytes[p..p + 4].try_into().unwrap());
            let new_off = old_off + 16;
            out.write_all(&new_off.to_le_bytes())?;
        }

        std::fs::remove_file(&self.tmp_path)?;
        Ok(())
    }
}

// --------------------------------------------------------------------------
// Tests
// --------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn build_index(dir: &Path, items: &[(&str, &str, &str, &str, u8)]) -> std::path::PathBuf {
        let idx_path = dir.join("inc.dat");
        let tmp_path = dir.join("inc.tmp");
        let mut maker = IncrementalMaker::new(&idx_path, &tmp_path).unwrap();
        for &(plain, tc, label, path, prio) in items {
            maker.add_item(plain, tc, label, path, prio).unwrap();
        }
        maker.finalize().unwrap();
        idx_path
    }

    const ITEMS: &[(&str, &str, &str, &str, u8)] = &[
        ("apple", "hw", "Apple", "entry/apple", 0),
        ("application", "hw", "Application", "entry/application", 0),
        ("apply", "hw", "Apply", "entry/apply", 1),
        ("banana", "hw", "Banana", "entry/banana", 0),
        ("band", "hw", "Band", "entry/band", 0),
    ];

    #[test]
    fn test_prefix_match() {
        let dir = tempdir().unwrap();
        let idx = build_index(dir.path(), ITEMS);
        let searcher = IncrementalSearcher::open(&idx).unwrap();
        let results = searcher.search("appl", 10);
        let labels: Vec<&str> = results.iter().map(|r| r.label.as_str()).collect();
        assert!(labels.contains(&"Apple"), "Apple missing");
        assert!(labels.contains(&"Application"), "Application missing");
        assert!(labels.contains(&"Apply"), "Apply missing");
        assert!(!labels.contains(&"Banana"), "Banana should not match");
    }

    #[test]
    fn test_exact_match() {
        let dir = tempdir().unwrap();
        let idx = build_index(dir.path(), ITEMS);
        let searcher = IncrementalSearcher::open(&idx).unwrap();
        let results = searcher.search("banana", 10);
        assert!(!results.is_empty());
        assert_eq!(results[0].label, "Banana");
    }

    #[test]
    fn test_no_match() {
        let dir = tempdir().unwrap();
        let idx = build_index(dir.path(), ITEMS);
        let searcher = IncrementalSearcher::open(&idx).unwrap();
        assert!(searcher.search("zzz", 10).is_empty());
    }

    #[test]
    fn test_limit() {
        let dir = tempdir().unwrap();
        let idx = build_index(dir.path(), ITEMS);
        let searcher = IncrementalSearcher::open(&idx).unwrap();
        let results = searcher.search("a", 2);
        assert!(results.len() <= 2);
    }

    #[test]
    fn test_empty_key() {
        let dir = tempdir().unwrap();
        let idx = build_index(dir.path(), ITEMS);
        let searcher = IncrementalSearcher::open(&idx).unwrap();
        assert!(searcher.search("", 10).is_empty());
    }

    #[test]
    fn test_broken_file_too_small() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("bad.dat");
        std::fs::write(&path, [0u8; 4]).unwrap();
        assert!(matches!(
            IncrementalSearcher::open(&path),
            Err(IncrementalError::TooSmall)
        ));
    }

    #[test]
    fn test_wrong_magic() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("bad.dat");
        std::fs::write(&path, [0u8; 16]).unwrap(); // all zeros → wrong magic
        assert!(matches!(
            IncrementalSearcher::open(&path),
            Err(IncrementalError::WrongMagic)
        ));
    }

    #[test]
    fn test_result_structure() {
        let dir = tempdir().unwrap();
        let idx = build_index(dir.path(), ITEMS);
        let searcher = IncrementalSearcher::open(&idx).unwrap();
        let results = searcher.search("band", 5);
        assert!(!results.is_empty());
        let r = &results[0];
        assert_eq!(r.label, "Band");
        assert_eq!(r.path, "entry/band");
        assert!(!r.plain.is_empty());
    }

    #[test]
    fn test_normalize_index_key() {
        assert_eq!(normalize_index_key("  Hello  "), "hello");
        assert_eq!(normalize_index_key("café"), "cafe");
        assert_eq!(normalize_index_key("UPPER"), "upper");
        assert_eq!(normalize_index_key(""), "");
        // copyright sign → 'c'
        assert_eq!(normalize_index_key("\u{00a9}"), "c");
    }
}
