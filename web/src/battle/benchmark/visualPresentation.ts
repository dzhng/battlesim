// The `native-visual-presentation` manifest: what the browser drew for one
// captured scene, beside its `battle-presentation-capture`, so a second client
// draws the same battle through the same lens and the same layers. The
// browser run is its one writer; it grows only by what native reads, and
// changes in place (a stale file is refused, never migrated).
// Plain Node runs the writer (the scene harness), so it imports nothing the
// renderer owns. What the battle is (its units, effects, light and
// buildings) a second client reads from the repository itself.
import type { DrawnInstances } from "@web/battle/render/sceneStats";
import { decodePresentationCapture, finite, hasKeys, isRecord } from "./presentationCapture.ts";

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

/** What the page knows of its own drawing; the files give the rest. */
export interface DrawnPresentation {
  lens: Lens;
  layers: readonly PresentationLayer[];
}

/** What a plate's frame posed, by kind (`modelCounts`): living and dying
 *  soldiers, ground vehicles, aircraft, lying corpses and corpses sinking
 *  away past the cap. */
export interface ModelCount {
  plate: string;
  soldiers: number;
  vehicles: number;
  aircraft: number;
  corpses: number;
  fading: number;
}

export const MODEL_COUNT_KINDS = ["soldiers", "vehicles", "aircraft", "corpses", "fading"] as const;

/** What a plate's frame drew of the effects (`Frame.counts`): those
 *  running at its clock by kind, its instances by shape, and its lights. */
export interface EffectCount {
  plate: string;
  running: Record<string, number>;
  instances: Record<string, number>;
  lights: number;
}

/** What one frame drew, counted (`?counts=1`): the frame by its index in
 *  the capture, its models and effects as a plate's are counted, and its
 *  building modules, scenery and props, as a plate's are counted but with
 *  no read-back: a counting run counts frames on a tick boundary and between. */
export interface FrameCount {
  frame: number;
  models: Omit<ModelCount, "plate">;
  effects: Omit<EffectCount, "plate">;
  drawn: DrawnInstances;
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
  /** The grass clumps each plate's frame grew, per screen cell. */
  grassCounts: GrassCount[];
  /** The building modules and scenery instances each plate's frame drew. */
  instanceCounts: InstanceCount[];
  /** The soldiers, vehicles and corpses each plate's frame posed. */
  modelCounts: ModelCount[];
  /** The combat effects each plate's frame drew. */
  effectCounts: EffectCount[];
  /** What each counted frame drew, in frame order; none unless counted. */
  frameCounts: FrameCount[];
}

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
 *  (the shared renderer's drawn-instance snapshot): per kit module the buildings' walk draws
 *  and tier, `module · TIERS + tier`; per population, scenery appearance (the
 *  ground's `scenery_kinds`) and tier likewise; the stumps per appearance
 *  alone; the props per appearance, at each tier. */
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
  "layers",
  "grassCounts",
  "instanceCounts",
  "modelCounts",
  "effectCounts",
  "frameCounts",
];
const text = (value: unknown) => typeof value === "string" && value.length > 0;
const isHash = (value: unknown) => typeof value === "string" && /^[0-9a-f]{64}$/.test(value);

const count = (value: unknown) => Number.isInteger(value) && (value as number) >= 0;
const counts = (value: unknown) => isRecord(value) && Object.values(value).every(count);

/** Whether `value` counts a frame's models by kind. */
const isModels = (value: unknown): boolean =>
  hasKeys(value, MODEL_COUNT_KINDS) && MODEL_COUNT_KINDS.every((k) => count(value[k]));

/** Whether `value` counts a frame's effects: by kind, by shape, and its lights. */
const isEffects = (value: unknown): boolean =>
  hasKeys(value, ["running", "instances", "lights"]) &&
  counts(value.running) &&
  counts(value.instances) &&
  count(value.lights);

const positive = (value: unknown) => finite(value) && value > 0;
const numbers = (value: unknown, n: number, valid: (v: unknown) => boolean) =>
  Array.isArray(value) && value.length === n && value.every(valid);
const distinct = (values: readonly unknown[]) =>
  new Set(values.map((v) => JSON.stringify(v))).size === values.length;

/** Whether `value` counts what a frame drew of the kit modules, scenery
 *  appearances and props (a count per module or appearance and tier). */
function isDrawn(value: unknown): boolean {
  if (!hasKeys(value, ["buildings", "scenery", "props"])) return false;
  const { scenery, props } = value;
  return (
    Array.isArray(props) &&
    props.every(
      (p) =>
        hasKeys(p, ["appearance", "levels"]) &&
        text(p.appearance) &&
        numbers(p.levels, TIERS, count),
    ) &&
    distinct(props.map((p) => p.appearance)) &&
    Array.isArray(value.buildings) &&
    value.buildings.length % TIERS === 0 &&
    value.buildings.every(count) &&
    hasKeys(scenery, [...SCENERY_POPULATIONS, "stumps"]) &&
    Array.isArray(scenery.stumps) &&
    SCENERY_POPULATIONS.every((p) =>
      numbers(scenery[p], (scenery.stumps as unknown[]).length * TIERS, count),
    ) &&
    scenery.stumps.every(count)
  );
}

/** A plate's row of counts: its plate's file and the counts `valid` checks. */
const plated =
  (valid: (value: unknown) => boolean) =>
  (value: unknown): boolean => {
    if (!isRecord(value) || !text(value.plate)) return false;
    const { plate: _, ...body } = value;
    return valid(body);
  };

const isModelCount = plated(isModels);
const isEffectCount = plated(isEffects);
const isInstanceCount = plated(isDrawn);

const isFrameCount = (value: unknown): boolean =>
  hasKeys(value, ["frame", "models", "effects", "drawn"]) &&
  count(value.frame) &&
  isModels(value.models) &&
  isEffects(value.effects) &&
  isDrawn(value.drawn);

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
  const layers = value.layers;
  if (
    !Array.isArray(layers) ||
    !layers.every((l) => (PRESENTATION_LAYERS as readonly unknown[]).includes(l)) ||
    new Set(layers).size !== layers.length
  )
    throw new Error("visual presentation layers must be distinct known layer names");
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
  if (
    !Array.isArray(instances) ||
    !instances.every(isInstanceCount) ||
    !distinct(instances.map((row) => row.plate))
  )
    throw new Error(
      "visual presentation instance counts need one row per plate, a count per kit module and tier, per scenery population, appearance and tier, and per prop appearance and tier",
    );
  const models = value.modelCounts;
  if (
    !Array.isArray(models) ||
    !models.every(isModelCount) ||
    !distinct(models.map((row) => row.plate))
  )
    throw new Error(
      "visual presentation model counts need one row per plate of whole soldiers, vehicles, aircraft, corpses and fading corpses",
    );
  const effects = value.effectCounts;
  if (
    !Array.isArray(effects) ||
    !effects.every(isEffectCount) ||
    !distinct(effects.map((row) => row.plate))
  )
    throw new Error(
      "visual presentation effect counts need one row per plate of whole effects by kind, instances by shape and lights",
    );
  const frames = value.frameCounts;
  if (
    !Array.isArray(frames) ||
    !frames.every(isFrameCount) ||
    !frames.every((row, i) => i === 0 || row.frame > frames[i - 1].frame)
  )
    throw new Error(
      "visual presentation frame counts need one row per counted frame, in frame order, of its models, effects and drawn instances",
    );
  return value as unknown as VisualPresentation;
}

async function sha256(bytes: Uint8Array): Promise<string> {
  const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", bytes as BufferSource));
  return Array.from(digest, (b) => b.toString(16).padStart(2, "0")).join("");
}

/** The manifest JSON for a capture file's text `captureJson` drawn on the
 *  map whose `map.json` is `mapBytes`: the
 *  identity is the capture's workload and both files' hashes, the viewport
 *  the canvas every frame drew; `grassCounts` one row per plate the capture
 *  names. */
export async function writeVisualPresentation({
  captureJson,
  mapBytes,
  drawn,
  grassCounts,
  instanceCounts,
  modelCounts,
  effectCounts,
  frameCounts = [],
}: {
  captureJson: string;
  mapBytes: Uint8Array;
  drawn: DrawnPresentation;
  grassCounts: readonly GrassCount[];
  instanceCounts: readonly InstanceCount[];
  modelCounts: readonly ModelCount[];
  effectCounts: readonly EffectCount[];
  frameCounts?: readonly FrameCount[];
}): Promise<string> {
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
  const stray = frameCounts.find((c) => c.frame >= capture.frames.length);
  if (stray)
    throw new Error(
      `frame counts name frame ${stray.frame}; the capture has ${capture.frames.length}`,
    );
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
    layers: drawn.layers,
    grassCounts: plates.map((plate) => grassCounts.find((g) => g.plate === plate)!),
    instanceCounts: plates.map((plate) => instanceCounts.find((g) => g.plate === plate)!),
    modelCounts: plates.map((plate) => modelCounts.find((g) => g.plate === plate)!),
    effectCounts: plates.map((plate) => effectCounts.find((g) => g.plate === plate)!),
    frameCounts: [...frameCounts],
  };
  return JSON.stringify(validateVisualPresentation(manifest));
}
