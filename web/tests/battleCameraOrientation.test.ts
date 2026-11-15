// @vitest-environment node
import assert from "node:assert/strict";
import { test } from "vitest";
import { gameCamera } from "@apps/battle-lab/src/gameCamera";
import { CameraController, type CameraIntent } from "@packages/renderer-core/src/cameraController";
import {
  createGpuMat4,
  createProjectedPoint,
  projectPoint,
  viewProjMatrix,
} from "@packages/renderer-core/src/camera3d";

const SIZE = [1000, 1000] as const;

function screen(camera: ReturnType<typeof gameCamera.fromStart>, point: readonly [number, number]) {
  const projection = projectPoint(
    createProjectedPoint(),
    viewProjMatrix(createGpuMat4(), { ...camera, aspect: 16 / 9 }),
    [point[0], point[1], 0],
  );
  return projection.ndc;
}

function inward(start: readonly [number, number]): [number, number] {
  const delta = [SIZE[0] / 2 - start[0], SIZE[1] / 2 - start[1]];
  const length = Math.hypot(delta[0], delta[1]);
  return [start[0] + (delta[0] / length) * 80, start[1] + (delta[1] / length) * 80];
}

test("battle openings put either commander's entry edge below the map", () => {
  for (const start of [
    [30, SIZE[1] / 2],
    [SIZE[0] - 30, SIZE[1] / 2],
    [SIZE[0] / 2, 30],
    [SIZE[0] / 2, SIZE[1] - 30],
  ] as const) {
    const camera = gameCamera.fromStart(start, SIZE);
    const inside = inward(start);
    assert.ok(
      screen(camera, start)[1] < screen(camera, inside)[1],
      `entry ${start} did not project below its inward road: ${camera.yaw}`,
    );
  }
});

test("battle camera controls use the opening view's screen axes", () => {
  const controller = new CameraController(gameCamera.config);
  for (const start of [
    [30, SIZE[1] / 2],
    [SIZE[0] - 30, SIZE[1] / 2],
    [SIZE[0] / 2, 30],
    [SIZE[0] / 2, SIZE[1] - 30],
  ] as const) {
    const camera = gameCamera.fromStart(start, SIZE);
    const cases: readonly [string, CameraIntent, readonly number[]][] = [
      ["up", { held: new Set(["KeyW"]) }, [0, 1]],
      ["down", { held: new Set(["KeyS"]) }, [0, -1]],
      ["left", { held: new Set(["KeyA"]) }, [-1, 0]],
      ["right", { held: new Set(["KeyD"]) }, [1, 0]],
      ["edge up", { edge: [0, 1] }, [0, 1]],
      ["edge down", { edge: [0, -1] }, [0, -1]],
      ["edge left", { edge: [-1, 0] }, [-1, 0]],
      ["edge right", { edge: [1, 0] }, [1, 0]],
    ];
    for (const [name, intent, axis] of cases) {
      const next = controller.step(camera, intent, 0.25);
      const delta = screen(camera, [next.target[0], next.target[1]]);
      assert.ok(
        delta[0] * axis[0] + delta[1] * axis[1] > 0,
        `${name} moved in map coordinates for yaw ${camera.yaw}`,
      );
    }
  }
});
