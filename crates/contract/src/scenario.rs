//! Authored battle setup: map, rules and initial units. Numeric rules come
//! from the fixture and carry no defaults; only authored setup (unit wear,
//! events, scripts, an opponent) may be left out.
use crate::ids::{Side, Tick};
use crate::map::{MapDefinition, PropDefinition};
use crate::movement::MovementRules;
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

/// How routes are planned. A unit holds where it is while its route is
/// worked out, a share of each tick's allowance at a time. A long ordered
/// leg goes by road where a road journey beats driving straight there.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NavigationRules {
    /// Motion steps and route work allowed to prove one garrison entry.
    /// A long leg's initial route uses the live planner's separate bound;
    /// shorter initial routes and repeated searches spend this allowance.
    pub move_validation_work: u32,
    /// How much of each end of a leg certification drives, and of the
    /// approach to a vehicle bend, intermediate hull or prop: the stretch
    /// between is taken on the route.
    pub move_rehearsal_m: f64,
    /// Planning work every side's units share each tick: one unit is about
    /// one grid cell searched.
    pub work_per_tick: u32,
    /// How far a search looks before it gives the route up as blocked: this
    /// many cells, plus `search_cells_per_m` for every metre between its
    /// two ends.
    pub search_cells_base: u32,
    pub search_cells_per_m: u32,
    /// An ordered move leg strictly longer than this, as the crow flies
    /// when the leg starts, is driven by the fastest policy: by road where
    /// that is quicker.
    pub road_leg_m: f64,
    /// How far from each end of a leg a road is looked for.
    pub road_access_m: f64,
    /// A unit sent where it has no room to stand stops at the nearest room
    /// it can reach, or short of it on its way there: it drives a detour to
    /// the nearer only while each metre of ground gained costs at most this
    /// many metres of extra driving.
    pub stop_short_detour_ratio: f64,
}

impl NavigationRules {
    /// The cells a search between two points `distance_m` apart may expand.
    pub fn search_limit(&self, distance_m: f64) -> usize {
        self.search_cells_base as usize + (self.search_cells_per_m as f64 * distance_m) as usize
    }

    fn check(&self) -> Result<(), String> {
        if self.move_validation_work == 0 {
            return Err("move validation work must be positive".into());
        }
        if self.work_per_tick == 0 {
            return Err("work_per_tick must be positive: no route would ever finish".into());
        }
        if self.search_cells_base == 0 {
            return Err("search_cells_base must be positive: no search would look anywhere".into());
        }
        if !self.stop_short_detour_ratio.is_finite() || self.stop_short_detour_ratio < 0.0 {
            return Err("stop_short_detour_ratio must be finite and not negative".into());
        }
        for (name, v) in [
            ("road_leg_m", self.road_leg_m),
            ("road_access_m", self.road_access_m),
            ("move_rehearsal_m", self.move_rehearsal_m),
        ] {
            if !v.is_finite() || v < 0.0 {
                return Err(format!(
                    "{name} must be a distance: finite and not negative"
                ));
            }
        }
        Ok(())
    }
}

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

/// The one infantry body every soldier shares (navigation's clearance, cover,
/// the fit authority) and round flight (the fixture's `physics` section).
/// A vehicle's body is its type's hull.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BodyRules {
    pub soldier_radius_m: f64,
    pub soldier_height_m: f64,
    pub infantry_eye_m: f64,
    pub infantry_muzzle_m: f64,
    /// Where rounds aim on a soldier, above his feet.
    pub infantry_aim_m: f64,
    /// Nominal aim height as a fraction of a vehicle body's full height.
    pub vehicle_aim_height_fraction: f64,
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
#[serde(deny_unknown_fields)]
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
    /// One flare restored every `flare_service_s`, each costing
    /// `stock_per_flare`, for any unit that carries flares and stands still
    /// in reach (a jet never does: it withdraws to rearm).
    pub flare_service_s: f64,
    pub stock_per_flare: u32,
}

/// Browser input controls carried alongside the simulation rules. The browser
/// owns their behavior (`reverseZone.ts`); the shared loader admits the
/// authored shape so it cannot be mistaken for an arbitrary rules section.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Controls {
    pub reverse_zone_length_m: f64,
    pub reverse_zone_margin_m: f64,
}

/// The game's rules (`fixtures/game.json` with its catalog). Loading
/// them checks what crosses sections: every mount names a weapon row.
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
    pub hull_limits: HullLimits,
    pub air_speed_limits_kmh: AirSpeedLimits,
    pub air: AirRules,
    pub shouts: ShoutRules,
}

/// Timing data for simulation-emitted battle barks.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShoutRules {
    pub cooldown_s: f64,
    pub talk_s: std::collections::BTreeMap<crate::observation::ShoutTrigger, f64>,
}

impl ShoutRules {
    fn check(&self) -> Result<(), String> {
        for trigger in crate::observation::ShoutTrigger::ALL {
            if !self.talk_s.get(&trigger).is_some_and(|s| *s > 0.0) {
                let name = serde_json::to_value(trigger).unwrap_or_default();
                return Err(format!("shouts: talk_s needs a positive time for {name}"));
            }
        }
        if self.cooldown_s > 0.0 {
            Ok(())
        } else {
            Err("shouts: cooldown_s must be positive".into())
        }
    }
}

/// How aircraft fly: the heights of each air layer, and the flight model of
/// each kind of airframe (D1, D2, D19). High air is the one layer that bends
/// physics: its heights are lowered from a real jet's for play, and every
/// treatment that follows from flying there keys on the layer.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AirRules {
    pub low_air: LowAirLayer,
    pub high_air: HighAirLayer,
    pub rotor: RotorFlight,
    pub fixed_wing: FixedWingFlight,
    /// Aircraft closer than this, centre to centre, drift apart.
    pub separation_m: f64,
}

/// The low-air layer's heights.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LowAirLayer {
    pub cruise_agl_m: f64,
}

/// The high-air layer's heights: where it cruises, and the hard floor
/// nothing in it flies below (D20).
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HighAirLayer {
    pub cruise_agl_m: f64,
    pub floor_agl_m: f64,
}

/// A helicopter's flight: clearance over obstacles and settling at a goal.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RotorFlight {
    pub clearance_m: f64,
    pub ceiling_agl_m: f64,
    pub obstacle_m: f64,
    /// Seconds an aircraft takes to gather its cruise speed from a hover,
    /// and to shed it: a helicopter's, not a car's.
    pub acceleration_s: f64,
    /// Braking for its goal it runs past by this share of the last leg's
    /// length, at most `settle_max_m`, then eases back onto the point.
    pub settle_share: f64,
    pub settle_max_m: f64,
}

/// A winged aircraft's flight, which never stops: its attack and dogfight
/// bands, afterburner ramp, and the margin it turns inside the map's edge.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixedWingFlight {
    pub strafe_ceiling_m: f64,
    pub bomb_ceiling_m: f64,
    pub dogfight_band_agl_m: [f64; 2],
    pub dogfight_break_m: f64,
    pub afterburner_ramp_s: f64,
    /// A ground attack fired on afterburner scatters this many times its
    /// weapon's authored dispersion: a fast pass is hard to stabilise.
    pub afterburner_scatter_multiplier: f64,
    pub edge_margin_m: f64,
    /// The cone, either side of a winged aircraft's heading, it counts as
    /// nose-on: where a row it aims by the nose fires, and what an attack
    /// pass needs before its dive begins.
    pub nose_on_deg: f64,
}

impl AirRules {
    fn check(&self) -> Result<(), String> {
        let low = &self.low_air;
        let high = &self.high_air;
        let rotor = &self.rotor;
        let wing = &self.fixed_wing;
        let rotor_ok = low.cruise_agl_m > 0.0
            && rotor.clearance_m > 0.0
            && rotor.obstacle_m + rotor.clearance_m <= rotor.ceiling_agl_m
            && low.cruise_agl_m <= rotor.ceiling_agl_m
            && rotor.acceleration_s > 0.0
            && (0.0..1.0).contains(&rotor.settle_share)
            && rotor.settle_max_m >= 0.0;
        let wing_ok = high.floor_agl_m > 0.0
            && high.floor_agl_m <= wing.strafe_ceiling_m
            && high.floor_agl_m <= wing.bomb_ceiling_m
            && wing.strafe_ceiling_m <= high.cruise_agl_m
            && wing.bomb_ceiling_m <= high.cruise_agl_m
            && wing.dogfight_band_agl_m[0] >= high.floor_agl_m
            && wing.dogfight_band_agl_m[0] <= wing.dogfight_band_agl_m[1]
            && wing.dogfight_break_m > 0.0
            && wing.afterburner_ramp_s > 0.0
            && wing.afterburner_scatter_multiplier >= 1.0
            && wing.edge_margin_m >= 0.0
            && wing.nose_on_deg > 0.0
            && wing.nose_on_deg < 180.0;
        let ok = rotor_ok && wing_ok && self.separation_m > 0.0;
        if ok {
            Ok(())
        } else {
            Err(
                "air: low-air cruise must be positive and within the rotor ceiling; the high-air floor must not exceed the fixed-wing ceilings, those ceilings must not exceed high-air cruise, the dogfight band must be ordered from the floor up, and air timings/margins must be positive, afterburner scatter at least 1, and the nose-on cone within (0, 180) degrees"
                    .into(),
            )
        }
    }

    /// How high `mobility` cruises over the ground: its own override, else
    /// its layer's height, never under its layer's floor (D40). The one
    /// height lookup shared by simulation and presentation readers.
    pub fn cruise_agl_m(&self, mobility: &crate::catalog::Mobility) -> f64 {
        let layer = mobility.layer();
        let authored = match (mobility, layer) {
            (_, crate::catalog::AltitudeLayer::Ground) => return 0.0,
            (
                crate::catalog::Mobility::Air {
                    cruise_agl_m: Some(own),
                    ..
                },
                _,
            ) => *own,
            (_, crate::catalog::AltitudeLayer::LowAir) => self.low_air.cruise_agl_m,
            (_, crate::catalog::AltitudeLayer::HighAir) => self.high_air.cruise_agl_m,
        };
        authored.max(self.floor_agl_m(layer).unwrap_or(0.0))
    }

    /// The hard floor nothing in `layer` flies below (D20); None for a layer
    /// that has none, where no fixed-wing flight is admitted.
    pub fn floor_agl_m(&self, layer: crate::catalog::AltitudeLayer) -> Option<f64> {
        match layer {
            crate::catalog::AltitudeLayer::HighAir => Some(self.high_air.floor_agl_m),
            _ => None,
        }
    }
}

/// The largest hull each drive may have: what the map generator and the
/// route planner are proven against, by tests that drive hulls built at
/// these limits. A unit past them is refused where the rules load, so every
/// admitted unit is covered by those tests.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HullLimits {
    pub tracked: DriveLimits,
    pub wheeled: DriveLimits,
    /// A helicopter's hull, without its rotor disc.
    pub rotor: DriveLimits,
    /// A fixed-wing aircraft's hull.
    pub fixed_wing: DriveLimits,
}

impl HullLimits {
    /// The limits of an aircraft that flies by `flight`.
    pub fn flight(&self, flight: crate::catalog::FlightModel) -> &DriveLimits {
        match flight {
            crate::catalog::FlightModel::Rotor => &self.rotor,
            crate::catalog::FlightModel::FixedWing => &self.fixed_wing,
        }
    }
}

/// The fastest an aircraft of each flight model may cruise, km/h.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AirSpeedLimits {
    pub rotor: f64,
    pub fixed_wing: f64,
}

impl AirSpeedLimits {
    /// The fastest an aircraft that flies by `flight` may cruise, km/h.
    pub fn flight(&self, flight: crate::catalog::FlightModel) -> f64 {
        match flight {
            crate::catalog::FlightModel::Rotor => self.rotor,
            crate::catalog::FlightModel::FixedWing => self.fixed_wing,
        }
    }

    /// The first aircraft of `catalog` cruising past its flight model's limit.
    fn check(&self, catalog: &crate::catalog::Catalog) -> Result<(), crate::catalog::CatalogError> {
        use crate::catalog::Mobility;
        for index in catalog.indices() {
            let Mobility::Air {
                cruise_kmh, flight, ..
            } = catalog.get(index).mobility
            else {
                continue;
            };
            let (name, limit) = (flight.name(), self.flight(flight));
            if cruise_kmh > limit {
                return Err(crate::catalog::CatalogError::Invalid {
                    section: "units",
                    id: catalog.id(index).into(),
                    error: format!(
                        "its {name} cruise_kmh {cruise_kmh} is past the limit of {limit} (air_speed_limits_kmh)"
                    ),
                });
            }
        }
        Ok(())
    }
}

/// One drive's largest hull; a tracked hull pivots and has no turn limit.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DriveLimits {
    pub half_width_m: f64,
    pub half_length_m: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turning_radius_m: Option<f64>,
}

impl HullLimits {
    /// The first unit of `catalog` whose hull passes its drive's limits.
    fn check(&self, catalog: &crate::catalog::Catalog) -> Result<(), crate::catalog::CatalogError> {
        use crate::catalog::Mobility;
        for index in catalog.indices() {
            let unit = catalog.get(index);
            let Some(hull) = unit.hull() else { continue };
            let (drive, limits, radius) = match unit.mobility {
                Mobility::Tracked { .. } => ("tracked", &self.tracked, None),
                Mobility::Wheeled {
                    turning_radius_m, ..
                } => ("wheeled", &self.wheeled, Some(turning_radius_m)),
                Mobility::Air { flight, .. } => (flight.name(), self.flight(flight), None),
                // A squad has no hull; the catalog refuses a hull on foot.
                Mobility::Foot { .. } => continue,
            };
            let [length, width, _] = hull.half_extents_m;
            let past = [
                ("half width", width, Some(limits.half_width_m)),
                ("half length", length, Some(limits.half_length_m)),
            ]
            .into_iter()
            .chain(radius.map(|r| ("turning radius", r, limits.turning_radius_m)))
            .find(|(_, value, limit)| limit.is_some_and(|limit| *value > limit));
            if let Some((what, value, limit)) = past {
                return Err(crate::catalog::CatalogError::Invalid {
                    section: "units",
                    id: catalog.id(index).into(),
                    error: format!(
                        "its {drive} {what} {value} m is past the limit of {} m (hull_limits)",
                        limit.unwrap()
                    ),
                });
            }
        }
        Ok(())
    }
}

/// [`Rules`] as read, before the checks across its sections.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UncheckedRules {
    // These sections belong to presentation and browser input, never simulation.
    #[serde(rename = "presentation")]
    _presentation: Option<serde::de::IgnoredAny>,
    #[serde(rename = "controls")]
    _controls: Option<Controls>,
    // The battle lab's default; a battle supplies its own seed separately.
    #[serde(rename = "seed")]
    _seed: Option<u64>,
    tick_hz: u32,
    commands: CommandRules,
    movement: MovementRules,
    navigation: NavigationRules,
    formation: FormationRules,
    infantry_movement: InfantryMovementRules,
    physics: BodyRules,
    catalog: crate::catalog::Catalog,
    pushing: PushingRules,
    ricochet: RicochetRules,
    guided: crate::ballistics::GuidedRules,
    launch: LaunchRules,
    sensors: SensorRules,
    forests: ForestRules,
    #[serde(deserialize_with = "crate::weapons::resolve_weapons")]
    weapons: crate::weapons::WeaponRules,
    service: ServiceRules,
    suppression: SuppressionRules,
    cover: CoverRules,
    ground: GroundRules,
    buildings: BuildingRules,
    garrison: GarrisonRules,
    carriage: CarriageRules,
    bail_out: std::collections::BTreeMap<BailOutHost, BailOutRow>,
    hull_limits: HullLimits,
    air_speed_limits_kmh: AirSpeedLimits,
    air: AirRules,
    shouts: ShoutRules,
}

impl TryFrom<UncheckedRules> for Rules {
    type Error = crate::catalog::CatalogError;
    fn try_from(r: UncheckedRules) -> Result<Self, Self::Error> {
        r.catalog.check_weapons(&r.weapons)?;
        r.hull_limits.check(&r.catalog)?;
        r.air_speed_limits_kmh.check(&r.catalog)?;
        for (id, checked) in [
            ("suppression", r.suppression.check()),
            ("navigation", r.navigation.check()),
            ("movement", r.movement.speed_factors.check()),
            ("sensors", r.sensors.check()),
            ("forests", r.forests.check(&r.catalog)),
            ("formation", r.formation.check()),
            ("commands", r.commands.check()),
            ("carriage", r.carriage.check()),
            ("air", r.air.check()),
            ("shouts", r.shouts.check()),
        ] {
            checked.map_err(|error| crate::catalog::CatalogError::Invalid {
                section: "rules",
                id: id.into(),
                error,
            })?;
        }
        // Every host a passenger can be thrown out of has its row, so a
        // bail-out never finds one missing mid-battle.
        for (host, name) in [
            (BailOutHost::Building, "building"),
            (BailOutHost::Ground, "ground"),
            (BailOutHost::Air, "air"),
        ] {
            if !r.bail_out.contains_key(&host) {
                return Err(crate::catalog::CatalogError::Invalid {
                    section: "rules",
                    id: "bail_out".into(),
                    error: format!("no row for {name} hosts"),
                });
            }
        }
        Ok(Rules {
            tick_hz: r.tick_hz,
            commands: r.commands,
            movement: r.movement,
            navigation: r.navigation,
            formation: r.formation,
            infantry_movement: r.infantry_movement,
            physics: r.physics,
            catalog: r.catalog,
            pushing: r.pushing,
            ricochet: r.ricochet,
            guided: r.guided,
            launch: r.launch,
            sensors: r.sensors,
            forests: r.forests,
            weapons: r.weapons,
            service: r.service,
            suppression: r.suppression,
            cover: r.cover,
            ground: r.ground,
            buildings: r.buildings,
            garrison: r.garrison,
            carriage: r.carriage,
            bail_out: r.bail_out,
            hull_limits: r.hull_limits,
            air_speed_limits_kmh: r.air_speed_limits_kmh,
            air: r.air,
            shouts: r.shouts,
        })
    }
}

impl Rules {
    /// These rules with a scenario's `overrides` merged in, read and checked
    /// again as the shipped rules are, so an override is held to every rule
    /// a shipped row is. The patch is shaped like the authored rules: a rules
    /// section of `game.json` (`weapons`, `sensors`, `air`, ...) or a catalog
    /// section (`units`, `soldiers`, `parts`, `roles`, `props`). It merges
    /// into the resolved rows as an `extends` child merges into its parent
    /// ([`crate::catalog::merge_entry`]); a row the rules lack is added, and
    /// may `extends` any resolved row of its section.
    pub fn overridden(
        self,
        overrides: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<Rules, String> {
        use crate::catalog::{merge, merge_entry, SECTIONS};
        if overrides.is_empty() {
            return Ok(self);
        }
        let mut document = serde_json::to_value(&self).map_err(|e| e.to_string())?;
        for (section, patch) in overrides {
            let in_catalog = SECTIONS.contains(&section.as_str());
            let base = match in_catalog {
                // A resolved catalog travels as one document.
                true => document["catalog"][0].get_mut(section),
                false if section == "catalog" => None,
                false => document.get_mut(section),
            }
            .ok_or_else(|| format!("{section} is not a section of the rules or the catalog"))?;
            if !in_catalog {
                merge(base, patch);
                continue;
            }
            let (Some(rows), Some(patch)) = (base.as_object_mut(), patch.as_object()) else {
                return Err(format!("{section} is a table of rows by id"));
            };
            for (id, row) in patch {
                match rows.get_mut(id) {
                    Some(base) => merge_entry(section, base, row),
                    None => {
                        rows.insert(id.clone(), row.clone());
                    }
                }
            }
        }
        serde_json::from_value(document).map_err(|e| e.to_string())
    }
}

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
}

/// Buildings as fighting positions (L08–L10): soldier capacity and the
/// cover strength occupants get. Their integrity and ruin are the body
/// table's `building` row (34c).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildingRules {
    pub capacity_soldiers: u32,
    /// Building cover strength in [0, 1] (contracts: building strength).
    pub cover_strength: f64,
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
    /// A soldier changing windows, and the squadmate he trades with, hold
    /// fire this long.
    pub window_swap_s: f64,
    /// A soldier who changed windows keeps his new one at least this long.
    pub window_hold_s: f64,
}

/// Timing and reach for squads boarding a carrier.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CarriageRules {
    pub board_s: f64,
    pub reach_m: f64,
    pub unload_search_m: f64,
}

impl CarriageRules {
    fn check(&self) -> Result<(), String> {
        for (name, value) in [
            ("board_s", self.board_s),
            ("reach_m", self.reach_m),
            ("unload_search_m", self.unload_search_m),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(format!("carriage.{name} must be finite and positive"));
            }
        }
        Ok(())
    }
}

/// The physical kind of host a forced passenger exit comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BailOutHost {
    Building,
    Ground,
    Air,
}

/// Injury and placement rules for one kind of bail-out host.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BailOutRow {
    #[serde(default)]
    pub squad_loss: Option<f64>,
    pub death: f64,
    pub hp_lost: [f64; 2],
    pub search_m: f64,
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
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
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
    /// A soldier who found nowhere to fight from searches again once the
    /// enemy has moved this far, or he has, or his side learns of a body or
    /// a crater within his search.
    pub step_out_retry_m: f64,
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
#[serde(deny_unknown_fields)]
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
    /// A weapon's scorch radius as a fraction of its blast radius.
    pub scorch_radius_fraction: f64,
    /// Scorch a burst adds at its centre, falling linearly to the rim.
    pub scorch_per_burst: f64,
    /// A burst of at least this blast radius marks the ground fully; a
    /// smaller one adds scorch in proportion to its radius's share of this,
    /// and crater depth by the square of it, so a small cannon shell leaves
    /// a dark pock where a shell leaves a crater and a black scorch.
    pub mark_reference_blast_m: f64,
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

/// A unit that starts the scenario already worn.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
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
