//! Content layer: content types and XML transformers.

pub mod transform;
pub mod types;

pub use transform::{
    to_ratatui_text, transform, transform_activator, transform_collocations, transform_entry,
    transform_etymologies, transform_examples, transform_phrases, transform_thesaurus,
    transform_word_families, transform_word_sets, Block, ContentPage, Inline,
};
pub use types::{ContentId, ContentType, SearchResultItem};
