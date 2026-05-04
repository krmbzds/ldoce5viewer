//! Search layer: incremental prefix-search and Tantivy full-text search.

pub mod incremental;
pub mod fulltext;

pub use incremental::{IncrementalMaker, IncrementalSearcher, IncrementalResult, IncrementalError, normalize_index_key};
pub use fulltext::{FulltextMaker, FulltextSearcher, FulltextResult, FulltextError};
