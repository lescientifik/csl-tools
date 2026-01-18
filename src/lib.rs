//! csl-tools: CLI for formatting citations and bibliographies in Markdown documents.
//!
//! This library provides functionality to:
//! - Parse citation syntax from Markdown documents
//! - Load CSL-JSON references and CSL styles
//! - Format citations and bibliographies using csl_proc
//! - Generate output with formatted citations

pub mod markdown;
pub mod output;
pub mod processor;
pub mod refs;
pub mod style;

pub use markdown::{
    extract_citation_clusters, extract_citations, Citation, CitationCluster, CitationItem,
};
pub use output::{generate_output, replace_citations};
pub use processor::{
    format_bibliography, format_citations, format_citations_clusters, ProcessedCitation,
};
pub use refs::load_refs;
pub use style::{builtin_style, load_style};
