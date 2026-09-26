// @vitest-environment node
import assert from "node:assert/strict";
import { test } from "vitest";
import { d } from "typegpu";
import { Camera } from "@packages/battle-renderer/src/world/camera.ts";
import {
  cameraUniformData,
  CAMERA_UNIFORM_BYTES,
  CAMERA_UNIFORM_FLOATS,
  type CameraSnapshot,
} from "@packages/renderer-core/src/cameraUniform.ts";
import {
  eyePosition,
  invViewProj,
  viewProjMatrix,
  type Camera3DParams,
} from "@packages/renderer-core/src/camera3d.ts";

// The fixture camera carries a deliberately wrong aspect (999): the packer must
// override it with the live width/height so resize has a single owner.
const CAM3D: Camera3DParams = {
  target: [12, -30, 0],
  distance: 220,
  pitch: 0.55,
  yaw: 0.2,
  fovY: 0.6,
  aspect: 999,
  near: 1,
  far: 4000,
};
// Every scalar distinct, so a field read from the wrong slot shows.
const SNAPSHOT: Required<CameraSnapshot> = {
  camera3d: CAM3D,
  width: 1000,
  height: 600,
  x: 12.5,
  y: -30.25,
  zoom: 3.75,
  time: 17.5,
  sunAzimuth: -0.35,
  sunElevation: 0.62,
};

const f32 = (value: number) => new Float32Array([value])[0];
type CameraValue = d.Infer<typeof Camera>;
/** The float the packed buffer holds where the WGSL struct reads `field`. */
const at = (data: Float32Array, field: (c: CameraValue) => unknown) =>
  data[d.memoryLayoutOf(Camera, field).offset / 4];

test("cameraUniform: the packed buffer is exactly the WGSL Camera struct, 48 floats", () => {
  assert.equal(CAMERA_UNIFORM_FLOATS, 48);
  assert.equal(d.sizeOf(Camera), CAMERA_UNIFORM_BYTES);
  assert.equal(cameraUniformData(SNAPSHOT).byteLength, CAMERA_UNIFORM_BYTES);
});

test("cameraUniform: each WGSL field reads the value packed for it", () => {
  const data = cameraUniformData(SNAPSHOT);
  const live: Camera3DParams = { ...CAM3D, aspect: 1000 / 600 };
  const vp = viewProjMatrix(live);
  const ivp = invViewProj(live);
  for (let i = 0; i < 16; i++) {
    assert.equal(data[d.memoryLayoutOf(Camera, (c) => c.viewProj).offset / 4 + i], vp[i]);
    assert.equal(data[d.memoryLayoutOf(Camera, (c) => c.invViewProj).offset / 4 + i], ivp[i]);
  }
  const eye = eyePosition(live);
  const expected: [string, (c: CameraValue) => unknown, number][] = [
    ["eye.x", (c) => c.eye.x, eye[0]],
    ["eye.y", (c) => c.eye.y, eye[1]],
    ["eye.z", (c) => c.eye.z, eye[2]],
    ["znear", (c) => c.znear, 1],
    ["focus.x", (c) => c.focus.x, 12.5],
    ["focus.y", (c) => c.focus.y, -30.25],
    ["width", (c) => c.width, 1000],
    ["height", (c) => c.height, 600],
    ["zoom", (c) => c.zoom, 3.75],
    ["tilt", (c) => c.tilt, Math.sin(0.55)],
    ["time", (c) => c.time, 17.5],
    ["zfar", (c) => c.zfar, 4000],
    ["sunAz", (c) => c.sunAz, -0.35],
    ["sunEl", (c) => c.sunEl, 0.62],
  ];
  for (const [name, field, value] of expected) assert.equal(at(data, field), f32(value), name);
});

test("cameraUniform: an infinite far plane packs the zero sentinel", () => {
  const data = cameraUniformData({ ...SNAPSHOT, camera3d: { ...CAM3D, far: undefined } });
  assert.equal(
    at(data, (c) => c.zfar),
    0,
  );
});
