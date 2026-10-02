// The biome: how the ground looks, as data. `fixtures/biomes/summer.json` is
// the one owner of the summer look, read by the terrain material, the grass
// field and the scenery (`trees`: species, forest fill, hedgerows, detail
// tiers); winter is a new file on the same schema.
//
// Ground colours are sRGB display values like every vertex colour, linearised
// once when the terrain surface packs them for the GPU. Tree tints are linear
// multipliers over the tree appearances' own albedo.
import type { Rgb } from "../light/sceneLight";
import { SURFACE_AREA_KINDS } from "./surfaces";

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

/** The water's edge: wet, bare soil on the banks along every river, out to
 *  `width_m` from the water, where no grass grows. */
export interface Shore {
  palette: string;
  width_m: number;
}

/** One kind of road's surface, drawn exactly where the simulation's road rule
 *  holds: a country road's gravel, a dirt track's packed earth. */
export interface Road {
  /** A palette of two colours: the surface, and what patches of it wear
   *  toward. A patch changes the surface's hue, never its brightness. */
  palette: string;
  /** Width of the blend across the road's edge, centred on it. */
  feather_m: number;
  /** How far a patch goes to the second colour, and a patch's size. */
  mottle: number;
  patch_m: number;
  /** Stones and clods: how far the grain lightens and darkens the surface,
   *  and the size of its largest lumps. Each scale of it fades to the
   *  surface's mean as it nears a pixel. */
  grain: number;
  grain_m: number;
  /** Where a road of a later kind joins this one, how far that road's
   *  surface is carried onto it, thinning out. */
  join_m: number;
  roughness: number;
  shoulder: Shoulder;
}

/** The worn ground beside a road, between its surface and the field. */
export interface Shoulder {
  /** Its colour: drawn at the luminance of the ground it lies on where that
   *  is the brighter, so it differs from the field by hue alone. */
  palette: string;
  /** Its width at the widest, from the road's edge; 0 for none. */
  width_m: number;
  /** The share of that width its outer edge wanders inward by, and the size
   *  of the wander. */
  jitter: number;
  jitter_m: number;
  /** The share of the field's grass that still grows where the ground is
   *  most worn, at the road's edge; the grass thins toward it across the
   *  shoulder. */
  grass: number;
}

/** The ground under the simulation's forests: leaf litter with patches of moss
 *  and dark humus, crossed by roots, meeting the field across a ragged verge,
 *  and lit through the canopy in sun flecks. Lengths in metres. */
export interface ForestFloor {
  /** A palette of at least three colours: litter, moss, humus. */
  palette: string;
  /** Size of the moss and humus patches. */
  patch_m: number;
  /** Strength of the ground's value noise over the floor. */
  mottle: number;
  /** How far a root darkens the floor, and the spacing of the roots. */
  roots: number;
  roots_m: number;
  /** Width of the verge where floor and field mix, centred on the rect's
   *  edge, and how far (over what length) its line wanders. */
  verge_m: number;
  verge_warp_m: number;
  verge_warp_scale_m: number;
  roughness: number;
  /** Sun flecks under the crowns: their size, the share of the floor they
   *  cover, and the share of the sun they let through. */
  dapple: { size_m: number; share: number; sun: number };
}

/** One tree species: a `tree` appearance, how often it is drawn, and its colour. */
export interface TreeSpecies {
  /** A catalog appearance whose unit is `tree` (`assets/catalog.json`). */
  appearance: string;
  weight: number;
  /** Linear multiplier over the appearance's own albedo: the season's green. */
  tint: Rgb;
}

/** How the simulation's trunks are drawn as trees: each crown stays in its
 *  forest's rect and under its canopy height. */
export interface ForestRules {
  /** Where a crown's top falls, as fractions of the forest's canopy height. */
  top: readonly [number, number];
  /** Horizontal scale over vertical, per tree (a crown's girth varies more than its height). */
  girth: readonly [number, number];
}

/** Hedgerows along the plot edges past the map: shrubs end to end, with
 *  trees standing in them. Scenery only: nothing is simulated there. */
export interface HedgerowRules {
  /** A `hedgerow` appearance. */
  appearance: string;
  tint: Rgb;
  /** Share of plot edges that carry a hedge. */
  chance: number;
  /** Edges shorter than this carry none. */
  min_edge_m: number;
  /** Shrub spacing along the hedge, metres. */
  spacing_m: number;
  /** A hedge stands this far inside its plot's edge, off the verge. */
  inset_m: number;
  /** Gap between the trees standing in a hedge, metres. */
  tree_gap_m: readonly [number, number];
  /** Tree height scale (over the appearance's own) in hedges and copses. */
  tree_scale: readonly [number, number];
}

/** Small woods past the map: a disc of trees in a plot. */
export interface CopseRules {
  chance: number;
  radius_m: readonly [number, number];
  spacing_m: number;
}

/** `biome.trees`: species and detail for forests and scenery. */
export interface BiomeTrees {
  species: readonly TreeSpecies[];
  /** Per-tree colour variation, as a fraction. */
  colour_jitter: number;
  forest: ForestRules;
  hedgerows: HedgerowRules;
  copses: CopseRules;
  /** Scenery past the map stands at least `clear_m` outside it and within
   *  `reach_m` of it; the haze hides anything farther. */
  backdrop: { clear_m: number; reach_m: number };
  /** Detail tiers by projected height: tier 0 above `lod_px[0]` pixels,
   *  tier 1 above `lod_px[1]`, tier 2 above `lod_px[2]`, tier 3 below. */
  lod_px: readonly [number, number, number];
}

/** Which grass kind grows on a plot kind (or the verge), and how thick. */
export interface GrassGrowth {
  /** A grass appearance in the catalog (`assets/catalog.json`, scenery "grass"). */
  appearance: string;
  /** Clumps as a fraction of the densest grass; 0 for none. */
  density: number;
  /** Scales the appearance's own height. */
  height: number;
}

/** The travelling wind every blade sways in: a steady lean, gust fronts
 *  rolling downwind, and a flutter per blade. */
export interface Wind {
  /** Where the wind blows toward, degrees from world +X. */
  heading_deg: number;
  /** Steady lean of a tip, as a fraction of the blade's height. */
  lean: number;
  /** Extra lean at a gust front's crest, as a fraction of the height. */
  gust: number;
  /** Distance between gust fronts, and their speed downwind. */
  gust_m: number;
  gust_mps: number;
  /** Per-blade flutter: amplitude (fraction of height) and frequency. */
  flutter: number;
  flutter_hz: number;
}

/** The grass field: where clumps grow and how many a frame draws. The
 *  clumps themselves are appearances. Distances in metres. */
export interface GrassRules {
  /** Per plot kind name, and "verge". A plot kind not listed grows none. */
  growth: Record<string, GrassGrowth>;
  /** About one clump per this many pixels of ground on screen, so a frame
   *  draws a similar number of clumps at any zoom. */
  pixels_per_clump: number;
  /** The most clumps a square metre holds, however near the camera. */
  max_clumps_m2: number;
  /** Clumps shrink away as one pixel grows from the first to the second
   *  footprint (metres per pixel); beyond, the painted ground alone. */
  fade_m_per_px: readonly [number, number];
  /** A clump taller than this many pixels draws its near tier. */
  near_tier_px: number;
  /** A blade is drawn at least this many pixels wide, so far blades hold. */
  min_blade_px: number;
  /** Bare margins: none within this of a prop's footprint, or of a forest
   *  or water edge. (Beside a road the grass thins across its shoulder:
   *  `roads.<kind>.shoulder`.) */
  clear_m: { prop: number; area: number };
  wind: Wind;
  /** Clumps the near and far tiers hold at most in one frame. */
  capacity: readonly [number, number];
}

/** How one ground-layer channel marks the ground: toward a palette colour,
 *  reading fully at `full` (the cell's byte, 0–255; the simulation adds a
 *  fixed amount per burst or pass and saturates). */
export interface ScarMark {
  palette: string;
  full: number;
  /** How far the albedo goes to the colour at full weight. */
  strength: number;
}

/** A crater: a bowl `relief_m` deep at full weight (shading only: the
 *  simulation's ground never moves), a raised rim `rim` of that depth
 *  around it, and its soil in the bowl and thrown onto the rim. */
export interface CraterMark extends ScarMark {
  relief_m: number;
  rim: number;
  /** The soil thrown onto the rim (a palette), and how far it colours it. */
  ejecta_palette: string;
  ejecta: number;
}

/** `biome.scars`: the side's learned ground drawn on the terrain, and the
 *  grass's answer to it. Tracks and trampling ease out over their `full`
 *  (one pass already shows), scorch eases in (a small black heart), a
 *  crater's bowl follows its depth. */
export interface BiomeScars {
  crater: CraterMark;
  scorch: ScarMark;
  tracks: ScarMark;
  trampled: ScarMark;
  grass: {
    /** Share of clumps a full crater or scorch leaves out; full tracks
     *  leave out `tracks_thin` of it. */
    thin: number;
    tracks_thin: number;
    /** How far full tracks or trampling lay the grass over, 1 flat. */
    flatten: number;
  };
}

/** The four scar channels, in the ground layer's byte order. */
export const SCAR_CHANNELS = ["crater", "scorch", "tracks", "trampled"] as const;

/** `fixtures/biomes/<name>.json`. */
export interface Biome {
  seed: number;
  /** Named colour lists. Besides the ones plots, verge, roads and the forest
   *  floor name, `water_bed`, `water` (the surface's own colour, over deep
   *  water) and `distant` (the land past the patchwork) are
   *  required. */
  palettes: Record<string, readonly Rgb[]>;
  plots: readonly PlotKind[];
  field_rules: FieldRules;
  verge: Verge;
  /** A row per paved kind a map can hold (`SURFACE_AREA_KINDS`); a kind with
   *  no row takes `default`, the country road's. */
  roads: Record<string, Road>;
  shore: Shore;
  forest_floor: ForestFloor;
  trees: BiomeTrees;
  grass: GrassRules;
  scars: BiomeScars;
}

/** The verge's key in `grass.growth`, beside the plot kinds. */
export const VERGE_GROWTH = "verge";

export const REQUIRED_PALETTES = ["water_bed", "water", "distant"] as const;

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
  if (!biome.roads?.default) bad("roads", "needs a default");
  for (const [kind, road] of Object.entries(biome.roads)) {
    const at = `roads.${kind}`;
    if (kind !== "default" && !(SURFACE_AREA_KINDS as readonly string[]).includes(kind))
      bad(at, "names no paved kind");
    palette(`${at}.palette`, road.palette);
    if (biome.palettes[road.palette].length < 2)
      bad(`${at}.palette`, "needs two colours: the surface and its patches");
    within(`${at}.feather_m`, road.feather_m, 0, 5);
    within(`${at}.mottle`, road.mottle, 0, 1);
    within(`${at}.patch_m`, road.patch_m, 0.1, 1000);
    within(`${at}.grain`, road.grain, 0, 1);
    within(`${at}.grain_m`, road.grain_m, 0.01, 100);
    within(`${at}.join_m`, road.join_m, 0, 20);
    within(`${at}.roughness`, road.roughness, 0, 1);
    const shoulder = road.shoulder;
    if (!shoulder || typeof shoulder !== "object") bad(`${at}.shoulder`, "is missing");
    palette(`${at}.shoulder.palette`, shoulder.palette);
    within(`${at}.shoulder.width_m`, shoulder.width_m, 0, 8);
    within(`${at}.shoulder.jitter`, shoulder.jitter, 0, 1);
    within(`${at}.shoulder.jitter_m`, shoulder.jitter_m, 0.1, 1000);
    within(`${at}.shoulder.grass`, shoulder.grass, 0, 1);
  }
  if (!biome.shore || typeof biome.shore !== "object") bad("shore", "is missing");
  palette("shore.palette", biome.shore.palette);
  within("shore.width_m", biome.shore.width_m, 0, 20);
  const f = biome.forest_floor;
  if (!f || typeof f !== "object") bad("forest_floor", "is missing");
  palette("forest_floor.palette", f.palette);
  if (biome.palettes[f.palette].length < 3)
    bad("forest_floor.palette", "needs three colours: litter, moss, humus");
  within("forest_floor.patch_m", f.patch_m, 0.1, 1000);
  within("forest_floor.mottle", f.mottle, 0, 1);
  within("forest_floor.roots", f.roots, 0, 1);
  within("forest_floor.roots_m", f.roots_m, 0.1, 100);
  within("forest_floor.verge_m", f.verge_m, 0, 50);
  within("forest_floor.verge_warp_m", f.verge_warp_m, 0, 50);
  within("forest_floor.verge_warp_scale_m", f.verge_warp_scale_m, 1, 10000);
  within("forest_floor.roughness", f.roughness, 0, 1);
  within("forest_floor.dapple.size_m", f.dapple?.size_m, 0.1, 100);
  within("forest_floor.dapple.share", f.dapple?.share, 0, 1);
  within("forest_floor.dapple.sun", f.dapple?.sun, 0, 1);
  const t = biome.trees;
  if (!t || !Array.isArray(t.species) || t.species.length === 0) bad("trees.species", "is empty");
  const tint = (path: string, c: readonly number[]) => {
    if (!Array.isArray(c) || c.length !== 3) bad(path, "must be [r, g, b]");
    c.forEach((v, ch) => within(`${path}[${ch}]`, v, 0, 4));
  };
  t.species.forEach((s, i) => {
    if (!s.appearance) bad(`trees.species[${i}].appearance`, "is empty");
    within(`trees.species[${i}].weight`, s.weight, 0, 1000);
    tint(`trees.species[${i}].tint`, s.tint);
  });
  if (!t.species.some((s) => s.weight > 0)) bad("trees.species", "every weight is 0");
  within("trees.colour_jitter", t.colour_jitter, 0, 0.5);
  range("trees.forest.top", t.forest.top, 0.1, 1);
  range("trees.forest.girth", t.forest.girth, 0.5, 2);
  const h = t.hedgerows;
  if (!h.appearance) bad("trees.hedgerows.appearance", "is empty");
  tint("trees.hedgerows.tint", h.tint);
  within("trees.hedgerows.chance", h.chance, 0, 1);
  within("trees.hedgerows.min_edge_m", h.min_edge_m, 0, 10000);
  within("trees.hedgerows.spacing_m", h.spacing_m, 0.5, 100);
  within("trees.hedgerows.inset_m", h.inset_m, 0, 50);
  range("trees.hedgerows.tree_gap_m", h.tree_gap_m, 1, 10000);
  range("trees.hedgerows.tree_scale", h.tree_scale, 0.1, 2);
  within("trees.copses.chance", t.copses.chance, 0, 1);
  range("trees.copses.radius_m", t.copses.radius_m, 1, 1000);
  within("trees.copses.spacing_m", t.copses.spacing_m, 1, 100);
  within("trees.backdrop.clear_m", t.backdrop.clear_m, 0, 1000);
  within("trees.backdrop.reach_m", t.backdrop.reach_m, 0, r.extent_m);
  const px = t.lod_px;
  if (!Array.isArray(px) || px.length !== 3) bad("trees.lod_px", "must be three thresholds");
  within("trees.lod_px[2]", px[2], 1, 10000);
  within("trees.lod_px[1]", px[1], px[2], 10000);
  within("trees.lod_px[0]", px[0], px[1], 10000);
  const g = biome.grass;
  if (!g || typeof g !== "object") bad("grass", "is missing");
  for (const [key, growth] of Object.entries(g.growth ?? {})) {
    const at = `grass.growth.${key}`;
    if (key !== VERGE_GROWTH && !biome.plots.some((p) => p.name === key))
      bad(at, `names no plot kind (or "${VERGE_GROWTH}")`);
    if (!growth.appearance) bad(`${at}.appearance`, "is empty");
    within(`${at}.density`, growth.density, 0, 1);
    within(`${at}.height`, growth.height, 0.1, 4);
  }
  within("grass.pixels_per_clump", g.pixels_per_clump, 1, 10000);
  within("grass.max_clumps_m2", g.max_clumps_m2, 0.01, 400);
  range("grass.fade_m_per_px", g.fade_m_per_px, 0.001, 10);
  within("grass.near_tier_px", g.near_tier_px, 0, 10000);
  within("grass.min_blade_px", g.min_blade_px, 0, 8);
  within("grass.clear_m.prop", g.clear_m?.prop, 0, 20);
  within("grass.clear_m.area", g.clear_m?.area, 0, 20);
  const w = g.wind;
  within("grass.wind.heading_deg", w?.heading_deg, -360, 360);
  within("grass.wind.lean", w?.lean, 0, 1);
  within("grass.wind.gust", w?.gust, 0, 1);
  within("grass.wind.gust_m", w?.gust_m, 1, 10000);
  within("grass.wind.gust_mps", w?.gust_mps, 0, 100);
  within("grass.wind.flutter", w?.flutter, 0, 1);
  within("grass.wind.flutter_hz", w?.flutter_hz, 0, 20);
  if (!Array.isArray(g.capacity) || g.capacity.length !== 2)
    bad("grass.capacity", "must be [near, far]");
  g.capacity.forEach((c, i) => {
    if (!Number.isInteger(c) || c < 1 || c > 4_000_000)
      bad(`grass.capacity[${i}]`, "must be a whole number within [1, 4000000]");
  });
  const sc = biome.scars;
  if (!sc || typeof sc !== "object") bad("scars", "is missing");
  for (const key of SCAR_CHANNELS) {
    const m = sc[key];
    if (!m || typeof m !== "object") bad(`scars.${key}`, "is missing");
    palette(`scars.${key}.palette`, m.palette);
    within(`scars.${key}.full`, m.full, 1, 255);
    within(`scars.${key}.strength`, m.strength, 0, 1);
  }
  within("scars.crater.relief_m", sc.crater.relief_m, 0, 5);
  within("scars.crater.rim", sc.crater.rim, 0, 2);
  palette("scars.crater.ejecta_palette", sc.crater.ejecta_palette);
  within("scars.crater.ejecta", sc.crater.ejecta, 0, 1);
  within("scars.grass.thin", sc.grass?.thin, 0, 1);
  within("scars.grass.tracks_thin", sc.grass?.tracks_thin, 0, 1);
  within("scars.grass.flatten", sc.grass?.flatten, 0, 1);
  return biome;
}
