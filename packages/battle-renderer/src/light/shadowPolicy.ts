// Adapted from ~/dev/game game-renderer/src/battle/shadowPolicy.ts (reuse
// manifest): the sun shadow's quality constants, PCF radius and light basis.
// Local changes: four cascades capped at 2,600 m; the single fitted map
// (`SingleShadowPolicy`, `viewShadowFit`) is not ported.
import { type Vec3 } from "@packages/renderer-core/src/camera3d";

export type SunShadowMode = "csm" | "single" | "off";
// Four cascades over the receiver range, not two from the near plane.
export const CSM_CASCADES = 4;
export const CSM_MAP_SIZE = 2048;
export const SINGLE_MAP_SIZE = 1024;
// The limited cascade range keeps distant haze from spending close shadow texels.
export const SHADOW_MAX_FAR = 2600;
export const CSM_LIGHT_MARGIN = 300;
// Depth bias is normalized; normal bias is in world units.
export const SHADOW_BIAS = -0.00003;
export const SHADOW_NORMAL_BIAS = 0.6;
export const SHADOW_CAM_NEAR = 1;
export const SHADOW_CAM_FAR = 2500;

/** Aerosol turbidity broadens the sampling radius without a second light preset.
 * The base curve every mode starts from; `sunShadowRadius` is what receivers get. */
function shadowRadiusForTurbidity(turbidity: number): number {
  return Math.min(3, Math.max(1, 1 + (turbidity - 2) * 0.28));
}

/** A narrower single-map footprint keeps small directional shadows distinct.
 * High retains its established softness; world-space blur also depends on the
 * current fit, so this is a visual policy rather than a texel-size equivalence. */
const SINGLE_SHADOW_RADIUS_SCALE = 0.6;

/** One owner for the PCF radius every receiver reads, in texels: the Three rig
 * sets it on `shadow.radius`, the native frame packs it into the receiver
 * block. Mode is explicit at both call sites — a default would let a new
 * consumer inherit the single-tier narrowing by omission. */
export function sunShadowRadius(turbidity: number, mode: SunShadowMode): number {
  if (mode === "off") return 0;
  const radius = shadowRadiusForTurbidity(turbidity);
  return mode === "single" ? radius * SINGLE_SHADOW_RADIUS_SCALE : radius;
}

/** The orthonormal frame three builds for the shadow camera, reproduced here so
 *  a fit computed on the CPU lands on the same texels three rasterises.
 *  `depth` points FROM the scene TOWARD the sun (three's lookAt +Z). */
export interface ShadowLightBasis {
  right: Vec3;
  upAxis: Vec3;
  depth: Vec3;
  /** The `up` three's lookAt must use to rebuild this basis. */
  up: Vec3;
}

export function shadowLightBasis(
  unitSunDirection: readonly [number, number, number],
  fixedUp?: readonly [number, number, number],
): ShadowLightBasis {
  const depth = unit3(unitSunDirection, [0, 0, 1]);
  // Three's shadow camera keeps its default +Y up; only a sun lying along it
  // would degenerate the cross product. CSM pins that +Y with no such guard, so
  // a cascade fit passes its up explicitly instead of inheriting this one.
  const up: Vec3 = fixedUp
    ? [fixedUp[0], fixedUp[1], fixedUp[2]]
    : Math.abs(depth[1]) > 0.99
      ? [0, 0, 1]
      : [0, 1, 0];
  const right = unit3(cross3(up, depth), [1, 0, 0]);
  return { right, upAxis: cross3(depth, right), depth, up };
}

function cross3(a: readonly number[], b: readonly number[]): Vec3 {
  return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
}

function unit3(v: readonly number[], fallback: Vec3): Vec3 {
  const length = Math.hypot(v[0], v[1], v[2]);
  return length > 1e-9 ? [v[0] / length, v[1] / length, v[2] / length] : [...fallback];
}
