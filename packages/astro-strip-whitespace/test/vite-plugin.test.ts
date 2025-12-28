import { viteStripWhitespacePlugin } from "astro-strip-whitespace/vite";
import { describe, expect, it } from "vitest";
import { getHookHandler } from "./_helpers";

describe("viteStripWhitespacePlugin", () => {
  it("skips non-Astro ids and query ids", async () => {
    const plugin = viteStripWhitespacePlugin();

    const transform = getHookHandler<
      (code: string, id: string, options?: { ssr?: boolean }) => any
    >(plugin.transform);

    expect(
      await transform?.call(undefined as any, "<div />", "file.ts")
    ).toBeUndefined();
    expect(
      await transform?.call(undefined as any, "<div />", "file.astro?raw")
    ).toBeUndefined();
  });

  it("reorders before astro:build on configResolved", async () => {
    const plugin = viteStripWhitespacePlugin();

    const plugins: any[] = [{ name: "astro:build" }, plugin];

    const configResolved = getHookHandler<(config: any) => any>(
      plugin.configResolved
    );
    configResolved?.({ plugins } as any);

    const astroBuildIdx = plugins.findIndex((p) => p?.name === "astro:build");
    const selfIdx = plugins.findIndex(
      (p) => p?.name === "astro-strip-whitespace"
    );

    expect(selfIdx).toBeGreaterThanOrEqual(0);
    expect(astroBuildIdx).toBeGreaterThanOrEqual(0);
    expect(selfIdx).toBeLessThan(astroBuildIdx);
  });
});
