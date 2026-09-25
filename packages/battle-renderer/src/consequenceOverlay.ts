// Lasting and recent consequences of fire for one side: fallen soldiers lying
// where they fell (own and enemy tinted apart, each on a pale ground mark so
// they read on grass and under trees), fading scorch rings where rounds
// struck, and a halo under each suppressed squad whose strength follows its
// suppression. Positions come from observation.
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

export const OWN_FALLEN: Rgba = [0.42, 0.62, 1.0, 1];
export const ENEMY_FALLEN: Rgba = [1.0, 0.4, 0.34, 1];
const FALLEN_MARK: Rgba = [0.92, 0.9, 0.84, 0.55];
export const SUPPRESSION: Rgba = [1.0, 0.62, 0.1, 1];
export const IMPACT: Rgba = [1.0, 0.86, 0.35, 1];
const SEGMENTS = 32;
const LIFT_M = 0.3;

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
  groundAnnulus(mesh, [cx, cy], inner, outer, {
    z,
    lift: LIFT_M,
    segments: SEGMENTS,
    colorIn: [base[0], base[1], base[2], a0],
    colorOut: [base[0], base[1], base[2], a1],
  });
}

export function buildConsequenceOverlay(
  corpses: readonly { position: P3; own: boolean }[],
  suppressed: readonly SuppressedSquad[],
  impacts: readonly ImpactMark[],
  z: SurfaceHeight,
): WorldMeshes {
  const opaque = new MeshBuilder();
  const translucent = new MeshBuilder();
  for (const c of corpses) {
    // Lying down: a long, low body on a pale mark.
    const [x, y, pz] = c.position;
    disc(translucent, x, y, 0, 1.6, FALLEN_MARK, FALLEN_MARK[3], FALLEN_MARK[3], z);
    opaque.box(x, y, pz + 0.2, 0.85, 0.3, 0.2, c.own ? OWN_FALLEN : ENEMY_FALLEN);
  }
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
