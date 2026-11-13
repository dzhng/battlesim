// The Armory's orbit, driven by drag and wheel to every extreme round a unit
// with yard props beside it: the eye never enters the unit, a prop or the
// ground, the lowest pitch dips below the battle's floor, and the closest
// zoom keeps the whole unit in the stage's cell. The browser tour in
// `web/scenes/armory.mjs` runs the same extremes on the drawn page.
import { expect, test } from "vitest";
import { vec3, type Vec3 } from "math";
import {
  createProjectedPoint,
  eyePosition,
  projectPoint,
  viewProjMatrix,
} from "@packages/renderer-core/src/camera3d";
import { mat4 } from "math";
import { PITCH_LIMITS } from "@packages/renderer-core/src/cameraController";
import type { ObstacleBox } from "@packages/renderer-core/src/cameraObstacles";
import { presentationSettings } from "@web/presentationSettings";
import { ArmoryCamera, boundsBox, type Box } from "@apps/battle-lab/src/armory/armoryCamera";

const ORBIT_SPEED = (presentationSettings.camera_armory as { orbit_speed: number }).orbit_speed;
const VIEW: Box = { left: 0, top: 0, width: 1280, height: 800 };
/** The centre cell between the panels, narrower than it is tall. */
const CELL: Box = { left: 340, top: 12, width: 540, height: 776 };

/** A hull, a prop close behind it and one to its side. */
const BOUNDS = { min: [-3.2, -1.8, 0] as Vec3, max: [3.2, 1.8, 2.6] as Vec3 };
const PROPS: ObstacleBox[] = [
  { center: [-9, 4], yaw: 0.4, half: [3, 1.2, 1.3], baseZ: 0 },
  { center: [2, 8], yaw: 0, half: [1, 1, 0.5], baseZ: 0 },
];
const BOXES = [...PROPS, boundsBox(BOUNDS)];

function opened() {
  const camera = new ArmoryCamera();
  camera.frame({
    bounds: BOUNDS,
    pose: { target: [0, 0, 1], distance: 16, pitch: 0.18, yaw: -0.5 },
    boxes: BOXES,
  });
  settle(camera);
  return camera;
}

/** Frames until the eye has come to rest. */
function settle(camera: ArmoryCamera) {
  for (let k = 0; k < 120; k++) camera.presented(VIEW, CELL, VIEW.width, VIEW.height, 1 / 30);
}

/** Whether `p` stands inside `box`. */
function inside(p: Vec3, box: ObstacleBox) {
  const [dx, dy] = [p[0] - box.center[0], p[1] - box.center[1]];
  const [c, s] = [Math.cos(box.yaw), Math.sin(box.yaw)];
  const [lx, ly] = [dx * c + dy * s, dy * c - dx * s];
  return (
    Math.abs(lx) <= box.half[0] &&
    Math.abs(ly) <= box.half[1] &&
    p[2] >= box.baseZ &&
    p[2] <= box.baseZ + 2 * box.half[2]
  );
}

const eye = (camera: ArmoryCamera) => eyePosition(vec3.create(), camera.drawn()!) as Vec3;

test("at every extreme of drag and wheel the eye stays out of the unit, the props and the ground", () => {
  const camera = opened();
  const seen: string[] = [];
  for (let yaw = 0; yaw < 8; yaw++) {
    camera.dragBy(Math.PI / 4 / ORBIT_SPEED, 0);
    for (const tilt of [-10, 0.3, 10]) {
      camera.dragBy(0, tilt);
      for (const wheel of [-1e5, 1e5]) {
        camera.wheelBy(wheel);
        settle(camera);
        const p = eye(camera);
        const where = `yaw ${yaw} tilt ${tilt} wheel ${wheel}: ${p.map((v) => v.toFixed(2))}`;
        expect(p[2], where).toBeGreaterThan(0);
        for (const box of BOXES) expect(inside(p, box), where).toBe(false);
        seen.push(where);
      }
    }
  }
  expect(seen).toHaveLength(48);
});

test("the lowest pitch dips below the battle camera's floor", () => {
  const camera = opened();
  camera.dragBy(0, -10);
  settle(camera);
  expect(camera.drawn()!.pitch).toBeLessThan(PITCH_LIMITS[0]);
});

test("the closest zoom keeps the whole unit inside the cell", () => {
  const camera = opened();
  camera.wheelBy(-1e5);
  settle(camera);
  const orbit = camera.drawn()!;
  const viewProj = viewProjMatrix(mat4.create(), { ...orbit, aspect: VIEW.width / VIEW.height });
  for (const x of [BOUNDS.min[0], BOUNDS.max[0]])
    for (const y of [BOUNDS.min[1], BOUNDS.max[1]])
      for (const z of [BOUNDS.min[2], BOUNDS.max[2]]) {
        const q = projectPoint(createProjectedPoint(), viewProj, [x, y, z]);
        const px = ((q.ndc[0] + 1) / 2) * VIEW.width;
        const py = ((1 - q.ndc[1]) / 2) * VIEW.height;
        expect(px).toBeGreaterThanOrEqual(CELL.left);
        expect(px).toBeLessThanOrEqual(CELL.left + CELL.width);
        expect(py).toBeGreaterThanOrEqual(CELL.top);
        expect(py).toBeLessThanOrEqual(CELL.top + CELL.height);
      }
});
