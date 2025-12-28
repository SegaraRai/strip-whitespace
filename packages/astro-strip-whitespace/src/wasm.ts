import { initSync } from "astro-strip-whitespace-wasm";
import wasmPath from "astro-strip-whitespace-wasm/index_bg.wasm";
import { readFileSync } from "node:fs";

export * from "astro-strip-whitespace-wasm";

export function initWasmOnce() {
  if ((initWasmOnce as any).initialized) {
    return;
  }

  (initWasmOnce as any).initialized = true;
  const wasm = readFileSync(new URL(wasmPath, import.meta.url));
  initSync({ module: wasm });
}
