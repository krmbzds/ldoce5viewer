//! Full-text search powered by Tantivy.
//!
//! Mirrors the Python `ldoce5viewer.fulltext` module.
//!
//! ## Schema
//!
//! | field      | type            | purpose                                  |
//! |------------|-----------------|------------------------------------------|
//! | content    | TEXT (indexed + stored) | searchable text                  |
//! | label      | TEXT (stored)   | display label (may contain markup)       |
//! | path       | TEXT (stored)   | dict:// URL path                         |
//! | prio       | u64  (stored)   | sort priority                            |
//! | sortkey    | TEXT (stored)   | normalised key for ordering              |
//! | itemtype   | TEXT (indexed)  | item type code (hm, hp, pl, p, e, d, …)  |
//! | asfilter   | TEXT (indexed)  | space-separated filter tokens            |

use std::path::Path;

use tantivy::collector::TopDocs;
use tantivy::query::{AllQuery, BooleanQuery, Occur, QueryParser, TermQuery};
use tantivy::schema::*;
use tantivy::tokenizer::{LowerCaser, SimpleTokenizer, StopWordFilter, TextAnalyzer};
use tantivy::{Index, IndexWriter, ReloadPolicy, TantivyDocument, Term};
use thiserror::Error;
use unicode_normalization::UnicodeNormalization;

use crate::search::incremental::normalize_index_key;

// --------------------------------------------------------------------------
// Errors
// --------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum FulltextError {
    #[error("Index not found or corrupt")]
    IndexNotFound,
    #[error("Tantivy error: {0}")]
    Tantivy(#[from] tantivy::TantivyError),
    #[error("Query parse error: {0}")]
    QueryParse(#[from] tantivy::query::QueryParserError),
}

// --------------------------------------------------------------------------
// Schema
// --------------------------------------------------------------------------

#[derive(Clone)]
pub struct LdoceSchema {
    pub schema:    Schema,
    pub content:   Field,
    pub label:     Field,
    pub path:      Field,
    pub prio:      Field,
    pub sortkey:   Field,
    pub itemtype:  Field,
    pub asfilter:  Field,
}

fn build_schema() -> LdoceSchema {
    let mut sb = Schema::builder();
    let content  = sb.add_text_field("content",  TEXT | STORED);
    let label    = sb.add_text_field("label",    STORED);
    let path     = sb.add_text_field("path",     STORED);
    let prio     = sb.add_u64_field("prio",      STORED);
    let sortkey  = sb.add_text_field("sortkey",  STORED);
    let itemtype = sb.add_text_field("itemtype", STRING);
    let asfilter = sb.add_text_field("asfilter", TEXT);
    LdoceSchema {
        schema: sb.build(),
        content, label, path, prio, sortkey, itemtype, asfilter,
    }
}

/// Normalise a token for indexing (accent-strip, lowercase).
fn normalize_token(text: &str) -> String {
    let s = text.replace('\u{00a9}', "c");
    s.nfkd()
        .filter(|c| {
            // Keep characters that are NOT Unicode non-spacing marks (Mn).
            // Combining diacritics are in the range U+0300–U+036F and a few others.
            // We approximate by keeping everything except chars whose `is_mark` property is true.
            // In practice, filtering out chars in the 0x0300–0x036F range is sufficient.
            let cp = *c as u32;
            !(0x0300..=0x036F).contains(&cp)
                && !(0x1DC0..=0x1DFF).contains(&cp)
                && !(0x20D0..=0x20FF).contains(&cp)
                && !(0xFE20..=0xFE2F).contains(&cp)
        })
        .collect()
}

fn make_analyzer(index: &Index) {
    let stopwords: Vec<String> = vec!["a".to_string(), "an".to_string()];
    let analyzer = TextAnalyzer::builder(SimpleTokenizer::default())
        .filter(LowerCaser)
        .filter(StopWordFilter::remove(stopwords))
        .build();
    index.tokenizers().register("ldoce_analyzer", analyzer);
}

// --------------------------------------------------------------------------
// Search result
// --------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct FulltextResult {
    pub label:     String,
    pub path:      String,
    pub sortkey:   String,
    pub prio:      u64,
    pub highlight: Option<String>,
}

// --------------------------------------------------------------------------
// Maker — builds the Tantivy index
// --------------------------------------------------------------------------

pub struct FulltextMaker {
    index:  Index,
    writer: IndexWriter,
    s:      LdoceSchema,
}

impl FulltextMaker {
    /// Create a new index at `dir` (directory will be created if needed).
    pub fn new(dir: &Path) -> Result<Self, FulltextError> {
        std::fs::create_dir_all(dir)?;
        let s = build_schema();
        let index = Index::create_in_dir(dir, s.schema.clone())?;
        make_analyzer(&index);
        let writer = index.writer(50_000_000)?;
        Ok(FulltextMaker { index, writer, s })
    }

    /// Add one item to the index.
    #[allow(clippy::too_many_arguments)]
    pub fn add_item(
        &mut self,
        itemtype:  &str,
        content:   &str,
        asfilter:  &str,
        label:     &str,
        path:      &str,
        prio:      u64,
        sortkey:   &str,
    ) -> Result<(), FulltextError> {
        let normalised_content = normalize_token(content);
        let normalised_sortkey = normalize_index_key(sortkey);

        let mut doc = TantivyDocument::default();
        doc.add_text(self.s.content,  &normalised_content);
        doc.add_text(self.s.label,    label);
        doc.add_text(self.s.path,     path);
        doc.add_u64(self.s.prio,      prio);
        doc.add_text(self.s.sortkey,  &normalised_sortkey);
        doc.add_text(self.s.itemtype, itemtype);
        doc.add_text(self.s.asfilter, asfilter);

        self.writer.add_document(doc)?;
        Ok(())
    }

    /// Commit the index.
    pub fn commit(mut self) -> Result<(), FulltextError> {
        self.writer.commit()?;
        Ok(())
    }
}

// --------------------------------------------------------------------------
// Searcher — queries the index
// --------------------------------------------------------------------------

pub struct FulltextSearcher {
    index:  Index,
    s:      LdoceSchema,
}

impl FulltextSearcher {
    /// Open an existing index at `dir`.
    pub fn open(dir: &Path) -> Result<Self, FulltextError> {
        if !dir.exists() {
            return Err(FulltextError::IndexNotFound);
        }
        let s = build_schema();
        let index = match Index::open_in_dir(dir) {
            Ok(idx) => idx,
            Err(_) => return Err(FulltextError::IndexNotFound),
        };
        make_analyzer(&index);
        Ok(FulltextSearcher { index, s })
    }

    /// Search the index.
    ///
    /// * `query_str`  – optional content query (may contain AND / NOT / wildcards)
    /// * `itemtypes`  – optional slice of item type codes to restrict to
    /// * `asfilters`  – optional asfilter query string
    /// * `limit`      – max results (`None` = unlimited, capped at 10 000)
    pub fn search(
        &self,
        query_str:   Option<&str>,
        itemtypes:   &[&str],
        asfilter_q:  Option<&str>,
        limit:       Option<usize>,
    ) -> Result<Vec<FulltextResult>, FulltextError> {
        let reader = self.index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;
        let searcher = reader.searcher();

        // --- Build the compound query ---
        let mut outer: Vec<(Occur, Box<dyn tantivy::query::Query>)> = Vec::new();

        // Content query (Must)
        if let Some(qs) = query_str.filter(|s| !s.is_empty()) {
            let qp = QueryParser::for_index(&self.index, vec![self.s.content]);
            let q = qp.parse_query(qs)?;
            outer.push((Occur::Must, q));
        }

        // itemtype filter: wrap each code as a Should inside a nested Must
        // e.g. Must( Should(type:hm) | Should(type:hp) )
        if !itemtypes.is_empty() {
            let type_clauses: Vec<(Occur, Box<dyn tantivy::query::Query>)> = itemtypes
                .iter()
                .map(|&it| {
                    let term = Term::from_field_text(self.s.itemtype, it);
                    let tq: Box<dyn tantivy::query::Query> =
                        Box::new(TermQuery::new(term, IndexRecordOption::Basic));
                    (Occur::Should, tq)
                })
                .collect();
            outer.push((Occur::Must, Box::new(BooleanQuery::new(type_clauses))));
        }

        // asfilter (Must)
        if let Some(af) = asfilter_q.filter(|s| !s.is_empty()) {
            let afp = QueryParser::for_index(&self.index, vec![self.s.asfilter]);
            let q = afp.parse_query(af)?;
            outer.push((Occur::Must, q));
        }

        let cap = limit.unwrap_or(10_000).min(10_000);

        let top_docs = match outer.len() {
            0 => searcher.search(&AllQuery, &TopDocs::with_limit(cap))?,
            1 => searcher.search(outer.remove(0).1.as_ref(), &TopDocs::with_limit(cap))?,
            _ => {
                let bq = BooleanQuery::new(outer);
                searcher.search(&bq, &TopDocs::with_limit(cap))?
            }
        };

        let mut results: Vec<FulltextResult> = Vec::with_capacity(top_docs.len());
        for (_score, addr) in top_docs {
            let doc: TantivyDocument = searcher.doc(addr)?;
            let label   = get_text(&doc, self.s.label);
            let path    = get_text(&doc, self.s.path);
            let sortkey = get_text(&doc, self.s.sortkey);
            let prio    = doc.get_first(self.s.prio)
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            results.push(FulltextResult { label, path, sortkey, prio, highlight: None });
        }

        // Sort by (sortkey, prio) – matches Python behaviour
        results.sort_by(|a, b| a.sortkey.cmp(&b.sortkey).then(a.prio.cmp(&b.prio)));
        Ok(results)
    }

    /// Suggest spelling corrections for a misspelled word.
    pub fn correct(&self, misspelled: &str, limit: usize) -> Vec<String> {
        let reader = match self.index.reader_builder().try_into() {
            Ok(r) => r,
            Err(_) => return vec![],
        };
        let ix_reader = reader.searcher();
        // Tantivy doesn't have a built-in spell corrector in 0.22,
        // so we return an empty list. A real implementation would use
        // a Levenshtein-based approach or an external crate.
        let _ = ix_reader;
        let _ = misspelled;
        let _ = limit;
        vec![]
    }
}

fn get_text(doc: &TantivyDocument, field: Field) -> String {
    doc.get_first(field)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_owned()
}

// --------------------------------------------------------------------------
// std::io::Error conversion (needed by Maker::new)
// --------------------------------------------------------------------------

impl From<std::io::Error> for FulltextError {
    fn from(e: std::io::Error) -> Self {
        FulltextError::Tantivy(tantivy::TantivyError::IoError(std::sync::Arc::new(e)))
    }
}

// --------------------------------------------------------------------------
// Tests
// --------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn build_index(dir: &Path) -> FulltextMaker {
        FulltextMaker::new(dir).unwrap()
    }

    #[test]
    fn test_basic_search() {
        let dir = tempdir().unwrap();
        let mut maker = build_index(dir.path());
        maker.add_item("hm", "apple", "", "Apple", "/fs/apple", 0, "apple").unwrap();
        maker.add_item("hm", "banana", "", "Banana", "/fs/banana", 0, "banana").unwrap();
        maker.commit().unwrap();

        let searcher = FulltextSearcher::open(dir.path()).unwrap();
        let results = searcher.search(Some("apple"), &[], None, None).unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].label, "Apple");
    }

    #[test]
    fn test_itemtype_filter() {
        let dir = tempdir().unwrap();
        let mut maker = build_index(dir.path());
        maker.add_item("hm", "run",  "", "run (verb)", "/fs/run_v",  0, "run").unwrap();
        maker.add_item("e",  "run fast", "", "example of run", "/fs/e1", 1, "run").unwrap();
        maker.commit().unwrap();

        let searcher = FulltextSearcher::open(dir.path()).unwrap();
        let results = searcher.search(Some("run"), &["hm"], None, None).unwrap();
        // Should only return headwords
        assert!(results.iter().all(|r| !r.path.starts_with("/fs/e")));
    }

    #[test]
    fn test_no_results() {
        let dir = tempdir().unwrap();
        let mut maker = build_index(dir.path());
        maker.add_item("hm", "elephant", "", "Elephant", "/fs/elephant", 0, "elephant").unwrap();
        maker.commit().unwrap();

        let searcher = FulltextSearcher::open(dir.path()).unwrap();
        let results = searcher.search(Some("zzz"), &[], None, None).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_limit() {
        let dir = tempdir().unwrap();
        let mut maker = build_index(dir.path());
        for i in 0..20u32 {
            maker.add_item(
                "hm", &format!("word{i}"), "", &format!("Word{i}"), &format!("/fs/w{i}"), i as u64, &format!("word{i}"),
            ).unwrap();
        }
        maker.commit().unwrap();

        let searcher = FulltextSearcher::open(dir.path()).unwrap();
        let results = searcher.search(Some("word"), &[], None, Some(5)).unwrap();
        assert!(results.len() <= 5);
    }

    #[test]
    fn test_sort_order() {
        let dir = tempdir().unwrap();
        let mut maker = build_index(dir.path());
        maker.add_item("hm", "zoo",   "", "Zoo",   "/fs/zoo",   0, "zoo").unwrap();
        maker.add_item("hm", "aardvark", "", "Aardvark", "/fs/aardvark", 0, "aardvark").unwrap();
        maker.add_item("hm", "middle", "", "Middle", "/fs/middle", 0, "middle").unwrap();
        maker.commit().unwrap();

        let searcher = FulltextSearcher::open(dir.path()).unwrap();
        // Search for all (empty query_str is treated as AllQuery)
        let results = searcher.search(None, &[], None, None).unwrap();
        // Results must be sorted by sortkey
        for w in results.windows(2) {
            assert!(w[0].sortkey <= w[1].sortkey, "not sorted: {:?} > {:?}", w[0].sortkey, w[1].sortkey);
        }
    }

    #[test]
    fn test_open_missing_dir() {
        let err = FulltextSearcher::open(Path::new("/nonexistent/path/xyz"));
        assert!(matches!(err, Err(FulltextError::IndexNotFound)));
    }
}
