// Recent consequences of fire for one side, as marks: fading scorch rings
// where rounds struck, and a halo under each suppressed squad whose strength
// follows its suppression. Positions come from observation. The fallen are
// the models layer's static corpses.
import { groundAnnulus, MeshBuilder, type Rgba } from "./mesh";
import type { SurfaceHeight } from "./orderOverlay";
import type { WorldMeshes } from "./scene";

type P3 = readonly [number, number, number];

export interface SuppressedSquad {
  center: readonly [number, number];
  radius: number;
  /** In [0, 1]. */
  level: number;
}

export interface ImpactMark {
  at: P3;
  /** Remaining life in [0, 1]. */
  fade: number;
}

export const SUPPRESSION: Rgba = [1.0, 0.62, 0.1, 1];
export const IMPACT: Rgba = [1.0, 0.86, 0.35, 1];
const SEGMENTS = 32;
/** Marks lie this close to the ground, under a prone soldier or a corpse
 *  (about 0.3 m tall), which draw over them. */
const LIFT_M = 0.06;
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
        lift: LIFT_M,
        segments: SEGMENTS,
        colorIn: [base[0], base[1], base[2], alpha(k)],
        colorOut: [base[0], base[1], base[2], alpha(k + 1)],
      },
    );
}

/** Halos under suppressed squads and recent strike marks. The fallen are the
 *  models layer's (static corpses), not marks. */
export function buildConsequenceOverlay(
  suppressed: readonly SuppressedSquad[],
  impacts: readonly ImpactMark[],
  z: SurfaceHeight,
): WorldMeshes {
  const opaque = new MeshBuilder();
  const translucent = new MeshBuilder();
  for (const s of suppressed) {
    if (s.level <= 0) continue;
    const alpha = 0.3 + 0.5 * s.level;
    disc(translucent, s.center[0], s.center[1], 0, s.radius, SUPPRESSION, alpha, alpha * 0.4, z);
    disc(translucent, s.center[0], s.center[1], s.radius, s.radius + 0.8, SUPPRESSION, 0.9, 0.9, z);
  }
  for (const i of impacts) {
    const a = 0.85 * i.fade;
    disc(translucent, i.at[0], i.at[1], 0.9, 1.6, IMPACT, a, a, z);
  }
  return { opaque: opaque.build(), translucent: translucent.build() };
}
