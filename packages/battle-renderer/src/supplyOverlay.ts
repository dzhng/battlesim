// Supply for one side: each shown supply vehicle's reach, a thin ring of the
// orders' line weight with the overlay's glow, so it reads at any zoom. It
// is an extent, not a state: whether the truck is set up and supplying is
// its info panel's. Which vehicles show is the caller's.
import { concatMeshes, groundAnnulus, isRgba, MeshBuilder, type Rgba } from "./mesh";
import type { SurfaceHeight } from "./orderOverlay";
import type { WorldMeshes } from "./scene";

export interface SupplySource {
  center: readonly [number, number];
  radius: number;
}

/** `presentation.overlay.supply`: the colour of a truck's reach. */
export interface SupplyStyle {
  reach: Rgba;
}

export function validateSupplyStyle(style: SupplyStyle): SupplyStyle {
  if (!isRgba(style?.reach))
    throw new Error("presentation.overlay.supply: rgba in [0, 1] for reach");
  return style;
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
    groundAnnulus(mesh, s.center, s.radius - line / 2, s.radius + line / 2, {
      z,
      segments: 64,
      colorIn: style.reach,
    });
  // Painted on the ground (`frame/paintedMarks.ts`).
  const none = new Float32Array(0);
  return { opaque: none, translucent: none, painted: concatMeshes([mesh.build()]) };
}
