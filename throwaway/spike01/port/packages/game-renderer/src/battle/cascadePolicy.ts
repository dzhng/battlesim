/** Renderer-neutral cascade split and fit policy — the High shadow tier's one
 *  geometry owner. The pinned Three `CSMShadowNode`/`CSMFrustum` pair reached
 *  through `web/tests/reference/threeShadowRig.ts` is the behaviour this
 *  reproduces; it stays a TEST ORACLE, never a runtime dependency of this file.
 *
 *  Division of labour: everything here is pure geometry (breaks, slice corners,
 *  square extents, texel-snapped light-space centres, matrices, blend weights).
 *  GPU packing, depth resources and culling views belong to the raw shadow
 *  module that consumes this. The single fitted map keeps its own owner in
 *  `shadowPolicy.ts`; only the light basis and the shared quality constants are
 *  common, and they are imported rather than restated.
 */
import {
  CSM_CASCADES,
  CSM_LIGHT_MARGIN,
  CSM_MAP_SIZE,
  SHADOW_BIAS,
  SHADOW_CAM_FAR,
  SHADOW_CAM_NEAR,
  SHADOW_MAX_FAR,
  SHADOW_NORMAL_BIAS,
  shadowLightBasis,
} from "./shadowPolicy";
import {
  projMatrix,
  viewMatrix,
  type Camera3DParams,
  type Vec3,
} from "../../../renderer-core/src/camera3d";
import {
  invert,
  lookAt,
  multiply,
  orthographicReverseZ,
  type Mat4,
} from "../../../renderer-core/src/mat4";

/** Practical-split blend between the uniform and logarithmic ladders. */
export const CASCADE_SPLIT_LAMBDA = 0.5;
/** Cascade shadow cameras keep Three's hard-coded +Y light orientation up. The
 *  single tier's basis guard (a sun lying along +Y) is NOT applied there, so it
 *  is passed explicitly here instead of inherited. */
export const CASCADE_LIGHT_UP: Vec3 = [0, 1, 0];

/** How this frame's camera far resolves into the three DIFFERENT far values the
 *  source uses. Keeping them named is the point: two of them are not equal and
 *  normalising them together would silently change the fitted extents. */
export interface CascadeFarResolution {
  near: number;
  /** The finite far the frustum projection is actually built from. */
  projectionFar: number;
  /** `min(projectionFar, SHADOW_MAX_FAR)` — splits and the receiver fade. */
  cappedFar: number;
  /** `max(projectionFar, SHADOW_MAX_FAR)` — the source's extent fade margin.
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
}

/** Normalises the camera far into the split/extent references. An absent,
 *  infinite or 0-sentinel far means an UNBOUNDED receiver range: the capped far
 *  is the shadow reach, never 0. A finite far at or behind the near plane is a
 *  malformed projection and is rejected rather than inverted. */
export function resolveCascadeFar({
  camera,
  resolvedFar,
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
    cappedFar: Math.min(projectionFar, SHADOW_MAX_FAR),
    extentFar: Math.max(projectionFar, SHADOW_MAX_FAR),
  };
}

/** Practical split: each internal break is the midpoint of the uniform and
 *  logarithmic depths at that fraction, normalised by the capped far. The final
 *  break is exactly 1. Breaks are normalised by f while the receiver's linear
 *  depth subtracts n, so a shader split plane sits `n*(1-break)` FARTHER out
 *  than the geometric plane these corners are cut on — bounded by n, and named
 *  in the reference tests rather than corrected here. */
export function cascadeBreaks(near: number, cappedFar: number, count = CSM_CASCADES): number[] {
  const breaks: number[] = [];
  for (let i = 1; i < count; i++) {
    const uniform = (near + (cappedFar - near) * (i / count)) / cappedFar;
    const logarithmic = (near * (cappedFar / near) ** (i / count)) / cappedFar;
    breaks.push(uniform + (logarithmic - uniform) * CASCADE_SPLIT_LAMBDA);
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
  /** World-unit normal offset — cloned unchanged across cascades. */
  normalBias: number;
  view: Mat4;
  projection: Mat4;
  viewProjection: Mat4;
}

export interface CascadeFrame extends CascadeFarResolution {
  breaks: readonly number[];
  mapSize: number;
  cascades: readonly CascadeFit[];
}

export interface CascadeFitInput extends CascadeCameraInput {
  unitSunDirection: readonly [number, number, number];
  cascades?: number;
  mapSize?: number;
  lightMargin?: number;
  /** SPIKE 01: view depth range where receivers (the map) actually are. Splits
   *  run over [receiverNear, min(receiverFar, SHADOW_MAX_FAR)] instead of from
   *  the camera near plane, so a 1 km-high camera does not spend cascades on air. */
  receiverNear?: number;
  receiverFar?: number;
}

/** Every active cascade's slice, extent, pose and matrices for ONE frame. Pure:
 *  the caller supplies the camera it is about to render with, so a fit and the
 *  culling it feeds always describe the same frame. */
export function cascadeFits(input: CascadeFitInput): CascadeFrame {
  const count = Math.max(1, Math.floor(input.cascades ?? CSM_CASCADES));
  const mapSize = Math.max(16, Math.floor(input.mapSize ?? CSM_MAP_SIZE));
  const lightMargin = input.lightMargin ?? CSM_LIGHT_MARGIN;
  const resolved = resolveCascadeFar(input);
  const splitNear = Math.max(resolved.near, input.receiverNear ?? resolved.near);
  const cappedFar = Math.max(
    splitNear * 1.5,
    Math.min(resolved.cappedFar, input.receiverFar ?? resolved.cappedFar),
  );
  const far = { ...resolved, near: splitNear, cappedFar };
  const breaks = cascadeBreaks(far.near, far.cappedFar, count);
  const basis = shadowLightBasis(input.unitSunDirection, CASCADE_LIGHT_UP);
  const camera = { ...input.camera, far: far.projectionFar };
  const view = viewMatrix(camera);
  const world = invert(view);
  if (!world) throw Error("Cascade fit camera view is singular");
  const projection = projMatrix(camera);
  const inverseProjection = invert(projection);
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
    ].map(([x, y]) => transformPoint(inverseProjection, [x, y, clipZ]));
  const atDepth = (depth: number) =>
    quad(0).map((vertex) => {
      const scale = depth / Math.abs(vertex[2]);
      return [vertex[0] * scale, vertex[1] * scale, vertex[2] * scale] as Vec3;
    });
  const mainNear = far.near > resolved.near ? atDepth(far.near) : quad(1);
  const mainFar = atDepth(far.cappedFar);

  const nearZ = mainNear[0][2];
  const farZ = mainFar[0][2];
  // Breaks are depth / cappedFar; nearZ and farZ are negative view z.
  const alphaFor = (fraction: number) => (fraction * farZ - nearZ) / (farZ - nearZ);
  const cascades: CascadeFit[] = [];
  for (let i = 0; i < count; i++) {
    const sliceNear =
      i === 0 ? mainNear : mainNear.map((v, j) => lerp3(v, mainFar[j], alphaFor(breaks[i - 1])));
    const sliceFar =
      i === count - 1 ? mainFar : mainNear.map((v, j) => lerp3(v, mainFar[j], alphaFor(breaks[i])));
    const extent = cascadeExtent(sliceNear, sliceFar, far);
    const worldUnitsPerTexel = extent / mapSize;
    const corners = [...sliceNear, ...sliceFar].map((v) => transformPoint(world, v));
    const position = lightSpaceCentre(corners, basis, worldUnitsPerTexel, lightMargin);
    // The source aims its cascade light one unit down the light's travel
    // direction, which is the negated basis depth axis.
    const target: Vec3 = [
      position[0] - basis.depth[0],
      position[1] - basis.depth[1],
      position[2] - basis.depth[2],
    ];
    const half = extent / 2;
    const cascadeView = lookAt(position, target, basis.up);
    const cascadeProjection = orthographicReverseZ(
      -half,
      half,
      half,
      -half,
      SHADOW_CAM_NEAR,
      SHADOW_CAM_FAR,
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
      far: SHADOW_CAM_FAR,
      depthBias: SHADOW_BIAS * (i + 1),
      normalBias: SHADOW_NORMAL_BIAS,
      view: cascadeView,
      projection: cascadeProjection,
      viewProjection: multiply(cascadeProjection, cascadeView),
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
  const across = distance3(anchor, sliceFar[2]);
  const diagonal = distance3(anchor, sliceNear[2]);
  const span = far.extentFar - far.near;
  const linearDepth = sliceFar[0][2] / span;
  return Math.max(across, diagonal) + 0.25 * linearDepth * linearDepth * span;
}

/** Light-space centre of a slice, snapped to its own texel grid with FLOOR (the
 *  source's quantisation, not rounding), pushed up the sun ray by the light
 *  margin so offscreen casters stay inside the depth range, and returned in
 *  world space as the cascade light position. */
function lightSpaceCentre(
  corners: readonly Vec3[],
  basis: ReturnType<typeof shadowLightBasis>,
  worldUnitsPerTexel: number,
  lightMargin: number,
): Vec3 {
  let xMin = Infinity,
    xMax = -Infinity,
    yMin = Infinity,
    yMax = -Infinity,
    zMax = -Infinity;
  for (const corner of corners) {
    const x = dot3(corner, basis.right);
    const y = dot3(corner, basis.upAxis);
    const z = dot3(corner, basis.depth);
    if (x < xMin) xMin = x;
    if (x > xMax) xMax = x;
    if (y < yMin) yMin = y;
    if (y > yMax) yMax = y;
    if (z > zMax) zMax = z;
  }
  const cx = Math.floor((xMin + xMax) / 2 / worldUnitsPerTexel) * worldUnitsPerTexel;
  const cy = Math.floor((yMin + yMax) / 2 / worldUnitsPerTexel) * worldUnitsPerTexel;
  const cz = zMax + lightMargin;
  return [
    basis.right[0] * cx + basis.upAxis[0] * cy + basis.depth[0] * cz,
    basis.right[1] * cx + basis.upAxis[1] * cy + basis.depth[1] * cz,
    basis.right[2] * cx + basis.upAxis[2] * cy + basis.depth[2] * cz,
  ];
}

/** Receiver linear depth: the same `(-viewZ - n) / (f - n)` the shader computes,
 *  with f the CAPPED far. Exposed so a test can drive the blend below from a
 *  world/view depth instead of restating the normalisation. */
export function cascadeReceiverDepth(viewZ: number, far: CascadeFarResolution): number {
  return (-viewZ - far.near) / (far.cappedFar - far.near);
}

/** The blend weight one cascade contributes at a receiver depth — the CPU
 *  definition the receiver shader transcribes. Zero means this cascade does not
 *  reach that depth at all; the visible result is
 *  `1 - sum over cascades of (1 - cascadeVisibility) * weight`.
 *
 *  Each interval [a,b] takes the margin from the edge NEAREST the sample
 *  (0.25 * e^2), extends its near edge by half of it, and its far edge likewise
 *  except on the last cascade — which therefore fades to unshadowed at the
 *  capped far rather than clipping. The first cascade does not fade on its
 *  nearest half. */
export function cascadeBlendWeight(
  linearDepth: number,
  interval: readonly [number, number],
  position: { first: boolean; last: boolean },
): number {
  const [start, end] = interval;
  const centre = (start + end) / 2;
  const closestEdge = linearDepth < centre ? start : end;
  const margin = 0.25 * closestEdge * closestEdge;
  const low = start - margin / 2;
  const high = position.last ? end : end + margin / 2;
  if (linearDepth < low || linearDepth > high) return 0;
  if (position.first && linearDepth <= centre) return 1;
  // A zero-width band is fully inside, not a 0/0 sample. Unreachable for the
  // shipped split (a positive near plane keeps every break above 0); the shader
  // carries the same guard so no NaN can reach a fragment.
  if (!(margin > 0)) return 1;
  return Math.min(1, Math.max(0, Math.min(linearDepth - low, high - linearDepth) / margin));
}

/** Point transform WITH the perspective divide — how the source unprojects its
 *  clip-space frustum corners and how a world corner lands in light space. */
function transformPoint(m: Mat4, v: readonly number[]): Vec3 {
  const x = m[0] * v[0] + m[4] * v[1] + m[8] * v[2] + m[12];
  const y = m[1] * v[0] + m[5] * v[1] + m[9] * v[2] + m[13];
  const z = m[2] * v[0] + m[6] * v[1] + m[10] * v[2] + m[14];
  const w = m[3] * v[0] + m[7] * v[1] + m[11] * v[2] + m[15];
  const s = w !== 0 && w !== 1 ? 1 / w : 1;
  return [x * s, y * s, z * s];
}

function lerp3(a: Vec3, b: Vec3, alpha: number): Vec3 {
  return [a[0] + (b[0] - a[0]) * alpha, a[1] + (b[1] - a[1]) * alpha, a[2] + (b[2] - a[2]) * alpha];
}

function dot3(a: readonly number[], b: readonly number[]): number {
  return a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
}

function distance3(a: readonly number[], b: readonly number[]): number {
  return Math.hypot(a[0] - b[0], a[1] - b[1], a[2] - b[2]);
}
