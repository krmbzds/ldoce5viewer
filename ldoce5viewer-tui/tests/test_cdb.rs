//! Integration tests for the CDB (Constant Database) reader / writer.

use std::fs::File;
use std::path::Path;
use tempfile::tempdir;

use ldoce5viewer_tui::data::cdb::{CDBMaker, CDBReader};

// --------------------------------------------------------------------------
// Helpers
// --------------------------------------------------------------------------

fn build_cdb(dir: &Path, pairs: &[(&[u8], &[u8])]) -> std::path::PathBuf {
    let path = dir.join("test.cdb");
    let f = File::create(&path).unwrap();
    let mut maker = CDBMaker::new(f).unwrap();
    for &(k, v) in pairs {
        maker.add(k, v).unwrap();
    }
    maker.finalize().unwrap();
    path
}

// --------------------------------------------------------------------------
// Tests
// --------------------------------------------------------------------------

#[test]
fn test_round_trip_single() {
    let dir = tempdir().unwrap();
    let path = build_cdb(dir.path(), &[(b"hello", b"world")]);
    let reader = CDBReader::open(&path).unwrap();
    assert_eq!(reader.get(b"hello"), Some(b"world".to_vec()));
}

#[test]
fn test_round_trip_many() {
    let dir = tempdir().unwrap();
    let pairs: Vec<(Vec<u8>, Vec<u8>)> = (0u32..100)
        .map(|i| (format!("key{i}").into_bytes(), format!("value{i}").into_bytes()))
        .collect();
    let bpairs: Vec<(&[u8], &[u8])> = pairs.iter().map(|(k, v)| (k.as_slice(), v.as_slice())).collect();
    let path = build_cdb(dir.path(), &bpairs);

    let reader = CDBReader::open(&path).unwrap();
    for (k, v) in &pairs {
        assert_eq!(reader.get(k), Some(v.clone()), "key {:?} missing", k);
    }
}

#[test]
fn test_missing_key_returns_none() {
    let dir = tempdir().unwrap();
    let path = build_cdb(dir.path(), &[(b"a", b"b")]);
    let reader = CDBReader::open(&path).unwrap();
    assert_eq!(reader.get(b"nonexistent"), None);
}

#[test]
fn test_empty_cdb() {
    let dir = tempdir().unwrap();
    let path = build_cdb(dir.path(), &[]);
    let reader = CDBReader::open(&path).unwrap();
    assert_eq!(reader.get(b"anything"), None);
}

#[test]
fn test_binary_keys_and_values() {
    let dir = tempdir().unwrap();
    let key = vec![0u8, 1, 2, 3, 255];
    let val = vec![10u8, 20, 30];
    let path = build_cdb(dir.path(), &[(&key, &val)]);
    let reader = CDBReader::open(&path).unwrap();
    assert_eq!(reader.get(&key), Some(val));
}

#[test]
fn test_iteration() {
    let dir = tempdir().unwrap();
    let path = build_cdb(dir.path(), &[
        (b"a", b"1"),
        (b"b", b"2"),
        (b"c", b"3"),
    ]);
    let reader = CDBReader::open(&path).unwrap();
    let items = reader.iter_items().collect::<Vec<_>>();
    // All three keys must appear (order not guaranteed)
    let keys: std::collections::HashSet<Vec<u8>> = items.iter()
        .map(|(k, _v): &(Vec<u8>, Vec<u8>)| k.clone())
        .collect();
    assert!(keys.contains(b"a".as_ref()));
    assert!(keys.contains(b"b".as_ref()));
    assert!(keys.contains(b"c".as_ref()));
    assert_eq!(keys.len(), 3);
}

#[test]
fn test_hash_collision_resilience() {
    // Pack enough keys to force hash-table chaining
    let dir = tempdir().unwrap();
    let pairs: Vec<(Vec<u8>, Vec<u8>)> = (0u32..300)
        .map(|i| (format!("k{i:04}").into_bytes(), format!("v{i}").into_bytes()))
        .collect();
    let bpairs: Vec<(&[u8], &[u8])> = pairs.iter().map(|(k, v)| (k.as_slice(), v.as_slice())).collect();
    let path = build_cdb(dir.path(), &bpairs);

    let reader = CDBReader::open(&path).unwrap();
    for (k, v) in &pairs {
        assert_eq!(reader.get(k), Some(v.clone()), "missing key {:?}", std::str::from_utf8(k).unwrap());
    }
}

#[test]
fn test_large_values() {
    let dir = tempdir().unwrap();
    let large_val: Vec<u8> = (0..10_000u32).map(|i| (i & 0xFF) as u8).collect();
    let path = build_cdb(dir.path(), &[(b"bigkey", &large_val)]);
    let reader = CDBReader::open(&path).unwrap();
    assert_eq!(reader.get(b"bigkey"), Some(large_val));
}
