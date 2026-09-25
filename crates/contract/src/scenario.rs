//! Authored battle setup: map, rules and initial units. Numeric rules come
//! from the fixture; nothing here carries defaults.
use crate::ids::Side;
use crate::map::MapDefinition;
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rules {
    pub tick_hz: u32,
    pub movement: MovementRules,
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
}
