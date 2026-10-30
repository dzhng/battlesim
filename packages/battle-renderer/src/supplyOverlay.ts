// Supply for one side: each shown supply vehicle's reach, a thin ring of the
// orders' line weight with the overlay's glow, so it reads at any zoom. It
// is an extent, not a state: whether the truck is set up and supplying is
// its info panel's. Which vehicles show is the caller's.
import { groundRing, MeshBuilder, paintOnly, type Rgba } from "./mesh";
import type { SurfaceHeight } from "./orderOverlay";
import type { WorldMeshes } from "./scene";

interface SupplySource {
  center: readonly [number, number];
  radius: number;
}

/** `presentation.overlay.supply`: the colour of a truck's reach. */
export interface SupplyStyle {
  reach: Rgba;
}

/** Rings `line` metres wide (the orders' line weight), glowing by the
 *  overlay's own halo rather than on a dark band. */
export function buildSupplyOverlay(
  sources: readonly SupplySource[],
  z: SurfaceHeight,
  line: number,
  style: SupplyStyle,
): WorldMeshes {
  const mesh = new MeshBuilder();
  for (const s of sources)
    groundRing(mesh, s.center, s.radius, line, style.reach, { z, segments: 64 });
  return paintOnly(mesh.build());
}
