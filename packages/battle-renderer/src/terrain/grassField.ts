// The grass field on the CPU: what the biome and the grass appearances say,
// packed for the GPU, and the per-frame window of world tiles the field is
// grown over. Where clumps stand, how many and how tall is decided per frame
// on the GPU (`frame/grassPass.ts`); nothing here is per clump.
//
// The field is a technique rewrite (reuse manifest) of ~/dev/game's
// grassField.ts and battleGrassResidency.ts, after the Ghost of Tsushima
// talk (research.md): the clumps are regrown on the GPU from world tiles
// whenever the view moves, so there is no CPU residency or upload, and a
// clump's place is a function of its tile alone, so what is drawn never
// reshuffles as the camera moves.
import { mat4, type Mat4, type Vec3 } from "math";
import { frustum, type Frustum } from "math/shapes";
import type { StaticBundle } from "@packages/scene-assets/src/schema.ts";
import type { InstalledAppearances } from "@packages/scene-assets/src/loader.ts";
import { GRASS_SEGMENTS, grassBladeVertices } from "@packages/scene-assets/src/grass.ts";
import { VERGE_GROWTH, type Biome, type GrassRules } from "./biome";

/** World tiles the field is grown over, metres a side. */
export const GRASS_TILE_M = 4;
/** Tiles a side the window may hold (the dispatch's bound). */
export const GRASS_MAX_TILES = 256;
/** Plot kinds the GPU growth table holds; the last row is the verge. */
export const GRASS_GROWTH_ROWS = 16;
/** Grass appearances one field draws. */
export const GRASS_MAX_KINDS = 16;
/** The field's two tiers, near then far: which of a grass kind's LOD tiers
 *  each draws (4 and 2 segments a blade). */
export const FIELD_LODS = [0, 2] as const;

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
   *  scale, appearance index, 0. A plot kind that grows none has density 0. */
  growth: Float32Array;
  /** The appearances the growth names, in index order. */
  appearances: { name: string; bundle: StaticBundle }[];
}

/** Resolve `biome.grass.growth` against `appearances`: null (no grass) until
 *  every appearance it names is installed. */
export function grassKinds(biome: Biome, appearances: GrassAppearances): GrassKinds | null {
  const rules = biome.grass;
  if (biome.plots.length > GRASS_GROWTH_ROWS - 1)
    throw new Error(`grass: at most ${GRASS_GROWTH_ROWS - 1} plot kinds grow grass`);
  const names = [...new Set(Object.values(rules.growth).map((g) => g.appearance))].sort();
  if (names.length > GRASS_MAX_KINDS)
    throw new Error(`grass: at most ${GRASS_MAX_KINDS} grass appearances`);
  if (!names.every((name) => appearances.has(name))) return null;
  const growth = new Float32Array(GRASS_GROWTH_ROWS * 4);
  const row = (at: number, key: string) => {
    const g = rules.growth[key];
    if (!g) return;
    growth.set([g.density, g.height, names.indexOf(g.appearance), 0], at * 4);
  };
  biome.plots.forEach((plot, k) => row(k, plot.name));
  row(GRASS_GROWTH_ROWS - 1, VERGE_GROWTH);
  return { growth, appearances: names.map((name) => ({ name, bundle: appearances.get(name)! })) };
}

/** Floats per clump vertex on the GPU: spine (xyz, height fraction), side
 *  (xyz, phase), normal (xyz, 0), tint (rgb, 0). */
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
          shapes[o + 4 + c] = p(v, c) - spine;
          shapes[o + 8 + c] = mesh.normals[v * 4 + c] / 32767;
          shapes[o + 12 + c] = linear(mesh.colors[v * 4 + c]) / mean[c];
        }
        shapes[o + 3] = mesh.uvs[v * 2 + 1];
        shapes[o + 7] = mesh.uvs[v * 2];
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
