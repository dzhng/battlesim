// Adapted from ~/dev/game game-renderer/src/battle/cascadePolicy.ts (reuse
// manifest). Local changes: splits run over the receiver range (where the map
// is), not from the camera near plane; the reach, split lambda, map size and
// depth bias are `presentation.light.cascades`; each light's depth range fits
// its slice instead of a fixed 2,500 m.
/** Pure cascade geometry: splits, slice corners, texel-snapped light-space
 *  fits and blend weights. The world shadow module owns packing and resources. */
import { CSM_LIGHT_MARGIN, SHADOW_CAM_NEAR, shadowLightBasis } from "./shadowPolicy";
import type { CascadeSettings } from "./sceneLight";
import { mat4, vec3, type Mat4, type Vec3 } from "math";
import {
  createGpuMat4,
  orthographicReverseZ,
  projMatrix,
  viewMatrix,
  type Camera3DParams,
} from "@packages/renderer-core/src/camera3d";

/** Pin the cascade fit's +Y light orientation instead of choosing a different
 *  basis near a vertical sun. */
export const CASCADE_LIGHT_UP: Vec3 = [0, 1, 0];

const _fit_view = createGpuMat4();
const _fit_world = createGpuMat4();
const _fit_projection = createGpuMat4();
const _fit_inverseProjection = createGpuMat4();

/** How this frame's camera far resolves into the three DIFFERENT far values the
 *  source uses. Keeping them named is the point: two of them are not equal and
 *  normalising them together would silently change the fitted extents. */
export interface CascadeFarResolution {
  near: number;
  /** The finite far the frustum projection is actually built from. */
  projectionFar: number;
  /** `min(projectionFar, maxFar)` — splits and the receiver fade. */
  cappedFar: number;
  /** `max(projectionFar, maxFar)` — the source's extent fade margin.
   *  Distinct from `cappedFar` on purpose; see `cascadeExtent`. */
  extentFar: number;
}

export interface CascadeCameraInput {
  /** The frame's resolved projection parameters (aspect already pinned). */
  camera: Camera3DParams;
  /** The finite far the projection owner substitutes when `camera.far` is
   *  absent, infinite, or the decoded 0 infinite-far sentinel. The battle scene
   *  resolves its camera before packing, so this is the same number that frame
   *  rendered with — never an arbitrary camera invented here. */
  resolvedFar: number;
  /** The shadow reach: no receiver past this view depth is shadowed. */
  maxFar: number;
}

/** Normalises the camera far into the split/extent references. An absent,
 *  infinite or 0-sentinel far means an UNBOUNDED receiver range: the capped far
 *  is the shadow reach, never 0. A finite far at or behind the near plane is a
 *  malformed projection and is rejected rather than inverted. */
export function resolveCascadeFar({
  camera,
  resolvedFar,
  maxFar,
}: CascadeCameraInput): CascadeFarResolution {
  const near = camera.near;
  if (!(Number.isFinite(near) && near > 0))
    throw Error("Cascade fit requires a positive camera near plane");
  const declared = camera.far;
  const unbounded = declared === undefined || !Number.isFinite(declared) || declared === 0;
  if (!unbounded && declared! <= near)
    throw Error("Cascade fit rejects a camera far plane at or behind its near plane");
  const projectionFar = unbounded ? resolvedFar : declared!;
  if (!(Number.isFinite(projectionFar) && projectionFar > near))
    throw Error("Cascade fit requires a finite resolved far plane beyond the near plane");
  return {
    near,
    projectionFar,
    cappedFar: Math.min(projectionFar, maxFar),
    extentFar: Math.max(projectionFar, maxFar),
  };
}

/** Practical split: each internal break is the midpoint of the uniform and
 *  logarithmic depths at that fraction, as a fraction of the receiver range
 *  `(depth - near) / (cappedFar - near)`. The final break is exactly 1. The
 *  receiver shader normalises its view depth the same way, so a split plane
 *  sits where the corners are cut.
 *  (The source normalised by the far alone, which with `near` a kilometre out
 *  squeezed the intervals under their blend margins and let three cascades
 *  shade one receiver.) */
export function cascadeBreaks(
  near: number,
  cappedFar: number,
  count: number,
  lambda: number,
): number[] {
  const breaks: number[] = [];
  for (let i = 1; i < count; i++) {
    const uniform = i / count;
    const logarithmic = (near * (cappedFar / near) ** (i / count) - near) / (cappedFar - near);
    breaks.push(uniform + (logarithmic - uniform) * lambda);
  }
  breaks.push(1);
  return breaks;
}

export interface CascadeFit {
  index: number;
  /** Normalised receiver-depth interval `[previous break, this break]`. */
  interval: readonly [number, number];
  /** The eight world-space corners this slice was fitted to. */
  corners: readonly Vec3[];
  /** Square light-plane extent, world units across. */
  extent: number;
  worldUnitsPerTexel: number;
  position: Vec3;
  target: Vec3;
  up: Vec3;
  left: number;
  right: number;
  top: number;
  bottom: number;
  near: number;
  far: number;
  /** Normalised depth bias, scaled by the cascade index as the source scales
   *  its cloned per-cascade shadow. */
  depthBias: number;
  view: Mat4;
  projection: Mat4;
  viewProjection: Mat4;
}

export interface CascadeFrame extends CascadeFarResolution {
  breaks: readonly number[];
  mapSize: number;
  cascades: readonly CascadeFit[];
}

export interface CascadeFitInput {
  camera: Camera3DParams;
  resolvedFar: number;
  unitSunDirection: Vec3;
  settings: CascadeSettings;
  lightMargin?: number;
  /** The view depth range where receivers (the map) actually are. Splits
   *  run over [receiverNear, min(receiverFar, max_far_m)] instead of from
   *  the camera near plane, so a 1 km-high camera does not spend cascades on air. */
  receiverNear?: number;
  receiverFar?: number;
}

/** Every active cascade's slice, extent, pose and matrices for ONE frame. Pure:
 *  the caller supplies the camera it is about to render with, so a fit and the
 *  culling it feeds always describe the same frame. */
export function cascadeFits(input: CascadeFitInput): CascadeFrame {
  const { settings } = input;
  const count = Math.max(1, Math.floor(settings.count));
  const mapSize = Math.max(16, Math.floor(settings.map_size));
  const lightMargin = input.lightMargin ?? CSM_LIGHT_MARGIN;
  const resolved = resolveCascadeFar({ ...input, maxFar: settings.max_far_m });
  const splitNear = Math.max(resolved.near, input.receiverNear ?? resolved.near);
  const cappedFar = Math.max(
    splitNear * 1.5,
    Math.min(resolved.cappedFar, input.receiverFar ?? resolved.cappedFar),
  );
  const far = { ...resolved, near: splitNear, cappedFar };
  const breaks = cascadeBreaks(far.near, far.cappedFar, count, settings.split_lambda);
  const basis = shadowLightBasis(input.unitSunDirection, CASCADE_LIGHT_UP);
  const camera = { ...input.camera, far: far.projectionFar };
  const world = mat4.invert(_fit_world, viewMatrix(_fit_view, camera));
  if (!world) throw Error("Cascade fit camera view is singular");
  const inverseProjection = mat4.invert(
    _fit_inverseProjection,
    projMatrix(_fit_projection, camera),
  );
  if (!inverseProjection) throw Error("Cascade fit camera projection is singular");

  // The source's near/far quad order, unprojected from reverse-Z clip space.
  // Near is clip z = 1 and far is clip z = 0; the far quad is then pulled in so
  // its view depth never exceeds the shadow reach.
  const quad = (clipZ: number) =>
    [
      [1, 1],
      [1, -1],
      [-1, -1],
      [-1, 1],
    ].map(([x, y]) => vec3.transformMat4(vec3.create(), [x, y, clipZ], inverseProjection));
  const atDepth = (depth: number) =>
    quad(0).map((vertex) => vec3.scale(vertex, vertex, depth / Math.abs(vertex[2])));
  const mainNear = far.near > resolved.near ? atDepth(far.near) : quad(1);
  const mainFar = atDepth(far.cappedFar);

  // Breaks are fractions of the receiver range, so they lerp the corners directly.
  const cascades: CascadeFit[] = [];
  for (let i = 0; i < count; i++) {
    const sliceNear =
      i === 0
        ? mainNear
        : mainNear.map((v, j) => vec3.lerp(vec3.create(), v, mainFar[j], breaks[i - 1]));
    const sliceFar =
      i === count - 1
        ? mainFar
        : mainNear.map((v, j) => vec3.lerp(vec3.create(), v, mainFar[j], breaks[i]));
    const extent = cascadeExtent(sliceNear, sliceFar, far);
    const worldUnitsPerTexel = extent / mapSize;
    const corners = [...sliceNear, ...sliceFar].map((v) =>
      vec3.transformMat4(vec3.create(), v, world),
    );
    const { position, depthSpan } = lightSpaceCentre(
      corners,
      basis,
      worldUnitsPerTexel,
      lightMargin,
    );
    // Deep enough for the slice seen along the sun, plus the margin at both ends.
    const lightFar = depthSpan + 2 * lightMargin;
    // The source aims its cascade light one unit down the light's travel
    // direction, which is the negated basis depth axis.
    const target = vec3.subtract(vec3.create(), position, basis.depth);
    const half = extent / 2;
    const cascadeView = mat4.lookAt(createGpuMat4(), position, target, basis.up);
    const cascadeProjection = orthographicReverseZ(
      createGpuMat4(),
      -half,
      half,
      half,
      -half,
      SHADOW_CAM_NEAR,
      lightFar,
    );
    cascades.push({
      index: i,
      interval: [i === 0 ? 0 : breaks[i - 1], breaks[i]],
      corners,
      extent,
      worldUnitsPerTexel,
      position,
      target,
      up: basis.up,
      left: -half,
      right: half,
      top: half,
      bottom: -half,
      near: SHADOW_CAM_NEAR,
      far: lightFar,
      depthBias: settings.depth_bias * (i + 1),
      view: cascadeView,
      projection: cascadeProjection,
      viewProjection: mat4.multiply(createGpuMat4(), cascadeProjection, cascadeView),
    });
  }
  return { ...far, breaks, mapSize, cascades };
}

/** The source's square extent: the longer of the far-plane diagonal and the
 *  near-to-far diagonal, widened by the fade margin.
 *
 *  The margin reads `extentFar` where the split and the receiver fade read
 *  `cappedFar`. That asymmetry is the source's, it is small but NOT zero at the
 *  resolved 1e7 far, and it is preserved deliberately: normalising it to the
 *  capped far would widen every cascade box by the whole blend band on evidence
 *  nobody has produced yet. */
export function cascadeExtent(
  sliceNear: readonly Vec3[],
  sliceFar: readonly Vec3[],
  far: CascadeFarResolution,
): number {
  const anchor = sliceFar[0];
  const across = vec3.distance(anchor, sliceFar[2]);
  const diagonal = vec3.distance(anchor, sliceNear[2]);
  const span = far.extentFar - far.near;
  const linearDepth = sliceFar[0][2] / span;
  return Math.max(across, diagonal) + 0.25 * linearDepth * linearDepth * span;
}

/** Light-space centre of a slice, snapped to its own texel grid with FLOOR (the
 *  source's quantisation, not rounding), pushed up the sun ray by the light
 *  margin so offscreen casters stay inside the depth range, and returned in
 *  world space as the cascade light position, with the slice's depth along
 *  the sun. */
function lightSpaceCentre(
  corners: readonly Vec3[],
  basis: ReturnType<typeof shadowLightBasis>,
  worldUnitsPerTexel: number,
  lightMargin: number,
): { position: Vec3; depthSpan: number } {
  let xMin = Infinity,
    xMax = -Infinity,
    yMin = Infinity,
    yMax = -Infinity,
    zMin = Infinity,
    zMax = -Infinity;
  for (const corner of corners) {
    const x = vec3.dot(corner, basis.right);
    const y = vec3.dot(corner, basis.upAxis);
    const z = vec3.dot(corner, basis.depth);
    if (x < xMin) xMin = x;
    if (x > xMax) xMax = x;
    if (y < yMin) yMin = y;
    if (y > yMax) yMax = y;
    if (z < zMin) zMin = z;
    if (z > zMax) zMax = z;
  }
  const cx = Math.floor((xMin + xMax) / 2 / worldUnitsPerTexel) * worldUnitsPerTexel;
  const cy = Math.floor((yMin + yMax) / 2 / worldUnitsPerTexel) * worldUnitsPerTexel;
  const cz = zMax + lightMargin;
  const position = vec3.scale(vec3.create(), basis.right, cx);
  vec3.scaleAndAdd(position, position, basis.upAxis, cy);
  vec3.scaleAndAdd(position, position, basis.depth, cz);
  return { position, depthSpan: zMax - zMin };
}
