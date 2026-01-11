# strip-whitespace-wasm

WASM (`wasm-bindgen`) wrapper around the Rust core crate.

This crate provides a JavaScript-friendly API (via `wasm-bindgen`) that is built and packaged into the workspace npm package at `packages/wasm`.

## Exports

The WASM module exports:

- `stripWhitespace(code, sourceName, language, config)` (returns `{ code, map }`)
- `stripWhitespaceNoSourcemap(code, language, config)` (returns `string`)

Language values are `"astro"` and `"svelte"`.

## Build

Preferred (matches repo tooling):

- From the repo root: `pnpm build:wasm` (release)
- From the repo root: `pnpm build:wasm-dev` (dev)

These run `wasm-pack` with `--target web` and emit artifacts into `packages/wasm/dist`.

You can also build the crate directly:

- `cargo build -p strip-whitespace-wasm --target wasm32-unknown-unknown --release`
