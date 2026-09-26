// The biome: how the ground looks, as data. `fixtures/biomes/summer.json` is
// the one owner of the summer look, read by the terrain material here and,
// later, by grass and trees; winter is a new file on the same schema.
//
// Colours are sRGB display values like every vertex colour, linearised once
// when the terrain surface packs them for the GPU.
import type { Rgb } from "../light/sceneLight";

/** One kind of plot in the patchwork: a meadow, a crop, ploughed earth. */
export interface PlotKind {
  name: string;
  /** Key into `palettes`: each plot of this kind takes one of its colours. */
  palette: string;
  /** Relative share of plots. */
  weight: number;
  /** Period of the drill rows or furrows across the plot, in metres; 0 for none. */
  furrow_m: number;
  /** How far the rows darken the albedo, as a fraction. */
  furrow_contrast: number;
  /** Strength of the painterly value noise over the plot. */
  mottle: number;
  roughness: number;
}

/** How the patchwork is cut: a seeded binary split of the land into convex
 *  plots, first along the roads, then by size. */
export interface FieldRules {
  /** The patchwork reaches this far past the map edge, over the backdrop. */
  extent_m: number;
  /** A plot stops splitting once its area is under the square of a side drawn from this range. */
  size_m: readonly [number, number];
  /** No cut leaves a plot narrower than this. */
  min_width_m: number;
  /** Chance that a plot is cut along its length into strips instead of across it. */
  strip_chance: number;
  /** The longest plot, as length over width. */
  max_aspect: number;
  /** The patchwork's prevailing heading, degrees from world +X. */
  orientation_deg: number;
  /** How far a large tract (over `tract_m` a side) turns from its parent's heading. */
  orientation_jitter_deg: number;
  tract_m: number;
  /** How far one cut leans off its tract's heading. */
  cut_jitter_deg: number;
  /** Where along the plot a cut falls, as fractions of its length. */
  cut_range: readonly [number, number];
  /** Plot edges wander this far from the straight cut, over this length scale. */
  edge_warp_m: number;
  edge_warp_scale_m: number;
  /** Per-plot variation of the palette colour, as a fraction. */
  colour_jitter: number;
  /** Length scales of the broad and fine value noise, in metres. */
  mottle_scale_m: readonly [number, number];
  /** Plots whose centre lies within this of a building are `settlement_kind`. */
  settlement_m: number;
  settlement_kind: string;
}

/** The grass strip along every plot edge. */
export interface Verge {
  palette: string;
  width_m: number;
  /** Width of the blend into the plot either side. */
  feather_m: number;
}

/** The road surface, drawn exactly where the simulation's road rule holds. */
export interface Road {
  palette: string;
  /** Width of the blend across the road's edge, centred on it. */
  feather_m: number;
  mottle: number;
  roughness: number;
}

/** `fixtures/biomes/<name>.json`. */
export interface Biome {
  seed: number;
  /** Named colour lists. Besides the ones plots, verge and road name,
   *  `forest_floor`, `water_bed` and `distant` (the land past the patchwork)
   *  are required. */
  palettes: Record<string, readonly Rgb[]>;
  plots: readonly PlotKind[];
  field_rules: FieldRules;
  verge: Verge;
  road: Road;
}

export const REQUIRED_PALETTES = ["forest_floor", "water_bed", "distant"] as const;

/** Checks every field the terrain reads; throws naming the first bad one. */
export function validateBiome(biome: Biome, name = "biome"): Biome {
  const bad = (path: string, why: string): never => {
    throw new Error(`${name}.${path}: ${why}`);
  };
  const within = (path: string, v: unknown, lo: number, hi: number) => {
    if (typeof v !== "number" || !Number.isFinite(v)) bad(path, "must be a finite number");
    if (!((v as number) >= lo && (v as number) <= hi))
      bad(path, `must be within [${lo}, ${hi}], got ${v}`);
  };
  const range = (path: string, r: readonly number[], lo: number, hi: number) => {
    if (!Array.isArray(r) || r.length !== 2) bad(path, "must be [low, high]");
    within(`${path}[0]`, r[0], lo, hi);
    within(`${path}[1]`, r[1], r[0], hi);
  };
  const palette = (path: string, key: string) => {
    if (!(key in biome.palettes)) bad(path, `names no palette "${key}"`);
  };
  within("seed", biome.seed, 0, 2 ** 32 - 1);
  for (const [key, colours] of Object.entries(biome.palettes)) {
    if (!Array.isArray(colours) || colours.length === 0) bad(`palettes.${key}`, "is empty");
    colours.forEach((c, i) => {
      if (!Array.isArray(c) || c.length !== 3) bad(`palettes.${key}[${i}]`, "must be [r, g, b]");
      c.forEach((v: number, ch: number) => within(`palettes.${key}[${i}][${ch}]`, v, 0, 1));
    });
  }
  for (const key of REQUIRED_PALETTES) palette(`palettes`, key);
  if (biome.plots.length === 0) bad("plots", "is empty");
  biome.plots.forEach((p, i) => {
    const at = `plots[${i}]`;
    if (!p.name) bad(`${at}.name`, "is empty");
    palette(`${at}.palette`, p.palette);
    within(`${at}.weight`, p.weight, 0, 1000);
    within(`${at}.furrow_m`, p.furrow_m, 0, 100);
    within(`${at}.furrow_contrast`, p.furrow_contrast, 0, 1);
    within(`${at}.mottle`, p.mottle, 0, 1);
    within(`${at}.roughness`, p.roughness, 0, 1);
  });
  if (!biome.plots.some((p) => p.weight > 0)) bad("plots", "every weight is 0");
  const r = biome.field_rules;
  within("field_rules.extent_m", r.extent_m, 0, 20000);
  range("field_rules.size_m", r.size_m, 1, 10000);
  within("field_rules.min_width_m", r.min_width_m, 0.5, r.size_m[0]);
  within("field_rules.strip_chance", r.strip_chance, 0, 1);
  within("field_rules.max_aspect", r.max_aspect, 1, 50);
  within("field_rules.orientation_deg", r.orientation_deg, -360, 360);
  within("field_rules.orientation_jitter_deg", r.orientation_jitter_deg, 0, 90);
  within("field_rules.tract_m", r.tract_m, r.size_m[1], 1e6);
  within("field_rules.cut_jitter_deg", r.cut_jitter_deg, 0, 45);
  range("field_rules.cut_range", r.cut_range, 0.05, 0.95);
  within("field_rules.edge_warp_m", r.edge_warp_m, 0, r.min_width_m / 2);
  within("field_rules.edge_warp_scale_m", r.edge_warp_scale_m, 1, 10000);
  within("field_rules.colour_jitter", r.colour_jitter, 0, 0.5);
  range("field_rules.mottle_scale_m", r.mottle_scale_m, 0.1, 10000);
  within("field_rules.settlement_m", r.settlement_m, 0, 10000);
  if (!biome.plots.some((p) => p.name === r.settlement_kind))
    bad("field_rules.settlement_kind", `names no plot kind "${r.settlement_kind}"`);
  palette("verge.palette", biome.verge.palette);
  within("verge.width_m", biome.verge.width_m, 0, 50);
  within("verge.feather_m", biome.verge.feather_m, 0, 50);
  palette("road.palette", biome.road.palette);
  within("road.feather_m", biome.road.feather_m, 0, 5);
  within("road.mottle", biome.road.mottle, 0, 1);
  within("road.roughness", biome.road.roughness, 0, 1);
  return biome;
}
