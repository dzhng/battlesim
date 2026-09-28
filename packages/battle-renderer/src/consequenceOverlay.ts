// Recent consequences of fire for one side, as marks: fading scorch rings
// where rounds struck. Positions come from observation. The fallen are the
// models layer's static corpses; a squad's suppression is its info panel's.
import { concatMeshes, groundAnnulus, isRgba, MeshBuilder, type Rgba } from "./mesh";
import type { SurfaceHeight } from "./orderOverlay";
import type { WorldMeshes } from "./scene";

type P3 = readonly [number, number, number];

export interface ImpactMark {
  at: P3;
  /** Remaining life in [0, 1]. */
  fade: number;
}

/** `presentation.overlay.consequences`: a strike's scorch ring (its alpha
 *  follows the fade). */
export interface ConsequenceStyle {
  impact: Rgba;
}

export function validateConsequenceStyle(style: ConsequenceStyle): ConsequenceStyle {
  if (!isRgba(style?.impact))
    throw new Error("presentation.overlay.consequences: rgba in [0, 1] for impact");
  return style;
}
const SEGMENTS = 32;
/** Marks lie on the ground, as all paint does: the ground reads the paint
 *  where it was drawn. Bodies are never painted, so a prone soldier or a
 *  corpse lies over them. */
/** Radial steps, so a low mark follows the ground rather than cutting into it. */
const RING_STEP_M = 1.5;

/** A flat disc or ring at ground level, alpha ramping from `a0` inside to `a1` outside. */
function disc(
  mesh: MeshBuilder,
  cx: number,
  cy: number,
  inner: number,
  outer: number,
  base: Rgba,
  a0: number,
  a1: number,
  z: SurfaceHeight,
) {
  const steps = Math.max(1, Math.ceil((outer - inner) / RING_STEP_M));
  const alpha = (k: number) => a0 + ((a1 - a0) * k) / steps;
  for (let k = 0; k < steps; k++)
    groundAnnulus(
      mesh,
      [cx, cy],
      inner + ((outer - inner) * k) / steps,
      inner + ((outer - inner) * (k + 1)) / steps,
      {
        z,
        segments: SEGMENTS,
        colorIn: [base[0], base[1], base[2], alpha(k)],
        colorOut: [base[0], base[1], base[2], alpha(k + 1)],
      },
    );
}

/** Recent strike marks. The fallen are the models layer's (static
 *  corpses), not marks. */
export function buildConsequenceOverlay(
  impacts: readonly ImpactMark[],
  z: SurfaceHeight,
  style: ConsequenceStyle,
): WorldMeshes {
  const mesh = new MeshBuilder();
  for (const i of impacts) {
    const a = 0.85 * i.fade;
    disc(mesh, i.at[0], i.at[1], 0.9, 1.6, style.impact, a, a, z);
  }
  // Painted on the ground (`frame/paintedMarks.ts`).
  const none = new Float32Array(0);
  return { opaque: none, translucent: none, painted: concatMeshes([mesh.build()]) };
}
