// Adapted from ~/dev/game game-renderer/src/battle/shadowPolicy.ts (reuse
// manifest): the cascade slots and the light basis.
// Local changes: four cascade slots; the reach, map size, biases and softness
// are `presentation.light.cascades` (slice 13); the single fitted map
// (`SingleShadowPolicy`, `viewShadowFit`) and the turbidity-driven PCF radius
// are not ported.
import { type Vec3 } from "@packages/renderer-core/src/camera3d";

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
