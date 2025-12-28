import { viteStripWhitespacePlugin } from "astro-strip-whitespace/vite";
import { readFile } from "node:fs/promises";

export function getHookHandler<T extends (...args: any[]) => any>(
  hook: unknown
): T | undefined {
  if (!hook) return undefined;
  if (typeof hook === "function") return hook as T;
  if (typeof hook === "object" && hook !== null && "handler" in hook) {
    const maybeHandler = (hook as any).handler;
    if (typeof maybeHandler === "function") return maybeHandler as T;
  }
  return undefined;
}

export async function loadFixture(name: string) {
  const fixturesDir = new URL("../../../fixtures/", import.meta.url);

  const input = await readFile(new URL(`${name}.astro`, fixturesDir), "utf8");
  const expectedCode = await readFile(
    new URL(`${name}.out.astro`, fixturesDir),
    "utf8"
  );
  const expectedMapRaw = await readFile(
    new URL(`${name}.out.astro.map`, fixturesDir),
    "utf8"
  );

  return {
    input,
    expectedCode,
    expectedMap: JSON.parse(expectedMapRaw),
  };
}

export async function transformFixture(name: string) {
  const { input, expectedCode, expectedMap } = await loadFixture(name);

  const plugin = viteStripWhitespacePlugin();

  const transform = getHookHandler<
    (code: string, id: string, options?: { ssr?: boolean }) => any
  >(plugin.transform);

  const result = await transform?.call(
    undefined as any,
    input,
    `${name}.astro`
  );
  return { result, expectedCode, expectedMap };
}
