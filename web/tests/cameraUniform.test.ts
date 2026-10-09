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
import { mat4, vec3 } from "math";
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
  time: 17.5,
};

const f32 = (value: number) => Math.fround(value);
const pack = (snapshot: Required<CameraSnapshot>) =>
  cameraUniformData(new Float32Array(CAMERA_UNIFORM_FLOATS), snapshot);
type CameraValue = d.Infer<typeof Camera>;
/** The float the packed buffer holds where the WGSL struct reads `field`. */
const at = (data: Float32Array, field: (c: CameraValue) => unknown) =>
  data[d.memoryLayoutOf(Camera, field).offset / 4];

test("cameraUniform: the packed buffer is exactly the WGSL Camera struct", () => {
  assert.equal(d.sizeOf(Camera), CAMERA_UNIFORM_BYTES);
  assert.equal(pack(SNAPSHOT).byteLength, CAMERA_UNIFORM_BYTES);
});

test("cameraUniform: each WGSL field reads the value packed for it", () => {
  const data = pack(SNAPSHOT);
  const live: Camera3DParams = { ...CAM3D, aspect: 1000 / 600 };
  const vp = viewProjMatrix(mat4.create(), live);
  const ivp = invViewProj(mat4.create(), live);
  // The packed matrices match the shared camera math at GPU float precision.
  for (let i = 0; i < 16; i++) {
    assert.equal(data[d.memoryLayoutOf(Camera, (c) => c.viewProj).offset / 4 + i], f32(vp[i]));
    assert.equal(data[d.memoryLayoutOf(Camera, (c) => c.invViewProj).offset / 4 + i], f32(ivp[i]));
  }
  const eye = eyePosition(vec3.create(), live);
  const expected: [string, (c: CameraValue) => unknown, number][] = [
    ["eye.x", (c) => c.eye.x, eye[0]],
    ["eye.y", (c) => c.eye.y, eye[1]],
    ["eye.z", (c) => c.eye.z, eye[2]],
    ["znear", (c) => c.znear, 1],
    ["width", (c) => c.width, 1000],
    ["height", (c) => c.height, 600],
    ["time", (c) => c.time, 17.5],
  ];
  for (const [name, field, value] of expected) assert.equal(at(data, field), f32(value), name);
});
