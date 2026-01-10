import type { AstroIntegration } from "astro";
import type { StripWhitespaceOptions } from "unplugin-strip-whitespace";
import viteStripWhitespace from "unplugin-strip-whitespace/vite";

function astroStripWhitespace(
  options?: StripWhitespaceOptions,
): AstroIntegration {
  return {
    name: "astro-strip-whitespace",
    hooks: {
      "astro:config:done": ({ config }) => {
        config.vite ??= {};
        config.vite.plugins ??= [];
        config.vite.plugins.unshift(viteStripWhitespace(options));
      },
    },
  } satisfies AstroIntegration;
}

export default astroStripWhitespace;
