//! Search layer: incremental prefix-search and Tantivy full-text search.

pub mod fulltext;
pub mod incremental;

pub use fulltext::{FulltextError, FulltextMaker, FulltextResult, FulltextSearcher};
pub use incremental::{
    normalize_index_key, IncrementalError, IncrementalMaker, IncrementalResult, IncrementalSearcher,
};
