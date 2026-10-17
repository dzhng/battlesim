// @vitest-environment node
import { expect, test } from "vitest";
import { spoken } from "../../apps/battle-lab/src/battleLinks";

test("size labels use the lobby's S/M/L/XL vocabulary", () => {
  expect(["small", "medium", "large", "xl"].map(spoken)).toEqual(["S", "M", "L", "XL"]);
});
