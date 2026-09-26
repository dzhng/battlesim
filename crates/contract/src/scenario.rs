//! Authored battle setup: map, rules and initial units. Numeric rules come
//! from the fixture and carry no defaults; only authored setup (unit wear,
//! events, scripts, an opponent) may be left out.
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

/// Body dimensions and round flight (the fixture's `physics` section).
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
    /// Chance, per face, that a kinetic round failing to penetrate glances
    /// off. It stands in for the plate's slope: independent of any 3D model.
    pub ricochet: FaceChances,
}

/// A probability in [0, 1] per hull face.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct FaceChances {
    pub front: f64,
    pub side: f64,
    pub rear: f64,
    pub roof: f64,
}

impl FaceChances {
    pub fn face(&self, face: Face) -> f64 {
        match face {
            Face::Front => self.front,
            Face::Side => self.side,
            Face::Rear => self.rear,
            Face::Roof => self.roof,
        }
    }
}

/// How a round glancing off a hull flies on (the fixture's `ricochet` section).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RicochetRules {
    /// Share of the arriving speed the deflected round keeps.
    pub speed_kept: f64,
    /// Largest angle between the mirror reflection and the deflected path.
    pub scatter_deg: f64,
    /// Share of its penetration a round keeps per ricochet.
    pub penetration_kept: f64,
    /// Ricochets a round may make; the next failed penetration stops it.
    pub max_bounces: u8,
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
    /// Detection reach decays as `exp(-foliage / forest_attenuation_m)`;
    /// positive, so reach only shrinks along a ray.
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
    /// Detection-range multiplier for infantry garrisoned in a building at
    /// full building strength (the strongest concealment source wins).
    pub building_range_multiplier: f64,
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
    /// Directional sight per vehicle kind (Q4+). Infantry sees an even 360°
    /// (Q5), so it has no entry.
    pub sight_shape: SightShapes,
}

/// How far a unit sees by direction, as multipliers of its ground range:
/// dead ahead, abeam and astern. `sim::sight` owns how it eases between them.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SightShape {
    pub front: f64,
    pub side: f64,
    pub rear: f64,
}

impl SightShape {
    /// Infantry's even 360°.
    pub const ISOTROPIC: SightShape = SightShape {
        front: 1.0,
        side: 1.0,
        rear: 1.0,
    };
}

/// The fixture's `sensors.sight_shape`: one shape per vehicle kind.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SightShapes {
    pub tank: SightShape,
    pub supply: SightShape,
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

/// Deployment, service and finite stock (the fixture's `service` section).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServiceRules {
    /// One duration for deploying and for packing (L01).
    pub deploy_and_pack_s: f64,
    /// Recipients within this distance of a deployed supply vehicle are served.
    pub radius_m: f64,
    /// Each supply vehicle's finite stock (L05).
    pub stock: u32,
    /// Stock per restored round, by weapon row. Every finite row is priced
    /// (checked when a battle is set up), so no round is given away (L05).
    pub round_costs: std::collections::BTreeMap<String, u32>,
    pub ammo_rounds_per_s: f64,
    pub vehicle_hp_per_s: f64,
    pub stock_per_hp: u32,
    pub soldier_replacement_s: f64,
    pub stock_per_soldier: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rules {
    pub tick_hz: u32,
    pub movement: MovementRules,
    #[serde(rename = "physics")]
    pub bodies: BodyRules,
    pub health: HealthRules,
    pub ricochet: RicochetRules,
    pub guided: crate::ballistics::GuidedRules,
    pub sensors: SensorRules,
    #[serde(rename = "cost_priority")]
    pub costs: CostRules,
    pub weapons: crate::weapons::WeaponRules,
    pub mounts: crate::weapons::MountRules,
    pub service: ServiceRules,
    pub suppression: SuppressionRules,
    pub cover: CoverRules,
    pub ground: GroundRules,
    pub buildings: BuildingRules,
    pub garrison: GarrisonRules,
}

/// Buildings as fighting positions (L08–L10): soldier capacity, structural
/// health, the ruin a collapse leaves and the cover strength occupants get.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BuildingRules {
    pub capacity_soldiers: u32,
    pub hp: f64,
    pub ruin_height_m: f64,
    /// Building cover strength in [0, 1] (contracts: building strength).
    pub cover_strength: f64,
}

/// Entering, leaving and escaping buildings (the fixture's `garrison` section).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GarrisonRules {
    /// One stationary timer to enter and to leave.
    pub enter_exit_s: f64,
    /// Each occupant survives a collapse with this probability.
    pub survival_probability_on_collapse: f64,
    /// How far from its slot a collapse survivor may find a legal place.
    pub exit_search_radius_m: f64,
    /// A squad this close to a building's footprint may start entering.
    pub entry_distance_m: f64,
    /// Perimeter slots stand this far outside the facade.
    pub slot_standoff_m: f64,
    /// A slot faces a target only if the line to it leaves the facade by more
    /// than this angle, so no outgoing round grazes its own wall.
    pub slot_facing_min_deg: f64,
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
    /// Suppression survivors of a collapse carry at least (L10).
    pub collapse_level: f64,
}

/// Cover multipliers at full strength (V03, P13): wider incoming spread and
/// fewer damaging fragments, never less damage per hit.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CoverRules {
    pub forest_spread_multiplier: f64,
    pub forest_fragment_probability_multiplier: f64,
    pub building_spread_multiplier: f64,
    pub building_fragment_probability_multiplier: f64,
}

/// The ground layer (D2, Q8): per-cell craters, which give infantry partial
/// cover and slow vehicles slightly, and cosmetic scorch, track wear and
/// trampling. Channels are bytes in [0, 255] that accumulate and saturate.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GroundRules {
    /// Cell edge length.
    pub cell_m: f64,
    /// A weapon's crater radius as a fraction of its blast radius.
    pub crater_radius_fraction: f64,
    /// Crater depth a burst adds at its centre, per metre of crater radius,
    /// falling linearly to nothing at the rim.
    pub crater_depth_per_m: f64,
    /// Depth at which a crater gives its full cover and slowdown (saturation).
    pub crater_full_depth: f64,
    /// Ground-cover strength of a full crater for infantry in it, on the
    /// forest's scale (weaker than any forest and any building).
    pub crater_cover: f64,
    /// Vehicle speed multiplier over a full crater, in (0, 1].
    pub crater_vehicle_mult: f64,
    /// A weapon's scorch radius as a fraction of its blast radius.
    pub scorch_radius_fraction: f64,
    /// Scorch a burst adds at its centre, falling linearly to the rim.
    pub scorch_per_burst: f64,
    /// Track wear a vehicle track adds to each cell it enters.
    pub tracks_per_pass: f64,
    /// Trampling a soldier adds to each cell it enters.
    pub trampled_per_pass: f64,
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
    /// Lab emitter: a round of the weapon row `weapon` bursts on the ground at
    /// `point`, leaving the ground marks a real burst leaves. It flies no
    /// round, hurts nobody and is not published.
    Burst {
        point: [f64; 2],
        weapon: String,
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
    /// Authored starting damage and spent ammunition (labs, the encounter).
    #[serde(default)]
    pub condition: Option<UnitCondition>,
    /// A supply vehicle's starting stock; the rules' full stock when omitted.
    #[serde(default)]
    pub stock: Option<u32>,
}

/// A unit that starts the scenario already worn.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct UnitCondition {
    /// Vehicle health at the start.
    #[serde(default)]
    pub hp: Option<f64>,
    /// Soldiers already fallen at the start (their corpses lie in formation).
    #[serde(default)]
    pub casualties: u32,
    /// Rounds already spent, by weapon row.
    #[serde(default)]
    pub spent: std::collections::BTreeMap<String, u32>,
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
    /// An observation-bound opponent playing one side (the village defender).
    #[serde(default)]
    pub opponent: Option<Opponent>,
    /// A fixture's local completion condition (the village hold).
    #[serde(default)]
    pub encounter: Option<EncounterRules>,
}

/// A small defensive policy for one side (encounter.md). It sees only that
/// side's observation and acts only through ordinary commands.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Opponent {
    pub side: Side,
    /// (unit, building prop id) garrisoned at the start.
    pub garrisons: Vec<[u32; 2]>,
    /// An AT team attacks the costliest tank its own sensors identify within this range.
    pub at_attack_range_m: f64,
    pub tank_retreat_hp_fraction: f64,
    pub tank_fallback: [f64; 2],
    pub infantry_retreat_survivor_fraction: f64,
    pub infantry_fallback: [f64; 2],
}

/// Hold the zone uncontested for `hold_s` to succeed; lose every combat unit
/// to fail; after `max_assessment_s` the result is inconclusive (play on).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EncounterRules {
    /// The side that must take the zone.
    pub attacker: Side,
    pub success_zone_center: [f64; 2],
    pub success_zone_radius_m: f64,
    pub hold_s: f64,
    pub max_assessment_s: f64,
}
