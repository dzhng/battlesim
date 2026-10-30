// @vitest-environment node
import { readFileSync } from "node:fs";
import { expect, test } from "vitest";
import { initSync, sight_multiplier_vectors } from "@wasm/game_wasm.js";
import { sightMultiplier } from "@packages/battle-renderer/src/sightOverlay";

initSync({ module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)) });

test("the TypeScript sight shape matches sim::sight::multiplier's oracle vectors", () => {
  const rows = sight_multiplier_vectors();
  expect(rows.length).toBeGreaterThan(500);
  for (let i = 0; i < rows.length; i += 5) {
    const [front, side, rear, off, m] = rows.subarray(i, i + 5);
    expect(sightMultiplier({ front, side, rear }, off)).toBeCloseTo(m, 12);
  }
});
