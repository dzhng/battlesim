//! Authored battle setup: map, rules and initial units. Numeric rules come
//! from the fixture and carry no defaults; only authored setup (unit wear,
//! events, scripts, an opponent) may be left out.
use crate::ids::{Side, Tick};
use crate::map::{MapDefinition, PropDefinition};
use crate::observation::SuppressionTier;
use serde::{Deserialize, Serialize};

pub use crate::maps::{MapIdentity, MapSource, ResolvedMap};

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

    /// The cover tier a body of this weight gives (Q24): its class, none
    /// when immovable. A vehicle's hull, and its wreck, give this.
    pub fn cover_tier(self) -> Option<CoverTier> {
        match self {
            WeightClass::Light => Some(CoverTier::Light),
            WeightClass::Medium => Some(CoverTier::Medium),
            WeightClass::Heavy => Some(CoverTier::Heavy),
            WeightClass::Immovable => None,
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

/// One surface kind's row. A mover on the surface travels at its road speed
/// times `speed_factor`, never slower than on open ground: 1 is a full road,
/// 0 no road at all.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurfaceRule {
    pub speed_factor: f64,
}

/// Every kind has a row, and no row is faster than a road: route planning's
/// best-case estimate is the unit's road speed.
fn surface_table<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<std::collections::BTreeMap<crate::map::SurfaceKind, SurfaceRule>, D::Error> {
    use serde::de::Error;
    let table = std::collections::BTreeMap::<crate::map::SurfaceKind, SurfaceRule>::deserialize(d)?;
    for kind in crate::map::SurfaceKind::ALL {
        let row = table
            .get(&kind)
            .ok_or_else(|| D::Error::custom(format!("surfaces has no row for {kind:?}")))?;
        if !(0.0..=1.0).contains(&row.speed_factor) {
            return Err(D::Error::custom(format!(
                "surfaces.{kind:?}.speed_factor must be within 0..=1"
            )));
        }
    }
    Ok(table)
}

/// The fixture's `forests` section: one rule every forest plays by (Q-G8b),
/// so a forest's art can never imply a different sight or movement rule.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForestRules {
    /// The prop type a forest's trees are (a `props` catalog entry).
    pub tree: String,
    pub rule: ForestRule,
}

/// How every forest's trunks stand and how much it hides (Q16, Q21).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForestRule {
    /// Trunk grid spacing; each trunk starts at its cell's centre, the
    /// first `spacing / 2` inside the shape's minimum corner.
    pub trunk_spacing_m: f64,
    /// Each trunk moves up to this fraction of the spacing off its cell's
    /// centre, on each axis (seeded by the forest, so every reader agrees).
    pub trunk_jitter: f64,
    /// Detection-range multiplier for a target under full foliage, by
    /// class: the per-class strength is the rule (Q21).
    pub concealment_infantry: f64,
    pub concealment_vehicle: f64,
    /// Foliage depth per metre a sight line crosses below the canopy.
    pub attenuation_per_m: f64,
    /// A trunk's crown: it conceals each fog cell whose centre lies this near.
    pub canopy_radius_m: f64,
    pub canopy_height_m: f64,
    pub trunk_radius_m: f64,
    pub trunk_height_m: f64,
    /// Trunks are omitted within this distance of roads and props.
    pub trunk_clearance_m: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MovementRules {
    pub forest_infantry_multiplier: f64,
    pub forest_vehicle_multiplier: f64,
    pub turret_turn_deg_s: f64,
    pub bearing_tolerance_deg: f64,
    pub drive: DriveRules,
}

/// How every vehicle drives its route (Q29, Q30), whatever its own speeds
/// and turning.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct DriveRules {
    /// Tracks turn in place beyond this heading error.
    pub turn_in_place_deg: f64,
    /// A wheeled vehicle reaches a waypoint it passes abeam within this.
    pub abeam_m: f64,
    /// Headings further off than this count as abeam or behind.
    pub abeam_deg: f64,
    /// A reversing leg of a three-point turn drives at least this far.
    pub min_leg_m: f64,
    /// The waypoint must lie this far outside the turning circle to end a leg.
    pub circle_margin_m: f64,
    /// Beyond this heading error a wheeled turn counts as a manoeuvre: it
    /// probes ahead, and its progress is not a stall.
    pub turning_deg: f64,
    /// A wheeled vehicle slows to this fraction of its speed at full lock.
    pub turn_slow: f64,
}

/// How a squad's soldiers spread out where a move ends (D1, Q7): each move
/// draws a fresh seeded arrangement, never a formation. And how each soldier
/// walks there on his own (Q6, Q10, Q23): his lane beside the squad's
/// corridor, his wander and pace, his personal space, his own route for the
/// final stretch, and how he yields to vehicles.
#[derive(Clone, Debug, Serialize, Deserialize)]
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

/// The one infantry body every soldier shares (navigation's clearance, cover,
/// the fit authority) and round flight (the fixture's `physics` section).
/// A vehicle's body is its type's hull.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BodyRules {
    pub soldier_radius_m: f64,
    pub soldier_height_m: f64,
    pub infantry_eye_m: f64,
    pub infantry_muzzle_m: f64,
    /// Where rounds aim on a soldier, above his feet.
    pub infantry_aim_m: f64,
    /// A soldier's middle, above his feet: where blast and near misses
    /// reach him.
    pub infantry_center_m: f64,
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
    /// A contact's area is this many times the footprint radius of the unit
    /// that caused it (its catalog type's: a hull's half-diagonal, or half a
    /// full squad's spread plus a soldier's body). So the area's size tells
    /// a vehicle's from a squad's, never which unit it is or where exactly.
    pub contact_radius_factor: f64,
    pub contact_lifetime_s: f64,
    /// Units carry their own loudness (their type's `sound`); shots carry
    /// this far.
    pub hearing_shot_m: f64,
    pub sound_bucket_s: f64,
    /// Height above ground tested for ground visibility.
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

/// The game's rules (`fixtures/village.json` with its catalog). Loading
/// them checks what crosses sections: every mount names a weapon row.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(try_from = "UncheckedRules")]
pub struct Rules {
    pub tick_hz: u32,
    pub movement: MovementRules,
    pub infantry_movement: InfantryMovementRules,
    pub physics: BodyRules,
    /// Every unit type and prop type (the catalog, `fixtures/units/` and
    /// `fixtures/props/`).
    pub catalog: crate::catalog::Catalog,
    pub pushing: PushingRules,
    pub ricochet: RicochetRules,
    pub guided: crate::ballistics::GuidedRules,
    pub sensors: SensorRules,
    /// The one forest rule (Q16, Q-G8b).
    pub forests: ForestRules,
    /// How fast each surface kind is (Q-G4): one row per kind.
    pub surfaces: std::collections::BTreeMap<crate::map::SurfaceKind, SurfaceRule>,
    /// The weapon rows, `extends` resolved (`weapons::resolve_weapons`).
    pub weapons: crate::weapons::WeaponRules,
    pub service: ServiceRules,
    pub suppression: SuppressionRules,
    pub cover: CoverRules,
    pub ground: GroundRules,
    pub buildings: BuildingRules,
    pub garrison: GarrisonRules,
}

/// [`Rules`] as read, before the checks across its sections.
#[derive(Deserialize)]
struct UncheckedRules {
    tick_hz: u32,
    movement: MovementRules,
    infantry_movement: InfantryMovementRules,
    physics: BodyRules,
    catalog: crate::catalog::Catalog,
    pushing: PushingRules,
    ricochet: RicochetRules,
    guided: crate::ballistics::GuidedRules,
    sensors: SensorRules,
    forests: ForestRules,
    #[serde(deserialize_with = "surface_table")]
    surfaces: std::collections::BTreeMap<crate::map::SurfaceKind, SurfaceRule>,
    #[serde(deserialize_with = "crate::weapons::resolve_weapons")]
    weapons: crate::weapons::WeaponRules,
    service: ServiceRules,
    suppression: SuppressionRules,
    cover: CoverRules,
    ground: GroundRules,
    buildings: BuildingRules,
    garrison: GarrisonRules,
}

impl TryFrom<UncheckedRules> for Rules {
    type Error = crate::catalog::CatalogError;
    fn try_from(r: UncheckedRules) -> Result<Self, Self::Error> {
        r.catalog.check_weapons(&r.weapons)?;
        r.suppression
            .check()
            .map_err(|error| crate::catalog::CatalogError::Invalid {
                section: "rules",
                id: "suppression".into(),
                error,
            })?;
        Ok(Rules {
            tick_hz: r.tick_hz,
            movement: r.movement,
            infantry_movement: r.infantry_movement,
            physics: r.physics,
            catalog: r.catalog,
            pushing: r.pushing,
            ricochet: r.ricochet,
            guided: r.guided,
            sensors: r.sensors,
            forests: r.forests,
            surfaces: r.surfaces,
            weapons: r.weapons,
            service: r.service,
            suppression: r.suppression,
            cover: r.cover,
            ground: r.ground,
            buildings: r.buildings,
            garrison: r.garrison,
        })
    }
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
    /// A soldier changing windows, and the squadmate he trades with, hold
    /// fire this long.
    pub window_swap_s: f64,
    /// A soldier who changed windows keeps his new one at least this long.
    pub window_hold_s: f64,
}

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
    // Which bodies cover, and how well, is a prop type's `cover_tier`
    // column; a live vehicle covers by its weight class (Q24).
    /// Ground cover: a crater at least `crater_min_fill` full.
    pub crater: CoverTier,
    pub crater_min_fill: f64,
    /// A body within this of a soldier can cover him (Q20).
    pub reach_m: f64,
    /// A squad's area reaches this far beyond half its spread: its
    /// soldiers seek cover anywhere inside it (Q7).
    pub search_m: f64,
    /// A soldier who cannot engage steps out, best cover first, within this
    /// far; beyond it, to the nearest place in the area he can engage from
    /// (D3, Q8).
    pub step_out_m: f64,
    /// A squad re-resolves its cover at most this often (Q11).
    pub reresolve_s: f64,
    /// A threat bearing swing that re-resolves cover (Q11).
    pub swing_deg: f64,
    /// A covering vehicle that moves this far re-resolves its users (Q24).
    pub vehicle_moved_m: f64,
    /// A soldier out on his lean fires at most this long before he tucks
    /// back in: he steps out, fires a burst, and ducks back.
    pub lean_burst_s: f64,
    /// How long he stays tucked in before leaning out again.
    pub lean_tuck_s: f64,
    /// How long he stays out after his last round from there: the firing
    /// pose's hold, which the pose driver reads too.
    pub lean_hold_s: f64,
    /// The farthest he leans from where he stands: a corner further off is
    /// a walk, the cover search's business.
    pub lean_max_m: f64,
    /// How far beyond his body radius a lean point keeps from the footprint.
    pub lean_clear_m: f64,
    /// A lean point is claimed like a place: none within this of another
    /// soldier's place or lean point.
    pub lean_apart_m: f64,
    /// A spot stands this far off the face it hides behind, beyond the
    /// soldier's own radius.
    pub standoff_m: f64,
    /// A face counts as turned away from the threat when the cosine between
    /// its normal and the threat's direction is at most this: one the threat
    /// sees nearly edge-on hides nobody.
    pub away_cos: f64,
    /// The cover search looks this far beyond the squad's area.
    pub search_slack_m: f64,
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
    /// A track's wheel line: this share of the hull's half width off its
    /// axis, where tracks wear the ground.
    pub track_gauge: f64,
    /// A vehicle knocking through trees clears this much beyond its hull on
    /// either side (Q16).
    pub lane_margin_m: f64,
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
