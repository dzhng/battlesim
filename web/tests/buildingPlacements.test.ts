// @vitest-environment node
// Buildings drawn from template art: a map's references, the rows a view
// draws of them at each tier, the pool's residency as the camera moves, the
// state a side knows a building in from what it was published, and what
// changes when it sees one destroyed.
import { expect, test } from "vitest";
import { color } from "math/color";
import {
  resolve,
  ROW_TRANSFORM_FLOATS,
  templateKits,
  type TemplateArtLibrary,
} from "@packages/scene-assets/src/templateLibrary";
import { createDetailView, setDetailView } from "@packages/battle-renderer/src/frame/detailView";
import {
  artCovers,
  buildingCasters,
  buildingDraws,
  COARSE,
  createBuildingScene,
  POOL,
  RUINS,
  selectBuildings,
  setFallenBuildings,
  type BuildingArt,
  type BuildingScene,
} from "@packages/battle-renderer/src/models/buildingPlacements";
import {
  fallenBuildings,
  indexBuildings,
  validateBuildingStyle,
  type BuildingStyle,
  type PlacedBuildings,
} from "@packages/battle-renderer/src/models/buildingReferences";
import { MODEL_RECORD_FLOATS } from "@packages/battle-renderer/src/models/modelInstances";
import type { KnownProp, MapProp } from "@packages/battle-renderer/src/models/propAppearance";
import type { PublicBuildings } from "@packages/battle-renderer/src/worldMesh";

const RECORD = MODEL_RECORD_FLOATS;
const [SHELL, WINDOW, ROOF, BOX] = [0, 1, 2, 3];
/** A second kit, with one module: what a stand-in template draws. */
const BOX_KIT = "city_kit_boxes";

type Row = [
  module: number,
  x: number,
  y: number,
  z: number,
  yaw: number,
  sx: number,
  sy: number,
  sz: number,
  tiers: number,
  r: number,
  g: number,
  b: number,
];

/** A library of `templates`, each a state to its rows. */
function libraryOf(templates: Record<string, Record<string, Row[]>>): TemplateArtLibrary {
  const rows = Object.values(templates).flatMap((states) => Object.values(states).flat());
  let first = 0;
  return {
    art_hash: "art",
    covers: ["physical"],
    kits: [
      { appearance: "city_kit_test", bundle: "a" },
      { appearance: BOX_KIT, bundle: "b" },
    ],
    modules: [
      { kit: 0, module: "shell" },
      { kit: 0, module: "window" },
      { kit: 0, module: "roof" },
      { kit: 1, module: "box" },
    ],
    templates: Object.entries(templates).map(([id, states]) => ({
      id,
      set: "test",
      status: "release" as const,
      states: Object.fromEntries(
        Object.entries(states).map(([state, list]) => {
          const range = { first, count: list.length };
          first += list.length;
          return [state, range];
        }),
      ),
    })),
    rows: {
      module: Uint16Array.from(rows, (r) => r[0]),
      transform: Float32Array.from(rows.flatMap((r) => r.slice(1, 8))),
      tiers: Uint8Array.from(rows, (r) => r[8]),
      tint: Uint8Array.from(rows.flatMap((r) => r.slice(9, 12))),
    },
  };
}

const ALL = 15;
// A house: a shell at every tier, a window at the two finest and one at the
// finest alone, and a roof whose tiers are not a prefix (the finest and the
// coarsest).
const HOUSE: Row[] = [
  [SHELL, 0, 0, 0, 0, 1, 1, 1, ALL, 255, 255, 255],
  [WINDOW, 5, 1, 1, Math.PI / 2, 1, 1, 1, 3, 255, 255, 255],
  [WINDOW, 5, -1, 1, Math.PI / 2, 1, 1, 1, 1, 255, 255, 255],
  [ROOF, 2, 0, 6, 0, 1, 1, 2, 9, 128, 255, 64],
];
// A block with many fine rows: what fills a pool.
const DENSE: Row[] = [
  [SHELL, 0, 0, 0, 0, 1, 1, 1, ALL, 255, 255, 255],
  ...Array.from(
    { length: 39 },
    (_, i): Row => [WINDOW, 5, i * 0.2 - 4, 1, 0, 1, 1, 1, 1, 255, 255, 255],
  ),
];
// The house collapsed: its remains at every tier (the shell, a quarter as
// tall and rubble-coloured) and wreckage on them at the two finest.
const RUBBLE: Row[] = [
  [SHELL, 1, 0, 0, 0, 1, 1, 0.25, ALL, 90, 80, 70],
  [ROOF, 2, 1, 1, 0.5, 0.5, 0.5, 0.5, 3, 60, 50, 40],
];
// A tower, and the tower gutted: its shell at full height, burnt, at every
// tier, and a burnt-out window at the two finest.
const TOWER: Row[] = [
  [SHELL, 0, 0, 0, 0, 1, 1, 5, ALL, 255, 255, 255],
  [WINDOW, 5, 1, 20, Math.PI / 2, 1, 1, 1, 3, 255, 255, 255],
];
const BURNT: Row[] = [
  [SHELL, 0, 0, 0, 0, 1, 1, 5, ALL, 40, 38, 36],
  [WINDOW, 5, 1, 20, Math.PI / 2, 1, 1, 1, 3, 10, 10, 10],
];
const LIBRARY = libraryOf({
  house: { intact: HOUSE, ruin: RUBBLE },
  dense: { intact: DENSE },
  stand_in: { intact: [[BOX, 0, 0, 0, 0, 20, 10, 12, ALL, 180, 180, 190]] },
  tower: { intact: TOWER, gutted: BURNT },
});
const ART: BuildingArt = {
  library: LIBRARY,
  bounds: [
    { min: [-5, -4, 0], max: [5, 4, 6] },
    { min: [-0.5, -0.1, 0], max: [0.5, 0.1, 1] },
    { min: [-5, -4, 0], max: [5, 4, 1] },
    { min: [-0.5, -0.5, 0], max: [0.5, 0.5, 1] },
  ],
};

const STYLE: BuildingStyle = validateBuildingStyle({
  lod_px_per_m: [12, 5, 2],
  chunk_m: 64,
  pool_records: 1024,
  expand_rows: 100000,
  tint_jitter: 0,
});

/** Buildings of `template` at each `[x, y, z, yaw]`. */
function placedOf(list: [template: string, frame: [number, number, number, number]][]) {
  const templates = [...new Set(list.map(([t]) => t))].sort();
  const placed: PlacedBuildings = {
    templates,
    template: Uint16Array.from(list, ([t]) => templates.indexOf(t)),
    frames: Float64Array.from(list.flatMap(([, frame]) => frame)),
    owners: Uint32Array.from(list, (_, i) => 100 + i),
  };
  return placed;
}

/** A camera straight down on (x, y) from `distance` metres, the frame's long
 *  side along x: it sees 0.75 of its distance either way in x and 0.42 in y.
 *  At 1080 pixels and this lens a metre covers 12 px at 106 m, 5 px at 255 m
 *  and 2 px at 638 m. */
const view = (x: number, y: number, distance: number) =>
  setDetailView(
    createDetailView(),
    {
      target: [x, y, 0],
      distance,
      pitch: 1.5,
      yaw: Math.PI / 2,
      fovY: 0.8,
      aspect: 16 / 9,
      near: 1,
    },
    1080,
  );
const SHADOW = { fall: [1, 0] as [number, number], reach: 2600 };

/** The records a draw's source buffer holds. */
function bufferOf(scene: BuildingScene, source: number): Float32Array {
  if (source === POOL) return scene.pool.records;
  return (source === COARSE ? scene.coarse : scene.ruins!).records;
}

interface Drawn {
  source: number;
  module: number;
  tier: number;
  /** x, y, z, yaw, then the scale. */
  transform: number[];
  tint: number[];
}

/** Every instance the last selection draws (or casts), one entry each. A
 *  record hidden in place (its scale zero) is drawn as nothing. */
function drawn(scene: BuildingScene, casters = false): Drawn[] {
  const out: Drawn[] = [];
  (casters ? buildingCasters : buildingDraws)(scene, (source, module, tier, first, count) => {
    const records = bufferOf(scene, source);
    for (let i = first; i < first + count; i++) {
      const r = [...records.subarray(i * RECORD, (i + 1) * RECORD)];
      if (r[12] === 0 && r[13] === 0 && r[14] === 0) continue;
      out.push({
        source,
        module,
        tier,
        transform: [...r.slice(0, 4), ...r.slice(12, 15)],
        tint: r.slice(8, 11),
      });
    }
  });
  return out;
}

/** What is drawn as "module@tier", sorted. */
const tally = (list: Drawn[]) => list.map((d) => `${d.module}@${d.tier}`).sort();

test("a row is drawn where the placed frame puts it: a turned building's records are the resolver's", () => {
  const frame = { translation: [1000, 2000, 3] as [number, number, number], yaw: 0.7 };
  const scene = createBuildingScene(
    placedOf([["house", [...frame.translation, frame.yaw]]]),
    ART,
    STYLE,
  );
  selectBuildings(scene, view(1000, 2000, 60), SHADOW);
  const resolved = resolve("house", frame, "intact", LIBRARY);
  const row = (i: number) =>
    [...resolved.transforms.subarray(i * ROW_TRANSFORM_FLOATS, (i + 1) * ROW_TRANSFORM_FLOATS)].map(
      Math.fround,
    );
  const fine = drawn(scene);
  expect(fine.every((d) => d.source === POOL && d.tier === 0)).toBe(true);
  expect(fine.filter((d) => d.module === SHELL).map((d) => d.transform)).toEqual([row(0)]);
  expect(
    fine
      .filter((d) => d.module === WINDOW)
      .map((d) => d.transform)
      .sort(),
  ).toEqual([row(1), row(2)].sort());
  // The window stands on the east wall, turned with the building.
  expect(row(1)[0]).toBeCloseTo(1000 + 5 * Math.cos(0.7) - Math.sin(0.7), 3);
  expect(row(1)[3]).toBeCloseTo(0.7 + Math.PI / 2, 5);
  // The roof keeps its row's scale and tint (sRGB bytes, linear on the GPU).
  const roof = fine.find((d) => d.module === ROOF)!;
  expect(roof.transform).toEqual(row(3));
  const tint = color.fromSRGB([128 / 255, 1, 64 / 255]);
  roof.tint.forEach((c, k) => expect(c).toBeCloseTo(tint[k], 5));
});

test("a row draws at a tier only if its mask has it, whether the masks nest or not", () => {
  const scene = createBuildingScene(placedOf([["house", [1000, 2000, 0, 0]]]), ART, STYLE);
  const at = (distance: number) => {
    selectBuildings(scene, view(1000, 2000, distance), SHADOW);
    return { view: tally(drawn(scene)), cast: tally(drawn(scene, true)) };
  };
  // Tier 0: all four rows, from the pool; they cast a tier coarser.
  expect(at(60)).toEqual({
    view: [`${ROOF}@0`, `${SHELL}@0`, `${WINDOW}@0`, `${WINDOW}@0`].sort(),
    cast: [`${ROOF}@1`, `${SHELL}@1`, `${WINDOW}@1`, `${WINDOW}@1`].sort(),
  });
  // Tier 1: the roof's mask skips it.
  expect(at(200)).toEqual({
    view: [`${SHELL}@1`, `${WINDOW}@1`],
    cast: [`${SHELL}@2`, `${WINDOW}@2`],
  });
  // Tier 2: the shell alone, casting at the coarsest mesh.
  expect(at(500)).toEqual({ view: [`${SHELL}@2`], cast: [`${SHELL}@3`] });
  // Tier 3: the coarse rows, the roof back among them.
  expect(at(2000)).toEqual({
    view: [`${ROOF}@3`, `${SHELL}@3`].sort(),
    cast: [`${ROOF}@3`, `${SHELL}@3`].sort(),
  });
  expect(drawn(scene).every((d) => d.source === COARSE)).toBe(true);
});

test("a chunk is expanded once when it comes near, and leaves the pool when it leaves", () => {
  const scene = createBuildingScene(placedOf([["house", [1000, 2000, 0, 0]]]), ART, STYLE);
  selectBuildings(scene, view(1000, 2000, 60), SHADOW);
  expect(scene.expandedRows).toBe(4);
  expect(scene.pool.used).toBe(4);
  // A still camera, and a camera that moves without changing the chunk's tier.
  selectBuildings(scene, view(1000, 2000, 60), SHADOW);
  expect(scene.expandedRows).toBe(0);
  selectBuildings(scene, view(1004, 2003, 70), SHADOW);
  expect(scene.expandedRows).toBe(0);
  expect(scene.pool.used).toBe(4);
  // Farther: the tier's rows replace the finer ones.
  selectBuildings(scene, view(1000, 2000, 200), SHADOW);
  expect(scene.expandedRows).toBe(2);
  expect(scene.pool.used).toBe(2);
  // Out of view: nothing is resident, and nothing is drawn.
  selectBuildings(scene, view(5000, 2000, 60), SHADOW);
  expect(scene.pool.used).toBe(0);
  expect(drawn(scene)).toEqual([]);
  // Yet its shadow still lands in view from just outside it.
  selectBuildings(scene, view(1055, 2000, 60), SHADOW);
  expect(tally(drawn(scene))).toEqual([]);
  expect(tally(drawn(scene, true))).toEqual([`${ROOF}@3`, `${SHELL}@3`].sort());
});

test("a building is in one chunk whole, however its rows straddle a chunk's edge", () => {
  // Placed a metre west of a chunk edge at x = 1024: its roof's own origin
  // and its east-wall windows stand past it.
  const scene = createBuildingScene(
    placedOf([
      ["house", [1023, 2000, 0, 0]],
      ["house", [1100, 2000, 0, 0]],
    ]),
    ART,
    STYLE,
  );
  expect(scene.chunkBuildings.map((list) => [...list])).toEqual([[0], [1]]);
  // From over the first, the second is a tier coarser: no row of either
  // takes the other's tier.
  selectBuildings(scene, view(1023, 2000, 100), SHADOW);
  const tiers = (x: number) => [
    ...new Set(
      drawn(scene)
        .filter((d) => Math.abs(d.transform[0] - x) < 10)
        .map((d) => d.tier),
    ),
  ];
  expect(tiers(1023)).toEqual([0]);
  expect(tiers(1100)).toEqual([1]);
});

test("a pan across a town keeps the pool inside its bound, each building drawn once, the nearest finest", () => {
  // 60 x 6 dense blocks, 24 m apart: 40 fine rows each, so a view's near
  // chunks hold more than the pool does.
  const blocks: [string, [number, number, number, number]][] = [];
  for (let i = 0; i < 60; i++)
    for (let j = 0; j < 6; j++) blocks.push(["dense", [1000 + i * 24, 1060 + j * 24, 0, 0]]);
  const scene = createBuildingScene(placedOf(blocks), ART, STYLE);
  let refused = 0;
  let expanded = 0;
  for (let x = 800; x <= 2700; x += 37) {
    selectBuildings(scene, view(x, 1120, 100), SHADOW);
    refused += scene.refused;
    expanded += scene.expandedRows;
    expect(scene.pool.used).toBeLessThanOrEqual(STYLE.pool_records);
    expect(scene.pending).toBe(false);
    const shells = drawn(scene).filter((d) => d.module === SHELL);
    // No building twice (coarse and resident at once), none half drawn.
    const at = shells.map((d) => `${d.transform[0]},${d.transform[1]}`);
    expect(new Set(at).size).toBe(at.length);
    // Whatever stands under the camera is drawn, at the finest tier.
    const under = blocks.filter(([, f]) => Math.abs(f[0] - x) <= 12 && Math.abs(f[1] - 1120) <= 12);
    for (const [, f] of under) {
      const shell = shells.find((d) => d.transform[0] === f[0] && d.transform[1] === f[1]);
      expect(shell?.tier, `the block at ${f[0]}, ${f[1]} from x = ${x}`).toBe(0);
      const windows = drawn(scene).filter(
        (d) => d.module === WINDOW && Math.abs(d.transform[0] - f[0] - 5) < 1e-3,
      );
      expect(windows.filter((d) => Math.abs(d.transform[1] - f[1]) < 5)).toHaveLength(39);
    }
  }
  // The bound was met on the way, and rows came and went.
  expect(refused).toBeGreaterThan(0);
  expect(expanded).toBeGreaterThan(STYLE.pool_records);
  // Past the town: nothing is left in the pool.
  selectBuildings(scene, view(9000, 1120, 100), SHADOW);
  expect(scene.pool.used).toBe(0);
});

test("a view change expands at most its budget, and the rest wait at the coarsest tier", () => {
  const scene = createBuildingScene(
    placedOf([
      ["house", [1000, 2000, 0, 0]],
      ["house", [1100, 2000, 0, 0]],
    ]),
    ART,
    { ...STYLE, expand_rows: 1 },
  );
  selectBuildings(scene, view(1035, 2000, 100), SHADOW);
  expect(scene.pending).toBe(true);
  expect(tally(drawn(scene))).toEqual(
    [`${ROOF}@0`, `${SHELL}@0`, `${WINDOW}@0`, `${WINDOW}@0`, `${ROOF}@3`, `${SHELL}@3`].sort(),
  );
  selectBuildings(scene, view(1035, 2000, 100), SHADOW);
  expect(scene.pending).toBe(false);
  expect(tally(drawn(scene))).toEqual(
    [`${ROOF}@0`, `${SHELL}@0`, `${WINDOW}@0`, `${WINDOW}@0`, `${SHELL}@1`, `${WINDOW}@1`].sort(),
  );
});

const part = (id: number, x: number, y: number, half: [number, number, number]): MapProp => ({
  id,
  kind: "building",
  center: [x, y],
  yaw: 0.25,
  half,
  baseZ: 0,
});

test("every building of a map is a reference, with its frame and its parts", () => {
  const buildings: PublicBuildings = {
    catalogueHash: "h",
    regionalFamily: "family",
    buildings: [
      ["house", 7, [3, 4]],
      ["stand_in", 11, [6]],
    ].map(([templateId, owner, props], i) => ({
      owner: owner as number,
      kind: "building",
      templateId: templateId as string,
      category: "detached_home",
      regionalFamily: "china",
      frame: { translation: [100 * i, 50, 2] as [number, number, number], yaw: 0.5 * i },
      parts: (props as number[]).map((prop, k) => ({ part: `p${k}`, prop })),
    })),
  };
  const props = [3, 4, 5, 6].map((id) => part(id, id * 10, 50, [4, 3, 3]));
  const index = indexBuildings(buildings, props);
  expect(index.placed.templates).toEqual(["house", "stand_in"]);
  expect([...index.placed.template]).toEqual([0, 1]);
  expect([...index.placed.owners]).toEqual([7, 11]);
  expect([...index.placed.frames]).toEqual([0, 50, 2, 0, 100, 50, 2, 0.5]);
  expect([...index.partBuilding]).toEqual([
    [3, 0],
    [4, 0],
    [6, 1],
  ]);
  // A prop that is no building's part is none of theirs.
  expect(index.partBuilding.has(5)).toBe(false);
});

test("a building whose template the library has no art for is refused by name", () => {
  const placed = placedOf([
    ["house", [0, 0, 0, 0]],
    ["shed", [100, 0, 0, 0]],
  ]);
  expect(() => createBuildingScene(placed, ART, STYLE)).toThrow(
    /template\.missing: template "shed" has no art/,
  );
});

test("a map needs only the kits its own templates draw from, in every state", () => {
  const standIn = placedOf([["stand_in", [0, 0, 0, 0]]]);
  const house = placedOf([["house", [0, 0, 0, 0]]]);
  expect([...templateKits(LIBRARY, standIn.templates)]).toEqual([BOX_KIT]);
  expect([...templateKits(LIBRARY, house.templates)]).toEqual(["city_kit_test"]);
  // With the box kit alone installed, the stand-in draws and the house waits.
  const boxOnly: BuildingArt = { library: LIBRARY, bounds: [null, null, null, ART.bounds[BOX]] };
  expect(artCovers(standIn, boxOnly)).toBe(true);
  expect(artCovers(house, boxOnly)).toBe(false);
  expect(artCovers(house, ART)).toBe(true);
  const scene = createBuildingScene(standIn, boxOnly, STYLE);
  selectBuildings(scene, view(0, 0, 60), null);
  expect(tally(drawn(scene))).toEqual([`${BOX}@0`]);
});

/** The prop types a gutted building's parts become, as the catalog names them. */
const SHELLS = new Set(["burnt_shell"]);

test("a building's state is what the side was published of it: remains are a ruin, a standing shell is gutted, nothing seen is intact", () => {
  // A house of two parts, a tower, and a shed as low as the least remains.
  const buildings: PublicBuildings = {
    catalogueHash: "h",
    regionalFamily: "family",
    buildings: (
      [
        ["house", 0, [0, 1]],
        ["tower", 10, [10]],
        ["house", 20, [20]],
      ] as const
    ).map(([templateId, owner, props], i) => ({
      owner,
      kind: "building",
      templateId,
      category: "detached_home",
      regionalFamily: "china",
      frame: { translation: [1000 + 40 * i, 2000, 0] as [number, number, number], yaw: 0 },
      parts: props.map((prop, k) => ({ part: `p${k}`, prop })),
    })),
  };
  const props = [
    part(0, 1000, 2000, [5, 4, 3]),
    part(1, 1004, 2004, [2, 2, 3]),
    part(10, 1040, 2000, [5, 4, 30]),
    part(20, 1080, 2000, [3, 3, 1]),
  ];
  const index = indexBuildings(buildings, props);
  const seen = (known: KnownProp[]) => fallenBuildings(index, known, SHELLS);
  const replacing = (prop: MapProp, kind: string, halfZ: number): KnownProp => ({
    ...prop,
    kind,
    half: [prop.half[0], prop.half[1], halfZ],
    authoredProp: prop.id,
    replaces: prop.id,
  });

  // Whatever was destroyed that the side did not see: every building intact.
  expect(seen([])).toEqual([]);
  // A wreck is no building's.
  const wreck: KnownProp = { ...props[0], kind: "heavy_wreck", authoredProp: null, replaces: null };
  expect(seen([wreck])).toEqual([]);

  // The house's parts known as remains: a ruin.
  const remains = [replacing(props[0], "ruin", 1), replacing(props[1], "ruin", 1)];
  expect(seen(remains)).toEqual([{ building: 0, state: "ruin" }]);
  // One known part is the whole building's state: its art is one building's.
  expect(seen([remains[1]])).toEqual([{ building: 0, state: "ruin" }]);
  // A part seen destroyed with nothing left is a ruin's too.
  expect(seen([{ ...remains[0], destroyed: true }])).toEqual([{ building: 0, state: "ruin" }]);

  // The tower's part known as a shell at its full height: gutted.
  const shell = replacing(props[2], "burnt_shell", 30);
  expect(seen([shell])).toEqual([{ building: 1, state: "gutted" }]);
  // The published prop decides, never its height: remains as tall as the
  // shed they replace (the least a ruin stands) are a ruin, and a shell is
  // gutted however low.
  expect(seen([replacing(props[3], "ruin", 1)])).toEqual([{ building: 2, state: "ruin" }]);
  expect(seen([replacing(props[3], "burnt_shell", 0.5)])).toEqual([
    { building: 2, state: "gutted" },
  ]);
  // Parts known both ways (the simulation ends a building one way): a ruin,
  // in either order.
  const mixed = [replacing(props[0], "burnt_shell", 3), remains[1]];
  expect(seen(mixed)).toEqual([{ building: 0, state: "ruin" }]);
  expect(seen([...mixed].reverse())).toEqual([{ building: 0, state: "ruin" }]);

  // Each building by its own knowledge, in the map's order.
  expect(seen([shell, wreck, ...remains])).toEqual([
    { building: 0, state: "ruin" },
    { building: 1, state: "gutted" },
  ]);
});

test("a building seen to collapse changes only its own records: it leaves the intact rows and draws as its template's ruin", () => {
  const scene = createBuildingScene(
    placedOf([
      ["house", [1000, 2000, 0, 0.5]],
      ["house", [1020, 2000, 0, 0]],
    ]),
    ART,
    STYLE,
  );
  const near = view(1010, 2000, 60);
  const far = view(1010, 2000, 2000);
  selectBuildings(scene, far, SHADOW);
  scene.coarseDirty.length = 0;

  setFallenBuildings(scene, [{ building: 0, state: "ruin" }]);
  // Its two coarse records are the only ones rewritten: a run of one each.
  const rewritten = scene.coarseDirty.filter((_, i) => i % 2 === 0);
  expect(scene.coarseDirty.filter((_, i) => i % 2 === 1)).toEqual([1, 1]);
  for (const at of rewritten) {
    const record = scene.coarse.records.subarray(at * RECORD, (at + 1) * RECORD);
    // The first building's shell and roof, not its neighbour's 20 m east.
    expect(record[0]).toBeLessThan(1010);
    expect([...record.subarray(12, 15)]).toEqual([0, 0, 0]);
  }
  selectBuildings(scene, near, SHADOW);
  setFallenBuildings(scene, []);
  selectBuildings(scene, near, SHADOW);
  expect(scene.pool.used).toBe(8);
  setFallenBuildings(scene, [{ building: 0, state: "ruin" }]);
  selectBuildings(scene, near, SHADOW);
  // Its chunk is expanded again without it; the neighbour is still whole.
  expect(scene.expandedRows).toBe(4);
  expect(scene.pool.used).toBe(4);
  const standing = drawn(scene).filter((d) => d.source === POOL);
  expect(standing.every((d) => Math.abs(d.transform[0] - 1020) < 6)).toBe(true);
  // Its remains: its template's ruin rows at the camera's tier, placed at
  // its own frame (turned with it) in the rows' own tints.
  const ruin = drawn(scene).filter((d) => d.source === RUINS);
  expect(tally(ruin)).toEqual([`${SHELL}@0`, `${ROOF}@0`].sort());
  const resolved = resolve("house", { translation: [1000, 2000, 0], yaw: 0.5 }, "ruin", LIBRARY);
  const row = (i: number) =>
    [...resolved.transforms.subarray(i * ROW_TRANSFORM_FLOATS, (i + 1) * ROW_TRANSFORM_FLOATS)].map(
      Math.fround,
    );
  expect(ruin.find((d) => d.module === SHELL)!.transform).toEqual(row(0));
  expect(ruin.find((d) => d.module === ROOF)!.transform).toEqual(row(1));
  const tint = color.fromSRGB([90 / 255, 80 / 255, 70 / 255]);
  ruin.find((d) => d.module === SHELL)!.tint.forEach((c, k) => expect(c).toBeCloseTo(tint[k], 5));
  // From afar its intact coarse rows are hidden, not the neighbour's, and
  // its remains are the ruin's own coarse row, which casts for it.
  selectBuildings(scene, far, SHADOW);
  const coarse = (list: Drawn[]) =>
    list.map((d) => `${d.source === RUINS ? "ruin" : "intact"} ${d.module}@${d.tier}`).sort();
  const farDrawn = [`intact ${ROOF}@3`, `intact ${SHELL}@3`, `ruin ${SHELL}@3`].sort();
  expect(coarse(drawn(scene))).toEqual(farDrawn);
  expect(coarse(drawn(scene, true))).toEqual(farDrawn);
  expect(drawn(scene).find((d) => d.source === RUINS)!.transform).toEqual(row(0));

  // The building is known intact again (another side's view): restored.
  setFallenBuildings(scene, []);
  expect(scene.ruins).toBeNull();
  selectBuildings(scene, far, SHADOW);
  expect(coarse(drawn(scene))).toEqual(
    [`intact ${ROOF}@3`, `intact ${ROOF}@3`, `intact ${SHELL}@3`, `intact ${SHELL}@3`].sort(),
  );
  selectBuildings(scene, near, SHADOW);
  expect(scene.pool.used).toBe(8);
});

test("a building known gutted is its template's gutted rows at every tier, and never its intact shell", () => {
  const scene = createBuildingScene(
    placedOf([
      ["tower", [1000, 2000, 0, 0]],
      ["house", [1020, 2000, 0, 0]],
    ]),
    ART,
    STYLE,
  );
  const burnt = color.fromSRGB([40 / 255, 38 / 255, 36 / 255]);
  const shells = (list: Drawn[]) => list.filter((d) => d.module === SHELL && d.transform[0] < 1010);
  /** The tower's shell as drawn: where its record comes from, at which tier,
   *  and whether it wears the burnt rows' tint. */
  const tower = (list: Drawn[]) =>
    shells(list).map((d) => ({
      rows: d.source === RUINS ? "gutted" : "intact",
      tier: d.tier,
      burnt: Math.abs(d.tint[0] - burnt[0]) < 1e-5,
      height: d.transform[6],
    }));
  setFallenBuildings(scene, [{ building: 0, state: "gutted" }]);
  // Tiers 0, 1, 2 and 3, each from the distance the game changes to it at.
  for (const [distance, tier] of [
    [60, 0],
    [200, 1],
    [500, 2],
    [2000, 3],
  ]) {
    selectBuildings(scene, view(1000, 2000, distance), SHADOW);
    // One shell, the gutted rows', burnt and at full height: the shell stands.
    expect(tower(drawn(scene)), `tier ${tier}`).toEqual([
      { rows: "gutted", tier, burnt: true, height: 5 },
    ]);
    // Its burnt window is a fine row: at the two finest tiers and no other.
    const windows = drawn(scene).filter((d) => d.source === RUINS && d.module === WINDOW);
    expect(windows.map((d) => d.tier)).toEqual(tier < 2 ? [tier] : []);
    // What casts for it is its own coarse gutted shell.
    expect(tower(drawn(scene, true))).toEqual([
      { rows: "gutted", tier: 3, burnt: true, height: 5 },
    ]);
  }
  // Its neighbour is untouched at every one of those.
  selectBuildings(scene, view(1000, 2000, 60), SHADOW);
  expect(drawn(scene).filter((d) => d.source === POOL && d.transform[0] < 1010)).toEqual([]);
  expect(tally(drawn(scene).filter((d) => d.source === POOL))).toEqual(
    [`${ROOF}@0`, `${SHELL}@0`, `${WINDOW}@0`, `${WINDOW}@0`].sort(),
  );
});

test("a destroyed building whose template has no rows for the state is refused by name, and nothing changes", () => {
  const scene = createBuildingScene(
    placedOf([
      ["tower", [1000, 2000, 0, 0]],
      ["dense", [1020, 2000, 0, 0]],
    ]),
    ART,
    STYLE,
  );
  const far = view(1010, 2000, 2000);
  selectBuildings(scene, far, SHADOW);
  const before = tally(drawn(scene));
  // A tower stands gutted: it has no ruin. The dense block has neither.
  expect(() => setFallenBuildings(scene, [{ building: 0, state: "ruin" }])).toThrow(
    /state\.missing: template "tower".*"ruin"/,
  );
  expect(() =>
    setFallenBuildings(scene, [
      { building: 0, state: "gutted" },
      { building: 1, state: "gutted" },
    ]),
  ).toThrow(/state\.missing: template "dense".*"gutted"/);
  selectBuildings(scene, far, SHADOW);
  expect(scene.ruins).toBeNull();
  expect(tally(drawn(scene))).toEqual(before);
});

test("identical neighbours differ a little in value, each the same every time", () => {
  const blocks = placedOf([
    ["stand_in", [1000, 2000, 0, 0]],
    ["stand_in", [1030, 2000, 0, 0]],
  ]);
  const tints = () => {
    const scene = createBuildingScene(blocks, ART, { ...STYLE, tint_jitter: 0.1 });
    selectBuildings(scene, view(1010, 2000, 2000), SHADOW);
    return drawn(scene).map((d) => d.tint);
  };
  const want = color.fromSRGB([180 / 255, 180 / 255, 190 / 255]);
  const scales = tints().map((tint) => {
    const k = tint[0] / want[0];
    tint.forEach((c, i) => expect(c / want[i]).toBeCloseTo(k, 4));
    expect(Math.abs(k - 1)).toBeLessThanOrEqual(0.1 + 1e-6);
    return k;
  });
  expect(scales[0]).not.toBeCloseTo(scales[1], 3);
  expect(tints()).toEqual(tints());
});

test("presentation.buildings is checked: tiers that fall, a pool, a jitter in range", () => {
  expect(() => validateBuildingStyle({ ...STYLE, lod_px_per_m: [5, 12, 2] })).toThrow(/fall/);
  expect(() => validateBuildingStyle({ ...STYLE, pool_records: 10 })).toThrow(/pool_records/);
  expect(() => validateBuildingStyle({ ...STYLE, tint_jitter: 0.9 })).toThrow(/tint_jitter/);
});
