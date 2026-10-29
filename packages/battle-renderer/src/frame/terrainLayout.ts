// The terrain's GPU records and its bind group layout: what the ground's
// tables (`static_world::ground::tables`) pack, word for word, and what the
// ground's material reads (`terrainMaterial.ts`).
import { tgpu, d } from "typegpu";
import { ShoreTuning, WATER_LOOKS_MAX, WaterLook } from "./waterLook";

/** The paved kinds a map can hold, a row of the look table each, in the
 *  simulation's tag order (`ground::biome::AREA_KINDS`). */
export const ROAD_KINDS = 4;
/** The deepest the plot split goes (`ground::plots::MAX_DEPTH`). */
export const MAX_PLOT_DEPTH = 64;
/** How one paved kind is drawn (`biome.roads`). */
export const RoadLook = d.struct({
  /** The surface: linear rgb, roughness. */
  core: d.vec4f,
  /** What its patches wear toward (linear rgb, at the surface's luminance),
   *  and how far a patch goes to it. */
  worn: d.vec4f,
  /** Edge feather metres, 1 / patch size, grain strength, 1 / grain size. */
  shape: d.vec4f,
  /** How far this kind's road is carried onto a road drawn over it, metres;
   *  then unused; its walk's width in metres (0 for none) and the row that
   *  draws the walk. */
  join: d.vec4f,
  /** The worn ground beside it: linear rgb, and its width in metres. */
  shoulder: d.vec4f,
  /** The shoulder's outer edge: the share of its width it wanders in by,
   *  1 / the wander's size; then the share of the field's grass that still
   *  grows where the ground is most worn, and how far the ground goes to the
   *  shoulder's colour there. */
  edge: d.vec4f,
  /** Its ruts: two distances from a stroke's centreline (0 for none), a
   *  rut's width, the steepest slope of a rut's side (rise over run). */
  ruts: d.vec4f,
  /** How far a rut darkens the surface at its middle; a centre strip's half
   *  width, and the widest stroke (as a half width) that has one; then 1
   *  where the kind is a carriageway (paving is not). */
  track: d.vec4f,
  /** Its walk's slabs: 1 / a slab's length along the road (0 for none), how
   *  far a joint darkens the surface; then its curb's face: its width, and
   *  its slope (rise over run). */
  slabs: d.vec4f,
  /** Its curb's stones: linear rgb, and their width in metres (0 for none). */
  curb: d.vec4f,
  /** Its painted lines: the paint (linear rgb) and how much of the surface
   *  it hides where it is whole. */
  paint: d.vec4f,
  /** The centre line's half width in metres (0 for no markings), a dash's
   *  length, a dash and its gap together, and the share of its cover the
   *  paint loses where it is worn. */
  marks: d.vec4f,
  /** A crossing's bars: a bar's width, its length along the road, how far
   *  from the crossing road's edge it starts and how far it keeps from its
   *  own road's edge. */
  crossing: d.vec4f,
  /** Its kerbstones: 1 / a stone's length along the road, and how far the
   *  joint between two darkens them; then its slabs where it is laid as an
   *  area: 1 / a slab's side (0 for none), how far a joint darkens it. */
  stones: d.vec4f,
});

export const TerrainParams = d.struct({
  /** The plot region: minX, minY, maxX, maxY. */
  region: d.vec4f,
  /** The surface field's grid (`terrain/surfaceField.ts`): its low corner,
   *  1 / the finest cell's side, the widest pixel the finest level serves. */
  field: d.vec4f,
  /** The finest level's cells across and up, and the levels. */
  fieldGrid: d.vec4u,
  /** Edge warp metres, 1 / warp scale, 1 / the mottle's scale, then unused. */
  shape: d.vec4f,
  /** Linear rgb, verge half width. */
  verge: d.vec4f,
  /** Verge feather, the pixel footprints (metres) over which plots give way
   *  to the distant colour, then unused. */
  feathers: d.vec4f,
  /** Each paved kind's look, by its tag. */
  roads: d.arrayOf(RoadLook, ROAD_KINDS),
  /** The rows' tags in the order they are painted, the lowest layer first. */
  roadOrder: d.vec4u,
  /** The forest floor's leaf litter, moss and humus (linear rgb); with the
   *  moss and the humus, how far each takes the litter over. */
  forestLitter: d.vec4f,
  forestMoss: d.vec4f,
  forestHumus: d.vec4f,
  /** 1 / patch scale, mottle, roughness, root contrast. */
  forestDetail: d.vec4f,
  /** Verge width, verge warp, 1 / verge warp scale, 1 / root spacing. */
  forestVerge: d.vec4f,
  /** 1 / fleck size, fleck share, the sun a fleck lets through, unused. */
  forestDapple: d.vec4f,
  /** A tree line's band: the length it narrows to a point over before each
   *  end of its strip, how far its edge wanders, then unused. */
  forestLine: d.vec4f,
  /** The water bed's colour (linear rgb), and how far past a bank's top a
   *  ground triangle can carry its tilt, in metres: a triangle's diagonal. */
  waterBed: d.vec4f,
  /** The wet bank's silt (linear rgb) and its width in metres. */
  shore: d.vec4f,
  /** The bare earth behind it (linear rgb) and how far from the water it ends. */
  shoreEarth: d.vec4f,
  /** How far in the earth's line wanders (a share of its reach), 1 / the
   *  wander's scale, the bands' lift over the ground's luminance, and the
   *  share of a bank's slope its shading shows. */
  shoreEdge: d.vec4f,
  distant: d.vec4f,
  shoreTuning: ShoreTuning,
  /** The biome's water looks, by a cell's look (`mesh::water`). */
  waterLooks: d.arrayOf(WaterLook, WATER_LOOKS_MAX),
  /** 1 draws the ground's classes (`groundClasses`) in place of its lit
   *  colour: the "ground-classes" frame view. Then 1 draws the roads plain
   *  (`setRoadWear`). Then 1 keeps the water's depth colour alone (the
   *  tier's `WaterDetail`). Then unused. */
  view: d.vec4u,
});
/** The scar texture's grid and the biome's scar look (`biome.scars`). */
export const ScarParams = d.struct({
  /** 1 / the grid's width and depth in metres, the cell's side, 1 when there
   *  is ground to draw (else 0). */
  grid: d.vec4f,
  /** Grid axes, sparse directory mask and reserved word. */
  pages: d.vec4u,
  /** Half-open world region for a bounded scar draw; zero span disables clipping. */
  clip: d.vec4f,
  /** Cache bounds in tile coordinates; misses inside use the exact common word. */
  cacheBounds: d.vec4u,
  defaultWord: d.vec4u,
  /** 255 / each channel's `full`: a texel's channel over its full mark. */
  full: d.vec4f,
  /** Linear rgb and strength per channel. */
  crater: d.vec4f,
  /** The soil thrown onto a crater's rim: linear rgb, strength. */
  ejecta: d.vec4f,
  scorch: d.vec4f,
  tracks: d.vec4f,
  trampled: d.vec4f,
  /** Crater relief metres, rim; 0, 0. */
  relief: d.vec4f,
  /** Grass: thin, tracks' share of it, flatten; 0. */
  grass: d.vec4f,
});
export const PlotNode = d.struct({ line: d.vec4f, children: d.vec4i });
export const PlotRecord = d.struct({
  /** Linear rgb, roughness. */
  colour: d.vec4f,
  /** Across the rows (unit), row period metres, row contrast. */
  rows: d.vec4f,
  /** Dry patches' strength, the plot's kind (an index into the biome's
   *  plots), how far the rows break along their length. */
  detail: d.vec4f,
  /** 1 / the grain's size, its contrast, 1 / its stretch along the rows. */
  grain: d.vec4f,
  /** 1 / a dry patch's length along the rows and 1 / its width across them;
   *  then unused. */
  dry: d.vec4f,
  /** Wheelings: one pair every this many rows, half a wheeling's width in
   *  metres, how far it darkens. */
  tram: d.vec4f,
});
export const SurfaceRecord = d.struct({ ends: d.vec4f, detail: d.vec4f });

export const terrainLayout = tgpu.bindGroupLayout({
  params: { uniform: TerrainParams, visibility: ["fragment", "compute"] },
  nodes: {
    storage: (n: number) => d.arrayOf(PlotNode, n),
    access: "readonly",
    visibility: ["fragment", "compute"],
  },
  plots: {
    storage: (n: number) => d.arrayOf(PlotRecord, n),
    access: "readonly",
    visibility: ["fragment", "compute"],
  },
  /** The surface field's records: every paved, forest and water primitive. */
  surfaces: {
    storage: (n: number) => d.arrayOf(SurfaceRecord, n),
    access: "readonly",
    visibility: ["fragment", "compute"],
  },
  /** The surface field's index: per level and cell, which records to read. */
  surfaceIndex: {
    storage: (n: number) => d.arrayOf(d.u32, n),
    access: "readonly",
    visibility: ["fragment", "compute"],
  },
  scarParams: { uniform: ScarParams, visibility: ["fragment", "compute"] },
  /** The side's learned ground (`scarTexture.ts`): crater, scorch, tracks,
   *  trampled per cell. */
  scarPages: { texture: d.texture2d(d.u32), visibility: ["fragment", "compute"] },
  scars: { texture: d.texture2dArray(d.f32), visibility: ["fragment", "compute"] },
  /** The water's normal map (`waterNormals.ts`), and the sampler that
   *  tiles it. */
  waterNormals: { texture: d.texture2d(d.f32), visibility: ["fragment"] },
  waterTiles: { sampler: "filtering", visibility: ["fragment"] },
});

export const NODE_WORDS = 8;
export const PLOT_WORDS = 24;
