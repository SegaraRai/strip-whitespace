import {
  initWasmOnce,
  stripWhitespace,
  stripWhitespaceNoSourcemap,
  type InitOutput,
} from "#wasm";
import { readFile } from "node:fs/promises";
import { it } from "vitest";

function getWasmMemoryBytes(initOutput: InitOutput): number | undefined {
  try {
    return initOutput?.memory.buffer?.byteLength;
  } catch {
    return undefined;
  }
}

function getLanguageFromFilename(filename: string) {
  const parts = filename.split(".");
  const extension = parts[parts.length - 1];
  const language = (
    {
      astro: "astro",
      svelte: "svelte",
    } as const
  )[extension];

  if (!language) {
    throw new Error(`unsupported file extension: .${extension}`);
  }

  return language;
}

it.concurrent.for([
  ["../../../examples/e2e-astro/src/layouts/Layout.astro", 2000],
  ["../../../fixtures/complex.astro", 2000],
  ["../../../examples/e2e-astro/src/components/SvelteCounter.svelte", 2000],
  ["../../../fixtures/complex.svelte", 2000],
] as const)(
  "wasm stress test: %s x %d",
  { timeout: 120_000 },
  async ([filepath, iterations], { expect }) => {
    const initOutput = initWasmOnce();

    const language = getLanguageFromFilename(filepath);

    const fileURL = new URL(filepath, import.meta.url);
    const input = await readFile(fileURL, "utf8");

    const { code: baselineCode, map: baselineMap } = stripWhitespace(
      input,
      filepath,
      language,
      {
        preserveBlankLines: false,
      },
    );
    expect(typeof baselineCode).toBe("string");
    expect(typeof baselineMap).toBe("string");

    for (let i = 0; i < iterations; i++) {
      let out;
      try {
        out = stripWhitespace(input, filepath, language, {
          preserveBlankLines: false,
        });
      } catch (error) {
        const memBytes = getWasmMemoryBytes(initOutput);
        throw new Error(
          [
            `wasm trap while stripping ${filepath} at iteration ${i}`,
            `input.length=${input.length}`,
            `wasm.memory.bytes=${memBytes ?? "unknown"}`,
            `error=${String((error as any)?.stack ?? error)}`,
          ].join("\n"),
        );
      }
      if (out.code !== baselineCode || out.map !== baselineMap) {
        throw new Error(
          `non-deterministic output for ${filepath} at iteration ${i}`,
        );
      }

      let out2;
      try {
        out2 = stripWhitespaceNoSourcemap(input, language, {
          preserveBlankLines: false,
        });
      } catch (error) {
        const memBytes = getWasmMemoryBytes(initOutput);
        throw new Error(
          [
            `wasm trap while stripping (no sourcemap) ${filepath} at iteration ${i}`,
            `input.length=${input.length}`,
            `wasm.memory.bytes=${memBytes ?? "unknown"}`,
            `error=${String((error as any)?.stack ?? error)}`,
          ].join("\n"),
        );
      }
      if (out2 !== baselineCode) {
        throw new Error(
          `non-deterministic output (no sourcemap) for ${filepath} at iteration ${i}`,
        );
      }
    }

    // Also exercise preserveBlankLines=true; we only care that it never traps.
    for (let i = 0; i < iterations; i++) {
      const out = stripWhitespace(input, filepath, language, {
        preserveBlankLines: true,
      });
      expect(typeof out.code).toBe("string");
      expect(typeof out.map).toBe("string");

      const out2 = stripWhitespaceNoSourcemap(input, language, {
        preserveBlankLines: true,
      });
      expect(typeof out2).toBe("string");
    }
  },
);
