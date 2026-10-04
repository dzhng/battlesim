// @vitest-environment node
import assert from "node:assert/strict";
import { test } from "vitest";
import { mat4, vec3, type Mat4, type Vec3 } from "math";
import {
  createGpuMat4,
  createProjectedPoint,
  createWorldRay,
  eyePosition,
  invViewProj,
  projectPoint,
  projMatrix,
  screenRay,
  screenRayFrom,
  viewMatrix,
  viewProjMatrix,
  type Camera3DParams,
} from "@packages/renderer-core/src/camera3d.ts";

// A representative oblique battle-ish camera. Finite far keeps the reverse-Z
// depth mapping exact at both planes for the monotonic test.
const CAM: Camera3DParams = {
  target: [12, -30, 0],
  distance: 220,
  pitch: 0.55,
  yaw: 0.2,
  fovY: 0.6,
  aspect: 1.6,
  near: 1,
  far: 4000,
};

/** Project through a camera's own view-projection. */
const project = (cam: Camera3DParams, world: Vec3) =>
  projectPoint(createProjectedPoint(), viewProjMatrix(createGpuMat4(), cam), world);

function maxAbsDiff(a: Mat4, b: Mat4): number {
  let m = 0;
  for (let i = 0; i < 16; i++) m = Math.max(m, Math.abs(a[i] - b[i]));
  return m;
}

test("camera3d: screen↔world round-trips on the ground plane", () => {
  // Project a grid of ground points, then unproject the pixel back onto the same
  // z-plane — the inverse must land on the original point.
  for (let gx = -180; gx <= 180; gx += 60) {
    for (let gy = -180; gy <= 180; gy += 60) {
      const world: Vec3 = [gx, gy, 0];
      const { ndc, clipW } = project(CAM, world);
      if (clipW <= 0) continue; // behind the eye — not a pickable pixel
      const back = vec3.create();
      const { origin, dir } = screenRay(createWorldRay(), CAM, ndc[0], ndc[1]);
      assert.ok(Math.abs(dir[2]) > 1e-9, `parallel ray at ${gx},${gy}`);
      const t = -origin[2] / dir[2];
      assert.ok(Number.isFinite(t) && t >= 0, `ground behind eye at ${gx},${gy}`);
      vec3.scaleAndAdd(back, origin, dir, t);
      // Camera matrices are float32 (they match the GPU uniform exactly), so the
      // round trip carries single-precision error dominated by the perspective
      // divide (~a few ×10⁻³ world units across the field) — far below soldier
      // spacing and sub-pixel for picking.
      assert.ok(
        Math.hypot(back[0] - gx, back[1] - gy, back[2]) < 1e-2,
        `round-trip ${gx},${gy} → ${back}`,
      );
    }
  }
});

test("camera3d: the target projects to the screen centre", () => {
  const { ndc, clipW } = project(CAM, CAM.target);
  assert.ok(clipW > 0);
  assert.ok(Math.abs(ndc[0]) < 1e-5 && Math.abs(ndc[1]) < 1e-5, `target ndc ${ndc}`);
});

test("camera3d: view rotation is orthonormal", () => {
  const v = viewMatrix(mat4.create(), CAM);
  // Upper-left 3×3 rotation (column-major): columns are the basis vectors.
  const cols = [
    [v[0], v[1], v[2]],
    [v[4], v[5], v[6]],
    [v[8], v[9], v[10]],
  ] as Vec3[];
  for (let i = 0; i < 3; i++) {
    assert.ok(Math.abs(vec3.dot(cols[i], cols[i]) - 1) < 1e-6, `column ${i} not unit`);
    for (let j = i + 1; j < 3; j++) {
      assert.ok(Math.abs(vec3.dot(cols[i], cols[j])) < 1e-6, `columns ${i},${j} not orthogonal`);
    }
  }
});

test("camera3d: reverse-Z depth is monotonic-decreasing and in [0,1]", () => {
  // March straight out along the eye→target ray; reverse-Z means nearer = higher
  // depth (near→1, far→0), strictly decreasing, always inside the unit range.
  const eye = eyePosition(vec3.create(), CAM);
  const ahead = vec3.normalize(vec3.create(), vec3.subtract(vec3.create(), CAM.target, eye));
  let prev = Infinity;
  for (const d of [2, 10, 50, 200, 800, 3500]) {
    const { ndc } = project(CAM, vec3.scaleAndAdd(vec3.create(), eye, ahead, d));
    assert.ok(ndc[2] > 0 && ndc[2] < 1, `depth ${ndc[2]} out of [0,1] at d=${d}`);
    assert.ok(ndc[2] < prev, `depth not decreasing at d=${d} (${ndc[2]} !< ${prev})`);
    prev = ndc[2];
  }
});

test("camera3d: invViewProj is a true inverse", () => {
  const vp = viewProjMatrix(mat4.create(), CAM);
  const prod = mat4.multiply(mat4.create(), vp, invViewProj(mat4.create(), CAM));
  const identity = mat4.create();
  assert.ok(
    maxAbsDiff(prod, identity) < 1e-4,
    `VP · VP⁻¹ ≠ I (max diff ${maxAbsDiff(prod, identity)})`,
  );
});

test("camera3d: infinite far plane maps the horizon toward depth 0", () => {
  const inf: Camera3DParams = { ...CAM, far: undefined };
  const eye = eyePosition(vec3.create(), inf);
  // A very distant ground point ahead should approach — but stay above — depth 0.
  const { ndc } = project(inf, [eye[0] + 1e6, eye[1], 0]);
  // Infinite far → depth collapses to ~0 (floating error may nudge it barely
  // negative); the contract is "vanishingly small", not a hard sign.
  assert.ok(Math.abs(ndc[2]) < 1e-2, `far depth ${ndc[2]} not near 0`);
});

test("camera3d: matrices are deterministic for identical params", () => {
  assert.deepEqual(viewProjMatrix(mat4.create(), CAM), viewProjMatrix(mat4.create(), CAM));
  assert.deepEqual(projMatrix(mat4.create(), CAM), projMatrix(mat4.create(), CAM));
});

test("camera3d: a top-down camera keeps world north at the top of the screen", () => {
  // Straight down, the view's forward is parallel to +Z, so the up hint falls
  // back to +Y: whatever the yaw, a point north of the target is screen-up.
  for (const yaw of [0, 0.7, 2.5, -1.9]) {
    const top: Camera3DParams = { ...CAM, pitch: Math.PI / 2, yaw };
    const [x, y, z] = top.target;
    const north = project(top, [x, y + 20, z]).ndc;
    const east = project(top, [x + 20, y, z]).ndc;
    assert.ok(north[1] > 0.1 && Math.abs(north[0]) < 0.02, `yaw ${yaw}: north at ${north}`);
    assert.ok(east[0] > 0.1 && Math.abs(east[1]) < 0.02, `yaw ${yaw}: east at ${east}`);
  }
});

test("camera3d: a ray cast from a precomputed inverse matches the camera's own ray", () => {
  const inverse = invViewProj(createGpuMat4(), CAM);
  const eye = eyePosition(vec3.create(), CAM);
  for (const [x, y] of [
    [0, 0],
    [-1, 1],
    [0.4, -0.7],
  ]) {
    const own = screenRay(createWorldRay(), CAM, x, y);
    const shared = screenRayFrom(createWorldRay(), inverse, eye, x, y);
    assert.deepEqual(shared, own);
    assert.ok(Math.abs(vec3.length(own.dir) - 1) < 1e-12, "ray direction is unit length");
    // The ray passes through the pixel it was cast from.
    const through = project(CAM, vec3.scaleAndAdd(vec3.create(), own.origin, own.dir, 50)).ndc;
    // Float32 camera matrices: 1e-4 NDC is a twentieth of a pixel at 1000 px.
    assert.ok(Math.hypot(through[0] - x, through[1] - y) < 1e-4, `ray ${x},${y} → ${through}`);
  }
});
