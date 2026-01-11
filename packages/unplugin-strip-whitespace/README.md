# unplugin-strip-whitespace

Strip inter-node whitespace in `.astro` and `.svelte` templates _before_ they’re compiled.

This package is an [unplugin](https://github.com/unjs/unplugin) plugin, so it works with Vite, Rollup, Webpack, Rspack, esbuild, Farm, Nuxt, and more.

If you’re using Astro, you may prefer the dedicated integration: [`astro-strip-whitespace`](../astro-strip-whitespace).

## Install

```bash
pnpm add -D unplugin-strip-whitespace
# npm i -D unplugin-strip-whitespace
# yarn add -D unplugin-strip-whitespace
```

## Usage

### Vite

```ts
// vite.config.ts
import { defineConfig } from "vite";
import stripWhitespace from "unplugin-strip-whitespace/vite";

export default defineConfig({
  plugins: [stripWhitespace()],
});
```

### Rollup

```ts
// rollup.config.ts
import stripWhitespace from "unplugin-strip-whitespace/rollup";

export default {
  plugins: [stripWhitespace()],
};
```

### Webpack

```ts
// webpack.config.js
const stripWhitespace = require("unplugin-strip-whitespace/webpack");

module.exports = {
  plugins: [stripWhitespace.default()],
};
```

### Rspack

```ts
// rspack.config.ts
import stripWhitespace from "unplugin-strip-whitespace/rspack";

export default {
  plugins: [stripWhitespace()],
};
```

### esbuild

```ts
import stripWhitespace from "unplugin-strip-whitespace/esbuild";

// ...
plugins: [stripWhitespace()],
```

### Nuxt

This package also ships a small Nuxt module.

```ts
// nuxt.config.ts
export default defineNuxtConfig({
  modules: ["unplugin-strip-whitespace/nuxt"],
  unpluginStripWhitespace: {
    preserveBlankLines: true,
  },
});
```

## Options

All options are optional.

```ts
import type { StripWhitespaceOptions } from "unplugin-strip-whitespace";
```

- `selectLanguage`: `("astro" | "svelte")[]` or `(id, content) => "astro" | "svelte" | false`
  - Default behavior processes `.astro` and `.svelte` files, skipping `node_modules` and any id containing a query string (`?`).
- `preserveBlankLines`: `boolean` or `(lang, id, content) => boolean`
  - When `true`, skips stripping for gaps that contain an empty line (useful as a “section break” marker).
- `movePluginBefore`: `RegExp` | `(name) => boolean` | `false` | `{ vite?: …; rollup?: …; ... }`
  - Attempts to move this plugin earlier in the final plugin list (where supported).
  - Default: `/^astro:build|^vite-plugin-svelte$/`.
- `skipOnError`: `boolean`
  - When `true`, errors are logged as warnings and the transform is skipped.

Example:

```ts
stripWhitespace({
  preserveBlankLines: true,
  movePluginBefore: {
    vite: /^astro:build|^vite-plugin-svelte$/,
  },
});
```

## What it strips (and why)

This removes whitespace-only gaps between nodes in templates. It’s designed to be safe for hydration by running _before_ Astro/Svelte compilation, rather than minifying the final HTML output.

## License

MIT
