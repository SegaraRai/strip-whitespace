import { expect, it } from "vitest";
import { transformFixture } from "./_helpers";

it("transforms fixture: unicode", async () => {
  const { result, expectedCode, expectedMap } =
    await transformFixture("unicode");
  expect(result).toBeTruthy();
  expect((result as any).code).toBe(expectedCode);
  expect((result as any).map).toEqual(expectedMap);
});
