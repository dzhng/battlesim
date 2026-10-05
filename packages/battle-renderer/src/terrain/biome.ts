// The biome: how the ground looks, as data. `fixtures/biomes/summer.json` is
// the one owner of the summer look, read by the terrain material, the grass
// field and the scenery (`trees`: species, forest fill, hedgerows, detail
// tiers); winter is a new file on the same schema.
//
// Ground colours are sRGB display values like every vertex colour, linearised
// once when the terrain surface packs them for the GPU. Tree tints are linear
// multipliers over the tree appearances' own albedo.
import { pick } from "@packages/renderer-core/src/kindTable";
import type { Rgb } from "../light/sceneLight";
import { SURFACE_AREA_KINDS, type SurfaceAreaKind } from "./surfaces";

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
  /** How far a row breaks along its length (0 ruled, 1 into dashes): clods
   *  on a furrow, tufts of stubble, gaps in a drilled row. */
  row_break: number;
  /** The ground's grain: the size in metres of its finest lumps (a clod, a
   *  tussock, a stubble tuft), how far they lighten and darken the albedo as
   *  a fraction, and how many times longer than wide they lie along the
   *  plot's rows. Coarser octaves ride on it, so a field keeps a grain as
   *  the camera pulls out. It leaves the plot's mean colour alone. */
  grain_m: number;
  grain: number;
  grain_stretch: number;
  /** Strength of the dry patches' shift toward ochre, at the plot's own
   *  luminance, and a patch's length along the plot's rows and width across
   *  them in metres: long strips in a drilled crop, blotches in a meadow. */
  mottle: number;
  patch_m: readonly [number, number];
  /** Wheelings, the bare tracks a tractor leaves through a drilled crop: a
   *  pair in every `rows` rows (0 for none), each `width_m` wide and darker
   *  by `contrast`. Thin lines only: nothing broad is darker than its plot. */
  tram: { rows: number; width_m: number; contrast: number };
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
  /** The land's heading, degrees from world +X: it is cut on it down to
   *  tracts no longer than `tract_m` either way. */
  orientation_deg: number;
  /** How far a tract's own grain turns from the land's heading, where no
   *  road gives it one: its plots and their rows keep the tract's grain. */
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
  /** Length scale of the fine value noise over verges, forest floor, shore
   *  and road, in metres. */
  mottle_m: number;
  /** A plot whose centre lies within `yard_m` of a building on the same
   *  side of every carriageway is
   *  `settlement_kind`: a town's yards, a farmstead's. Any other whose centre
   *  lies within `settlement_m` of one is `surround_kind`: the paddocks and
   *  commons round a settlement, where no crop is drilled. */
  settlement_kind: string;
  yard_m: number;
  settlement_m: number;
  surround_kind: string;
}

/** The grass strip along every plot edge. */
export interface Verge {
  palette: string;
  width_m: number;
  /** Width of the blend into the plot either side. */
  feather_m: number;
}

/** The water's edge along every river, in bands by distance from it: wet
 *  silt for `wet_m`, where no grass grows, then bare earth out to `mud_m`,
 *  the grass thickening across it. The earth's outer line wanders in toward
 *  the water by up to `wander` of `mud_m`, over `wander_scale_m`: never out
 *  past `mud_m`, so the water's distance is read no farther. */
export interface Shore {
  /** Two colours: the wet silt, the bare earth. */
  palette: string;
  wet_m: number;
  mud_m: number;
  wander: number;
  wander_scale_m: number;
  /** Both bands are drawn at least this many times as light as the ground
   *  they cover: a shore darker than its field reads as a shadow on it. */
  lift: number;
  /** The share of a bank's true slope its shading shows. In full, the bank
   *  facing away from a low sun goes dark with nothing above it to cast a
   *  shadow. */
  relief: number;
}

/** The water surface's look. */
export interface Water {
  /** How opaque the surface is at its edge, and out in the channel. */
  opacity: readonly [number, number];
  /** How far inside its edge the surface has taken on 63% of the channel's
   *  colour and opacity. */
  shallows_m: number;
  /** How much lighter than the surface its streaks of light are, as a share
   *  of its colour, from every side and under any sun. */
  streak: number;
}

/** One kind of paving's surface, drawn exactly where the simulation's rule
 *  holds: a country road's gravel, a dirt track's packed earth, a town
 *  street's asphalt, a yard's paving. */
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
  /** Where this kind's road runs under a road drawn over it, how far its
   *  surface is carried onto that road, thinning out in drifts; 0 ends it at
   *  that road's edge. */
  join_m: number;
  roughness: number;
  shoulder: Shoulder;
  ruts: Ruts;
  centre_strip: CentreStrip;
  /** Where two kinds' paving overlaps, the higher layer is drawn on top.
   *  Left out, the simulation's own order: 4 for a road, 3 for a country
   *  road, 2 for a dirt track, 1 for paving. */
  layer?: number;
  /** A road through a town: where a settlement's own ground
   *  (`field_rules.settlement_kind`) lies within `beside_m` of a stroke's
   *  edge on both sides, the stroke is drawn by `kind`'s row, and on across
   *  any gap in that ground shorter than `gap_m`. A country road is a street
   *  between a town's houses; the street's surface runs its own `join_m` on
   *  under the road's, either end. Left out, the road is itself everywhere. */
  town?: { kind: string; beside_m: number; gap_m: number };
  /** A town street's walk; left out, the road has none. */
  walk?: Walk;
  /** The curb along its strokes' edges; left out, the road has none. */
  curb?: Curb;
  /** The lines painted on its strokes; left out, the road has none. */
  markings?: Markings;
  /** The joints of the slabs it is laid in as an area (a court, an apron),
   *  square to its outline's dominant bearing (`areaBearings`); left out, an
   *  area has none. */
  slabs?: Slabs;
  /** A region's own finish of this kind: per regional family (the map's
   *  `regional_family`), the fields that differ from this row. A map of a
   *  family with no entry draws the row as it is (`regionalBiome`). */
  families?: Record<string, Partial<Omit<Road, "families">>>;
}

/** An area's slabs: a slab's side, and how far the joint between two darkens
 *  the surface. A joint fades out as it nears a pixel. */
export interface Slabs {
  slab_m: number;
  joint: number;
}

/** A road's painted lines: a dashed line down the middle of each stroke,
 *  and a crossing's bars on it wherever another road crosses it. They are
 *  laid out by the strokes alone, the same on every machine, and fade out
 *  as a line nears a pixel. */
export interface Markings {
  /** A palette of one colour: the paint. */
  palette: string;
  /** How much of the surface the paint hides where it is whole, and the
   *  share of that it loses where it is worn. */
  cover: number;
  wear: number;
  /** The centre line's width, a dash's length and the gap after it. */
  line_m: number;
  dash_m: readonly [number, number];
  crossing: Crossing;
}

/** A crossing's bars, lying along the road they are painted on. */
export interface Crossing {
  /** A bar's width, which is also the gap between two. */
  bar_m: number;
  /** A bar's length along the road. */
  length_m: number;
  /** How far the bars start from the edge of the road that crosses. */
  gap_m: number;
  /** How far they keep from their own road's edge. */
  inset_m: number;
}

/** A curb: a line of kerbstones along a road's edge, and the step up to
 *  them from the roadbed. Shading only: the ground is never moved, so
 *  nothing stands behind it and nothing climbs it. */
export interface Curb {
  /** A palette of one colour: the kerbstones. */
  palette: string;
  /** The stones' width, outward from the road's edge, a stone's length
   *  along it, and how far the joint between two darkens them (0 for none). */
  width_m: number;
  stone_m: number;
  joint: number;
  /** The step's face: how far inside the road's edge it starts, and the
   *  tilt it gives the shading normal there, in degrees: at most 40 (steeper
   *  catches the sky and reads as a sheen). */
  face_m: number;
  tilt_deg: number;
}

/** The paved walk along both sides of a kind's strokes, where a country
 *  road has its shoulder. It is a look, as the shoulder is: units find
 *  ground there, as they do on paving the map names. */
export interface Walk {
  /** The paved kind whose row draws it. */
  kind: string;
  /** Its width, from the road's edge. */
  width_m: number;
  /** Its slabs' length along the road, and how far the joint between two
   *  darkens the surface; 0 for none. A joint fades out as it nears a pixel. */
  slab_m: number;
  joint: number;
}

/** Wheel ruts along a road drawn as a stroke: shading only, the ground is
 *  never moved. They fade to the surface's mean as a rut nears a pixel or
 *  two wide. */
export interface Ruts {
  /** Each rut's distance from the stroke's centreline, mirrored either side:
   *  one or two, or none. A rut the stroke is too narrow to hold is left out. */
  offsets_m: readonly number[];
  width_m: number;
  /** The steepest a rut's side tilts the shading normal, in degrees: at most
   *  15 (steeper catches the sky and reads as a sheen). */
  tilt_deg: number;
  /** How far a rut darkens the surface at its middle. The surface between
   *  the ruts lightens by their share of the road, so the road's mean is its
   *  own and a rut is never darker than the grass beside the road. */
  tint: number;
}

/** The grass strip down the middle of a narrow track. */
export interface CentreStrip {
  /** Half the strip's width; 0 for none. */
  half_width_m: number;
  /** A stroke wider than this has no strip. */
  max_road_width_m: number;
}

/** The worn ground beside a road, between its surface and the field. */
export interface Shoulder {
  /** Its colour: drawn at the luminance of the ground it lies on where that
   *  is the brighter, so it differs from the field by hue alone. `cover` is
   *  how far the ground goes to it where the wear is whole. */
  palette: string;
  cover: number;
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

/** The ground under the simulation's forests: leaf litter drifting into moss
 *  and darker humus, crossed by roots, meeting the field across a ragged verge,
 *  and lit through the canopy in sun flecks. Lengths in metres. */
export interface ForestFloor {
  /** A palette of at least three colours: litter, moss, humus. */
  palette: string;
  /** Size of the moss and humus drifts. */
  patch_m: number;
  /** How far the moss, and the humus, takes the litter over where it lies
   *  thickest. The humus is the darker: a strong one reads as shadow. */
  moss: number;
  humus: number;
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
  /** A strip of forest (a tree line) has no floor of its own: under it lies
   *  a band of the plots' verge, with the verge's grass. The band is the
   *  strip itself, its edge wandering by `warp_m` either way, drawn in to a
   *  point over the last `taper_m` before each end of the strip. */
  tree_line: { taper_m: number; warp_m: number };
  roughness: number;
  /** Sun flecks under the crowns: their size, the share of the floor they
   *  cover, and the share of the sun they let through. */
  dapple: { size_m: number; share: number; sun: number };
  dressing: ForestDressing;
}

/** One kind of forest-floor dressing: a `dressing` appearance, how often it
 *  is scattered, its size and its colour. */
export interface DressingKind {
  /** A catalog appearance whose scenery kind is `dressing`. */
  appearance: string;
  /** Its share of the scattered pieces, against the other kinds' weights. */
  weight: number;
  /** How far it gathers in drifts: 0 stands evenly, 1 only where its own
   *  drift lies. */
  drift: number;
  /** A piece's size as a share of its appearance's own. At most 1: the
   *  appearance is what the asset validator holds under a man's waist
   *  (`fit.dressing`), and nothing may be drawn taller. */
  scale: readonly [number, number];
  /** Linear multiplier over the appearance's own albedo. */
  tint: Rgb;
}

/** What grows and lies on a forest's floor with no body of its own, drawn by
 *  the scenery layer under the trees. Lengths in metres. */
export interface ForestDressing {
  /** Pieces scattered per hectare of forest, before a kind's drifts thin it
   *  and trunks, bodies, paving and water refuse theirs: the knob that cuts
   *  the dressing's cost. */
  per_ha: number;
  kinds: readonly DressingKind[];
  /** The size of a kind's drifts. */
  drift_m: number;
  /** The lowest a piece stands, as a share of its own scaled height: pieces
   *  of one kind differ in build as well as in size. At most 1. */
  squat: number;
  /** A piece's foot keeps this far inside its forest's edge: the floor is
   *  drawn out to a verge that wanders about that edge
   *  (`verge_m`, `verge_warp_m`), and a piece stands on the floor. */
  edge_m: number;
  /** A piece's foot keeps this far from a trunk's axis. */
  trunk_clear_m: number;
  /** A piece's foot keeps this far from a body's footprint, paving and water. */
  clear_m: number;
  /** Per-piece colour variation, as a fraction. */
  colour_jitter: number;
  /** Detail tiers by the projected height, in pixels, of the tallest piece
   *  of a chunk: tier 0 above `lod_px[0]`, tier 1 above `lod_px[1]`, tier 2
   *  above `lod_px[2]`, tier 3 below. */
  lod_px: readonly [number, number, number];
  /** A piece is drawn whole while its projected height is over this many
   *  pixels, and shrinks to nothing as that halves; a chunk whose tallest
   *  piece has gone is left out. */
  fade_px: number;
}

/** One tree species: a `tree` appearance, how often it is drawn, and its colour. */
export interface TreeSpecies {
  /** A catalog appearance whose unit is `tree` (`assets/catalog.json`). */
  appearance: string;
  /** Its share of all trees, against the other species' weights. */
  weight: number;
  /** Linear multiplier over the appearance's own albedo: the season's green. */
  tint: Rgb;
  /** The family whose stands it grows in, with the family's other species
   *  (`stands`). A species of no family is the odd tree among any stand's. */
  family?: string;
  /** It stands only this far inside its forest's edge, metres, and never in
   *  a forest strip or past the map: a bare tree there would draw a gap in
   *  foliage the simulation has. Only a species of no family may have one. */
  interior_m?: number;
}

/** A wood is stands: patches about `size_m` across, each one family's.
 *  `purity` of a stand's family trees are of its own family; the rest are of
 *  any, by weight. */
export interface StandRules {
  size_m: number;
  purity: number;
}

/** How the simulation's trunks are drawn as trees: each crown under its
 *  forest's canopy height and within the canopy's radius of its trunk. */
export interface ForestRules {
  /** Where a crown's top falls, as fractions of the forest's canopy height. */
  top: readonly [number, number];
  /** A tree's width, as a share of its appearance's own: how far the canopy
   *  closes. At most 1: an appearance is built inside the simulation's canopy
   *  radius (`fit.canopy`), and no drawn crown may pass it. Past the map,
   *  where nothing is simulated, it is a tree's width over its height. */
  girth: readonly [number, number];
}

/** A tree the simulation holds outside every forest (a street's tree): a
 *  body with no canopy rule, drawn at its own box. */
export interface LoneTreeRules {
  /** The species it may be drawn as, each the `appearance` of a row of
   *  `species`, whose weight and tint it takes. */
  species: readonly string[];
  /** Where its top falls, as fractions of its body's height. */
  top: readonly [number, number];
  /** Its width, as a share of its appearance's own: a tree beside a street
   *  is narrower than one in a wood. */
  girth: readonly [number, number];
}

/** The shrubs under a tree line (a strip of forest): rows of hedge along its
 *  length, under its trees, so the line looks like what the simulation says
 *  it is, a strip sight does not cross. Every shrub keeps its whole reach
 *  inside the strip, off paving and water and clear of other bodies; a wood
 *  has none (its floor's dressing stays under a man's waist). */
export interface UnderstoreyRules {
  /** A `hedgerow` appearance. */
  appearance: string;
  tint: Rgb;
  /** Shrub spacing along a row, metres. */
  spacing_m: number;
  /** Rows stand this far apart across the strip, as many as fit it. */
  row_m: number;
  /** How far a shrub strays from its row, either way. */
  sway_m: number;
  /** The share of shrubs left out: gaps in the hedge. */
  gap: number;
  /** A shrub's length and width, and its height, over its appearance's own. */
  length: readonly [number, number];
  height: readonly [number, number];
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

/** A felled tree as the side saw it go (`BattleFrame.setFelled`): it tips
 *  over from its stump, gathering speed as it falls, rocks back and lies.
 *  It blocks nothing and gives no cover, so lying it reads as brush: no
 *  taller than `rest_height_m` and its crown a `rest_spread` share of its
 *  width. A stump of its trunk stands where it grew. */
export interface FelledRules {
  /** Seconds from standing to striking the ground. */
  fall_s: number;
  /** Seconds it rocks back and settles after striking it. */
  settle_s: number;
  /** How far it rocks back up, degrees. */
  settle_deg: number;
  /** The highest a lying tree reaches above the ground, metres. */
  rest_height_m: number;
  /** A lying crown's width across its trunk, over its standing width. */
  rest_spread: number;
  /** The stump's height, metres; the tree hinges at its top. */
  stump_height_m: number;
  /** The stump's cut face, linear albedo. */
  cut: Rgb;
}

/** `biome.trees`: species and detail for forests and scenery. */
export interface BiomeTrees {
  species: readonly TreeSpecies[];
  stands: StandRules;
  /** Per-tree colour variation, as a fraction. */
  colour_jitter: number;
  forest: ForestRules;
  lone: LoneTreeRules;
  understorey: UnderstoreyRules;
  hedgerows: HedgerowRules;
  copses: CopseRules;
  felled: FelledRules;
  /** Scenery past the map stands at least `clear_m` outside it and within
   *  `reach_m` of it; the haze hides anything farther. */
  backdrop: { clear_m: number; reach_m: number };
  /** Detail tiers by projected height: tier 0 above `lod_px[0]` pixels,
   *  tier 1 above `lod_px[1]`, tier 2 above `lod_px[2]`, tier 3 below. */
  lod_px: readonly [number, number, number];
}

/** The most grasses one growth row mixes. */
export const GRASS_MIX_MAX = 4;

/** One of the grasses a kind of ground grows. */
export interface GrassSpecies {
  /** A grass appearance in the catalog (`assets/catalog.json`, scenery "grass"). */
  appearance: string;
  /** Its relative share of the row's clumps. */
  share: number;
  /** How far it gathers in drifts of its own instead of spreading evenly:
   *  0 even, 1 found only in its drifts. */
  drift: number;
  /** How far its clumps stand toward dry straw from the ground's colour
   *  (as a dry patch does); 0 for none. */
  dry: number;
}

/** How a stand varies across a field, in world-anchored patches a few
 *  metres across (`GrassRules.patch_m`). Every term is one-sided and none
 *  darkens: a broad darker patch reads as a cloud's shadow. */
export interface GrassPatches {
  /** The stand's height from its lowest patches to its tallest, as scales. */
  height: readonly [number, number];
  /** How far its sparse patches thin: 0 none, 1 bare. */
  thin: number;
  /** How far its dry patches stand toward straw: the ground's hue shifted
   *  and lightened (`dry_lift`), never darkened. */
  dry: number;
}

/** What grows on a plot kind (or the verge), how thick, and how it varies. */
export interface GrassGrowth {
  /** Clumps as a fraction of the densest grass; 0 for none. */
  density: number;
  /** Scales every appearance's own height. */
  height: number;
  /** The grasses, one to `GRASS_MIX_MAX`: each clump is one of them, by share. */
  mix: readonly GrassSpecies[];
  patches: GrassPatches;
  /** How closely the clumps keep to the plot kind's drill rows (its
   *  `furrow_m`, the rows the ground itself is painted with): 0 scattered,
   *  1 on the row. A drilled crop reads as rows, a meadow never. */
  rows: number;
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
  /** The length scales of the patches a stand varies in (`GrassPatches`),
   *  of the drifts a kind gathers in, and of the tussocks its grain follows. */
  patch_m: { height: number; thin: number; dry: number; drift: number; grain: number };
  /** About one clump per this many pixels of ground on screen, so a frame
   *  draws a similar number of clumps at any zoom. */
  pixels_per_clump: number;
  /** The most clumps a square metre holds, however near the camera. */
  max_clumps_m2: number;
  /** Clumps shrink away as one pixel grows from the first to the second
   *  footprint (metres per pixel); beyond, the painted ground alone. */
  fade_m_per_px: readonly [number, number];
  /** A clump's own shading (dark roots, pale tips, each blade's facing)
   *  gives way to the ground's as one pixel grows from the first to the
   *  second footprint: seen from far a tuft shows its tops, and drawn with
   *  its roots it reads as dark flecks. */
  soften_m_per_px: readonly [number, number];
  /** How far it gives way: 1 leaves a far clump its one colour, lit as the
   *  ground; less keeps that share of its own shading at any distance. */
  soften: number;
  /** How far a blade's own facing lights it, against the ground's under it:
   *  0 lights every blade as the ground, 1 by its own normal (which
   *  glitters). */
  blade_facing: number;
  /** Each clump's brightness varies by up to this fraction either way, half
   *  on its own and half with the tussock it stands in (`patch_m.grain`):
   *  the grain a field keeps when it is too far to show blades. */
  clump_value: number;
  /** A wholly dry clump is this much lighter than the ground under it: straw
   *  is paler than green grass, and a dry patch held to the ground's own
   *  luminance reads as rust. */
  dry_lift: number;
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
   *  floor name, `water_bed`, `water` (the surface's own colour: out in the
   *  channel, then at its edge) and `distant` (the land past the patchwork)
   *  are required. */
  palettes: Record<string, readonly Rgb[]>;
  plots: readonly PlotKind[];
  field_rules: FieldRules;
  verge: Verge;
  /** A row per paved kind a map can hold (`SURFACE_AREA_KINDS`); a kind with
   *  no row takes `default`, the country road's. */
  roads: Record<string, Road>;
  shore: Shore;
  water: Water;
  forest_floor: ForestFloor;
  trees: BiomeTrees;
  grass: GrassRules;
  scars: BiomeScars;
}

/** The biome as a map of regional family `family` draws it: each paved
 *  row with that family's own fields over it (`Road.families`). The one
 *  place a map's region reaches the ground's look. */
export function regionalBiome(biome: Biome, family: string | null): Biome {
  const roads = Object.fromEntries(
    Object.entries(biome.roads).map(([kind, row]) => {
      const own = family === null ? undefined : row.families?.[family];
      return [kind, own ? { ...row, ...own } : row];
    }),
  );
  return { ...biome, roads };
}

/** The authored paved kind selects its look row, or the biome default.
 *  A country road's promotion through built ground belongs to that row. */
export const roadRow = (biome: Pick<Biome, "roads">, kind: SurfaceAreaKind): Road =>
  pick(biome.roads, kind);

/** The verge's key in `grass.growth`, beside the plot kinds. */
export const VERGE_GROWTH = "verge";

export const REQUIRED_PALETTES = ["water_bed", "water", "distant"] as const;

/** A palette colour as the terrain packs it for the GPU: linear rgb. */
export const linearRgb = (c: Rgb): [number, number, number] => [
  c[0] ** 2.2,
  c[1] ** 2.2,
  c[2] ** 2.2,
];

/** A plot's colour channels vary by this share of `colour_jitter`, each on
 *  its own, under the jitter of its value. */
export const PLOT_HUE_JITTER = 0.4;

/** No plot's ground is drawn darker than this, as CIELAB L* of its albedo:
 *  its palette's darkest colour at the low end of the per-plot jitter, its
 *  rows at their mean. It is how dark the ground the fog styles were tuned
 *  on gets: a style dims unseen ground to about half, so a sunlit field
 *  darker than this comes as dark as ordinary ground under fog and reads as
 *  unseen. A palette answers to the floor, never `light.shadow_floor` to a
 *  palette (SG4). */
export const PLOT_MIN_LSTAR = 24;

/** The CIELAB L* of the darkest ground a plot of `kind` is drawn with. */
export function darkestPlot(biome: Biome, kind: PlotKind): number {
  const jitter = biome.field_rules.colour_jitter;
  const low = (1 - jitter) * (1 - PLOT_HUE_JITTER * jitter);
  const rows = 1 - kind.furrow_contrast / 2;
  return Math.min(
    ...biome.palettes[kind.palette].map((colour) => {
      const [r, g, b] = linearRgb(colour.map((ch) => ch * low) as unknown as Rgb);
      return 116 * Math.cbrt((0.2126 * r + 0.7152 * g + 0.0722 * b) * rows) - 16;
    }),
  );
}

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
    within(`${at}.row_break`, p.row_break, 0, 1);
    within(`${at}.grain_m`, p.grain_m, 0.05, 20);
    within(`${at}.grain`, p.grain, 0, 0.5);
    within(`${at}.grain_stretch`, p.grain_stretch, 1, 32);
    within(`${at}.mottle`, p.mottle, 0, 1);
    if (!Array.isArray(p.patch_m) || p.patch_m.length !== 2)
      bad(`${at}.patch_m`, "must be [along, across]");
    within(`${at}.patch_m[0]`, p.patch_m[0], 0.5, 1000);
    within(`${at}.patch_m[1]`, p.patch_m[1], 0.5, 1000);
    const tram = p.tram?.rows;
    if (!Number.isInteger(tram) || tram < 0 || (tram > 0 && tram < 4))
      bad(`${at}.tram.rows`, "must be 0, or a whole number of rows from 4 up");
    if (p.tram.rows > 0 && p.furrow_m <= 0) bad(`${at}.tram.rows`, "needs rows (furrow_m)");
    // A wheeling is a furrow laid bare, never wider than its row.
    within(`${at}.tram.width_m`, p.tram.width_m, 0, Math.max(p.furrow_m, 0));
    within(`${at}.tram.contrast`, p.tram.contrast, 0, 0.6);
    within(`${at}.roughness`, p.roughness, 0, 1);
  });
  within("field_rules.colour_jitter", biome.field_rules.colour_jitter, 0, 0.5);
  biome.plots.forEach((p, i) => {
    const lstar = darkestPlot(biome, p);
    if (!(lstar >= PLOT_MIN_LSTAR))
      bad(
        `plots[${i}].palette`,
        `draws "${p.name}" as dark as L* ${lstar.toFixed(1)}, under the floor of ${PLOT_MIN_LSTAR}`,
      );
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
  within("field_rules.mottle_m", r.mottle_m, 0.1, 10000);
  within("field_rules.settlement_m", r.settlement_m, 0, 10000);
  within("field_rules.yard_m", r.yard_m, 0, r.settlement_m);
  for (const key of ["settlement_kind", "surround_kind"] as const) {
    const kind = biome.plots.find((p) => p.name === r[key]);
    if (!kind) bad(`field_rules.${key}`, `names no plot kind "${r[key]}"`);
    else if (kind.furrow_m > 0) bad(`field_rules.${key}`, `"${r[key]}" is a drilled crop`);
  }
  palette("verge.palette", biome.verge.palette);
  within("verge.width_m", biome.verge.width_m, 0, 50);
  within("verge.feather_m", biome.verge.feather_m, 0, 50);
  if (!biome.roads?.default) bad("roads", "needs a default");
  // Each row, and each region's finish of it as the row it makes.
  const rows = Object.entries(biome.roads).flatMap(([kind, road]) => [
    [`roads.${kind}`, kind, road] as const,
    ...Object.entries(road.families ?? {}).map(
      ([family, own]) => [`roads.${kind}.families.${family}`, kind, { ...road, ...own }] as const,
    ),
  ]);
  for (const [at, kind, road] of rows) {
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
    within(`${at}.shoulder.cover`, shoulder.cover, 0, 1);
    within(`${at}.shoulder.width_m`, shoulder.width_m, 0, 8);
    within(`${at}.shoulder.jitter`, shoulder.jitter, 0, 1);
    within(`${at}.shoulder.jitter_m`, shoulder.jitter_m, 0.1, 1000);
    within(`${at}.shoulder.grass`, shoulder.grass, 0, 1);
    const ruts = road.ruts;
    if (!ruts || !Array.isArray(ruts.offsets_m) || ruts.offsets_m.length > 2)
      bad(`${at}.ruts.offsets_m`, "must list at most two distances");
    ruts.offsets_m.forEach((o, i) => within(`${at}.ruts.offsets_m[${i}]`, o, 0.05, 50));
    within(`${at}.ruts.width_m`, ruts.width_m, 0, 3);
    within(`${at}.ruts.tilt_deg`, ruts.tilt_deg, 0, 15);
    within(`${at}.ruts.tint`, ruts.tint, 0, 0.5);
    within(`${at}.centre_strip.half_width_m`, road.centre_strip?.half_width_m, 0, 5);
    within(`${at}.centre_strip.max_road_width_m`, road.centre_strip?.max_road_width_m, 0, 100);
    const paved = (path: string, row: string) => {
      if (!(SURFACE_AREA_KINDS as readonly string[]).includes(row))
        bad(path, `names no paved kind "${row}"`);
    };
    if (road.layer !== undefined) within(`${at}.layer`, road.layer, 0, 100);
    if (road.town) {
      paved(`${at}.town.kind`, road.town.kind);
      within(`${at}.town.beside_m`, road.town.beside_m, 0, 100);
      within(`${at}.town.gap_m`, road.town.gap_m, 0, 10000);
    }
    if (road.walk) {
      paved(`${at}.walk.kind`, road.walk.kind);
      within(`${at}.walk.width_m`, road.walk.width_m, 0, 8);
      within(`${at}.walk.slab_m`, road.walk.slab_m, 0, 100);
      within(`${at}.walk.joint`, road.walk.joint, 0, 0.5);
    }
    if (road.curb) {
      palette(`${at}.curb.palette`, road.curb.palette);
      within(`${at}.curb.width_m`, road.curb.width_m, 0.05, 2);
      within(`${at}.curb.stone_m`, road.curb.stone_m, 0.1, 100);
      within(`${at}.curb.joint`, road.curb.joint, 0, 0.5);
      within(`${at}.curb.face_m`, road.curb.face_m, 0.02, 2);
      within(`${at}.curb.tilt_deg`, road.curb.tilt_deg, 0, 40);
    }
    if (road.slabs) {
      within(`${at}.slabs.slab_m`, road.slabs.slab_m, 0.2, 20);
      within(`${at}.slabs.joint`, road.slabs.joint, 0, 0.5);
    }
    if (road.markings) {
      const marks = road.markings;
      palette(`${at}.markings.palette`, marks.palette);
      within(`${at}.markings.cover`, marks.cover, 0, 1);
      within(`${at}.markings.wear`, marks.wear, 0, 1);
      within(`${at}.markings.line_m`, marks.line_m, 0.05, 1);
      if (!Array.isArray(marks.dash_m) || marks.dash_m.length !== 2)
        bad(`${at}.markings.dash_m`, "must be [dash, gap]");
      within(`${at}.markings.dash_m[0]`, marks.dash_m[0], 0.5, 50);
      within(`${at}.markings.dash_m[1]`, marks.dash_m[1], 0, 50);
      const crossing = marks.crossing;
      if (!crossing || typeof crossing !== "object") bad(`${at}.markings.crossing`, "is missing");
      within(`${at}.markings.crossing.bar_m`, crossing.bar_m, 0.1, 3);
      within(`${at}.markings.crossing.length_m`, crossing.length_m, 0, 8);
      within(`${at}.markings.crossing.gap_m`, crossing.gap_m, 0, 8);
      within(`${at}.markings.crossing.inset_m`, crossing.inset_m, 0, 5);
    }
  }
  if (!biome.shore || typeof biome.shore !== "object") bad("shore", "is missing");
  const shore = biome.shore;
  palette("shore.palette", shore.palette);
  if (biome.palettes[shore.palette].length < 2)
    bad("shore.palette", "needs two colours: wet silt, bare earth");
  within("shore.wet_m", shore.wet_m, 0, 20);
  within("shore.wander", shore.wander, 0, 0.9);
  // The earth's line never wanders in over the wet bank.
  within("shore.mud_m", shore.mud_m, shore.wet_m / (1 - shore.wander), 20);
  within("shore.wander_scale_m", shore.wander_scale_m, 0.5, 1000);
  within("shore.lift", shore.lift, 1, 3);
  within("shore.relief", shore.relief, 0, 1);
  if (!biome.water || typeof biome.water !== "object") bad("water", "is missing");
  if (biome.palettes.water.length < 2)
    bad("palettes.water", "needs two colours: the channel, the water's edge");
  range("water.opacity", biome.water.opacity, 0, 1);
  within("water.shallows_m", biome.water.shallows_m, 0.05, 50);
  within("water.streak", biome.water.streak, 0, 1);
  const f = biome.forest_floor;
  if (!f || typeof f !== "object") bad("forest_floor", "is missing");
  palette("forest_floor.palette", f.palette);
  if (biome.palettes[f.palette].length < 3)
    bad("forest_floor.palette", "needs three colours: litter, moss, humus");
  within("forest_floor.patch_m", f.patch_m, 0.1, 1000);
  within("forest_floor.moss", f.moss, 0, 1);
  within("forest_floor.humus", f.humus, 0, 1);
  within("forest_floor.mottle", f.mottle, 0, 1);
  within("forest_floor.roots", f.roots, 0, 1);
  within("forest_floor.roots_m", f.roots_m, 0.1, 100);
  within("forest_floor.verge_m", f.verge_m, 0, 50);
  within("forest_floor.verge_warp_m", f.verge_warp_m, 0, 50);
  within("forest_floor.verge_warp_scale_m", f.verge_warp_scale_m, 1, 10000);
  within("forest_floor.tree_line.taper_m", f.tree_line?.taper_m, 0.1, 1000);
  // The band's edge wanders within what the surface field reads of a forest.
  within("forest_floor.tree_line.warp_m", f.tree_line.warp_m, 0, f.verge_m + f.verge_warp_m);
  within("forest_floor.roughness", f.roughness, 0, 1);
  within("forest_floor.dapple.size_m", f.dapple?.size_m, 0.1, 100);
  within("forest_floor.dapple.share", f.dapple?.share, 0, 1);
  within("forest_floor.dapple.sun", f.dapple?.sun, 0, 1);
  const tint = (path: string, c: readonly number[]) => {
    if (!Array.isArray(c) || c.length !== 3) bad(path, "must be [r, g, b]");
    c.forEach((v, ch) => within(`${path}[${ch}]`, v, 0, 4));
  };
  const dressing = f.dressing;
  if (!dressing || !Array.isArray(dressing.kinds))
    bad("forest_floor.dressing", "is missing its kinds");
  within("forest_floor.dressing.per_ha", dressing.per_ha, 0, 20000);
  if (dressing.per_ha > 0 && !dressing.kinds.some((k) => k.weight > 0))
    bad("forest_floor.dressing.kinds", "no kind has a weight");
  dressing.kinds.forEach((k, i) => {
    const row = `forest_floor.dressing.kinds[${i}]`;
    if (!k.appearance) bad(`${row}.appearance`, "is empty");
    within(`${row}.weight`, k.weight, 0, 1000);
    within(`${row}.drift`, k.drift, 0, 1);
    range(`${row}.scale`, k.scale, 0.1, 1);
    tint(`${row}.tint`, k.tint);
  });
  within("forest_floor.dressing.drift_m", dressing.drift_m, 1, 10000);
  within("forest_floor.dressing.squat", dressing.squat, 0.1, 1);
  within("forest_floor.dressing.edge_m", dressing.edge_m, 0, 100);
  within("forest_floor.dressing.trunk_clear_m", dressing.trunk_clear_m, 0, 100);
  within("forest_floor.dressing.clear_m", dressing.clear_m, 0, 100);
  within("forest_floor.dressing.colour_jitter", dressing.colour_jitter, 0, 0.5);
  within("forest_floor.dressing.fade_px", dressing.fade_px, 0.1, 10000);
  const dpx = dressing.lod_px;
  if (!Array.isArray(dpx) || dpx.length !== 3)
    bad("forest_floor.dressing.lod_px", "must be three thresholds");
  within("forest_floor.dressing.lod_px[2]", dpx[2], dressing.fade_px / 2, 10000);
  within("forest_floor.dressing.lod_px[1]", dpx[1], dpx[2], 10000);
  within("forest_floor.dressing.lod_px[0]", dpx[0], dpx[1], 10000);
  const t = biome.trees;
  if (!t || !Array.isArray(t.species) || t.species.length === 0) bad("trees.species", "is empty");
  t.species.forEach((s, i) => {
    if (!s.appearance) bad(`trees.species[${i}].appearance`, "is empty");
    within(`trees.species[${i}].weight`, s.weight, 0, 1000);
    tint(`trees.species[${i}].tint`, s.tint);
    if (s.family !== undefined && !s.family) bad(`trees.species[${i}].family`, "is empty");
    if (s.interior_m !== undefined) {
      within(`trees.species[${i}].interior_m`, s.interior_m, 0, 1000);
      if (s.family !== undefined)
        bad(
          `trees.species[${i}].interior_m`,
          "is for a species of no family: a stand grows to its forest's edge",
        );
    }
  });
  // Every tree a forest's edge, a strip or the backdrop draws is of a family.
  if (!t.species.some((s) => s.weight > 0 && s.family !== undefined))
    bad("trees.species", "no species of a family has a weight");
  within("trees.stands.size_m", t.stands?.size_m, 1, 100000);
  within("trees.stands.purity", t.stands?.purity, 0, 1);
  within("trees.colour_jitter", t.colour_jitter, 0, 0.5);
  range("trees.forest.top", t.forest.top, 0.1, 1);
  range("trees.forest.girth", t.forest.girth, 0.3, 1);
  if (!Array.isArray(t.lone?.species) || t.lone.species.length === 0)
    bad("trees.lone.species", "is empty");
  t.lone.species.forEach((name, i) => {
    if (!t.species.some((s) => s.appearance === name && s.weight > 0))
      bad(`trees.lone.species[${i}]`, `names no weighted species "${name}"`);
  });
  range("trees.lone.top", t.lone.top, 0.1, 1);
  range("trees.lone.girth", t.lone.girth, 0.3, 1);
  const u = t.understorey;
  if (!u?.appearance) bad("trees.understorey.appearance", "is empty");
  tint("trees.understorey.tint", u.tint);
  within("trees.understorey.spacing_m", u.spacing_m, 0.5, 100);
  within("trees.understorey.row_m", u.row_m, 0.5, 100);
  within("trees.understorey.sway_m", u.sway_m, 0, 10);
  within("trees.understorey.gap", u.gap, 0, 1);
  range("trees.understorey.length", u.length, 0.1, 2);
  range("trees.understorey.height", u.height, 0.1, 2);
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
  const fell = t.felled;
  if (!fell || typeof fell !== "object") bad("trees.felled", "is missing");
  within("trees.felled.fall_s", fell.fall_s, 0.1, 30);
  within("trees.felled.settle_s", fell.settle_s, 0, 10);
  within("trees.felled.settle_deg", fell.settle_deg, 0, 30);
  within("trees.felled.stump_height_m", fell.stump_height_m, 0.05, 3);
  within("trees.felled.rest_height_m", fell.rest_height_m, fell.stump_height_m + 0.05, 10);
  within("trees.felled.rest_spread", fell.rest_spread, 0.1, 1);
  tint("trees.felled.cut", fell.cut);
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
    within(`${at}.density`, growth.density, 0, 1);
    within(`${at}.height`, growth.height, 0.1, 4);
    if (!Array.isArray(growth.mix) || growth.mix.length < 1 || growth.mix.length > GRASS_MIX_MAX)
      bad(`${at}.mix`, `must name one to ${GRASS_MIX_MAX} grasses`);
    growth.mix.forEach((species, i) => {
      if (!species.appearance) bad(`${at}.mix[${i}].appearance`, "is empty");
      within(`${at}.mix[${i}].share`, species.share, 1e-3, 1000);
      within(`${at}.mix[${i}].drift`, species.drift, 0, 1);
      within(`${at}.mix[${i}].dry`, species.dry, 0, 1);
    });
    within(`${at}.rows`, growth.rows, 0, 1);
    range(`${at}.patches.height`, growth.patches?.height, 0.1, 2);
    within(`${at}.patches.thin`, growth.patches.thin, 0, 1);
    within(`${at}.patches.dry`, growth.patches.dry, 0, 1);
  }
  for (const key of ["height", "thin", "dry", "drift", "grain"] as const)
    within(`grass.patch_m.${key}`, g.patch_m?.[key], 0.5, 1000);
  within("grass.pixels_per_clump", g.pixels_per_clump, 1, 10000);
  within("grass.max_clumps_m2", g.max_clumps_m2, 0.01, 400);
  range("grass.fade_m_per_px", g.fade_m_per_px, 0.001, 10);
  range("grass.soften_m_per_px", g.soften_m_per_px, 0.001, 10);
  within("grass.soften", g.soften, 0, 1);
  within("grass.blade_facing", g.blade_facing, 0, 1);
  within("grass.clump_value", g.clump_value, 0, 0.3);
  within("grass.dry_lift", g.dry_lift, 0, 0.5);
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
