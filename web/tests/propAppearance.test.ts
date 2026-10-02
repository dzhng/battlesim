// @vitest-environment node
// The prop seam: each simulation prop draws as an installed scenery appearance
// fitted to its box, and a side draws what it knows: a wall stands until the
// side learns of its rubble, and the rubble replaces it in the same list. A
// building's parts are not props drawn here (`buildingPlacements.test.ts`).
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
import { modelFog } from "@packages/battle-renderer/src/models/modelFog.ts";
import { UNITS } from "@packages/scene-assets/src/shippedUnits.ts";

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
    ["tank_wreck", entry("scenery", "wreck", [3.5, 1.8, 1.2], "default")],
    ["truck_wreck", entry("scenery", "wreck", [3, 1.4, 1.8], "default")],
    ["village_ruin", entry("scenery", "ruin", [15, 12, 1], "default")],
    ["field_wall", entry("scenery", "wall", [2, 0.3, 0.8], "default")],
    ["bridge_deck", entry("scenery", "bridge_deck", [18, 5, 0.4], "default")],
  ]),
};
/** The shipped prop types' bindings and blocking, as the world layout
 *  carries them. */
const props = Object.entries(UNITS.view.props);
const layout = {
  propAppearance: Object.fromEntries(props.map(([id, t]) => [id, t.appearance])),
  blockingPropKinds: Object.fromEntries(
    (["infantry", "vehicle"] as const).map((mover) => [
      mover,
      props.filter(([, t]) => t.body.blocks[mover]).map(([id]) => id),
    ]),
  ),
};
const appearances = new PropAppearances(installed, layout);

/** A one-module stretch of wall, 4 m long. */
const wallAt = (id: number, x: number): MapProp => ({
  id,
  kind: "wall",
  center: [x, 50],
  yaw: 0.3,
  half: [2, 0.3, 0.8],
  baseZ: 2,
});
const walls = [wallAt(0, 100), wallAt(1, 200)];
/** The rubble a wall falls to, on its plan. */
const rubbleOf = (wall: MapProp): KnownProp => ({
  kind: "rubble",
  center: wall.center,
  yaw: wall.yaw,
  half: [wall.half[0], wall.half[1], 0.2],
  baseZ: wall.baseZ,
  replaces: wall.id,
  authoredProp: wall.id,
});
const drawnAt = (list: ModelInstance[]) => list.map((m) => [m.appearance, m.x]);

test("a wall stands until the side knows it fell: an unseen fall stays unseen", () => {
  expect(drawnAt(structureModels(walls, [], appearances))).toEqual([
    ["field_wall", 100],
    ["field_wall", 200],
  ]);
});

test("known rubble replaces its wall atomically, in its place", () => {
  const drawn = structureModels(walls, [rubbleOf(walls[1])], appearances);
  expect(drawnAt(drawn)).toEqual([
    ["field_wall", 100],
    ["village_ruin", 200],
  ]);
  const rubble = drawn[1];
  expect([rubble.x, rubble.y, rubble.z, rubble.yaw]).toEqual([200, 50, 2, 0.3]);
});

test("a map prop the side saw destroyed with nothing in its place is drawn no more", () => {
  // 34c: a crate blown away; its known entry only removes it.
  const gone: KnownProp = { ...rubbleOf(walls[1]), kind: "wall", destroyed: true };
  expect(drawnAt(structureModels(walls, [gone], appearances))).toEqual([["field_wall", 100]]);
});

test("a placed prop takes the appearance nearest its box, scaled to fit it", () => {
  const wreck = (half: Vec3): KnownProp => ({
    kind: "heavy_wreck",
    center: [10, 20],
    yaw: 1,
    half,
    baseZ: 0,
    replaces: null,
    authoredProp: null,
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
  // A ruin no building owns is the ruin appearance, fitted.
  const [loose] = structureModels(
    [],
    [
      {
        kind: "ruin",
        center: [0, 0],
        yaw: 0,
        half: [7.5, 6, 1],
        baseZ: 0,
        replaces: null,
        authoredProp: null,
      },
    ],
    appearances,
  );
  expect(loose.appearance).toBe("village_ruin");
  expect(loose.scale).toEqual([0.5, 0.5, 1]);
});

test("a building's part is no prop appearance: its building draws it", () => {
  const part: MapProp = {
    id: 7,
    kind: "building",
    center: [0, 0],
    yaw: 0,
    half: [15, 12, 4],
    baseZ: 0,
  };
  expect(structureModels([part], [], appearances)).toEqual([]);
  // Nor does a map with one install anything for it here.
  expect(appearances.drawnFor([part])).toEqual(appearances.drawnFor([]));
});

test("a prop movers stand on takes the ground paint; a body that stops them does not", () => {
  // The bridge deck stops no mover: marks painted on it show on it, as on
  // the ground. A wall stops both: it is a body, never painted.
  const at = (kind: string, half: Vec3): MapProp => ({
    id: 1,
    kind,
    center: [0, 0],
    yaw: 0,
    half,
    baseZ: 0,
  });
  const [deck] = structureModels([at("bridge_deck", [18, 5, 0.4])], [], appearances);
  const [wall] = structureModels([at("wall", [2, 0.3, 0.8])], [], appearances);
  expect(modelFog(deck.pose)).toBe("paintedFaces");
  expect(modelFog(wall.pose)).toBe("faces");
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
  const drawn = structureModels(walls, [rubbleOf(walls[0])], appearances, () => false);
  // Only the known rubble: the standing walls are the world's.
  expect(drawnAt(drawn)).toEqual([["village_ruin", 100]]);
});

test("the battle installs the map's props and every body a battle can leave or place", () => {
  const deck: MapProp = {
    id: 9,
    kind: "bridge_deck",
    center: [0, 0],
    yaw: 0,
    half: [18, 5, 0.4],
    baseZ: 1,
  };
  // A deck is only ever a map's: it is installed for a map that has one.
  expect([...appearances.drawnFor([])].sort()).toEqual([
    "field_wall",
    "tank_wreck",
    "truck_wreck",
    "village_ruin",
  ]);
  expect(appearances.drawnFor([deck]).has("bridge_deck")).toBe(true);
});

test("a second replacement never resurrects the static body", () => {
  const source = walls[0];
  const current: KnownProp = { ...rubbleOf(source), replaces: 41, authoredProp: source.id };
  expect(drawnAt(structureModels([source], [current], appearances))).toEqual([
    ["village_ruin", 100],
  ]);
});
