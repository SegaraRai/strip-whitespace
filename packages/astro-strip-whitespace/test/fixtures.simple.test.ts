import { expect, it } from "vitest";
import { transformFixture } from "./_helpers";

it("transforms fixture: simple", async () => {
  const { result, expectedCode, expectedMap } =
    await transformFixture("simple");
  expect(result).toBeTruthy();
  expect((result as any).code).toBe(expectedCode);
  expect((result as any).map).toEqual(expectedMap);
});
