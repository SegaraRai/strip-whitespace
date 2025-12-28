import { defineConfig } from "tsdown";

export default defineConfig({
  entry: ["src/index.ts", "src/vite.ts"],
  outDir: "dist",
  inputOptions: {
    moduleTypes: {
      ".wasm": "asset",
    },
  },
});
