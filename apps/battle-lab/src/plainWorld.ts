// Plain flat worlds: level ground in 5 m cells with nothing on it but its
// paved pads, a stage that shows only what is put on it (the showcase, the
// Armory's yard, the facade lab and the workbench's measured ground), with
// the land past it level or rising in hills. Plain data, with no module to
// load: the scene's presentation resolves it when it is set.
import { quat } from "math";
import { obb3, type OBB3 } from "math/shapes";
import { MeshBuilder, type Mesh, type Rgba } from "@packages/battle-renderer/src/mesh";
import type { WorldLayers } from "@packages/battle-renderer/src/scene";
import {
  bareTerrain,
  type BackdropReliefInput,
  type TerrainSurface,
} from "@packages/battle-renderer/src/terrain/terrainSurface";
import { gameBiome } from "./gameBiome";

/** How far a plain ground reaches each way, metres: as far as the battle
 *  views see. */
export const GROUND_REACH_M = 250;
/** A plain ground's colour, a neutral grey shadows read on. */
export const PLAIN_GROUND: Rgba = [0.46, 0.47, 0.44, 1];

const NONE = new Float32Array(0);

/** A paved rectangle on a plain ground: its centre, its half sides, its turn
 *  (radians counter-clockwise from +X) and its colour. */
export interface GroundPad {
  at: [number, number];
  half_m: [number, number];
  yaw: number;
  tint: Rgba;
}

/** How far a pad stands above the ground or the pads it lies on, metres:
 *  enough to draw over them, too little to see. */
const PAD_LIFT_M = 0.005;

/** Each pad's level: one above the highest earlier pad it lies over, so a
 *  later pad draws over an earlier one and pads side by side stay level. */
function padLevels(pads: readonly GroundPad[]): number[] {
  const boxes: OBB3[] = pads.map((p) =>
    obb3.setFromCenterHalfExtentsQuaternion(
      obb3.create(),
      [p.at[0], p.at[1], 0],
      [p.half_m[0], p.half_m[1], 1],
      quat.setAxisAngle(quat.create(), [0, 0, 1], p.yaw),
    ),
  );
  const levels: number[] = [];
  boxes.forEach((box, i) => {
    let under = 0;
    for (let j = 0; j < i; j++)
      if (levels[j] > under && obb3.intersectsOBB3(box, boxes[j])) under = levels[j];
    levels.push(under + 1);
  });
  return levels;
}

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
 *  it, and `relief` past it (level land without it): its vertex tints are
 *  opaque, so it draws as tinted, not as the biome's patchwork. */
export function flatTerrain(
  ground: Mesh,
  reach = GROUND_REACH_M,
  relief?: BackdropReliefInput,
): TerrainSurface {
  const map = [-reach, -reach, reach, reach] as const;
  return bareTerrain(ground, map, 2 * reach, gameBiome, {}, relief);
}

/** A world of plain ground in `tint` out to `reach` each way, paved with
 *  `pads`, and nothing else; past it, the scene's presentation draws the
 *  backdrop over `relief` (level without it). */
export function plainWorld(
  tint: Rgba = PLAIN_GROUND,
  reach = GROUND_REACH_M,
  pads: readonly GroundPad[] = [],
  relief?: BackdropReliefInput,
): WorldLayers {
  const mesh = new MeshBuilder();
  // One colour: the cells only keep the mesh to a hundred a side.
  flatCells(mesh, reach, () => tint, Math.max(5, (2 * reach) / 100));
  const levels = padLevels(pads);
  pads.forEach((pad, i) => {
    const [c, s] = [Math.cos(pad.yaw), Math.sin(pad.yaw)];
    const z = PAD_LIFT_M * levels[i];
    const corner = (u: number, v: number): [number, number, number] => {
      const [x, y] = [u * pad.half_m[0], v * pad.half_m[1]];
      return [pad.at[0] + c * x - s * y, pad.at[1] + s * x + c * y, z];
    };
    mesh.quad(corner(-1, -1), corner(1, -1), corner(1, 1), corner(-1, 1), pad.tint);
  });
  return {
    terrain: flatTerrain(mesh.build(), reach, relief),
    extraFaces: NONE,
    scenery: null,
    grass: null,
  };
}
