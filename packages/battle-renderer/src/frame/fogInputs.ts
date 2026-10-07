// What renderer fog is cut from, assembled on the CPU. Three inputs, each with
// one owner:
// - `FogGeometryPresentation`: `presentation.fog_geometry`, the technique's
//   resolution and budgets (spike 02's numbers, provisional);
// - `FogWorld`: the public static map sight is cut by (the simulation's terrain
//   grid, sampled with its triangle rule, and the foliage its standing trees
//   give each 8 m cell), plus the sensor
//   rule numbers fog shares with the simulation's sweep;
// - `FogSight`: per publication, every own eye (`OwnUnit.sight` at the
//   published tick, never interpolated) and the occluders the side knows
//   stand: the static map's occluding props less those it has seen fall, plus
//   the obstacles, ruins and wrecks it has learned. A prop the side has not
//   learned never occludes.
import type { WorldExports, WorldLayout } from "../worldMesh";
import type { SightLobe, SightShape } from "../sightOverlay";
import type { KnownProp } from "../models/propAppearance";
import type { TerrainGrid } from "../terrain/terrainGrid";

/** `presentation.fog_geometry`: resolution and budgets of the sight lights. */
export interface FogGeometryPresentation {
  /** Rays per eye of the full map, where buildings are intersected exactly. */
  azimuth_bins: number;
  /** Rays per eye of the coarse terrain march. */
  terrain_azimuth_bins: number;
  /** Log-spaced radial bins per ray, from `first_bin_m` to the eye's reach. */
  radial_bins: number;
  first_bin_m: number;
  /** The terrain march's step: `fraction` of the distance, within `[min, max]` metres. */
  terrain_step_m: readonly [number, number];
  terrain_step_fraction: number;
  /** Screen tile of the per-tile eye lists, pixels a side. */
  tile_px: number;
  /** Eyes a tile's list holds; more are dropped (and counted). */
  tile_eyes_max: number;
  /** A face probes this far outside itself, along its normal. */
  face_probe_m: number;
  /** A roof or canopy top above an eye looks this far toward it for seen air
   *  at its own height (fogTerm.ts); 0 leaves such surfaces as sight says. */
  roof_reach_m: number;
  /** Moved eyes rebuilt per frame; the rest keep their last map until their
   *  turn. New eyes are always built at once. */
  rebuild_eyes_per_frame: number;
  /** An occluder takes fog whole (`fogWholeSeen`): its walls and roof are
   *  sampled this far apart, and any sample seen shows all of it. */
  whole_step_m: number;
}

export function validateFogGeometry(g: FogGeometryPresentation): FogGeometryPresentation {
  const whole = (v: number, name: string, min = 1) => {
    if (!Number.isInteger(v) || v < min)
      throw new Error(`fog_geometry.${name} must be an integer ≥ ${min}`);
  };
  whole(g.azimuth_bins, "azimuth_bins", 8);
  whole(g.terrain_azimuth_bins, "terrain_azimuth_bins", 8);
  whole(g.radial_bins, "radial_bins", 2);
  whole(g.tile_px, "tile_px");
  whole(g.tile_eyes_max, "tile_eyes_max");
  whole(g.rebuild_eyes_per_frame, "rebuild_eyes_per_frame");
  // One cull workgroup per tile: WebGPU guarantees 256 invocations.
  if (g.tile_px * g.tile_px > 256) throw new Error("fog_geometry.tile_px must be ≤ 16");
  if (!(g.first_bin_m > 0 && g.face_probe_m > 0 && g.terrain_step_fraction > 0))
    throw new Error(
      "fog_geometry: first_bin_m, face_probe_m and terrain_step_fraction must be > 0",
    );
  if (!(g.roof_reach_m >= 0)) throw new Error("fog_geometry.roof_reach_m must be ≥ 0");
  if (!(g.whole_step_m >= 0.25)) throw new Error("fog_geometry.whole_step_m must be ≥ 0.25");
  const [lo, hi] = g.terrain_step_m;
  if (!(lo > 0 && hi >= lo)) throw new Error("fog_geometry.terrain_step_m must be 0 < min ≤ max");
  return g;
}

/** The sensor rules fog shares with the simulation's sweep (`rules.sensors`). */
export interface FogSensorRules {
  fog_target_height_m: number;
  foliage_full_block: number;
  min_sight_gap_m: number;
}

/** Foliage depth is stored per bin in steps of this, in 8 bits. */
export const FOLIAGE_STEP = 0.005;
export const FOLIAGE_MAX = 255 * FOLIAGE_STEP;

/** The public static map sight is cut by. */
export interface FogWorld extends TerrainGrid {
  /** Sparse simulation foliage: `nx, ny, cell_m`, then sorted
   * `[column, row, canopy_m, depth_per_m]` records. Missing cells are open. */
  foliage: Float32Array;
  targetHeightM: number;
  foliageFullBlock: number;
  /** Least room a line of sight needs between occluders on its two sides. */
  minSightGapM: number;
}

/** The static world fog reads, from the simulation's own exported geometry. */
export function fogWorld(exports: WorldExports, sensors: FogSensorRules): FogWorld {
  // Foliage saturates at 8 bits: the full block must fall inside that.
  if (sensors.foliage_full_block > FOLIAGE_MAX)
    throw new Error(`fog: foliage_full_block must be ≤ ${FOLIAGE_MAX}`);
  return {
    ...exports.terrain,
    foliage: exports.foliage,
    targetHeightM: sensors.fog_target_height_m,
    foliageFullBlock: sensors.foliage_full_block,
    minSightGapM: sensors.min_sight_gap_m,
  };
}

/** An oriented box that hides what lies behind it, and takes fog whole
 *  (a building, a ruin, a wall: `fogWholeSeen`). */
export interface FogOccluder {
  x: number;
  y: number;
  yaw: number;
  /** Half extents along and across the heading. */
  hx: number;
  hy: number;
  /** Heights of its base and its top. */
  base: number;
  top: number;
}

/** The static map's occluding props, built once for a world: each as the
 *  same box object for the whole battle, by map prop id. */
export interface MapOccluders {
  boxes: readonly FogOccluder[];
  ids: readonly number[];

  /** The occluding prop kinds, for what a side learns. */
  occludes: ReadonlySet<string>;
}

export function mapOccluders(exports: WorldExports, layout: WorldLayout): MapOccluders {
  const occludes = new Set(layout.occludingPropKinds);
  const at = Object.fromEntries(layout.propFields.map((f, i) => [f, i]));
  const props = exports.props;
  const boxes: FogOccluder[] = [];
  const ids: number[] = [];
  for (let r = 0; r < props.length; r += layout.propStride) {
    if (!occludes.has(layout.propKinds[props[r + at.kind]])) continue;
    ids.push(props[r + at.idLo] + props[r + at.idHi] * 2 ** layout.limbBits);
    boxes.push({
      x: props[r + at.x],
      y: props[r + at.y],
      yaw: props[r + at.yaw],
      hx: props[r + at.hx],
      hy: props[r + at.hy],
      base: props[r + at.baseZ],
      top: props[r + at.baseZ] + 2 * props[r + at.hz],
    });
  }
  return { boxes, ids, occludes };
}

/** The occluders a side knows stand: the static map's occluding props less
 *  the ones it has seen fall (the same box objects, so a change is found by
 *  identity: `fogAffectedEyes`), plus the learned props that occlude. */
export function knownOccluders(map: MapOccluders, known: readonly KnownProp[]): FogOccluder[] {
  const fallen = new Set(known.flatMap((p) => (p.authoredProp === null ? [] : [p.authoredProp])));
  const out = fallen.size ? map.boxes.filter((_, i) => !fallen.has(map.ids[i])) : [...map.boxes];
  for (const p of known) {
    if (!map.occludes.has(p.kind) || p.destroyed) continue;
    out.push({
      x: p.center[0],
      y: p.center[1],
      yaw: p.yaw,
      hx: p.half[0],
      hy: p.half[1],
      base: p.baseZ,
      top: p.baseZ + 2 * p.half[2],
    });
  }
  return out;
}

/** Words a row of the `wholes` texture holds (`fogWholeSeen`). */
export const WHOLE_TEXTURE_WIDTH = 256;
/** Words per structure box in the `wholes` texture: centre x, y; cos and sin
 *  of its yaw; half extents along and across it; base and top heights (f32
 *  bits). */
export const WHOLE_BOX_WORDS = 8;

/** The structures that take fog whole, as `wholes` texture words: a grid of
 *  cells over their footprints (each `start << 8 | count` into the item
 *  lists), the lists, the boxes (`WHOLE_BOX_WORDS` each), then a seen flag
 *  per box from the first row after. `margin` grows each footprint, as the
 *  fragment's test does. */
export function wholeWords(boxes: readonly FogOccluder[], margin: number) {
  const n = boxes.length;
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;
  // Each grown box's own bounds: grown in its frame, then turned.
  const extents = new Float64Array(2 * n);
  boxes.forEach((b, i) => {
    const c = Math.abs(Math.cos(b.yaw));
    const s = Math.abs(Math.sin(b.yaw));
    const ex = (extents[2 * i] = (b.hx + margin) * c + (b.hy + margin) * s);
    const ey = (extents[2 * i + 1] = (b.hx + margin) * s + (b.hy + margin) * c);
    minX = Math.min(minX, b.x - ex);
    minY = Math.min(minY, b.y - ey);
    maxX = Math.max(maxX, b.x + ex);
    maxY = Math.max(maxY, b.y + ey);
  });
  if (!n) {
    minX = minY = 0;
    maxX = maxY = 1;
  }
  const cell = Math.max(WHOLE_CELL_M, Math.max(maxX - minX, maxY - minY) / WHOLE_GRID_MAX);
  // One more than the far edge's cell: a point exactly on it still has one.
  const nx = Math.floor((maxX - minX) / cell) + 1;
  const ny = Math.floor((maxY - minY) / cell) + 1;
  // Each box's cell range, then the lists by counting: a cell's list holds
  // its boxes in list order, the lists in cell order.
  const ranges = new Int32Array(4 * n);
  const counts = new Uint32Array(nx * ny);
  boxes.forEach((b, i) => {
    const ex = extents[2 * i];
    const ey = extents[2 * i + 1];
    const i0 = (ranges[4 * i] = Math.max(0, Math.floor((b.x - ex - minX) / cell)));
    const i1 = (ranges[4 * i + 1] = Math.min(nx - 1, Math.floor((b.x + ex - minX) / cell)));
    const j0 = (ranges[4 * i + 2] = Math.max(0, Math.floor((b.y - ey - minY) / cell)));
    const j1 = (ranges[4 * i + 3] = Math.min(ny - 1, Math.floor((b.y + ey - minY) / cell)));
    for (let j = j0; j <= j1; j++) for (let k = i0; k <= i1; k++) counts[j * nx + k]++;
  });
  let items = 0;
  for (const count of counts) {
    if (count > 255) throw new Error("fog: more than 255 structures share a whole-fog cell");
    items += count;
  }
  const itemsBase = nx * ny;
  const boxesBase = itemsBase + items;
  const W = WHOLE_TEXTURE_WIDTH;
  const flagsRow = Math.ceil((boxesBase + n * WHOLE_BOX_WORDS) / W);
  const flagRows = Math.max(1, Math.ceil(n / W));
  const words = new Uint32Array((flagsRow + flagRows) * W);
  const floats = new Float32Array(words.buffer);
  // A cell's word is its list's start and length; `next` its fill cursor.
  const next = new Uint32Array(nx * ny);
  let start = 0;
  counts.forEach((count, c) => {
    words[c] = (start << 8) | count;
    next[c] = itemsBase + start;
    start += count;
  });
  for (let i = 0; i < n; i++)
    for (let j = ranges[4 * i + 2]; j <= ranges[4 * i + 3]; j++)
      for (let k = ranges[4 * i]; k <= ranges[4 * i + 1]; k++) words[next[j * nx + k]++] = i;
  boxes.forEach((b, i) => {
    floats.set(
      [b.x, b.y, Math.cos(b.yaw), Math.sin(b.yaw), b.hx, b.hy, b.base, b.top],
      boxesBase + i * WHOLE_BOX_WORDS,
    );
  });
  return {
    words,
    rows: flagsRow + flagRows,
    flagsRow,
    flagRows,
    params: {
      wholeCount: n,
      wholeNx: nx,
      wholeNy: ny,
      wholeCellM: cell,
      wholeOrigin: [minX, minY] as [number, number],
      wholeItemsBase: itemsBase,
      wholeBoxesBase: boxesBase,
      wholeFlagsBase: flagsRow * W,
    },
  };
}
/** The whole-fog grid's smallest cell, metres, and most cells a side. */
const WHOLE_CELL_M = 8;
const WHOLE_GRID_MAX = 512;

/** One own eye at the published tick. */
export interface FogEye {
  /** Stable per unit and eye index, so an eye that did not move keeps its map. */
  key: string;
  position: readonly [number, number, number];
  forward: number;
  shape: SightShape;
  range: number;
}

/** What the side sees from, and what it knows stands in the way. */
export interface FogSight {
  eyes: readonly FogEye[];
  /** Replaced whole when knowledge changes: a new array rebuilds every map. */
  occluders: readonly FogOccluder[];
}

/** Every eye of every own unit (a garrison gives one per facade it holds). */
export function fogEyes(own: readonly { id: number; sight: SightLobe }[]): FogEye[] {
  return own.flatMap((u) =>
    u.sight.eyes.map((position, k) => ({
      key: `${u.id}:${k}`,
      position,
      forward: u.sight.forward,
      shape: u.sight.shape,
      range: u.sight.range,
    })),
  );
}

/** How far an eye reaches in any direction: its map's radius. */
export function eyeReach(eye: FogEye): number {
  return eye.range * Math.max(eye.shape.front, eye.shape.side, eye.shape.rear);
}

/** Everything fog is drawn from; `null` draws the world clear. */
export interface FogInput {
  world: FogWorld;
  sight: FogSight;
}
