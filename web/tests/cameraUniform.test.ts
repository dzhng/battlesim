// @vitest-environment node
import assert from "node:assert/strict";
import { test } from "vitest";
import { d } from "typegpu";
import { Camera } from "@packages/battle-renderer/src/world/camera.ts";
import { CAMERA_WORDS } from "@packages/battle-renderer/src/records.gen.ts";
import {
  CAMERA_UNIFORM_BYTES,
  CAMERA_UNIFORM_FLOATS,
  packCameraUniform,
} from "@packages/renderer-core/src/cameraUniform.ts";

const VIEW_PROJ = Array.from({ length: 16 }, (_, i) => 1 + i);
const INVERSE = Array.from({ length: 16 }, (_, i) => 101 + i);
// Every scalar distinct, so a field read from the wrong slot shows.
const pack = () =>
  packCameraUniform(
    new Float32Array(CAMERA_UNIFORM_FLOATS),
    VIEW_PROJ,
    INVERSE,
    [7, 8, 9],
    1,
    1000,
    600,
    17.5,
  );
type CameraValue = d.Infer<typeof Camera>;
/** The float the packed buffer holds where the WGSL struct reads `field`. */
const at = (data: Float32Array, field: (c: CameraValue) => unknown) =>
  data[d.memoryLayoutOf(Camera, field).offset / 4];

test("cameraUniform: the packed buffer and the frame's camera record are exactly the WGSL Camera struct", () => {
  assert.equal(d.sizeOf(Camera), CAMERA_UNIFORM_BYTES);
  assert.equal(CAMERA_WORDS * 4, CAMERA_UNIFORM_BYTES);
  assert.equal(pack().byteLength, CAMERA_UNIFORM_BYTES);
});

test("cameraUniform: each WGSL field reads the value packed for it", () => {
  const data = pack();
  for (let i = 0; i < 16; i++) {
    assert.equal(data[d.memoryLayoutOf(Camera, (c) => c.viewProj).offset / 4 + i], VIEW_PROJ[i]);
    assert.equal(data[d.memoryLayoutOf(Camera, (c) => c.invViewProj).offset / 4 + i], INVERSE[i]);
  }
  const expected: [string, (c: CameraValue) => unknown, number][] = [
    ["eye.x", (c) => c.eye.x, 7],
    ["eye.y", (c) => c.eye.y, 8],
    ["eye.z", (c) => c.eye.z, 9],
    ["znear", (c) => c.znear, 1],
    ["width", (c) => c.width, 1000],
    ["height", (c) => c.height, 600],
    ["time", (c) => c.time, 17.5],
  ];
  for (const [name, field, value] of expected) assert.equal(at(data, field), value, name);
});
