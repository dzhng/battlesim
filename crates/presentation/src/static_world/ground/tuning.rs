//! Authored ground settings: distant-plot sampling and water/shore uniforms.
//! Shader-only tuning lives in WGSL; this reader refuses unused keys.
use serde::Deserialize;

/// What the tables read of `presentation.terrain`.
#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct TerrainTuning {
    pub distant: Distant,
    pub water: WaterTuning,
    pub shore: ShoreTuning,
}

/// The water's surface, as every look draws it (`WaterLook`'s shared part).
#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct WaterTuning {
    pub shore_fade_m: f64,
    pub lobes: Lobes,
    pub shadow: f64,
    pub normals: Normals,
    pub refraction: Refraction,
    pub caustics: Caustics,
}

/// The sun's and the sky's reflection off the water (`waterPass`): three
/// lobes of rising roughness, each weighted by its share of the light.
#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Lobes {
    pub roughness: [f64; 3],
    pub weights: [f64; 3],
}

/// The light the waves focus on the bed (`waterCaustics`): their pattern's
/// tile, their strength, the share of their light lost a metre down, and
/// how sharp their crests are.
#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Caustics {
    pub tile_m: f64,
    pub strength: f64,
    pub fade_per_m: f64,
    pub sharpness: f64,
}

/// How the bed shows displaced through the surface's slope (`waterUnder`):
/// its shift as a share of the depth it is seen through, over at most
/// `depth_m`, never past `most` of the screen's height; the metres of water
/// per mip it blurs by.
#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Refraction {
    pub share: f64,
    pub depth_m: f64,
    pub most: f64,
    pub blur_m: f64,
}

/// The baked normal map's three layers (`waterNormal`): their tiles in
/// metres (primary, macro, detail), each's share of the wind's drift, how
/// far the map's slope leans the normal, and the pixel footprints the
/// ripples fade over.
#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Normals {
    pub tiles_m: [f64; 3],
    pub drifts: [f64; 3],
    pub strength: f64,
    pub fade_m: [f64; 2],
}

/// The bank's bands (`groundShore`) and its ground (`ShoreTuning` in
/// `TerrainParams`).
#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct ShoreTuning {
    pub wet_whole: f64,
    pub earth_whole: f64,
    pub fine: f64,
    pub fine_share: f64,
    pub turns: [f64; 2],
    pub fine_fade: [f64; 2],
    pub earth_mottle: f64,
    pub field_ease_m: f64,
    pub earth_roughness: f64,
    pub wet_roughness: f64,
    pub bed_ease_m: f64,
}

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Distant {
    pub plot_pixels: [f64; 2],
}
