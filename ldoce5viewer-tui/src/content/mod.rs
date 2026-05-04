//! Content layer: content types and XML transformers.

pub mod types;
pub mod transform;

pub use types::{ContentType, ContentId, SearchResultItem};
pub use transform::{
    ContentPage, Block, Inline,
    to_ratatui_text,
    transform, transform_entry, transform_thesaurus, transform_collocations,
    transform_word_families, transform_phrases, transform_examples,
    transform_etymologies, transform_word_sets, transform_activator,
};
