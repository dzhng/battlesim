// The `native-visual-presentation` manifest: what the browser drew for one
// captured scene, beside its `battle-presentation-capture`, so a second client
// draws the same battle through the same lens and the same layers. The
// browser run is its one writer; it grows only by what native reads, and
// changes in place (a stale file is refused, never migrated).
// Plain Node runs the writer (the scene harness), so it imports nothing the
// renderer owns; the page validates and derives the light before handing it over.
import type { LightPresentation } from "@packages/battle-renderer/src/light/sceneLight";
import type {
  TerrainTableName,
  TerrainTables,
  WordLayout,
} from "@packages/battle-renderer/src/frame/terrainTables";
import { decodePresentationCapture } from "./presentationCapture.ts";

/** The presentation layers a frame can draw. */
export const PRESENTATION_LAYERS = [
  "sky",
  "terrain",
  "water",
  "grass",
  "scenery",
  "props",
  "structures",
  "buildings",
  "models",
  "effects",
  "scars",
  "fog",
  "overlays",
] as const;
export type PresentationLayer = (typeof PRESENTATION_LAYERS)[number];

/** The drawn lens: vertical field of view (radians) and near plane (metres),
 *  through the engine-wide reverse-Z depth with no far plane. */
export interface Lens {
  fovY: number;
  near: number;
  depth: "reverse-z";
  far: "infinite";
}

/** Something the browser meant to draw and did not. */
export interface Omission {
  owner: string;
  reason: string;
  sourceId: string;
}

/** The light the battle frame was built with (`presentation.light`, as
 *  `validateLight` admits it), with the one sun direction its angles give
 *  (`sunDirection`): a unit vector toward the sun in sim space, z up. */
export type ManifestLight = LightPresentation & { sunDirection: [number, number, number] };

/** What the page knows of its own drawing; the files give the rest. */
/** Nested tuning numbers: `terrainTuning` or `grassTuning` as the shaders read them. */
export interface TuningTable {
  readonly [key: string]: number | readonly number[] | TuningTable;
}

export interface DrawnPresentation {
  lens: Lens;
  light: ManifestLight;
  /** The tuning tables the terrain and grass shaders were built from. */
  tuning: { terrain: TuningTable; grass: TuningTable };
  layers: readonly PresentationLayer[];
  omissions: readonly Omission[];
}

export interface VisualPresentation extends DrawnPresentation {
  schema: "native-visual-presentation";
  identity: {
    scene: string;
    map: string;
    encounter: string;
    seed: number;
    /** sha256 of the saved map's `map.json` bytes. */
    mapHash: string;
    /** sha256 of the capture file's bytes. */
    captureHash: string;
  };
  /** The canvas every frame drew, at one pixel per CSS pixel. */
  viewport: { width: number; height: number; scale: 1 };
  /** The ground's GPU tables, each in its own file (`terrainFile`). */
  terrain: TerrainArtifact[];
  /** The grass clumps each plate's frame grew, per screen cell. */
  grassCounts: GrassCount[];
}

/** The terrain tables, in the manifest's order. */
export const TERRAIN_TABLES = [
  "heights",
  "nodes",
  "plots",
  "surfaces",
  "surfaceIndex",
  "params",
] as const satisfies readonly TerrainTableName[];

/** A terrain table written beside the manifest: `words` little-endian u32
 *  words in `terrainFile(name)`, whose bytes hash to `sha256`, read by `layout`. */
export interface TerrainArtifact {
  name: TerrainTableName;
  sha256: string;
  words: number;
  layout: WordLayout;
}

/** The file a terrain table is written to, beside the manifest. */
export const terrainFile = (name: TerrainTableName) => `terrain-${name}.u32`;

/** The screen grid grass is counted over: columns, rows. */
export const GRASS_CELLS = [4, 3] as const;

/** The grass clumps a plate's frame grew (`grassPass` probes), by the screen
 *  cell their roots project into: row-major from the top-left cell, a cell
 *  being a quarter of the width and a third of the height. */
export interface GrassCount {
  plate: string;
  cells: number[];
}

const IDENTITY_KEYS = ["scene", "map", "encounter", "seed", "mapHash", "captureHash"];
const MANIFEST_KEYS = [
  "schema",
  "identity",
  "viewport",
  "lens",
  "light",
  "tuning",
  "layers",
  "omissions",
  "terrain",
  "grassCounts",
];
const LAYOUT_KEYS: Record<WordLayout["kind"], readonly string[]> = {
  records: ["kind", "stride", "count", "fields"],
  words: ["kind", "count", "constants"],
  "height-pages": ["kind", "nx", "ny", "spacing", "pageSize"],
};

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null && !Array.isArray(value);

const hasKeys = (value: unknown, keys: readonly string[]): value is Record<string, unknown> =>
  isRecord(value) && Object.keys(value).length === keys.length && keys.every((k) => k in value);

const text = (value: unknown) => typeof value === "string" && value.length > 0;
const isHash = (value: unknown) => typeof value === "string" && /^[0-9a-f]{64}$/.test(value);

const finite = (v: unknown) => typeof v === "number" && Number.isFinite(v);
const isTuning = (value: unknown): boolean =>
  isRecord(value) &&
  Object.values(value).every((v) =>
    Array.isArray(v) ? v.length > 0 && v.every(finite) : finite(v) || isTuning(v),
  );

/** `value` as the manifest's light: an object with a unit sun direction.
 *  The page admitted its numbers (`validateLight`) and derived the direction. */
function validateManifestLight(value: unknown): ManifestLight {
  if (!isRecord(value)) throw new Error("visual presentation light is malformed");
  const sun = value.sunDirection;
  if (
    !Array.isArray(sun) ||
    sun.length !== 3 ||
    !sun.every((v) => typeof v === "number" && Number.isFinite(v)) ||
    Math.abs(Math.hypot(...(sun as number[])) - 1) > 1e-6
  )
    throw new Error("visual presentation light needs a unit sunDirection");
  return value as unknown as ManifestLight;
}

const count = (value: unknown) => Number.isInteger(value) && (value as number) >= 0;
const positive = (value: unknown) => typeof value === "number" && value > 0;

/** Whether `value` is a word layout a reader can follow. */
function isLayout(value: unknown): boolean {
  if (!isRecord(value) || !Object.hasOwn(LAYOUT_KEYS, value.kind as string)) return false;
  if (!hasKeys(value, LAYOUT_KEYS[value.kind as WordLayout["kind"]])) return false;
  const v = value as Record<string, unknown>;
  switch (v.kind as WordLayout["kind"]) {
    case "records":
      return (
        positive(v.stride) &&
        count(v.stride) &&
        count(v.count) &&
        Array.isArray(v.fields) &&
        v.fields.every(
          (f) =>
            hasKeys(f, ["name", "word", "type", "count"]) &&
            text(f.name) &&
            count(f.word) &&
            ["f32", "u32", "i32"].includes(f.type as string) &&
            positive(f.count) &&
            count(f.count) &&
            (f.word as number) + (f.count as number) <= (v.stride as number),
        )
      );
    case "words":
      return (
        count(v.count) &&
        isRecord(v.constants) &&
        Object.values(v.constants).every((c) => Number.isInteger(c))
      );
    case "height-pages":
      return [v.nx, v.ny, v.pageSize].every((n) => positive(n) && count(n)) && positive(v.spacing);
  }
}

/** `value` as a manifest, or a refusal naming the part at fault. */
export function validateVisualPresentation(value: unknown): VisualPresentation {
  if (!hasKeys(value, MANIFEST_KEYS)) throw new Error("visual presentation is malformed");
  if (value.schema !== "native-visual-presentation")
    throw new Error(`unsupported visual presentation schema ${JSON.stringify(value.schema)}`);
  const id = value.identity;
  if (
    !hasKeys(id, IDENTITY_KEYS) ||
    ![id.scene, id.map, id.encounter].every(text) ||
    !Number.isInteger(id.seed) ||
    !isHash(id.mapHash) ||
    !isHash(id.captureHash)
  )
    throw new Error("visual presentation identity is malformed");
  const view = value.viewport;
  if (
    !hasKeys(view, ["width", "height", "scale"]) ||
    !Number.isInteger(view.width) ||
    !Number.isInteger(view.height) ||
    (view.width as number) <= 0 ||
    (view.height as number) <= 0 ||
    view.scale !== 1
  )
    throw new Error("visual presentation viewport is malformed");
  const lens = value.lens;
  if (
    !hasKeys(lens, ["fovY", "near", "depth", "far"]) ||
    !(typeof lens.fovY === "number" && lens.fovY > 0 && lens.fovY < Math.PI) ||
    !(typeof lens.near === "number" && lens.near > 0) ||
    lens.depth !== "reverse-z" ||
    lens.far !== "infinite"
  )
    throw new Error("visual presentation lens is malformed");
  validateManifestLight(value.light);
  const tuning = value.tuning;
  if (
    !hasKeys(tuning, ["terrain", "grass"]) ||
    !isTuning(tuning.terrain) ||
    !isTuning(tuning.grass)
  )
    throw new Error("visual presentation tuning needs terrain and grass tables of finite numbers");
  const layers = value.layers;
  if (
    !Array.isArray(layers) ||
    !layers.every((l) => (PRESENTATION_LAYERS as readonly unknown[]).includes(l)) ||
    new Set(layers).size !== layers.length
  )
    throw new Error("visual presentation layers must be distinct known layer names");
  const omissions = value.omissions;
  if (
    !Array.isArray(omissions) ||
    !omissions.every(
      (o) =>
        hasKeys(o, ["owner", "reason", "sourceId"]) && [o.owner, o.reason, o.sourceId].every(text),
    )
  )
    throw new Error("visual presentation omission rows need an owner, reason and sourceId");
  const terrain = value.terrain;
  if (
    !Array.isArray(terrain) ||
    terrain.length !== TERRAIN_TABLES.length ||
    !terrain.every(
      (t, i) =>
        hasKeys(t, ["name", "sha256", "words", "layout"]) &&
        t.name === TERRAIN_TABLES[i] &&
        isHash(t.sha256) &&
        count(t.words) &&
        isLayout(t.layout),
    )
  )
    throw new Error(
      `visual presentation terrain must list ${TERRAIN_TABLES.join(", ")}, each with its sha256, words and layout`,
    );
  const grass = value.grassCounts;
  if (
    !Array.isArray(grass) ||
    !grass.every(
      (g) =>
        hasKeys(g, ["plate", "cells"]) &&
        text(g.plate) &&
        Array.isArray(g.cells) &&
        g.cells.length === GRASS_CELLS[0] * GRASS_CELLS[1] &&
        g.cells.every(count),
    ) ||
    new Set(grass.map((g) => g.plate)).size !== grass.length
  )
    throw new Error(
      `visual presentation grass counts need one row of ${GRASS_CELLS[0] * GRASS_CELLS[1]} whole counts per plate`,
    );
  return value as unknown as VisualPresentation;
}

async function sha256(bytes: Uint8Array): Promise<string> {
  const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", bytes as BufferSource));
  return Array.from(digest, (b) => b.toString(16).padStart(2, "0")).join("");
}

/** `words` as little-endian bytes. */
function littleEndian(words: Uint32Array): Uint8Array {
  const bytes = new Uint8Array(words.length * 4);
  const view = new DataView(bytes.buffer);
  words.forEach((w, i) => view.setUint32(i * 4, w, true));
  return bytes;
}

/** The words of the artifact `entry` names, from its file's `bytes`; refused
 *  unless the bytes are the ones the manifest hashed. */
export async function readTerrainArtifact(
  entry: TerrainArtifact,
  bytes: Uint8Array,
): Promise<Uint32Array> {
  if (bytes.length !== entry.words * 4)
    throw new Error(
      `${terrainFile(entry.name)} holds ${bytes.length / 4} words, not ${entry.words}`,
    );
  if ((await sha256(bytes)) !== entry.sha256)
    throw new Error(`${terrainFile(entry.name)} does not match its sha256`);
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  return Uint32Array.from({ length: entry.words }, (_, i) => view.getUint32(i * 4, true));
}

/** The manifest JSON for a capture file's text `captureJson` drawn on the
 *  map whose `map.json` is `mapBytes`, and the artifact files beside it: the
 *  identity is the capture's workload and both files' hashes, the viewport
 *  the canvas every frame drew. `terrain` holds the ground's tables as the
 *  page uploaded them; `grassCounts` one row per plate the capture names. */
export async function writeVisualPresentation({
  captureJson,
  mapBytes,
  drawn,
  terrain,
  grassCounts,
}: {
  captureJson: string;
  mapBytes: Uint8Array;
  drawn: DrawnPresentation;
  terrain: TerrainTables;
  grassCounts: readonly GrassCount[];
}): Promise<{ manifest: string; artifacts: { file: string; bytes: Uint8Array }[] }> {
  const capture = decodePresentationCapture(captureJson);
  const [first] = capture.frames;
  if (!first) throw new Error("a capture with no frame has no viewport");
  const [width, height] = first.canvas;
  const resized = capture.frames.find((f) => f.canvas[0] !== width || f.canvas[1] !== height);
  if (resized)
    throw new Error(
      `the canvas changed from ${width}×${height} to ${resized.canvas.join("×")} at frame ${resized.index}`,
    );
  const plates = capture.frames.flatMap((f) => (f.plate === null ? [] : [f.plate]));
  const counted = grassCounts.map((g) => g.plate);
  const uncounted = plates.filter((p) => !counted.includes(p));
  const stray = counted.filter((p) => !plates.includes(p));
  if (uncounted.length || stray.length)
    throw new Error(
      `grass counts must be one per plate: none for ${uncounted.join(", ") || "-"}; no plate ${stray.join(", ") || "-"}`,
    );
  const unknown = Object.keys(terrain).filter(
    (name) => !(TERRAIN_TABLES as readonly string[]).includes(name),
  );
  if (unknown.length) throw new Error(`unknown terrain tables ${unknown.join(", ")}`);
  const artifacts = TERRAIN_TABLES.map((name) => ({
    name,
    file: terrainFile(name),
    bytes: littleEndian(terrain[name].words),
  }));
  const { scene, map, encounter, seed } = capture.workload;
  const manifest: VisualPresentation = {
    schema: "native-visual-presentation",
    identity: {
      scene,
      map,
      encounter,
      seed,
      mapHash: await sha256(mapBytes),
      captureHash: await sha256(new TextEncoder().encode(captureJson)),
    },
    viewport: { width, height, scale: 1 },
    lens: drawn.lens,
    light: drawn.light,
    tuning: drawn.tuning,
    layers: drawn.layers,
    omissions: drawn.omissions,
    terrain: await Promise.all(
      artifacts.map(async ({ name, bytes }) => ({
        name,
        sha256: await sha256(bytes),
        words: bytes.length / 4,
        layout: terrain[name].layout,
      })),
    ),
    grassCounts: plates.map((plate) => grassCounts.find((g) => g.plate === plate)!),
  };
  return {
    manifest: JSON.stringify(validateVisualPresentation(manifest)),
    artifacts: artifacts.map(({ file, bytes }) => ({ file, bytes })),
  };
}
