// Buildings drawn from template art, at a map's scale: which module
// instances a view draws, and from where. Pure bookkeeping; the models layer
// (`buildingLayer.ts`) holds the buffers and draws the modules' meshes.
//
// A building is its template's rows (`scene-assets` `templateLibrary.ts`):
// each a kit module placed on the template, with the detail tiers it draws
// at. A map holds up to 16,000 buildings and a template up to 700 rows, so
// rows are never expanded for a whole map. Instead:
//
// - **The coarse population** is every building's rows at the coarsest tier
//   (one to a few a building), expanded once for the map: a population of the
//   static chunk owner (`frame/staticChunks.ts`), a kind a module. The whole
//   map draws from it at the overview, as a range a kind, with no work.
// - **A chunk near enough for a finer tier is resident**: its buildings' rows
//   at that tier are expanded once, when it enters, into a fixed pool
//   (`placementPool.ts`), and leave it when the chunk leaves the view or the
//   tier. While it is resident its coarse records are hidden in place (their
//   scale is zero), so the coarse ranges never split round it. A chunk the
//   pool has no room for, or has not expanded yet, draws coarse meanwhile.
//   The pool is kept kind by kind (a kind is a module at a tier), so it
//   draws in one range a kind.
// - **A building the side has seen fall** leaves both (its coarse records are
//   hidden, its chunk expanded again without it) and is drawn by the fallen
//   population: its template's rows for that state where the library has
//   them, else each part's remains as a box. That population is small, so
//   all its tiers are expanded and it is rebuilt when knowledge changes.
//
// A chunk takes one tier, from the distance of its nearest point: a building
// is in exactly one chunk (it is bucketed by where it was placed), so it is
// never drawn at two tiers. A row draws at a tier only if its mask has it.
//
// So a view's draws are a range or two a module from the coarse buffer and
// one a module and tier from the pool: their number follows the kit, not the
// map or the camera.
import { color } from "math/color";
import { mulberry32, random } from "math/random";
import { vec3 } from "math";
import { box3 } from "math/shapes";
import { PROTOTYPE_KIT, PROTOTYPE_MODULE } from "@packages/scene-assets/src/prototypeSet";
import { TIER_COUNT, type Bounds } from "@packages/scene-assets/src/schema";
import {
  placeRows,
  ROW_TRANSFORM_FLOATS,
  TemplateArtError,
  templateKits,
  templateRows,
  type RowRange,
  type TemplateArtLibrary,
} from "@packages/scene-assets/src/templateLibrary";
import type { DetailView } from "../frame/detailView";
import {
  createStaticChunks,
  selectChunks,
  type StaticChunks,
  type SunShadow,
} from "../frame/staticChunks";
import {
  FRAME_FLOATS,
  type BuildingStyle,
  type FallenBuilding,
  type PlacedBuildings,
} from "./buildingReferences";
import { MODEL_RECORD_FLOATS } from "./modelInstances";
import { createPlacementPool, poolAppend, poolRemove, type PlacementPool } from "./placementPool";

const RECORD = MODEL_RECORD_FLOATS;
/** The tier the whole map can draw at. */
const COARSEST = TIER_COUNT - 1;
/** A resident chunk's casters draw this many tiers coarser than the view,
 *  as every model's do. */
const CASTER_COARSER = 1;
/** Two coarse ranges of a kind this few records apart draw as one. The
 *  records between belong to chunks out of view: drawing them costs their
 *  vertices, which is less than the draw it saves. */
const BRIDGE_RECORDS = 256;

/** The template art a scene draws from. */
export interface BuildingArt {
  library: TemplateArtLibrary;
  /** Per library module: its mesh's bounds in its own frame, or null for a
   *  module of a kit that is not installed (`buildingKits` names the kits a
   *  map's buildings need). */
  bounds: readonly (Bounds | null)[];
}

/** The kit appearances a scene of `placed` draws modules of: its templates'
 *  own, in every state, and the prototype kit, whose box a fallen part with
 *  no art for its state is drawn as. */
export function buildingKits(placed: PlacedBuildings, library: TemplateArtLibrary): Set<string> {
  return templateKits(library, placed.templates).add(PROTOTYPE_KIT);
}

/** Whether every kit `placed` needs is installed in `art`. The buildings and
 *  the kits that draw them arrive apart; a scene is built once both have. */
export function artCovers(placed: PlacedBuildings, art: BuildingArt): boolean {
  const { kits, modules } = art.library;
  const needed = buildingKits(placed, art.library);
  return modules.every((m, i) => art.bounds[i] !== null || !needed.has(kits[m.kit].appearance));
}

/** A template's intact rows, sorted for expansion. */
interface TemplateRows {
  /** Per tier: the runs of library rows drawn at it, as `first, count` pairs. */
  runs: Uint32Array[];
  /** Per tier: how many rows that is. */
  counts: number[];
  /** Everything its intact rows draw, in the template's frame. */
  min: [number, number, number];
  max: [number, number, number];
}

/** Rows expanded into records and chunked: a population of the chunk owner
 *  whose kinds are modules, each with the tiers its rows draw at. */
export interface RowPopulation {
  chunks: StaticChunks;
  /** Per kind: the library module it draws, and its rows' tier mask. */
  module: Uint16Array;
  mask: Uint8Array;
  /** Per kind: where its records start in the population's one buffer. */
  base: Uint32Array;
  /** That buffer's contents, every kind's records in chunk order
   *  (`chunks.sorted` are views of it), and how many records it holds. */
  records: Float32Array<ArrayBuffer>;
  count: number;
}

export interface BuildingScene {
  style: BuildingStyle;
  art: BuildingArt;
  placed: PlacedBuildings;
  /** Per `placed.templates`: its rows. */
  templates: TemplateRows[];
  /** The library module a fallen part's remains draw with, or -1. */
  box: number;
  /** Per building: its bounds in the world (min, max), its tint's value, and
   *  whether the side knows it fell. */
  bounds: Float32Array;
  jitter: Float32Array;
  fallen: Uint8Array;
  /** Per building: its chunk in `coarse`, or -1 for one with no coarse row. */
  chunkOf: Int32Array;

  coarse: RowPopulation;
  /** Per building: its coarse rows, a run of the rows as placed, and where
   *  each row's record is in `coarse.records`. */
  coarseFirst: Uint32Array;
  coarseCount: Uint32Array;
  coarseAt: Uint32Array;
  /** Per coarse record: its building, and its scale (a hidden record's is
   *  zero in the records: a fallen building's, a resident chunk's). */
  coarseOwner: Uint32Array;
  coarseScale: Float32Array;
  /** Runs of coarse records changed since the layer last uploaded them, as
   *  `first, count` pairs. */
  coarseDirty: number[];

  /** Per chunk: its buildings. */
  chunkBuildings: Uint32Array[];
  pool: PlacementPool;
  /** Per chunk: the tier it is resident at (-1: not), its records' handles,
   *  and whether knowledge changed under it. */
  level: Int8Array;
  handles: (Int32Array | null)[];
  stale: Uint8Array;
  /** Per chunk, from the last view it was in: the tier it should draw at,
   *  and its distance. */
  wanted: Int8Array;
  distance: Float32Array;
  /** The chunks the last view should draw finer than coarse, and the
   *  resident ones. */
  near: number[];
  resident: number[];

  /** The fallen buildings' rows, and a count of its rebuilds. */
  ruins: RowPopulation | null;
  ruinsVersion: number;

  /** What the last `selectBuildings` did: rows expanded into the pool,
   *  chunks drawn coarse for want of room, and whether chunks still wait for
   *  expansion (the frame should draw again). */
  expandedRows: number;
  refused: number;
  pending: boolean;
}

/** The tier a chunk `distance` metres off draws at. */
function buildingLevel(
  lodPxPerM: BuildingStyle["lod_px_per_m"],
  view: DetailView,
  distance: number,
): number {
  const px = view.pixelsPerMetre / Math.max(distance, 1e-3);
  return px > lodPxPerM[0] ? 0 : px > lodPxPerM[1] ? 1 : px > lodPxPerM[2] ? 2 : COARSEST;
}

const _srgb = color.create();
let _linear: Float32Array | null = null;
/** A byte of sRGB, linear. */
function linearOf(): Float32Array {
  if (!_linear) {
    _linear = new Float32Array(256);
    for (let v = 0; v < 256; v++)
      _linear[v] = color.setFromSRGB(_srgb, [v / 255, v / 255, v / 255])[0];
  }
  return _linear;
}

const NO_ROWS: TemplateRows = {
  runs: Array.from({ length: TIER_COUNT }, () => new Uint32Array(0)),
  counts: Array.from({ length: TIER_COUNT }, () => 0),
  min: [0, 0, 0],
  max: [0, 0, 0],
};

/** Grow `min`..`max` by row `r` of `art`: its module's bounds, scaled, turned
 *  and moved as the row says. */
function growByRow(art: BuildingArt, r: number, min: number[], max: number[]) {
  const { rows } = art.library;
  const b = art.bounds[rows.module[r]];
  if (!b) {
    const module = art.library.modules[rows.module[r]];
    throw new TemplateArtError(
      "kit.missing",
      `kit "${art.library.kits[module.kit].appearance}" is not installed`,
    );
  }
  const t = r * ROW_TRANSFORM_FLOATS;
  const [x, y, z, yaw] = [
    rows.transform[t],
    rows.transform[t + 1],
    rows.transform[t + 2],
    rows.transform[t + 3],
  ];
  const [sx, sy, sz] = [rows.transform[t + 4], rows.transform[t + 5], rows.transform[t + 6]];
  const [cos, sin] = [Math.cos(yaw), Math.sin(yaw)];
  for (const mx of [b.min[0] * sx, b.max[0] * sx])
    for (const my of [b.min[1] * sy, b.max[1] * sy]) {
      const [px, py] = [x + cos * mx - sin * my, y + sin * mx + cos * my];
      min[0] = Math.min(min[0], px);
      max[0] = Math.max(max[0], px);
      min[1] = Math.min(min[1], py);
      max[1] = Math.max(max[1], py);
    }
  min[2] = Math.min(min[2], z + b.min[2] * sz);
  max[2] = Math.max(max[2], z + b.max[2] * sz);
}

function templateRowsOf(art: BuildingArt, range: RowRange): TemplateRows {
  if (range.count === 0) return NO_ROWS;
  const { tiers } = art.library.rows;
  const runs: number[][] = Array.from({ length: TIER_COUNT }, () => []);
  const counts = Array.from({ length: TIER_COUNT }, () => 0);
  const min = [Infinity, Infinity, Infinity];
  const max = [-Infinity, -Infinity, -Infinity];
  for (let r = range.first; r < range.first + range.count; r++) {
    growByRow(art, r, min, max);
    for (let t = 0; t < TIER_COUNT; t++) {
      if (!(tiers[r] & (1 << t))) continue;
      const run = runs[t];
      counts[t]++;
      if (run.length && run[run.length - 2] + run[run.length - 1] === r) run[run.length - 1]++;
      else run.push(r, 1);
    }
  }
  return {
    runs: runs.map((run) => Uint32Array.from(run)),
    counts,
    min: min as [number, number, number],
    max: max as [number, number, number],
  };
}

// Expansion scratch: one run's transforms, one record, one frame.
let _transforms = new Float32Array(256 * ROW_TRANSFORM_FLOATS);
const _record = new Float32Array(RECORD);
const _range: RowRange = { first: 0, count: 0 };
const _frame = { translation: [0, 0, 0] as [number, number, number], yaw: 0 };

/** Place library rows `[first, first + count)` at building `b`'s frame: their
 *  transforms, `ROW_TRANSFORM_FLOATS` each, in the scratch returned. */
function placeRun(scene: BuildingScene, b: number, first: number, count: number): Float32Array {
  if (_transforms.length < count * ROW_TRANSFORM_FLOATS)
    _transforms = new Float32Array(count * 2 * ROW_TRANSFORM_FLOATS);
  const f = b * FRAME_FLOATS;
  const { frames } = scene.placed;
  _frame.translation[0] = frames[f];
  _frame.translation[1] = frames[f + 1];
  _frame.translation[2] = frames[f + 2];
  _frame.yaw = frames[f + 3];
  _range.first = first;
  _range.count = count;
  placeRows(scene.art.library, _range, _frame, _transforms, 0);
  return _transforms;
}

/** Write a model record into `out` at float `o`: a transform (x, y, z, yaw,
 *  then the scale per axis) and a linear tint. The identity palette slot, no
 *  x-ray, no card. */
function writeRecord(
  out: Float32Array,
  o: number,
  transform: ArrayLike<number>,
  t: number,
  r: number,
  g: number,
  b: number,
) {
  out[o] = transform[t];
  out[o + 1] = transform[t + 1];
  out[o + 2] = transform[t + 2];
  out[o + 3] = transform[t + 3];
  out[o + 4] = 0;
  out[o + 5] = 0;
  out[o + 6] = 0;
  out[o + 7] = 0;
  out[o + 8] = r;
  out[o + 9] = g;
  out[o + 10] = b;
  out[o + 11] = -1;
  out[o + 12] = transform[t + 4];
  out[o + 13] = transform[t + 5];
  out[o + 14] = transform[t + 6];
  out[o + 15] = 0;
}

/** Library row `r` of building `b`, placed (`transforms` at `t`), as a
 *  record: the row's tint times the building's own value. */
function writeRow(
  scene: BuildingScene,
  out: Float32Array,
  o: number,
  b: number,
  r: number,
  transforms: Float32Array,
  t: number,
) {
  const linear = linearOf();
  const { tint } = scene.art.library.rows;
  const value = scene.jitter[b];
  writeRecord(
    out,
    o,
    transforms,
    t,
    linear[tint[r * 3]] * value,
    linear[tint[r * 3 + 1]] * value,
    linear[tint[r * 3 + 2]] * value,
  );
}

/** Rows gathered for a population: records, each one's kind key (its module
 *  and tier mask) and its building. */
interface GatheredRows {
  records: number[];
  keys: number[];
  buildings: number[];
}
const kindKey = (module: number, mask: number) => mask * 65536 + module;

const _corner = vec3.create();

/** Chunk gathered rows: a kind a distinct (module, mask), each row bucketed
 *  where its building was placed and bounded by its building. */
function rowPopulation(scene: BuildingScene, rows: GatheredRows, levels: number): RowPopulation {
  const count = rows.keys.length;
  const keys = [...new Set(rows.keys)].sort((a, b) => a - b);
  const kindOf = new Map(keys.map((key, kind) => [key, kind]));
  const { bounds } = scene;
  const { frames } = scene.placed;
  const anchors = new Float32Array(count * 2);
  const sizes = new Float32Array(count);
  rows.buildings.forEach((b, i) => {
    anchors[i * 2] = frames[b * FRAME_FLOATS];
    anchors[i * 2 + 1] = frames[b * FRAME_FLOATS + 1];
    sizes[i] = bounds[b * 6 + 5] - bounds[b * 6 + 2];
  });
  const chunks = createStaticChunks(
    {
      records: Float32Array.from(rows.records),
      stride: RECORD,
      kinds: Uint16Array.from(rows.keys, (key) => kindOf.get(key)!),
      sizes,
      anchors,
      bound(i, box) {
        const o = rows.buildings[i] * 6;
        box3.expandByPoint(box, box, vec3.set(_corner, bounds[o], bounds[o + 1], bounds[o + 2]));
        box3.expandByPoint(
          box,
          box,
          vec3.set(_corner, bounds[o + 3], bounds[o + 4], bounds[o + 5]),
        );
      },
    },
    keys.length,
    scene.style.chunk_m,
    levels,
  );
  // One buffer for the population, a kind after another.
  const base = new Uint32Array(keys.length);
  const records = new Float32Array(count * RECORD);
  let at = 0;
  chunks.sorted.forEach((sorted, k) => {
    base[k] = at;
    records.set(sorted, at * RECORD);
    chunks.sorted[k] = records.subarray(at * RECORD, at * RECORD + sorted.length);
    at += sorted.length / RECORD;
  });
  return {
    chunks,
    module: Uint16Array.from(keys, (key) => key % 65536),
    mask: Uint8Array.from(keys, (key) => Math.floor(key / 65536)),
    base,
    records,
    count,
  };
}

/** The scene of `placed` buildings drawn from `art`, every one intact. A
 *  template the library has no art for is refused by name (`TemplateArtError`). */
export function createBuildingScene(
  placed: PlacedBuildings,
  art: BuildingArt,
  style: BuildingStyle,
): BuildingScene {
  const { library } = art;
  const count = placed.template.length;
  const templates = placed.templates.map((id) =>
    templateRowsOf(art, templateRows(library, id, "intact")),
  );
  const bounds = new Float32Array(count * 6);
  const jitter = new Float32Array(count);
  for (let b = 0; b < count; b++) {
    const rows = templates[placed.template[b]];
    const f = b * FRAME_FLOATS;
    const [x, y, z, yaw] = [
      placed.frames[f],
      placed.frames[f + 1],
      placed.frames[f + 2],
      placed.frames[f + 3],
    ];
    const [cos, sin] = [Math.cos(yaw), Math.sin(yaw)];
    let [x0, y0, x1, y1] = [Infinity, Infinity, -Infinity, -Infinity];
    for (const lx of [rows.min[0], rows.max[0]])
      for (const ly of [rows.min[1], rows.max[1]]) {
        const [px, py] = [x + cos * lx - sin * ly, y + sin * lx + cos * ly];
        x0 = Math.min(x0, px);
        x1 = Math.max(x1, px);
        y0 = Math.min(y0, py);
        y1 = Math.max(y1, py);
      }
    bounds.set([x0, y0, z + rows.min[2], x1, y1, z + rows.max[2]], b * 6);
    // The building's own value, from its owner: the same in every battle.
    const seeded = mulberry32.create(placed.owners[b]);
    jitter[b] = 1 + random.float(() => mulberry32.sample(seeded), -1, 1) * style.tint_jitter;
  }
  const scene: BuildingScene = {
    style,
    art,
    placed,
    templates,
    box: library.modules.findIndex(
      (m) => library.kits[m.kit].appearance === PROTOTYPE_KIT && m.module === PROTOTYPE_MODULE,
    ),
    bounds,
    jitter,
    fallen: new Uint8Array(count),
    chunkOf: new Int32Array(count).fill(-1),
    coarse: null as unknown as RowPopulation,
    coarseFirst: new Uint32Array(count),
    coarseCount: new Uint32Array(count),
    coarseAt: new Uint32Array(0),
    coarseOwner: new Uint32Array(0),
    coarseScale: new Float32Array(0),
    coarseDirty: [],
    chunkBuildings: [],
    pool: createPlacementPool(
      style.pool_records,
      RECORD,
      Math.max(1, library.modules.length) * COARSEST,
    ),
    level: new Int8Array(0),
    handles: [],
    stale: new Uint8Array(0),
    wanted: new Int8Array(0),
    distance: new Float32Array(0),
    near: [],
    resident: [],
    ruins: null,
    ruinsVersion: 0,
    expandedRows: 0,
    refused: 0,
    pending: false,
  };

  // The coarse population: every building's coarsest rows.
  const rows: GatheredRows = { records: [], keys: [], buildings: [] };
  for (let b = 0; b < count; b++) {
    scene.coarseFirst[b] = rows.keys.length;
    gatherRows(scene, rows, b, templates[placed.template[b]].runs[COARSEST], 1 << COARSEST);
    scene.coarseCount[b] = rows.keys.length - scene.coarseFirst[b];
  }
  const coarse = rowPopulation(scene, rows, 1);
  scene.coarse = coarse;
  scene.coarseAt = new Uint32Array(coarse.count);
  scene.coarseOwner = new Uint32Array(coarse.count);
  scene.coarseScale = new Float32Array(coarse.count * 3);
  coarse.chunks.order.forEach((order, k) =>
    order.forEach((row, i) => {
      const at = coarse.base[k] + i;
      scene.coarseAt[row] = at;
      scene.coarseOwner[at] = rows.buildings[row];
      scene.coarseScale.set(coarse.records.subarray(at * RECORD + 12, at * RECORD + 15), at * 3);
    }),
  );
  const chunkCount = coarse.chunks.chunks.length;
  scene.chunkBuildings = coarse.chunks.chunks.map((chunk, c) => {
    const inChunk = new Set<number>();
    for (let k = 0; k < coarse.chunks.kinds; k++)
      for (let i = chunk.start[k]; i < chunk.end[k]; i++)
        inChunk.add(scene.coarseOwner[coarse.base[k] + i]);
    const list = Uint32Array.from(inChunk).sort();
    for (const b of list) scene.chunkOf[b] = c;
    return list;
  });
  scene.level = new Int8Array(chunkCount).fill(-1);
  scene.handles = Array.from({ length: chunkCount }, () => null);
  scene.stale = new Uint8Array(chunkCount);
  scene.wanted = new Int8Array(chunkCount).fill(COARSEST);
  scene.distance = new Float32Array(chunkCount);
  return scene;
}

/** Gather building `b`'s rows in `runs`, each under the tier mask `mask`
 *  (the row's own when zero). */
function gatherRows(
  scene: BuildingScene,
  into: GatheredRows,
  b: number,
  runs: Uint32Array,
  mask: number,
) {
  const { module, tiers } = scene.art.library.rows;
  for (let run = 0; run < runs.length; run += 2) {
    const [first, count] = [runs[run], runs[run + 1]];
    const transforms = placeRun(scene, b, first, count);
    for (let i = 0; i < count; i++) {
      writeRow(scene, _record, 0, b, first + i, transforms, i * ROW_TRANSFORM_FLOATS);
      for (let f = 0; f < RECORD; f++) into.records.push(_record[f]);
      into.keys.push(kindKey(module[first + i], mask || tiers[first + i]));
      into.buildings.push(b);
    }
  }
}

/** A resident chunk's pool kind for `module` at tier `level`. */
const poolKind = (scene: BuildingScene, module: number, level: number) =>
  level * scene.art.library.modules.length + module;

/** Expand chunk `c`'s intact buildings' rows at tier `level` into the pool:
 *  their handles, or null (and nothing kept) when the pool has no room. */
function expand(scene: BuildingScene, c: number, level: number): Int32Array | null {
  const { placed, templates, pool } = scene;
  const { module } = scene.art.library.rows;
  let rows = 0;
  for (const b of scene.chunkBuildings[c])
    if (!scene.fallen[b]) rows += templates[placed.template[b]].counts[level];
  if (rows > pool.capacity - pool.used) return null;
  const handles = new Int32Array(rows);
  let held = 0;
  for (const b of scene.chunkBuildings[c]) {
    if (scene.fallen[b]) continue;
    const runs = templates[placed.template[b]].runs[level];
    for (let run = 0; run < runs.length; run += 2) {
      const [first, count] = [runs[run], runs[run + 1]];
      const transforms = placeRun(scene, b, first, count);
      for (let i = 0; i < count; i++) {
        writeRow(scene, _record, 0, b, first + i, transforms, i * ROW_TRANSFORM_FLOATS);
        const handle = poolAppend(pool, poolKind(scene, module[first + i], level), _record, 0);
        if (handle < 0) {
          for (let h = 0; h < held; h++) poolRemove(pool, handles[h]);
          return null;
        }
        handles[held++] = handle;
      }
    }
  }
  scene.expandedRows += rows;
  return handles;
}

/** Show or hide coarse records `[first, first + count)` as their buildings
 *  now stand: hidden while the side knows the building fell, or while its
 *  chunk is resident (the pool draws it). */
function showCoarse(scene: BuildingScene, first: number, count: number) {
  const { records } = scene.coarse;
  let changed = false;
  for (let at = first; at < first + count; at++) {
    const b = scene.coarseOwner[at];
    const hidden = scene.fallen[b] === 1 || scene.level[scene.chunkOf[b]] >= 0;
    for (let axis = 0; axis < 3; axis++) {
      const scale = hidden ? 0 : scene.coarseScale[at * 3 + axis];
      changed ||= records[at * RECORD + 12 + axis] !== scale;
      records[at * RECORD + 12 + axis] = scale;
    }
  }
  if (changed) scene.coarseDirty.push(first, count);
}

/** Chunk `c`'s coarse records, as they stand now. */
function showChunkCoarse(scene: BuildingScene, c: number) {
  const { chunks, base } = scene.coarse;
  const chunk = chunks.chunks[c];
  for (let k = 0; k < chunks.kinds; k++)
    if (chunk.end[k] > chunk.start[k])
      showCoarse(scene, base[k] + chunk.start[k], chunk.end[k] - chunk.start[k]);
}

/** Chunk `c` resident at `level` (-1: not), with its records' handles. */
function setResident(scene: BuildingScene, c: number, level: number, handles: Int32Array | null) {
  const before = scene.handles[c];
  if (before) for (const handle of before) poolRemove(scene.pool, handle);
  const was = scene.level[c] >= 0;
  scene.handles[c] = handles;
  scene.level[c] = level;
  scene.stale[c] = 0;
  if (was !== level >= 0) showChunkCoarse(scene, c);
}

const evict = (scene: BuildingScene, c: number) => setResident(scene, c, -1, null);

/** Bring the pool to the view's near chunks (`near`, each with its `wanted`
 *  tier and `distance`): chunks out of it leave, and chunks in it are
 *  expanded at their tier, nearest first, while the view's expansion budget
 *  and the pool's room last. A resident chunk waiting to change tier keeps
 *  the one it has. */
function settleResidency(scene: BuildingScene) {
  const { level, wanted, distance, stale } = scene;
  const near = scene.near.sort((a, b) => distance[a] - distance[b]);
  const wantedNow = new Set(near);
  for (const c of scene.resident) if (!wantedNow.has(c)) evict(scene, c);

  let budget = scene.style.expand_rows;
  let full = false;
  for (const c of near) {
    if (level[c] === wanted[c] && !stale[c]) continue;
    const held = level[c] >= 0;
    if (full) {
      if (!held) scene.refused++;
      continue;
    }
    if (budget <= 0) {
      scene.pending = true;
      continue;
    }
    const before = scene.expandedRows;
    let next = expand(scene, c, wanted[c]);
    // No room beside its own old tier: without it, then.
    if (!next && held) {
      evict(scene, c);
      next = expand(scene, c, wanted[c]);
    }
    // No room at all: the farthest residents make way, while any is farther.
    for (let far = near.length - 1; !next && distance[near[far]] > distance[c]; far--) {
      if (level[near[far]] < 0) continue;
      evict(scene, near[far]);
      next = expand(scene, c, wanted[c]);
    }
    if (!next) {
      full = true;
      scene.refused++;
      continue;
    }
    setResident(scene, c, wanted[c], next);
    budget -= scene.expandedRows - before;
  }
  scene.resident = near.filter((c) => level[c] >= 0);
}

/**
 * Choose what `view` draws of the scene: the coarse population's ranges and
 * casters, the pool's residents, and the fallen population's ranges. Expands
 * and evicts as the view needs; with nothing changed since the last call for
 * the same view it expands nothing.
 */
export function selectBuildings(
  scene: BuildingScene,
  view: DetailView,
  shadow: SunShadow | null,
): void {
  const { coarse, style } = scene;
  scene.expandedRows = 0;
  scene.refused = 0;
  scene.pending = false;
  scene.near = [];
  // Every chunk in view draws its coarse records; a resident one's are hidden.
  selectChunks(
    coarse.chunks,
    view,
    (_chunk, distance, v, index) => {
      scene.wanted[index] = buildingLevel(style.lod_px_per_m, v, distance);
      scene.distance[index] = distance;
      if (scene.wanted[index] < COARSEST) scene.near.push(index);
      return 0;
    },
    shadow,
  );
  settleResidency(scene);
  if (scene.ruins)
    selectChunks(
      scene.ruins.chunks,
      view,
      (_chunk, distance, v) => buildingLevel(style.lod_px_per_m, v, distance),
      shadow,
    );
}

const _box_transform = new Float32Array(ROW_TRANSFORM_FLOATS);

/**
 * What the side knows fell (`fallen`, replacing the last list). A building
 * that changes leaves or rejoins the intact rows: its coarse records are
 * hidden or shown in place (`coarseDirty` names them) and its chunk, if
 * resident, is expanded again at the next view. The fallen population is
 * rebuilt whole: it holds only what fell.
 */
export function setFallenBuildings(scene: BuildingScene, fallen: readonly FallenBuilding[]): void {
  const count = scene.placed.template.length;
  const known = fallen.filter((f) => f.building >= 0 && f.building < count);
  const next = new Uint8Array(count);
  for (const f of known) next[f.building] = 1;
  const before = scene.fallen;
  scene.fallen = next;
  for (let b = 0; b < count; b++) {
    if (next[b] === before[b]) continue;
    for (let row = scene.coarseFirst[b]; row < scene.coarseFirst[b] + scene.coarseCount[b]; row++)
      showCoarse(scene, scene.coarseAt[row], 1);
    const c = scene.chunkOf[b];
    if (c >= 0 && scene.level[c] >= 0) scene.stale[c] = 1;
  }
  scene.ruinsVersion++;
  scene.ruins = null;
  if (!known.length) return;

  const { library } = scene.art;
  const byId = new Map(library.templates.map((t) => [t.id, t]));
  const linear = linearOf();
  const rows: GatheredRows = { records: [], keys: [], buildings: [] };
  for (const f of known) {
    const b = f.building;
    const state = byId.get(scene.placed.templates[scene.placed.template[b]])?.states[f.state];
    if (state) {
      gatherRows(scene, rows, b, Uint32Array.of(state.first, state.count), 0);
      continue;
    }
    // No art for the state: each part as the side knows it, a box.
    if (scene.box < 0) continue;
    const value = scene.jitter[b];
    const [r, g, bl] = scene.style.ruin_tint.map((c) => linear[Math.round(c * 255)] * value);
    for (const part of f.parts) {
      _box_transform.set([
        part.center[0],
        part.center[1],
        part.baseZ,
        part.yaw,
        2 * part.half[0],
        2 * part.half[1],
        2 * part.half[2],
      ]);
      writeRecord(_record, 0, _box_transform, 0, r, g, bl);
      for (let i = 0; i < RECORD; i++) rows.records.push(_record[i]);
      rows.keys.push(kindKey(scene.box, (1 << TIER_COUNT) - 1));
      rows.buildings.push(b);
    }
  }
  if (rows.keys.length) scene.ruins = rowPopulation(scene, rows, TIER_COUNT);
}

/** Which buffer a draw's records are in. */
export const COARSE = 0;
export const POOL = 1;
export const RUINS = 2;

/** One draw: `count` records from `first` of buffer `source`, as library
 *  module `module`'s mesh at `tier`. */
export type BuildingDraw = (
  source: number,
  module: number,
  tier: number,
  first: number,
  count: number,
) => void;

/** Draw kind `k`'s `ranges` of `population`, those within `bridge` records
 *  of each other as one. */
function eachRange(
  population: RowPopulation,
  source: number,
  k: number,
  ranges: number[],
  tier: number,
  draw: BuildingDraw,
  bridge = 0,
) {
  for (let r = 0; r < ranges.length; ) {
    const first = ranges[r];
    let end = first + ranges[r + 1];
    for (r += 2; r < ranges.length && ranges[r] >= end && ranges[r] - end <= bridge; r += 2)
      end = ranges[r] + ranges[r + 1];
    draw(source, population.module[k], tier, population.base[k] + first, end - first);
  }
}

/** Every draw of the last `selectBuildings` into the view. */
export function buildingDraws(scene: BuildingScene, draw: BuildingDraw): void {
  const { coarse, pool, ruins } = scene;
  for (let k = 0; k < coarse.chunks.kinds; k++)
    eachRange(coarse, COARSE, k, coarse.chunks.ranges[k][0], COARSEST, draw, BRIDGE_RECORDS);
  const modules = scene.art.library.modules.length;
  for (let kind = 0; kind < pool.count.length; kind++)
    if (pool.count[kind])
      draw(POOL, kind % modules, Math.floor(kind / modules), pool.first[kind], pool.count[kind]);
  if (ruins)
    for (let tier = 0; tier < TIER_COUNT; tier++)
      for (let k = 0; k < ruins.chunks.kinds; k++)
        if (ruins.mask[k] & (1 << tier))
          eachRange(ruins, RUINS, k, ruins.chunks.ranges[k][tier], tier, draw);
}

/** Every draw of the last `selectBuildings` into the sun's cascades: the
 *  coarse rows of what is drawn coarse and of chunks out of view whose
 *  shadow lands in it, and the residents' own rows a tier coarser. */
export function buildingCasters(scene: BuildingScene, draw: BuildingDraw): void {
  const { coarse, pool, ruins } = scene;
  for (let k = 0; k < coarse.chunks.kinds; k++)
    eachRange(coarse, COARSE, k, coarse.chunks.cast[k], COARSEST, draw, BRIDGE_RECORDS);
  const modules = scene.art.library.modules.length;
  for (let kind = 0; kind < pool.count.length; kind++)
    if (pool.count[kind])
      draw(
        POOL,
        kind % modules,
        Math.min(COARSEST, Math.floor(kind / modules) + CASTER_COARSER),
        pool.first[kind],
        pool.count[kind],
      );
  if (ruins)
    for (let k = 0; k < ruins.chunks.kinds; k++)
      if (ruins.mask[k] & (1 << COARSEST))
        eachRange(ruins, RUINS, k, ruins.chunks.cast[k], COARSEST, draw);
}
