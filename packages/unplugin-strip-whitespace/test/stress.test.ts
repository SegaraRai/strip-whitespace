import {
  initWasmOnce,
  strip_whitespace,
  strip_whitespace_no_sourcemap,
  type InitOutput,
} from "#wasm";
import { readFile } from "node:fs/promises";
import { describe, expect, it } from "vitest";

function getWasmMemoryBytes(initOutput: InitOutput): number | undefined {
  try {
    return initOutput?.memory.buffer?.byteLength;
  } catch {
    return undefined;
  }
}

describe("wasm stress", () => {
  it("does not trap under repeated parsing", { timeout: 60_000 }, async () => {
    const initOutput = initWasmOnce();

    const layoutUrl = new URL(
      "../../../examples/e2e-astro/src/layouts/Layout.astro",
      import.meta.url,
    );
    const complexUrl = new URL(
      "../../../fixtures/complex.astro",
      import.meta.url,
    );

    const layout = await readFile(layoutUrl, "utf8");
    const complex = await readFile(complexUrl, "utf8");

    // These loops are intentionally high enough to catch the old wasm-only
    // instability (which used to trap after just a handful of iterations).
    for (const [name, input, iterations] of [
      ["Layout.astro", layout, 2000],
      ["complex.astro", complex, 750],
    ] as const) {
      const baseline = strip_whitespace_no_sourcemap(input, {
        preserve_blank_lines: false,
      });
      expect(typeof baseline).toBe("string");

      for (let i = 0; i < iterations; i++) {
        let out: string;
        try {
          out = strip_whitespace_no_sourcemap(input, {
            preserve_blank_lines: false,
          });
        } catch (error) {
          const memBytes = getWasmMemoryBytes(initOutput);
          throw new Error(
            [
              `wasm trap while stripping ${name} at iteration ${i}`,
              `input.length=${input.length}`,
              `wasm.memory.bytes=${memBytes ?? "unknown"}`,
              `error=${String((error as any)?.stack ?? error)}`,
            ].join("\n"),
          );
        }
        if (out !== baseline) {
          throw new Error(
            `non-deterministic output for ${name} at iteration ${i}`,
          );
        }
      }

      // Also exercise preserveBlankLines=true; we only care that it never traps.
      for (let i = 0; i < Math.min(200, iterations); i++) {
        const out = strip_whitespace_no_sourcemap(input, {
          preserve_blank_lines: true,
        });
        expect(typeof out).toBe("string");
      }
    }

    // Exercise the wasm output object + free() path too.
    for (let i = 0; i < 250; i++) {
      const out = strip_whitespace(layout, "Layout.astro", {
        preserve_blank_lines: false,
      });

      expect(typeof out.code).toBe("string");
      expect(typeof out.sourcemap).toBe("string");
      // Ensure sourcemap is valid JSON.
      JSON.parse(out.sourcemap);
    }
  });
});
