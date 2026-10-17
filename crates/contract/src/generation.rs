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
    Xl,
}

impl MapSize {
    pub const ALL: [MapSize; 4] = [MapSize::Small, MapSize::Medium, MapSize::Large, MapSize::Xl];
    pub fn name(self) -> &'static str {
        match self {
            MapSize::Small => "small",
            MapSize::Medium => "medium",
            MapSize::Large => "large",
            MapSize::Xl => "xl",
        }
    }
    /// M04: the side of the square playable area. A user decision, not a preset.
    pub fn extent_m(self) -> f64 {
        match self {
            MapSize::Small => 4_000.0,
            MapSize::Medium => 6_000.0,
            MapSize::Large => 8_000.0,
            MapSize::Xl => 10_000.0,
        }
    }

    pub fn skirmish_extent_m(self) -> f64 {
        skirmish_size_data()
            .iter()
            .find(|tier| tier.size == self)
            .expect("skirmish size fixture contains every map size")
            .extent_m
    }
}

#[derive(Deserialize)]
struct SkirmishSizeTier {
    min_players: usize,
    #[serde(default)]
    max_players: Option<usize>,
    size: MapSize,
    extent_m: f64,
}

fn skirmish_size_data() -> &'static [SkirmishSizeTier] {
    static DATA: std::sync::OnceLock<Vec<SkirmishSizeTier>> = std::sync::OnceLock::new();
    DATA.get_or_init(|| {
        #[derive(Deserialize)]
        struct Fixture {
            tiers: Vec<SkirmishSizeTier>,
        }
        serde_json::from_str::<Fixture>(include_str!("../../../fixtures/skirmish-sizes.json"))
            .expect("valid skirmish-sizes fixture")
            .tiers
    })
}

pub fn skirmish_size_floor(players_per_side: usize) -> MapSize {
    let players_per_side = players_per_side.max(1);
    skirmish_size_data()
        .iter()
        .find(|tier| {
            tier.min_players <= players_per_side
                && tier.max_players.is_none_or(|max| players_per_side <= max)
        })
        .expect("skirmish size fixture covers every player count")
        .size
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

/// Geometry family, independent of the player's size tier.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GenerationProfile {
    #[default]
    Standard,
    Skirmish,
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
    /// Number of shared entry arms on each side. One is the legacy layout;
    /// larger values are admitted only for skirmish generation.
    #[serde(
        default = "default_entries_per_side",
        skip_serializing_if = "is_one_entries_per_side"
    )]
    pub entries_per_side: usize,
    #[serde(default)]
    pub profile: GenerationProfile,
    /// The regional family every building of the map is built in, one of
    /// the presets' `parcels.regional_families`. Absent, the seed draws one;
    /// a request without it serializes as it did before the field existed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    /// The compiler's admission for the plan this request yields.
    pub limits: CompileLimits,
}

fn default_entries_per_side() -> usize {
    1
}

fn is_one_entries_per_side(value: &usize) -> bool {
    *value == 1
}

impl GenerationRequest {
    pub fn extent_m(&self) -> f64 {
        match self.profile {
            GenerationProfile::Standard => self.size.extent_m(),
            GenerationProfile::Skirmish => self.size.skirmish_extent_m(),
        }
    }
}
