//! Shared CPU/GPU numbers. Values belong to their concepts; this table
//! only names the exports consumed by WGSL and generated TypeScript.
use crate::fog::sight;
use crate::static_world::ground::{biome::GRASS_MIX_MAX, grass, sway::SWAY, tables, walk::HEART};
use crate::wave::WAVE;

/// A shader scalar: floats retain the owner's precision until WGSL admits
/// them as f32; integers remain u32 for array lengths and workgroup sizes.
#[derive(Clone, Copy, Debug)]
pub enum Scalar {
    F32(f64),
    U32(u32),
}
use Scalar::{F32, U32};

pub const CONSTANTS: &[(&str, Scalar)] = &[
    (
        "SURFACE_FLOATS",
        U32(crate::static_world::ground::field::FLOATS as u32),
    ),
    ("CUT_A", U32(crate::static_world::ground::CUT_A)),
    ("CUT_B", U32(crate::static_world::ground::CUT_B)),
    ("COLOR_LUMA_R", F32(crate::color::LUMA[0])),
    ("COLOR_LUMA_G", F32(crate::color::LUMA[1])),
    ("COLOR_LUMA_B", F32(crate::color::LUMA[2])),
    ("SWAY_BEND", F32(SWAY.bend)),
    ("SWAY_LEAN", F32(SWAY.lean)),
    ("SWAY_GUST", F32(SWAY.gust)),
    ("SWAY_SWING", F32(SWAY.swing)),
    ("SWAY_RATE", F32(SWAY.rate)),
    ("SWAY_FULL_GUST_MPS", F32(SWAY.full_gust_mps)),
    ("SWAY_SHIVER_M", F32(SWAY.shiver_m)),
    ("SWAY_SHIVER_RATE", F32(SWAY.shiver_rate)),
    ("SWAY_HEART", F32(HEART)),
    ("WAVE_TEXELS", F32(WAVE.texels as f64)),
    ("WAVE_SPEED", F32(WAVE.speed)),
    ("WAVE_DAMPING", F32(WAVE.damping)),
    ("WAVE_NORMAL", F32(WAVE.normal)),
    ("GRASS_HEIGHT_MIN", F32(grass::HEIGHT_VARIATION[0])),
    ("GRASS_HEIGHT_MAX", F32(grass::HEIGHT_VARIATION[1])),
    ("GRASS_MAX_HEIGHT_M", F32(grass::LIMITS.tallest)),
    ("GRASS_GROWTH_ROWS", U32(grass::GROWTH_ROWS as u32)),
    ("GRASS_MIX_MAX", U32(GRASS_MIX_MAX as u32)),
    ("GRASS_MAX_KINDS", U32(grass::MAX_KINDS as u32)),
    ("GROUND_GRASS_EDGE_M", F32(tables::GRASS_EDGE_M)),
    (
        "GROUND_FOREST_VERGE_FEATHER",
        F32(tables::FOREST_VERGE_FEATHER),
    ),
    ("GROUND_BANK_EASE_M", F32(tables::BANK_EASE_M)),
    ("GROUND_MARKS_CLEAR_M", F32(tables::MARKS_CLEAR_M)),
    ("GROUND_SCAR_DEPTH_CAP", F32(tables::SCAR_DEPTH_CAP)),
    ("FOG_TILE_PX", U32(sight::TILE_PX)),
    ("FOG_RADIAL_BINS", U32(sight::RADIAL_BINS)),
    ("SHADOW_CASCADES", U32(crate::shadow::CASCADES as u32)),
    (
        "LIGHT_CAST_LIGHTS_MAX",
        U32(crate::effects::lights::CAST_LIGHTS_MAX as u32),
    ),
    ("FOG_FOLIAGE_QUANTA", U32(sight::FOLIAGE_QUANTA)),
    (
        "FOG_WHOLE_TEXTURE_WIDTH",
        U32(sight::WHOLE_TEXTURE_WIDTH as u32),
    ),
    ("FOG_HORIZON_SECTORS", U32(sight::HORIZON_SECTORS as u32)),
    ("FOG_BUILD_WORKGROUP", U32(sight::BUILD_WORKGROUP as u32)),
    ("FOG_FOLIAGE_STEP", F32(sight::FOLIAGE_STEP)),
];
