//! CDB (Constant Database) reader and writer.
//!
//! A pure-Rust port of D.J. Bernstein's CDB format, matching the Python
//! `ldoce5viewer.utils.cdb` implementation exactly so that CDB files
//! written by the Python app are readable here and vice-versa.
//!
//! Format overview:
//! - 2 048-byte header: 256 × (position: u32le, count: u32le)
//! - Data section: records of (klen: u32le, vlen: u32le, key_bytes, value_bytes)
//! - Hash subtables (one per bucket, appended after data)

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

use memmap2::Mmap;
use thiserror::Error;

// --------------------------------------------------------------------------
// Hash function (djb2 variant used by DJB's CDB)
// --------------------------------------------------------------------------

pub fn cdb_hash(key: &[u8]) -> u32 {
    let mut h: u32 = 5381;
    for &b in key {
        h = h.wrapping_mul(33).wrapping_add(b as u32);
    }
    h
}

// --------------------------------------------------------------------------
// Errors
// --------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum CDBError {
    #[error("CDB file too small")]
    TooSmall,
    #[error("CDB file is broken")]
    Broken,
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}

// --------------------------------------------------------------------------
// Reader
// --------------------------------------------------------------------------

/// Read-only accessor for a CDB file.
pub struct CDBReader {
    mmap: Mmap,
    /// 256 × (subtable_pos, subtable_entry_count)
    main_table: [(u32, u32); 256],
}

impl CDBReader {
    /// Open a CDB file from disk using a memory-mapped view.
    pub fn open(path: &Path) -> Result<Self, CDBError> {
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };
        if mmap.len() < 2048 {
            return Err(CDBError::TooSmall);
        }
        let mut main_table = [(0u32, 0u32); 256];
        for i in 0..256 {
            let off = i * 8;
            let pos = u32::from_le_bytes(mmap[off..off + 4].try_into().unwrap());
            let cnt = u32::from_le_bytes(mmap[off + 4..off + 8].try_into().unwrap());
            main_table[i] = (pos, cnt);
        }
        // Basic sanity: every subtable position must be >= 2048 (or zero for empty)
        for &(pos, cnt) in &main_table {
            if cnt > 0 && pos < 2048 {
                return Err(CDBError::Broken);
            }
        }
        Ok(CDBReader { mmap, main_table })
    }

    /// Look up `key`.  Returns `Some(value_bytes)` or `None` if not present.
    pub fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        let mm = &self.mmap;
        let hashed = cdb_hash(key);
        let subidx = (hashed & 0xFF) as usize;
        let hashed_high = hashed >> 8; // i.e. hashed / 256
        let (pos_subtable, num_entries) = self.main_table[subidx];
        if num_entries == 0 {
            return None;
        }
        let pos_subtable = pos_subtable as usize;
        let num_entries = num_entries as usize;

        let ini = (hashed_high as usize) % num_entries;

        // Wrap-around scan of the subtable
        let scan = (ini..num_entries).chain(0..ini);
        for slot in scan {
            let p = pos_subtable + slot * 8;
            if p + 8 > mm.len() {
                return None;
            }
            let h = u32::from_le_bytes(mm[p..p + 4].try_into().unwrap());
            let ptr = u32::from_le_bytes(mm[p + 4..p + 8].try_into().unwrap()) as usize;
            if ptr == 0 {
                // Empty slot → key not present
                return None;
            }
            if h == hashed {
                // Candidate match – verify key
                if ptr + 8 > mm.len() {
                    return None;
                }
                let klen = u32::from_le_bytes(mm[ptr..ptr + 4].try_into().unwrap()) as usize;
                let vlen = u32::from_le_bytes(mm[ptr + 4..ptr + 8].try_into().unwrap()) as usize;
                let pk = ptr + 8;
                if pk + klen + vlen > mm.len() {
                    return None;
                }
                if &mm[pk..pk + klen] == key {
                    return Some(mm[pk + klen..pk + klen + vlen].to_vec());
                }
            }
        }
        None
    }

    /// Returns `true` if the database contains `key`.
    pub fn contains(&self, key: &[u8]) -> bool {
        self.get(key).is_some()
    }

    /// Iterate over all (key, value) pairs in insertion order.
    pub fn iter_items(&self) -> impl Iterator<Item = (Vec<u8>, Vec<u8>)> + '_ {
        let mm = &self.mmap;
        // Total record count = sum of (count / 2) for each bucket.
        // But we can just walk from offset 2048 forward until we hit the subtables.
        // The subtables start after the data; the minimum subtable position is
        // min(pos) for buckets with cnt > 0.
        let first_subtable = self
            .main_table
            .iter()
            .filter(|&&(_, cnt)| cnt > 0)
            .map(|&(pos, _)| pos as usize)
            .min()
            .unwrap_or(mm.len());

        let mut pos = 2048usize;
        let mut items = Vec::new();
        while pos + 8 <= first_subtable {
            let klen = u32::from_le_bytes(mm[pos..pos + 4].try_into().unwrap()) as usize;
            let vlen = u32::from_le_bytes(mm[pos + 4..pos + 8].try_into().unwrap()) as usize;
            pos += 8;
            if pos + klen + vlen > first_subtable {
                break;
            }
            let k = mm[pos..pos + klen].to_vec();
            let v = mm[pos + klen..pos + klen + vlen].to_vec();
            items.push((k, v));
            pos += klen + vlen;
        }
        items.into_iter()
    }
}

// --------------------------------------------------------------------------
// Writer
// --------------------------------------------------------------------------

/// Builds a CDB file.
pub struct CDBMaker<W: Write + Seek> {
    writer: W,
    /// Bucket entries: (hash, record_file_offset)
    sub: [Vec<(u32, u32)>; 256],
    /// Number of slots to allocate per bucket (2 × number of entries)
    sub_num: [u32; 256],
}

impl<W: Write + Seek> CDBMaker<W> {
    /// Create a new maker wrapping `writer`.
    /// The writer must be seekable; the first 2 048 bytes will be the header.
    pub fn new(mut writer: W) -> io::Result<Self> {
        // Reserve space for the header
        writer.seek(SeekFrom::Start(2048))?;
        Ok(CDBMaker {
            writer,
            sub: std::array::from_fn(|_| Vec::new()),
            sub_num: [0u32; 256],
        })
    }

    /// Add a key-value pair.
    pub fn add(&mut self, key: &[u8], value: &[u8]) -> io::Result<()> {
        let pointer = self.writer.stream_position()? as u32;
        let klen = key.len() as u32;
        let vlen = value.len() as u32;
        self.writer.write_all(&klen.to_le_bytes())?;
        self.writer.write_all(&vlen.to_le_bytes())?;
        self.writer.write_all(key)?;
        self.writer.write_all(value)?;

        let hashed = cdb_hash(key);
        let s = (hashed & 0xFF) as usize;
        self.sub_num[s] = self.sub_num[s].saturating_add(2);
        self.sub[s].push((hashed, pointer));
        Ok(())
    }

    /// Finalize the CDB: write the subtables and the 2 048-byte header.
    pub fn finalize(&mut self) -> io::Result<()> {
        let mut sub_pos = [0u32; 256];

        for s in 0..256usize {
            let num = self.sub_num[s] as usize;
            sub_pos[s] = self.writer.stream_position()? as u32;

            // Allocate the subtable as a zeroed buffer of `num` slots (8 bytes each)
            let mut buf = vec![0u8; num * 8];

            for &(hashed, pointer) in &self.sub[s] {
                let hashed_high = hashed >> 8;
                let ini = (hashed_high as usize) % num;
                // Linear probe from ini
                let mut placed = false;
                for offset in (ini..num).chain(0..ini) {
                    let p = offset * 8;
                    let existing_ptr =
                        u32::from_le_bytes(buf[p + 4..p + 8].try_into().unwrap());
                    if existing_ptr == 0 {
                        buf[p..p + 4].copy_from_slice(&hashed.to_le_bytes());
                        buf[p + 4..p + 8].copy_from_slice(&pointer.to_le_bytes());
                        placed = true;
                        break;
                    }
                }
                debug_assert!(placed, "CDBMaker: failed to place entry in subtable");
            }

            self.writer.write_all(&buf)?;
        }

        // Write the 2 048-byte header
        self.writer.seek(SeekFrom::Start(0))?;
        for i in 0..256usize {
            self.writer.write_all(&sub_pos[i].to_le_bytes())?;
            self.writer.write_all(&self.sub_num[i].to_le_bytes())?;
        }
        Ok(())
    }
}

/// Convenience: create a CDB file at `path` from an iterator of (key, value) pairs.
pub fn write_cdb<I>(path: &Path, items: I) -> Result<(), CDBError>
where
    I: IntoIterator<Item = (Vec<u8>, Vec<u8>)>,
{
    use std::io::BufWriter;
    let f = File::create(path)?;
    let mut maker = CDBMaker::new(BufWriter::new(f))?;
    for (k, v) in items {
        maker.add(&k, &v)?;
    }
    maker.finalize()?;
    Ok(())
}

// --------------------------------------------------------------------------
// Tests
// --------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn round_trip(pairs: &[(&[u8], &[u8])]) -> CDBReader {
        // Write to an in-memory buffer
        let buf = Vec::new();
        let cursor = Cursor::new(buf);
        let mut maker = CDBMaker::new(cursor).unwrap();
        for &(k, v) in pairs {
            maker.add(k, v).unwrap();
        }
        maker.finalize().unwrap();
        let bytes = maker.writer.into_inner();

        // Persist to a temp file so we can mmap it
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.cdb");
        std::fs::write(&path, &bytes).unwrap();
        // Keep dir alive via a separate thread-local or just leak it for the test
        // (tempfile cleans up on Drop; to avoid that we box-leak the dir)
        let _ = Box::leak(Box::new(dir));
        CDBReader::open(&path).unwrap()
    }

    #[test]
    fn test_single_entry() {
        let reader = round_trip(&[(b"hello", b"world")]);
        assert_eq!(reader.get(b"hello"), Some(b"world".to_vec()));
    }

    #[test]
    fn test_multiple_entries() {
        let pairs: Vec<(Vec<u8>, Vec<u8>)> = (0..50u8)
            .map(|i| (format!("key{i}").into_bytes(), format!("val{i}").into_bytes()))
            .collect();
        let refs: Vec<(&[u8], &[u8])> = pairs.iter().map(|(k, v)| (k.as_slice(), v.as_slice())).collect();
        let reader = round_trip(&refs);
        for (k, v) in &pairs {
            assert_eq!(reader.get(k), Some(v.clone()), "key {:?}", k);
        }
    }

    #[test]
    fn test_missing_key_returns_none() {
        let reader = round_trip(&[(b"exists", b"yes")]);
        assert_eq!(reader.get(b"nope"), None);
    }

    #[test]
    fn test_contains() {
        let reader = round_trip(&[(b"present", b"1")]);
        assert!(reader.contains(b"present"));
        assert!(!reader.contains(b"absent"));
    }

    #[test]
    fn test_iter_items() {
        let mut expected: HashMap<Vec<u8>, Vec<u8>> = HashMap::new();
        expected.insert(b"k1".to_vec(), b"v1".to_vec());
        expected.insert(b"k2".to_vec(), b"v2".to_vec());
        expected.insert(b"k3".to_vec(), b"v3".to_vec());
        let refs: Vec<(&[u8], &[u8])> = expected
            .iter()
            .map(|(k, v)| (k.as_slice(), v.as_slice()))
            .collect();
        let reader = round_trip(&refs);
        let got: HashMap<Vec<u8>, Vec<u8>> = reader.iter_items().collect();
        assert_eq!(got, expected);
    }

    #[test]
    fn test_empty_value() {
        let reader = round_trip(&[(b"k", b"")]);
        assert_eq!(reader.get(b"k"), Some(vec![]));
    }

    #[test]
    fn test_binary_key_and_value() {
        let key: Vec<u8> = (0u8..=255).collect();
        let val: Vec<u8> = (0u8..=255).rev().collect();
        let reader = round_trip(&[(&key, &val)]);
        assert_eq!(reader.get(&key), Some(val));
    }

    #[test]
    fn test_file_too_small() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("small.cdb");
        std::fs::write(&path, [0u8; 10]).unwrap();
        assert!(matches!(CDBReader::open(&path), Err(CDBError::TooSmall)));
    }

    #[test]
    fn test_hash_determinism() {
        // Same key always produces the same hash
        assert_eq!(cdb_hash(b"hello"), cdb_hash(b"hello"));
        assert_ne!(cdb_hash(b"hello"), cdb_hash(b"world"));
    }
}
