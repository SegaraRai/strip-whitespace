import { initWasmOnce, strip_whitespace } from "#wasm";
import { readdirSync } from "node:fs";
import { readFile } from "node:fs/promises";
import { beforeAll, expect, it } from "vitest";

const fixturesDir = new URL("../../../fixtures/", import.meta.url);

const fixtures = readdirSync(fixturesDir)
  .filter((filename) => !filename.includes(".out."))
  .toSorted();

beforeAll(() => {
  initWasmOnce();
});

it.each(fixtures)("transforms fixture: %s", async (fixtureFilename) => {
  const [, fixture, extension] = /^(.+?)(\.[^.]*)$/.exec(fixtureFilename) ?? [];
  if (!fixture || !extension) {
    throw new Error(`Invalid fixture filename: ${fixtureFilename}`);
  }

  const expectedCode = await readFile(
    new URL(`${fixture}.out${extension}`, fixturesDir),
    "utf-8",
  );
  const expectedMap = await readFile(
    new URL(`${fixture}.out${extension}.map`, fixturesDir),
    "utf-8",
  );

  const input = await readFile(new URL(fixtureFilename, fixturesDir), "utf-8");

  const result = strip_whitespace(input, fixtureFilename, {
    preserve_blank_lines: false,
  });

  expect(result.code).toBe(expectedCode);
  expect(result.sourcemap).toBe(expectedMap);
});
