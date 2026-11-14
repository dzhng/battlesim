// A battle's world layers: its ground under the game's biome as the dev
// server last read it (an edit to its looks reaches the ground in place),
// built once per world, and the art its scenery and grass draw from, which a
// new art generation (a newly fielded kind's) changes without a new terrain.
import { useMemo } from "react";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader";
import { buildWorldGround, withWorldArt } from "@packages/battle-renderer/src/worldMesh";
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
  const ground = useMemo(
    () => world && buildWorldGround(world.exports, world.layout, biome, "surface"),
    [world, biome],
  );
  return useMemo(
    () => ground && appearances && withWorldArt(ground, biome, appearances, scenery !== null),
    [ground, appearances, scenery, biome],
  );
}
