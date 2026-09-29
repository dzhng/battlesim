// Recent consequences of fire for one side, as marks (the labs'): fading
// scorch rings where rounds struck. Positions come from observation. The
// fallen are the models layer's static corpses; a squad's suppression is its
// info panel's.
import { groundAnnulus, MeshBuilder, paintOnly, rgbA, type Rgba } from "./mesh";
import type { SurfaceHeight } from "./orderOverlay";
import type { WorldMeshes } from "./scene";

type P3 = readonly [number, number, number];

interface ImpactMark {
  at: P3;
  /** Remaining life in [0, 1]. */
  fade: number;
}

/** Recent strike marks, each a scorch ring in `color` (its alpha follows
 *  the fade). The fallen are the models layer's (static corpses), not
 *  marks. */
export function buildConsequenceOverlay(
  impacts: readonly ImpactMark[],
  z: SurfaceHeight,
  color: Rgba,
): WorldMeshes {
  const mesh = new MeshBuilder();
  for (const i of impacts)
    groundAnnulus(mesh, [i.at[0], i.at[1]], 0.9, 1.6, {
      z,
      segments: 32,
      colorIn: rgbA(color, 0.85 * i.fade),
    });
  return paintOnly(mesh.build());
}
