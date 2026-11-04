// @vitest-environment node
// Buildings drawn from template art through the presentation's static
// batches: a map's references, the rows a view draws of them at each tier,
// the pool's residency as the camera moves, and what changes when the side
// sees one destroyed (the rule that reads its state from the known props is
// `static_world/fallen.rs`'s, tested there).
import { expect, test } from "vitest";
import { loadPresentation } from "@web/battle/sim/presentation";
import { color } from "math/color";
import {
  encodeTemplateLibrary,
  ROW_TRANSFORM_FLOATS,
  templateKits,
  templateRows,
  type TemplateArtLibrary,
  type TemplateState,
} from "@packages/scene-assets/src/templateLibrary";
import type { Camera3DParams } from "@packages/renderer-core/src/camera3d";
import {
  artCovers,
  indexBuildings,
  type BuildingArt,
  type BuildingStyle,
  type PlacedBuildings,
} from "@packages/battle-renderer/src/models/buildingReferences";
import { tierStyle } from "@apps/battle-lab/src/buildingTier";
import type { MapProp } from "@packages/battle-renderer/src/models/propAppearance";
import type { PublicBuildings } from "@packages/battle-renderer/src/worldMesh";
import {
  presentBuildings,
  type DrawnModule as Drawn,
  type PresentedSun,
} from "./support/presentedBuildings";

await loadPresentation();

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
    covers: "physical",
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
  bytes: encodeTemplateLibrary(LIBRARY),
  bounds: [
    { min: [-5, -4, 0], max: [5, 4, 6] },
    { min: [-0.5, -0.1, 0], max: [0.5, 0.1, 1] },
    { min: [-5, -4, 0], max: [5, 4, 1] },
    { min: [-0.5, -0.5, 0], max: [0.5, 0.5, 1] },
  ],
};

const STYLE: BuildingStyle = {
  lod_px_per_m: [12, 5, 2],
  chunk_m: 64,
  pool_records: 1024,
  tint_jitter: 0,
};

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
const view = (x: number, y: number, distance: number): Camera3DParams => ({
  target: [x, y, 0],
  distance,
  pitch: 1.5,
  yaw: Math.PI / 2,
  fovY: 0.8,
  aspect: 16 / 9,
  near: 1,
});
/** A sun low in the west: a shadow falls a metre east for a metre of height,
 *  received about 2,600 m out. */
const SUN: PresentedSun = { azimuth: Math.PI, elevation: Math.PI / 4, farM: 2000 };

/** The prop types a gutted building's parts become, as the catalog names them. */
const SHELLS: ReadonlySet<string> = new Set(["burnt_shell"]);

/** The buildings `placed` presented from `ART` (or `art`), casting under
 *  `sun` (null: no shadows); each building one part, its owner prop. */
const walkOf = (
  placed: PlacedBuildings,
  style = STYLE,
  art = ART,
  sun: PresentedSun | null = SUN,
) =>
  presentBuildings(placed, art, style, sun ?? undefined, {
    partBuilding: new Map(Array.from(placed.owners, (owner, b) => [owner, b])),
    shells: SHELLS,
  });

/** What a side that saw building `b` (owner `100 + b`, as `placedOf` numbers
 *  them) destroyed into `state` was published: its part's place taken by a
 *  burnt shell or by remains. */
const seen = (b: number, state: "ruin" | "gutted") => ({
  authoredProp: 100 + b,
  kind: state === "gutted" ? "burnt_shell" : "ruin",
  destroyed: false,
});
type Walk = ReturnType<typeof walkOf>;

/** Frames `camera` at 1080 pixels; true when what is drawn or cast changed. */
const select = (walk: Walk, camera: Camera3DParams) => walk.look(camera, 1080);

/** Row `i` of `template`'s `state` placed at `frame` (x, y, z, yaw): turned
 *  by the frame's yaw about +Z, then moved, as float32. */
function placedRow(
  template: string,
  state: TemplateState,
  i: number,
  [x, y, z, yaw]: readonly number[],
): number[] {
  const r = templateRows(LIBRARY, template, state).first + i;
  const t = LIBRARY.rows.transform.subarray(
    r * ROW_TRANSFORM_FLOATS,
    (r + 1) * ROW_TRANSFORM_FLOATS,
  );
  const [cos, sin] = [Math.cos(yaw), Math.sin(yaw)];
  return [x + cos * t[0] - sin * t[1], y + sin * t[0] + cos * t[1], z + t[2], yaw + t[3]]
    .concat([...t.subarray(4)])
    .map(Math.fround);
}

/** What is drawn as "module@tier", sorted. */
const tally = (list: Drawn[]) => list.map((d) => `${d.module}@${d.tier}`).sort();

/** Decode the same mask the renderer uses, retaining every record in each
 *  cascade so missing casters and duplicate submissions both fail. */
const casterLists = (walk: Walk) =>
  [0, 1, 2, 3].map((cascade) =>
    walk.drawn(true).filter((d) => (d.cascadeMask & (1 << cascade)) !== 0),
  );
const everyCascade = <T>(rows: T[]) => [rows, rows, rows, rows];

test("a row is drawn where the placed frame puts it: a turned building's records are the resolver's", () => {
  const frame = { translation: [1000, 2000, 3] as [number, number, number], yaw: 0.7 };
  const walk = walkOf(placedOf([["house", [...frame.translation, frame.yaw]]]));
  select(walk, view(1000, 2000, 60));
  const row = (i: number) => placedRow("house", "intact", i, [...frame.translation, frame.yaw]);
  const fine = walk.drawn();
  expect(fine.every((d) => d.tier === 0)).toBe(true);
  expect(walk.stats().resident).toBe(4);
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
  const walk = walkOf(placedOf([["house", [1000, 2000, 0, 0]]]));
  const at = (distance: number) => {
    select(walk, view(1000, 2000, distance));
    return { view: tally(walk.drawn()), cast: casterLists(walk).map(tally) };
  };
  // Tier 0: all four rows, from the pool; they cast a tier coarser.
  expect(at(60)).toEqual({
    view: [`${ROOF}@0`, `${SHELL}@0`, `${WINDOW}@0`, `${WINDOW}@0`].sort(),
    cast: everyCascade([`${ROOF}@1`, `${SHELL}@1`, `${WINDOW}@1`, `${WINDOW}@1`].sort()),
  });
  // Tier 1: the roof's mask skips it.
  expect(at(200)).toEqual({
    view: [`${SHELL}@1`, `${WINDOW}@1`],
    cast: everyCascade([`${SHELL}@2`, `${WINDOW}@2`]),
  });
  // Tier 2: the shell alone, casting at the coarsest mesh.
  expect(at(500)).toEqual({ view: [`${SHELL}@2`], cast: everyCascade([`${SHELL}@3`]) });
  // Tier 3: the coarse rows, the roof back among them.
  expect(at(2000)).toEqual({
    view: [`${ROOF}@3`, `${SHELL}@3`].sort(),
    cast: everyCascade([`${ROOF}@3`, `${SHELL}@3`].sort()),
  });
  expect(walk.stats().resident).toBe(0);
});

test("a lower graphics quality draws a building coarser at the same distance, and the authored tiers return at 1", () => {
  const walk = walkOf(placedOf([["house", [1000, 2000, 0, 0]]]));
  const shell = () => {
    select(walk, view(1000, 2000, 200));
    return walk.drawn().find((d) => d.module === SHELL)?.tier;
  };
  expect(shell()).toBe(1);
  // Thresholds three times the authored: a building needs three times the
  // pixels per metre for each tier.
  walk.setDetailScale(1, 3);
  expect(shell()).toBe(2);
  walk.setDetailScale(1, 1);
  expect(shell()).toBe(1);
});

test("a forced tier draws every building at that tier from any distance, and no other tier", () => {
  // Two blocks of many rows a chunk apart, from near and from afar.
  const placed = placedOf([
    ["dense", [1000, 2000, 0, 0]],
    ["house", [1100, 2000, 0, 0]],
  ]);
  for (const tier of [0, 1, 2, 3]) {
    for (const distance of [60, 200, 500, 2000]) {
      const walk = walkOf(placed, tierStyle(STYLE, tier));
      select(walk, view(1050, 2000, distance));
      const tiers = [...new Set(walk.drawn().map((d) => d.tier))];
      expect(tiers, `tier ${tier} from ${distance} m`).toEqual([tier]);
      expect(walk.stats().refused).toBe(0);
      walk.free();
    }
  }
});

test("a chunk is expanded once when it comes near, and leaves the pool when it leaves", () => {
  const walk = walkOf(placedOf([["house", [1000, 2000, 0, 0]]]));
  expect(select(walk, view(1000, 2000, 60))).toBe(true);
  expect(walk.stats()).toMatchObject({ expanded: 4, resident: 4, residentChunks: 1 });
  // A still camera is not walked again: every batch is named unchanged. A
  // camera that moves without changing the chunk's tier expands nothing.
  expect(select(walk, view(1000, 2000, 60))).toBe(false);
  select(walk, view(1004, 2003, 70));
  expect(walk.stats()).toMatchObject({ expanded: 0, resident: 4 });
  // Farther: the tier's rows replace the finer ones.
  select(walk, view(1000, 2000, 200));
  expect(walk.stats()).toMatchObject({ expanded: 2, resident: 2 });
  // Out of view: nothing is resident, and nothing is drawn.
  select(walk, view(5000, 2000, 60));
  expect(walk.stats()).toMatchObject({ resident: 0, residentChunks: 0 });
  expect(walk.drawn()).toEqual([]);
  // Yet its shadow still lands in view from just outside it.
  select(walk, view(1055, 2000, 60));
  expect(tally(walk.drawn())).toEqual([]);
  // The nearest receiver partition excludes it; the other three retain it.
  const coarse = [`${ROOF}@3`, `${SHELL}@3`].sort();
  expect(casterLists(walk).map(tally)).toEqual([[], coarse, coarse, coarse]);
});

test("a building is in one chunk whole, however its rows straddle a chunk's edge", () => {
  // Placed a metre west of a chunk edge at x = 1024: its roof's own origin
  // and its east-wall windows stand past it.
  const walk = walkOf(
    placedOf([
      ["house", [1023, 2000, 0, 0]],
      ["house", [1100, 2000, 0, 0]],
    ]),
  );
  // Two chunks, one building each, all 4 coarse rows between them.
  expect(walk.stats()).toMatchObject({ chunks: 2, coarse: 4 });
  // From over the first, the second is a tier coarser: no row of either
  // takes the other's tier.
  select(walk, view(1023, 2000, 100));
  expect(walk.stats()).toMatchObject({ residentChunks: 2, resident: 6 });
  const tiers = (x: number) => [
    ...new Set(
      walk
        .drawn()
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
  const walk = walkOf(placedOf(blocks));
  let refused = 0;
  let expanded = 0;
  for (let x = 800; x <= 2700; x += 37) {
    select(walk, view(x, 1120, 100));
    const stats = walk.stats();
    refused += stats.refused;
    expanded += stats.expanded;
    expect(stats.resident).toBeLessThanOrEqual(STYLE.pool_records);
    const shells = walk.drawn().filter((d) => d.module === SHELL);
    // No building twice (coarse and resident at once), none half drawn.
    const at = shells.map((d) => `${d.transform[0]},${d.transform[1]}`);
    expect(new Set(at).size).toBe(at.length);
    // Whatever stands under the camera is drawn, at the finest tier.
    const under = blocks.filter(([, f]) => Math.abs(f[0] - x) <= 12 && Math.abs(f[1] - 1120) <= 12);
    for (const [, f] of under) {
      const shell = shells.find((d) => d.transform[0] === f[0] && d.transform[1] === f[1]);
      expect(shell?.tier, `the block at ${f[0]}, ${f[1]} from x = ${x}`).toBe(0);
      const windows = walk
        .drawn()
        .filter((d) => d.module === WINDOW && Math.abs(d.transform[0] - f[0] - 5) < 1e-3);
      expect(windows.filter((d) => Math.abs(d.transform[1] - f[1]) < 5)).toHaveLength(39);
    }
  }
  // The bound was met on the way, and rows came and went.
  expect(refused).toBeGreaterThan(0);
  expect(expanded).toBeGreaterThan(STYLE.pool_records);
  // Past the town: nothing is left in the pool.
  select(walk, view(9000, 1120, 100));
  expect(walk.stats()).toMatchObject({ resident: 0, residentChunks: 0 });
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
  expect(() => walkOf(placed)).toThrow(/template\.missing: template "shed" has no art/);
});

test("a map needs only the kits its own templates draw from, in every state", () => {
  const standIn = placedOf([["stand_in", [0, 0, 0, 0]]]);
  const house = placedOf([["house", [0, 0, 0, 0]]]);
  expect([...templateKits(LIBRARY, standIn.templates)]).toEqual([BOX_KIT]);
  expect([...templateKits(LIBRARY, house.templates)]).toEqual(["city_kit_test"]);
  // With the box kit alone installed, the stand-in draws and the house waits.
  const boxOnly: BuildingArt = { ...ART, bounds: [null, null, null, ART.bounds[BOX]] };
  expect(artCovers(standIn, boxOnly)).toBe(true);
  expect(artCovers(house, boxOnly)).toBe(false);
  expect(artCovers(house, ART)).toBe(true);
  const walk = walkOf(standIn, STYLE, boxOnly, null);
  select(walk, view(0, 0, 60));
  expect(tally(walk.drawn())).toEqual([`${BOX}@0`]);
});

test("a building seen to collapse changes only its own records: it leaves the intact rows and draws as its template's ruin", () => {
  const walk = walkOf(
    placedOf([
      ["house", [1000, 2000, 0, 0.5]],
      ["house", [1020, 2000, 0, 0]],
    ]),
  );
  const near = view(1010, 2000, 60);
  const far = view(1010, 2000, 2000);
  const row = (i: number) => placedRow("house", "ruin", i, [1000, 2000, 0, 0.5]);
  /** What is drawn of the first building (west of 1010) and of its
   *  neighbour, "ruin" for a record of the first's ruin rows. */
  const kinds = (list: Drawn[]) =>
    list
      .map((d) => {
        const ruin = [row(0), row(1)].some((r) => r.every((v, k) => v === d.transform[k]));
        const who = d.transform[0] < 1010 ? "first" : "neighbour";
        return `${who} ${ruin ? "ruin" : "intact"} ${d.module}@${d.tier}`;
      })
      .sort();
  select(walk, near);
  expect(walk.stats().resident).toBe(8);
  walk.know([seen(0, "ruin")]);
  select(walk, near);
  expect(walk.fallen()).toEqual([{ building: 0, state: "ruin" }]);
  // Its chunk is expanded again without it; the neighbour is still whole.
  expect(walk.stats()).toMatchObject({ expanded: 4, resident: 4 });
  expect(kinds(walk.drawn())).toEqual(
    [
      `neighbour intact ${ROOF}@0`,
      `neighbour intact ${SHELL}@0`,
      `neighbour intact ${WINDOW}@0`,
      `neighbour intact ${WINDOW}@0`,
      `first ruin ${SHELL}@0`,
      `first ruin ${ROOF}@0`,
    ].sort(),
  );
  // Its remains: its template's ruin rows at the camera's tier, placed at
  // its own frame (turned with it) in the rows' own tints.
  const ruin = walk.drawn().filter((d) => d.transform[0] < 1010);
  expect(ruin.find((d) => d.module === SHELL)!.transform).toEqual(row(0));
  expect(ruin.find((d) => d.module === ROOF)!.transform).toEqual(row(1));
  const tint = color.fromSRGB([90 / 255, 80 / 255, 70 / 255]);
  ruin.find((d) => d.module === SHELL)!.tint.forEach((c, k) => expect(c).toBeCloseTo(tint[k], 5));
  // From afar its intact coarse rows are gone, not the neighbour's, and its
  // remains are the ruin's own coarse row, which casts for it.
  select(walk, far);
  const farDrawn = [
    `neighbour intact ${ROOF}@3`,
    `neighbour intact ${SHELL}@3`,
    `first ruin ${SHELL}@3`,
  ].sort();
  expect(kinds(walk.drawn())).toEqual(farDrawn);
  expect(casterLists(walk).map(kinds)).toEqual(everyCascade(farDrawn));

  // The building is known intact again (another side's view): restored.
  walk.know([]);
  select(walk, far);
  expect(kinds(walk.drawn())).toEqual(
    [
      `first intact ${ROOF}@3`,
      `first intact ${SHELL}@3`,
      `neighbour intact ${ROOF}@3`,
      `neighbour intact ${SHELL}@3`,
    ].sort(),
  );
  select(walk, near);
  expect(walk.stats().resident).toBe(8);
});

test("a building known gutted is its template's gutted rows at every tier, and never its intact shell", () => {
  const walk = walkOf(
    placedOf([
      ["tower", [1000, 2000, 0, 0]],
      ["house", [1020, 2000, 0, 0]],
    ]),
  );
  const burnt = color.fromSRGB([40 / 255, 38 / 255, 36 / 255]);
  const burntWindow = color.fromSRGB([10 / 255, 10 / 255, 10 / 255]);
  const west = (list: Drawn[]) => list.filter((d) => d.transform[0] < 1010);
  /** The tower's shells as drawn: at which tier, whether each wears the
   *  gutted rows' burnt tint (the intact rows' is white), and how tall. */
  const tower = (list: Drawn[]) =>
    west(list)
      .filter((d) => d.module === SHELL)
      .map((d) => ({
        tier: d.tier,
        burnt: Math.abs(d.tint[0] - burnt[0]) < 1e-5,
        height: d.transform[6],
      }));
  walk.know([seen(0, "gutted")]);
  // Tiers 0, 1, 2 and 3, each from the distance the game changes to it at.
  for (const [distance, tier] of [
    [60, 0],
    [200, 1],
    [500, 2],
    [2000, 3],
  ]) {
    select(walk, view(1000, 2000, distance));
    // One shell, the gutted rows', burnt and at full height: the shell stands.
    expect(tower(walk.drawn()), `tier ${tier}`).toEqual([{ tier, burnt: true, height: 5 }]);
    // Its burnt window is a fine row: at the two finest tiers and no other.
    const windows = west(walk.drawn()).filter((d) => d.module === WINDOW);
    expect(
      windows.map((d) => ({ tier: d.tier, burnt: Math.abs(d.tint[0] - burntWindow[0]) < 1e-5 })),
    ).toEqual(tier < 2 ? [{ tier, burnt: true }] : []);
    // What casts for it is its own coarse gutted shell.
    expect(casterLists(walk).map(tower)).toEqual(
      everyCascade([{ tier: 3, burnt: true, height: 5 }]),
    );
  }
  // Its neighbour is untouched at every one of those.
  select(walk, view(1000, 2000, 60));
  expect(tally(walk.drawn().filter((d) => d.transform[0] >= 1010))).toEqual(
    [`${ROOF}@0`, `${SHELL}@0`, `${WINDOW}@0`, `${WINDOW}@0`].sort(),
  );
  expect(walk.stats().resident).toBe(4);
});

test("a destroyed building whose template has no rows for the state is refused by name, and nothing changes", () => {
  const placed = placedOf([
    ["tower", [1000, 2000, 0, 0]],
    ["dense", [1020, 2000, 0, 0]],
  ]);
  const walk = walkOf(placed);
  const far = view(1010, 2000, 2000);
  select(walk, far);
  const before = walk.drawn();
  // A tower stands gutted: it has no ruin. The dense block has neither. The
  // frame refuses the building by its template's missing state.
  walk.know([seen(0, "ruin")]);
  expect(() => select(walk, far)).toThrow(/state\.missing: template "tower" has no "ruin" rows/);
  walk.know([seen(0, "gutted"), seen(1, "gutted")]);
  expect(() => select(walk, far)).toThrow(/state\.missing: template "dense" has no "gutted" rows/);
  // The tower is not gutted either: a refused knowledge changes nothing.
  select(walk, far);
  expect(walk.drawn()).toEqual(before);
  expect(walk.fallen()).toEqual([]);
});

test("identical neighbours differ a little in value, each the same every time", () => {
  const blocks = placedOf([
    ["stand_in", [1000, 2000, 0, 0]],
    ["stand_in", [1030, 2000, 0, 0]],
  ]);
  const tints = () => {
    const walk = walkOf(blocks, { ...STYLE, tint_jitter: 0.1 });
    select(walk, view(1010, 2000, 2000));
    return walk.drawn().map((d) => d.tint);
  };
  const want = color.fromSRGB([180 / 255, 180 / 255, 190 / 255]);
  const scales = tints().map((tint) => {
    const k = tint[0] / want[0];
    tint.forEach((c, i) => expect(c / want[i]).toBeCloseTo(k, 4));
    expect(Math.abs(k - 1)).toBeLessThanOrEqual(0.1 + 1e-6);
    return k;
  });
  expect(scales).toHaveLength(2);
  expect(scales[0]).not.toBeCloseTo(scales[1], 3);
  expect(tints()).toEqual(tints());
});
