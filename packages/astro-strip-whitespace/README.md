# astro-strip-whitespace

Astro integration that strips inter-node whitespace in `.astro` and `.svelte` templates _before_ compilation.

Under the hood, this prepends the Vite plugin from [`unplugin-strip-whitespace`](../unplugin-strip-whitespace) in `astro:config:done` so it runs early.

## Install

```bash
astro add astro-strip-whitespace

# pnpm add -D astro-strip-whitespace
# npm i -D astro-strip-whitespace
# yarn add -D astro-strip-whitespace
```

## Usage

In `astro.config.mjs` / `astro.config.ts`:

```ts
import { defineConfig } from "astro/config";
import stripWhitespace from "astro-strip-whitespace";

export default defineConfig({
  integrations: [stripWhitespace()],
});
```

### Options

The integration accepts the same options as `unplugin-strip-whitespace` (`StripWhitespaceOptions`). Common ones:

- `preserveBlankLines`: keep "section breaks" by skipping gaps that contain an empty line.
- `selectLanguage`: restrict processing to only Astro or only Svelte.
- `skipOnError`: skip transform on errors instead of failing the build.

Example: only process Astro files

```ts
stripWhitespace({
  selectLanguage: ["astro"],
});
```

## Notes

- If you also use Svelte via `@astrojs/svelte`, the integration can strip whitespace in `.svelte` files too (default behavior).
- This runs as a `pre` transform to avoid hydration mismatches that can happen when whitespace is removed only in the final HTML.

## License

MIT
