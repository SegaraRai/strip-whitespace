import { defineConfig } from "tsdown";

export default defineConfig({
  entry: ["src/*"],
  attw: {
    profile: "esm-only",
  },
});
