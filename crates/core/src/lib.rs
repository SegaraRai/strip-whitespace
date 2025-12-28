//! Astro whitespace stripping library (with WASM bindings).
//!
//! This crate provides a whitespace stripper for Astro that focuses on removing inter-node
//! whitespace gaps while keeping mappings predictable.
//!
//! Entry points:
//!
//! - [`strip`] contains the core Rust APIs for stripping and (re)mapping sourcemaps.
//!
//! Internals:
//!
//! - [`alloc`] contains the tree-sitter allocator override for WASM targets.
//! - [`edit`] holds the edit model and sourcemap creation/rewriting helpers.
//! - [`parse`] contains the tree-sitter parsing logic.
//! - [`utf16`] provides UTF-16 column indexing support for sourcemaps.

pub mod alloc;
pub mod edit;
pub mod parse;
pub mod strip;
pub mod utf16;

/// Errors that can occur during stripping.
#[derive(thiserror::Error, Debug)]
pub enum StripError {
    #[error("tree-sitter failed to parse input")]
    ParseFailed,

    #[error("invalid edit: {0}")]
    InvalidEdit(String),

    #[error("overlapping edits: [{a_start},{a_end}) overlaps [{b_start},{b_end})")]
    OverlappingEdits {
        a_start: usize,
        a_end: usize,
        b_start: usize,
        b_end: usize,
    },

    #[error("invalid sourcemap: {0}")]
    SourceMap(#[from] sourcemap::Error),
}
