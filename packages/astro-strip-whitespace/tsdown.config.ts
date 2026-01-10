import { defineConfig } from "tsdown";

export default defineConfig({
  entry: ["src/*", "!**/*.test.ts"],
  attw: {
    profile: "esm-only",
  },
});
