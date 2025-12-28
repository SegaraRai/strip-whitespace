import type { AstroStripWhitespaceOptions } from "./astro.js";
import {
  initWasmOnce,
  strip_whitespace_no_sourcemap,
  strip_whitespace_sourcemap_create,
} from "./wasm.js";
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

let wasmTransformLock: Promise<void> = Promise.resolve();

// HACK: Extract the Vite plugin type from Astro’s types.
// This is because we cannot depend on Vite directly as Astro might upgrade/downgrade it.
type VitePlugin = Extract<
  Awaited<NonNullable<import("astro").ViteUserConfig["plugins"]>[number]>,
  { name: string }
>;

const PLUGIN_NAME = "astro-strip-whitespace";

function moveBeforeAstroBuild(
  plugins: readonly VitePlugin[],
  selfName: string
): void {
  const mutable = plugins as unknown as VitePlugin[];

  const astroBuildIdx = mutable.findIndex((p) => p?.name === "astro:build");
  const selfIdx = mutable.findIndex((p) => p?.name === selfName);

  if (astroBuildIdx === -1 || selfIdx === -1) return;
  if (selfIdx < astroBuildIdx) return;

  const [self] = mutable.splice(selfIdx, 1);
  mutable.splice(astroBuildIdx, 0, self);
}

export function viteStripWhitespacePlugin(
  options: AstroStripWhitespaceOptions = {}
): VitePlugin {
  initWasmOnce();

  const { preserveBlankLines = false } = options;

  return {
    name: PLUGIN_NAME,
    enforce: "pre",

    configResolved(config) {
      // Ensure we run before Astro’s internal build plugin.
      moveBeforeAstroBuild(config.plugins, PLUGIN_NAME);
    },

    async transform(code: string, id: string) {
      // Only transform actual Astro files.
      if (!id.endsWith(".astro") || id.includes("?")) {
        return;
      }

      let unlock: (() => void) | undefined;
      const prev = wasmTransformLock;
      wasmTransformLock = new Promise<void>((resolve) => {
        unlock = resolve;
      });
      await prev;
      try {
        try {
          // Produce a sourcemap so downstream tools (and our fixtures) can validate output.
          // We pass `id` as the source name; in real Vite usage this will typically be a path.
          const out = strip_whitespace_sourcemap_create(
            code,
            id,
            preserveBlankLines
          );

          return {
            code: out.code,
            map: JSON.parse(out.sourcemap),
          };
        } catch (error) {
          const dumpDir =
            process.env.ASTRO_STRIP_WHITESPACE_DUMP_DIR ??
            ".astro-strip-whitespace-dumps";
          mkdirSync(dumpDir, { recursive: true });
          writeFileSync(join(dumpDir, "last-id.txt"), id, "utf8");
          writeFileSync(join(dumpDir, "last-input.astro"), code, "utf8");
          writeFileSync(
            join(dumpDir, "last-error.txt"),
            String((error as any)?.stack ?? error),
            "utf8"
          );

          return {
            code,
            map: null,
          };
        }
      } finally {
        unlock?.();
      }
    },
  };
}
