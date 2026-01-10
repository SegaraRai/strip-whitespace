use std::cell::RefCell;

use tree_sitter::Parser;

use crate::{Language, StripError, alloc::ensure_tree_sitter_allocator};

thread_local! {
    /// Shared Tree-sitter parser instance. We reuse it to avoid reloading the language for each call.
    static ASTRO_PARSER: RefCell<Parser> = {
        ensure_tree_sitter_allocator();

        let mut parser = Parser::new();
        let language: tree_sitter::Language = tree_sitter_astro::LANGUAGE.into();
        parser
            .set_language(&language)
            .expect("tree-sitter-astro language load failed");
        RefCell::new(parser)
    };

    /// Shared Tree-sitter parser instance for Svelte files.
    static SVELTE_PARSER: RefCell<Parser> = {
        ensure_tree_sitter_allocator();

        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_svelte_ng::LANGUAGE.into())
            .expect("tree-sitter-svelte language load failed");
        RefCell::new(parser)
    };
}

/// Parse source code for the given language.
pub fn parse(source: &str, language: Language) -> Result<tree_sitter::Tree, StripError> {
    ensure_tree_sitter_allocator();

    match language {
        Language::Astro => ASTRO_PARSER
            .with(|p| {
                let mut parser = p.borrow_mut();
                parser.parse(source, None)
            })
            .ok_or(StripError::ParseFailed),
        Language::Svelte => SVELTE_PARSER
            .with(|p| {
                let mut parser = p.borrow_mut();
                parser.parse(source, None)
            })
            .ok_or(StripError::ParseFailed),
    }
}
