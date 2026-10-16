// @vitest-environment node
import { beforeAll, expect, test } from "vitest";
import { loadPresentation } from "@web/battle/sim/presentation";
import { sectorPan } from "@packages/battle-audio/src/soundFrame";
import { cameraListener } from "@packages/battle-audio/src/battleAudio";
import { describeCue } from "../src/battle/present/captions";

beforeAll(() => loadPresentation());

test("captions carry exactly the cue's category, band and direction", () => {
  const text = describeCue(
    { listener: 2, category: "vehicle", sector: 1, band: "far", moving: true },
    "rifle #2",
  );
  expect(text).toBe("Heard engine, moving, far, north-east of rifle #2");
});

test("a cue's stereo pan follows the camera: east is right when looking north", () => {
  const north = [0, 1, 0];
  expect(sectorPan(0, north)).toBeCloseTo(1);
  expect(sectorPan(4, north)).toBeCloseTo(-1);
  expect(sectorPan(2, north)).toBeCloseTo(0);
  // Turn to look east: north is now on the left.
  expect(sectorPan(2, [1, 0, 0])).toBeCloseTo(-1);
});

test("cameraListener places listener between target and eye based on share", () => {
  const camera = {
    target: [100, 200, 0] as [number, number, number],
    distance: 100,
    pitch: -Math.PI / 4,
    yaw: 0,
  };
  const listener = cameraListener(camera, 0.25);
  expect(listener.position).toHaveLength(3);
  expect(listener.forward).toHaveLength(3);
  const atTarget = cameraListener(camera, 0).position;
  const atEye = cameraListener(camera, 1).position;
  expect(atTarget[0]).toBeCloseTo(camera.target[0]);
  expect(atTarget[1]).toBeCloseTo(camera.target[1]);
  expect(atTarget[2]).toBeCloseTo(camera.target[2]);
  const dist0 = Math.hypot(
    listener.position[0] - atTarget[0],
    listener.position[1] - atTarget[1],
    listener.position[2] - atTarget[2],
  );
  const dist1 = Math.hypot(
    listener.position[0] - atEye[0],
    listener.position[1] - atEye[1],
    listener.position[2] - atEye[2],
  );
  expect(dist0).toBeLessThan(dist1);
});

test("cameraListener forward vector depends on yaw", () => {
  const camera1 = {
    target: [0, 0, 0] as [number, number, number],
    distance: 100,
    pitch: -Math.PI / 4,
    yaw: 0,
  };
  const camera2 = {
    target: [0, 0, 0] as [number, number, number],
    distance: 100,
    pitch: -Math.PI / 4,
    yaw: Math.PI / 2,
  };
  const forward1 = cameraListener(camera1, 0.25).forward;
  const forward2 = cameraListener(camera2, 0.25).forward;
  expect(Math.hypot(forward1[0], forward1[1])).toBeGreaterThan(0);
  expect(Math.hypot(forward2[0], forward2[1])).toBeGreaterThan(0);
  expect(forward1[0]).not.toBeCloseTo(forward2[0]);
});
