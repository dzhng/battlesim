// Plain flat worlds: level ground in 5 m cells with nothing on it, a stage
// that shows only what is put on it (the showcase, the Armory's yard, the
// facade lab and the workbench's measured ground). Plain data, with no
// module to load: the scene's presentation resolves it when it is set.
import { MeshBuilder, type Mesh, type Rgba } from "@packages/battle-renderer/src/mesh";
import type { WorldLayers } from "@packages/battle-renderer/src/scene";
import {
  bareTerrain,
  type TerrainSurface,
} from "@packages/battle-renderer/src/terrain/terrainSurface";
import { gameBiome } from "./gameBiome";

/** How far a plain ground reaches each way, metres: as far as the battle
 *  views see. */
export const GROUND_REACH_M = 250;
/** A plain ground's colour, a neutral grey shadows read on. */
export const PLAIN_GROUND: Rgba = [0.46, 0.47, 0.44, 1];

const NONE = new Float32Array(0);

/** Ground in `cell`-metre cells out to `reach` each way, each coloured by
 *  its corner. */
export function flatCells(
  mesh: MeshBuilder,
  reach: number,
  color: (x: number, y: number) => Rgba,
  cell = 5,
) {
  for (let x = -reach; x < reach; x += cell)
    for (let y = -reach; y < reach; y += cell)
      mesh.quad(
        [x, y, 0],
        [x + cell, y, 0],
        [x + cell, y + cell, 0],
        [x, y + cell, 0],
        color(x, y),
      );
}

/** A flat ground mesh reaching `reach` each way as terrain with nothing on
 *  it: its vertex tints are opaque, so it draws as tinted, not as the
 *  biome's patchwork. */
export function flatTerrain(ground: Mesh, reach = GROUND_REACH_M): TerrainSurface {
  return bareTerrain(ground, [-reach, -reach, reach, reach], 2 * reach, gameBiome);
}

/** A world of plain ground in `tint` out to `reach` each way, and nothing
 *  else. */
export function plainWorld(tint: Rgba = PLAIN_GROUND, reach = GROUND_REACH_M): WorldLayers {
  const mesh = new MeshBuilder();
  // One colour: the cells only keep the mesh to a hundred a side.
  flatCells(mesh, reach, () => tint, Math.max(5, (2 * reach) / 100));
  return {
    terrain: flatTerrain(mesh.build(), reach),
    extraFaces: NONE,
    scenery: null,
    grass: null,
  };
}
