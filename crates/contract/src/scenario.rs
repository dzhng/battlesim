//! Authored battle setup: map, rules and initial units. Numeric rules come
//! from the fixture and carry no defaults; only authored setup (unit wear,
//! events, scripts, an opponent) may be left out.
use crate::ids::{Side, Tick};
use crate::map::{MapDefinition, PropDefinition};
use serde::{Deserialize, Serialize};

/// How hard a body is to shove (Q3, Q4). A pusher moves only bodies
/// strictly lighter than its push class; nothing moves an immovable body.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeightClass {
    Light,
    Medium,
    Heavy,
    Immovable,
}

impl WeightClass {
    /// Light 1, medium 2, heavy 3; immovable beyond every push class.
    pub fn rank(self) -> u8 {
        match self {
            WeightClass::Light => 1,
            WeightClass::Medium => 2,
            WeightClass::Heavy => 3,
            WeightClass::Immovable => u8::MAX,
        }
    }
}

/// What a vehicle can shove aside (Q3): every body strictly lighter than its
/// class. Room is left for obstacle clearers (`super_heavy` clears tank
/// wrecks and dragon's teeth).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PushClass {
    None,
    Light,
    Medium,
    Heavy,
    SuperHeavy,
}

impl PushClass {
    pub const ALL: [PushClass; 5] = [
        PushClass::None,
        PushClass::Light,
        PushClass::Medium,
        PushClass::Heavy,
        PushClass::SuperHeavy,
    ];

    /// None 0, light 1, medium 2, heavy 3, super-heavy 4.
    pub fn rank(self) -> u8 {
        self as u8
    }

    /// Whether this class shoves a body of `weight`: strictly lighter only.
    pub fn pushes(self, weight: WeightClass) -> bool {
        weight.rank() < self.rank()
    }

    /// The share of its speed a pusher keeps while shoving a body whose
    /// weight class has rank `weight_rank` (Q2: slowed by the class ratio):
    /// a tank shoving a light body keeps 2/3, a medium one 1/3.
    pub fn shove_speed(self, weight_rank: u8) -> f64 {
        1.0 - f64::from(weight_rank) / f64::from(self.rank()).max(1.0)
    }
}

/// Which movers a body stops: navigation and collision read these, nothing else.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Blocks {
    pub infantry: bool,
    pub vehicle: bool,
}

impl Blocks {
    pub fn class(&self, class: crate::map::MoverClass) -> bool {
        match class {
            crate::map::MoverClass::Infantry => self.infantry,
            crate::map::MoverClass::Vehicle => self.vehicle,
        }
    }
}

/// One row of the body table (Q19, Q28), keyed by `PropKind`. Each column
/// is independent and has its own readers: `blocks` navigation and
/// collision; `stops_rounds` flight; `occludes` the fog sweep, sensing and
/// the renderer's sight-light occluders; `weight_class` pushing;
/// `cover_tier` cover. A row that blocks nobody, stops no rounds and gives
/// no cover but occludes for a `lifetime_s` is a smoke screen.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PropBody {
    pub blocks: Blocks,
    pub stops_rounds: bool,
    /// Hides what lies behind it from sight (Q25: only big static bodies).
    pub occludes: bool,
    pub weight_class: WeightClass,
    /// The cover it gives infantry (Q4, Q24): from behind a body that blocks
    /// infantry, or from inside a ground body (rubble) that does not.
    #[serde(default)]
    pub cover_tier: Option<CoverTier>,
    /// A transient body: it goes this long after it appears.
    #[serde(default)]
    pub lifetime_s: Option<f64>,
    /// How much of a fog cell one such body conceals, in [0, 1) (Q21): a
    /// cell's foliage is `1 − Π(1 − conceals)` over the concealing bodies
    /// whose canopy covers it. Trunks conceal; every other kind 0 for now.
    #[serde(default)]
    pub conceals: f64,
    /// Integrity (Q17, 34c): the structural damage it takes to destroy one
    /// such body. None: ordinary fire never destroys it.
    #[serde(default)]
    pub hp: Option<f64>,
    /// The share of a direct round's structural damage this kind takes
    /// (blast is not scaled); 1 when absent.
    #[serde(default = "one")]
    pub armor: f64,
    /// What a destroyed body becomes; required with `hp`.
    #[serde(default)]
    pub destroyed: Option<Destroyed>,
}

fn one() -> f64 {
    1.0
}

/// A destroyed body's state (Q17): gone, gone and its ground cleared (a
/// tree: open ground, like a lane a tank knocks through), or another prop
/// kind on the same plan at `height_m` (a building's ruin, sandbags'
/// rubble, a lighter wreck).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Destroyed {
    Removed,
    Cleared,
    Into {
        kind: crate::map::PropKind,
        height_m: f64,
    },
}

/// The fixture's `forests` section: each density a forest may name.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ForestRules {
    pub densities: std::collections::BTreeMap<String, ForestDensity>,
}

/// One forest density (Q16): how its trunks stand and how much it hides.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ForestDensity {
    /// Trunk grid spacing; each trunk starts at its cell's centre, the
    /// first `spacing / 2` inside the rect's minimum corner.
    pub trunk_spacing_m: f64,
    /// Each trunk moves up to this fraction of the spacing off its cell's
    /// centre, on each axis (seeded by the forest, so every reader agrees).
    pub trunk_jitter: f64,
    /// Detection-range multiplier for a target under full foliage of this
    /// density, by class: the per-class strength is the rule (Q21).
    pub concealment_infantry: f64,
    pub concealment_vehicle: f64,
    /// Foliage depth per metre a sight line crosses below the canopy.
    pub attenuation_per_m: f64,
    /// A trunk's crown: it conceals each fog cell whose centre lies this near.
    pub canopy_radius_m: f64,
}

/// The fixture's body table: props by kind.
pub type PropTable = std::collections::BTreeMap<crate::map::PropKind, PropBody>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MovementRules {
    pub forest_infantry_multiplier: f64,
    pub forest_vehicle_multiplier: f64,
    pub turret_turn_deg_s: f64,
    pub bearing_tolerance_deg: f64,
}

/// How a squad's soldiers spread out where a move ends (D1, Q7): each move
/// draws a fresh seeded arrangement, never a formation. And how each soldier
/// walks there on his own (Q6, Q10, Q23): his lane beside the squad's
/// corridor, his wander and pace, his personal space, his own route for the
/// final stretch, and how he yields to vehicles.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InfantryMovementRules {
    /// Diameter of the ground a squad of `spread_squad_size` soldiers spreads
    /// over; other sizes keep the same ground per soldier.
    pub spread_m: f64,
    pub spread_squad_size: u32,
    /// Soldiers of one squad end a move at least this far apart.
    pub spacing_m: f64,
    /// Soft personal space: a soldier eases away from anyone closer (Q10).
    pub personal_space_m: f64,
    /// A soldier steers for the point of his lane this far ahead.
    pub steer_ahead_m: f64,
    /// His lane narrows toward the corridor this far ahead of a solid.
    pub lane_lookahead_m: f64,
    /// Metres per second his lane may widen or narrow.
    pub lane_shift_mps: f64,
    /// Wander: the most his lane drifts either side, and its period.
    pub wander_m: f64,
    pub wander_period_s: f64,
    /// Each soldier's pace swings between `1 - pace_variation` and full
    /// speed, from his own seeded phase.
    pub pace_variation: f64,
    /// Each soldier sets off up to this long after the order.
    pub stagger_s: f64,
    /// A soldier plans his own route (on the exact bodies) for the last
    /// this-many metres to his spot, or to rejoin the corridor.
    pub final_leg_m: f64,
    /// The side of the square window his own route is planned in.
    pub window_m: f64,
    /// A soldier yields when a vehicle's path over this many seconds would
    /// cross him, stepping this far clear of its hull (Q23).
    pub yield_horizon_s: f64,
    pub yield_margin_m: f64,
}

/// The one infantry body every soldier shares (navigation's clearance, cover,
/// the fit authority) and round flight (the fixture's `physics` section).
/// A vehicle's body is its type's hull.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BodyRules {
    pub soldier_radius_m: f64,
    pub soldier_height_m: f64,
    pub infantry_eye_m: f64,
    pub infantry_muzzle_m: f64,
    /// Angular spread multiplier while the firing unit moves (W04).
    pub moving_scatter_multiplier: f64,
    /// Extra room a round's predicted path must keep from friendly vehicles (P11).
    pub friendly_prefire_margin_m: f64,
    #[serde(flatten)]
    pub flight: crate::ballistics::FlightRules,
}

/// Armour by impacted face (P10).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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

/// Optical sensing and concealment (the fixture's `sensors` section).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SensorRules {
    /// A ground ray whose foliage depth (the sum of each crossed metre's
    /// `attenuation_per_m` below the canopy) reaches this is blocked
    /// outright; below it, reach is `range · exp(−depth)`.
    pub foliage_full_block: f64,
    /// Detection-range multiplier for infantry garrisoned in a building at
    /// full building strength (the strongest concealment source wins).
    pub building_range_multiplier: f64,
    /// Seconds an acquisition survives lost identification (V12).
    pub acquisition_grace_s: f64,
    pub contact_radius_m: f64,
    pub contact_lifetime_s: f64,
    /// Units carry their own loudness (their type's `sound`); shots carry
    /// this far.
    pub hearing_shot_m: f64,
    pub sound_bucket_s: f64,
    /// Ground visibility field resolution and the height it tests above ground.
    pub fog_cell_m: f64,
    pub fog_target_height_m: f64,
}

/// How far a unit sees by direction, as multipliers of its ground range:
/// dead ahead, abeam and astern. `sim::sight` owns how it eases between them.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SightShape {
    pub front: f64,
    pub side: f64,
    pub rear: f64,
}

/// How a deployed supplier serves (the fixture's `service` section); its
/// stock and deploy time are its type's capabilities.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServiceRules {
    /// Recipients within this distance of a deployed supply vehicle are served.
    pub radius_m: f64,
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
    pub infantry_movement: InfantryMovementRules,
    pub physics: BodyRules,
    /// The body table (Q19): every prop kind's row.
    pub props: PropTable,
    /// Every unit type (the unit catalog, `fixtures/units/`).
    pub catalog: crate::catalog::Catalog,
    pub pushing: PushingRules,
    pub ricochet: RicochetRules,
    pub guided: crate::ballistics::GuidedRules,
    pub sensors: SensorRules,
    /// Forest densities (Q16): what a forest's `density` names.
    pub forests: ForestRules,
    #[serde(deserialize_with = "crate::weapons::resolve_weapons")]
    pub weapons: crate::weapons::WeaponRules,
    pub service: ServiceRules,
    pub suppression: SuppressionRules,
    pub cover: CoverRules,
    pub ground: GroundRules,
    pub buildings: BuildingRules,
    pub garrison: GarrisonRules,
}

/// The kinematic shove (Q2): a pusher slides a lighter body out of its hull
/// along the contact, turns it when struck off-centre, and is slowed by the
/// ratio of the two classes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PushingRules {
    /// Degrees a struck body turns per metre it is shoved, when struck at
    /// the end of its long side (scaled down toward its middle).
    pub turn_deg_per_m: f64,
    /// A side re-learns a body it has seen move once it has moved this far
    /// from where the side last saw it, or has come to rest (Q13).
    pub relearn_m: f64,
}

/// Buildings as fighting positions (L08–L10): soldier capacity and the
/// cover strength occupants get. Their integrity and ruin are the body
/// table's `building` row (34c).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BuildingRules {
    pub capacity_soldiers: u32,
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

/// How much a cover body protects infantry (D6), weakest first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverTier {
    Light,
    Medium,
    Heavy,
}

/// Incoming spread multiplier per cover tier (Q5: cover works through scatter only).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CoverTiers {
    pub light: f64,
    pub medium: f64,
    pub heavy: f64,
}

impl CoverTiers {
    pub fn spread(&self, tier: CoverTier) -> f64 {
        match tier {
            CoverTier::Light => self.light,
            CoverTier::Medium => self.medium,
            CoverTier::Heavy => self.heavy,
        }
    }
}

/// Infantry cover (D3–D6, Q5, Q7, Q11, Q20, Q24): a hard-coded game rule for
/// soldiers only. When a round is aimed at a soldier, the strongest cover
/// body within `reach_m` of him that lies between him and the shooter widens
/// that round's spread by its tier; a crater he stands in is light cover.
/// A garrison keeps its building shelter instead (Q22, the named exception):
/// wider spread and fewer damaging fragments, never less damage per hit.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CoverRules {
    pub tiers: CoverTiers,
    // Which bodies cover, and how well, is the body table's `cover_tier`
    // column; a live vehicle covers by its weight class (Q24).
    /// Ground cover: a crater at least `crater_min_fill` full.
    pub crater: CoverTier,
    pub crater_min_fill: f64,
    /// A body within this of a soldier can cover him (Q20).
    pub reach_m: f64,
    /// A squad's area reaches this far beyond half its spread (27d): its
    /// soldiers seek cover anywhere inside it (Q7).
    pub search_m: f64,
    /// A soldier who cannot engage steps out, best cover first, within this
    /// far; beyond it, to the nearest place in the area he can engage from
    /// (D3, Q8, 27d).
    pub step_out_m: f64,
    /// A squad re-resolves its cover at most this often (Q11).
    pub reresolve_s: f64,
    /// A threat bearing swing that re-resolves cover (Q11).
    pub swing_deg: f64,
    /// A covering vehicle that moves this far re-resolves its users (Q24).
    pub vehicle_moved_m: f64,
    /// A soldier out on his lean fires at most this long before he tucks
    /// back in (27d): he steps out, fires a burst, and ducks back.
    pub lean_burst_s: f64,
    /// How long he stays tucked in before leaning out again (27d).
    pub lean_tuck_s: f64,
    pub building_spread_multiplier: f64,
    pub building_fragment_probability_multiplier: f64,
}

/// The ground layer (D2, Q8): per-cell craters, which are ground cover for
/// infantry (`CoverRules::crater`) and slow vehicles slightly, and cosmetic scorch, track wear and
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
    /// The unit type's catalog id.
    pub kind: String,
    pub position: [f64; 2],
    #[serde(default)]
    pub yaw: f64,
    /// Initial fire policy; fire at will when omitted.
    #[serde(default)]
    pub engagement: Option<crate::command::Engagement>,
    /// Authored starting damage and spent ammunition (labs, the encounter).
    #[serde(default)]
    pub condition: Option<UnitCondition>,
    /// A supply vehicle's starting stock; its type's when omitted.
    #[serde(default)]
    pub stock: Option<u32>,
}

/// A unit that starts the scenario already worn.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct UnitCondition {
    /// Vehicle health at the start.
    #[serde(default)]
    pub hp: Option<f64>,
    /// Soldiers already fallen at the start (their corpses lie where they stood).
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
