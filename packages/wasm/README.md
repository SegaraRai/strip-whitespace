# strip-whitespace-wasm

This directory contains the workspace package that holds the built WebAssembly (WASM) + JS bindings for `strip-whitespace`.

- Rust source lives in [crates/wasm](../../crates/wasm).
- Build output is written to `dist/` (git-ignored) and then consumed by the JavaScript packages in this repo (e.g. the unplugin and Astro integration).
- This package is marked `private` and is intended for internal/workspace use, not direct installation.

## About `dist/`

This folder is intentionally not committed (it’s git-ignored), so this directory will look effectively empty in a fresh checkout until you build.

When built, `dist/` contains an ES module wrapper plus the actual `.wasm` binary and TypeScript typings:

- `dist/index.js` / `dist/index.d.ts` - JS entry + types
- `dist/index_bg.wasm` - the WASM binary
- `dist/index_bg.wasm.d.ts` - typing for the WASM export

The package also exports `./wasm` to allow consumers/bundlers to reference the raw WASM file.

## Build

From the repo root:

- `pnpm build:wasm` - release build
- `pnpm build:wasm-dev` - dev build (faster, less optimized)

These run `wasm-pack` with `--target web` and emit the artifacts into `packages/wasm/dist`.
