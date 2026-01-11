# strip-whitespace (core)

Rust core for stripping inter-node whitespace in Astro and Svelte templates.

This crate:

- Parses templates via tree-sitter (Astro + Svelte grammars).
- Computes a minimal set of edits to remove whitespace-only gaps between nodes.
- Optionally produces a rewritten sourcemap.

It’s consumed by:

- The WASM wrapper in `crates/wasm` (used by the JS tooling packages).
- The fuzz harness in `crates/fuzz`.

## API

The primary entry points are:

- `strip_whitespace(code, source_name, language, config) -> CodeAndSourcemap`
- `strip_whitespace_no_sourcemap(code, language, config) -> String`

See the crate root for exports.

## Development

From the repo root:

- `cargo test -p strip-whitespace`
- `cargo run -p strip-whitespace --example strip -- --help` (see `crates/core/examples/`)
