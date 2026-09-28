// Supply for one side: each shown supply vehicle's service radius, a solid
// ring once it is fully deployed and can serve, faint and dashed while it
// cannot. Every ring is a thin line with the overlay's glow, so it reads at
// any zoom. Which vehicles show is the caller's: the selected ones.
import { concatMeshes, groundAnnulus, isRgba, MeshBuilder, type Rgba } from "./mesh";
import type { SurfaceHeight } from "./orderOverlay";
import type { WorldMeshes } from "./scene";

export interface SupplySource {
  center: readonly [number, number];
  radius: number;
  ready: boolean;
}

/** `presentation.overlay.supply`: a truck's reach, set up (`ready`) or not
 *  (`idle`). Hues kept apart from the deployment ring's green and orange. */
export interface SupplyStyle {
  ready: Rgba;
  idle: Rgba;
}

export function validateSupplyStyle(style: SupplyStyle): SupplyStyle {
  if (![style?.ready, style?.idle].every(isRgba))
    throw new Error("presentation.overlay.supply: rgba in [0, 1] for ready, idle");
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
  const opaque = new MeshBuilder();
  const translucent = new MeshBuilder();
  for (const s of sources) {
    const [mesh, color, dashed] = s.ready
      ? [opaque, style.ready, false]
      : [translucent, style.idle, true];
    groundAnnulus(mesh, s.center, s.radius - line / 2, s.radius + line / 2, {
      z,
      segments: 64,
      colorIn: color,
      dashed,
    });
  }
  // Painted on the ground (`frame/paintedMarks.ts`).
  const none = new Float32Array(0);
  return {
    opaque: none,
    translucent: none,
    painted: concatMeshes([opaque.build(), translucent.build()]),
  };
}
