import { defineConfig } from "tsdown";

export default defineConfig({
  entry: ["src/*.ts"],
  inputOptions: {
    moduleTypes: {
      ".wasm": "asset",
    },
  },
  attw: {
    profile: "esm-only",
  },
});
