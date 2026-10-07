// @vitest-environment node
// A presentation table keyed by weapon row: a row of its own, else its
// nearest ancestor's, else the default.
import { expect, test } from "vitest";
import { inheritRows, pick } from "@packages/renderer-core/src/kindTable.ts";

test("a weapon without a row of its own takes its nearest ancestor's", () => {
  const weapons = {
    rifle: {},
    carbine: { extends: "rifle" },
    // Two steps from a styled row.
    short_carbine: { extends: "carbine" },
    cannon: {},
    // Its own row wins over its parent's.
    sabot: { extends: "cannon" },
    // No styled ancestor at all.
    flare: { extends: "signal" },
    signal: {},
  };
  const table = inheritRows(
    { default: "plain", rifle: "rifle look", cannon: "cannon look", sabot: "sabot look" },
    weapons,
  );
  expect(pick(table, "carbine")).toBe("rifle look");
  expect(pick(table, "short_carbine")).toBe("rifle look");
  expect(pick(table, "sabot")).toBe("sabot look");
  expect(pick(table, "flare")).toBe("plain");
  expect(pick(table, "unheard_of")).toBe("plain");
});
