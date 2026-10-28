// The grass pass's GPU contract: the field's parameters, a blade shape's
// vertex, and the build's and the draw's bind group layouts.
import { tgpu, d } from "typegpu";
import { GRASS_GROWTH_ROWS, GRASS_MAX_KINDS, GRASS_MIX_MAX } from "../terrain/grassField";
import { Clump } from "./grassClump";

/** The field's numbers, kilobytes of them: a shader reads them through a
 *  pointer (`let P = &….params`). A `let` of the struct itself is copied whole
 *  by every invocation: per blade vertex, that cost 8 to 12 ms a frame. */
export const GrassParams = d
  .struct({
    /** The view's side planes: inward normal, offset. */
    planes: d.arrayOf(d.vec4f, 4),
    /** The eye, and metres one pixel spans per metre of distance. */
    eye: d.vec4f,
    /** Tile (0, 0)'s corner, the tile's side, the tallest clump. */
    window: d.vec4f,
    /** Tiles across and up; the near and far tiers' capacities. */
    grid: d.vec4u,
    /** Pixels per clump, the most clumps a square metre, the fade's footprints. */
    density: d.vec4f,
    /** Near tier's height in pixels, a blade's least width in pixels, the
     *  footprints a clump's own shading softens across. */
    tiers: d.vec4f,
    /** How far that shading softens, how far a blade's own facing lights it,
     *  each clump's brightness variation, 0. */
    shading: d.vec4f,
    /** The bare margin round a forest or water, the farthest a clump moves
     *  to its row; 0, 0. (A prop's margin widens its footprint in the prop
     *  table: `Ground::grass_props`.) */
    clear: d.vec4f,
    /** The height grid's size, 0, 0. */
    counts: d.vec4u,
    /** The height grid's spacing, then 0. */
    ground: d.vec4f,
    /** How grass sways in the wind (`cameraWind`): steady lean, gust
     *  lean, flutter, flutter radians a second. */
    sway: d.vec4f,
    /** Per plot kind (the verge last): density, height scale, grasses mixed,
     *  how closely it keeps to the plot's rows. */
    growth: d.arrayOf(d.vec4f, GRASS_GROWTH_ROWS),
    /** Per plot kind, its patches: lowest and tallest height scale, how far
     *  the sparse ones thin, how far the dry ones dry. */
    patches: d.arrayOf(d.vec4f, GRASS_GROWTH_ROWS),
    /** Per plot kind, its grasses: kind, share, drift, dryness. */
    mixes: d.arrayOf(d.vec4f, GRASS_GROWTH_ROWS * GRASS_MIX_MAX),
    /** 1 / the patches' length scales: height, thinning, drying, drift. */
    patchScales: d.vec4f,
    /** 1 / the tussocks' length scale, how far drying lightens; 0, 0. */
    grain: d.vec4f,
    /** Per kind: height, blades, a blade's root width, 0. */
    kinds: d.arrayOf(d.vec4f, GRASS_MAX_KINDS),
    /** Per kind: the near and far tiers' first shape vertex. */
    bases: d.arrayOf(d.vec4u, GRASS_MAX_KINDS),
  })
  .$name("GrassParams");

export const ShapeVertex = d
  .struct({ spine: d.vec4f, side: d.vec4f, normal: d.vec4f, tint: d.vec4f })
  .$name("GrassShapeVertex");

/** Two indexed-indirect draws' arguments (5 words each), then each tier's
 *  raw count before clamping to capacity. */
export const ARGS_WORDS = 12;

export const grassBuildLayout = tgpu.bindGroupLayout({
  params: { uniform: GrassParams, visibility: ["compute"] },
  heights: {
    storage: (n: number) => d.arrayOf(d.u32, n),
    access: "readonly",
    visibility: ["compute"],
  },
  /** The prop footprints, bucketed by grid cell (`Ground::grass_props`). */
  props: {
    storage: (n: number) => d.arrayOf(d.vec4f, n),
    access: "readonly",
    visibility: ["compute"],
  },
  clumps: {
    storage: (n: number) => d.arrayOf(Clump, n),
    access: "mutable",
    visibility: ["compute"],
  },
  args: {
    storage: d.arrayOf(d.atomic(d.u32), ARGS_WORDS),
    access: "mutable",
    visibility: ["compute"],
  },
});

export const grassDrawLayout = tgpu.bindGroupLayout({
  params: { uniform: GrassParams, visibility: ["vertex"] },
  /** Tier, its first clump, vertices per blade, 0. */
  tier: { uniform: d.vec4u, visibility: ["vertex"] },
  clumps: {
    storage: (n: number) => d.arrayOf(Clump, n),
    access: "readonly",
    visibility: ["vertex"],
  },
  shapes: {
    storage: (n: number) => d.arrayOf(ShapeVertex, n),
    access: "readonly",
    visibility: ["vertex"],
  },
});
