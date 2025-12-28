use std::cell::RefCell;

use tree_sitter::Parser;

use crate::{StripError, alloc::ensure_tree_sitter_allocator};

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
}

pub fn parse_astro(source: &str) -> Result<tree_sitter::Tree, StripError> {
    ensure_tree_sitter_allocator();

    ASTRO_PARSER
        .with(|p| {
            let mut parser = p.borrow_mut();
            parser.parse(source, None)
        })
        .ok_or(StripError::ParseFailed)
}
