// @vitest-environment node
import { createHash } from "node:crypto";
import { expect, test } from "vitest";
import {
  encodePresentationCapture,
  type PresentationCapture,
  type PresentationFrame,
} from "@web/battle/benchmark/presentationCapture";
import {
  readTerrainArtifact,
  validateVisualPresentation,
  writeVisualPresentation,
  type DrawnPresentation,
  type GrassCount,
  type VisualPresentation,
} from "@web/battle/benchmark/visualPresentation";
import type { TerrainTables, WordTable } from "@packages/battle-renderer/src/frame/terrainTables";
import { sunDirection } from "@packages/battle-renderer/src/light/sceneLight";
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
const cells = (n: number) => Array.from({ length: 12 }, (_, i) => i * n);
const grassCounts: GrassCount[] = [{ plate: "plate-1.png", cells: cells(3) }];
const platedCapture = () =>
  encodePresentationCapture(capture([frame(0), frame(1, [1280, 720], "plate-1.png")]));
/** The manifest and its artifacts for `captureJson`, drawn with the tables and counts above. */
async function written(captureJson: string, counts: GrassCount[] = []) {
  const { manifest, artifacts } = await writeVisualPresentation({
    captureJson,
    mapBytes,
    drawn,
    terrain,
    grassCounts: counts,
  });
  return { manifest: JSON.parse(manifest) as VisualPresentation, artifacts };
}
const sha256 = (bytes: Uint8Array | string) => createHash("sha256").update(bytes).digest("hex");

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
    layers: drawn.layers,
    omissions: [],
    terrain: expect.any(Array),
    grassCounts: [],
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
  expect(artifacts.map((a) => a.file)).toEqual(
    manifest.terrain.map((t) => `terrain-${t.name}.u32`),
  );
  for (const [i, entry] of manifest.terrain.entries()) {
    const { bytes } = artifacts[i];
    const table = terrain[entry.name];
    expect(entry).toEqual({
      name: entry.name,
      sha256: sha256(bytes),
      words: table.words.length,
      layout: table.layout,
    });
    expect(await readTerrainArtifact(entry, bytes)).toEqual(table.words);
  }
  // Little-endian: the first height's f32 1.5 lies low byte first.
  const heights = artifacts[0].bytes;
  expect(new DataView(heights.buffer, heights.byteOffset).getFloat32(16, true)).toBe(1.5);
  expect(Array.from(heights.subarray(16, 20))).toEqual([0, 0, 0xc0, 0x3f]);
});

test("an artifact whose bytes are not the ones the manifest hashed is refused", async () => {
  const { manifest, artifacts } = await written(platedCapture(), grassCounts);
  const [entry] = manifest.terrain;
  const bytes = artifacts[0].bytes;
  const tampered = bytes.slice();
  tampered[20] ^= 1;
  await expect(readTerrainArtifact(entry, tampered)).rejects.toThrow(/sha256/);
  await expect(readTerrainArtifact(entry, bytes.subarray(0, bytes.length - 4))).rejects.toThrow(
    /words/,
  );
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
