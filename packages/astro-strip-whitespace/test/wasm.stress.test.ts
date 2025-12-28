import {
  initSync,
  strip_whitespace_no_sourcemap,
  strip_whitespace_sourcemap_create,
} from "astro-strip-whitespace-wasm";
import { readFileSync } from "node:fs";
import { readFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { describe, expect, it } from "vitest";

let wasmInitialized = false;
let wasmExports: any | undefined;

function initWasmOnceForTest() {
  if (wasmInitialized) return;
  wasmInitialized = true;

  // Avoid importing `.wasm` as a module (Vite doesn't support that in Vitest by default).
  // Instead, resolve the actual wasm file path via Node and feed the bytes to `initSync`.
  const require = createRequire(import.meta.url);
  const wasmFilePath =
    require.resolve("astro-strip-whitespace-wasm/index_bg.wasm");
  const wasmBytes = readFileSync(wasmFilePath);
  wasmExports = initSync({ module: wasmBytes });
}

function getWasmMemoryBytes(): number | undefined {
  try {
    const mem = wasmExports?.memory;
    if (!mem) return undefined;
    return mem.buffer?.byteLength;
  } catch {
    return undefined;
  }
}

describe("wasm stress", () => {
  it("does not trap under repeated parsing", { timeout: 60_000 }, async () => {
    initWasmOnceForTest();

    const layoutUrl = new URL(
      "../../../examples/e2e/src/layouts/Layout.astro",
      import.meta.url
    );
    const complexUrl = new URL(
      "../../../fixtures/complex.astro",
      import.meta.url
    );

    const layout = await readFile(layoutUrl, "utf8");
    const complex = await readFile(complexUrl, "utf8");

    // These loops are intentionally high enough to catch the old wasm-only
    // instability (which used to trap after just a handful of iterations).
    for (const [name, input, iterations] of [
      ["Layout.astro", layout, 2000],
      ["complex.astro", complex, 750],
    ] as const) {
      const baseline = strip_whitespace_no_sourcemap(input, false);
      expect(typeof baseline).toBe("string");

      for (let i = 0; i < iterations; i++) {
        let out: string;
        try {
          out = strip_whitespace_no_sourcemap(input, false);
        } catch (error) {
          const memBytes = getWasmMemoryBytes();
          throw new Error(
            [
              `wasm trap while stripping ${name} at iteration ${i}`,
              `input.length=${input.length}`,
              `wasm.memory.bytes=${memBytes ?? "unknown"}`,
              `error=${String((error as any)?.stack ?? error)}`,
            ].join("\n")
          );
        }
        if (out !== baseline) {
          throw new Error(
            `non-deterministic output for ${name} at iteration ${i}`
          );
        }
      }

      // Also exercise preserveBlankLines=true; we only care that it never traps.
      for (let i = 0; i < Math.min(200, iterations); i++) {
        const out = strip_whitespace_no_sourcemap(input, true);
        expect(typeof out).toBe("string");
      }
    }

    // Exercise the wasm output object + free() path too.
    for (let i = 0; i < 250; i++) {
      const out = strip_whitespace_sourcemap_create(
        layout,
        "Layout.astro",
        false
      );
      try {
        expect(typeof out.code).toBe("string");
        expect(typeof out.sourcemap).toBe("string");
        // Ensure sourcemap is valid JSON.
        JSON.parse(out.sourcemap);
      } finally {
        out.free();
      }
    }
  });
});
