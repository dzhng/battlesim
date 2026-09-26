// Adapted from ~/dev/game game-renderer/src/battle/shadowPolicy.ts (reuse
// manifest): the cascade slots and the light basis.
// Local changes: four cascade slots; the reach, map size, biases and softness
// are `presentation.light.cascades` (slice 13); the single fitted map
// (`SingleShadowPolicy`, `viewShadowFit`) and the turbidity-driven PCF radius
// are not ported.
import { vec3, type Vec3 } from "math";

/** Receiver-layout cascade slots: the WGSL block and the depth array are
 *  sized by this, so `presentation.light.cascades.count` must equal it. */
export const CSM_CASCADES = 4;
/** How far above the highest slice corner each cascade light sits, so casters
 *  just off screen still land in its depth range. */
export const CSM_LIGHT_MARGIN = 300;
export const SHADOW_CAM_NEAR = 1;

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

export function shadowLightBasis(unitSunDirection: Vec3, fixedUp?: Vec3): ShadowLightBasis {
  const depth = unitOr(vec3.create(), unitSunDirection, SUN_FALLBACK);
  // Three's shadow camera keeps its default +Y up; only a sun lying along it
  // would degenerate the cross product. CSM pins that +Y with no such guard, so
  // a cascade fit passes its up explicitly instead of inheriting this one.
  const up = vec3.clone(fixedUp ?? (Math.abs(depth[1]) > 0.99 ? AXIS_Z : AXIS_Y));
  const right = vec3.cross(vec3.create(), up, depth);
  unitOr(right, right, AXIS_X);
  return { right, upAxis: vec3.cross(vec3.create(), depth, right), depth, up };
}

const AXIS_X: Vec3 = [1, 0, 0];
const AXIS_Y: Vec3 = [0, 1, 0];
const AXIS_Z: Vec3 = [0, 0, 1];
const SUN_FALLBACK = AXIS_Z;

/** `v` normalised into `out`, or `fallback` when `v` is too short to have a
 *  direction (1e-9: far below any real sun or basis vector, above rounding). */
function unitOr(out: Vec3, v: Vec3, fallback: Vec3): Vec3 {
  return vec3.length(v) > 1e-9 ? vec3.normalize(out, v) : vec3.copy(out, fallback);
}
