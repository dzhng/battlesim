// @vitest-environment node
import { readFileSync } from "node:fs";
import { beforeAll, expect, test } from "vitest";
import * as wasm from "@wasm/game_wasm.js";
import { builderEconomy } from "@apps/battle-lab/src/scenario-builder/model";
import { playersFor } from "@apps/battle-lab/src/skirmishSetup";

const size = [3000, 3000] as const;
let syntheticSites: (
  size: readonly [number, number],
  entries: number,
) => {
  side: "blue" | "red";
  center: [number, number];
  yaw: number;
}[];

beforeAll(() => {
  const module = wasm.initSync({
    module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)),
  });
  syntheticSites = (mapSize, entries) =>
    JSON.parse(wasm.synthetic_sites(JSON.stringify(mapSize), entries)).entries;
  // Keep the initialized module alive for this file's calls.
  expect(module.memory).toBeDefined();
});

test.each([
  [1, 1],
  [2, 2],
  [3, 3],
] as const)(
  "builder economy uses Rust synthetic sites centre-first for %s entries",
  (count, entries) => {
    const players = playersFor(count, count);
    const builder = builderEconomy(size, players, syntheticSites);
    expect(builder.sites.entries).toEqual(syntheticSites(size, entries));
    expect(builder.sites.entries.filter((entry) => entry.side === "blue")[0]).toEqual(
      builder.sites.entries[0],
    );
  },
);
