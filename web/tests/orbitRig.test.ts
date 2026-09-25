// @vitest-environment node
import { expect, test } from "vitest";
import { projectPoint, type Camera3DParams } from "@packages/renderer-core/src/camera3d.ts";
import { orbitCamera, panCamera, zoomCamera } from "@packages/renderer-core/src/orbitRig.ts";

const CAM: Camera3DParams = {
  target: [0, 0, 0],
  distance: 100,
  pitch: 0.7,
  yaw: 1.1,
  fovY: 0.8,
  aspect: 1.6,
  near: 1,
};

test("pan right moves the view so the old centre slides left on screen", () => {
  const panned = panCamera(CAM, 0.1, 0);
  expect(projectPoint(panned, [0, 0, 0]).ndc[0]).toBeLessThan(-0.01);
  expect(Math.abs(projectPoint(panned, [0, 0, 0]).ndc[1])).toBeLessThan(1e-4);
});

test("pan forward moves the view so the old centre slides down on screen", () => {
  expect(projectPoint(panCamera(CAM, 0, 0.1), [0, 0, 0]).ndc[1]).toBeLessThan(-0.01);
});

test("orbit pitch and zoom stay within their limits", () => {
  expect(orbitCamera(CAM, 0, 10).pitch).toBeLessThan(Math.PI / 2);
  expect(orbitCamera(CAM, 0, -10).pitch).toBeGreaterThan(0);
  expect(zoomCamera(CAM, 1e-6).distance).toBeGreaterThan(0);
});
