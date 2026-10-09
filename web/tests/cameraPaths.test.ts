// @vitest-environment node
// The camera paths the game ships, against the street test map's real buildings and
// ground under the fixture's own camera numbers.
import { readFileSync } from "node:fs";
import { beforeAll, expect, test } from "vitest";
import { loadMap } from "@web/maps/node";
import { TEST_RULES } from "./catalog";
import { gameCamera } from "@apps/battle-lab/src/gameCamera";
import {
  buildingObstacles,
  buildingPartProps,
} from "@packages/battle-renderer/src/buildingObstacles";
import { mapProps } from "@packages/battle-renderer/src/models/propAppearance";
import { readWorldExports, type WorldLayout } from "@packages/battle-renderer/src/worldMesh";
import { createClearanceState } from "@packages/renderer-core/src/cameraClearance";
import { CameraController } from "@packages/renderer-core/src/cameraController";
import type { CameraObstacles } from "@packages/renderer-core/src/cameraObstacles";
import * as wasm from "@wasm/game_wasm.js";
import { cityContactTour, sampleTour } from "../src/battle/benchmark/camera";
import { generationRequest } from "../src/maps/source";
import generated from "@fixtures/generated-battle.json";
import { CITY_CONTACT } from "../src/battle/benchmark/presets";
import { WHOLE_MAP_MS } from "./support/wholeMap";

const streetMap = loadMap("street").definition;

let obstacles: CameraObstacles;
let rig: CameraController;

beforeAll(() => {
  wasm.initSync({
    module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)),
  });
  const rules = JSON.stringify(TEST_RULES);
  const world = new wasm.WorldView(JSON.stringify(streetMap), rules);
  const exports = readWorldExports(world);
  const ground = (x: number, y: number) => world.surface_at(x, y)[0] ?? 0;
  obstacles = buildingObstacles(
    mapProps(exports, JSON.parse(wasm.world_layout(rules)) as WorldLayout),
    [],
    buildingPartProps(exports.buildings),
    ground,
  );
  rig = new CameraController(gameCamera.config, ground);
});

test("a test map's buildings are the camera's obstacles", () => {
  // The street's three-storey shops, 12.65 m to the roof: the ceiling is the tallest.
  expect(obstacles.ceiling).toBe(Math.fround(12.65));
  expect(obstacles.clear([975, 752, 6], 0.1)).toBe(false);
  expect(obstacles.clear([975, 752, 14], 0.1)).toBe(true);
});

test("the opening framing is drawn as authored", () => {
  const opening = { ...gameCamera.opening(), aspect: 16 / 9 };
  expect(rig.resolve(createClearanceState(), opening, 0, obstacles)).toBe(opening);
});

test("the battle opening keeps the blue deployment edge in frame", () => {
  const start: [number, number] = [0, 500];
  const camera = gameCamera.fromStart(start, [1000, 1000]);
  expect(camera.target[0]).toBeGreaterThan(start[0]);
  expect(camera.target[0]).toBeLessThan(500);
  expect(camera.distance).toBeGreaterThan(gameCamera.opening().distance);
});

test(
  "the city benchmark tour is drawn as flown over the generated buildings",
  () => {
    const rules = JSON.stringify(TEST_RULES);
    const documents = {
      presets: readFileSync(new URL("../../fixtures/map-presets.json", import.meta.url), "utf8"),
      templates: readFileSync(
        new URL("../../fixtures/prototype-building-templates.json", import.meta.url),
        "utf8",
      ),
    };
    const request = generationRequest(wasm, CITY_CONTACT.generated, documents, generated.limits);
    const result = JSON.parse(
      wasm.generate_map(JSON.stringify(request), documents.presets, documents.templates, rules),
    );
    expect(result.status).toBe("ok");
    const world = new wasm.WorldView(JSON.stringify(result.result.map), rules);
    try {
      const exports = readWorldExports(world);
      const ground = (x: number, y: number) => world.surface_at(x, y)[0] ?? 0;
      const obstacles = buildingObstacles(
        mapProps(exports, JSON.parse(wasm.world_layout(rules)) as WorldLayout),
        [],
        buildingPartProps(exports.buildings),
        ground,
      );
      const size: [number, number] = result.result.map.size;
      const rendered = JSON.parse(world.extents()).rendered;
      const rig = new CameraController(gameCamera.forMap(size, rendered), ground);
      const tour = cityContactTour(size, rendered);
      for (const durationMs of Object.values(CITY_CONTACT.durationMs)) {
        for (const aspect of [16 / 9, 8 / 5]) {
          const state = createClearanceState();
          let asked = { ...gameCamera.opening(), aspect };
          for (let ms = 0; ms <= durationMs; ms += 1000 / 30) {
            const { pose } = sampleTour(tour, ms, durationMs);
            asked = rig.place(asked, pose);
            expect(asked).toMatchObject({
              ...pose,
              target: [...pose.target, ground(...pose.target)],
            });
            const drawn = rig.resolve(state, asked, 1 / 30, obstacles);
            const at = `at ${(ms / 1000).toFixed(2)} / ${durationMs / 1000} s (${state.hold}; aspect ${aspect})`;
            expect(drawn.target, at).toEqual(asked.target);
            for (const axis of ["distance", "pitch", "yaw"] as const)
              expect(Math.abs(drawn[axis] - asked[axis]), `${axis} ${at}`).toBeLessThan(1e-6);
          }
        }
      }
    } finally {
      world.free();
    }
  },
  WHOLE_MAP_MS,
);
