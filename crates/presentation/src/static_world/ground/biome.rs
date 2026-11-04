//! The biome: how the ground looks, as data. `fixtures/biomes/summer.json`
//! is the one owner of the summer look, read by the terrain's tables, the
//! grass field and the scenery (`trees`: species, forest fill, hedgerows,
//! detail tiers); winter is a new file on the same schema. A map draws it in
//! its region's finish (`Biome::load`).
//!
//! Ground colours are sRGB display values like every vertex colour,
//! linearised once when the tables pack them for the GPU. Tree tints are
//! linear multipliers over the tree appearances' own albedo. Lengths are
//! metres.
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

use crate::wind::WindSettings;

mod water;
pub use water::*;

pub type Rgb = [f64; 3];
pub type Range = [f64; 2];

/// `fixtures/biomes/<name>.json`.
#[derive(Clone, Deserialize)]
pub struct Biome {
    pub seed: u32,
    /// Named colour lists. Besides the ones plots, verge, roads and the
    /// forest floor name, `water_bed`, `water` (the surface's own colour:
    /// out in the channel, then at its edge) and `distant` (the land past
    /// the patchwork) are required.
    pub palettes: BTreeMap<String, Vec<Rgb>>,
    pub plots: Vec<PlotKind>,
    pub field_rules: FieldRules,
    pub verge: Verge,
    /// A row per paved kind a map can hold (`AREA_KINDS`); a kind with no
    /// row takes `default`, the country road's.
    pub roads: BTreeMap<String, Road>,
    pub shore: Shore,
    pub water: Water,
    pub forest_floor: ForestFloor,
    pub trees: Trees,
    pub grass: GrassRules,
    pub scars: Scars,
    /// The one wind: grass sways in it, smoke and dust drift on it.
    pub wind: WindSettings,
}

/// One kind of plot in the patchwork: a meadow, a crop, ploughed earth.
#[derive(Clone, Deserialize)]
pub struct PlotKind {
    pub name: String,
    /// Key into `palettes`: each plot of this kind takes one of its colours.
    pub palette: String,
    /// Relative share of plots.
    pub weight: f64,
    /// Period of the drill rows or furrows across the plot; 0 for none.
    pub furrow_m: f64,
    /// How far the rows darken the albedo, as a fraction.
    pub furrow_contrast: f64,
    /// How far a row breaks along its length (0 ruled, 1 into dashes).
    pub row_break: f64,
    /// The ground's grain: its finest lumps' size, how far they lighten and
    /// darken the albedo, and how many times longer than wide they lie
    /// along the rows. It leaves the plot's mean colour alone.
    pub grain_m: f64,
    pub grain: f64,
    pub grain_stretch: f64,
    /// Strength of the dry patches' shift toward ochre, and a patch's
    /// length along the rows and width across them.
    pub mottle: f64,
    pub patch_m: Range,
    pub tram: Tram,
    pub roughness: f64,
}

/// Wheelings, the bare tracks a tractor leaves through a drilled crop: a
/// pair in every `rows` rows (0 for none), each `width_m` wide and darker
/// by `contrast`.
#[derive(Clone, Deserialize)]
pub struct Tram {
    pub rows: f64,
    pub width_m: f64,
    pub contrast: f64,
}

/// How the patchwork is cut: a seeded binary split of the land into convex
/// plots, first along the roads, then by size.
#[derive(Clone, Deserialize)]
pub struct FieldRules {
    /// The patchwork reaches this far past the map edge, over the backdrop.
    pub extent_m: f64,
    /// A plot stops splitting once its area is under the square of a side
    /// drawn from this range.
    pub size_m: Range,
    /// No cut leaves a plot narrower than this.
    pub min_width_m: f64,
    /// Chance that a plot is cut along its length into strips.
    pub strip_chance: f64,
    /// The longest plot, as length over width.
    pub max_aspect: f64,
    /// The land's heading, degrees from +x: it is cut on it down to tracts
    /// no longer than `tract_m` either way; a tract's own grain turns from
    /// it by up to `orientation_jitter_deg` where no road gives it one.
    pub orientation_deg: f64,
    pub orientation_jitter_deg: f64,
    pub tract_m: f64,
    /// How far one cut leans off its tract's heading.
    pub cut_jitter_deg: f64,
    /// Where along the plot a cut falls, as fractions of its length.
    pub cut_range: Range,
    /// Plot edges wander this far from the straight cut, over this scale.
    pub edge_warp_m: f64,
    pub edge_warp_scale_m: f64,
    /// Per-plot variation of the palette colour, as a fraction.
    pub colour_jitter: f64,
    /// Length scale of the fine value noise over verges, forest floor,
    /// shore and road.
    pub mottle_m: f64,
    /// A plot whose centre lies within `yard_m` of a building on the same
    /// side of every carriageway is `settlement_kind`; any other within
    /// `settlement_m` of one is `surround_kind`.
    pub settlement_kind: String,
    pub yard_m: f64,
    pub settlement_m: f64,
    pub surround_kind: String,
}

/// The grass strip along every plot edge, and its blend into the plot.
#[derive(Clone, Deserialize)]
pub struct Verge {
    pub palette: String,
    pub width_m: f64,
    pub feather_m: f64,
}

/// One kind of paving's surface, drawn exactly where the simulation's rule
/// holds.
#[derive(Clone, Deserialize)]
pub struct Road {
    /// Two colours: the surface, and what its patches wear toward (a change
    /// of hue, never of brightness).
    pub palette: String,
    /// Width of the blend across the road's edge, centred on it.
    pub feather_m: f64,
    /// How far a patch goes to the second colour, and a patch's size.
    pub mottle: f64,
    pub patch_m: f64,
    /// Stones and clods: their strength and the largest lumps' size.
    pub grain: f64,
    pub grain_m: f64,
    /// Where this kind runs under a road drawn over it, how far its surface
    /// is carried onto that road; 0 ends it at that road's edge.
    pub join_m: f64,
    pub roughness: f64,
    pub shoulder: Shoulder,
    pub ruts: Ruts,
    pub centre_strip: CentreStrip,
    /// Where two kinds overlap, the higher layer is drawn on top; left out,
    /// the simulation's own order (4 road, 3 country road, 2 dirt track,
    /// 1 paving).
    pub layer: Option<f64>,
    /// A road through a town, drawn by another kind's row (`Town`).
    pub town: Option<Town>,
    pub walk: Option<Walk>,
    pub curb: Option<Curb>,
    pub markings: Option<Markings>,
    /// The joints of the slabs it is laid in as an area, square to its
    /// outline's dominant bearing.
    pub slabs: Option<Slabs>,
    /// A region's own finish of this row: per regional family, the fields
    /// that differ (`Biome::load`).
    #[serde(default)]
    pub families: BTreeMap<String, Value>,
}

/// Where a settlement's own ground lies within `beside_m` of a stroke's
/// edge on both sides, the stroke is drawn by `kind`'s row, and on across
/// any gap in that ground shorter than `gap_m`.
#[derive(Clone, Deserialize)]
pub struct Town {
    pub kind: String,
    pub beside_m: f64,
    pub gap_m: f64,
}

/// The worn ground beside a road: its colour (at the ground's luminance
/// where that is brighter), how far the ground goes to it, its widest width
/// from the road's edge, the share and scale its outer edge wanders in by,
/// and the share of grass that still grows at the road's edge.
#[derive(Clone, Deserialize)]
pub struct Shoulder {
    pub palette: String,
    pub cover: f64,
    pub width_m: f64,
    pub jitter: f64,
    pub jitter_m: f64,
    pub grass: f64,
}

/// Wheel ruts, shading only: each rut's distance from the centreline
/// (mirrored, at most two), its width, the steepest tilt of its sides
/// (degrees) and how far it darkens the surface at its middle.
#[derive(Clone, Deserialize)]
pub struct Ruts {
    pub offsets_m: Vec<f64>,
    pub width_m: f64,
    pub tilt_deg: f64,
    pub tint: f64,
}

/// The grass strip down a narrow track: half its width (0 for none), and
/// the widest stroke that has one.
#[derive(Clone, Deserialize)]
pub struct CentreStrip {
    pub half_width_m: f64,
    pub max_road_width_m: f64,
}

/// The paved walk along a street: the kind whose row draws it, its width,
/// its slabs' length and how far their joint darkens (0 for none).
#[derive(Clone, Deserialize)]
pub struct Walk {
    pub kind: String,
    pub width_m: f64,
    pub slab_m: f64,
    pub joint: f64,
}

/// A curb, shading only: the stones' colour, width, length and joint, and
/// the step's face: how far inside the edge it starts and its tilt
/// (degrees, at most 40).
#[derive(Clone, Deserialize)]
pub struct Curb {
    pub palette: String,
    pub width_m: f64,
    pub stone_m: f64,
    pub joint: f64,
    pub face_m: f64,
    pub tilt_deg: f64,
}

/// A road's painted lines: the paint, how much it hides whole and the share
/// it loses worn, the centre line's width, a dash and its gap, and the
/// crossings' bars.
#[derive(Clone, Deserialize)]
pub struct Markings {
    pub palette: String,
    pub cover: f64,
    pub wear: f64,
    pub line_m: f64,
    pub dash_m: Range,
    pub crossing: Crossing,
}

/// A crossing's bars: a bar's width (and the gap between two), its length
/// along the road, how far from the crossing road's edge they start, and
/// how far they keep from their own road's edge.
#[derive(Clone, Deserialize)]
pub struct Crossing {
    pub bar_m: f64,
    pub length_m: f64,
    pub gap_m: f64,
    pub inset_m: f64,
}

/// An area's slabs: a slab's side, and how far a joint darkens the surface.
#[derive(Clone, Deserialize)]
pub struct Slabs {
    pub slab_m: f64,
    pub joint: f64,
}

/// The ground under the simulation's forests: leaf litter drifting into
/// moss and darker humus, crossed by roots, meeting the field across a
/// ragged verge, and lit through the canopy in sun flecks.
#[derive(Clone, Deserialize)]
pub struct ForestFloor {
    /// At least three colours: litter, moss, humus.
    pub palette: String,
    pub patch_m: f64,
    pub moss: f64,
    pub humus: f64,
    pub mottle: f64,
    /// How far a root darkens the floor, and their spacing.
    pub roots: f64,
    pub roots_m: f64,
    /// The verge where floor and field mix, centred on the edge, and how far
    /// (over what length) its line wanders.
    pub verge_m: f64,
    pub verge_warp_m: f64,
    pub verge_warp_scale_m: f64,
    pub tree_line: TreeLine,
    pub roughness: f64,
    pub dapple: Dapple,
    pub dressing: ForestDressing,
}

/// A strip of forest has the plots' verge under it, its edge wandering by
/// `warp_m`, drawn in to a point over the last `taper_m` of each end.
#[derive(Clone, Deserialize)]
pub struct TreeLine {
    pub taper_m: f64,
    pub warp_m: f64,
}

/// Sun flecks under the crowns: their size, share of the floor, and the
/// share of the sun they let through.
#[derive(Clone, Deserialize)]
pub struct Dapple {
    pub size_m: f64,
    pub share: f64,
    pub sun: f64,
}

/// What grows and lies on a forest's floor with no body of its own.
#[derive(Clone, Deserialize)]
pub struct ForestDressing {
    /// Pieces scattered per hectare, before drifts thin them and trunks,
    /// bodies, paving and water refuse theirs.
    pub per_ha: f64,
    pub kinds: Vec<DressingKind>,
    /// The size of a kind's drifts.
    pub drift_m: f64,
    /// The lowest a piece stands, as a share of its scaled height.
    pub squat: f64,
    /// A piece's foot keeps this far inside its forest's edge, from a
    /// trunk's axis, and from a body's footprint, paving and water.
    pub edge_m: f64,
    pub trunk_clear_m: f64,
    pub clear_m: f64,
    pub colour_jitter: f64,
    /// Detail tiers by the projected height of a chunk's tallest piece.
    pub lod_px: [f64; 3],
    /// A piece is whole while its projected height is over this, and
    /// shrinks to nothing as that halves.
    pub fade_px: f64,
}

/// A `dressing` appearance, its share, how far it gathers in drifts, its
/// size as a share of its appearance's own (at most 1) and its tint.
#[derive(Clone, Deserialize)]
pub struct DressingKind {
    pub appearance: String,
    pub weight: f64,
    pub drift: f64,
    pub scale: Range,
    pub tint: Rgb,
}

/// `biome.trees`: species and detail for forests and scenery.
#[derive(Clone, Deserialize)]
pub struct Trees {
    pub species: Vec<TreeSpecies>,
    pub stands: Stands,
    /// Per-tree colour variation, as a fraction.
    pub colour_jitter: f64,
    pub forest: ForestRules,
    pub lone: LoneTrees,
    pub understorey: Understorey,
    pub hedgerows: Hedgerows,
    pub copses: Copses,
    pub felled: Felled,
    pub backdrop: Backdrop,
    /// Detail tiers by projected height: tier 0 above `lod_px[0]` pixels.
    pub lod_px: [f64; 3],
}

/// A `tree` appearance, its share of all trees, its tint; the family whose
/// stands it grows in (none: the odd tree among any stand's), and for a
/// species of no family how far inside its forest's edge it stands.
#[derive(Clone, Deserialize)]
pub struct TreeSpecies {
    pub appearance: String,
    pub weight: f64,
    pub tint: Rgb,
    pub family: Option<String>,
    pub interior_m: Option<f64>,
}

/// A wood is stands about `size_m` across, each one family's; `purity` of
/// a stand's trees are of its family.
#[derive(Clone, Deserialize)]
pub struct Stands {
    pub size_m: f64,
    pub purity: f64,
}

/// Where a crown's top falls (fractions of its forest's canopy), and a
/// tree's width as a share of its appearance's own (at most 1).
#[derive(Clone, Deserialize)]
pub struct ForestRules {
    pub top: Range,
    pub girth: Range,
}

/// A tree outside every forest: the species it may be, its top as
/// fractions of its body's height, and its girth.
#[derive(Clone, Deserialize)]
pub struct LoneTrees {
    pub species: Vec<String>,
    pub top: Range,
    pub girth: Range,
}

/// The shrubs under a tree line, in rows along it: the `hedgerow`
/// appearance and tint, the spacing along and across, how far one strays,
/// the share left out, and its length and height over its appearance's.
#[derive(Clone, Deserialize)]
pub struct Understorey {
    pub appearance: String,
    pub tint: Rgb,
    pub spacing_m: f64,
    pub row_m: f64,
    pub sway_m: f64,
    pub gap: f64,
    pub length: Range,
    pub height: Range,
}

/// Hedgerows along the plot edges past the map, with trees standing in
/// them: their appearance and tint, the share of edges that carry one, the
/// shortest edge that does, spacing, inset, the gap between their trees and
/// those trees' height scale.
#[derive(Clone, Deserialize)]
pub struct Hedgerows {
    pub appearance: String,
    pub tint: Rgb,
    pub chance: f64,
    pub min_edge_m: f64,
    pub spacing_m: f64,
    pub inset_m: f64,
    pub tree_gap_m: Range,
    pub tree_scale: Range,
}

/// Small woods past the map: a disc of trees in a plot.
#[derive(Clone, Deserialize)]
pub struct Copses {
    pub chance: f64,
    pub radius_m: Range,
    pub spacing_m: f64,
}

/// A felled tree as the side saw it go: its fall and settle, how far it
/// rocks back, how high and wide it lies, its stump and the stump's cut.
#[derive(Clone, Deserialize)]
pub struct Felled {
    pub fall_s: f64,
    pub settle_s: f64,
    pub settle_deg: f64,
    pub rest_height_m: f64,
    pub rest_spread: f64,
    pub stump_height_m: f64,
    pub cut: Rgb,
}

/// Scenery past the map stands at least `clear_m` outside it and within
/// `reach_m` of it.
#[derive(Clone, Deserialize)]
pub struct Backdrop {
    pub clear_m: f64,
    pub reach_m: f64,
}

/// The most grasses one growth row mixes.
pub const GRASS_MIX_MAX: usize = 4;
/// The verge's key in `grass.growth`, beside the plot kinds.
pub const VERGE_GROWTH: &str = "verge";

/// The grass field: where clumps grow and how many a frame draws.
#[derive(Clone, Deserialize)]
pub struct GrassRules {
    /// Per plot kind name, and "verge". A kind not listed grows none.
    pub growth: BTreeMap<String, GrassGrowth>,
    pub patch_m: GrassScales,
    /// About one clump per this many pixels of ground on screen.
    pub pixels_per_clump: f64,
    /// The most clumps a square metre holds.
    pub max_clumps_m2: f64,
    /// Clumps shrink away as a pixel grows from the first footprint to the
    /// second (metres per pixel).
    pub fade_m_per_px: Range,
    /// A clump's own shading gives way to the ground's over these
    /// footprints, by `soften` at most.
    pub soften_m_per_px: Range,
    pub soften: f64,
    /// How far a blade's own facing lights it.
    pub blade_facing: f64,
    /// Each clump's brightness varies by up to this either way.
    pub clump_value: f64,
    /// A wholly dry clump is this much lighter than the ground under it.
    pub dry_lift: f64,
    /// A clump taller than this many pixels draws its near tier.
    pub near_tier_px: f64,
    pub min_blade_px: f64,
    /// Bare margins round props, and forest and water edges.
    pub clear_m: GrassClear,
    pub sway: Sway,
    /// Clumps the near and far tiers hold at most in one frame.
    pub capacity: [u32; 2],
}

/// What grows on a plot kind (or the verge), how thick, and how it varies.
#[derive(Clone, Deserialize)]
pub struct GrassGrowth {
    /// Clumps as a fraction of the densest grass.
    pub density: f64,
    /// Scales every appearance's own height.
    pub height: f64,
    /// One to `GRASS_MIX_MAX` grasses.
    pub mix: Vec<GrassSpecies>,
    pub patches: GrassPatches,
    /// How closely the clumps keep to the plot kind's drill rows.
    pub rows: f64,
}

/// A grass appearance, its share, how far it gathers in drifts, and how far
/// its clumps stand toward dry straw.
#[derive(Clone, Deserialize)]
pub struct GrassSpecies {
    pub appearance: String,
    pub share: f64,
    pub drift: f64,
    pub dry: f64,
}

/// How a stand varies: its height range as scales, how far sparse patches
/// thin and dry ones dry.
#[derive(Clone, Deserialize)]
pub struct GrassPatches {
    pub height: Range,
    pub thin: f64,
    pub dry: f64,
}

/// The length scales of the patches, the drifts and the tussocks.
#[derive(Clone, Deserialize)]
pub struct GrassScales {
    pub height: f64,
    pub thin: f64,
    pub dry: f64,
    pub drift: f64,
    pub grain: f64,
}

#[derive(Clone, Deserialize)]
pub struct GrassClear {
    pub prop: f64,
    pub area: f64,
}

/// How grass answers the biome's wind: its steady lean, a gust front's
/// extra lean, and a blade's flutter and its rate.
#[derive(Clone, Deserialize)]
pub struct Sway {
    pub lean: f64,
    pub gust: f64,
    pub flutter: f64,
    pub flutter_hz: f64,
}

/// The side's learned ground on the terrain, and the grass's answer.
#[derive(Clone, Deserialize)]
pub struct Scars {
    pub crater: Crater,
    pub scorch: ScarMark,
    pub tracks: ScarMark,
    pub trampled: ScarMark,
    pub grass: ScarGrass,
}

/// How one channel marks the ground: toward a palette colour, whole at the
/// cell's byte `full`, by `strength`.
#[derive(Clone, Deserialize)]
pub struct ScarMark {
    pub palette: String,
    pub full: f64,
    pub strength: f64,
}

/// A crater: a bowl `relief_m` deep at full weight, a rim `rim` of that,
/// and its soil thrown onto the rim.
#[derive(Clone, Deserialize)]
pub struct Crater {
    #[serde(flatten)]
    pub mark: ScarMark,
    pub relief_m: f64,
    pub rim: f64,
    pub ejecta_palette: String,
    pub ejecta: f64,
}

/// The share of clumps a full crater or scorch leaves out, full tracks'
/// share of it, and how far tracks or trampling lay the grass over.
#[derive(Clone, Deserialize)]
pub struct ScarGrass {
    pub thin: f64,
    pub tracks_thin: f64,
    pub flatten: f64,
}

/// The paved kinds, in the simulation's tag order.
pub const AREA_KINDS: [&str; 4] = ["road", "country_road", "dirt_track", "paving"];
/// The palettes besides those a row names.
const REQUIRED_PALETTES: [&str; 3] = ["water_bed", "water", "distant"];
/// A plot's channels vary by this share of `colour_jitter`, each alone.
pub const PLOT_HUE_JITTER: f64 = 0.4;
/// No plot is drawn darker than this CIELAB L*: a style dims unseen ground
/// to about half, so a sunlit field darker reads as unseen (SG4).
const PLOT_MIN_LSTAR: f64 = 24.0;

/// A palette colour as the GPU reads it: linear rgb.
pub fn linear(c: Rgb) -> [f64; 3] {
    c.map(|v| libm::pow(v, 2.2))
}

impl Biome {
    /// The biome `json` as a map of regional family `family` draws it: each
    /// paved row with that family's own fields over it. The one place a
    /// map's region reaches the ground's look. Every row and every family's
    /// finish of it is checked, then the whole.
    pub fn load(json: &str, family: Option<&str>) -> Result<Biome, String> {
        let mut value: Value = serde_json::from_str(json).map_err(|e| format!("biome: {e}"))?;
        let biome: Biome =
            serde_json::from_value(value.clone()).map_err(|e| format!("biome: {e}"))?;
        for (kind, road) in &biome.roads {
            for family in road.families.keys() {
                let row = finish(&value["roads"][kind], family);
                let road: Road = serde_json::from_value(row)
                    .map_err(|e| format!("biome.roads.{kind}.families.{family}: {e}"))?;
                biome.check_road(&format!("roads.{kind}.families.{family}"), kind, &road)?;
            }
        }
        if let (Some(family), Some(roads)) = (family, value["roads"].as_object_mut()) {
            for row in roads.values_mut() {
                *row = finish(row, family);
            }
        }
        let biome: Biome = serde_json::from_value(value).map_err(|e| format!("biome: {e}"))?;
        biome.check()?;
        Ok(biome)
    }

    /// A paved kind's row, or the default's.
    pub fn road(&self, kind: &str) -> &Road {
        self.roads.get(kind).unwrap_or(&self.roads["default"])
    }

    pub fn palette(&self, key: &str) -> &[Rgb] {
        &self.palettes[key]
    }

    /// The CIELAB L* of the darkest ground a plot of `kind` is drawn with.
    fn darkest(&self, kind: &PlotKind) -> f64 {
        let jitter = self.field_rules.colour_jitter;
        let low = (1.0 - jitter) * (1.0 - PLOT_HUE_JITTER * jitter);
        let rows = 1.0 - kind.furrow_contrast / 2.0;
        self.palettes[&kind.palette]
            .iter()
            .map(|c| {
                let [r, g, b] = linear(c.map(|v| v * low));
                116.0 * libm::cbrt(crate::color::luminance([r, g, b]) * rows) - 16.0
            })
            .fold(f64::INFINITY, f64::min)
    }

    fn has_palette(&self, at: &str, key: &str, least: usize) -> Result<(), String> {
        match self.palettes.get(key) {
            None => bad(at, format!("names no palette \"{key}\"")),
            Some(p) if p.len() < least => bad(at, format!("needs {least} colours")),
            _ => Ok(()),
        }
    }

    /// Checks every number the ground, grass and scenery read; refuses
    /// naming the first bad one.
    fn check(&self) -> Result<(), String> {
        for (key, colours) in &self.palettes {
            if colours.is_empty() {
                bad(&format!("palettes.{key}"), "is empty")?;
            }
            for (i, c) in colours.iter().enumerate() {
                for (ch, &v) in c.iter().enumerate() {
                    within(&format!("palettes.{key}[{i}][{ch}]"), v, 0.0, 1.0)?;
                }
            }
        }
        for key in REQUIRED_PALETTES {
            self.has_palette("palettes", key, 1)?;
        }
        self.has_palette("palettes.water", "water", 2)?;
        let r = &self.field_rules;
        within("field_rules.colour_jitter", r.colour_jitter, 0.0, 0.5)?;
        if !self.plots.iter().any(|p| p.weight > 0.0) {
            bad("plots", "no kind has a weight")?;
        }
        for (i, p) in self.plots.iter().enumerate() {
            let at = |f: &str| format!("plots[{i}].{f}");
            self.has_palette(&at("palette"), &p.palette, 1)?;
            within(&at("weight"), p.weight, 0.0, 1000.0)?;
            within(&at("furrow_m"), p.furrow_m, 0.0, 100.0)?;
            within(&at("furrow_contrast"), p.furrow_contrast, 0.0, 1.0)?;
            within(&at("row_break"), p.row_break, 0.0, 1.0)?;
            within(&at("grain_m"), p.grain_m, 0.05, 20.0)?;
            within(&at("grain"), p.grain, 0.0, 0.5)?;
            within(&at("grain_stretch"), p.grain_stretch, 1.0, 32.0)?;
            within(&at("mottle"), p.mottle, 0.0, 1.0)?;
            within(&at("patch_m[0]"), p.patch_m[0], 0.5, 1000.0)?;
            within(&at("patch_m[1]"), p.patch_m[1], 0.5, 1000.0)?;
            let rows = p.tram.rows;
            if rows.fract() != 0.0 || rows < 0.0 || (rows > 0.0 && rows < 4.0) {
                bad(
                    &at("tram.rows"),
                    "must be 0, or a whole number of rows from 4 up",
                )?;
            }
            if rows > 0.0 && p.furrow_m <= 0.0 {
                bad(&at("tram.rows"), "needs rows (furrow_m)")?;
            }
            // A wheeling is a furrow laid bare, never wider than its row.
            within(
                &at("tram.width_m"),
                p.tram.width_m,
                0.0,
                p.furrow_m.max(0.0),
            )?;
            within(&at("tram.contrast"), p.tram.contrast, 0.0, 0.6)?;
            within(&at("roughness"), p.roughness, 0.0, 1.0)?;
            let lstar = self.darkest(p);
            if lstar.is_nan() || lstar < PLOT_MIN_LSTAR {
                bad(&at("palette"), format!("draws \"{}\" as dark as L* {lstar:.1}, under the floor of {PLOT_MIN_LSTAR}", p.name))?;
            }
        }
        within("field_rules.extent_m", r.extent_m, 0.0, 20000.0)?;
        range("field_rules.size_m", r.size_m, 1.0, 10000.0)?;
        within("field_rules.min_width_m", r.min_width_m, 0.5, r.size_m[0])?;
        within("field_rules.strip_chance", r.strip_chance, 0.0, 1.0)?;
        within("field_rules.max_aspect", r.max_aspect, 1.0, 50.0)?;
        within(
            "field_rules.orientation_deg",
            r.orientation_deg,
            -360.0,
            360.0,
        )?;
        within(
            "field_rules.orientation_jitter_deg",
            r.orientation_jitter_deg,
            0.0,
            90.0,
        )?;
        within("field_rules.tract_m", r.tract_m, r.size_m[1], 1e6)?;
        within("field_rules.cut_jitter_deg", r.cut_jitter_deg, 0.0, 45.0)?;
        range("field_rules.cut_range", r.cut_range, 0.05, 0.95)?;
        within(
            "field_rules.edge_warp_m",
            r.edge_warp_m,
            0.0,
            r.min_width_m / 2.0,
        )?;
        within(
            "field_rules.edge_warp_scale_m",
            r.edge_warp_scale_m,
            1.0,
            10000.0,
        )?;
        within("field_rules.mottle_m", r.mottle_m, 0.1, 10000.0)?;
        within("field_rules.settlement_m", r.settlement_m, 0.0, 10000.0)?;
        within("field_rules.yard_m", r.yard_m, 0.0, r.settlement_m)?;
        for (key, name) in [
            ("settlement_kind", &r.settlement_kind),
            ("surround_kind", &r.surround_kind),
        ] {
            match self.plots.iter().find(|p| &p.name == name) {
                None => bad(
                    &format!("field_rules.{key}"),
                    format!("names no plot kind \"{name}\""),
                )?,
                Some(k) if k.furrow_m > 0.0 => bad(
                    &format!("field_rules.{key}"),
                    format!("\"{name}\" is a drilled crop"),
                )?,
                _ => {}
            }
        }
        self.has_palette("verge.palette", &self.verge.palette, 1)?;
        within("verge.width_m", self.verge.width_m, 0.0, 50.0)?;
        within("verge.feather_m", self.verge.feather_m, 0.0, 50.0)?;
        if !self.roads.contains_key("default") {
            bad("roads", "needs a default")?;
        }
        for (kind, road) in &self.roads {
            self.check_road(&format!("roads.{kind}"), kind, road)?;
        }
        self.check_water()?;
        self.check_floor()?;
        self.check_trees()?;
        self.check_grass()?;
        self.wind.check()?;
        let sc = &self.scars;
        for (key, m) in [
            ("crater", &sc.crater.mark),
            ("scorch", &sc.scorch),
            ("tracks", &sc.tracks),
            ("trampled", &sc.trampled),
        ] {
            self.has_palette(&format!("scars.{key}.palette"), &m.palette, 1)?;
            within(&format!("scars.{key}.full"), m.full, 1.0, 255.0)?;
            within(&format!("scars.{key}.strength"), m.strength, 0.0, 1.0)?;
        }
        within("scars.crater.relief_m", sc.crater.relief_m, 0.0, 5.0)?;
        within("scars.crater.rim", sc.crater.rim, 0.0, 2.0)?;
        self.has_palette("scars.crater.ejecta_palette", &sc.crater.ejecta_palette, 1)?;
        within("scars.crater.ejecta", sc.crater.ejecta, 0.0, 1.0)?;
        within("scars.grass.thin", sc.grass.thin, 0.0, 1.0)?;
        within("scars.grass.tracks_thin", sc.grass.tracks_thin, 0.0, 1.0)?;
        within("scars.grass.flatten", sc.grass.flatten, 0.0, 1.0)
    }

    fn check_road(&self, at: &str, kind: &str, road: &Road) -> Result<(), String> {
        let at = |f: &str| format!("{at}.{f}");
        if kind != "default" && !AREA_KINDS.contains(&kind) {
            bad(&at(""), "names no paved kind")?;
        }
        let paved = |path: &str, row: &str| match AREA_KINDS.contains(&row) {
            true => Ok(()),
            false => bad(path, format!("names no paved kind \"{row}\"")),
        };
        self.has_palette(&at("palette"), &road.palette, 2)?;
        within(&at("feather_m"), road.feather_m, 0.0, 5.0)?;
        within(&at("mottle"), road.mottle, 0.0, 1.0)?;
        within(&at("patch_m"), road.patch_m, 0.1, 1000.0)?;
        within(&at("grain"), road.grain, 0.0, 1.0)?;
        within(&at("grain_m"), road.grain_m, 0.01, 100.0)?;
        within(&at("join_m"), road.join_m, 0.0, 20.0)?;
        within(&at("roughness"), road.roughness, 0.0, 1.0)?;
        let s = &road.shoulder;
        self.has_palette(&at("shoulder.palette"), &s.palette, 1)?;
        within(&at("shoulder.cover"), s.cover, 0.0, 1.0)?;
        within(&at("shoulder.width_m"), s.width_m, 0.0, 8.0)?;
        within(&at("shoulder.jitter"), s.jitter, 0.0, 1.0)?;
        within(&at("shoulder.jitter_m"), s.jitter_m, 0.1, 1000.0)?;
        within(&at("shoulder.grass"), s.grass, 0.0, 1.0)?;
        let ruts = &road.ruts;
        if ruts.offsets_m.len() > 2 {
            bad(&at("ruts.offsets_m"), "must list at most two distances")?;
        }
        for (i, &o) in ruts.offsets_m.iter().enumerate() {
            within(&at(&format!("ruts.offsets_m[{i}]")), o, 0.05, 50.0)?;
        }
        within(&at("ruts.width_m"), ruts.width_m, 0.0, 3.0)?;
        within(&at("ruts.tilt_deg"), ruts.tilt_deg, 0.0, 15.0)?;
        within(&at("ruts.tint"), ruts.tint, 0.0, 0.5)?;
        within(
            &at("centre_strip.half_width_m"),
            road.centre_strip.half_width_m,
            0.0,
            5.0,
        )?;
        within(
            &at("centre_strip.max_road_width_m"),
            road.centre_strip.max_road_width_m,
            0.0,
            100.0,
        )?;
        if let Some(layer) = road.layer {
            within(&at("layer"), layer, 0.0, 100.0)?;
        }
        if let Some(t) = &road.town {
            paved(&at("town.kind"), &t.kind)?;
            within(&at("town.beside_m"), t.beside_m, 0.0, 100.0)?;
            within(&at("town.gap_m"), t.gap_m, 0.0, 10000.0)?;
        }
        if let Some(w) = &road.walk {
            paved(&at("walk.kind"), &w.kind)?;
            within(&at("walk.width_m"), w.width_m, 0.0, 8.0)?;
            within(&at("walk.slab_m"), w.slab_m, 0.0, 100.0)?;
            within(&at("walk.joint"), w.joint, 0.0, 0.5)?;
        }
        if let Some(c) = &road.curb {
            self.has_palette(&at("curb.palette"), &c.palette, 1)?;
            within(&at("curb.width_m"), c.width_m, 0.05, 2.0)?;
            within(&at("curb.stone_m"), c.stone_m, 0.1, 100.0)?;
            within(&at("curb.joint"), c.joint, 0.0, 0.5)?;
            within(&at("curb.face_m"), c.face_m, 0.02, 2.0)?;
            within(&at("curb.tilt_deg"), c.tilt_deg, 0.0, 40.0)?;
        }
        if let Some(s) = &road.slabs {
            within(&at("slabs.slab_m"), s.slab_m, 0.2, 20.0)?;
            within(&at("slabs.joint"), s.joint, 0.0, 0.5)?;
        }
        if let Some(m) = &road.markings {
            self.has_palette(&at("markings.palette"), &m.palette, 1)?;
            within(&at("markings.cover"), m.cover, 0.0, 1.0)?;
            within(&at("markings.wear"), m.wear, 0.0, 1.0)?;
            within(&at("markings.line_m"), m.line_m, 0.05, 1.0)?;
            within(&at("markings.dash_m[0]"), m.dash_m[0], 0.5, 50.0)?;
            within(&at("markings.dash_m[1]"), m.dash_m[1], 0.0, 50.0)?;
            let c = &m.crossing;
            within(&at("markings.crossing.bar_m"), c.bar_m, 0.1, 3.0)?;
            within(&at("markings.crossing.length_m"), c.length_m, 0.0, 8.0)?;
            within(&at("markings.crossing.gap_m"), c.gap_m, 0.0, 8.0)?;
            within(&at("markings.crossing.inset_m"), c.inset_m, 0.0, 5.0)?;
        }
        Ok(())
    }

    fn check_floor(&self) -> Result<(), String> {
        let f = &self.forest_floor;
        self.has_palette("forest_floor.palette", &f.palette, 3)?;
        within("forest_floor.patch_m", f.patch_m, 0.1, 1000.0)?;
        within("forest_floor.moss", f.moss, 0.0, 1.0)?;
        within("forest_floor.humus", f.humus, 0.0, 1.0)?;
        within("forest_floor.mottle", f.mottle, 0.0, 1.0)?;
        within("forest_floor.roots", f.roots, 0.0, 1.0)?;
        within("forest_floor.roots_m", f.roots_m, 0.1, 100.0)?;
        within("forest_floor.verge_m", f.verge_m, 0.0, 50.0)?;
        within("forest_floor.verge_warp_m", f.verge_warp_m, 0.0, 50.0)?;
        within(
            "forest_floor.verge_warp_scale_m",
            f.verge_warp_scale_m,
            1.0,
            10000.0,
        )?;
        within(
            "forest_floor.tree_line.taper_m",
            f.tree_line.taper_m,
            0.1,
            1000.0,
        )?;
        // The band's edge wanders within what the surface field reads of a forest.
        within(
            "forest_floor.tree_line.warp_m",
            f.tree_line.warp_m,
            0.0,
            f.verge_m + f.verge_warp_m,
        )?;
        within("forest_floor.roughness", f.roughness, 0.0, 1.0)?;
        within("forest_floor.dapple.size_m", f.dapple.size_m, 0.1, 100.0)?;
        within("forest_floor.dapple.share", f.dapple.share, 0.0, 1.0)?;
        within("forest_floor.dapple.sun", f.dapple.sun, 0.0, 1.0)?;
        let d = &f.dressing;
        within("forest_floor.dressing.per_ha", d.per_ha, 0.0, 20000.0)?;
        if d.per_ha > 0.0 && !d.kinds.iter().any(|k| k.weight > 0.0) {
            bad("forest_floor.dressing.kinds", "no kind has a weight")?;
        }
        for (i, k) in d.kinds.iter().enumerate() {
            let at = |f: &str| format!("forest_floor.dressing.kinds[{i}].{f}");
            if k.appearance.is_empty() {
                bad(&at("appearance"), "is empty")?;
            }
            within(&at("weight"), k.weight, 0.0, 1000.0)?;
            within(&at("drift"), k.drift, 0.0, 1.0)?;
            range(&at("scale"), k.scale, 0.1, 1.0)?;
            tint(&at("tint"), k.tint)?;
        }
        within("forest_floor.dressing.drift_m", d.drift_m, 1.0, 10000.0)?;
        within("forest_floor.dressing.squat", d.squat, 0.1, 1.0)?;
        within("forest_floor.dressing.edge_m", d.edge_m, 0.0, 100.0)?;
        within(
            "forest_floor.dressing.trunk_clear_m",
            d.trunk_clear_m,
            0.0,
            100.0,
        )?;
        within("forest_floor.dressing.clear_m", d.clear_m, 0.0, 100.0)?;
        within(
            "forest_floor.dressing.colour_jitter",
            d.colour_jitter,
            0.0,
            0.5,
        )?;
        within("forest_floor.dressing.fade_px", d.fade_px, 0.1, 10000.0)?;
        tiers("forest_floor.dressing.lod_px", d.lod_px, d.fade_px / 2.0)
    }

    fn check_trees(&self) -> Result<(), String> {
        let t = &self.trees;
        if t.species.is_empty() {
            bad("trees.species", "is empty")?;
        }
        for (i, s) in t.species.iter().enumerate() {
            let at = |f: &str| format!("trees.species[{i}].{f}");
            if s.appearance.is_empty() {
                bad(&at("appearance"), "is empty")?;
            }
            within(&at("weight"), s.weight, 0.0, 1000.0)?;
            tint(&at("tint"), s.tint)?;
            if s.family.as_deref() == Some("") {
                bad(&at("family"), "is empty")?;
            }
            if let Some(m) = s.interior_m {
                within(&at("interior_m"), m, 0.0, 1000.0)?;
                if s.family.is_some() {
                    bad(
                        &at("interior_m"),
                        "is for a species of no family: a stand grows to its forest's edge",
                    )?;
                }
            }
        }
        // Every tree a forest's edge, a strip or the backdrop draws is of a family.
        if !t
            .species
            .iter()
            .any(|s| s.weight > 0.0 && s.family.is_some())
        {
            bad("trees.species", "no species of a family has a weight")?;
        }
        within("trees.stands.size_m", t.stands.size_m, 1.0, 100000.0)?;
        within("trees.stands.purity", t.stands.purity, 0.0, 1.0)?;
        within("trees.colour_jitter", t.colour_jitter, 0.0, 0.5)?;
        range("trees.forest.top", t.forest.top, 0.1, 1.0)?;
        range("trees.forest.girth", t.forest.girth, 0.3, 1.0)?;
        if t.lone.species.is_empty() {
            bad("trees.lone.species", "is empty")?;
        }
        for (i, name) in t.lone.species.iter().enumerate() {
            if !t
                .species
                .iter()
                .any(|s| &s.appearance == name && s.weight > 0.0)
            {
                bad(
                    &format!("trees.lone.species[{i}]"),
                    format!("names no weighted species \"{name}\""),
                )?;
            }
        }
        range("trees.lone.top", t.lone.top, 0.1, 1.0)?;
        range("trees.lone.girth", t.lone.girth, 0.3, 1.0)?;
        let u = &t.understorey;
        if u.appearance.is_empty() {
            bad("trees.understorey.appearance", "is empty")?;
        }
        tint("trees.understorey.tint", u.tint)?;
        within("trees.understorey.spacing_m", u.spacing_m, 0.5, 100.0)?;
        within("trees.understorey.row_m", u.row_m, 0.5, 100.0)?;
        within("trees.understorey.sway_m", u.sway_m, 0.0, 10.0)?;
        within("trees.understorey.gap", u.gap, 0.0, 1.0)?;
        range("trees.understorey.length", u.length, 0.1, 2.0)?;
        range("trees.understorey.height", u.height, 0.1, 2.0)?;
        let h = &t.hedgerows;
        if h.appearance.is_empty() {
            bad("trees.hedgerows.appearance", "is empty")?;
        }
        tint("trees.hedgerows.tint", h.tint)?;
        within("trees.hedgerows.chance", h.chance, 0.0, 1.0)?;
        within("trees.hedgerows.min_edge_m", h.min_edge_m, 0.0, 10000.0)?;
        within("trees.hedgerows.spacing_m", h.spacing_m, 0.5, 100.0)?;
        within("trees.hedgerows.inset_m", h.inset_m, 0.0, 50.0)?;
        range("trees.hedgerows.tree_gap_m", h.tree_gap_m, 1.0, 10000.0)?;
        range("trees.hedgerows.tree_scale", h.tree_scale, 0.1, 2.0)?;
        within("trees.copses.chance", t.copses.chance, 0.0, 1.0)?;
        range("trees.copses.radius_m", t.copses.radius_m, 1.0, 1000.0)?;
        within("trees.copses.spacing_m", t.copses.spacing_m, 1.0, 100.0)?;
        let f = &t.felled;
        within("trees.felled.fall_s", f.fall_s, 0.1, 30.0)?;
        within("trees.felled.settle_s", f.settle_s, 0.0, 10.0)?;
        within("trees.felled.settle_deg", f.settle_deg, 0.0, 30.0)?;
        within("trees.felled.stump_height_m", f.stump_height_m, 0.05, 3.0)?;
        within(
            "trees.felled.rest_height_m",
            f.rest_height_m,
            f.stump_height_m + 0.05,
            10.0,
        )?;
        within("trees.felled.rest_spread", f.rest_spread, 0.1, 1.0)?;
        tint("trees.felled.cut", f.cut)?;
        within("trees.backdrop.clear_m", t.backdrop.clear_m, 0.0, 1000.0)?;
        within(
            "trees.backdrop.reach_m",
            t.backdrop.reach_m,
            0.0,
            self.field_rules.extent_m,
        )?;
        tiers("trees.lod_px", t.lod_px, 1.0)
    }

    fn check_grass(&self) -> Result<(), String> {
        let g = &self.grass;
        for (key, growth) in &g.growth {
            let at = |f: &str| format!("grass.growth.{key}.{f}");
            if key != VERGE_GROWTH && !self.plots.iter().any(|p| &p.name == key) {
                bad(
                    &format!("grass.growth.{key}"),
                    format!("names no plot kind (or \"{VERGE_GROWTH}\")"),
                )?;
            }
            within(&at("density"), growth.density, 0.0, 1.0)?;
            within(&at("height"), growth.height, 0.1, 4.0)?;
            if growth.mix.is_empty() || growth.mix.len() > GRASS_MIX_MAX {
                bad(
                    &at("mix"),
                    format!("must name one to {GRASS_MIX_MAX} grasses"),
                )?;
            }
            for (i, s) in growth.mix.iter().enumerate() {
                if s.appearance.is_empty() {
                    bad(&at(&format!("mix[{i}].appearance")), "is empty")?;
                }
                within(&at(&format!("mix[{i}].share")), s.share, 1e-3, 1000.0)?;
                within(&at(&format!("mix[{i}].drift")), s.drift, 0.0, 1.0)?;
                within(&at(&format!("mix[{i}].dry")), s.dry, 0.0, 1.0)?;
            }
            within(&at("rows"), growth.rows, 0.0, 1.0)?;
            range(&at("patches.height"), growth.patches.height, 0.1, 2.0)?;
            within(&at("patches.thin"), growth.patches.thin, 0.0, 1.0)?;
            within(&at("patches.dry"), growth.patches.dry, 0.0, 1.0)?;
        }
        let p = &g.patch_m;
        for (key, v) in [
            ("height", p.height),
            ("thin", p.thin),
            ("dry", p.dry),
            ("drift", p.drift),
            ("grain", p.grain),
        ] {
            within(&format!("grass.patch_m.{key}"), v, 0.5, 1000.0)?;
        }
        within("grass.pixels_per_clump", g.pixels_per_clump, 1.0, 10000.0)?;
        within("grass.max_clumps_m2", g.max_clumps_m2, 0.01, 400.0)?;
        range("grass.fade_m_per_px", g.fade_m_per_px, 0.001, 10.0)?;
        range("grass.soften_m_per_px", g.soften_m_per_px, 0.001, 10.0)?;
        within("grass.soften", g.soften, 0.0, 1.0)?;
        within("grass.blade_facing", g.blade_facing, 0.0, 1.0)?;
        within("grass.clump_value", g.clump_value, 0.0, 0.3)?;
        within("grass.dry_lift", g.dry_lift, 0.0, 0.5)?;
        within("grass.near_tier_px", g.near_tier_px, 0.0, 10000.0)?;
        within("grass.min_blade_px", g.min_blade_px, 0.0, 8.0)?;
        within("grass.clear_m.prop", g.clear_m.prop, 0.0, 20.0)?;
        within("grass.clear_m.area", g.clear_m.area, 0.0, 20.0)?;
        let s = &g.sway;
        within("grass.sway.lean", s.lean, 0.0, 1.0)?;
        within("grass.sway.gust", s.gust, 0.0, 1.0)?;
        within("grass.sway.flutter", s.flutter, 0.0, 1.0)?;
        within("grass.sway.flutter_hz", s.flutter_hz, 0.0, 20.0)?;
        for (i, &c) in g.capacity.iter().enumerate() {
            within(&format!("grass.capacity[{i}]"), c as f64, 1.0, 4e6)?;
        }
        Ok(())
    }
}

/// `row` with its family's own fields over it.
fn finish(row: &Value, family: &str) -> Value {
    let mut out = row.clone();
    if let (Some(own), Some(fields)) = (row["families"][family].as_object(), out.as_object_mut()) {
        fields.extend(own.clone());
    }
    out
}

fn bad(at: &str, why: impl std::fmt::Display) -> Result<(), String> {
    Err(format!("biome.{}: {why}", at.trim_end_matches('.')))
}

pub(crate) fn within(at: &str, v: f64, lo: f64, hi: f64) -> Result<(), String> {
    match v.is_finite() && v >= lo && v <= hi {
        true => Ok(()),
        false => bad(at, format!("must be within [{lo}, {hi}], got {v}")),
    }
}

fn range(at: &str, r: Range, lo: f64, hi: f64) -> Result<(), String> {
    within(&format!("{at}[0]"), r[0], lo, hi)?;
    within(&format!("{at}[1]"), r[1], r[0], hi)
}

fn tint(at: &str, c: Rgb) -> Result<(), String> {
    (0..3).try_for_each(|ch| within(&format!("{at}[{ch}]"), c[ch], 0.0, 4.0))
}

/// Three detail thresholds, falling, the last above `least`.
fn tiers(at: &str, px: [f64; 3], least: f64) -> Result<(), String> {
    within(&format!("{at}[2]"), px[2], least, 10000.0)?;
    within(&format!("{at}[1]"), px[1], px[2], 10000.0)?;
    within(&format!("{at}[0]"), px[0], px[1], 10000.0)
}

#[cfg(test)]
pub mod tests {
    use super::Biome;

    pub const SUMMER: &str = include_str!("../../../../../fixtures/biomes/summer.json");

    /// The summer biome with `edit` applied to its JSON.
    pub fn summer(edit: impl FnOnce(&mut serde_json::Value)) -> Result<Biome, String> {
        let mut b: serde_json::Value = serde_json::from_str(SUMMER).unwrap();
        edit(&mut b);
        Biome::load(&b.to_string(), None)
    }

    #[test]
    fn the_summer_biome_is_admitted_in_every_region() {
        for family in [None, Some("paris"), Some("china"), Some("nowhere")] {
            Biome::load(SUMMER, family).unwrap();
        }
    }

    #[test]
    fn a_region_finishes_its_rows_and_no_others() {
        let paris = Biome::load(SUMMER, Some("paris")).unwrap();
        let plain = Biome::load(SUMMER, None).unwrap();
        assert_eq!(paris.road("paving").palette, "paving_paris");
        assert!(paris.road("paving").slabs.is_some() && plain.road("paving").slabs.is_none());
        assert_eq!(paris.road("road").palette, plain.road("road").palette);
    }

    #[test]
    fn a_bad_number_is_refused_by_its_path() {
        let e = summer(|b| b["grass"]["sway"]["lean"] = 2.into())
            .err()
            .unwrap();
        assert_eq!(e, "biome.grass.sway.lean: must be within [0, 1], got 2");
        let e = summer(|b| b["roads"]["paving"]["families"]["paris"]["mottle"] = 3.into());
        assert_eq!(
            e.err().unwrap(),
            "biome.roads.paving.families.paris.mottle: must be within [0, 1], got 3"
        );
    }

    #[test]
    fn a_palette_darker_than_unseen_ground_is_refused() {
        let e = summer(|b| b["palettes"]["meadow"] = serde_json::json!([[0.05, 0.06, 0.04]]));
        assert!(e.err().unwrap().contains("under the floor of 24"));
    }

    #[test]
    fn a_missing_row_field_is_refused_by_serde() {
        let e = summer(|b| {
            b["shore"]
                .as_object_mut()
                .unwrap()
                .remove("lift")
                .map(drop)
                .unwrap_or(())
        });
        assert!(e.err().unwrap().contains("lift"));
    }
}
