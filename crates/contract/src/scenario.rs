//! Authored battle setup: map, rules and initial units. Numeric rules come
//! from the fixture and carry no defaults; only authored setup (unit wear,
//! events, scripts, an opponent) may be left out.
use crate::ids::{Side, Tick};
use crate::map::{MapDefinition, PropDefinition};
use crate::movement::MovementRules;
use crate::observation::SuppressionTier;
use serde::{Deserialize, Serialize};

mod admission;
mod ranges;
pub use admission::RulesError;
use admission::UncheckedRules;

pub use crate::maps::{MapIdentity, MapSource, ResolvedMap};

/// How hard a body is to shove (Q3, Q4). A pusher moves only bodies
/// strictly lighter than its push class; nothing moves an immovable body.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeightClass {
    Trivial,
    Light,
    Medium,
    Heavy,
    Immovable,
}

impl WeightClass {
    /// Numeric rank for pushing.
    pub fn rank(self) -> u8 {
        match self {
            WeightClass::Trivial => 0,
            WeightClass::Light => 1,
            WeightClass::Medium => 2,
            WeightClass::Heavy => 3,
            WeightClass::Immovable => u8::MAX,
        }
    }

    /// The cover tier a body of this weight gives.
    pub fn cover_tier(self) -> Option<CoverTier> {
        match self {
            WeightClass::Trivial => None,
            WeightClass::Light => Some(CoverTier::Light),
            WeightClass::Medium => Some(CoverTier::Medium),
            WeightClass::Heavy => Some(CoverTier::Heavy),
            WeightClass::Immovable => None,
        }
    }
}

/// What a mover can shove aside (Q3): every body strictly lighter than its
/// class. A vehicle's is its hull's weight (`Hull::push_class`); a soldier
/// shoves nothing.
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
#[serde(deny_unknown_fields)]
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

mod forest;
pub use forest::{ForestRule, ForestRules};

mod navigation;
pub use navigation::NavigationRules;

/// How a squad's soldiers spread out where a move ends (D1, Q7): each move
/// draws a fresh seeded arrangement, never a formation. And how each soldier
/// walks there on his own (Q6, Q10, Q23): his lane beside the squad's
/// corridor, his wander and pace, his personal space, his own route for the
/// final stretch, and how he yields to vehicles.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InfantryMovementRules {
    /// The half width a squad's route keeps clear of solids, and its soldiers
    /// placed out of a building keep from its walls: a squad threads gaps a
    /// vehicle cannot.
    pub path_clearance_m: f64,
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

mod body;
pub use body::BodyRules;

/// What a launcher needing a clear launch path (a weapon row's
/// `requires_clear_launch_path`) holds for.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LaunchRules {
    /// How far along its arc from the muzzle its round must meet nothing.
    pub clearance_m: f64,
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
#[serde(deny_unknown_fields)]
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

mod sensors;
pub use sensors::{RadarReport, SensorRules};
mod targeting;
pub use targeting::TargetingRules;

/// How far a unit sees by direction, as multipliers of its ground range:
/// dead ahead, abeam and astern. `sim::sight` owns how it eases between them.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SightShape {
    pub front: f64,
    pub side: f64,
    pub rear: f64,
}

mod service;
pub use service::ServiceRules;

/// Battle rules, checked across sections on deserialization.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(try_from = "UncheckedRules")]
pub struct Rules {
    pub tick_hz: u32,
    pub commands: CommandRules,
    pub movement: MovementRules,
    pub navigation: NavigationRules,
    pub formation: FormationRules,
    pub infantry_movement: InfantryMovementRules,
    pub physics: BodyRules,
    /// Every unit type and prop type (the catalog, `fixtures/units/` and
    /// `fixtures/props/`).
    pub catalog: crate::catalog::Catalog,
    pub pushing: PushingRules,
    pub ricochet: RicochetRules,
    pub guided: crate::ballistics::GuidedRules,
    pub launch: LaunchRules,
    pub sensors: SensorRules,
    pub targeting: TargetingRules,
    /// The one forest rule (Q16, Q-G8b).
    pub forests: ForestRules,
    /// The weapon rows, `extends` resolved (`weapons::resolve_weapons`).
    pub weapons: crate::weapons::WeaponRules,
    pub service: ServiceRules,
    pub suppression: SuppressionRules,
    pub cover: CoverRules,
    pub ground: GroundRules,
    pub buildings: BuildingRules,
    pub garrison: GarrisonRules,
    pub carriage: CarriageRules,
    pub bail_out: std::collections::BTreeMap<BailOutHost, BailOutRow>,
    pub air: AirRules,
    pub shouts: ShoutRules,
}

mod shouts;
pub use shouts::ShoutRules;

mod air;
pub use air::{AirRules, FixedWingFlight, HighAirLayer, LowAirLayer, RotorFlight};

mod overrides;

/// How fast a side may give orders: a backstop no player reaches (a
/// tournament player's effective rate is about 5 a second), so a flood from
/// a script or a stuck input cannot buy the battle unbounded work.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandRules {
    /// Orders a side may give in a second, and in one burst; one past it is
    /// dropped without a word.
    pub orders_per_s: u32,
}

impl CommandRules {
    fn check(&self) -> Result<(), String> {
        if self.orders_per_s == 0 {
            return Err("orders_per_s must be positive: no order would ever be taken".into());
        }
        Ok(())
    }
}

/// Joint group placement; radius grows with footprint demand and obstructions.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormationRules {
    pub min_radius_m: f64,
    pub spacing_m: f64,
    pub fill_ratio: f64,
    pub growth_factor: f64,
    /// Alternative candidates across the entire selection; every unit still
    /// gets its intended-point check, even when this allowance is exhausted.
    pub candidate_checks: u32,
    pub max_expansions: u32,
}

impl FormationRules {
    fn check(&self) -> Result<(), String> {
        if !(self.min_radius_m.is_finite()
            && self.min_radius_m > 0.0
            && self.spacing_m.is_finite()
            && self.spacing_m >= 0.0
            && self.fill_ratio > 0.0
            && self.fill_ratio <= 1.0
            && self.growth_factor.is_finite()
            && self.growth_factor > 1.0
            && self.candidate_checks > 0
            && self.max_expansions <= 32)
        {
            return Err("positive finite extent, nonnegative spacing, fill in (0,1], growth > 1, positive search allowance and at most 32 expansions required".into());
        }
        Ok(())
    }
}

/// The kinematic shove (Q2): a pusher slides a lighter body out of its hull
/// along the contact, turns it when struck off-centre, and is slowed by the
/// ratio of the two classes.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PushingRules {
    /// Degrees a struck body turns per metre it is shoved, when struck at
    /// the end of its long side (scaled down toward its middle).
    pub turn_deg_per_m: f64,
    /// A side re-learns a body it has seen move once it has moved this far
    /// from where the side last saw it, or has come to rest (Q13).
    pub relearn_m: f64,
    /// A body struck nearly square on the nose slides off it this many
    /// degrees aside of the pusher's travel, so it leaves the hull's path
    /// rather than being driven along ahead of it.
    pub aside_deg: f64,
}

/// Buildings as fighting positions (L08–L10): supported soldier capacity.
/// Their integrity and ruin are the body table's `building` row (34c).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildingRules {
    pub capacity_soldiers: u32,
}

/// Entering, leaving and escaping buildings (the fixture's `garrison` section).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GarrisonRules {
    /// One stationary timer to enter and to leave.
    pub enter_exit_s: f64,
    /// A squad this close to a building's footprint may start entering.
    pub entry_distance_m: f64,
    /// Perimeter slots stand this far outside the facade.
    pub slot_standoff_m: f64,
    /// A slot faces a target only if the line to it leaves the facade by more
    /// than this angle, so no outgoing round grazes its own wall.
    pub slot_facing_min_deg: f64,
    /// A soldier changing windows, and the soldier he trades with, hold
    /// fire this long.
    pub window_swap_s: f64,
    /// A soldier who changed windows keeps his new one at least this long.
    pub window_hold_s: f64,
}

mod carriage;
pub use carriage::{BailOutHost, BailOutRow, CarriageRules};

/// Infantry suppression (P14): a hidden level in [0, 1] that near misses
/// raise and that fades after a lull, read only through its tiers. Below
/// `suppressed.level` a squad fights normally; from it, the suppressed
/// tier's fixed penalties apply; from `pinned.level`, the pinned tier's.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuppressionRules {
    /// Seconds without a new near miss before the level starts to fade.
    pub recovery_delay_s: f64,
    /// Level lost per second once it fades.
    pub decay_per_s: f64,
    pub suppressed: SuppressionTierRules,
    /// Its level is also what collapse survivors carry at least (L10).
    pub pinned: SuppressionTierRules,
}

/// One suppression tier: the level it starts at and its fixed penalties.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuppressionTierRules {
    pub level: f64,
    /// Share of movement speed lost.
    pub move_penalty: f64,
    /// Share of reload/cycle progress lost each tick.
    pub reload_cycle_penalty: f64,
    /// Multiplier on launch dispersion, composed with movement and cover.
    pub scatter_multiplier: f64,
}

impl SuppressionRules {
    /// The tier a hidden level falls in.
    pub fn tier(&self, level: f64) -> SuppressionTier {
        if level >= self.pinned.level {
            SuppressionTier::Pinned
        } else if level >= self.suppressed.level {
            SuppressionTier::Suppressed
        } else {
            SuppressionTier::None
        }
    }

    /// The fixed penalties at a hidden level; none below the suppressed tier.
    pub fn penalties(&self, level: f64) -> Option<&SuppressionTierRules> {
        match self.tier(level) {
            SuppressionTier::None => None,
            SuppressionTier::Suppressed => Some(&self.suppressed),
            SuppressionTier::Pinned => Some(&self.pinned),
        }
    }

    /// The tiers climb inside (0, 1], each penalty is a share, a deeper
    /// tier never costs less, and the level fades.
    fn check(&self) -> Result<(), String> {
        let (s, p) = (&self.suppressed, &self.pinned);
        if !(0.0 < s.level && s.level < p.level && p.level <= 1.0) {
            return Err(format!(
                "tier levels must climb inside (0, 1]: suppressed {}, pinned {}",
                s.level, p.level
            ));
        }
        for (name, t) in [("suppressed", s), ("pinned", p)] {
            if !t.scatter_multiplier.is_finite() || t.scatter_multiplier < 1.0 {
                return Err(format!(
                    "{name}.scatter_multiplier must be finite and at least 1"
                ));
            }
            for (what, v) in [
                ("move_penalty", t.move_penalty),
                ("reload_cycle_penalty", t.reload_cycle_penalty),
            ] {
                if !(0.0..1.0).contains(&v) {
                    return Err(format!("{name}.{what} must be in [0, 1): {v}"));
                }
            }
        }
        if p.move_penalty < s.move_penalty
            || p.reload_cycle_penalty < s.reload_cycle_penalty
            || p.scatter_multiplier < s.scatter_multiplier
        {
            return Err("pinned must cost at least what suppressed does".into());
        }
        if !(self.decay_per_s > 0.0 && self.recovery_delay_s >= 0.0) {
            return Err(format!(
                "decay_per_s must be positive and recovery_delay_s non-negative: {}, {}",
                self.decay_per_s, self.recovery_delay_s
            ));
        }
        Ok(())
    }
}

/// How much a cover body protects infantry (D6), weakest first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverTier {
    Light,
    Medium,
    Heavy,
}

mod cover;
pub use cover::{CoverRules, CoverTiers};

mod ground;
pub use ground::GroundRules;

/// An authored change at a fixed tick: part of the fixture, replayed with it.
#[derive(Clone, Debug, Serialize)]
pub struct ScenarioEvent {
    pub tick: Tick,
    #[serde(flatten)]
    pub action: EventAction,
}

impl<'de> Deserialize<'de> for ScenarioEvent {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::{Error, MapAccess, Visitor};
        struct EventVisitor;
        impl<'de> Visitor<'de> for EventVisitor {
            type Value = ScenarioEvent;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a tick and one scenario action")
            }
            fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
                let mut fields = std::collections::BTreeMap::new();
                while let Some(key) = map.next_key::<String>()? {
                    let raw = map.next_value::<Box<serde_json::value::RawValue>>()?;
                    if fields.insert(key.clone(), raw).is_some() {
                        return Err(M::Error::custom(format!(
                            "duplicate scenario event field {key}"
                        )));
                    }
                }
                let tick = fields
                    .remove("tick")
                    .ok_or_else(|| M::Error::missing_field("tick"))?;
                let tick = serde_json::from_str(tick.get()).map_err(M::Error::custom)?;
                // Keep the flattened action's physical tokens until its existing
                // enum/PropDefinition owner decodes them. No second action schema.
                let action = serde_json::from_str(
                    &serde_json::to_string(&fields).map_err(M::Error::custom)?,
                )
                .map_err(M::Error::custom)?;
                Ok(ScenarioEvent { tick, action })
            }
        }
        deserializer.deserialize_map(EventVisitor)
    }
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

/// Executable authored behavior carried inside a portable scenario projection.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoredScript {
    pub id: String,
    pub source: String,
    #[serde(default = "default_script_side")]
    pub side: Side,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub player: Option<String>,
}

fn default_script_side() -> Side {
    Side::Blue
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitSetup {
    pub side: Side,
    /// Scenario-bundle player that owns this unit. Legacy authored
    /// encounters omit it and remain side-owned; portable bundles materialize
    /// it before the battle starts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
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

/// Starting unit wear.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct UnitCondition {
    /// Vehicle health at the start.
    #[serde(default)]
    pub hp: Option<f64>,
    /// Soldiers already fallen at the start (their corpses lie where they stood).
    #[serde(default)]
    pub casualties: u32,
    /// Each soldier's health at the start, in place of his type's, and may
    /// exceed it: a directed battle keeps a squad standing as long as its
    /// shots need. Left out of the serialized setup when unset, so older
    /// setups digest as before.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub soldier_hp: Option<f64>,
    /// Rounds already spent, by weapon row.
    #[serde(default)]
    pub spent: std::collections::BTreeMap<String, u32>,
    /// A squad's hidden suppression level at the start, in [0, 1]: it starts
    /// in that level's tier and recovers from tick 0 like any other.
    /// Left out of the serialized setup at 0, so older setups digest as before.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub suppression: f64,
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioDefinition {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub players: Vec<crate::players::Player>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skirmish: Option<crate::skirmish::SkirmishSetup>,
    pub map: MapDefinition,
    pub rules: Rules,
    pub units: Vec<UnitSetup>,
    #[serde(default)]
    pub events: Vec<ScenarioEvent>,
    #[serde(default)]
    pub scripts: Vec<ScriptedOrder>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authored_scripts: Vec<AuthoredScript>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub victory: Option<String>,
    /// An observation-bound opponent playing one side (the encounter defender).
    #[serde(default)]
    pub opponent: Option<Opponent>,
    /// An encounter's local completion condition (holding its objective).
    #[serde(default)]
    pub encounter: Option<EncounterRules>,
    /// An authored scenario's director, with the zones and forces it names.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub director: Option<crate::director::DirectorSetup>,
}

impl ScenarioDefinition {
    /// Every unit type, added prop type and burst weapon row the scenario
    /// names that its rules do not hold, as (location, message) pairs
    /// relative to the scenario. A battle starts only when this is empty.
    pub fn unknown_catalog_references(&self) -> Vec<(String, String)> {
        let catalog = &self.rules.catalog;
        let mut unknown = Vec::new();
        for (i, unit) in self.units.iter().enumerate() {
            if catalog.index(&unit.kind).is_none() {
                unknown.push((
                    format!("units[{i}].kind"),
                    format!("unknown unit type {:?}", unit.kind),
                ));
            }
        }
        for (i, event) in self.events.iter().enumerate() {
            match &event.action {
                EventAction::AddProp(prop) => {
                    let placed = catalog
                        .props()
                        .check_placement_of(&prop.kind, crate::catalog::PropPlacement::Ordinary);
                    if let Err(message) = placed {
                        unknown.push((format!("events[{i}].add_prop.kind"), message));
                    }
                }
                EventAction::Burst { weapon, .. } if !self.rules.weapons.contains_key(weapon) => {
                    unknown.push((
                        format!("events[{i}].burst.weapon"),
                        format!("unknown weapon row {weapon:?}"),
                    ));
                }
                EventAction::Burst { .. } | EventAction::Fire { .. } => {}
            }
        }
        unknown
    }
}

/// The encounter half of a scenario: the forces, events and scripted orders
/// laid on a map, and the scenario's own rules, as a saved map's
/// `encounters/<name>.json` holds them. A field it does not know is refused.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EncounterDefinition {
    pub units: Vec<UnitSetup>,
    #[serde(default)]
    pub events: Vec<ScenarioEvent>,
    #[serde(default)]
    pub scripts: Vec<ScriptedOrder>,
    #[serde(default)]
    pub opponent: Option<Opponent>,
    #[serde(default)]
    pub encounter: Option<EncounterRules>,
    /// What this scenario's rules change from the rules it is laid on
    /// ([`Rules::overridden`]): any weapon, unit, soldier, prop or rule
    /// section, a row patched or a scenario-only row added. Absent, the
    /// scenario plays the rules as given.
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub overrides: serde_json::Map<String, serde_json::Value>,
}

impl EncounterDefinition {
    /// The scenario this encounter makes on `map` under `rules`, its
    /// `overrides` merged in. Every reader that plays a saved encounter, the
    /// native and the browser's, makes its scenario here; a refused override
    /// names its section, row and field.
    pub fn on(self, map: MapDefinition, rules: Rules) -> Result<ScenarioDefinition, String> {
        let rules = rules
            .overridden(&self.overrides)
            .map_err(|e| format!("overrides: {e}"))?;
        Ok(ScenarioDefinition {
            players: Vec::new(),
            skirmish: None,
            map,
            rules,
            units: self.units,
            events: self.events,
            scripts: self.scripts,
            authored_scripts: Vec::new(),
            victory: None,
            opponent: self.opponent,
            encounter: self.encounter,
            director: None,
        })
    }
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
#[serde(deny_unknown_fields)]
pub struct EncounterRules {
    /// The side that must take the zone.
    pub attacker: Side,
    pub success_zone_center: [f64; 2],
    pub success_zone_radius_m: f64,
    pub hold_s: f64,
    pub max_assessment_s: f64,
}
