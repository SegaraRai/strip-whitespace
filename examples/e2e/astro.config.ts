import stripWhitespace from "astro-strip-whitespace";
import mdx from "@astrojs/mdx";
import svelte from "@astrojs/svelte";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "astro/config";
import { env } from "node:process";

const mode =
  ({ default: "default", preserve: "preserve", none: "none" } as const)[
    env.STRIP_MODE ?? ""
  ] ?? "default";

const integrations = {
  default: [stripWhitespace({ preserveBlankLines: false })],
  preserve: [stripWhitespace({ preserveBlankLines: true })],
  none: [],
}[mode];

export default defineConfig({
  outDir: `snapshots/${mode}`,
  build: {
    format: "file",
  },
  integrations: [svelte(), mdx(), ...integrations],
  vite: {
    plugins: [tailwindcss()],
  },
});
