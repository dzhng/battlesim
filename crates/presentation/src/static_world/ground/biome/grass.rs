//! The biome's grass field: what grows on each plot kind and the verge,
//! its patches and sway, and how many clumps a frame draws.
use super::{bad, range, within, Biome, Range};
use serde::Deserialize;
use std::collections::BTreeMap;

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

impl Biome {
    pub(super) fn check_grass(&self) -> Result<(), String> {
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
