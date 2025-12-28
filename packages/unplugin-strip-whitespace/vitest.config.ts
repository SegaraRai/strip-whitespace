import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "node",
    isolate: true,
  },
  resolve: {
    // Vite(st) does not seem to support conditional imports yet?
    // In the future we should use `resolve.conditions` instead.
    alias: {
      "#wasm": "./src/internal/wasm-vite.ts",
    },
  },
});
