// @vitest-environment node
// The prop seam: each simulation prop draws as an installed scenery appearance
// fitted to its box, and a side draws what it knows: a wall stands until the
// side learns of its rubble, and the rubble replaces it in the same list. A
// building's parts are not props drawn here (`buildingPlacements.test.ts`).
import { expect, test } from "vitest";
import type { Vec3 } from "math";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader.ts";
import type { AppearanceUnit, StaticBundle } from "@packages/scene-assets/src/schema.ts";
import { color } from "math/color";
import { STAND_IN_KIT, STAND_IN_MODULE } from "@packages/scene-assets/src/standInKit.ts";
import {
  drawnStructures,
  fitMapProps,
  PropAppearances,
  sideStructures,
  structureBodies,
  validateStandIns,
  type KnownProp,
  type MapProp,
} from "@packages/battle-renderer/src/models/propAppearance.ts";
import type { ModelInstance } from "@packages/battle-renderer/src/models/modelInstances.ts";
import { modelFog } from "@packages/battle-renderer/src/models/modelFog.ts";
import { UNITS } from "./catalog";

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
  regionalFamily: null,
  paints: null,
  wreck: null as string | null,
  factions: null,
  bundle: bundle(...states),
});
/** A vehicle appearance and the wreck appearance it names. */
const vehicle = (wreck: string) => ({ ...entry("vehicle", null, [0, 0, 0]), wreck });

const installed: InstalledAppearances = {
  generation: 1,
  sides: { blue: [1, 1, 1], red: [1, 1, 1] },
  skeletons: new Map(),
  onRequest: new Map(),
  appearances: new Map([
    ["test_tank", vehicle("test_tank_wreck")],
    ["test_jeep", vehicle("test_jeep_wreck")],
    ["test_tank_wreck", entry("scenery", "wreck", [3.5, 1.8, 1.2], "default")],
    ["test_jeep_wreck", entry("scenery", "wreck", [2.2, 1.0, 0.95], "default")],
    ["ruin", entry("scenery", "ruin", [15, 12, 1], "default")],
    ["field_wall", entry("scenery", "wall", [2, 0.3, 0.8], "default")],
    ["bridge_deck", entry("scenery", "bridge_deck", [18, 5, 0.4], "default")],
  ]),
};
/** The shipped prop types' bindings and blocking, as the world layout
 *  carries them. */
const props = Object.entries(UNITS.view.props);
const layout = {
  propAppearance: Object.fromEntries(props.map(([id, t]) => [id, t.appearance])),
  unitAppearance: Object.fromEntries(
    UNITS.ids.filter((id) => UNITS.hull(id)).map((id) => [id, UNITS.type(id).appearance!]),
  ),
  blockingPropKinds: Object.fromEntries(
    (["infantry", "vehicle"] as const).map((mover) => [
      mover,
      props.filter(([, t]) => t.body.blocks[mover]).map(([id]) => id),
    ]),
  ),
};
const appearances = new PropAppearances(installed, layout, null);

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
/** Every model the side draws, body after body. */
const drawnModels = (...args: Parameters<typeof structureBodies>) =>
  structureBodies(...args).flatMap((b) => b.models);
const drawnAt = (list: ModelInstance[]) => list.map((m) => [m.appearance, m.x]);

test("a wall stands until the side knows it fell: an unseen fall stays unseen", () => {
  expect(drawnAt(drawnModels(walls, [], appearances))).toEqual([
    ["field_wall", 100],
    ["field_wall", 200],
  ]);
});

test("known rubble replaces its wall atomically, in its place", () => {
  const drawn = drawnModels(walls, [rubbleOf(walls[1])], appearances);
  expect(drawnAt(drawn)).toEqual([
    ["field_wall", 100],
    ["ruin", 200],
  ]);
  const rubble = drawn[1];
  expect([rubble.x, rubble.y, rubble.z, rubble.yaw]).toEqual([200, 50, 2, 0.3]);
});

test("a map prop the side saw destroyed with nothing in its place is drawn no more", () => {
  // 34c: a crate blown away; its known entry only removes it.
  const gone: KnownProp = { ...rubbleOf(walls[1]), kind: "wall", destroyed: true };
  expect(drawnAt(drawnModels(walls, [gone], appearances))).toEqual([["field_wall", 100]]);
});

/** A wreck the side knows, of unit type `wreckOf`, on a box of `half`. */
const wreckOf = (wreckOf: string, half: Vec3, kind = "heavy_wreck"): KnownProp => ({
  kind,
  center: [10, 20],
  yaw: 1,
  half,
  baseZ: 0,
  replaces: null,
  authoredProp: null,
  wreckOf,
});

test("a wreck is drawn as its own unit's wreck, whatever box it lies on", () => {
  const [tank] = drawnModels([], [wreckOf("test_tank", [3.5, 1.8, 1.2])], appearances);
  expect(tank.appearance).toBe("test_tank_wreck");
  expect(tank.scale).toEqual([1, 1, 1]);
  // A jeep's wreck on a tank-sized box is still the jeep's, stretched to it:
  // the unit says whose wreck it is, never the nearest footprint.
  const [jeep] = drawnModels([], [wreckOf("test_jeep", [3.5, 1.8, 1.2])], appearances);
  expect(jeep.appearance).toBe("test_jeep_wreck");
  expect(jeep.scale).toEqual([3.5 / 2.2, 1.8, 1.2 / 0.95]);
  // Knocked down to a lighter wreck, it is the same tank's, lower.
  const [lower] = drawnModels(
    [],
    [wreckOf("test_tank", [3.5, 1.8, 0.6], "medium_wreck")],
    appearances,
  );
  expect(lower.appearance).toBe("test_tank_wreck");
  expect(lower.scale).toEqual([1, 1, 0.5]);
});

test("a wreck whose unit has no wreck art never borrows another unit's", () => {
  // The supply truck's wreck is not installed here: nothing, not the tank's.
  expect(drawnModels([], [wreckOf("test_supply", [3, 1.4, 1.8])], appearances)).toEqual([]);
});

test("a placed prop takes the appearance nearest its box, scaled to fit it", () => {
  // A deck longer and wider than the authored one stretches to its box.
  const deck: MapProp = {
    id: 9,
    kind: "bridge_deck",
    center: [0, 0],
    yaw: 0,
    half: [27, 5, 0.4],
    baseZ: 1,
  };
  const [drawn] = drawnModels([deck], [], appearances);
  expect(drawn.appearance).toBe("bridge_deck");
  expect(drawn.scale).toEqual([1.5, 1, 1]);
  // A ruin no building owns is the ruin appearance, fitted.
  const [loose] = drawnModels(
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
  expect(loose.appearance).toBe("ruin");
  expect(loose.scale).toEqual([0.5, 0.5, 1]);
});

test("a map draws a kind in its own region's look, else the shared one, never another region's", () => {
  // One wall kind: a shared look, two of Paris (a short and a long module)
  // and one of New York, which fits this box best of all.
  const regional: InstalledAppearances = {
    ...installed,
    appearances: new Map([
      ...installed.appearances,
      [
        "paris_wall",
        { ...entry("scenery", "wall", [1, 0.3, 0.8], "default"), regionalFamily: "paris" },
      ],
      [
        "paris_long_wall",
        { ...entry("scenery", "wall", [4, 0.4, 1], "default"), regionalFamily: "paris" },
      ],
      [
        "ny_wall",
        { ...entry("scenery", "wall", [2, 0.3, 0.8], "default"), regionalFamily: "new_york" },
      ],
    ]),
  };
  const wall: MapProp = {
    id: 1,
    kind: "wall",
    center: [0, 0],
    yaw: 0,
    half: [2, 0.3, 0.8],
    baseZ: 0,
  };
  const drawn = (family: string | null) => {
    const fit = new PropAppearances(regional, layout, family);
    const walls = [...new Set(drawnModels([wall], [], fit).map((m) => m.appearance))];
    return { walls, installs: [...fit.drawnFor([wall])].filter((n) => n.endsWith("wall")).sort() };
  };
  // Paris prefers its own over the better-fitting shared and New York
  // looks; between its own, the nearer footprint.
  expect(drawn("paris")).toEqual({
    walls: ["paris_wall"],
    installs: ["paris_long_wall", "paris_wall"],
  });
  // A region with no look of its own, and a map of no region: the shared one.
  expect(drawn("china")).toEqual({ walls: ["field_wall"], installs: ["field_wall"] });
  expect(drawn(null)).toEqual({ walls: ["field_wall"], installs: ["field_wall"] });
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
  expect(drawnModels([part], [], appearances)).toEqual([]);
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
  const [deck] = drawnModels([at("bridge_deck", [18, 5, 0.4])], [], appearances);
  const [wall] = drawnModels([at("wall", [2, 0.3, 0.8])], [], appearances);
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
  const along = drawnModels([wall([10, 0.3, 0.8], 0)], [], appearances);
  expect(along).toHaveLength(5);
  expect(along.map((m) => m.x)).toEqual([-8, -4, 0, 4, 8]);
  for (const m of along) {
    expect(m.scale).toEqual([1, 1, 1]);
    expect(m.y).toBeCloseTo(0, 9);
  }
  // Long along its box's y: each module turns to run along it.
  const across = drawnModels([wall([0.3, 6, 0.8], 0)], [], appearances);
  expect(across).toHaveLength(3);
  for (const m of across) {
    expect(m.yaw).toBeCloseTo(Math.PI / 2, 9);
    expect(m.x).toBeCloseTo(0, 9);
  }
  expect(across.map((m) => Math.round(m.y))).toEqual([-4, 0, 4]);
});

test("each body a side draws owns its own models, a repeated wall several", () => {
  const long: MapProp = { ...wallAt(5, 300), half: [10, 0.3, 0.8] };
  const bodies = structureBodies([walls[0], long, walls[1]], [rubbleOf(walls[1])], appearances);
  // The walls in map order, less the fallen one, then what the side knows.
  expect(bodies.map((b) => [b.body.center[0], drawnAt(b.models)])).toEqual([
    [100, [["field_wall", 100]]],
    [
      300,
      Array.from({ length: 5 }, (_, k) => [
        "field_wall",
        expect.closeTo(300 + (k * 4 - 8) * Math.cos(0.3), 9),
      ]),
    ],
    [200, [["ruin", 200]]],
  ]);
});

test("what a side learns hides only the props it replaced, over the map fitted once", () => {
  const long: MapProp = { ...wallAt(5, 300), half: [10, 0.3, 0.8] };
  const fitted = fitMapProps([walls[0], long, walls[1]], appearances);
  const unaware = sideStructures(fitted, [], appearances);
  const aware = sideStructures(fitted, [rubbleOf(long)], appearances);
  // Every module of the repeated wall goes, its rubble comes after the map's.
  expect(drawnAt(drawnStructures(aware))).toEqual([
    ["field_wall", 100],
    ["field_wall", 200],
    ["ruin", 300],
  ]);
  expect(drawnStructures(unaware)).toHaveLength(7);
  // Both draw from the one fitted list: learning changes no map model.
  expect(aware.map).toBe(unaware.map);
  expect(aware.hidden).toEqual([1, 2, 3, 4, 5]);
  expect(unaware.hidden).toEqual([]);
});

test("a map prop a route draws from the world is left to it", () => {
  const drawn = drawnModels(walls, [rubbleOf(walls[0])], appearances, () => false);
  // Only the known rubble: the standing walls are the world's.
  expect(drawnAt(drawn)).toEqual([["ruin", 100]]);
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
    "ruin",
    "test_jeep_wreck",
    "test_tank_wreck",
  ]);
  expect(appearances.drawnFor([deck]).has("bridge_deck")).toBe(true);
});

test("a second replacement never resurrects the static body", () => {
  const source = walls[0];
  const current: KnownProp = { ...rubbleOf(source), replaces: 41, authoredProp: source.id };
  expect(drawnAt(drawnModels([source], [current], appearances))).toEqual([["ruin", 100]]);
});

// A prop kind with no art fitted is drawn as the stand-in kit's unit box (a
// metre cube on its base).
const withKit: InstalledAppearances = {
  ...installed,
  appearances: new Map([
    ...installed.appearances,
    [STAND_IN_KIT, { ...entry("kit", null, [0, 0, 0], STAND_IN_MODULE), footprint: null }],
  ]),
};
const standIns = validateStandIns({
  tints: { parked_car: [1, 0.5, 0], default: [0.5, 0.5, 0.5] },
});
const artless = new PropAppearances(withKit, layout, null, standIns);
const car: MapProp = {
  id: 40,
  kind: "parked_car",
  center: [10, 20],
  yaw: 0.7,
  half: [2.1, 0.9, 0.75],
  baseZ: 3,
};
const linear = (srgb: [number, number, number]) => [...color.fromSRGB(srgb)];

test("a prop kind with no art is drawn as a box of its own size, tinted by its kind", () => {
  const lamp: MapProp = { ...car, id: 41, kind: "lamp", half: [0.15, 0.15, 3] };
  const [box, post] = drawnModels([car, lamp], [], artless);
  expect(box.appearance).toBe(STAND_IN_KIT);
  expect(box.pose).toMatchObject({ kind: "static", state: STAND_IN_MODULE });
  expect([box.x, box.y, box.z, box.yaw]).toEqual([10, 20, 3, 0.7]);
  // The unit box is a metre on a side: its scale is the prop's whole size.
  expect(box.scale).toEqual([4.2, 1.8, 1.5]);
  box.tint!.forEach((c, k) => expect(c).toBeCloseTo(linear([1, 0.5, 0])[k], 6));
  // A kind the style does not list takes the default tint.
  expect(post.scale).toEqual([0.3, 0.3, 6]);
  post.tint!.forEach((c, k) => expect(c).toBeCloseTo(linear([0.5, 0.5, 0.5])[k], 6));
  // The models layer installs the kit for it.
  expect(artless.drawnFor([car]).has(STAND_IN_KIT)).toBe(true);
});

test("a building's part is never a stand-in box: its building draws it from its template's art", () => {
  const part: MapProp = { ...car, id: 42, kind: "building", half: [6, 5, 4] };
  expect(drawnModels([part], [], artless)).toEqual([]);
  expect(drawnModels([part, car], [], artless).map((m) => m.x)).toEqual([car.center[0]]);
});

test("a stand-in follows what the side knows: shoved, burnt out, gone", () => {
  const known = (over: Partial<KnownProp>): KnownProp => ({
    ...car,
    replaces: car.id,
    authoredProp: car.id,
    ...over,
  });
  const [shoved] = drawnModels([car], [known({ center: [14, 22], yaw: 1.2 })], artless);
  expect([shoved.x, shoved.y, shoved.yaw]).toEqual([14, 22, 1.2]);
  shoved.tint!.forEach((c, k) => expect(c).toBeCloseTo(linear([1, 0.5, 0])[k], 6));
  const [wreck] = drawnModels(
    [car],
    [known({ kind: "car_wreck", half: [2.1, 0.9, 0.35] })],
    artless,
  );
  expect(wreck.scale).toEqual([4.2, 1.8, 0.7]);
  wreck.tint!.forEach((c, k) => expect(c).toBeCloseTo(linear([0.5, 0.5, 0.5])[k], 6));
  expect(drawnModels([car], [known({ destroyed: true })], artless)).toEqual([]);
});

test("a tree is never a stand-in, in a forest or beside a street, and nothing is without the kit", () => {
  // The scenery draws every tree body as a tree.
  const tree = (kind: string): MapProp => ({ ...car, kind, half: [0.35, 0.35, 5] });
  expect(drawnModels([tree("trunk")], [], artless)).toEqual([]);
  expect(drawnModels([tree("street_tree")], [], artless)).toEqual([]);
  // The stand-in kit not installed, or no stand-in style: nothing.
  expect(drawnModels([car], [], new PropAppearances(installed, layout, null, standIns))).toEqual(
    [],
  );
  expect(drawnModels([car], [], appearances)).toEqual([]);
});

test("a car wears one of its appearance's paints, the same car the same paint wherever it is shoved", () => {
  const paints: Vec3[] = [
    [0.9, 0.9, 0.88],
    [0.2, 0.2, 0.2],
    [0.45, 0.47, 0.3],
  ];
  const painted = new PropAppearances(
    {
      ...installed,
      appearances: new Map([
        ...installed.appearances,
        ["parked_car", { ...entry("scenery", "parked_car", [2.1, 0.9, 0.75], "default"), paints }],
      ]),
    },
    layout,
    null,
  );
  const car = (id: number, x: number): MapProp => ({
    id,
    kind: "parked_car",
    center: [x, 10],
    yaw: 0,
    half: [2.1, 0.9, 0.75],
    baseZ: 0,
  });
  const street = Array.from({ length: 40 }, (_, i) => car(i, i * 5));
  const linear = paints.map((p) => [...color.fromSRGB(p)].map((c) => +c.toFixed(5)).join());
  const tints = drawnModels(street, [], painted).map((m) =>
    m.tint!.map((c) => +c.toFixed(5)).join(),
  );
  // Each car one of the paints, and a street of them more than one.
  expect(tints.every((t) => linear.includes(t))).toBe(true);
  expect(new Set(tints).size).toBeGreaterThan(1);
  // Shoved along the kerb, a car keeps its paint.
  const [moved] = drawnModels(
    street,
    [{ ...car(7, 0), center: [61, 13], replaces: 7, authoredProp: 7 }],
    painted,
  ).filter((m) => m.x === 61);
  expect(moved.tint!.map((c) => +c.toFixed(5)).join()).toBe(tints[7]);
  // A prop whose appearance has no paints is drawn as authored.
  expect(drawnModels(walls, [], painted).every((m) => m.tint === undefined)).toBe(true);
});
