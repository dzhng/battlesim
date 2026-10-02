//! What a caller asks the map generator for. The generator and its presets
//! live in `mapgen`; the request is here because a map's source
//! (`maps::MapSource`) and a battle's preparation (`preparation`) name it.
use crate::identity::Seed;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MapType {
    Open,
    Mixed,
    Metro,
}

impl MapType {
    pub const ALL: [MapType; 3] = [MapType::Open, MapType::Mixed, MapType::Metro];
    pub fn name(self) -> &'static str {
        match self {
            MapType::Open => "open",
            MapType::Mixed => "mixed",
            MapType::Metro => "metro",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MapSize {
    Small,
    Medium,
    Large,
}

impl MapSize {
    pub const ALL: [MapSize; 3] = [MapSize::Small, MapSize::Medium, MapSize::Large];
    pub fn name(self) -> &'static str {
        match self {
            MapSize::Small => "small",
            MapSize::Medium => "medium",
            MapSize::Large => "large",
        }
    }
    /// M04: the side of the square playable area. A user decision, not a preset.
    pub fn extent_m(self) -> f64 {
        match self {
            MapSize::Small => 6_000.0,
            MapSize::Medium => 8_000.0,
            MapSize::Large => 10_000.0,
        }
    }
}

/// The compiler's admission for one plan: execution policy, apart from the
/// map's identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompileLimits {
    /// Ordinary authored bodies plus materialized template parts only.
    pub max_authored_parts: u32,
    pub max_bay_positions: u64,
    /// Polygon vertices plus rounded stroke samples, over surfaces, forests
    /// and rivers.
    pub max_ground_points: u64,
}

/// Everything that decides a generated plan. The same request and presets
/// give the same plan bytes on every target. The generator refuses a request
/// that pins another generator version, preset revision or catalogue than
/// the ones it is given, so a stored request never quietly yields a new map.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerationRequest {
    pub generator_version: String,
    pub preset_revision: String,
    pub seed: Seed,
    pub template_catalog_hash: String,
    #[serde(rename = "type")]
    pub map_type: MapType,
    pub size: MapSize,
    /// The compiler's admission for the plan this request yields.
    pub limits: CompileLimits,
}
