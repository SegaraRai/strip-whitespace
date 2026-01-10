import { initSync, type InitOutput } from "strip-whitespace-wasm";
import wasmPath from "strip-whitespace-wasm/wasm";
import { readFileSync } from "node:fs";

export * from "strip-whitespace-wasm";

let initOutput: InitOutput | undefined;

export function initWasmOnce(): InitOutput {
  if (!initOutput) {
    const wasm = readFileSync(new URL(wasmPath, import.meta.url));
    initOutput = initSync({ module: wasm });
  }
  return initOutput;
}
