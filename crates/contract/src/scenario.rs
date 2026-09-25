//! Authored battle setup: map, rules and initial units. Numeric rules come
//! from the fixture; nothing here carries defaults.
use crate::ids::{Side, Tick};
use crate::map::{MapDefinition, PropDefinition};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnitKind {
    Rifle,
    Recon,
    At,
    Tank,
    Supply,
}

impl UnitKind {
    pub fn is_infantry(self) -> bool {
        matches!(self, UnitKind::Rifle | UnitKind::Recon | UnitKind::At)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MovementRules {
    pub infantry_mps: f64,
    pub infantry_road_multiplier: f64,
    pub tank_mps: f64,
    pub tank_road_mps: f64,
    pub supply_mps: f64,
    pub supply_road_mps: f64,
    pub forest_infantry_multiplier: f64,
    pub forest_vehicle_multiplier: f64,
    pub vehicle_turn_deg_s: f64,
    pub turret_turn_deg_s: f64,
    pub bearing_tolerance_deg: f64,
}

/// Body dimensions (the fixture's `physics` section; other keys belong to later slices).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BodyRules {
    pub soldier_radius_m: f64,
    pub soldier_height_m: f64,
    /// Half length (along heading), half width, half height.
    pub tank_half_extents_m: [f64; 3],
    pub supply_half_extents_m: [f64; 3],
}

/// Squad strengths (the fixture's `health` section).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SquadRules {
    pub rifle_squad_size: u32,
    pub recon_squad_size: u32,
    pub at_squad_size: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rules {
    pub tick_hz: u32,
    pub movement: MovementRules,
    #[serde(rename = "physics")]
    pub bodies: BodyRules,
    #[serde(rename = "health")]
    pub squads: SquadRules,
}

/// An authored change at a fixed tick: part of the fixture, replayed with it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScenarioEvent {
    pub tick: Tick,
    pub add_prop: PropDefinition,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UnitSetup {
    pub side: Side,
    pub kind: UnitKind,
    pub position: [f64; 2],
    #[serde(default)]
    pub yaw: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScenarioDefinition {
    pub map: MapDefinition,
    pub rules: Rules,
    pub units: Vec<UnitSetup>,
    #[serde(default)]
    pub events: Vec<ScenarioEvent>,
}
