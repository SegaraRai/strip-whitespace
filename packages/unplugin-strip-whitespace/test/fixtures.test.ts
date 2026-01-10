import { initWasmOnce, stripWhitespace } from "#wasm";
import { readdirSync } from "node:fs";
import { readFile } from "node:fs/promises";
import { beforeAll, it } from "vitest";

const fixturesDir = new URL("../../../fixtures/", import.meta.url);

const fixtures = readdirSync(fixturesDir)
  .filter((filename) => !filename.includes(".out."))
  .toSorted()
  .map((filename) => [filename] as const);

beforeAll(() => {
  initWasmOnce();
});

it.for(fixtures)(
  "transforms fixture: %s",
  async ([fixtureFilename], { expect }) => {
    const [, fixture, extension] =
      /^(.+?)(\.[^.]*)$/.exec(fixtureFilename) ?? [];
    if (!fixture || !extension) {
      throw new Error(`Invalid fixture filename: ${fixtureFilename}`);
    }

    const language = ({ ".astro": "astro", ".svelte": "svelte" } as const)[
      extension
    ];
    if (!language) {
      throw new Error(`Unsupported fixture extension: ${extension}`);
    }

    const expectedCode = await readFile(
      new URL(`${fixture}.out${extension}`, fixturesDir),
      "utf-8",
    );
    const expectedMap = await readFile(
      new URL(`${fixture}.out${extension}.map`, fixturesDir),
      "utf-8",
    );

    const input = await readFile(
      new URL(fixtureFilename, fixturesDir),
      "utf-8",
    );

    const result = stripWhitespace(input, fixtureFilename, language, {
      preserveBlankLines: false,
    });

    expect(result.code).toBe(expectedCode);
    expect(result.map).toBe(expectedMap);
  },
);
