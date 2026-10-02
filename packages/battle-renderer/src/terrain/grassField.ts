// The grass field on the CPU: what the biome and the grass appearances say,
// packed for the GPU, and the per-frame window of world tiles the field is
// grown over. Where clumps stand, how many and how tall is decided per frame
// on the GPU (`frame/grassPass.ts`); nothing here is per clump.
//
// The field is a technique rewrite of ~/dev/game's
// grassField.ts and battleGrassResidency.ts, after the Ghost of Tsushima
// talk (research.md): the clumps are regrown on the GPU from world tiles
// whenever the view moves, so there is no CPU residency or upload, and a
// clump's place is a function of its tile alone, so what is drawn never
// reshuffles as the camera moves.
import { mat4, type Mat4, type Vec3 } from "math";
import { frustum, type Frustum } from "math/shapes";
import type { StaticBundle } from "@packages/scene-assets/src/schema.ts";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader.ts";
import {
  GRASS_MAX_HEIGHT_M,
  GRASS_SEGMENTS,
  grassBladeVertices,
  maxFieldGrassHeight,
  grassMeshHeight,
} from "@packages/scene-assets/src/grass.ts";
import { GRASS_MIX_MAX, VERGE_GROWTH, type Biome, type GrassRules } from "./biome";

/** World tiles the field is grown over, metres a side. */
export const GRASS_TILE_M = 4;
/** Tiles a side the window may hold (the dispatch's bound). */
export const GRASS_MAX_TILES = 256;
/** Plot kinds the GPU growth table holds; the last row is the verge. */
export const GRASS_GROWTH_ROWS = 16;
/** Grass thins and lowers over this far past a road's or a wood's margin. */
export const GRASS_EDGE_M = 1.2;
/** Grass appearances one field draws. */
export const GRASS_MAX_KINDS = 16;
/** The field's two tiers, near then far: which of a grass kind's LOD tiers
 *  each draws (4 and 2 segments a blade). */
export const FIELD_LODS = [0, 2] as const;

/** The prop table's grid: its smallest cell, metres, and most cells a side. */
const PROP_CELL_M = 8;
const PROP_GRID_MAX = 512;
/** A prop is listed this far past its bounds, so a point the GPU puts in a
 *  neighbouring cell (f32 against the table's f64) still finds it. */
const PROP_PAD_M = 0.05;

/** Where no grass grows: every static prop's footprint, widened by
 *  `margin`, as the table the grass build reads. A clump asks only the
 *  props listed for its own grid cell, so the build costs what is near a
 *  clump, never the map's prop count. In `vec4f` elements:
 *  - 0: the grid's origin, its cell's side, 0;
 *  - 1 (as `u32`): cells across and up, 0, 0;
 *  - then two cells an element, each (first record's element, record count);
 *  - then the records, grouped by cell, two elements each:
 *    (x, y, cos yaw, sin yaw), (half extents with the margin, 0, 0).
 *  `footprints` holds `x, y, yaw, hx, hy` per prop. */
export function packGrassProps(footprints: Float32Array, margin: number): Float32Array {
  const count = footprints.length / 5;
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;
  const bounds = new Float64Array(count * 4);
  for (let i = 0; i < count; i++) {
    const [x, y, yaw, hx, hy] = footprints.subarray(i * 5, i * 5 + 5);
    const [c, s] = [Math.abs(Math.cos(yaw)), Math.abs(Math.sin(yaw))];
    const ex = (hx + margin) * c + (hy + margin) * s + PROP_PAD_M;
    const ey = (hx + margin) * s + (hy + margin) * c + PROP_PAD_M;
    bounds.set([x - ex, y - ey, x + ex, y + ey], i * 4);
    minX = Math.min(minX, x - ex);
    minY = Math.min(minY, y - ey);
    maxX = Math.max(maxX, x + ex);
    maxY = Math.max(maxY, y + ey);
  }
  const cell = count
    ? Math.max(PROP_CELL_M, Math.max(maxX - minX, maxY - minY) / PROP_GRID_MAX)
    : PROP_CELL_M;
  const nx = count ? Math.floor((maxX - minX) / cell) + 1 : 0;
  const ny = count ? Math.floor((maxY - minY) / cell) + 1 : 0;
  // Two passes: count each cell's props, then fill its run of records.
  const counts = new Uint32Array(nx * ny);
  const span = (i: number) => [
    Math.floor((bounds[i * 4] - minX) / cell),
    Math.floor((bounds[i * 4 + 1] - minY) / cell),
    Math.floor((bounds[i * 4 + 2] - minX) / cell),
    Math.floor((bounds[i * 4 + 3] - minY) / cell),
  ];
  let records = 0;
  for (let i = 0; i < count; i++) {
    const [i0, j0, i1, j1] = span(i);
    for (let j = j0; j <= j1; j++) for (let k = i0; k <= i1; k++) counts[j * nx + k]++;
    records += (i1 - i0 + 1) * (j1 - j0 + 1);
  }
  const cellElements = Math.ceil((nx * ny) / 2);
  const recordsBase = 2 + cellElements;
  const table = new Float32Array((recordsBase + records * 2) * 4);
  const words = new Uint32Array(table.buffer);
  table.set([count ? minX : 0, count ? minY : 0, cell, 0]);
  words.set([nx, ny, 0, 0], 4);
  const next = new Uint32Array(nx * ny);
  let first = recordsBase;
  for (let c = 0; c < nx * ny; c++) {
    words.set([first, counts[c]], 8 + c * 2);
    next[c] = first;
    first += counts[c] * 2;
  }
  for (let i = 0; i < count; i++) {
    const [x, y, yaw, hx, hy] = footprints.subarray(i * 5, i * 5 + 5);
    const record = [x, y, Math.cos(yaw), Math.sin(yaw), hx + margin, hy + margin, 0, 0];
    const [i0, j0, i1, j1] = span(i);
    for (let j = j0; j <= j1; j++)
      for (let k = i0; k <= i1; k++) {
        table.set(record, next[j * nx + k] * 4);
        next[j * nx + k] += 2;
      }
  }
  return table;
}

/** Grass appearances (scenery kind "grass") by catalog name. */
export type GrassAppearances = ReadonlyMap<string, StaticBundle>;

/** The grass kinds in an installed catalog generation. */
export function grassAppearancesOf(installed: InstalledAppearances | null): GrassAppearances {
  const out = new Map<string, StaticBundle>();
  for (const [name, { scenery, bundle }] of installed?.appearances ?? [])
    if (scenery === "grass" && bundle.kind === "static") out.set(name, bundle);
  return out;
}

/** The biome's growth resolved against the installed appearances. */
export interface GrassKinds {
  /** Per growth row (plot kind index, the verge last): density, height
   *  scale, how many grasses it mixes, 0. A plot kind that grows none has
   *  density 0. */
  growth: Float32Array;
  /** Per growth row, its patches: lowest and tallest height scale, how far
   *  sparse patches thin, how far dry patches dry. */
  patches: Float32Array;
  /** Per growth row, `GRASS_MIX_MAX` grasses: appearance index, share of the
   *  row's clumps (the row's shares sum to one), drift, dryness. */
  mixes: Float32Array;
  /** The appearances the growth names, in index order. */
  appearances: { name: string; bundle: StaticBundle }[];
  /** The tallest any clump is drawn, over every row and grass. */
  tallest: number;
}

/** Resolve `biome.grass.growth` against `appearances`: null (no grass) until
 *  every appearance it names is installed. */
export function grassKinds(biome: Biome, appearances: GrassAppearances): GrassKinds | null {
  const rules = biome.grass;
  if (biome.plots.length > GRASS_GROWTH_ROWS - 1)
    throw new Error(`grass: at most ${GRASS_GROWTH_ROWS - 1} plot kinds grow grass`);
  const names = [
    ...new Set(Object.values(rules.growth).flatMap((g) => g.mix.map((s) => s.appearance))),
  ].sort();
  if (names.length > GRASS_MAX_KINDS)
    throw new Error(`grass: at most ${GRASS_MAX_KINDS} grass appearances`);
  if (!names.every((name) => appearances.has(name))) return null;
  const growth = new Float32Array(GRASS_GROWTH_ROWS * 4);
  const patches = new Float32Array(GRASS_GROWTH_ROWS * 4);
  const mixes = new Float32Array(GRASS_GROWTH_ROWS * GRASS_MIX_MAX * 4);
  let tallest = 0;
  const row = (at: number, key: string) => {
    const g = rules.growth[key];
    if (!g) return;
    const shares = g.mix.reduce((sum, s) => sum + s.share, 0);
    g.mix.forEach((species, i) => {
      const maximum = maxFieldGrassHeight(
        grassMeshHeight(appearances.get(species.appearance)!.states[0].tiers),
        g.height,
        g.patches.height[1],
      );
      if (!Number.isFinite(maximum) || maximum <= 0 || maximum > GRASS_MAX_HEIGHT_M)
        throw new Error(
          `grass.growth.${key}: ${species.appearance}'s effective field height ${maximum} m exceeds the ${GRASS_MAX_HEIGHT_M} m cap`,
        );
      tallest = Math.max(tallest, maximum);
      mixes.set(
        [names.indexOf(species.appearance), species.share / shares, species.drift, species.dry],
        (at * GRASS_MIX_MAX + i) * 4,
      );
    });
    growth.set([g.density, g.height, g.mix.length, 0], at * 4);
    patches.set([g.patches.height[0], g.patches.height[1], g.patches.thin, g.patches.dry], at * 4);
  };
  biome.plots.forEach((plot, k) => row(k, plot.name));
  row(GRASS_GROWTH_ROWS - 1, VERGE_GROWTH);
  return {
    growth,
    patches,
    mixes,
    appearances: names.map((name) => ({ name, bundle: appearances.get(name)! })),
    tallest,
  };
}

/** Floats per clump vertex on the GPU: spine (xyz, height fraction), side
 *  (xyz, phase), normal (xyz, 0), tint (rgb, how far it answers the wind). */
export const SHAPE_FLOATS = 16;

const linear = (c: number) => (c / 255) ** 2.2;

/** Every kind's two field tiers as spine-and-side vertices, each vertex's
 *  tint its colour relative to the clump's mean colour (which the ground it
 *  grows on replaces); plus per kind its height, blades and root width, and
 *  where each tier starts. */
export function packGrassShapes(kinds: GrassKinds) {
  const perTier = FIELD_LODS.map((lod) => grassBladeVertices(GRASS_SEGMENTS[lod]));
  const bladesOf = (b: StaticBundle) => b.states[0].tiers[0].positions.length / 3 / perTier[0];
  const total = kinds.appearances.reduce(
    (n, { bundle }) => n + bladesOf(bundle) * (perTier[0] + perTier[1]),
    0,
  );
  const shapes = new Float32Array(Math.max(1, total) * SHAPE_FLOATS);
  const rows = new Float32Array(GRASS_MAX_KINDS * 4);
  const bases = new Uint32Array(GRASS_MAX_KINDS * 4);
  let at = 0;
  let maxBlades = 1;
  kinds.appearances.forEach(({ name, bundle }, k) => {
    const state = bundle.states[0];
    const blades = bladesOf(bundle);
    maxBlades = Math.max(maxBlades, blades);
    const lod0 = state.tiers[0];
    const vertices = lod0.positions.length / 3;
    const mean = [0, 1, 2].map((c) => {
      let sum = 0;
      for (let v = 0; v < vertices; v++) sum += linear(lod0.colors[v * 4 + c]);
      return Math.max(sum / vertices, 1e-4);
    });
    const rootWidth = Math.hypot(
      lod0.positions[0] - lod0.positions[3],
      lod0.positions[1] - lod0.positions[4],
    );
    rows.set([state.bounds.max[2], blades, rootWidth, 0], k * 4);
    FIELD_LODS.forEach((lod, t) => {
      bases[k * 4 + t] = at;
      const mesh = state.tiers[lod];
      const per = perTier[t];
      if (mesh.positions.length / 3 !== blades * per)
        throw new Error(`grass: ${name} LOD${lod} is not ${blades} blade strips`);
      const p = (v: number, c: number) => mesh.positions[v * 3 + c];
      for (let v = 0; v < blades * per; v++) {
        const o = at * SHAPE_FLOATS;
        const inBlade = v % per;
        const tip = inBlade === per - 1;
        // A strip pair's spine is its midpoint; the tip is its own spine.
        const left = tip ? v : v - (inBlade % 2);
        const right = tip ? v : left + 1;
        for (let c = 0; c < 3; c++) {
          const spine = (p(left, c) + p(right, c)) / 2;
          shapes[o + c] = spine;
          shapes[o + 4 + c] = c === 2 ? 0 : p(v, c) - spine;
          shapes[o + 8 + c] = mesh.normals[v * 4 + c] / 32767;
          shapes[o + 12 + c] = linear(mesh.colors[v * 4 + c]) / mean[c];
        }
        shapes[o + 3] = mesh.uvs[v * 2 + 1];
        shapes[o + 7] = mesh.uvs[v * 2];
        shapes[o + 15] = mesh.colors[v * 4 + 3] / 255;
        at++;
      }
    });
  });
  return { shapes, rows, bases, maxBlades };
}

/** The window of world tiles a frame grows grass over. */
export interface GrassWindow {
  /** The south-west corner of tile (0, 0), on the tile grid. */
  x0: number;
  y0: number;
  tilesX: number;
  tilesY: number;
  /** The farthest a clump stands from the eye. */
  reach: number;
}

export function createGrassWindow(): GrassWindow {
  return { x0: 0, y0: 0, tilesX: 0, tilesY: 0, reach: 0 };
}

/** Metres one pixel spans per metre of distance from the eye. */
export function metresPerPixel(fovY: number, heightPx: number): number {
  return (2 * Math.tan(fovY / 2)) / heightPx;
}

/** Tiles around `eye` within the grass's reach: where one pixel spans less
 *  than the fade's end, down to the lowest ground. No tiles when the eye is
 *  higher than that (the strategic height). */
export function grassWindow(
  out: GrassWindow,
  eye: Vec3,
  lowestGround: number,
  pixelScale: number,
  rules: GrassRules,
): GrassWindow {
  const reach = rules.fade_m_per_px[1] / pixelScale;
  const drop = Math.max(0, eye[2] - lowestGround);
  out.reach = reach;
  if (drop >= reach) {
    out.tilesX = out.tilesY = 0;
    return out;
  }
  const flat = Math.sqrt(reach * reach - drop * drop);
  const lo = (v: number) => Math.floor((v - flat) / GRASS_TILE_M) * GRASS_TILE_M;
  const count = (v: number, from: number) =>
    Math.min(GRASS_MAX_TILES, Math.ceil((v + flat - from) / GRASS_TILE_M));
  out.x0 = lo(eye[0]);
  out.y0 = lo(eye[1]);
  out.tilesX = count(eye[0], out.x0);
  out.tilesY = count(eye[1], out.y0);
  return out;
}

/** The view matrix handed to the side planes' extraction: the view is
 *  already in the view-projection. */
const _grassSides_view = mat4.create();

/** The view's four side planes (inward normals), from its view-projection.
 *  Near and far are left out: reach bounds the field. */
export function grassSides(out: Frustum, viewProj: Mat4): Frustum {
  return frustum.setFromViewProjectionMatrixSides(out, viewProj, _grassSides_view);
}
