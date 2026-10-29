// The battle's biome, from its one owner, `fixtures/biomes/summer.json`.
// Every lab route draws its ground under it, as every lab reuses the
// game's look; the ground admits it in Rust (`ground::biome`).
import { useSyncExternalStore } from "react";
import summer from "@fixtures/biomes/summer.json";
import type { Biome } from "@packages/battle-renderer/src/terrain/terrainSurface";

/** `looks` with the row query parameter `param` names drawn everywhere
 *  (`?water_look=<row>` for every river, `?swamp_look=<row>` for every
 *  swamp): the other looks lose their weight, so Rust's one pick
 *  (`water::look`) draws that row. */
function pin<T extends { name: string; weight: number }>(looks: T[], param: string): T[] {
  const row =
    typeof window === "undefined" ? null : new URLSearchParams(window.location.search).get(param);
  if (row === null) return looks;
  const at = Number(row);
  if (!Number.isInteger(at) || at < 0 || at >= looks.length)
    throw new Error(
      `?${param}=${row}: the biome has looks 0 to ${looks.length - 1} (${looks.map((l) => l.name).join(", ")})`,
    );
  return looks.map((l, i) => ({ ...l, weight: i === at ? 1 : 0 }));
}

/** `biome` with its rivers' and its swamps' looks pinned (`pin`). */
function pinned(biome: Biome): Biome {
  const { rivers, swamps } = biome.water;
  return {
    ...biome,
    water: { ...biome.water, rivers: pin(rivers, "water_look"), swamps: pin(swamps, "swamp_look") },
  };
}

export const gameBiome: Biome = pinned(summer);

// On the dev server, an edit to the biome's file reaches a battle's ground
// in place (`useGameBiome`), with no shader rebuilt: its looks are uniforms.
let live = gameBiome;
const listeners = new Set<() => void>();
if (import.meta.hot)
  import.meta.hot.accept("@fixtures/biomes/summer.json", (next) => {
    if (!next) return;
    live = pinned(next.default as Biome);
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
