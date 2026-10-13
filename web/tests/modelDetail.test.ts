// @vitest-environment node
// What the frame draws of a model: nothing off screen,
// a mesh tier by projected height, or its impostor card when far; which fog
// group it binds.
import { expect, test } from "vitest";
import { frustum } from "math/shapes";
import game from "@fixtures/game.json";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { createDetailView, setDetailView } from "@packages/battle-renderer/src/frame/detailView";
import {
  CULLED,
  IMPOSTOR,
  SHADOW_MARGIN_M,
  detailAt,
  modelDetail,
  validateModelDetail,
  type ModelDetailPresentation,
} from "@packages/battle-renderer/src/models/modelDetail";
import { loadPresentation, ScenePresentation } from "@web/battle/sim/presentation";
import { presentedCamera } from "../../apps/battle-lab/src/presentedFrame";

const DETAIL = validateModelDetail(game.presentation.models as ModelDetailPresentation);
const HEIGHT = 1080;
/** The street test map's default framing: 65 m from a target on the ground, looking north. */
const camera = (
  distance: number,
  target: [number, number, number] = [0, 0, 0],
): Camera3DParams => ({
  target,
  distance,
  pitch: 0.85,
  yaw: -Math.PI / 2,
  fovY: 0.8,
  aspect: 16 / 9,
  near: 1,
});
await loadPresentation();
const presentation = new ScenePresentation();
/** The detail view of the frame the presentation makes for the camera. */
const view = (distance: number) => {
  const camera3d = camera(distance);
  const ui = { selected: [], xray: false, ghosts: [] };
  const frame = presentation.frame(
    0,
    presentedCamera({ camera3d, width: Math.round(HEIGHT * camera3d.aspect), height: HEIGHT }),
    ui,
    0,
  );
  return setDetailView(createDetailView(), frame.view.f32);
};
const SOLDIER = 1.8;

test("a soldier's tier falls with his projected height, to a card only if he has one", () => {
  const near = view(25);
  const at = (d: number, impostor = true) => detailAt(DETAIL, near, SOLDIER, d, impostor);
  const px = (d: number) => (SOLDIER * near.pixelsPerMetre) / d;
  const tiers = [10, 20, 40, 80, 160, 400].map((d) => [Math.round(px(d)), at(d)]);
  // Each step outward is never finer than the one before.
  for (let i = 1; i < tiers.length; i++)
    expect(tiers[i][1]).toBeGreaterThanOrEqual(tiers[i - 1][1]);
  expect(at(10)).toBe(0);
  expect(at(400)).toBe(IMPOSTOR);
  // No atlas: the coarsest mesh stands in, never nothing.
  expect(at(400, false)).toBe(3);
  // The fixture's thresholds decide the edges exactly.
  const dAt = (p: number) => (SOLDIER * near.pixelsPerMetre) / p;
  expect(at(dAt(DETAIL.lod_px[0] + 1))).toBe(0);
  expect(at(dAt(DETAIL.lod_px[0] - 1))).toBe(1);
  expect(at(dAt(DETAIL.impostor_px + 0.5))).toBe(3);
  expect(at(dAt(DETAIL.impostor_px - 0.5))).toBe(IMPOSTOR);
});

test("models off screen are culled, but not those whose shadow can reach into view", () => {
  const v = view(65);
  // The camera looks north (+y) from the south: the target is on screen,
  // a point far to the east is not; one just past the side edge still draws.
  expect(modelDetail(DETAIL, v, 0, 0, 0, SOLDIER, 1, true)).not.toBe(CULLED);
  expect(modelDetail(DETAIL, v, 400, 0, 0, SOLDIER, 1, true)).toBe(CULLED);
  let edge = 0;
  while (modelDetail(DETAIL, v, edge, 0, 0, SOLDIER, 1, true) !== CULLED) edge += 0.25;
  // Culled only past where his own sphere leaves the view by the shadow margin.
  let bare = 0;
  while (frustum.sidesIntersectsSphere(v.sides, { center: [bare, 0, SOLDIER / 2], radius: 1 }))
    bare += 0.25;
  expect(edge - bare).toBeGreaterThanOrEqual(SHADOW_MARGIN_M * 0.9);
});

test("an airframe off screen still draws while its shadow, cast far below it, is in view", () => {
  const v = view(65);
  const sun = {
    azimuth: game.presentation.light.sun_azimuth,
    elevation: game.presentation.light.sun_elevation,
  };
  const fall = (lift: number) => [
    (-Math.cos(sun.azimuth) * lift) / Math.tan(sun.elevation),
    (-Math.sin(sun.azimuth) * lift) / Math.tan(sun.elevation),
  ];
  // 20 m up, it throws its shadow this far across the ground.
  const [fx, fy] = fall(20);
  expect(Math.hypot(fx, fy)).toBeGreaterThan(4 * SHADOW_MARGIN_M);
  // An airframe just far enough out of view that its own sphere is culled,
  // on the side its shadow falls back toward the view's centre from.
  const at = (k: number): [number, number, number] => [-fx * k, -fy * k, 20];
  let k = 0;
  while (modelDetail(DETAIL, v, ...at(k), 3.2, 8, true) !== CULLED) k += 0.05;
  const [x, y, z] = at(k);
  expect(modelDetail(DETAIL, v, x, y, z, 3.2, 8, true, [x + fx, y + fy, 0])).not.toBe(CULLED);
  // Its tier is still its own size at its own distance, not its shadow's.
  expect(modelDetail(DETAIL, v, x, y, z, 3.2, 8, true, [x + fx, y + fy, 0])).toBe(
    detailAt(DETAIL, v, 3.2, Math.hypot(x - v.eye[0], y - v.eye[1], z + 1.6 - v.eye[2]), true),
  );
});
