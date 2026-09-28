// @vitest-environment node
// What the frame draws of a model: nothing off screen,
// a mesh tier by projected height, or its impostor card when far; which fog
// group it binds; and corpses as static instances, chunked for the thousands.
import { expect, test } from "vitest";
import { frustum } from "math/shapes";
import village from "@fixtures/village.json";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import { createDetailView, setDetailView } from "@packages/battle-renderer/src/frame/detailView";
import {
  CULLED,
  IMPOSTOR,
  SHADOW_MARGIN_M,
  chunkCorpses,
  chunkIsFar,
  detailAt,
  modelDetail,
  validateModelDetail,
  type ModelDetailPresentation,
} from "@packages/battle-renderer/src/models/modelDetail";
import { modelFog } from "@packages/battle-renderer/src/models/modelFog";
import {
  corpseInstances,
  poseFrameInstances,
  type ModelInstance,
} from "@packages/battle-renderer/src/models/modelInstances";
import type { PoseFrame } from "@packages/battle-renderer/src/models/poseDriver";

const DETAIL = validateModelDetail(village.presentation.models as ModelDetailPresentation);
const HEIGHT = 1080;
/** The village's default framing: 65 m from a target on the ground, looking north. */
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
const view = (distance: number) => setDetailView(createDetailView(), camera(distance), HEIGHT);
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

test("posed soldiers are units, never fogged; corpses take the ground's fog, buildings their faces'", () => {
  expect(modelFog({ kind: "skinned", clip: "idle", phase: 0, blend: null })).toBe("units");
  expect(modelFog({ kind: "corpse" })).toBe("ground");
  expect(modelFog({ kind: "static", state: "intact" })).toBe("faces");
});

test("corpses become static instances: no clip, the side's tint, and never a posed model", () => {
  const frame: PoseFrame = {
    soldiers: [
      {
        soldier: 1,
        unit: 3,
        kind: "rifle",
        slot: 0,
        side: "blue",
        position: [1, 2, 0],
        facing: 0.5,
        clip: "walk",
        phase: 0.25,
        blend: null,
      },
    ],
    vehicles: [],
    corpses: [{ soldier: 2, kind: "at", slot: 0, side: "red", position: [5, 5, 0], yaw: 1 }],
    corpsesVersion: 1,
  };
  const resolve = (kind: string, side: string) => ({
    appearance: kind,
    tint: (side === "red" ? [1.18, 1, 0.78] : [1, 1, 1]) as [number, number, number],
  });
  const models = poseFrameInstances([] as ModelInstance[], frame, resolve);
  expect(models).toHaveLength(1);
  expect(models[0]).toMatchObject({ appearance: "rifle", xray: null, yaw: 0.5 });
  expect(models[0].pose).toMatchObject({ kind: "skinned", clip: "walk", phase: 0.25 });
  expect(corpseInstances(frame, resolve)).toEqual([
    { appearance: "at", x: 5, y: 5, z: 0, yaw: 1, tint: [1.18, 1, 0.78] },
  ]);
});

test("a soldier wears the variant his id resolves to, alive and fallen", () => {
  const pose = (soldier: number) => ({
    soldier,
    unit: 3,
    kind: "rifle" as const,
    slot: 0,
    side: "blue" as const,
    position: [0, 0, 0] as [number, number, number],
    facing: 0,
    clip: "idle",
    phase: 0,
    blend: null,
  });
  const frame: PoseFrame = {
    soldiers: [pose(7), pose(8)],
    vehicles: [],
    corpses: [{ soldier: 7, kind: "rifle", slot: 0, side: "blue", position: [0, 0, 0], yaw: 0 }],
    corpsesVersion: 1,
  };
  const resolve = (kind: string, _side: string, id: number) => ({
    appearance: `${kind}_${id % 3}`,
    tint: [1, 1, 1] as [number, number, number],
  });
  const worn = poseFrameInstances([] as ModelInstance[], frame, resolve).map((m) => m.appearance);
  expect(worn).toEqual(["rifle_1", "rifle_2"]);
  expect(corpseInstances(frame, resolve)[0].appearance).toBe("rifle_1");
});

test("each unit's models are x-rayed in the colour presentation gives its unit, or not at all", () => {
  const soldier = (side: "blue" | "red", id: number) => ({
    soldier: id,
    unit: id,
    kind: "rifle" as const,
    slot: 0,
    side,
    position: [0, 0, 0] as [number, number, number],
    facing: 0,
    clip: "idle",
    phase: 0,
    blend: null,
  });
  const frame: PoseFrame = {
    soldiers: [soldier("blue", 1), soldier("red", 2)],
    vehicles: [],
    corpses: [],
    corpsesVersion: 0,
  };
  const resolve = (kind: string) => ({
    appearance: kind,
    tint: [1, 1, 1] as [number, number, number],
  });
  const OWN = [0.6, 0.8, 1, 0.5] as const;
  const SELECTED = [1, 0.9, 0.4, 0.6] as const;
  // The session's rule: the observing side's units, the selection in its own colour.
  const xray = (own: "blue" | "red", selected: number[] = []) =>
    poseFrameInstances([] as ModelInstance[], frame, resolve, (side, unit) =>
      side !== own ? null : selected.includes(unit) ? SELECTED : OWN,
    ).map((m) => m.xray);
  expect(xray("blue")).toEqual([OWN, null]);
  expect(xray("red", [2])).toEqual([null, SELECTED]);
  expect(poseFrameInstances([] as ModelInstance[], frame, resolve).map((m) => m.xray)).toEqual([
    null,
    null,
  ]);
});

test("corpses chunk by ground, and a far chunk draws whole as cards", () => {
  const n = 2000;
  const positions = new Float32Array(n * 3);
  const sizes = new Float32Array(n).fill(SOLDIER);
  for (let i = 0; i < n; i++) positions.set([(i % 50) * 7, Math.floor(i / 50) * 7, 0], i * 3);
  const { order, chunks } = chunkCorpses(positions, sizes, n);
  // Every corpse in exactly one chunk, chunks contiguous in `order`.
  expect([...order].sort((a, b) => a - b)).toEqual(Array.from({ length: n }, (_, i) => i));
  expect(chunks.reduce((k, c) => k + c.end - c.start, 0)).toBe(n);
  // From the strategic height every chunk is far; from the ground the one
  // under the camera is not.
  const high = setDetailView(createDetailView(), camera(2000, [175, 140, 0]), HEIGHT);
  expect(chunks.every((c) => chunkIsFar(DETAIL, high, c))).toBe(true);
  const low = setDetailView(createDetailView(), camera(25, [30, 30, 0]), HEIGHT);
  expect(chunks.some((c) => !chunkIsFar(DETAIL, low, c))).toBe(true);
});
