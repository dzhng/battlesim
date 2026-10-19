// @vitest-environment node
// The browser's size rule asks the map generator for as many entry roads a
// side as the simulation will seat; the two are separate implementations of
// one tier table, so they are checked against each other through the
// simulation's own skirmish setup.
import { readFileSync } from "node:fs";
import { beforeAll, expect, test } from "vitest";
import * as wasm from "@wasm/game_wasm.js";
import {
  entryCountForPlayers,
  MAX_PLAYERS_PER_SIDE,
  playersFor,
} from "@apps/battle-lab/src/skirmishSetup";

beforeAll(() => {
  const module = wasm.initSync({
    module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)),
  });
  // Keep the initialized module alive for this file's calls.
  expect(module.memory).toBeDefined();
});

const counts = Array.from({ length: MAX_PLAYERS_PER_SIDE }, (_, index) => index + 1);

test.each(counts.flatMap((count) => [[count, count] as const, [count, 1] as const]))(
  "%sv%s: the browser's entry count is the simulation's entries a side",
  (blueCount, redCount) => {
    const players = playersFor(blueCount, redCount, null);
    const setup = JSON.parse(
      wasm.synthetic_skirmish_setup(JSON.stringify([3000, 3000]), JSON.stringify(players)),
    ) as { sites: { entries: { side: "blue" | "red" }[] } };
    for (const side of ["blue", "red"] as const)
      expect(setup.sites.entries.filter((entry) => entry.side === side)).toHaveLength(
        entryCountForPlayers(players),
      );
  },
);
