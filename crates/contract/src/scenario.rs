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
    pub infantry_eye_m: f64,
    pub tank_eye_m: f64,
    pub supply_eye_m: f64,
    pub infantry_muzzle_m: f64,
    /// Muzzle in the hull frame (forward, left, up) for turreted vehicle mounts.
    pub tank_muzzle_local_m: [f64; 3],
    /// Angular spread multiplier while the firing unit moves (W04).
    pub moving_scatter_multiplier: f64,
    /// Extra room a round's predicted path must keep from friendly vehicles (P11).
    pub friendly_prefire_margin_m: f64,
    #[serde(flatten)]
    pub flight: crate::ballistics::FlightRules,
}

/// Armour by impacted face (P10).
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Armor {
    pub front: f64,
    pub side: f64,
    pub rear: f64,
    pub roof: f64,
}

/// The hull face a hit or blast meets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    Front,
    Side,
    Rear,
    Roof,
}

impl Armor {
    pub fn weakest(&self) -> f64 {
        self.front.min(self.side).min(self.rear).min(self.roof)
    }

    pub fn face(&self, face: Face) -> f64 {
        match face {
            Face::Front => self.front,
            Face::Side => self.side,
            Face::Rear => self.rear,
            Face::Roof => self.roof,
        }
    }
}

/// Health, armour and squad strength (the fixture's `health` section).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthRules {
    pub soldier: f64,
    pub tank: f64,
    pub supply: f64,
    pub tank_armor: Armor,
    pub supply_armor: Armor,
    pub rifle_squad_size: u32,
    pub recon_squad_size: u32,
    pub at_squad_size: u32,
}

/// Optical sensing and concealment (the fixture's `sensors` section).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SensorRules {
    pub infantry_ground_m: f64,
    pub recon_ground_m: f64,
    pub tank_ground_m: f64,
    pub supply_ground_m: f64,
    /// Detection reach decays as `exp(-foliage / forest_attenuation_m)`.
    pub forest_attenuation_m: f64,
    /// A continuous foliage run this long blocks a ground ray outright.
    pub forest_full_block_m: f64,
    /// Infantry concealment strength is `edge + depth / ramp` inside a forest.
    pub infantry_concealment_edge_strength: f64,
    pub infantry_concealment_ramp_m: f64,
    /// Vehicle concealment strength is `(depth - depth_m) / ramp`.
    pub vehicle_concealment_depth_m: f64,
    pub vehicle_concealment_ramp_m: f64,
    pub infantry_forest_range_multiplier: f64,
    pub vehicle_forest_range_multiplier: f64,
    /// Seconds an acquisition survives lost identification (V12).
    pub acquisition_grace_s: f64,
    pub contact_radius_m: f64,
    pub contact_lifetime_s: f64,
    pub hearing_infantry_m: f64,
    pub hearing_vehicle_m: f64,
    pub hearing_shot_m: f64,
    pub sound_bucket_s: f64,
    /// Ground visibility field resolution and the height it tests above ground.
    pub fog_cell_m: f64,
    pub fog_target_height_m: f64,
}

/// Unit value used for target priority (the fixture's `cost_priority`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CostRules {
    pub rifle: u32,
    pub recon: u32,
    pub at: u32,
    pub tank: u32,
    pub supply: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rules {
    pub tick_hz: u32,
    pub movement: MovementRules,
    #[serde(rename = "physics")]
    pub bodies: BodyRules,
    pub health: HealthRules,
    pub sensors: SensorRules,
    #[serde(rename = "cost_priority")]
    pub costs: CostRules,
    pub weapons: crate::weapons::WeaponRules,
    pub mounts: crate::weapons::MountRules,
    pub suppression: SuppressionRules,
    pub cover: CoverRules,
}

/// Infantry suppression (P14): accumulated in [0, 1], decaying after a lull.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SuppressionRules {
    pub recovery_delay_s: f64,
    pub decay_per_s: f64,
    /// Movement speed lost at full suppression.
    pub max_move_penalty: f64,
    /// Reload/cycle progress lost at full suppression.
    pub max_reload_cycle_penalty: f64,
}

/// Cover multipliers at full strength (V03, P13): wider incoming spread and
/// fewer damaging fragments, never less damage per hit.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CoverRules {
    pub forest_spread_multiplier: f64,
    pub forest_fragment_probability_multiplier: f64,
}

/// An authored change at a fixed tick: part of the fixture, replayed with it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScenarioEvent {
    pub tick: Tick,
    #[serde(flatten)]
    pub action: EventAction,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventAction {
    AddProp(PropDefinition),
    /// Lab emitter: `unit` fires, producing the same firing evidence a weapon's
    /// shot does. It launches no projectile.
    Fire {
        unit: crate::ids::UnitId,
    },
}

/// A fixture-scripted order for either side, submitted at `tick` like input.
/// A scripted controller: replays feed the recorded command instead.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScriptedOrder {
    pub tick: Tick,
    pub side: Side,
    pub order: crate::command::Order,
    #[serde(default)]
    pub queued: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UnitSetup {
    pub side: Side,
    pub kind: UnitKind,
    pub position: [f64; 2],
    #[serde(default)]
    pub yaw: f64,
    /// Initial fire policy; fire at will when omitted.
    #[serde(default)]
    pub engagement: Option<crate::command::Engagement>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScenarioDefinition {
    pub map: MapDefinition,
    pub rules: Rules,
    pub units: Vec<UnitSetup>,
    #[serde(default)]
    pub events: Vec<ScenarioEvent>,
    #[serde(default)]
    pub scripts: Vec<ScriptedOrder>,
}
