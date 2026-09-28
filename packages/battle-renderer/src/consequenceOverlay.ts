// Recent consequences of fire for one side, as marks: fading scorch rings
// where rounds struck, and a halo under each suppressed squad whose strength
// follows its suppression. Positions come from observation. The fallen are
// the models layer's static corpses.
import { concatMeshes, groundAnnulus, isRgba, MeshBuilder, type Rgba } from "./mesh";
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

/** `presentation.overlay.consequences`: the halo under a suppressed squad
 *  and a strike's scorch ring (their alphas follow the level and the fade). */
export interface ConsequenceStyle {
  suppression: Rgba;
  impact: Rgba;
}

export function validateConsequenceStyle(style: ConsequenceStyle): ConsequenceStyle {
  if (![style?.suppression, style?.impact].every(isRgba))
    throw new Error("presentation.overlay.consequences: rgba in [0, 1] for suppression, impact");
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

/** Halos under suppressed squads and recent strike marks. The fallen are the
 *  models layer's (static corpses), not marks. A halo is a ring `line`
 *  metres wide (the orders' line weight) over a faint wash toward its rim,
 *  both stronger as the squad is more suppressed (a projection, not a
 *  painted disc). */
export function buildConsequenceOverlay(
  suppressed: readonly SuppressedSquad[],
  impacts: readonly ImpactMark[],
  z: SurfaceHeight,
  line: number,
  style: ConsequenceStyle,
): WorldMeshes {
  const opaque = new MeshBuilder();
  const translucent = new MeshBuilder();
  for (const s of suppressed) {
    if (s.level <= 0) continue;
    const wash = 0.04 + 0.2 * s.level;
    disc(
      translucent,
      s.center[0],
      s.center[1],
      0,
      s.radius,
      style.suppression,
      wash * 0.3,
      wash,
      z,
    );
    const rim = 0.45 + 0.55 * s.level;
    disc(
      translucent,
      s.center[0],
      s.center[1],
      s.radius,
      s.radius + line,
      style.suppression,
      rim,
      rim,
      z,
    );
  }
  for (const i of impacts) {
    const a = 0.85 * i.fade;
    disc(translucent, i.at[0], i.at[1], 0.9, 1.6, style.impact, a, a, z);
  }
  // Painted on the ground (`frame/paintedMarks.ts`).
  const none = new Float32Array(0);
  return {
    opaque: none,
    translucent: none,
    painted: concatMeshes([opaque.build(), translucent.build()]),
  };
}
