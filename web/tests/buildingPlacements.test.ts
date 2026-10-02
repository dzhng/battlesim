// @vitest-environment node
// Buildings drawn from template art: a map's references, the rows a view
// draws of them at each tier, the pool's residency as the camera moves, and
// what changes when a side sees one fall.
import { expect, test } from "vitest";
import { color } from "math/color";
import { PROTOTYPE_KIT, PROTOTYPE_MODULE } from "@packages/scene-assets/src/prototypeSet";
import {
  resolve,
  ROW_TRANSFORM_FLOATS,
  type TemplateArtLibrary,
} from "@packages/scene-assets/src/templateLibrary";
import { createDetailView, setDetailView } from "@packages/battle-renderer/src/frame/detailView";
import {
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
      { appearance: PROTOTYPE_KIT, bundle: "b" },
    ],
    modules: [
      { kit: 0, module: "shell" },
      { kit: 0, module: "window" },
      { kit: 0, module: "roof" },
      { kit: 1, module: PROTOTYPE_MODULE },
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
  [ROOF, 0, 0, 6, 0, 1, 1, 2, 9, 128, 255, 64],
];
// A block with many fine rows: what fills a pool.
const DENSE: Row[] = [
  [SHELL, 0, 0, 0, 0, 1, 1, 1, ALL, 255, 255, 255],
  ...Array.from(
    { length: 39 },
    (_, i): Row => [WINDOW, 5, i * 0.2 - 4, 1, 0, 1, 1, 1, 1, 255, 255, 255],
  ),
];
const LIBRARY = libraryOf({
  house: { intact: HOUSE },
  dense: { intact: DENSE },
  stand_in: { intact: [[BOX, 0, 0, 0, 0, 20, 10, 12, ALL, 180, 180, 190]] },
  wrecked: {
    intact: [[SHELL, 0, 0, 0, 0, 1, 1, 1, ALL, 255, 255, 255]],
    ruin: [[BOX, 1, 0, 0, 0, 10, 8, 1.5, ALL, 90, 80, 70]],
  },
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
  prototype_tints: { default: [0.5, 0.5, 0.5] },
  ruin_tint: [0.3, 0.3, 0.3],
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
  // Placed a metre west of a chunk edge at x = 1024: its east-wall windows
  // stand past it.
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

test("only buildings whose template has art are references, each with its frame and its parts", () => {
  const buildings: PublicBuildings = {
    catalogueHash: "h",
    buildings: [
      ["house", 7, [3, 4]],
      ["village-house", 9, [5]],
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
  const index = indexBuildings(buildings, props, (id) =>
    LIBRARY.templates.some((t) => t.id === id),
  );
  expect(index.placed.templates).toEqual(["house", "stand_in"]);
  expect([...index.placed.template]).toEqual([0, 1]);
  expect([...index.placed.owners]).toEqual([7, 11]);
  expect([...index.placed.frames]).toEqual([0, 50, 2, 0, 200, 50, 2, 1]);
  expect([...index.partBuilding]).toEqual([
    [3, 0],
    [4, 0],
    [6, 1],
  ]);
  // The village's house is not one of them: its part is no art-drawn part.
  expect(index.partBuilding.has(5)).toBe(false);
});

test("a side draws a building intact until it has seen it fall, then each part as it knows it", () => {
  const buildings: PublicBuildings = {
    catalogueHash: "h",
    buildings: [0, 1].map((i) => ({
      owner: 10 * i,
      kind: "building",
      templateId: "house",
      category: "detached_home",
      regionalFamily: "china",
      frame: { translation: [1000 + 20 * i, 2000, 0] as [number, number, number], yaw: 0 },
      parts: [{ part: "main", prop: 10 * i }].concat(i ? [] : [{ part: "wing", prop: 1 }]),
    })),
  };
  const props = [
    part(0, 1000, 2000, [5, 4, 3]),
    part(1, 1004, 2004, [2, 2, 3]),
    part(10, 1020, 2000, [5, 4, 3]),
  ];
  const index = indexBuildings(buildings, props, () => true);
  expect(fallenBuildings(index, [])).toEqual([]);
  // Blue saw the first building's main part come down: its remains, and the
  // wing as it last knew it. A wreck is no building's.
  const remains: KnownProp = {
    ...part(0, 1000, 2000, [5, 4, 0.75]),
    kind: "ruin",
    authoredProp: 0,
    replaces: 0,
  };
  const wreck: KnownProp = { ...remains, kind: "heavy_wreck", authoredProp: null, replaces: null };
  expect(fallenBuildings(index, [remains, wreck])).toEqual([
    { building: 0, state: "ruin", parts: [remains, props[1]] },
  ]);
  // A part seen destroyed with nothing left draws nothing.
  const gone: KnownProp = { ...remains, authoredProp: 1, replaces: 1, destroyed: true };
  expect(fallenBuildings(index, [remains, gone])[0].parts).toEqual([remains]);
});

test("a building seen to fall changes only its own records: it leaves the intact rows and draws as its remains", () => {
  const scene = createBuildingScene(
    placedOf([
      ["house", [1000, 2000, 0, 0]],
      ["house", [1020, 2000, 0, 0]],
      ["wrecked", [3000, 2000, 0, 0.5]],
    ]),
    ART,
    STYLE,
  );
  const near = view(1010, 2000, 60);
  const far = view(1010, 2000, 2000);
  selectBuildings(scene, far, SHADOW);
  scene.coarseDirty.length = 0;

  const box = part(0, 1000, 2000, [5, 4, 0.75]);
  setFallenBuildings(scene, [{ building: 0, state: "ruin", parts: [box] }]);
  // Its two coarse records are the only ones rewritten: a run of one each.
  const rewritten = scene.coarseDirty.filter((_, i) => i % 2 === 0);
  expect(scene.coarseDirty.filter((_, i) => i % 2 === 1)).toEqual([1, 1]);
  for (const at of rewritten) {
    const record = scene.coarse.records.subarray(at * RECORD, (at + 1) * RECORD);
    expect([record[0], record[1]]).toEqual([1000, 2000]);
    expect([...record.subarray(12, 15)]).toEqual([0, 0, 0]);
  }
  selectBuildings(scene, near, SHADOW);
  setFallenBuildings(scene, []);
  selectBuildings(scene, near, SHADOW);
  expect(scene.pool.used).toBe(8);
  setFallenBuildings(scene, [{ building: 0, state: "ruin", parts: [box] }]);
  selectBuildings(scene, near, SHADOW);
  // Its chunk is expanded again without it; the neighbour is still whole.
  expect(scene.expandedRows).toBe(4);
  expect(scene.pool.used).toBe(4);
  const standing = drawn(scene).filter((d) => d.source === POOL);
  expect(standing.every((d) => Math.abs(d.transform[0] - 1020) < 6)).toBe(true);
  // Its remains: the part's box in the ruin tint, at the camera's tier.
  const ruin = drawn(scene).filter((d) => d.source === RUINS);
  expect(ruin.map((d) => [d.module, d.tier])).toEqual([[BOX, 0]]);
  expect(ruin[0].transform).toEqual([1000, 2000, 0, 0.25, 10, 8, 1.5].map(Math.fround));
  const tint = color.fromSRGB([0.3, 0.3, 0.3]);
  ruin[0].tint.forEach((c, k) => expect(c).toBeCloseTo(tint[k], 2));
  // From afar its coarse rows are hidden, not the neighbour's, and its
  // remains draw at the coarsest tier.
  selectBuildings(scene, far, SHADOW);
  expect(tally(drawn(scene))).toEqual([`${BOX}@3`, `${ROOF}@3`, `${SHELL}@3`].sort());
  expect(tally(drawn(scene, true))).toEqual([`${BOX}@3`, `${ROOF}@3`, `${SHELL}@3`].sort());

  // A template with ruin rows draws those, at its frame, not its parts' boxes.
  setFallenBuildings(scene, [{ building: 2, state: "ruin", parts: [box] }]);
  selectBuildings(scene, view(3000, 2000, 60), SHADOW);
  const art = drawn(scene).filter((d) => d.source === RUINS);
  expect(art).toHaveLength(1);
  expect(art[0].transform).toEqual(
    [3000 + Math.cos(0.5), 2000 + Math.sin(0.5), 0, 0.5, 10, 8, 1.5].map(Math.fround),
  );
  // The first building is known intact again (another side's view): restored.
  selectBuildings(scene, far, SHADOW);
  expect(tally(drawn(scene).filter((d) => d.source === COARSE))).toEqual(
    [`${ROOF}@3`, `${ROOF}@3`, `${SHELL}@3`, `${SHELL}@3`].sort(),
  );
  setFallenBuildings(scene, []);
  expect(scene.ruins).toBeNull();
  selectBuildings(scene, near, SHADOW);
  expect(scene.pool.used).toBe(8);
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
  expect(() => validateBuildingStyle({ ...STYLE, prototype_tints: {} })).toThrow(/default/);
});
