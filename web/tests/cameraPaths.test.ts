// @vitest-environment node
// The camera paths the game ships, against the village's real buildings and
// ground under the fixture's own camera numbers.
import { readFileSync } from "node:fs";
import { beforeAll, expect, test } from "vitest";
import { loadMap } from "@web/maps/node";
import { VILLAGE_RULES } from "@apps/battle-lab/src/scenarios";
import { villageCamera } from "@apps/battle-lab/src/villageCamera";
import {
  buildingObstacles,
  buildingPartProps,
} from "@packages/battle-renderer/src/buildingObstacles";
import { mapProps } from "@packages/battle-renderer/src/models/propAppearance";
import { readWorldExports, type WorldLayout } from "@packages/battle-renderer/src/worldMesh";
import { createClearanceState } from "@packages/renderer-core/src/cameraClearance";
import { CameraController } from "@packages/renderer-core/src/cameraController";
import type { CameraObstacles } from "@packages/renderer-core/src/cameraObstacles";
import { initSync, WorldView, world_layout } from "@wasm/game_wasm.js";
import { sampleTour } from "../src/battle/benchmark/camera";
import { VILLAGE_CONTACT } from "../src/battle/benchmark/scenario";

const villageMap = loadMap("village").definition;

let obstacles: CameraObstacles;
let rig: CameraController;

beforeAll(() => {
  initSync({ module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)) });
  const rules = JSON.stringify(VILLAGE_RULES);
  const world = new WorldView(JSON.stringify(villageMap), rules);
  const exports = readWorldExports(world);
  const ground = (x: number, y: number) => world.surface_at(x, y)[0] ?? 0;
  obstacles = buildingObstacles(
    mapProps(exports, JSON.parse(world_layout(rules)) as WorldLayout),
    [],
    buildingPartProps(exports.buildings),
    ground,
  );
  rig = new CameraController(villageCamera.config, ground);
});

test("the village's buildings are the camera's obstacles", () => {
  expect(obstacles.ceiling).toBe(8);
  expect(obstacles.clear([975, 752, 6], 0.1)).toBe(false);
  expect(obstacles.clear([975, 752, 12], 0.1)).toBe(true);
});

test("the benchmark tour is drawn as flown: clearance moves none of its frames", () => {
  const { tour, durationMs } = VILLAGE_CONTACT;
  const lens = { ...villageCamera.opening(), aspect: 16 / 9 };
  for (const hz of [30, 120]) {
    const state = createClearanceState();
    let asked = lens;
    for (let ms = 0; ms <= durationMs.short; ms += 1000 / hz) {
      asked = rig.place(asked, sampleTour(tour, ms, durationMs.short).pose);
      const drawn = rig.resolve(state, asked, 1 / hz, obstacles);
      expect(drawn, `at ${(ms / 1000).toFixed(2)} s (${state.hold})`).toBe(asked);
    }
  }
});

test("the village's opening framing is drawn as authored", () => {
  const opening = { ...villageCamera.opening(), aspect: 16 / 9 };
  expect(rig.resolve(createClearanceState(), opening, 0, obstacles)).toBe(opening);
});
