// The battle's biome, from its one owner, `fixtures/biomes/summer.json`.
// Every lab route draws its ground under it, as every lab reuses the
// game's look; the ground admits it in Rust (`ground::biome`).
import { useSyncExternalStore } from "react";
import summer from "@fixtures/biomes/summer.json";
import type { Biome } from "@packages/battle-renderer/src/terrain/terrainSurface";

/** `biome` with every river drawing look `row` (`?water_look=<row>`): the
 *  other looks lose their weight, so Rust's one pick (`water::look`) draws
 *  that row everywhere. */
function pinned(biome: Biome, row: string | null): Biome {
  if (row === null) return biome;
  const rivers = biome.water.rivers;
  const at = Number(row);
  if (!Number.isInteger(at) || at < 0 || at >= rivers.length)
    throw new Error(
      `?water_look=${row}: the biome's rivers have looks 0 to ${rivers.length - 1} (${rivers.map((l) => l.name).join(", ")})`,
    );
  const water = {
    ...biome.water,
    rivers: rivers.map((l, i) => ({ ...l, weight: i === at ? 1 : 0 })),
  };
  return { ...biome, water };
}

const look = () =>
  typeof window === "undefined"
    ? null
    : new URLSearchParams(window.location.search).get("water_look");

export const gameBiome: Biome = pinned(summer, look());

// On the dev server, an edit to the biome's file reaches a battle's ground
// in place (`useGameBiome`), with no shader rebuilt: its looks are uniforms.
let live = gameBiome;
const listeners = new Set<() => void>();
if (import.meta.hot)
  import.meta.hot.accept("@fixtures/biomes/summer.json", (next) => {
    if (!next) return;
    live = pinned(next.default as Biome, look());
    for (const listener of listeners) listener();
  });

/** The biome as the dev server last read its file: `gameBiome` until it
 *  changes. */
export function useGameBiome(): Biome {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    () => live,
  );
}
