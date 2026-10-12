// The `native-visual-presentation` manifest: what the browser drew for one
// captured scene, beside its `battle-presentation-capture`, so a second client
// draws the same battle through the same lens and the same layers. The
// browser run is its one writer; it grows only by what native reads, and
// changes in place (a stale file is refused, never migrated).
// Plain Node runs the writer (the scene harness), so it imports nothing the
// renderer owns; the page validates and derives the light before handing it over.
import type { LightPresentation } from "@packages/battle-renderer/src/light/sceneLight";
import type { TerrainTableName } from "@packages/battle-renderer/src/frame/terrainTables";
import type {
  GrassConstants,
  GrassTableName,
} from "@packages/battle-renderer/src/frame/grassTables";
import type {
  UploadedTables,
  WordLayout,
  WordTable,
} from "@packages/battle-renderer/src/frame/wordTables";
import type {
  SceneryTableName,
  SceneryTables,
  SceneryTiers,
} from "@packages/battle-renderer/src/scenery/sceneryTables";
import type { DrawnInstances } from "@packages/battle-renderer/src/scene";
import { decodePresentationCapture, finite, hasKeys, isRecord } from "./presentationCapture.ts";
import {
  isEffectCount,
  validateEffects,
  type EffectCount,
  type EffectsManifest,
} from "./effectManifest.ts";
import {
  propArtifacts,
  validateProps,
  type DrawnProps,
  type PropsManifest,
} from "./propManifest.ts";
import {
  isModelCount,
  unitArtifacts,
  validateUnits,
  type DrawnUnits,
  type ModelCount,
  type UnitsManifest,
} from "./unitManifest.ts";

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

/** Nested tuning numbers: `terrainTuning` or `grassTuning` as the shaders read them. */
export interface TuningTable {
  readonly [key: string]: number | readonly number[] | TuningTable;
}

/** What the page knows of its own drawing; the files give the rest. */
export interface DrawnPresentation {
  lens: Lens;
  light: ManifestLight;
  /** The tuning tables the terrain and grass shaders were built from. */
  tuning: { terrain: TuningTable; grass: TuningTable };
  layers: readonly PresentationLayer[];
  omissions: readonly Omission[];
  /** What poses and draws the battle's units (`unitManifest.ts`). */
  units: DrawnUnits;
  /** What draws the side's props and fallen buildings (`propManifest.ts`). */
  props: DrawnProps;
  /** What turns the side's publications into combat effects (`effectManifest.ts`). */
  effects: EffectsManifest;
}

export interface VisualPresentation extends Omit<DrawnPresentation, "units" | "props"> {
  /** The props, the map's fitted models in their own file (`PROP_TABLE_FILE`). */
  props: PropsManifest;
  /** The units, their clip tables each in its own file (`unitTableFile`). */
  units: UnitsManifest;
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
  /** The ground's GPU tables, each in its own file (`tableFile`). */
  terrain: TableArtifact<TerrainTableName>[];
  /** The grass pass's GPU tables, the numbers its draws read from uniforms,
   *  and the grass kinds it draws, in kind order. */
  grass: {
    tables: TableArtifact<GrassTableName>[];
    constants: GrassConstants;
    kinds: string[];
  };
  /** Every tree, shrub and piece of dressing, the appearances they name,
   *  and the tier rule's numbers. */
  scenery: {
    tables: TableArtifact<SceneryTableName>[];
    kinds: string[];
    tiers: SceneryTiers;
    felled: SceneryTables["felled"];
  };
  /** The grass clumps each plate's frame grew, per screen cell. */
  grassCounts: GrassCount[];
  /** The building modules and scenery instances each plate's frame drew. */
  instanceCounts: InstanceCount[];
  /** The soldiers, vehicles and corpses each plate's frame posed. */
  modelCounts: ModelCount[];
  /** The combat effects each plate's frame drew. */
  effectCounts: EffectCount[];
}

/** Each group's tables, in the manifest's order. */
export const TABLES = {
  terrain: [
    "heights",
    "nodes",
    "plots",
    "surfaces",
    "surfaceIndex",
    "params",
    "scars",
    "water",
  ] as const satisfies readonly TerrainTableName[],
  grass: [
    "params",
    "props",
    "shapes",
    "indicesNear",
    "indicesFar",
  ] as const satisfies readonly GrassTableName[],
  scenery: [
    "forest",
    "understorey",
    "backdrop",
    "dressing",
    "dressingCells",
  ] as const satisfies readonly SceneryTableName[],
};
export type TableGroup = keyof typeof TABLES;

/** Each grass constant and whether it is a count or a length (metres or a
 *  scale); a list holds a pair. */
const GRASS_CONSTANTS = {
  tileM: "count",
  segments: "count",
  bladeVertices: "count",
  capacity: "count",
  firstClump: "count",
  indexCount: "count",
  maxBlades: "count",
  edgeM: "length",
  heightVariation: "length",
} as const satisfies Record<keyof GrassConstants, "count" | "length">;

/** A table written beside the manifest: `words` little-endian u32 words in
 *  `tableFile(group, name)`, whose bytes hash to `sha256`, read by `layout`. */
export interface TableArtifact<Name extends string = string> {
  name: Name;
  sha256: string;
  words: number;
  layout: WordLayout;
}

/** The file a group's table is written to, beside the manifest. */
const tableFile = (group: TableGroup, name: string) => `${group}-${name}.u32`;

/** The screen grid grass is counted over: columns, rows. */
export const GRASS_CELLS = [4, 3] as const;

/** The grass clumps a plate's frame grew (`grassPass` probes), by the screen
 *  cell their roots project into: row-major from the top-left cell, a cell
 *  being a quarter of the width and a third of the height. */
export interface GrassCount {
  plate: string;
  cells: number[];
}

/** The detail tiers every building module and scenery appearance has
 *  (`scene-assets` `TIER_COUNT`). */
export const TIERS = 4;

/** What a plate's frame drew of the buildings, the scenery and the props
 *  its side knows (`BattleFrame.drawnInstances`): per kit module Rust
 *  resolved (`crates/presentation`'s static world) and tier, `module · TIERS + tier`; per population,
 *  appearance of `scenery.kinds` and tier likewise; the stumps per
 *  appearance alone; the known props per appearance, at each tier and then
 *  as a card. */
export interface InstanceCount extends DrawnInstances {
  plate: string;
}

const SCENERY_POPULATIONS = ["forest", "understorey", "backdrop", "dressing", "felled"] as const;

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
  "grass",
  "scenery",
  "grassCounts",
  "instanceCounts",
  "units",
  "modelCounts",
  "props",
  "effects",
  "effectCounts",
];
const LAYOUT_KEYS: Record<WordLayout["kind"], readonly string[]> = {
  records: ["kind", "stride", "count", "fields"],
  words: ["kind", "count", "constants"],
  u16: ["kind", "count"],
  "height-pages": ["kind", "nx", "ny", "spacing", "pageSize"],
  "prop-cells": ["kind", "count"],
};

const text = (value: unknown) => typeof value === "string" && value.length > 0;
const isHash = (value: unknown) => typeof value === "string" && /^[0-9a-f]{64}$/.test(value);

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
    !sun.every(finite) ||
    Math.abs(Math.hypot(...(sun as number[])) - 1) > 1e-6
  )
    throw new Error("visual presentation light needs a unit sunDirection");
  return value as unknown as ManifestLight;
}

const count = (value: unknown) => Number.isInteger(value) && (value as number) >= 0;
const nonNegative = (value: unknown) => finite(value) && value >= 0;
const positive = (value: unknown) => finite(value) && value > 0;

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
    case "u16":
    case "prop-cells":
      return count(v.count);
    case "height-pages":
      return [v.nx, v.ny, v.pageSize].every((n) => positive(n) && count(n)) && positive(v.spacing);
  }
}

/** Whether `value` lists exactly `names`' tables, in order, each hashed and laid out. */
const isTableList = (value: unknown, names: readonly string[]) =>
  Array.isArray(value) &&
  value.length === names.length &&
  value.every(
    (t, i) =>
      hasKeys(t, ["name", "sha256", "words", "layout"]) &&
      t.name === names[i] &&
      isHash(t.sha256) &&
      count(t.words) &&
      isLayout(t.layout),
  );

const numbers = (value: unknown, n: number, valid: (v: unknown) => boolean) =>
  Array.isArray(value) && value.length === n && value.every(valid);
const distinct = (values: readonly unknown[]) =>
  new Set(values.map((v) => JSON.stringify(v))).size === values.length;

/** `biome.trees.felled`, as a felled tree reads it (`felled.ts`). */
const FELLED_KEYS = [
  "fall_s",
  "settle_s",
  "settle_deg",
  "rest_height_m",
  "rest_spread",
  "stump_height_m",
  "cut",
];

/** Whether `value` is a scenery section: its tables, its distinct
 *  appearances and its tier numbers. */
function isScenery(value: unknown): boolean {
  if (!hasKeys(value, ["tables", "kinds", "tiers", "felled"])) return false;
  const { tables, kinds, tiers, felled } = value;
  if (!hasKeys(tiers, ["lodPx", "chunkM", "casterCoarser", "dressing"])) return false;
  const dressing = tiers.dressing;
  return (
    isTableList(tables, TABLES.scenery) &&
    Array.isArray(kinds) &&
    kinds.length > 0 &&
    kinds.every(text) &&
    distinct(kinds) &&
    numbers(tiers.lodPx, 3, positive) &&
    positive(tiers.chunkM) &&
    count(tiers.casterCoarser) &&
    hasKeys(dressing, [
      "lodPx",
      "fadePx",
      "gone",
      "cellM",
      "topM",
      "reachM",
      "ground",
      "slots",
      "cellsPerView",
    ]) &&
    numbers(dressing.lodPx, 3, positive) &&
    [dressing.fadePx, dressing.gone, dressing.cellM].every(positive) &&
    [dressing.topM, dressing.reachM].every(nonNegative) &&
    numbers(dressing.ground, 2, finite) &&
    [dressing.slots, dressing.cellsPerView].every(count) &&
    hasKeys(felled, FELLED_KEYS) &&
    FELLED_KEYS.slice(0, -1).every((key) => finite(felled[key]) && felled[key] >= 0) &&
    positive(felled.fall_s) &&
    positive(felled.rest_spread) &&
    numbers(felled.cut, 3, finite)
  );
}

/** Whether `value` counts what a plate drew of the kit modules Rust resolved
 *  (a count per module and tier) and `kinds` scenery appearances. */
function isInstanceCount(value: unknown, kinds: number): boolean {
  if (!hasKeys(value, ["plate", "buildings", "scenery", "knownProps"]) || !text(value.plate))
    return false;
  const { scenery, knownProps } = value;
  return (
    Array.isArray(knownProps) &&
    knownProps.every(
      (p) =>
        hasKeys(p, ["appearance", "levels"]) &&
        text(p.appearance) &&
        numbers(p.levels, TIERS + 1, count),
    ) &&
    distinct(knownProps.map((p) => p.appearance)) &&
    Array.isArray(value.buildings) &&
    value.buildings.length % TIERS === 0 &&
    value.buildings.every(count) &&
    hasKeys(scenery, [...SCENERY_POPULATIONS, "stumps"]) &&
    SCENERY_POPULATIONS.every((p) => numbers(scenery[p], kinds * TIERS, count)) &&
    numbers(scenery.stumps, kinds, count)
  );
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
    !(positive(lens.fovY) && (lens.fovY as number) < Math.PI) ||
    !positive(lens.near) ||
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
  if (!isTableList(value.terrain, TABLES.terrain))
    throw new Error(
      `visual presentation terrain must list ${TABLES.terrain.join(", ")}, each with its sha256, words and layout`,
    );
  const field = value.grass;
  if (
    !hasKeys(field, ["tables", "constants", "kinds"]) ||
    !isTableList(field.tables, TABLES.grass) ||
    !hasKeys(field.constants, Object.keys(GRASS_CONSTANTS)) ||
    !Object.entries(GRASS_CONSTANTS).every(([key, kind]) => {
      const valid = kind === "count" ? count : nonNegative;
      const c = (field.constants as Record<string, unknown>)[key];
      return Array.isArray(c) ? c.length === 2 && c.every(valid) : valid(c);
    }) ||
    !Array.isArray(field.kinds) ||
    field.kinds.length === 0 ||
    !field.kinds.every(text) ||
    new Set(field.kinds).size !== field.kinds.length
  )
    throw new Error(
      `visual presentation grass needs tables ${TABLES.grass.join(", ")}, its constants and its distinct kinds`,
    );
  if (!isScenery(value.scenery))
    throw new Error(
      `visual presentation scenery needs tables ${TABLES.scenery.join(", ")}, distinct appearances and their tier numbers`,
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
  const instances = value.instanceCounts;
  const kinds = (value.scenery as { kinds: unknown[] }).kinds.length;
  if (
    !Array.isArray(instances) ||
    !instances.every((row) => isInstanceCount(row, kinds)) ||
    !distinct(instances.map((row) => row.plate))
  )
    throw new Error(
      "visual presentation instance counts need one row per plate, a count per kit module and tier, per scenery population, appearance and tier, and per known prop appearance and level",
    );
  validateUnits(value.units);
  validateProps(value.props);
  const models = value.modelCounts;
  if (
    !Array.isArray(models) ||
    !models.every(isModelCount) ||
    !distinct(models.map((row) => row.plate))
  )
    throw new Error(
      "visual presentation model counts need one row per plate of whole soldiers, vehicles, aircraft, corpses and fading corpses",
    );
  validateEffects(value.effects);
  const effects = value.effectCounts;
  if (
    !Array.isArray(effects) ||
    !effects.every(isEffectCount) ||
    !distinct(effects.map((row) => row.plate))
  )
    throw new Error(
      "visual presentation effect counts need one row per plate of whole effects by kind, instances by shape and lights",
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

/** The manifest JSON for a capture file's text `captureJson` drawn on the
 *  map whose `map.json` is `mapBytes`, and the artifact files beside it: the
 *  identity is the capture's workload and both files' hashes, the viewport
 *  the canvas every frame drew. `tables` holds the ground's and the grass's
 *  tables as the page uploaded them; `grassCounts` one row per plate the
 *  capture names. */
export async function writeVisualPresentation({
  captureJson,
  mapBytes,
  drawn,
  tables,
  grassCounts,
  instanceCounts,
  modelCounts,
  effectCounts,
}: {
  captureJson: string;
  mapBytes: Uint8Array;
  drawn: DrawnPresentation;
  tables: UploadedTables;
  grassCounts: readonly GrassCount[];
  instanceCounts: readonly InstanceCount[];
  modelCounts: readonly ModelCount[];
  effectCounts: readonly EffectCount[];
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
  for (const [what, rows] of [
    ["grass", grassCounts],
    ["instance", instanceCounts],
    ["model", modelCounts],
    ["effect", effectCounts],
  ] as const) {
    const counted = rows.map((g) => g.plate);
    const uncounted = plates.filter((p) => !counted.includes(p));
    const stray = counted.filter((p) => !plates.includes(p));
    if (uncounted.length || stray.length)
      throw new Error(
        `${what} counts must be one per plate: none for ${uncounted.join(", ") || "-"}; no plate ${stray.join(", ") || "-"}`,
      );
  }
  const groups: Record<TableGroup, Record<string, WordTable>> = {
    terrain: tables.terrain,
    grass: tables.grass.tables,
    scenery: tables.scenery.tables,
  };
  const files = (group: TableGroup) => {
    const names: readonly string[] = TABLES[group];
    const unknown = Object.keys(groups[group]).filter((name) => !names.includes(name));
    if (unknown.length) throw new Error(`unknown ${group} tables ${unknown.join(", ")}`);
    return names.map((name) => ({
      name,
      file: tableFile(group, name),
      bytes: littleEndian(groups[group][name].words),
      layout: groups[group][name].layout,
    }));
  };
  const terrain = files("terrain");
  const grass = files("grass");
  const scenery = files("scenery");
  const entries = <Name extends string>(list: ReturnType<typeof files>) =>
    Promise.all(
      list.map(async ({ name, bytes, layout }) => ({
        name: name as Name,
        sha256: await sha256(bytes),
        words: bytes.length / 4,
        layout,
      })),
    );
  const units = await unitArtifacts(drawn.units, sha256);
  const props = await propArtifacts(drawn.props, sha256);
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
    terrain: await entries<TerrainTableName>(terrain),
    grass: {
      tables: await entries<GrassTableName>(grass),
      constants: tables.grass.constants,
      kinds: tables.grass.kinds,
    },
    scenery: {
      tables: await entries<SceneryTableName>(scenery),
      kinds: tables.scenery.kinds,
      tiers: tables.scenery.tiers,
      felled: tables.scenery.felled,
    },
    grassCounts: plates.map((plate) => grassCounts.find((g) => g.plate === plate)!),
    instanceCounts: plates.map((plate) => instanceCounts.find((g) => g.plate === plate)!),
    units: units.units,
    props: props.props,
    modelCounts: plates.map((plate) => modelCounts.find((g) => g.plate === plate)!),
    effects: drawn.effects,
    effectCounts: plates.map((plate) => effectCounts.find((g) => g.plate === plate)!),
  };
  return {
    manifest: JSON.stringify(validateVisualPresentation(manifest)),
    artifacts: [
      ...[...terrain, ...grass, ...scenery].map(({ file, bytes }) => ({
        file,
        bytes,
      })),
      ...props.artifacts,
      ...units.artifacts,
    ],
  };
}
