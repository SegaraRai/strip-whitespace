# strip-whitespace

[![CI](https://github.com/SegaraRai/strip-whitespace/actions/workflows/ci.yml/badge.svg)](https://github.com/SegaraRai/strip-whitespace/actions/workflows/ci.yml)

Strip inter-node whitespace in Astro and Svelte templates.

<img src="assets/transform.svg" alt="Before/after transform" style="width:100%;height:auto;" />

This repo contains:

- A Rust core that parses templates via tree-sitter and rewrites whitespace.
- A WASM wrapper used from JavaScript tooling.
- An [unplugin](https://github.com/unjs/unplugin) plugin (Vite/Rollup/Webpack/etc).
- A tiny Astro integration that wires the Vite plugin into Astro.

## How it works

This project strips _inter-node_ whitespace (whitespace-only gaps between elements/expressions) in `.astro` and `.svelte` templates.

Importantly, this is a _rewrite_, not a simple "delete all whitespace text nodes" pass: in some cases we preserve the original formatting/newlines by moving whitespace into places that don’t create runtime text nodes. For example, when encountering `<p>\n  text`, we may rewrite it to `<p \n >text`.

- The Rust core parses the template (via tree-sitter), finds whitespace-only gaps between nodes, and rewrites them.
- The JS plugins run this transform on the source templates using an unplugin `transform` hook with `enforce: "pre"`, so the whitespace is removed _before_ Astro/Svelte compilation.
- The unplugin also tries to reorder itself ahead of the framework compilers (by default, before `astro:build` and `vite-plugin-svelte`) so it consistently runs early.

### Why not minify after build?

Minifying the _built output_ (HTML emitted after compilation) can introduce hydration mismatches: the server-rendered HTML no longer matches what the client-side framework expects to hydrate (especially around text nodes / whitespace at component boundaries). By stripping whitespace _before_ the template processor/compiler runs, the compiled output and hydration expectations stay in sync.

## Install

### Astro integration

```bash
astro add astro-strip-whitespace

# or: pnpm add -D astro-strip-whitespace
# or: npm i -D astro-strip-whitespace
# or: yarn add -D astro-strip-whitespace
```

### Vite / other bundlers (unplugin)

```bash
pnpm add -D unplugin-strip-whitespace
# or: npm i -D unplugin-strip-whitespace
# or: yarn add -D unplugin-strip-whitespace
```

## Usage

### Astro

In `astro.config.mjs` / `astro.config.ts`:

```ts
import { defineConfig } from "astro/config";
import stripWhitespace from "astro-strip-whitespace";

export default defineConfig({
  integrations: [stripWhitespace()],
});
```

### Vite

In `vite.config.ts`:

```ts
import { defineConfig } from "vite";
import stripWhitespace from "unplugin-strip-whitespace/vite";

export default defineConfig({
  plugins: [stripWhitespace()],
});
```

### Rollup / Webpack / others

This project ships per-bundler entrypoints. Examples:

- `unplugin-strip-whitespace/rollup`
- `unplugin-strip-whitespace/webpack`
- `unplugin-strip-whitespace/esbuild`
- `unplugin-strip-whitespace/nuxt`
- `unplugin-strip-whitespace/rspack`
- `unplugin-strip-whitespace/farm`

Refer to the bundler-specific unplugin docs for the exact wiring pattern.

## Development

This is a monorepo containing Rust crates and JavaScript packages:

```text
strip-whitespace/
├── crates/             # [Rust] Rust crates excluding fuzz targets
│   ├── core/           #        Core Rust library for parsing & rewriting templates
│   └── wasm/           #        WASM wrapper for JavaScript integration
├── packages/           # [JS] JavaScript packages
│   ├── wasm/                       # WASM bindings generated from crates/wasm (private package)
│   ├── unplugin-strip-whitespace/  # Unplugin for Vite/Rollup/Webpack/etc
│   └── astro-strip-whitespace/     # Astro integration wrapper
├── examples/           # [JS] Example projects
│   └── e2e-astro/      #      End-to-end Astro test app
├── fixtures/           # Test fixtures
├── fuzz/               # [Rust] Fuzzing targets for Rust crates
├── scripts/            # Build and utility scripts
└── assets/             # Documentation assets (images, SVGs)
```

### Key Components

- **[crates/core/](crates/core/)**: The core Rust library that uses tree-sitter to parse Astro and Svelte templates and rewrite whitespace.
- **[crates/wasm/](crates/wasm/)**: WASM bindings that expose the Rust core to JavaScript via wasm-pack.
- **[packages/unplugin-strip-whitespace/](packages/unplugin-strip-whitespace/)**: An [unplugin](https://github.com/unjs/unplugin) that works with Vite, Rollup, Webpack, esbuild, and other bundlers.
- **[packages/astro-strip-whitespace/](packages/astro-strip-whitespace/)**: A tiny Astro integration that configures the Vite plugin for Astro projects.
- **[examples/e2e-astro/](examples/e2e-astro/)**: An example Astro app for testing and development.

### Common Commands

- `pnpm test` - run JS tests (Vitest)
- `pnpm run build` - build WASM + packages
- `pnpm run build:e2e-astro` / `pnpm run dev:e2e-astro` - run the Astro example app
