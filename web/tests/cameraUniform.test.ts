// @vitest-environment node
import assert from "node:assert/strict";
import { test } from "vitest";
import { d } from "typegpu";
import { Camera } from "@packages/battle-renderer/src/scene.ts";
import {
  cameraUniformData,
  CAMERA_UNIFORM_BYTES,
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
const SNAPSHOT: CameraSnapshot = { camera3d: CAM3D, width: 1000, height: 600 };

const f32 = (value: number) => new Float32Array([value])[0];

test("cameraUniform: byte size matches the WGSL Camera struct the scene binds", () => {
  assert.equal(d.sizeOf(Camera), CAMERA_UNIFORM_BYTES);
  assert.equal(cameraUniformData(SNAPSHOT).byteLength, CAMERA_UNIFORM_BYTES);
});

test("cameraUniform: packed matrices and eye equal camera3d at the live aspect", () => {
  const data = cameraUniformData(SNAPSHOT);
  const resolved: Camera3DParams = { ...CAM3D, aspect: 1000 / 600 };
  const vp = viewProjMatrix(resolved);
  const ivp = invViewProj(resolved);
  const eye = eyePosition(resolved);
  for (let i = 0; i < 16; i++) assert.equal(data[i], vp[i], `viewProj[${i}]`);
  for (let i = 0; i < 16; i++) assert.equal(data[16 + i], ivp[i], `invViewProj[${i}]`);
  assert.deepEqual([data[32], data[33], data[34]], eye.map(f32));
  assert.equal(data[35], 1);
  assert.equal(data[36], 1000);
  assert.equal(data[37], 600);
});
