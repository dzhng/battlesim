// @vitest-environment node
import { createHash } from "node:crypto";
import { expect, test } from "vitest";
import {
  encodePresentationCapture,
  type PresentationCapture,
  type PresentationFrame,
} from "@web/battle/benchmark/presentationCapture";
import {
  validateVisualPresentation,
  writeVisualPresentation,
  type DrawnPresentation,
  type GrassCount,
  type InstanceCount,
  type VisualPresentation,
} from "@web/battle/benchmark/visualPresentation";
import type { TerrainTables } from "@packages/battle-renderer/src/frame/terrainTables";
import type { GrassTables } from "@packages/battle-renderer/src/frame/grassTables";
import type { WordTable } from "@packages/battle-renderer/src/frame/wordTables";
import type { BuildingTables } from "@packages/battle-renderer/src/models/buildingTables";
import type { SceneryTables } from "@packages/battle-renderer/src/scenery/sceneryTables";
import { sunDirection } from "@packages/battle-renderer/src/light/sceneLight";
import { terrainTuning } from "@packages/battle-renderer/src/terrain/terrainTuning";
import { grassTuning } from "@packages/battle-renderer/src/terrain/grassTuning";
import { gameLight } from "@apps/battle-lab/src/gameLight";

const frame = (
  index: number,
  canvas: [number, number] = [1280, 720],
  plate: string | null = null,
): PresentationFrame => ({
  index,
  reelMs: index * 16,
  tick: 90,
  simTime: 3,
  eye: [0, -40, 6],
  target: [0, 0, 0],
  up: [0, 0.1, 1],
  canvas,
  plate,
});

const capture = (frames: PresentationFrame[]): PresentationCapture => ({
  schema: "battle-presentation-capture",
  workload: {
    id: "menu-reel",
    fingerprint: "deadbeef",
    scene: "paris-corner",
    map: "paris-corner",
    encounter: "menu",
    seed: 7,
  },
  tickHz: 30,
  warmTick: 90,
  side: "blue",
  layout: "{}",
  samples: [],
  frames,
});

const drawn: DrawnPresentation = {
  lens: { fovY: 0.8, near: 1, depth: "reverse-z", far: "infinite" },
  light: {
    ...gameLight,
    sunDirection: Array.from(sunDirection(gameLight)) as [number, number, number],
  },
  tuning: { terrain: terrainTuning, grass: grassTuning },
  layers: ["sky", "terrain", "grass"],
  omissions: [],
};
const mapBytes = new TextEncoder().encode('{"map":"bytes"}');

const records = (words: number[]): WordTable => ({
  words: Uint32Array.from(words),
  layout: {
    kind: "records",
    stride: 4,
    count: words.length / 4,
    fields: [{ name: "line", word: 0, type: "f32", count: 4 }],
  },
});
/** Small tables of every kind; the heights hold a page whose first height is 1.5 m. */
const terrain: TerrainTables = {
  heights: {
    words: Uint32Array.from([2, 1, 1, 1, 0x3fc00000, 0, 0, 0]),
    layout: { kind: "height-pages", nx: 2, ny: 2, spacing: 4, pageSize: 2 },
  },
  nodes: records([1, 2, 3, 4]),
  plots: records([5, 6, 7, 8, 9, 10, 11, 12]),
  surfaces: records([0xffffffff, 0, 1, 2]),
  surfaceIndex: {
    words: Uint32Array.from([3, 0, 1 << 30]),
    layout: { kind: "words", count: 3, constants: { kindShift: 30 } },
  },
  params: records([13, 14, 15, 16]),
};
/** A small grass field: two u16 strips, an empty prop grid, two kinds. */
const grass: GrassTables = {
  tables: {
    params: records([1, 2, 3, 4]),
    props: {
      words: Uint32Array.from([0, 0, 0x41000000, 0, 0, 0, 0, 0]),
      layout: { kind: "prop-cells", count: 2 },
    },
    shapes: records([0x3f800000, 0, 0, 0]),
    indicesNear: {
      words: Uint32Array.from([0x00010000, 0x00000002]),
      layout: { kind: "u16", count: 3 },
    },
    indicesFar: { words: Uint32Array.from([0x00010000]), layout: { kind: "u16", count: 2 } },
  },
  constants: {
    tileM: 4,
    segments: [4, 2],
    bladeVertices: [9, 5],
    capacity: [100, 200],
    firstClump: [0, 100],
    indexCount: [3, 2],
    maxBlades: 1,
    edgeM: 1.2,
    heightVariation: [0.8, 1.2],
  },
  kinds: ["meadow_grass", "wheat"],
};
/** Two buildings' tables: a record each, and module rows in each state. */
const buildings: BuildingTables = {
  tables: {
    buildings: records([1, 2, 3, 4, 5, 6, 7, 8]),
    intact: records([9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]),
    ruin: records([21, 22, 23, 24]),
    gutted: records([]),
  },
  modules: [
    { kit: "city_kit_paris", module: "balcony" },
    { kit: "city_kit_paris", module: "railing" },
  ],
  tiers: {
    lodPxPerM: [12, 5, 2],
    chunkM: 64,
    poolRecords: 1024,
    expandRows: 4000,
    casterCoarser: 1,
  },
};
const scenery: SceneryTables = {
  tables: {
    forest: records([0x40000000, 0, 0, 0]),
    understorey: records([]),
    backdrop: records([1, 2, 3, 4, 5, 6, 7, 8]),
    dressing: records([0x3f000000, 1, 2, 3]),
    dressingCells: records([3, 4, 0, 1]),
  },
  kinds: ["oak", "fern"],
  tiers: {
    lodPx: [400, 150, 40],
    chunkM: 128,
    casterCoarser: 1,
    dressing: {
      lodPx: [300, 100, 20],
      fadePx: 6,
      gone: 0.5,
      cellM: 64,
      topM: 0.6,
      reachM: 0.4,
      ground: [-1, 9],
      slots: 128,
      cellsPerView: 2,
    },
  },
};
const tables = { terrain, grass, buildings, scenery };
const cells = (n: number) => Array.from({ length: 12 }, (_, i) => i * n);
const tiered = (kinds: number, n: number) => Array.from({ length: kinds * 4 }, (_, i) => i * n);
const instanceCounts: InstanceCount[] = [
  {
    plate: "plate-1.png",
    buildings: tiered(2, 3),
    scenery: {
      forest: tiered(2, 2),
      understorey: tiered(2, 0),
      backdrop: tiered(2, 5),
      dressing: tiered(2, 7),
      felled: tiered(2, 0),
      stumps: [1, 0],
    },
    knownProps: [
      { appearance: "abrams_wreck", levels: [0, 1, 0, 0, 2] },
      { appearance: "car_wreck_sedan", levels: [3, 0, 0, 0, 0] },
    ],
  },
];
const grassCounts: GrassCount[] = [{ plate: "plate-1.png", cells: cells(3) }];
const platedCapture = () =>
  encodePresentationCapture(capture([frame(0), frame(1, [1280, 720], "plate-1.png")]));
/** The manifest and its artifacts for `captureJson`, drawn with the tables and counts above. */
async function written(
  captureJson: string,
  counts: GrassCount[] = [],
  instances: InstanceCount[] = counts.length ? instanceCounts : [],
) {
  const { manifest, artifacts } = await writeVisualPresentation({
    captureJson,
    mapBytes,
    drawn,
    tables,
    grassCounts: counts,
    instanceCounts: instances,
  });
  return { manifest: JSON.parse(manifest) as VisualPresentation, artifacts };
}
const sha256 = (bytes: Uint8Array | string) => createHash("sha256").update(bytes).digest("hex");
/** An artifact file's bytes read back as little-endian u32 words. */
const wordsOf = (bytes: Uint8Array) => {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  return Uint32Array.from({ length: bytes.length / 4 }, (_, i) => view.getUint32(i * 4, true));
};

test("the manifest names its battle by the capture's workload and the hashes of both files", async () => {
  const captureJson = encodePresentationCapture(capture([frame(0), frame(1)]));
  const { manifest } = await written(captureJson);
  expect(validateVisualPresentation(manifest)).toEqual(manifest);
  expect(manifest).toEqual({
    schema: "native-visual-presentation",
    identity: {
      scene: "paris-corner",
      map: "paris-corner",
      encounter: "menu",
      seed: 7,
      mapHash: sha256(mapBytes),
      captureHash: sha256(captureJson),
    },
    viewport: { width: 1280, height: 720, scale: 1 },
    lens: drawn.lens,
    light: drawn.light,
    tuning: drawn.tuning,
    layers: drawn.layers,
    omissions: [],
    terrain: expect.any(Array),
    grass: expect.any(Object),
    buildings: expect.any(Object),
    scenery: expect.any(Object),
    grassCounts: [],
    instanceCounts: [],
  });
});

test("a run whose canvas changed size, or drew nothing, has no one viewport and is refused", async () => {
  const resized = encodePresentationCapture(capture([frame(0), frame(1, [1280, 800])]));
  await expect(written(resized)).rejects.toThrow(/canvas/);
  const empty = encodePresentationCapture(capture([]));
  await expect(written(empty)).rejects.toThrow(/frame/);
});

test("the validator refuses a versioned id, the hashless identity and an unknown layer", async () => {
  const captureJson = encodePresentationCapture(capture([frame(0)]));
  const { manifest } = await written(captureJson);
  const reject = (change: object) => () => validateVisualPresentation({ ...manifest, ...change });
  expect(reject({ schema: "native-visual-presentation/v1" })).toThrow(/schema/);
  const { mapHash: _, ...hashless } = manifest.identity;
  expect(reject({ identity: hashless })).toThrow(/identity/);
  expect(reject({ layers: ["terrain", "water-boxes"] })).toThrow(/layer/);
  expect(reject({ layers: ["terrain", "terrain"] })).toThrow(/layer/);
  expect(reject({ omissions: [{ owner: "grass", reason: "" }] })).toThrow(/omission/);
});

test("the manifest carries the light the frames were drawn under, with a unit sun direction", async () => {
  const captureJson = encodePresentationCapture(capture([frame(0)]));
  const { manifest } = await written(captureJson);
  const reject = (change: object) => () => validateVisualPresentation({ ...manifest, ...change });
  const { light: _, ...unlit } = manifest;
  expect(() => validateVisualPresentation(unlit)).toThrow(/malformed/);
  expect(reject({ light: { ...manifest.light, sunDirection: [0, 0, 2] } })).toThrow(/sun/);
  const { sunDirection: _sun, ...sunless } = manifest.light;
  expect(reject({ light: sunless })).toThrow(/sun/);
});

test("each terrain table is an artifact of its words, little-endian, named by the hash of its bytes", async () => {
  const { manifest, artifacts } = await written(platedCapture(), grassCounts);
  expect(manifest.terrain.map((t) => t.name)).toEqual([
    "heights",
    "nodes",
    "plots",
    "surfaces",
    "surfaceIndex",
    "params",
  ]);
  for (const [i, entry] of manifest.terrain.entries()) {
    expect(artifacts[i].file).toBe(`terrain-${entry.name}.u32`);
    const { bytes } = artifacts[i];
    const table = terrain[entry.name];
    expect(entry).toEqual({
      name: entry.name,
      sha256: sha256(bytes),
      words: table.words.length,
      layout: table.layout,
    });
    expect(wordsOf(bytes)).toEqual(table.words);
  }
  // Little-endian: the first height's f32 1.5 lies low byte first.
  const heights = artifacts[0].bytes;
  expect(new DataView(heights.buffer, heights.byteOffset).getFloat32(16, true)).toBe(1.5);
  expect(Array.from(heights.subarray(16, 20))).toEqual([0, 0, 0xc0, 0x3f]);
});

test("grass counts hold one row of twelve cells for each plate the capture drew, and only those", async () => {
  const { manifest } = await written(platedCapture(), grassCounts);
  expect(manifest.grassCounts).toEqual(grassCounts);
  await expect(written(platedCapture(), [])).rejects.toThrow(/plate-1\.png/);
  await expect(
    written(platedCapture(), [...grassCounts, { plate: "plate-0.png", cells: cells(1) }]),
  ).rejects.toThrow(/plate-0\.png/);
  const reject = (row: object) => () =>
    validateVisualPresentation({ ...manifest, grassCounts: [row] });
  expect(reject({ plate: "plate-1.png", cells: cells(1).slice(1) })).toThrow(/grass/);
  expect(reject({ plate: "plate-1.png", cells: [-1, ...cells(1).slice(1)] })).toThrow(/grass/);
  expect(reject({ plate: "plate-1.png", cells: [0.5, ...cells(1).slice(1)] })).toThrow(/grass/);
  expect(reject({ plate: "plate-1.png", cells: cells(1), tier: 0 })).toThrow(/grass/);
});

test("the validator refuses a missing, unknown or stale terrain artifact entry", async () => {
  const { manifest } = await written(platedCapture(), grassCounts);
  const reject = (change: object) => () => validateVisualPresentation({ ...manifest, ...change });
  const { terrain: _, ...bare } = manifest;
  expect(() => validateVisualPresentation(bare)).toThrow(/malformed/);
  const { grassCounts: _counts, ...countless } = manifest;
  expect(() => validateVisualPresentation(countless)).toThrow(/malformed/);
  const [heights, ...rest] = manifest.terrain;
  expect(reject({ terrain: rest })).toThrow(/terrain/);
  expect(reject({ terrain: [{ ...heights, name: "relief" }, ...rest] })).toThrow(/terrain/);
  expect(reject({ terrain: [{ ...heights, bytes: 32 }, ...rest] })).toThrow(/terrain/);
  expect(reject({ terrain: [{ ...heights, sha256: "abc" }, ...rest] })).toThrow(/terrain/);
  expect(
    reject({ terrain: [{ ...heights, layout: { ...heights.layout, origin: [0, 0] } }, ...rest] }),
  ).toThrow(/terrain/);
  const nodes = rest[0];
  if (nodes.layout.kind !== "records") throw new Error("nodes are records");
  const field = { ...nodes.layout.fields[0], type: "f16" };
  expect(
    reject({
      terrain: [
        heights,
        { ...nodes, layout: { ...nodes.layout, fields: [field] } },
        ...rest.slice(1),
      ],
    }),
  ).toThrow(/terrain/);
});

test("the manifest carries the terrain and grass tuning tables the shaders were built from", async () => {
  const { manifest } = await written(encodePresentationCapture(capture([frame(0)])));
  expect(manifest.tuning).toEqual({ terrain: terrainTuning, grass: grassTuning });
  const reject = (tuning: unknown) => () => validateVisualPresentation({ ...manifest, tuning });
  expect(reject({ terrain: terrainTuning })).toThrow(/tuning/);
  expect(reject({ terrain: { mottle: { scale: "4" } }, grass: grassTuning })).toThrow(/tuning/);
  expect(reject({ terrain: { mottle: { scale: Number.NaN } }, grass: grassTuning })).toThrow(
    /tuning/,
  );
});

test("the grass tables are artifacts beside the terrain's, with the field's constants and kinds", async () => {
  const { manifest, artifacts } = await written(platedCapture(), grassCounts);
  expect(manifest.grass.tables.map((t) => t.name)).toEqual([
    "params",
    "props",
    "shapes",
    "indicesNear",
    "indicesFar",
  ]);
  expect(manifest.grass.constants).toEqual(grass.constants);
  expect(manifest.grass.kinds).toEqual(grass.kinds);
  const files = artifacts.slice(manifest.terrain.length);
  for (const [i, entry] of manifest.grass.tables.entries()) {
    expect(files[i].file).toBe(`grass-${entry.name}.u32`);
    expect(entry.sha256).toBe(sha256(files[i].bytes));
    expect(entry.layout).toEqual(grass.tables[entry.name].layout);
    expect(wordsOf(files[i].bytes)).toEqual(grass.tables[entry.name].words);
  }
});

test("the validator refuses a grass section with a missing table, a stray constant or no kinds", async () => {
  const { manifest } = await written(platedCapture(), grassCounts);
  const reject = (change: object) => () =>
    validateVisualPresentation({ ...manifest, grass: { ...manifest.grass, ...change } });
  const { grass: _, ...bare } = manifest;
  expect(() => validateVisualPresentation(bare)).toThrow(/malformed/);
  expect(reject({ tables: manifest.grass.tables.slice(1) })).toThrow(/grass/);
  const [params, props, ...rest] = manifest.grass.tables;
  expect(
    reject({
      tables: [params, { ...props, layout: { kind: "prop-cells", count: 2, cell: 8 } }, ...rest],
    }),
  ).toThrow(/grass/);
  const { maxBlades: _blades, ...short } = manifest.grass.constants;
  expect(reject({ constants: short })).toThrow(/grass/);
  expect(reject({ constants: { ...manifest.grass.constants, lod: 0 } })).toThrow(/grass/);
  expect(reject({ constants: { ...manifest.grass.constants, tileM: "4" } })).toThrow(/grass/);
  const { edgeM: _edge, ...noEdge } = manifest.grass.constants;
  expect(reject({ constants: noEdge })).toThrow(/grass/);
  expect(reject({ constants: { ...manifest.grass.constants, edgeM: -1 } })).toThrow(/grass/);
  expect(reject({ constants: { ...manifest.grass.constants, heightVariation: [0.8] } })).toThrow(
    /grass/,
  );
  expect(reject({ kinds: [] })).toThrow(/grass/);
  expect(reject({ kinds: ["wheat", "wheat"] })).toThrow(/grass/);
  expect(reject({ seed: 1 })).toThrow(/grass/);
});

test("the buildings' and scenery's tables are artifacts after the grass's, with their kinds and tier rules", async () => {
  const { manifest, artifacts } = await written(platedCapture(), grassCounts);
  expect(manifest.buildings.modules).toEqual(buildings.modules);
  expect(manifest.buildings.tiers).toEqual(buildings.tiers);
  expect(manifest.scenery.kinds).toEqual(scenery.kinds);
  expect(manifest.scenery.tiers).toEqual(scenery.tiers);
  let at = manifest.terrain.length + manifest.grass.tables.length;
  for (const [group, source] of [
    ["buildings", buildings.tables],
    ["scenery", scenery.tables],
  ] as const) {
    const entries = manifest[group].tables;
    expect(entries.map((t) => t.name)).toEqual(Object.keys(source));
    for (const entry of entries) {
      const { file, bytes } = artifacts[at++];
      const table = (source as Record<string, WordTable>)[entry.name];
      expect(file).toBe(`${group}-${entry.name}.u32`);
      expect(entry).toEqual({
        name: entry.name,
        sha256: sha256(bytes),
        words: table.words.length,
        layout: table.layout,
      });
      expect(wordsOf(bytes)).toEqual(table.words);
    }
  }
  expect(artifacts).toHaveLength(at);
});

test("instance counts hold one row per plate, sized by the modules and the scenery kinds", async () => {
  const { manifest } = await written(platedCapture(), grassCounts);
  expect(manifest.instanceCounts).toEqual(instanceCounts);
  await expect(written(platedCapture(), grassCounts, [])).rejects.toThrow(/instance.*plate-1\.png/);
  const [row] = instanceCounts;
  const reject = (change: object) => () =>
    validateVisualPresentation({ ...manifest, instanceCounts: [{ ...row, ...change }] });
  expect(reject({})).not.toThrow();
  expect(reject({ buildings: tiered(1, 3) })).toThrow(/instance/);
  expect(reject({ buildings: [-1, ...tiered(2, 3).slice(1)] })).toThrow(/instance/);
  expect(reject({ scenery: { ...row.scenery, forest: tiered(3, 1) } })).toThrow(/instance/);
  expect(reject({ scenery: { ...row.scenery, stumps: [1] } })).toThrow(/instance/);
  const { felled: _, ...unfelled } = row.scenery;
  expect(reject({ scenery: unfelled })).toThrow(/instance/);
  expect(reject({ tier: 0 })).toThrow(/instance/);
  const [wreck] = row.knownProps;
  expect(reject({ knownProps: [{ ...wreck, levels: [0, 1, 0, 0] }] })).toThrow(/instance/);
  expect(reject({ knownProps: [wreck, wreck] })).toThrow(/instance/);
  const { knownProps: _known, ...unknowing } = row;
  expect(() => validateVisualPresentation({ ...manifest, instanceCounts: [unknowing] })).toThrow(
    /instance/,
  );
});

test("the validator refuses building and scenery sections that are missing a table, a kind or a rule", async () => {
  const { manifest } = await written(platedCapture(), grassCounts);
  const reject = (group: "buildings" | "scenery", change: object) => () =>
    validateVisualPresentation({ ...manifest, [group]: { ...manifest[group], ...change } });
  const { scenery: _, ...bare } = manifest;
  expect(() => validateVisualPresentation(bare)).toThrow(/malformed/);
  expect(reject("buildings", { tables: manifest.buildings.tables.slice(1) })).toThrow(/buildings/);
  expect(reject("buildings", { modules: [{ kit: "city_kit_paris" }] })).toThrow(/buildings/);
  const twice = [buildings.modules[0], buildings.modules[0]];
  expect(reject("buildings", { modules: twice })).toThrow(/buildings/);
  const { chunkM: _chunk, ...chunkless } = buildings.tiers;
  expect(reject("buildings", { tiers: chunkless })).toThrow(/buildings/);
  expect(reject("buildings", { tiers: { ...buildings.tiers, lodPxPerM: [12, 5] } })).toThrow(
    /buildings/,
  );
  expect(reject("scenery", { tables: manifest.scenery.tables.slice(0, 4) })).toThrow(/scenery/);
  expect(reject("scenery", { kinds: [] })).toThrow(/scenery/);
  expect(reject("scenery", { kinds: ["oak", "oak"] })).toThrow(/scenery/);
  const { fadePx: _fade, ...fadeless } = scenery.tiers.dressing;
  expect(reject("scenery", { tiers: { ...scenery.tiers, dressing: fadeless } })).toThrow(/scenery/);
  expect(reject("scenery", { tiers: { ...scenery.tiers, chunkM: -1 } })).toThrow(/scenery/);
});
