import type { AstroIntegration } from "astro";
import { viteStripWhitespacePlugin } from "./vite.js";

export interface AstroStripWhitespaceOptions {
  /**
   * If true, preserves "section breaks" by skipping gaps that contain an empty line.
   */
  preserveBlankLines?: boolean;
}

export function astroStripWhitespace(
  options: AstroStripWhitespaceOptions = {}
): AstroIntegration {
  return {
    name: "astro-strip-whitespace",
    hooks: {
      "astro:config:setup": ({ updateConfig }) => {
        updateConfig({
          vite: {
            plugins: [viteStripWhitespacePlugin(options)],
          },
        });
      },
    },
  };
}
