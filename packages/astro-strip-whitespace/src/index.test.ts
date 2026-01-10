import { test } from "vitest";

test("index must have a default export", async ({ expect }) => {
  const mod = await import("./index.js");
  if (!mod?.default) {
    throw new Error("No default export found in index module");
  }

  expect(mod.default().name).toBe("astro-strip-whitespace");
});
