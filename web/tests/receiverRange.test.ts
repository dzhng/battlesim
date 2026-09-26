// @vitest-environment node
// Cascades split over where the map is in view depth, not from the camera's
// near plane (spike 01, landmine 1).
import { expect, test } from "vitest";
import { mapBox, receiverRange } from "@packages/battle-renderer/src/frame/receiverRange.ts";
import { MeshBuilder } from "@packages/battle-renderer/src/mesh.ts";
import { mat4, vec2, vec3, type Vec3 } from "math";
import type { Box3 } from "math/shapes";
import { viewMatrix, type Camera3DParams } from "@packages/renderer-core/src/camera3d.ts";

/** A flat 1600 m map with one 20 m rise, as world triangles. */
function map() {
  const mesh = new MeshBuilder();
  mesh.box(800, 800, 10, 800, 800, 10, [0.4, 0.6, 0.3, 1]);
  return mesh.build();
}

const camera = (over: Partial<Camera3DParams>): Camera3DParams => ({
  target: [800, 800, 0],
  distance: 1000,
  pitch: 0.85,
  yaw: -1.57,
  fovY: 0.8,
  aspect: 16 / 9,
  near: 1,
  ...over,
});

/** View depth of a world point. */
const depthOf = (cam: Camera3DParams, p: Vec3) =>
  -vec3.transformMat4(vec3.create(), p, viewMatrix(mat4.create(), cam))[2];
const rangeOf = (cam: Camera3DParams, box: Box3 | null) => receiverRange(vec2.create(), cam, box);

test("a high camera's receiver range starts at the map, not at its near plane", () => {
  const cam = camera({});
  const [near, far] = rangeOf(cam, mapBox(map()));
  const target = depthOf(cam, cam.target);
  // The map fills the view: its nearest visible point is hundreds of metres out.
  expect(near).toBeGreaterThan(500);
  expect(near).toBeLessThan(target);
  expect(far).toBeGreaterThan(target);
});

test("the range widens as the camera drops toward the ground", () => {
  const high = rangeOf(camera({}), mapBox(map()));
  const low = rangeOf(camera({ distance: 25, pitch: 0.22 }), mapBox(map()));
  expect(low[0]).toBeLessThan(high[0]);
  expect(low[1] / low[0]).toBeGreaterThan(high[1] / high[0]);
});

test("without a map, or looking away from it, the range collapses past the near plane", () => {
  expect(rangeOf(camera({}), null)).toEqual([1, 2]);
  const away = camera({ target: [800, 800, 5000], pitch: -0.3, distance: 100 });
  expect(rangeOf(away, mapBox(map()))).toEqual([1, 2]);
});
