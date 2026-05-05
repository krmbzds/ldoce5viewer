//! IDM `.skn` archive reader.
//!
//! The Longman Dictionary of Contemporary English 5th Edition stores its
//! content in a proprietary archive format developed by IDM.  Each archive
//! is a directory whose internal structure is described below.
//!
//! ## Layout of `<archive>.skn/`
//!
//! ```text
//! <archive>.skn/
//!   dirs.skn/
//!     config.cft      – INI-like field-size spec
//!     NAME.tda        – null-separated directory names
//!     dirs.dat        – fixed-size records (contains parent index)
//!   files.skn/
//!     config.cft      – INI-like field-size spec
//!     NAME.tda        – null-separated file names
//!     files.dat       – fixed-size records (contains content offset + parent)
//!     CONTENT.tda     – zlib-compressed content blocks (concatenated)
//!     CONTENT.tda.tdz – catalog: pairs of (orig_size: u32le, cmp_size: u32le)
//! ```
//!
//! A `location` tuple `(cmp_offset, cmp_size, orig_offset, orig_size)` fully
//! describes where and how to read one file from CONTENT.tda.

use std::collections::HashMap;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use flate2::read::ZlibDecoder;
use thiserror::Error;

// --------------------------------------------------------------------------
// Known archive names → relative sub-paths
// --------------------------------------------------------------------------

pub static ARCHIVE_DIRS: &[(&str, &str)] = &[
    ("etymologies", "etymologies.skn"),
    ("word_families", "word_families.skn"),
    ("examples", "examples.skn"),
    ("sound", "sound.skn"),
    ("fs", "fs.skn"),
    ("us_hwd_pron", "us_hwd_pron.skn"),
    ("gb_hwd_pron", "gb_hwd_pron.skn"),
    ("picture", "picture.skn"),
    ("phrases", "phrases.skn"),
    ("sfx", "sfx.skn"),
    ("thesaurus", "thesaurus.skn"),
    ("gram", "gram.skn"),
    ("collocations", "collocations.skn"),
    ("exa_pron", "exa_pron.skn"),
    ("common_errors", "common_errors.skn"),
    ("word_sets", "word_sets.skn"),
    ("menus", "menus.skn"),
    ("word_lists", "word_lists.skn"),
    ("verb_forms", "verb_forms.skn"),
    ("activator", "activator.skn"),
    // Sub-archives within activator
    ("activator_section", "activator.skn/activator_section.skn"),
    ("activator_concept", "activator.skn/activator_concept.skn"),
];

// --------------------------------------------------------------------------
// Errors
// --------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum ArchiveError {
    #[error("Unknown archive name: {0}")]
    UnknownArchive(String),
    #[error("Invalid or missing LDOCE5 data directory")]
    InvalidDataDir,
    #[error("Broken archive: {0}")]
    Broken(String),
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}

// --------------------------------------------------------------------------
// Field type sizes for config.cft parsing
// --------------------------------------------------------------------------

fn field_type_size(t: &str) -> Option<usize> {
    match t.trim() {
        "UBYTE" => Some(1),
        "USHORT" => Some(2),
        "U24" => Some(3),
        "ULONG" => Some(4),
        _ => None,
    }
}

/// Parse a simple `config.cft` file (subset of INI format).
/// Returns a map from field name → `(byte_offset, byte_size)` and
/// the total record size.
fn parse_cft(path: &Path) -> Result<(HashMap<String, (usize, usize)>, usize), ArchiveError> {
    let content = fs::read_to_string(path)?;
    let mut offsets: HashMap<String, (usize, usize)> = HashMap::new();
    let mut offset = 0usize;
    let mut in_dat = false;

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.eq_ignore_ascii_case("[DAT]") {
            in_dat = true;
            continue;
        }
        if line.starts_with('[') {
            in_dat = false;
            continue;
        }
        if !in_dat || line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        // Format: `<field_name_expr> = <TYPE>`
        if let Some((lhs, rhs)) = line.split_once('=') {
            let field_name = lhs.split(',').next().unwrap_or("").trim().to_lowercase();
            // Only accept known IDM types; ignore unknown lines (matches Python behaviour)
            if let Some(size) = field_type_size(rhs.trim()) {
                offsets.insert(field_name, (offset, size));
                offset += size;
            } else {
                // Unknown/irrelevant type (e.g. `NAME.tda = files.dat`) — skip
                continue;
            }
        }
    }
    Ok((offsets, offset))
}

/// Read a little-endian integer of `size` bytes from `data[start..]`.
fn read_int(data: &[u8], start: usize, size: usize) -> usize {
    let mut r = 0usize;
    for i in 0..size {
        r |= (data[start + i] as usize) << (i * 8);
    }
    r
}

// --------------------------------------------------------------------------
// Directory + file list helpers
// --------------------------------------------------------------------------

/// Returns `(name, parent_idx)` for each directory in `dirs.skn/`.
fn load_dir_list(
    base: &Path,
    fields: &HashMap<String, (usize, usize)>,
    rsize: usize,
) -> Result<Vec<(String, usize)>, ArchiveError> {
    let dirs_base = base.join("dirs.skn");
    // Names
    let name_bytes = fs::read(dirs_base.join("NAME.tda"))?;
    let names: Vec<String> = name_bytes
        .split(|&b| b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| String::from_utf8_lossy(s).into_owned())
        .collect();

    // Parent indices
    let (p_off, p_sz) = fields
        .get("$parent")
        .copied()
        .ok_or_else(|| ArchiveError::Broken("missing $parent field".into()))?;
    let dat_bytes = fs::read(dirs_base.join("dirs.dat"))?;
    let num_records = dat_bytes.len() / rsize;

    let mut dirs = Vec::with_capacity(num_records);
    for i in 0..num_records {
        let rec_off = i * rsize;
        let parent = read_int(&dat_bytes, rec_off + p_off, p_sz);
        let name = names.get(i).cloned().unwrap_or_default();
        dirs.push((name, parent));
    }
    Ok(dirs)
}

/// Build the full path (as a `Vec<String>`) for directory index `i`.
///
/// The root directory (index 0) is a virtual root and its name is not included
/// in the returned path.  Mirrors the Python `get_path()` helper in `idmreader.py`:
///   ```python
///   while idx != 0:
///       path.append(dirnames[idx])
///       idx = dir_to_parent[idx]
///   ```
fn build_dir_path(dirs: &[(String, usize)], i: usize) -> Vec<String> {
    let mut path = Vec::new();
    let mut idx = i;
    let mut guard = 0usize;
    while idx != 0 && guard < dirs.len() {
        let (ref name, parent) = dirs[idx];
        path.push(name.clone());
        idx = parent;
        guard += 1;
    }
    path.reverse();
    path
}

/// A file entry with its path information and content location.
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// Directory components, e.g. `["us_hwd_pron", "a"]`
    pub dir_path: Vec<String>,
    /// File name, e.g. `"able_u_1.mp3"`
    pub name: String,
    /// `(cmp_offset, cmp_size, orig_offset, orig_size)` in CONTENT.tda
    pub location: (u64, u64, u64, u64),
}

/// List all files in an archive, yielding `FileEntry` items.
pub fn list_files(data_root: &Path, archive_name: &str) -> Result<Vec<FileEntry>, ArchiveError> {
    let rel = ARCHIVE_DIRS
        .iter()
        .find(|&&(n, _)| n == archive_name)
        .map(|&(_, rel)| rel)
        .ok_or_else(|| ArchiveError::UnknownArchive(archive_name.to_owned()))?;

    let target_base = data_root.join(rel);

    // ── Parse configs ─────────────────────────────────────────────────────
    let (f_fields, f_rsize) = parse_cft(&target_base.join("files.skn").join("config.cft"))?;
    let (d_fields, d_rsize) = parse_cft(&target_base.join("dirs.skn").join("config.cft"))?;

    // ── Load directory list ────────────────────────────────────────────────
    let dirs = load_dir_list(&target_base, &d_fields, d_rsize)?;

    // ── Load file names ────────────────────────────────────────────────────
    let files_base = target_base.join("files.skn");
    let fname_bytes = fs::read(files_base.join("NAME.tda"))?;
    let file_names: Vec<String> = fname_bytes
        .split(|&b| b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| String::from_utf8_lossy(s).into_owned())
        .collect();

    // ── Load file records ──────────────────────────────────────────────────
    let (off_off, off_sz) = f_fields
        .get("$content")
        .copied()
        .ok_or_else(|| ArchiveError::Broken("missing $content field".into()))?;
    let (par_off, par_sz) = f_fields
        .get("$a_dirs")
        .copied()
        .ok_or_else(|| ArchiveError::Broken("missing $a_dirs field".into()))?;

    let dat_bytes = fs::read(files_base.join("files.dat"))?;
    let num_files = dat_bytes.len() / f_rsize;

    let mut offsets: Vec<usize> = Vec::with_capacity(num_files);
    let mut parents: Vec<usize> = Vec::with_capacity(num_files);
    for i in 0..num_files {
        let rec = i * f_rsize;
        offsets.push(read_int(&dat_bytes, rec + off_off, off_sz));
        parents.push(read_int(&dat_bytes, rec + par_off, par_sz));
    }

    // ── Load catalog (CONTENT.tda.tdz) ────────────────────────────────────
    let tdz_bytes = fs::read(files_base.join("CONTENT.tda.tdz"))?;
    let num_chunks = tdz_bytes.len() / 8;
    let mut orig_offsets: Vec<u64> = vec![0];
    let mut cmp_offsets: Vec<u64> = vec![0];
    let mut orig_sizes: Vec<u64> = Vec::with_capacity(num_chunks);
    let mut cmp_sizes: Vec<u64> = Vec::with_capacity(num_chunks);

    for i in 0..num_chunks {
        let p = i * 8;
        let orig_sz = u32::from_le_bytes(tdz_bytes[p..p + 4].try_into().unwrap()) as u64;
        let cmp_sz = u32::from_le_bytes(tdz_bytes[p + 4..p + 8].try_into().unwrap()) as u64;
        orig_sizes.push(orig_sz);
        cmp_sizes.push(cmp_sz);
        orig_offsets.push(orig_offsets.last().unwrap() + orig_sz);
        cmp_offsets.push(cmp_offsets.last().unwrap() + cmp_sz);
    }

    // ── Build per-file sizes ───────────────────────────────────────────────
    let total_orig = orig_offsets.last().copied().unwrap_or(0);
    let mut sizes: Vec<i64> = Vec::with_capacity(num_files);
    for i in 0..num_files {
        if i + 1 < num_files {
            sizes.push(offsets[i + 1] as i64 - offsets[i] as i64 - 1);
        } else {
            sizes.push(-1); // last file
        }
    }

    // ── Map each file to a catalog chunk ──────────────────────────────────
    let mut ci = 0usize;
    let mut entries = Vec::with_capacity(num_files);
    for i in 0..num_files {
        let file_offset = offsets[i] as u64;
        // Advance chunk index
        while ci + 1 < num_chunks && file_offset >= orig_offsets[ci + 1] {
            ci += 1;
        }
        let cmp_offset = cmp_offsets[ci];
        let cmp_size = cmp_sizes[ci];
        let orig_offset_in_chunk = file_offset - orig_offsets[ci];

        let raw_size = if sizes[i] < 0 {
            orig_sizes[ci] - orig_offset_in_chunk - 1
        } else {
            sizes[i] as u64
        };

        let location = (cmp_offset, cmp_size, orig_offset_in_chunk, raw_size);
        let dir_path = build_dir_path(&dirs, parents[i]);
        let name = file_names.get(i).cloned().unwrap_or_default();

        entries.push(FileEntry {
            dir_path,
            name,
            location,
        });
    }

    Ok(entries)
}

// --------------------------------------------------------------------------
// ArchiveReader — reads decompressed content
// --------------------------------------------------------------------------

/// Reads decompressed files from an IDM archive's CONTENT.tda.
pub struct ArchiveReader {
    content_path: PathBuf,
    /// Cache: (cmp_offset, decompressed_block)
    cache: Option<(u64, Vec<u8>)>,
}

impl ArchiveReader {
    pub fn new(data_dir: &Path, archive_name: &str) -> Result<Self, ArchiveError> {
        let rel = ARCHIVE_DIRS
            .iter()
            .find(|&&(n, _)| n == archive_name)
            .map(|&(_, rel)| rel)
            .ok_or_else(|| ArchiveError::UnknownArchive(archive_name.to_owned()))?;

        let content_path = data_dir.join(rel).join("files.skn").join("CONTENT.tda");

        if !content_path.exists() {
            return Err(ArchiveError::InvalidDataDir);
        }
        Ok(ArchiveReader {
            content_path,
            cache: None,
        })
    }

    /// Read the decompressed content of a file given its `location`.
    pub fn read(&mut self, location: (u64, u64, u64, u64)) -> Result<Vec<u8>, ArchiveError> {
        let (cmp_offset, cmp_size, orig_offset, orig_size) = location;

        // Decompress the block, using cache when possible
        let block = if let Some((cached_off, ref cached_data)) = self.cache {
            if cached_off == cmp_offset {
                cached_data.clone()
            } else {
                self.decompress_block(cmp_offset, cmp_size)?
            }
        } else {
            self.decompress_block(cmp_offset, cmp_size)?
        };

        let start = orig_offset as usize;
        let end = start + orig_size as usize;
        if end > block.len() {
            return Err(ArchiveError::Broken(format!(
                "file slice {start}..{end} exceeds decompressed block size {}",
                block.len()
            )));
        }
        let data = block[start..end].to_vec();

        // Update cache
        self.cache = Some((cmp_offset, block));
        Ok(data)
    }

    fn decompress_block(&self, cmp_offset: u64, cmp_size: u64) -> Result<Vec<u8>, ArchiveError> {
        use std::io::prelude::*;
        let mut f = fs::File::open(&self.content_path)?;
        use std::io::Seek;
        f.seek(io::SeekFrom::Start(cmp_offset))?;
        let mut compressed = vec![0u8; cmp_size as usize];
        f.read_exact(&mut compressed)?;
        let mut decoder = ZlibDecoder::new(compressed.as_slice());
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;
        Ok(decompressed)
    }
}

// --------------------------------------------------------------------------
// Validation helper
// --------------------------------------------------------------------------

/// Returns `true` if `path` looks like a valid LDOCE5 data directory.
pub fn is_ldoce5_dir(path: &Path) -> bool {
    // Spot-check a few required archives
    let check_names = ["fs", "us_hwd_pron", "gb_hwd_pron", "sound"];
    for name in check_names {
        let rel = match ARCHIVE_DIRS.iter().find(|&&(n, _)| n == name) {
            Some(&(_, r)) => r,
            None => return false,
        };
        let base = path.join(rel);
        let ok = base.join("dirs.skn").join("config.cft").is_file()
            && base.join("dirs.skn").join("NAME.tda").is_file()
            && base.join("dirs.skn").join("dirs.dat").is_file()
            && base.join("files.skn").join("config.cft").is_file()
            && base.join("files.skn").join("NAME.tda").is_file()
            && base.join("files.skn").join("files.dat").is_file()
            && base.join("files.skn").join("CONTENT.tda").is_file()
            && base.join("files.skn").join("CONTENT.tda.tdz").is_file();
        if !ok {
            return false;
        }
    }
    true
}

// --------------------------------------------------------------------------
// Tests
// --------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_archive_dir_names() {
        // All known archive names are present
        let names: Vec<&str> = ARCHIVE_DIRS.iter().map(|&(n, _)| n).collect();
        assert!(names.contains(&"fs"));
        assert!(names.contains(&"us_hwd_pron"));
        assert!(names.contains(&"gb_hwd_pron"));
        assert!(names.contains(&"activator_section"));
    }

    #[test]
    fn test_is_ldoce5_dir_false_for_tmp() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!is_ldoce5_dir(dir.path()));
    }

    #[test]
    fn test_list_files_unknown_archive() {
        let dir = tempfile::tempdir().unwrap();
        let err = list_files(dir.path(), "no_such_archive");
        assert!(matches!(err, Err(ArchiveError::UnknownArchive(_))));
    }

    #[test]
    fn test_read_int() {
        let data = [0xAB, 0xCD, 0xEF, 0x01];
        assert_eq!(read_int(&data, 0, 1), 0xAB);
        assert_eq!(read_int(&data, 0, 2), 0xCDAB);
        assert_eq!(read_int(&data, 0, 3), 0xEFCDAB);
        assert_eq!(read_int(&data, 0, 4), 0x01EFCDAB);
    }

    #[test]
    fn test_build_dir_path_root() {
        // Index 0 is the root: path is empty (root is not included)
        let dirs = vec![("root".to_string(), 0)];
        let path = build_dir_path(&dirs, 0);
        assert!(path.is_empty());
    }

    #[test]
    fn test_build_dir_path_nested() {
        // dirs: [("root", 0), ("child", 0), ("grandchild", 1)]
        // The root (index 0) is NOT included in the returned path — this matches
        // the Python semantics: walk up while idx != 0, collect names, reverse.
        let dirs = vec![
            ("root".to_string(), 0),
            ("child".to_string(), 0),
            ("grandchild".to_string(), 1),
        ];
        let path = build_dir_path(&dirs, 2);
        // parent of 2 is index 1 ("child"); parent of 1 is 0 (root, not included)
        assert_eq!(path, vec!["child", "grandchild"]);
    }

    #[test]
    fn test_parse_cft_basic() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.cft");
        std::fs::write(&path, "[DAT]\n$content = U24\n$a_dirs = USHORT\n").unwrap();
        let (fields, rsize) = parse_cft(&path).unwrap();
        assert_eq!(fields["$content"], (0, 3));
        assert_eq!(fields["$a_dirs"], (3, 2));
        assert_eq!(rsize, 5);
    }
}
