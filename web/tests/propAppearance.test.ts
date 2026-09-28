// @vitest-environment node
// The prop seam: each simulation prop draws as an installed appearance fitted
// to its box, and a side draws what it knows — a building stands until the
// side learns its ruin, and the ruin replaces it in the same list.
import { expect, test } from "vitest";
import type { Vec3 } from "math";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader.ts";
import type { AppearanceUnit, StaticBundle } from "@packages/scene-assets/src/schema.ts";
import {
  PropAppearances,
  structureModels,
  type KnownProp,
  type MapProp,
} from "@packages/battle-renderer/src/models/propAppearance.ts";
import type { ModelInstance } from "@packages/battle-renderer/src/models/modelInstances.ts";

const bundle = (...states: string[]): StaticBundle => {
  const bounds = { min: [0, 0, 0] as Vec3, max: [1, 1, 1] as Vec3 };
  const tier = {
    positions: new Float32Array(9),
    normals: new Int16Array(12),
    uvs: new Float32Array(6),
    colors: new Uint8Array(12),
    indices: Uint16Array.of(0, 1, 2),
    draws: [{ material: 0, first: 0, count: 3 }],
  };
  return {
    kind: "static",
    states: states.map((name) => ({ name, tiers: [tier, tier, tier, tier], bounds })),
    materials: [],
    textures: [],
    bounds,
  };
};

const entry = (
  unit: AppearanceUnit,
  scenery: string | null,
  footprint: Vec3,
  ...states: string[]
) => ({
  unit,
  scenery,
  footprint,
  mounts: null,
  bundle: bundle(...states),
});

const installed: InstalledAppearances = {
  generation: 1,
  sides: { blue: [1, 1, 1], red: [1, 1, 1] },
  skeletons: new Map(),
  appearances: new Map([
    ["house_a", entry("building", null, [15, 12, 4], "intact", "ruin")],
    ["house_c", entry("building", null, [13, 11, 4], "intact", "ruin")],
    ["tank_wreck", entry("scenery", "wreck", [3.5, 1.8, 1.2], "default")],
    ["truck_wreck", entry("scenery", "wreck", [3, 1.4, 1.8], "default")],
    ["village_ruin", entry("scenery", "ruin", [15, 12, 1], "default")],
    ["field_wall", entry("scenery", "wall", [2, 0.3, 0.8], "default")],
    ["bridge_deck", entry("scenery", "bridge_deck", [18, 5, 0.4], "default")],
  ]),
};
const appearances = new PropAppearances(installed);

const building = (id: number, x: number, half: Vec3): MapProp => ({
  id,
  kind: "building",
  center: [x, 50],
  yaw: 0.3,
  half,
  baseZ: 2,
});
const houses = [building(0, 100, [15, 12, 4]), building(1, 200, [13, 11, 4])];
const ruinOf = (b: MapProp): KnownProp => ({
  kind: "ruin",
  center: b.center,
  yaw: b.yaw,
  half: [b.half[0], b.half[1], 1],
  baseZ: b.baseZ,
  replaces: b.id,
});
const state = (m: ModelInstance) => (m.pose.kind === "static" ? m.pose.state : m.pose.kind);

test("a building stands until the side knows it fell: an unseen collapse stays unseen", () => {
  const drawn = structureModels(houses, [], appearances);
  expect(drawn.map((m) => [m.appearance, state(m)])).toEqual([
    ["house_a", "intact"],
    ["house_c", "intact"],
  ]);
});

test("a known ruin replaces its building atomically: its own appearance, ruined, in its place", () => {
  const drawn = structureModels(houses, [ruinOf(houses[1])], appearances);
  expect(drawn.map((m) => [m.appearance, state(m)])).toEqual([
    ["house_a", "intact"],
    ["house_c", "ruin"],
  ]);
  const ruin = drawn[1];
  expect([ruin.x, ruin.y, ruin.z, ruin.yaw]).toEqual([200, 50, 2, 0.3]);
  // Its plan is the building's; its height is the one it was authored to.
  expect(ruin.scale).toEqual([1, 1, 1]);
});

test("a map prop the side saw destroyed with nothing in its place is drawn no more", () => {
  // 34c: a crate blown away; its known entry only removes it.
  const gone: KnownProp = { ...ruinOf(houses[1]), kind: "building", destroyed: true };
  const drawn = structureModels(houses, [gone], appearances);
  expect(drawn.map((m) => [m.appearance, state(m)])).toEqual([["house_a", "intact"]]);
});

test("a placed prop takes the appearance nearest its box, scaled to fit it", () => {
  const wreck = (half: Vec3): KnownProp => ({
    kind: "tank_wreck",
    center: [10, 20],
    yaw: 1,
    half,
    baseZ: 0,
    replaces: null,
  });
  const [tank] = structureModels([], [wreck([3.5, 1.8, 1.2])], appearances);
  expect(tank.appearance).toBe("tank_wreck");
  expect(tank.scale).toEqual([1, 1, 1]);
  const [truck] = structureModels([], [wreck([3, 1.4, 1.8])], appearances);
  expect(truck.appearance).toBe("truck_wreck");
  // A deck longer and wider than the authored one stretches to its box.
  const deck: MapProp = {
    id: 9,
    kind: "bridge_deck",
    center: [0, 0],
    yaw: 0,
    half: [27, 5, 0.4],
    baseZ: 1,
  };
  const [drawn] = structureModels([deck], [], appearances);
  expect(drawn.appearance).toBe("bridge_deck");
  expect(drawn.scale).toEqual([1.5, 1, 1]);
  // A ruin with no building of its own is the generic ruin, fitted.
  const [loose] = structureModels(
    [],
    [{ ...ruinOf(houses[0]), half: [7.5, 6, 1], replaces: null }],
    appearances,
  );
  expect(loose.appearance).toBe("village_ruin");
  expect(loose.scale).toEqual([0.5, 0.5, 1]);
});

test("a wall repeats its module along the box's long side instead of stretching it", () => {
  const wall = (half: Vec3, yaw: number): MapProp => ({
    id: 3,
    kind: "wall",
    center: [0, 0],
    yaw,
    half,
    baseZ: 0,
  });
  const along = structureModels([wall([10, 0.3, 0.8], 0)], [], appearances);
  expect(along).toHaveLength(5);
  expect(along.map((m) => m.x)).toEqual([-8, -4, 0, 4, 8]);
  for (const m of along) {
    expect(m.scale).toEqual([1, 1, 1]);
    expect(m.y).toBeCloseTo(0, 9);
  }
  // Long along its box's y: each module turns to run along it.
  const across = structureModels([wall([0.3, 6, 0.8], 0)], [], appearances);
  expect(across).toHaveLength(3);
  for (const m of across) {
    expect(m.yaw).toBeCloseTo(Math.PI / 2, 9);
    expect(m.x).toBeCloseTo(0, 9);
  }
  expect(across.map((m) => Math.round(m.y))).toEqual([-4, 0, 4]);
});

test("a map prop a route draws from the world is left to it", () => {
  const drawn = structureModels(houses, [ruinOf(houses[0])], appearances, () => false);
  // Only the known ruin: the standing buildings are the world's.
  expect(drawn.map((m) => [m.appearance, state(m)])).toEqual([["house_a", "ruin"]]);
});

test("the battle installs the map's buildings and every body a battle can leave or place", () => {
  expect([...appearances.drawnFor(houses)].sort()).toEqual([
    "field_wall",
    "house_a",
    "house_c",
    "tank_wreck",
    "truck_wreck",
    "village_ruin",
  ]);
});
