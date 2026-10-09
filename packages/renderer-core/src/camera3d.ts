// The real 3D perspective camera — the ONE projection owner engine-wide: the
// single source of truth for view/projection matrices and screen↔world mapping,
// shared by the GPU uniform packer (cameraUniform.ts), the orbit rig and CPU
// picking. Pure and GPU-free: everything here is unit-tested with no device.
//
// World convention: XY is the ground plane, +Z is up (matches the sim's world
// convention). Matrix and vector math is the `math` package's: column-major,
// out-first calls, module scratch named `_owner_purpose`. Camera matrices live
// in float32 storage (`createGpuMat4`), the precision the GPU uniforms hold,
// so CPU picking, projection and cascade fits compute with exactly the
// matrices the shaders read. Depth is reverse-Z in WebGPU clip space (near → 1,
// far → 0); `math` builds only forward-Z projections, so the two reverse-Z
// builders below are the only matrix code kept here.
import { mat4, vec3, vec4, type Mat4, type Vec3 } from "math";

export interface Camera3DParams {
  /** Point the camera orbits and looks at, in world space (XY ground, +Z up). */
  target: Vec3;
  /** Eye distance from `target`. */
  distance: number;
  /** Elevation of the eye above the ground plane, radians. π/2 = straight down
   *  (top-down); small = near the horizon (oblique vista). */
  pitch: number;
  /** Azimuth around +Z, radians. */
  yaw: number;
  /** Vertical field of view, radians. */
  fovY: number;
  /** Viewport aspect (width / height). */
  aspect: number;
  /** Near plane distance (> 0). */
  near: number;
  /** Far plane distance. Omit for an infinite far plane. */
  far?: number;
}

/** Metres one pixel spans at the target of a camera `distance` away with
 *  vertical field of view `fovY`, on a viewport `heightPx` tall. */
export function metresPerPxAt(distance: number, fovY: number, heightPx: number): number {
  return (2 * distance * Math.tan(fovY / 2)) / Math.max(1, heightPx);
}

// Far plane for a consumer that cannot take an infinite far plane (the
// cascade fit's frustum corners); the canonical projection above still omits
// `far` for the infinite limit. Under reverse-Z the depth terms converge to the
// infinite limit at about near/far relative error. Merged from ~/dev/game
// renderer-core.
export const FINITE_CAMERA_FAR_FALLBACK = 1e7;

/** A `Mat4` (identity) in float32 storage: every camera and cascade matrix.
 *  `math` computes in double precision and each matrix rounds once as it is
 *  stored, exactly as the GPU uniform will hold it. */
export function createGpuMat4(): Mat4 {
  return mat4.identity(new Float32Array(16) as unknown as Mat4);
}

const _view_eye = vec3.create();
const _view_up = vec3.create();
const _viewProj_view = createGpuMat4();
const _viewProj_projection = createGpuMat4();
const _invViewProj_viewProj = createGpuMat4();
const _project_clip = vec4.create();
const _ray_eye = vec3.create();
const _ray_inverse = createGpuMat4();
const _ray_near = vec4.create();

/** Eye position derived from the orbit params. Pitch is clamped just shy of
 *  vertical so the view's up vector never degenerates at exact top-down. */
export function eyePosition(out: Vec3, p: Camera3DParams): Vec3 {
  const pitch = Math.min(Math.PI / 2 - 1e-3, Math.max(-Math.PI / 2 + 1e-3, p.pitch));
  const cp = Math.cos(pitch);
  return vec3.set(
    out,
    p.target[0] + p.distance * cp * Math.cos(p.yaw),
    p.target[1] + p.distance * cp * Math.sin(p.yaw),
    p.target[2] + p.distance * Math.sin(pitch),
  );
}

/** World → eye space, right-handed, the camera looking down its −Z. The up
 *  hint is the orbit's own up (pitched back from +Z by the pitch), not world
 *  +Z: it never lines up with the view, so the screen keeps the yaw's heading
 *  at the top all the way to straight down instead of spinning near vertical. */
export function viewMatrix(out: Mat4, p: Camera3DParams): Mat4 {
  eyePosition(_view_eye, p);
  const s = Math.sin(p.pitch);
  vec3.set(_view_up, -s * Math.cos(p.yaw), -s * Math.sin(p.yaw), Math.cos(p.pitch));
  return mat4.lookAt(out, _view_eye, p.target, _view_up);
}

export function projMatrix(out: Mat4, p: Camera3DParams): Mat4 {
  return perspectiveReverseZ(out, p.fovY, p.aspect, p.near, p.far);
}

export function viewProjMatrix(out: Mat4, p: Camera3DParams): Mat4 {
  viewMatrix(_viewProj_view, p);
  projMatrix(_viewProj_projection, p);
  return mat4.multiply(out, _viewProj_projection, _viewProj_view);
}

/** The inverse view-projection; the identity if the projection is singular. */
export function invViewProj(out: Mat4, p: Camera3DParams): Mat4 {
  viewProjMatrix(_invViewProj_viewProj, p);
  return mat4.invert(out, _invViewProj_viewProj) ?? mat4.identity(out);
}

/** Reverse-Z perspective for WebGPU clip space (NDC z ∈ [0,1], y up),
 *  right-handed view space (looking down −Z). Near maps to depth 1, far to
 *  depth 0 — the precision-optimal convention. `far` omitted → infinite far
 *  plane (far → depth 0 in the limit), which long oblique views want. */
export function perspectiveReverseZ(
  out: Mat4,
  fovY: number,
  aspect: number,
  near: number,
  far?: number,
): Mat4 {
  const f = 1 / Math.tan(fovY / 2);
  mat4.zero(out);
  out[0] = f / aspect;
  out[5] = f;
  out[11] = -1;
  // Row 2 (depth): finite far uses A,B; infinite far is the limit A=0, B=near.
  out[10] = far === undefined ? 0 : near / (far - near);
  out[14] = far === undefined ? near : (near * far) / (far - near);
  return out;
}

/** Orthographic WebGPU projection, right-handed view space, near→1 and far→0.
 *  Merged from ~/dev/game renderer-core: the sun's cascade
 *  cameras. */
export function orthographicReverseZ(
  out: Mat4,
  left: number,
  right: number,
  top: number,
  bottom: number,
  near: number,
  far: number,
): Mat4 {
  mat4.identity(out);
  out[0] = 2 / (right - left);
  out[5] = 2 / (top - bottom);
  out[10] = 1 / (far - near);
  out[12] = -(right + left) / (right - left);
  out[13] = -(top + bottom) / (top - bottom);
  out[14] = far / (far - near);
  return out;
}

/** A world point in normalised device coordinates. `clipW` is the homogeneous
 *  w (view-space depth, positive in front): `clipW <= 0` is behind the eye. */
export interface ProjectedPoint {
  ndc: Vec3;
  clipW: number;
}

export function createProjectedPoint(): ProjectedPoint {
  return { ndc: vec3.create(), clipW: 0 };
}

/** Project a world point through a view-projection (`viewProjMatrix`), so a
 *  caller projecting many points builds the matrix once. */
export function projectPoint(out: ProjectedPoint, viewProj: Mat4, world: Vec3): ProjectedPoint {
  vec4.set(_project_clip, world[0], world[1], world[2], 1);
  vec4.transformMat4(_project_clip, _project_clip, viewProj);
  const w = _project_clip[3];
  const inv = w !== 0 ? 1 / w : 0;
  vec3.set(out.ndc, _project_clip[0] * inv, _project_clip[1] * inv, _project_clip[2] * inv);
  out.clipW = w;
  return out;
}

/** A world-space ray: origin at the analytic eye, unit direction. */
export interface WorldRay {
  origin: Vec3;
  dir: Vec3;
}

export function createWorldRay(): WorldRay {
  return { origin: vec3.create(), dir: vec3.create() };
}

/** The ray through an NDC pixel (ndcX, ndcY ∈ [-1, 1]) from a precomputed
 *  inverse view-projection and eye, for callers casting many rays. The
 *  direction points through the near-plane unprojection of the pixel.
 *  Anchoring at the eye (rather than differencing near/far NDC points) keeps
 *  this well-defined for an infinite far plane, where the far-plane
 *  unprojection is a point at infinity. */
export function screenRayFrom(
  out: WorldRay,
  inverseViewProj: Mat4,
  eye: Vec3,
  ndcX: number,
  ndcY: number,
): WorldRay {
  // Reverse-Z: the near plane is depth 1.
  vec4.set(_ray_near, ndcX, ndcY, 1, 1);
  vec4.transformMat4(_ray_near, _ray_near, inverseViewProj);
  const iw = _ray_near[3] !== 0 ? 1 / _ray_near[3] : 0;
  vec3.set(
    out.dir,
    _ray_near[0] * iw - eye[0],
    _ray_near[1] * iw - eye[1],
    _ray_near[2] * iw - eye[2],
  );
  vec3.normalize(out.dir, out.dir);
  vec3.copy(out.origin, eye);
  return out;
}

/** The world-space ray through an NDC pixel of camera `p`. */
export function screenRay(out: WorldRay, p: Camera3DParams, ndcX: number, ndcY: number): WorldRay {
  eyePosition(_ray_eye, p);
  invViewProj(_ray_inverse, p);
  return screenRayFrom(out, _ray_inverse, _ray_eye, ndcX, ndcY);
}
