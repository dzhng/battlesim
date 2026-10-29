// A battle's world layers (`buildWorldLayers`): its ground under the game's
// biome as the dev server last read it (an edit to its looks reaches the
// ground in place), and the art its scenery and grass draw from.
import { useMemo } from "react";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import { buildWorldLayers } from "@packages/battle-renderer/src/worldMesh";
import { useGameBiome } from "./gameBiome";
import type { StaticWorld } from "./useStaticWorld";

/** `world`'s layers once its `appearances` are in; its scenery drawn unless
 *  `scenery` is null (a scenario with none). */
export function useWorldLayers(
  world: StaticWorld | null,
  appearances: InstalledAppearances | null,
  scenery: ReadonlySet<string> | null | undefined,
) {
  const biome = useGameBiome();
  return useMemo(
    () =>
      world &&
      appearances &&
      buildWorldLayers(
        world.exports,
        world.layout,
        biome,
        "surface",
        appearances,
        scenery !== null,
      ),
    [world, appearances, scenery, biome],
  );
}
