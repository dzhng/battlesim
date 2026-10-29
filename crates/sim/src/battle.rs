//! The one battle authority: commands in, fixed ticks, side observations out.
use std::collections::{BTreeMap, BTreeSet, VecDeque};

use contract::bundle::ControllerKind;
use contract::catalog::{AltitudeLayer, Destroyed, TypeIndex};
use contract::command::{
    BuildingPlacement, BuildingPreviewRequest, CommandAck, CommandEnvelope, Engagement,
    MovePlacement, MovePreviewRequest, Order, OrderError, RoutePolicy, TargetRef,
};
use contract::ids::{Side, Tick, UnitId};
use contract::map::{MoverClass, PropDefinition};
use contract::observation::{
    AirTransit, AirTransitDirection, Blast, Corpse, EncounterStatus, FallenBody, FallingAirframe,
    Grind, GuidedMissile, KnownProp, MemberOrder, MoveState, ObservationFrame, OwnUnit, Posture,
    PropDisplayPose, SegmentHit, SegmentRicochet, ServiceStatus, SoundCue, SquadArea, UnitSight,
    VisibilityField, VisibleSegment,
};
use contract::scenario::{
    EncounterRules, EventAction, Opponent, Rules, ScenarioDefinition, ScenarioEvent, ScriptedOrder,
    UnitCondition,
};
use contract::script::{CompiledScript, ScriptInput};
use serde::{Deserialize, Serialize};

use crate::arrangement;

mod controllers;
mod dispatch;
mod fire;
mod flown;
mod replay_identity;
mod skirmish_turn;
mod spawn;
mod state_digest;
use crate::cover;
use crate::damage::{self, DamageContext, HullResolver};
use crate::deployment;
use crate::digest::{self, Digest};
use crate::encounter::{Defender, Referee};
use crate::flight::{
    self, Body, BodyId, FlightEvent, Pose, ProjectileId, Projectiles, Shape, Struck,
};
use crate::garrison;
use crate::ground::{GroundLayer, KnownGround, Wear};
use crate::hearing;
use crate::knowledge::SideKnowledge;
use crate::math::{body_distance, v2, v3, Obb2, Rotation, V2, V3};
use crate::movement::{self, MovementContext, SideGeometry};
use crate::navigation::RoadNet;
use crate::route_planner::RoutePlanner;
use crate::sensing::{self, Sighting};
use crate::shouts::{self, ShoutEvidence, ShoutLedger};
use crate::sight;
use crate::structures::Structures;
use crate::supply;
use crate::units::{self, MoveOrder, Pursuit, Soldier, Unit, UnitOrder};
use crate::visibility::OcclusionGrid;
use crate::weapons::{self, Arsenal, FireContext, Support, Target, VEHICLE_BODY_BASE};
use crate::world::{PropId, WorldGeometry};
use contract::random::Rng;
use flown::{clip_to_seen, Flown};
use replay_identity::{config_digest, scenario_digest};
use spawn::spawn_unit;

/// Ticks between ground-visibility sweeps for each side ([`fog_phase`]).
/// Identification runs every [`sensing::SENSE_EVERY`] ticks per observer;
/// the fog display lags by at most this many ticks.
const FOG_INTERVAL_TICKS: u64 = 6;

/// The tick within each fog interval on which `side` sweeps: the two
/// combatants half an interval apart, so their sweeps never share a tick,
/// and the passive neutral side on blue's.
fn fog_phase(side: Side) -> u64 {
    match side {
        Side::Red => FOG_INTERVAL_TICKS / 2,
        Side::Blue | Side::Neutral => 0,
    }
}
/// Seed salt for each side's observation-uncertainty stream (contact placement),
/// kept apart from combat and policy randomness.
const OBSERVATION_STREAM: u64 = 0x6f62_7365_7276_6531;
/// Seed salt for combat randomness (spread, area aim points).
const COMBAT_STREAM: u64 = 0x636f_6d62_6174_2121;
/// Seed salt for blast fragment sampling, kept apart from aim randomness.
const DAMAGE_STREAM: u64 = 0x6461_6d61_6765_2121;
/// Seed salt for ricochet rolls and scatter, drawn during flight.
const RICOCHET_STREAM: u64 = 0x7269_636f_6368_6574;
/// An attack reaching its target's last reported place within this distance,
/// without regaining sight, is complete.
const PURSUIT_ARRIVAL_M: f64 = 5.0;

/// Where a round in flight came from, for consequences and visibility.
#[derive(Clone, Copy, Debug)]
pub struct Round {
    pub weapon: usize,
    pub unit: UnitId,
    pub side: Side,
    /// A seeker missile keeps its target independently of launcher support.
    pub seeker_target: Option<Target>,
    /// The unit it was fired at, for its proximity fuse (seeker or not):
    /// a fuse bursts near whatever its round was aimed at.
    pub fused_target: Option<UnitId>,
    /// The height band its shooter fired it from, for its whole flight:
    /// what an interceptor reaches it by. Never read from its height.
    pub layer: AltitudeLayer,
}

/// Where everything stood at one moment of a tick: each unit's pose, and
/// each soldier's position in unit then member order.
struct Poses {
    units: Vec<Pose>,
    soldiers: Vec<V3>,
}

fn xyz(p: V3) -> [f64; 3] {
    [p.x, p.y, p.z]
}

/// Completed work brackets for native cost reports. The ordinary tick uses
/// the same path with a no-op callback; measurement never enters battle state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TickPhase {
    Orders,
    Navigation,
    Movement,
    Flight,
    Sight,
    Fog,
    Learning,
    Weapons,
    Observation,
    Other,
}

/// A snapshot of the battle's size (see [`Battle::load`]).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Load {
    pub living_units: usize,
    pub living_soldiers: usize,
    pub corpses: usize,
    pub wrecks: usize,
    pub active_projectiles: usize,
    pub rounds_launched: u64,
    pub path_searches: u64,
    /// Units holding for a route, and the planning work the latest tick spent.
    pub routes_pending: usize,
    pub planning_work: u64,
    /// Cells both sides' planning grids have worked out again since the
    /// battle began, as the sides learned of bodies and cleared ground.
    pub grid_cells_relaid: u64,
    /// Bytes the ground layer holds.
    pub ground_bytes: usize,
    /// Bytes both sides' learned copies of it hold.
    pub known_ground_bytes: usize,
}

/// Everything needed to reproduce a battle in the same build: the setup
/// identity, the seed and every sequenced command from both sides.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Replay {
    pub engine_build: String,
    pub scenario_digest: String,
    pub config_digest: String,
    pub seed: u64,
    /// `(applied_tick, command)` in acceptance order.
    pub accepted: Vec<(Tick, CommandEnvelope)>,
    /// Authored victory/error terminal state captured after the last tick.
    #[serde(default)]
    pub script_victory: Option<bool>,
    #[serde(default)]
    pub script_error: Option<String>,
    #[serde(default)]
    pub script_terminal_tick: Option<Tick>,
    /// Controller choices used for the recorded session. Replays restore the
    /// same passive/active policy before consuming accepted commands.
    #[serde(default)]
    pub controllers: Vec<(String, ControllerKind)>,
    #[serde(default)]
    pub controller_transitions: Vec<ControllerTransition>,
    #[serde(default)]
    pub script_events: Vec<contract::script::ScriptEvent>,
    /// Authored script outputs at the tick where they became authoritative.
    #[serde(default)]
    pub script_trace: Vec<ScriptTrace>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ControllerTransition {
    pub tick: Tick,
    pub player: String,
    pub controller: ControllerKind,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScriptTrace {
    pub tick: Tick,
    #[serde(default)]
    pub events: Vec<contract::script::ScriptEvent>,
    #[serde(default)]
    pub victory: Option<bool>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ReplayError {
    BuildMismatch,
    ScenarioMismatch,
    ConfigMismatch,
}

/// The shared native/Wasm engine fingerprint generated from simulation build inputs.
pub const ENGINE_BUILD_ID: &str = env!("SIM_ENGINE_BUILD_ID");

impl std::fmt::Display for ReplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::BuildMismatch => "replay was recorded by a different simulation build",
            Self::ScenarioMismatch => "replay does not match this scenario",
            Self::ConfigMismatch => "replay does not match these simulation rules",
        })
    }
}

pub struct Battle {
    skirmish: Option<crate::skirmish::Skirmish>,
    world: WorldGeometry,
    rules: Rules,
    seed: u64,
    tick: Tick,
    units: Vec<Unit>,
    /// Props authored with the map (ids below this) are known to every side.
    authored_props: PropId,
    /// The bodies shoved this tick, by whom and where the hull met them.
    shoved: Vec<movement::Shove>,
    sides: [SideGeometry; 3],
    /// The map's road graph, built once: public terrain, the same for both
    /// sides.
    roads: RoadNet,
    /// Every unit's route request in progress.
    planner: RoutePlanner,
    events: VecDeque<ScenarioEvent>,
    scripts: VecDeque<ScriptedOrder>,
    authored_scripts: Vec<(CompiledScript, Side, Option<String>)>,
    victory_script: Option<CompiledScript>,
    script_victory: Option<bool>,
    script_error: Option<String>,
    script_terminal_tick: Option<Tick>,
    script_events: Vec<contract::script::ScriptEvent>,
    script_trace: Vec<ScriptTrace>,
    replay_script_trace: BTreeMap<Tick, ScriptTrace>,
    /// The controllers of players with no skirmish seat; a seated player's
    /// is its seat's (`Battle::controller`).
    controllers: BTreeMap<String, ControllerKind>,
    player_ids: Vec<String>,
    controller_transitions: Vec<ControllerTransition>,
    replay_controller_transitions: BTreeMap<Tick, Vec<ControllerTransition>>,
    knowledge: [SideKnowledge; 3],
    occlusion: OcclusionGrid,
    fog: [VisibilityField; 3],
    /// Each side's sightings as of each observer's latest identification run
    /// (27 perf), in observer then target order.
    sightings: [Vec<Sighting>; 3],
    /// Units that fired during the current sound bucket.
    fired: BTreeSet<UnitId>,
    audible: [Vec<SoundCue>; 3],
    arsenal: Arsenal,
    /// Building health and the ruins collapses left.
    structures: Structures,
    /// Craters and wear on the ground.
    ground: GroundLayer,
    projectiles: Projectiles,
    rounds: BTreeMap<ProjectileId, Round>,
    /// The strongest suppression each round in flight has dealt each squad
    /// (by unit index): a round suppresses a squad once however many ticks
    /// it takes to pass it.
    suppressed: BTreeMap<(ProjectileId, usize), f64>,
    combat_rng: Rng,
    damage_rng: Rng,
    ricochet_rng: Rng,
    /// The last soldier id issued; replacements continue from it (L03).
    last_soldier: u32,

    /// This tick's flight events, in order.
    flight_events: Vec<FlightEvent>,
    /// This tick's flown stretch per round.
    segments: Vec<Flown>,
    /// This tick's bursts, by firing side.
    blasts: Vec<(Side, Blast)>,
    next_seq: [u64; 3],
    order_bucket: [(u64, Tick); 3],
    player_next_seq: Vec<u64>,
    player_order_bucket: Vec<(u64, Tick)>,
    /// The observation-bound opponent and its memory (off while replaying).
    opponent: Option<(Opponent, Defender)>,
    skirmish_ai: BTreeMap<String, crate::skirmish_ai::BalancedDoctrine>,
    /// Per-side planner hysteresis. This is deliberately outside the digest;
    /// accepted AI commands are the replay authority.
    team_memory: [crate::skirmish_ai::TeamMemory; 2],
    team_tuning: crate::skirmish_ai::team::TeamTuning,
    /// The fixture's completion referee and its latest verdict.
    referee: Option<(EncounterRules, Referee)>,
    encounter: Option<EncounterStatus>,
    pending: Vec<PreparedCommand>,
    accepted: Vec<(Tick, CommandEnvelope)>,
    /// Recorded commands still to apply when this battle is a replay.
    replaying: Option<VecDeque<(Tick, CommandEnvelope)>>,
    observations: [ObservationFrame; 3],
    refusals: [Vec<contract::command::Refusal>; 3],
    pub(crate) shouts: [ShoutLedger; 3],
    /// Each unit as barks saw it at the start of this tick.
    shout_before: Vec<shouts::Before>,
    shout_evidence: ShoutEvidence,
    shout_fired: BTreeSet<UnitId>,
    shout_kills: BTreeSet<UnitId>,
    shout_under_fire: BTreeSet<UnitId>,
    /// Per listening side, the enemies that shouted unseen since the last
    /// sound bucket.
    shouted_unseen: [BTreeSet<UnitId>; 3],
    scenario_digest: u64,
    config_digest: u64,
    /// Transient bodies (a row with a `lifetime_s`) and the tick each goes.
    expiries: BTreeMap<PropId, Tick>,
    /// Every toppling body that went down: where, which way and when. Each
    /// side publishes those it knocked down or has seen where they stood.
    fallen: BTreeMap<PropId, FallenBody>,
    /// Downed aircraft still falling, in the order they went down.
    crashes: Vec<crate::crash::Crash>,
    air_transits: [Vec<AirTransit>; 3],
    ai_receipts: Vec<AiReceipt>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AiReceipt {
    pub tick: Tick,
    pub player: String,
    pub accepted: bool,
    pub error: Option<OrderError>,
}

fn policy_for_player(seed: u64, index: usize) -> crate::skirmish_ai::BalancedDoctrine {
    let stream = contract::random::mix(
        seed ^ (index as u64 + 1).wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ 0x4241_4c41_4e43_4544,
    );
    crate::skirmish_ai::BalancedDoctrine::new(stream)
}

/// Whether any fog cell under a prop's footprint is seen: large remains
/// (a ruin) are learned when any of them is in view, not only their middle.
fn footprint_seen(field: &VisibilityField, prop: &crate::world::Prop) -> bool {
    let step = field.cell_m / 2.0;
    let rotation = Rotation::new(prop.yaw);
    let (nx, ny) = (
        (2.0 * prop.half.x / step).ceil().max(1.0) as usize,
        (2.0 * prop.half.y / step).ceil().max(1.0) as usize,
    );
    (0..=nx).any(|i| {
        (0..=ny).any(|j| {
            let local = v2(
                -prop.half.x + 2.0 * prop.half.x * i as f64 / nx as f64,
                -prop.half.y + 2.0 * prop.half.y * j as f64 / ny as f64,
            );
            let p = prop.center + rotation.apply(local);
            field.visible(p.x, p.y)
        })
    })
}

/// Authored starting wear: vehicle health, fallen soldiers (their records lie
/// where they stand in the squad's starting arrangement) and spent rounds.
fn wear(unit: &mut Unit, c: &UnitCondition, arsenal: &Arsenal, rules: &Rules) {
    if let (Some(hp), Some(_)) = (c.hp, unit.hull) {
        unit.hp = hp.clamp(1.0, unit.max_hp(rules));
    }
    if let Some(hp) = c.soldier_hp {
        for soldier in &mut unit.members {
            soldier.hp = hp.max(1.0);
        }
    }
    let n = unit.members.len();
    for k in n.saturating_sub(c.casualties as usize)..n {
        let at = unit.members[k].position;
        unit.members[k].fall(at, unit.yaw, None);
    }
    if !unit.members.is_empty() {
        unit.suppression = c.suppression.clamp(0.0, 1.0);
    }
    let specs = arsenal.specs(unit.kind);
    for mount in &mut unit.mounts {
        for (k, &row) in specs[mount.spec].kinds.iter().enumerate() {
            if let (Some(spent), Some(n)) = (
                c.spent.get(&arsenal.weapons[row].id),
                mount.ammo[k].as_mut(),
            ) {
                *n = n.saturating_sub(*spent);
            }
        }
        for cycle in &mut mount.cycles {
            if cycle.loaded.is_some_and(|k| mount.ammo[k] == Some(0)) {
                cycle.loaded = None;
            }
        }
    }
}

impl Battle {
    fn player_faction(&self, side: Side) -> Option<contract::catalog::Faction> {
        self.player_ids.iter().find_map(|id| {
            self.skirmish.as_ref().and_then(|s| {
                s.players
                    .iter()
                    .find(|p| p.id == *id && p.side == side)
                    .map(|p| p.faction)
            })
        })
    }
    /// A dogfight ends when either aircraft can no longer fight or they
    /// draw apart past the break range.
    fn break_dogfights(&mut self) {
        for i in 0..self.units.len() {
            let Some(target) = self.units[i]
                .air
                .as_ref()
                .and_then(|air| air.dogfight_target)
            else {
                continue;
            };
            let broken =
                !crate::air::dogfight::can_dogfight(&self.units[i], &self.units[target.0 as usize])
                    || crate::air::dogfight::breaks(
                        body_distance(
                            self.units[i].position,
                            self.units[target.0 as usize].position,
                        ),
                        &self.rules.air.fixed_wing,
                    );
            if broken {
                crate::air::dogfight::clear(&mut self.units[i]);
            }
        }
    }

    /// A finite-endurance aircraft turns for home while it still has enough
    /// fuel to reach it: a jet flies for the nearest point on its side's back
    /// edge, a helicopter for its friendly entry site. The ordinary
    /// withdrawal route then retires it there and settles its weighted
    /// refund, so a combat order cannot strand an airframe after its tank is
    /// effectively empty.
    fn trigger_fuel_withdrawals(&mut self) {
        let Some(sites) = self.skirmish.as_ref().map(|s| s.setup.sites.clone()) else {
            return;
        };
        for index in 0..self.units.len() {
            let unit = &self.units[index];
            if !unit.alive() || unit.retired || unit.withdrawing {
                continue;
            }
            let Some(air_state) = unit.air.as_ref() else {
                continue;
            };
            let Some(capability) = unit.unit_type(&self.rules).capabilities.air.as_ref() else {
                continue;
            };
            let entry = if unit.fixed_wing() {
                let edge = crate::air::edge::back_edge(unit.side, &sites, &self.world);
                crate::air::edge::entry(&edge, unit.position.xy()).0
            } else {
                let selected = crate::entries::select(
                    &sites,
                    unit.side,
                    unit.kind,
                    [unit.position.x, unit.position.y],
                    |entry, kind, point| {
                        self.entry_route_time(unit.side, kind, entry.center, point)
                    },
                );
                let entry = crate::entries::site(&sites, selected);
                v2(entry.center[0], entry.center[1])
            };
            let units::Motion::Air(flight) = unit.motion else {
                continue;
            };
            let speed = flight.cruise_mps
                * if air_state.afterburner {
                    capability.afterburner.map_or(1.0, |a| a.speed_factor)
                } else {
                    1.0
                };
            let return_s = (unit.position.xy() - entry).length() / speed.max(1.0) + 2.0;
            if air_state.fuel_s > return_s {
                continue;
            }
            let unit = &mut self.units[index];
            if let Some(air) = unit.air.as_mut() {
                air.afterburner = false;
            }
            crate::air::break_off(unit);
            unit.enqueue(
                UnitOrder::Move(MoveOrder {
                    destination: entry,
                    policy: RoutePolicy::Fastest,
                    gesture: 0,
                    direction: contract::command::MoveDirection::Forward,
                    facing: None,
                    short: None,
                }),
                false,
            );
            unit.withdrawing = true;
        }
    }

    fn retire_withdrawals(&mut self) {
        let Some(mut skirmish) = self.skirmish.take() else {
            return;
        };
        let mut retiring = Vec::new();
        for (index, unit) in self.units.iter().enumerate() {
            if !unit.alive() || !unit.withdrawing {
                continue;
            }
            if unit.fixed_wing() {
                let edge =
                    crate::air::edge::back_edge(unit.side, &skirmish.setup.sites, &self.world);
                let dt = 1.0 / f64::from(self.rules.tick_hz);
                let previous = unit.position.xy() - unit.flight_velocity().xy() * dt;
                if !crate::air::edge::crossed(&edge, previous, unit.position.xy()) {
                    continue;
                }
                let air = unit.air.as_ref().expect("an aircraft's flight state");
                self.air_transits[unit.side.index()].push(AirTransit {
                    kind: unit.kind,
                    point: [unit.position.x, unit.position.y, unit.position.z],
                    heading: unit.yaw,
                    speed_mps: air.velocity.xy().length(),
                    edge_tick: self.tick,
                    dir: AirTransitDirection::Out,
                });
            } else if unit.movement_goal().is_some() {
                continue;
            }
            // Rotor retirement follows completed ordinary movement at any side entry.
            let half = unit.hull.map_or(2.0, |half| half.x.max(half.y));
            if !unit.fixed_wing()
                && !crate::entries::retire_at(
                    &skirmish.setup.sites,
                    unit.side,
                    [unit.position.x, unit.position.y],
                    half,
                )
            {
                continue;
            }
            retiring.push(index);
        }
        let carriers: Vec<UnitId> = retiring
            .iter()
            .map(|&index| self.units[index].id)
            .filter(|&carrier| {
                self.units[carrier.0 as usize]
                    .unit_type(&self.rules)
                    .capabilities
                    .carriage
                    .is_some()
            })
            .collect();
        for carrier in carriers {
            retiring.extend(crate::carriage::passengers(&self.units, carrier));
        }
        retiring.sort_unstable();
        retiring.dedup();
        for index in retiring {
            let unit = &mut self.units[index];
            let Some(entered) = skirmish.entered.get(&unit.id) else {
                continue;
            };
            let (health, ammo) = crate::withdrawal::condition(unit, &self.rules, &self.arsenal);
            let fuel = crate::withdrawal::fuel(unit, &self.rules);
            let age = self.tick.saturating_sub(entered.entered_tick) as f64
                / f64::from(self.rules.tick_hz);
            let refund = unit
                .unit_type(&self.rules)
                .capabilities
                .air
                .as_ref()
                .map(|air| (air.refund.health, air.refund.ammunition, air.refund.fuel))
                .map_or_else(
                    || crate::withdrawal::fraction(health, ammo, age),
                    |weights| {
                        crate::withdrawal::weighted_fraction(health, ammo, fuel, weights, age)
                    },
                );
            let credits = (f64::from(entered.price) * refund * crate::skirmish::CREDIT_SCALE as f64)
                .round() as u64;
            let player = entered.player;
            skirmish.players[player].wallet += credits;
            skirmish.record_refund(
                player,
                unit.id,
                unit.kind,
                credits as f64 / crate::skirmish::CREDIT_SCALE as f64,
                self.tick,
            );
            unit.retired = true;
            unit.withdrawing = false;
            unit.orders.clear();
            unit.route = None;
            unit.hold = None;
        }
        self.skirmish = Some(skirmish);
    }

    pub fn new(setup: &ScenarioDefinition, seed: u64) -> Self {
        Self::from_prepared(
            setup,
            seed,
            crate::encounter::PreparedMap::new(&setup.map, &setup.rules),
        )
    }

    /// Consume the physical world and navigation already used to place this encounter.
    /// `prepared` must have been built from this scenario's map and rules.
    pub fn from_prepared(
        setup: &ScenarioDefinition,
        seed: u64,
        prepared: crate::encounter::PreparedMap,
    ) -> Self {
        setup
            .validate_player_contract()
            .expect("admitted scenario has a valid player contract");
        let rules = setup.rules.clone();
        debug_assert!(rules.check().is_ok(), "rule sections were admitted");
        debug_assert!(
            supply::validate(&rules).is_ok(),
            "supply rules were admitted"
        );
        let crate::encounter::PreparedMap {
            world,
            roads,
            base: grid,
            ..
        } = prepared;
        let arsenal = Arsenal::new(&rules);
        // An uploaded scenario was refused before this with these same errors.
        let unknown = setup.unknown_catalog_references();
        debug_assert!(
            unknown.is_empty(),
            "scenario names kinds its rules lack: {unknown:?}"
        );
        let mut soldier_ids = 0u32;
        let mut units = setup
            .units
            .iter()
            .enumerate()
            .map(|(i, u)| spawn_unit(i, u, &world, &rules, &arsenal, &mut soldier_ids, seed))
            .collect::<Vec<Unit>>();
        for (unit, setup) in units.iter_mut().zip(&setup.units) {
            if let Some(c) = &setup.condition {
                wear(unit, c, &arsenal, &rules);
            }
            unit.settle();
        }
        let mut events: Vec<ScenarioEvent> = setup.events.clone();
        events.sort_by_key(|e| e.tick);
        let mut scripts: Vec<ScriptedOrder> = setup.scripts.clone();
        scripts.sort_by_key(|o| o.tick);
        let authored_scripts = setup
            .authored_scripts
            .iter()
            .map(|script| {
                (
                    CompiledScript::compile(&script.source)
                        .expect("scenario authored script was admitted"),
                    script.side,
                    script.player.clone(),
                )
            })
            .collect();
        let victory_script = setup
            .victory
            .as_deref()
            .map(|source| CompiledScript::compile(source).expect("scenario victory was admitted"));
        let occlusion = OcclusionGrid::new(&world, setup.map.fog_cell_m);
        let structures = Structures::default();
        let ground = GroundLayer::new(world.width(), world.depth(), &rules.ground);
        let knowledge = std::array::from_fn(|i| {
            SideKnowledge::new(seed ^ OBSERVATION_STREAM ^ i as u64, &ground)
        });
        let mut battle = Battle {
            skirmish: setup
                .skirmish
                .clone()
                .map(|s| crate::skirmish::Skirmish::new(s, &setup.players)),
            authored_props: world.props().count() as PropId,
            shoved: Vec::new(),
            world,
            rules,
            seed,
            tick: 0,
            units,
            sides: std::array::from_fn(|_| SideGeometry::new(std::sync::Arc::clone(&grid))),
            roads,
            planner: RoutePlanner::default(),
            events: events.into(),
            scripts: scripts.into(),
            authored_scripts,
            victory_script,
            script_victory: None,
            script_error: None,
            script_terminal_tick: None,
            script_events: Vec::new(),
            script_trace: Vec::new(),
            replay_script_trace: BTreeMap::new(),
            controllers: BTreeMap::new(),
            player_ids: setup.players.iter().map(|p| p.id.clone()).collect(),
            controller_transitions: Vec::new(),
            replay_controller_transitions: BTreeMap::new(),
            knowledge,
            sightings: Default::default(),
            fired: BTreeSet::new(),
            audible: Default::default(),
            projectiles: Projectiles::new(arsenal.config.clone()),
            arsenal,
            structures,
            ground,
            rounds: BTreeMap::new(),
            suppressed: BTreeMap::new(),
            combat_rng: Rng::new(seed ^ COMBAT_STREAM),
            damage_rng: Rng::new(seed ^ DAMAGE_STREAM),
            ricochet_rng: Rng::new(seed ^ RICOCHET_STREAM),
            last_soldier: soldier_ids,

            flight_events: Vec::new(),
            segments: Vec::new(),
            blasts: Vec::new(),
            fog: std::array::from_fn(|_| occlusion.field()),
            occlusion,
            next_seq: [1; 3],
            order_bucket: [(
                u64::from(setup.rules.commands.orders_per_s) * u64::from(setup.rules.tick_hz),
                0,
            ); 3],
            player_next_seq: vec![1; setup.players.len()],
            player_order_bucket: vec![
                (
                    u64::from(setup.rules.commands.orders_per_s) * u64::from(setup.rules.tick_hz),
                    0
                );
                setup.players.len()
            ],
            opponent: setup.opponent.clone().map(|o| (o, Defender::default())),
            skirmish_ai: if setup.skirmish.is_some() {
                setup
                    .players
                    .iter()
                    .enumerate()
                    .map(|(index, player)| (player.id.clone(), policy_for_player(seed, index)))
                    .collect()
            } else {
                BTreeMap::new()
            },
            team_memory: std::array::from_fn(|_| crate::skirmish_ai::TeamMemory::default()),
            team_tuning: crate::skirmish_ai::team::TeamTuning::shipped(),
            referee: setup.encounter.clone().map(|e| (e, Referee::default())),
            encounter: None,
            pending: Vec::new(),
            accepted: Vec::new(),
            replaying: None,
            observations: Default::default(),
            refusals: Default::default(),
            shouts: Default::default(),
            shout_before: Vec::new(),
            shout_evidence: ShoutEvidence::default(),
            shout_fired: BTreeSet::new(),
            shout_kills: BTreeSet::new(),
            shout_under_fire: BTreeSet::new(),
            shouted_unseen: Default::default(),
            scenario_digest: scenario_digest(setup),
            config_digest: config_digest(setup),
            expiries: BTreeMap::new(),
            fallen: BTreeMap::new(),
            crashes: Vec::new(),
            air_transits: Default::default(),
            ai_receipts: Vec::new(),
        };

        // Controllers are installed at tick 0 for every player, including a player
        // that has not bought a unit yet. This is the authoritative controller set.
        if setup.skirmish.is_some() {
            for player in &setup.players {
                let transition = ControllerTransition {
                    tick: 0,
                    player: player.id.clone(),
                    controller: player.controller.clone(),
                };
                battle.apply_controller(&transition);
                battle.controller_transitions.push(transition);
            }
        }
        let authored: Vec<PropId> = battle.world.props().map(|p| p.id).collect();
        for id in authored {
            battle.schedule_expiry(id);
        }
        sight::snapshot(&mut battle.units, &battle.arsenal);
        shouts::remember(&mut battle.shout_before, &battle.units, &battle.rules);
        for side in Side::ALL {
            battle.sense(side, true);
            battle.sweep_fog(side);
        }
        battle.observe_all();
        battle
    }

    /// A battle that re-applies `replay`'s commands at their recorded ticks.
    /// Player input is refused for its whole life.
    pub fn from_replay(setup: &ScenarioDefinition, replay: &Replay) -> Result<Self, ReplayError> {
        Self::load_replay(setup, replay, || Self::new(setup, replay.seed))
    }

    /// Restore a replay while retaining the prepared map's physical allocations.
    pub fn from_prepared_replay(
        setup: &ScenarioDefinition,
        replay: &Replay,
        prepared: crate::encounter::PreparedMap,
    ) -> Result<Self, ReplayError> {
        Self::load_replay(setup, replay, || {
            Self::from_prepared(setup, replay.seed, prepared)
        })
    }

    fn load_replay(
        setup: &ScenarioDefinition,
        replay: &Replay,
        build: impl FnOnce() -> Self,
    ) -> Result<Self, ReplayError> {
        if replay.engine_build != ENGINE_BUILD_ID {
            return Err(ReplayError::BuildMismatch);
        }
        if format!("{:016x}", scenario_digest(setup)) != replay.scenario_digest {
            return Err(ReplayError::ScenarioMismatch);
        }
        if format!("{:016x}", config_digest(setup)) != replay.config_digest {
            return Err(ReplayError::ConfigMismatch);
        }
        let mut battle = build();
        for transition in &replay.controller_transitions {
            if transition.tick == 0 {
                battle.apply_controller(transition);
            } else {
                battle
                    .replay_controller_transitions
                    .entry(transition.tick)
                    .or_default()
                    .push(transition.clone());
            }
        }
        if replay.controller_transitions.is_empty() {
            for (player, controller) in &replay.controllers {
                battle.apply_controller(&ControllerTransition {
                    tick: 0,
                    player: player.clone(),
                    controller: controller.clone(),
                });
            }
        }
        battle.replay_script_trace = replay
            .script_trace
            .iter()
            .cloned()
            .map(|trace| (trace.tick, trace))
            .collect();
        battle.replaying = Some(replay.accepted.iter().cloned().collect());
        Ok(battle)
    }

    pub fn tick(&self) -> Tick {
        self.tick
    }

    pub fn rules(&self) -> &Rules {
        &self.rules
    }

    pub fn world(&self) -> &WorldGeometry {
        &self.world
    }

    /// This tick's flight events, in order; damage has already applied them.
    pub fn flight_events(&self) -> &[FlightEvent] {
        &self.flight_events
    }

    /// Rounds currently in flight, with where each came from.
    pub fn rounds(&self) -> impl Iterator<Item = (&flight::Projectile, &Round)> {
        self.projectiles
            .active()
            .iter()
            .filter_map(|p| self.rounds.get(&p.id).map(|r| (p, r)))
    }

    /// What the battle is carrying now, read from the owning stores (the
    /// endurance report's counters; no second bookkeeping).
    pub fn load(&self) -> Load {
        let living = self.units.iter().filter(|u| u.alive());
        Load {
            living_units: living.clone().count(),
            living_soldiers: living
                .map(|u| u.members.iter().filter(|s| s.alive()).count())
                .sum(),
            corpses: self
                .units
                .iter()
                .flat_map(|u| &u.members)
                .filter(|s| !s.alive())
                .count(),
            wrecks: self
                .world
                .props()
                .filter(|p| {
                    let catalog = &self.rules.catalog;
                    let kind = self.world.types().id(p.kind);
                    catalog
                        .indices()
                        .any(|t| catalog.get(t).hull().is_some_and(|h| h.wreck == kind))
                })
                .count(),
            active_projectiles: self.projectiles.active().len(),
            rounds_launched: self.projectiles.launched(),
            path_searches: Side::ALL.into_iter().map(|s| self.route_searches(s)).sum(),
            routes_pending: self.planner.waiting(),
            planning_work: self.planner.spent(),
            grid_cells_relaid: self.sides.iter().map(|s| s.cells_relaid()).sum(),
            ground_bytes: self.ground.bytes(),
            known_ground_bytes: self.knowledge.iter().map(|k| k.ground().bytes()).sum(),
        }
    }

    /// A unit as the authority holds it, for native tests and reports.
    pub fn unit(&self, id: UnitId) -> Option<&Unit> {
        self.units.get(id.0 as usize)
    }

    /// The referee's latest verdict (the one both sides' observations carry);
    /// `None` without an encounter or before the first step.
    pub fn encounter(&self) -> Option<EncounterStatus> {
        self.encounter
    }

    pub fn arsenal(&self) -> &Arsenal {
        &self.arsenal
    }

    /// Every prop's integrity and remains, for native tests and reports.
    pub fn structures(&self) -> &Structures {
        &self.structures
    }

    /// Craters and wear on the ground, for native tests and reports. Players
    /// only ever receive a side's [`Battle::known_ground`].
    pub fn ground(&self) -> &GroundLayer {
        &self.ground
    }

    /// The ground as `side` last saw it: what its publications deliver.
    pub fn known_ground(&self, side: Side) -> &KnownGround {
        self.knowledge[side.index()].ground()
    }

    /// `side`'s navigation revision: it rises only when the side learns an
    /// obstacle that changes its plans.
    pub fn navigation_revision(&self, side: Side) -> u64 {
        self.sides[side.index()].revision
    }

    /// Route searches run so far for `side`.
    pub fn route_searches(&self, side: Side) -> u64 {
        self.sides[side.index()].searches
    }

    /// Travel time known to one side for one of its observed units. The query
    /// deliberately uses that side's planning geometry, so external policy
    /// hosts cannot inspect hidden obstacles or route an enemy unit.
    pub fn route_time(&self, side: Side, id: UnitId, destination: [f64; 2]) -> Option<f64> {
        let frame = self.observe(side);
        let own = frame.own.iter().find(|unit| unit.id == id)?;
        let (kind, from) = (own.kind, [own.position[0], own.position[1]]);
        self.entry_route_time(side, kind, from, destination)
    }

    /// Travel time for a catalog unit from an entry site to a point, using the
    /// side's planning geometry. Entry selection uses the same route owner as
    /// ordinary movement admission.
    fn entry_route_time(
        &self,
        side: Side,
        kind: TypeIndex,
        from: [f64; 2],
        destination: [f64; 2],
    ) -> Option<f64> {
        // Movement admission refuses a goal off the map; so does the route
        // estimate, or a planner would pick a goal its order cannot reach.
        self.world.height_at(destination[0], destination[1])?;
        let t = self.rules.catalog.get(kind);
        let from = v2(from[0], from[1]);
        let goal = v2(destination[0], destination[1]);
        let mobility = match units::motion(t, &self.rules) {
            units::Motion::Ground(m) => m,
            units::Motion::Air(f) => return Some((goal - from).length() / f.cruise_mps),
        };
        let mut known = self.sides[side.index()].clone();
        let grid = known.grid(&self.world, self.authored_props);
        let (plan, _) = crate::navigation::plan(
            grid,
            &self.roads,
            crate::navigation::Leg {
                from,
                goal,
                m: &mobility,
                policy: RoutePolicy::Fastest,
                avoid: &[],
            },
            &self.rules.navigation,
        );
        let crate::navigation::Plan::Route(route) = plan else {
            return None;
        };
        let time = grid.route_time(from, &route, &mobility);
        time.is_finite().then_some(time)
    }

    fn select_entry(
        &self,
        side: Side,
        kind: TypeIndex,
        point: [f64; 2],
    ) -> crate::entries::EntryIndex {
        let sites = self
            .skirmish
            .as_ref()
            .expect("skirmish entry selection")
            .setup
            .sites
            .clone();
        crate::entries::select(&sites, side, kind, point, |entry, kind, destination| {
            self.entry_route_time(side, kind, entry.center, destination)
        })
    }

    /// Validate and schedule a client's command for the next step. Every
    /// command whose sequence number is next for its side is recorded for
    /// replay, including ones rejected for their content, so a replay
    /// reproduces the same acks.
    pub fn accept(&mut self, command: CommandEnvelope) -> CommandAck {
        self.receive(command, Issuer::Client)
    }

    /// Schedule an order the battle's own AI, scripts or opponents give.
    fn issue(&mut self, command: CommandEnvelope) -> CommandAck {
        self.receive(command, Issuer::Battle)
    }

    fn receive(&mut self, command: CommandEnvelope, issuer: Issuer) -> CommandAck {
        let applied_tick = self.tick + 1;
        let ack = |error, placement, building, refused| CommandAck {
            seq: command.seq,
            applied_tick,
            error,
            refused,
            placement,
            building,
        };
        if self.replaying.is_some() {
            return ack(Some(OrderError::ReplayInProgress), None, None, Vec::new());
        }
        // A player under AI takes orders only from its controller. The refusal
        // comes before the command is recorded: the replay log does not say
        // who issued a command, so a replay could not tell this one from the
        // AI's own.
        if issuer == Issuer::Client && self.ai_controlled(command.player.as_deref()) {
            return ack(Some(OrderError::AiControlled), None, None, Vec::new());
        }
        self.admit(command.clone(), applied_tick).map_or_else(
            |e| ack(Some(e), None, None, Vec::new()),
            |(placement, building, refused)| {
                let error = placement
                    .as_ref()
                    .filter(|p| p.destinations.iter().all(|d| !d.placed))
                    .filter(|_| building.as_ref().is_none_or(|b| b.entrant.is_none()))
                    .map(|_| OrderError::NoValidDestination);
                ack(error, placement, building, refused)
            },
        )
    }

    fn spend_order(&mut self, side: usize, player: Option<usize>, tick: Tick) -> bool {
        let per_s = u64::from(self.rules.commands.orders_per_s);
        let hz = u64::from(self.rules.tick_hz);
        let bucket = player.map_or_else(
            || &mut self.order_bucket[side],
            |i| &mut self.player_order_bucket[i],
        );
        let (left, at) = bucket;
        *left = (*left + tick.saturating_sub(*at) * per_s).min(per_s * hz);
        *at = (*at).max(tick);
        if *left < hz {
            return false;
        }
        *left -= hz;
        true
    }

    fn admit(
        &mut self,
        command: CommandEnvelope,
        applied_tick: Tick,
    ) -> Result<Admission, OrderError> {
        let side = command.side.index();
        let player = self
            .skirmish
            .as_ref()
            .and_then(|s| command.player.as_deref().and_then(|id| s.player_index(id)));
        let expected = player.map_or(self.next_seq[side], |i| self.player_next_seq[i]);
        if command.seq != expected {
            return Err(OrderError::OutOfSequence { expected });
        }
        if let Some(i) = player {
            self.player_next_seq[i] += 1;
        } else {
            self.next_seq[side] += 1;
        }
        self.accepted.push((applied_tick, command.clone()));
        if !self.spend_order(side, player, applied_tick) {
            return Err(OrderError::RateLimited);
        }
        let prepared = self.prepare(command, applied_tick.saturating_sub(1))?;
        let placement = prepared.placement.clone();
        let building = prepared.building.as_ref().map(|p| p.placement.clone());
        let refused = prepared.refused.clone();
        self.pending.push(prepared);
        Ok((placement, building, refused))
    }

    /// A side-scoped target reference as the sim's own target, if the side
    /// holds it right now and can aim at it (never an air contact, D22).
    fn resolve_target(&self, side: Side, target: &TargetRef) -> Option<Target> {
        let knowledge = &self.knowledge[side.index()];
        match *target {
            TargetRef::Identified { id } => knowledge.unit_for(id).map(Target::Unit),
            TargetRef::Contact { id } => {
                knowledge.ground_contact(id).map(|c| Target::Contact(c.id))
            }
            // The aim point sits on the public terrain, whatever height the client sent.
            TargetRef::Ground { point } => self
                .world
                .height_at(point[0], point[1])
                .map(|z| Target::Ground(v3(point[0], point[1], z))),
        }
    }

    /// Give one player's own policy a fixed purchase plan. The policy emits
    /// ordinary purchase commands through its normal per-player sequence.
    pub fn script_purchases(
        &mut self,
        player: impl Into<String>,
        purchases: Vec<TypeIndex>,
        repeat_from: Option<usize>,
    ) {
        let player = player.into();
        if let Some(ai) = self.skirmish_ai.get_mut(&player) {
            ai.set_purchase_plan_with_repeat(purchases, repeat_from);
        }
    }

    fn controller_turn(&mut self) {
        if self.replaying.is_some() || self.controllers.is_empty() || self.skirmish.is_some() {
            return;
        }
        enum Intent {
            Attack(TargetRef),
            Move([f64; 2]),
        }
        let mut orders: Vec<(Side, String, Vec<UnitId>, Intent)> = Vec::new();
        for (player, controller) in &self.controllers {
            if *controller != ControllerKind::ActiveAi {
                continue;
            }
            let Some(unit) = self
                .units
                .iter()
                .find(|unit| unit.owner.as_deref() == Some(player.as_str()) && unit.alive())
            else {
                continue;
            };
            if unit.side == Side::Neutral {
                continue;
            }
            let frame = self.observations[unit.side.index()].clone();
            let ids: Vec<_> = self
                .units
                .iter()
                .filter(|candidate| {
                    candidate.owner.as_deref() == Some(player.as_str()) && candidate.alive()
                })
                .map(|candidate| candidate.id)
                .collect();
            if ids.is_empty() {
                continue;
            }
            if let Some(target) = frame.identified.iter().max_by(|a, b| {
                let score = |candidate: &contract::observation::IdentifiedUnit| {
                    let dx = candidate.position[0] - unit.position.x;
                    let dy = candidate.position[1] - unit.position.y;
                    f64::from(candidate.cost) / (dx * dx + dy * dy + 1.0).sqrt()
                };
                score(a)
                    .partial_cmp(&score(b))
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| b.id.cmp(&a.id))
            }) {
                orders.push((
                    unit.side,
                    player.clone(),
                    ids,
                    Intent::Attack(TargetRef::Identified { id: target.id }),
                ));
            } else if let Some(objective) = frame.skirmish.as_ref().and_then(|view| {
                view.objectives
                    .iter()
                    .find(|objective| objective.owner.is_none())
            }) {
                orders.push((
                    unit.side,
                    player.clone(),
                    ids,
                    Intent::Move(objective.center),
                ));
            }
        }
        for (side, player, units, intent) in orders {
            let seq = self.next_seq[side.index()];
            let order = match intent {
                Intent::Attack(target) => Order::Attack { units, target },
                Intent::Move(goal) => Order::Move {
                    units,
                    gesture: 0,
                    goal,
                    route: contract::command::RoutePolicy::Shortest,
                    direction: contract::command::MoveDirection::Forward,
                    facing: None,
                },
            };
            let _ = self.issue(CommandEnvelope {
                side,
                player: Some(player),
                seq,
                order,
                queued: false,
            });
        }
    }

    pub fn finished(&self) -> bool {
        self.script_terminal_tick
            .is_some_and(|tick| self.tick >= tick)
            || self
                .skirmish
                .as_ref()
                .is_some_and(|s| s.phase == contract::skirmish::Phase::Finished)
    }

    fn validate(&self, command: &CommandEnvelope) -> Result<(), OrderError> {
        if self.finished() {
            return Err(OrderError::MatchFinished);
        }
        if command.side == Side::Neutral {
            return Err(OrderError::PassiveSide);
        }
        let skirmish_player = if let Some(skirmish) = &self.skirmish {
            let id = command.player.as_deref().ok_or(OrderError::MissingPlayer)?;
            let index = skirmish.player_index(id).ok_or(OrderError::MissingPlayer)?;
            if skirmish.players[index].side != command.side {
                return Err(OrderError::WrongFaction);
            }
            Some(index)
        } else {
            None
        };
        let units = match &command.order {
            Order::Refund { units } => {
                let Some(skirmish) = &self.skirmish else {
                    return Err(OrderError::NotSkirmish);
                };
                self.validate_command_units(command, units)?;
                if units.iter().any(|id| !skirmish.entered.contains_key(id)) {
                    return Err(OrderError::NotSkirmish);
                }
                units
            }
            Order::Ready | Order::ConfirmPurchase { .. } => {
                return skirmish_player.map(|_| ()).ok_or(OrderError::NotSkirmish);
            }
            Order::CancelPending { purchase } => {
                let index = skirmish_player.ok_or(OrderError::NotSkirmish)?;
                if self
                    .skirmish
                    .as_ref()
                    .unwrap()
                    .players
                    .iter()
                    .enumerate()
                    .any(|(other, p)| {
                        other != index && p.reservations.iter().any(|r| r.data.id == *purchase)
                    })
                {
                    return Err(OrderError::WrongFaction);
                }
                return Ok(());
            }
            Order::Move { units, goal, .. } | Order::AttackMove { units, goal, .. } => {
                if self.world.height_at(goal[0], goal[1]).is_none() {
                    return Err(OrderError::OutOfBounds);
                }
                units
            }
            Order::Attack { units, target } => {
                if let TargetRef::Contact { id } = *target {
                    let held = self.knowledge[command.side.index()].contact(id);
                    if held.is_some_and(|c| c.layer != AltitudeLayer::Ground) {
                        return Err(OrderError::AirContact);
                    }
                }
                if self.resolve_target(command.side, target).is_none() {
                    return Err(OrderError::UnknownTarget);
                }
                units
            }
            Order::Garrison { units, building } => {
                self.validate_command_units(command, units)?;
                // A squad ordered in earlier this tick holds no order yet.
                let claimed = self.pending.iter().any(|c| {
                    c.command.side == command.side
                        && (matches!(c.command.order, Order::Garrison { building: b, .. } if self.world.remembered_structure_owner(b) == self.world.remembered_structure_owner(*building))
                            || c.building.as_ref().is_some_and(|b| b.placement.entrant.is_some() && b.placement.building == self.world.remembered_structure_owner(*building)))
                });
                return garrison::validate(
                    &self.world,
                    &self.units,
                    command.side,
                    units,
                    self.sides[command.side.index()]
                        .prop(&self.world, self.authored_props, *building)
                        .filter(|p| self.world.building_of(p.id).is_some())
                        .map(|mut p| {
                            p.id = self.world.remembered_structure_owner(p.id);
                            p
                        }),
                    claimed,
                    &self.rules,
                );
            }
            Order::Board { units, carrier, .. } => {
                self.validate_command_units(command, units)?;
                if units
                    .iter()
                    .any(|id| self.units[id.0 as usize].hull.is_some())
                {
                    return Err(OrderError::NotInfantry { unit: units[0] });
                }
                let Some(c) = self.units.get(carrier.0 as usize) else {
                    return Err(OrderError::NotACarrier);
                };
                if c.side != command.side
                    || c.unit_type(&self.rules).capabilities.carriage.is_none()
                {
                    return Err(OrderError::NotACarrier);
                }
                if !c.alive() {
                    return Err(OrderError::Destroyed { unit: *carrier });
                }
                // A squad rides only its own player's carrier, as a carrier
                // collects only its own player's squads.
                self.validate_player(command, *carrier)?;
                units
            }
            Order::Collect { units, squads, .. } => {
                self.validate_command_units(command, units)?;
                for &squad in squads {
                    self.validate_command_units(command, &[squad])?;
                    if self.units[squad.0 as usize].hull.is_some() {
                        return Err(OrderError::NotInfantry { unit: squad });
                    }
                }
                if units.iter().any(|id| {
                    self.units[id.0 as usize]
                        .unit_type(&self.rules)
                        .capabilities
                        .carriage
                        .is_none()
                }) {
                    return Err(OrderError::NotACarrier);
                }
                units
            }
            Order::Unload { units, goal, .. } => {
                self.validate_command_units(command, units)?;
                if let Some(goal) = goal {
                    if self.world.height_at(goal[0], goal[1]).is_none() {
                        return Err(OrderError::OutOfBounds);
                    }
                }
                if units.iter().any(|id| {
                    self.units[id.0 as usize]
                        .unit_type(&self.rules)
                        .capabilities
                        .carriage
                        .is_none()
                }) {
                    return Err(OrderError::NotACarrier);
                }
                units
            }
            Order::Stop { units }
            | Order::OccupyBuilding { units, .. }
            | Order::SetAfterburner { units, .. }
            | Order::SetEngagement { units, .. }
            | Order::SetDeployment { units, .. }
            | Order::ExitBuilding { units } => units,
            // Gesture tokens are scoped by side at application; nothing else to check.
            Order::UpgradeMove { .. } => return Ok(()),
        };
        self.validate_command_units(command, units)
    }

    fn validate_command_units(
        &self,
        command: &CommandEnvelope,
        units: &[UnitId],
    ) -> Result<(), OrderError> {
        self.validate_units(command.side, units)?;
        for &unit in units {
            self.validate_player(command, unit)?;
        }
        Ok(())
    }

    /// A unit a player owns takes orders only from that player.
    fn validate_player(&self, command: &CommandEnvelope, unit: UnitId) -> Result<(), OrderError> {
        let Some(owner) = self.units[unit.0 as usize].owner.as_deref() else {
            return Ok(());
        };
        match command.player.as_deref() {
            None => Err(OrderError::MissingPlayer),
            Some(player) if player == owner => Ok(()),
            Some(_) => Err(OrderError::WrongPlayer { unit }),
        }
    }

    /// Every unit named exists, is the side's own and is alive.
    fn validate_units(&self, side: Side, units: &[UnitId]) -> Result<(), OrderError> {
        if units.is_empty() {
            return Err(OrderError::NoUnits);
        }
        for &unit in units {
            match self.units.get(unit.0 as usize) {
                None => return Err(OrderError::UnknownUnit { unit }),
                Some(u) if u.side != side => return Err(OrderError::NotOwnUnit { unit }),
                Some(u) if !u.alive() => return Err(OrderError::Destroyed { unit }),
                Some(u) if !u.on_field() => return Err(OrderError::Aboard { unit }),
                Some(_) => {}
            }
        }
        Ok(())
    }

    /// Advance one fixed tick: authored events, scheduled commands, movement, observation.
    pub fn step(&mut self) -> Tick {
        self.step_profiled(|_| {})
    }

    /// Advance the production tick, notifying the report after each work
    /// bracket. Repeated phases (movement and each side's fog) add together.
    pub fn step_profiled(&mut self, mut completed: impl FnMut(TickPhase)) -> Tick {
        if self.finished() {
            return self.tick;
        }
        if let Some(transitions) = self.replay_controller_transitions.remove(&self.tick) {
            for transition in transitions {
                self.apply_controller(&transition);
            }
        }
        self.skirmish_ai_turn();
        self.controller_turn();
        self.tick += 1;
        for transits in &mut self.air_transits {
            transits.clear();
        }
        shouts::remember(&mut self.shout_before, &self.units, &self.rules);
        for side in Side::ALL {
            for contact in self.knowledge[side.index()].expire_contacts(self.tick) {
                for unit in self.units.iter_mut().filter(|u| u.side == side) {
                    unit.orders.retain(|order| {
                        !matches!(order, UnitOrder::Attack { target, .. } if *target == Target::Contact(contact))
                    });
                    for mount in &mut unit.mounts {
                        if mount
                            .lock
                            .as_ref()
                            .is_some_and(|l| l.target == Target::Contact(contact))
                        {
                            mount.lock = None;
                        }
                    }
                }
            }
        }
        self.prune_attackers();
        if let Some(recorded) = self.replaying.as_mut() {
            let mut due = Vec::new();
            while recorded.front().is_some_and(|(t, _)| *t == self.tick) {
                due.push(recorded.pop_front().unwrap().1);
            }
            for command in due {
                let tick = self.tick;
                // Recorded content errors reproduce as the same no-op.
                let _ = self.admit(command, tick);
            }
        }
        self.expire_props();
        while self.events.front().is_some_and(|e| e.tick <= self.tick) {
            let event = self.events.pop_front().unwrap();
            match event.action {
                EventAction::AddProp(prop) => {
                    self.add_prop(&prop);
                }
                EventAction::Fire { unit } => self.record_fire(unit, 0),
                EventAction::Burst { point, weapon } => self.burst_event(point, &weapon),
            }
        }
        // Fixture scripts are authored setup, identical live and in replay.
        while self.scripts.front().is_some_and(|o| o.tick <= self.tick) {
            let mut script = self.scripts.pop_front().unwrap();
            // A fixed script addresses a formation's survivors. Keep invalid
            // ids for ordinary validation; only its own fallen actors leave.
            match &mut script.order {
                Order::Refund { units }
                | Order::Move { units, .. }
                | Order::Stop { units }
                | Order::Attack { units, .. }
                | Order::AttackMove { units, .. }
                | Order::SetAfterburner { units, .. }
                | Order::SetEngagement { units, .. }
                | Order::SetDeployment { units, .. }
                | Order::Garrison { units, .. }
                | Order::OccupyBuilding { units, .. }
                | Order::ExitBuilding { units }
                | Order::Board { units, .. }
                | Order::Collect { units, .. }
                | Order::Unload { units, .. } => units.retain(|id| {
                    self.units
                        .get(id.0 as usize)
                        .is_none_or(|u| u.side != script.side || u.alive())
                }),
                Order::UpgradeMove { .. }
                | Order::Ready
                | Order::ConfirmPurchase { .. }
                | Order::CancelPending { .. } => {}
            }
            let command = CommandEnvelope {
                side: script.side,
                player: None,
                seq: 0,
                order: script.order,
                queued: script.queued,
            };
            if let Ok(prepared) = self.prepare(command, self.tick.saturating_sub(1)) {
                self.pending.push(prepared);
            }
        }
        for command in std::mem::take(&mut self.pending) {
            self.apply(command);
        }
        if let Some(skirmish) = &mut self.skirmish {
            skirmish.advance(self.tick, self.rules.tick_hz);
        }
        self.break_dogfights();
        self.dispatch_reinforcements();
        completed(TickPhase::Orders);
        crate::carriage::advance(
            &self.world,
            &mut self.units,
            &self.rules,
            &mut self.refusals,
        );
        crate::hold::advance(
            &self.world,
            &self.sides,
            self.authored_props,
            &mut self.units,
            &self.rules,
            self.seed,
            &mut self.refusals,
        );
        crate::carriage::copy_passenger_positions(&mut self.units);
        self.update_pursuit();
        damage::recover(&mut self.units, &self.rules, self.tick);
        // Progress moves before movement, so the gate opens on the tick packing completes.
        deployment::advance_all(&mut self.units);
        let before = self.poses();
        let stood = self.standing();
        let treads = self.treads();
        // Every prop the world changed since last tick, for each side's
        // planning grid to look at again.
        let touched = self.world.take_touched();
        for side in &mut self.sides {
            side.touch(&touched);
        }
        let ctx = MovementContext {
            world: &self.world,
            roads: &self.roads,
            ground: &self.ground,
            authored: self.authored_props,
            tick: self.tick,
            seed: self.seed,
            rules: &self.rules,
            knowledge: std::array::from_fn(|i| &self.knowledge[i]),
            arsenal: &self.arsenal,
        };
        completed(TickPhase::Other);
        let shoves = movement::advance(
            &ctx,
            &mut self.units,
            &mut self.sides,
            &mut self.planner,
            &mut completed,
        );
        self.shove_props(shoves);
        self.clear_lanes(&before);
        let trod = self.treads();
        self.ground
            .tread(&self.world, treads, trod, &self.rules.ground);
        let after = self.poses();
        self.trigger_fuel_withdrawals();
        let moved = self.moved(&before, &after, &stood);
        completed(TickPhase::Movement);
        self.guide(&moved);
        self.fly(&before, &after);
        self.fall_crashes();
        // Where each unit looks, before this tick's fire turns any turret.
        completed(TickPhase::Flight);
        sight::snapshot(&mut self.units, &self.arsenal);
        for side in Side::ALL {
            self.sense(side, false);
            completed(TickPhase::Sight);
            if self.tick % FOG_INTERVAL_TICKS == fog_phase(side) {
                self.sweep_fog_profiled(side, &mut completed);
                completed(TickPhase::Learning);
            }
        }
        self.prune_attackers();
        let fired = self.fire(&moved);
        self.shout_fired = fired.clone();
        supply::service(
            &self.world,
            &mut self.units,
            &self.arsenal,
            &self.rules,
            &moved.units,
            &fired,
            &mut self.last_soldier,
        );
        self.retire_withdrawals();
        if let Some(skirmish) = &mut self.skirmish {
            if skirmish.phase == contract::skirmish::Phase::Active {
                skirmish.objectives.advance(
                    &skirmish.setup.sites.objectives,
                    &self.units,
                    &self.rules,
                );
                if skirmish.objectives.result.is_some() {
                    skirmish.phase = contract::skirmish::Phase::Finished;
                }
            }
        }
        completed(TickPhase::Weapons);
        let bucket = (self.rules.sensors.sound_bucket_s * self.rules.tick_hz as f64).round() as u64;
        for side in Side::ALL {
            self.audible[side.index()] = if self.tick.is_multiple_of(bucket) {
                hearing::hear(
                    side,
                    self.tick,
                    &self.units,
                    &self.knowledge[side.index()],
                    &self.fired,
                    &self.shouted_unseen[side.index()],
                    &self.rules,
                )
            } else {
                Vec::new()
            };
        }
        if self.tick.is_multiple_of(bucket) {
            self.fired.clear();
            for shouted in &mut self.shouted_unseen {
                shouted.clear();
            }
        }
        if let Some((rules, referee)) = self.referee.as_mut() {
            self.encounter = Some(referee.judge(
                rules,
                &self.rules.catalog,
                &self.units,
                self.tick,
                self.rules.tick_hz,
            ));
        }
        // A squad stands where its living soldiers stand, the fallen and the
        // joined included.
        for unit in &mut self.units {
            unit.settle();
        }
        self.ground.seal();
        completed(TickPhase::Other);
        self.observe_all();
        completed(TickPhase::Observation);
        self.run_authored_scripts();
        completed(TickPhase::Other);
        self.opponent_turn();
        completed(TickPhase::Other);
        self.tick
    }

    /// Execute authored code from the side-scoped observation produced this
    /// tick. Replay consumes the accepted command log and never reruns code.
    fn run_authored_scripts(&mut self) {
        if self.finished() {
            return;
        }
        if self.replaying.is_some() {
            if let Some(trace) = self.replay_script_trace.remove(&self.tick) {
                self.apply_script_trace(&trace);
            }
            return;
        }
        let mut outputs = Vec::new();
        for (index, (script, side, player)) in self.authored_scripts.iter().enumerate() {
            // Each script draws its own stream: the seed hashes the battle
            // seed, the tick, the side and the script's place in the scenario.
            let seed = [self.tick, side.index() as u64, index as u64]
                .into_iter()
                .fold(
                    contract::random::mix(self.seed ^ 0x5343_5249_5054_0000),
                    |h, v| contract::random::mix(h ^ v),
                );
            let input = ScriptInput::new(*side, &self.observations[side.index()], seed);
            let input = match player.as_deref() {
                Some(player) => input.with_player(player.to_owned()),
                None => input,
            };
            match script.run(&input) {
                Ok(output) => outputs.push(output),
                Err(error) => {
                    self.script_error = Some(error.to_string());
                    self.script_terminal_tick = Some(self.tick);
                    self.script_trace.push(ScriptTrace {
                        tick: self.tick,
                        events: Vec::new(),
                        victory: None,
                        error: self.script_error.clone(),
                    });
                    return;
                }
            }
        }
        let mut trace_victory = false;
        if let Some(script) = &self.victory_script {
            let input = ScriptInput::new(
                Side::Blue,
                &self.observations[Side::Blue.index()],
                self.seed ^ 0x5649_4354_4f52_5900 ^ self.tick,
            );
            match script.run(&input) {
                Ok(output) => {
                    if output.victory == Some(true) {
                        self.script_victory = Some(true);
                        self.script_terminal_tick = Some(self.tick);
                        trace_victory = true;
                    }
                }
                Err(error) => {
                    self.script_error = Some(error.to_string());
                    self.script_terminal_tick = Some(self.tick);
                    self.script_trace.push(ScriptTrace {
                        tick: self.tick,
                        events: Vec::new(),
                        victory: None,
                        error: self.script_error.clone(),
                    });
                    return;
                }
            }
        }
        let mut trace_events = Vec::new();
        for output in outputs {
            trace_events.extend(output.events.iter().cloned());
            self.script_events.extend(output.events);
            if output.victory == Some(true) {
                self.script_victory = Some(true);
                trace_victory = true;
            }
            for command in output.commands {
                let seq = self.next_seq[command.side.index()];
                let _ = self.issue(CommandEnvelope {
                    side: command.side,
                    player: command.player,
                    seq,
                    order: command.order,
                    queued: command.queued,
                });
            }
        }
        if !trace_events.is_empty() || trace_victory {
            self.script_trace.push(ScriptTrace {
                tick: self.tick,
                events: trace_events,
                victory: trace_victory.then_some(true),
                error: None,
            });
        }
    }

    fn apply_script_trace(&mut self, trace: &ScriptTrace) {
        if let Some(error) = &trace.error {
            self.script_error = Some(error.clone());
            self.script_terminal_tick = Some(trace.tick);
        }
        self.script_events.extend(trace.events.clone());
        if trace.victory == Some(true) {
            self.script_victory = Some(true);
            self.script_terminal_tick = Some(trace.tick);
        }
    }

    /// Add a prop after setup, scheduling its end if its row is transient.
    fn add_prop(&mut self, def: &PropDefinition) -> PropId {
        let id = self.world.add_prop(def);
        self.schedule_expiry(id);
        id
    }

    /// A transient body (its row has a `lifetime_s`) goes that long after
    /// it appears (Q28).
    fn schedule_expiry(&mut self, id: PropId) {
        let lifetime = self.world.prop(id).and_then(|p| p.body.lifetime_s);
        if let Some(s) = lifetime {
            let ticks = (s * self.rules.tick_hz as f64).round().max(1.0) as Tick;
            self.expiries.insert(id, self.tick + ticks);
        }
    }

    /// Transient bodies whose time is up go; every side sees them go, and
    /// both revisions bump.
    fn expire_props(&mut self) {
        let due: Vec<PropId> = self
            .expiries
            .iter()
            .filter(|(_, &t)| t <= self.tick)
            .map(|(&id, _)| id)
            .collect();
        for id in due {
            self.expiries.remove(&id);
            let Some(prop) = self.world.remove_prop(id) else {
                continue;
            };
            for side in &mut self.sides {
                side.forget(&prop);
            }
        }
    }

    /// Move the bodies vehicles shoved this tick (Q2). A side that has not
    /// seen an authored body move keeps it where it stood (L1); the soldiers
    /// a body now covers step out of it.
    fn shove_props(&mut self, shoves: Vec<movement::Shove>) {
        self.shoved.clear();
        for s in shoves {
            let Some(prop) = self.world.prop(s.prop).cloned() else {
                continue;
            };
            if prop.body.topples {
                self.knock_down(prop, s.by, s.heading);
                continue;
            }
            for side in &mut self.sides {
                side.before_move(&prop, self.authored_props);
            }
            self.world.move_prop(s.prop, s.center, s.yaw, self.tick);
            if let Some(moved) = self.world.prop(s.prop).map(|p| p.footprint()) {
                let r = self.rules.physics.soldier_radius_m;
                movement::clear_of(&self.world, &mut self.units, &moved, r);
                self.shoved.push(s);
            }
        }
    }

    /// A vehicle of side `by`, driving along `heading`, knocked a tree down
    /// (Q16): its body goes and it falls the way the vehicle drove. Its side
    /// replans without it and knows the fall at once (contact). Every other
    /// side keeps it standing until it sees the ground where it stood (L1).
    fn knock_down(&mut self, prop: crate::world::Prop, by: Side, heading: V2) {
        self.world.knock_down(prop.id);
        self.keep_standing(&prop, Some(by));
        self.fell(&prop, heading);
        self.knowledge[by.index()].learn_fallen(prop.id);
    }

    /// Record that toppling `prop` went down toward `toward` this tick. A
    /// degenerate direction (a burst at the foot) falls back to one from the
    /// body's id, so the record is always a unit vector and deterministic.
    fn fell(&mut self, prop: &crate::world::Prop, toward: V2) {
        let len = toward.length();
        let toward = if len > 1e-9 {
            toward * (1.0 / len)
        } else {
            // No trigonometry: it must match bit for bit in Wasm.
            const D: f64 = std::f64::consts::FRAC_1_SQRT_2;
            const COMPASS: [(f64, f64); 8] = [
                (1.0, 0.0),
                (D, D),
                (0.0, 1.0),
                (-D, D),
                (-1.0, 0.0),
                (-D, -D),
                (0.0, -1.0),
                (D, -D),
            ];
            let (x, y) = COMPASS[prop.id as usize % 8];
            v2(x, y)
        };
        self.fallen.insert(
            prop.id,
            FallenBody {
                prop: prop.id,
                at: [prop.center.x, prop.center.y],
                toward: [toward.x, toward.y],
                tick: self.tick,
            },
        );
    }

    /// `prop` is gone: side `by` (the one that touched it) replans without
    /// it at once; every other side that planned with it keeps it standing
    /// until it sees where it stood (L1).
    fn keep_standing(&mut self, prop: &crate::world::Prop, by: Option<Side>) {
        for side in Side::ALL {
            let known = &mut self.sides[side.index()];
            if Some(side) == by {
                known.forget(prop);
            } else if prop.id < self.authored_props || known.seen.contains_key(&prop.id) {
                known.keep_standing(prop.clone());
            }
        }
    }

    /// Destroy a prop whose integrity ran out (Q17) into its row's destroyed
    /// state: a tree falls and its spot becomes open ground, like a lane a
    /// tank knocks through; a crate is removed; sandbags, a wall, a building
    /// or a wreck leave their remains on the same plan (a building's
    /// occupants escape its collapse, L10). No side learns of it by contact:
    /// each sees it gone once it sees where it stood. A tree falls `toward`
    /// the way the felling blow pushed. Returns units destroyed.
    fn destroy_prop(&mut self, id: PropId, toward: V2) -> Vec<UnitId> {
        let Some(owner) = self.world.structure_owner(id) else {
            return Vec::new();
        };
        let parts = self.world.structure_parts(owner);
        let replacements: Vec<_> = parts
            .iter()
            .copied()
            .filter_map(|part| self.destroy_part(part, toward).map(|new| (part, new)))
            .collect();
        self.world.replace_building_parts(owner, &replacements);
        // All shells/remains are final before any occupant attempts escape.
        garrison::collapse(
            &self.world,
            &mut self.units,
            owner,
            &self.rules,
            self.seed,
            self.tick,
            &parts,
        )
    }

    fn destroy_part(&mut self, id: PropId, toward: V2) -> Option<PropId> {
        let prop = self.world.prop(id).cloned()?;
        let state = self.world.prop_type(prop.kind).destroyed.clone()?;
        self.keep_standing(&prop, None);
        // Whatever it leaves, a toppling body falls the way it was pushed.
        if prop.body.topples {
            self.fell(&prop, toward);
        }
        match state {
            Destroyed::Cleared => {
                self.world.knock_down(id);
                // The tree's own share of the forest, out to its spacing.
                let reach = if prop.forest_tree {
                    self.world.forest_rule().trunk_spacing_m
                } else {
                    self.world.cleared_cell_m()
                };
                for cell in self.world.clear_spot(prop.center, reach) {
                    self.ground.clear(cell);
                }
                None
            }
            Destroyed::Removed => {
                self.world.remove_prop(id);
                self.structures.note_removed(prop);
                None
            }
            Destroyed::Into {
                prop: remains,
                mut height_m,
                building,
            } => {
                let mut remains = remains;
                if let Some(rule) = building {
                    if let Some(definition) = self.world.building(id) {
                        let geometry = &definition.geometry;
                        if geometry.floor_z.len() > rule.collapse_max_floors {
                            remains = rule.gutted_prop;
                            height_m = 2.0 * prop.half.z;
                        } else {
                            height_m = (geometry.height_m * rule.height_fraction)
                                .clamp(height_m, rule.max_height_m);
                        }
                    }
                }
                self.world.remove_prop(id);
                let remains = self.world.add_replacement(
                    &PropDefinition {
                        kind: remains,
                        center: [prop.center.x, prop.center.y],
                        yaw: prop.yaw,
                        half_extents: [prop.half.x, prop.half.y, height_m / 2.0],
                        base_z: Some(prop.base_z),
                        // A lighter wreck is still the same unit's.
                        wreck_of: prop.wreck_of.map(|t| self.rules.catalog.id(t).to_string()),
                    },
                    id,
                );
                self.schedule_expiry(remains);
                self.structures.note_replaced(remains, id);
                // Remains that still close an authored body's footprint to
                // every mover it stopped are planned with by every side, as
                // the body was: a fall a side never saw cannot open a route.
                let closes =
                    |c| !prop.blocks(c) || self.world.prop(remains).is_some_and(|r| r.blocks(c));
                if id < self.authored_props && MoverClass::ALL.into_iter().all(closes) {
                    self.world.set_known_to_all(remains);
                }
                Some(remains)
            }
        }
    }

    /// Every vehicle that knocks trees down and moved this tick clears the
    /// forest ground its hull (and `ground.lane_margin_m` to either side) has left
    /// behind: the lane stops being forest (Q16), with crushed-ground marks.
    /// The ground under the hull itself stays forest until it has passed, so
    /// carving a lane goes at forest speed and only the lane is open ground.
    fn clear_lanes(&mut self, before: &Poses) {
        // A forest is its trees: what knocks them down is its tree's weight.
        let tree = self.world.types().by_id(&self.rules.forests.tree).body;
        for (u, was) in self.units.iter().zip(&before.units) {
            let (Some(hull), Some(h)) = (u.ground_footprint(), u.hull) else {
                continue;
            };
            let knocks = tree.topples && u.ground().push.pushes(tree.weight_class);
            if !u.alive() || !knocks || (was.base - u.position).length() <= 1e-9 {
                continue;
            }
            if !self.world.forest_near(
                hull.center,
                hull.half.length() + self.rules.ground.lane_margin_m,
            ) {
                continue;
            }
            let left = Obb2 {
                center: was.base.xy(),
                yaw: was.yaw,
                // Wider, not longer: a margin ahead would clear the ground
                // the hull is about to cover.
                half: h.xy() + v2(0.0, self.rules.ground.lane_margin_m),
            };
            for cell in self.world.clear(&left, &hull) {
                self.ground.clear(cell);
            }
        }
    }

    /// The lab emitter: a round of `weapon` bursts on the ground at `point`,
    /// marking it and wearing down the props its blast reaches.
    fn burst_event(&mut self, point: [f64; 2], weapon: &str) {
        let w = self
            .arsenal
            .weapons
            .iter()
            .find(|w| w.id == weapon)
            .expect("burst weapons are checked at setup");
        let Some(z) = self.world.height_at(point[0], point[1]) else {
            return;
        };
        let at = v3(point[0], point[1], z);
        // Its blast wears the props it reaches down, as a round's would (Q17).
        let mut structural = Vec::new();
        damage::blast_props(&self.world, &w.def, at, None, None, &mut structural);
        self.ground
            .burst(&self.world, at, w.def.blast_radius_m, &self.rules.ground);
        for hit in structural {
            if self.structures.damage(&self.world, hit.prop, hit.amount) {
                self.destroy_prop(hit.prop, hit.toward);
            }
        }
    }

    /// Where every walking soldier and every vehicle track touches the ground,
    /// in unit order: the same list before and after movement.
    fn treads(&self) -> Vec<(V2, Wear)> {
        let mut out = Vec::new();
        for unit in self
            .units
            .iter()
            .filter(|u| u.on_field() && !u.garrisoned() && !u.airborne())
        {
            match unit.hull {
                Some(half) => {
                    let side = v2(0.0, half.y * self.rules.ground.track_gauge).rotated(unit.yaw);
                    let c = unit.position.xy();
                    out.push((c + side, Wear::Tracks));
                    out.push((c - side, Wear::Tracks));
                }
                None => out.extend(unit.member_positions().map(|p| (p.xy(), Wear::Trampled))),
            }
        }
        out
    }

    /// The opponent reads its side's fresh observation and commands through
    /// the same path as a player; its commands are recorded, so a replay
    /// replays them and never reruns it. Its memory is outside the digest:
    /// continuing live play from a replay or a snapshot would need it.
    fn opponent_turn(&mut self) {
        if self.replaying.is_some() {
            return;
        }
        let mut commands = Vec::new();
        if let Some((op, defender)) = self.opponent.as_mut() {
            let side = op.side;
            commands.extend(
                defender
                    .decide(op, &self.observations[side.index()], &self.rules)
                    .into_iter()
                    .map(|order| (side, order)),
            );
        }
        for (side, order) in commands {
            let seq = self.next_seq[side.index()];
            let _ = self.issue(CommandEnvelope {
                side,
                player: None,
                seq,
                order,
                queued: false,
            });
        }
    }

    pub fn ai_receipts(&self) -> &[AiReceipt] {
        &self.ai_receipts
    }

    /// Doctrine pivots accumulated by the live policy instances, grouped by
    /// side for native reports.
    pub fn ai_pivots_by_side(&self) -> [u32; 2] {
        let Some(skirmish) = &self.skirmish else {
            return [0, 0];
        };
        let mut pivots = [0u32, 0u32];
        for player in &skirmish.players {
            if let Some(ai) = self.skirmish_ai.get(&player.id) {
                pivots[player.side.index()] =
                    pivots[player.side.index()].saturating_add(ai.pivot_count());
            }
        }
        pivots
    }

    /// Latest non-digested team assignments, for tournament evidence.
    pub fn team_plans(&self) -> [Option<&crate::skirmish_ai::TeamPlan>; 2] {
        [
            self.team_memory[Side::Blue.index()].cached_plan(),
            self.team_memory[Side::Red.index()].cached_plan(),
        ]
    }

    pub fn team_tuning(&self) -> crate::skirmish_ai::team::TeamTuning {
        self.team_tuning
    }

    /// `unit` fired: every opposing side learns an uncertain firing area and
    /// may hear the shot, sounding like the weapon rows in `heard` (the lab
    /// emitter's shot is of no weapon: 0). Weapons and the lab emitter share
    /// this seam.
    fn record_fire(&mut self, unit: UnitId, heard: u64) {
        let Some(shooter) = self.units.get(unit.0 as usize) else {
            return;
        };
        let (side, at, layer) = (shooter.side, shooter.position, shooter.layer());
        for other in Side::ALL.into_iter().filter(|s| *s != side) {
            self.knowledge[other.index()].note_fire(unit, at, layer, heard);
        }
        self.fired.insert(unit);
    }

    /// Every unit's pose (ground contact centre and heading), and every
    /// soldier's position in unit then member order.
    fn poses(&self) -> Poses {
        Poses {
            units: self
                .units
                .iter()
                .map(|u| Pose {
                    base: u.position,
                    yaw: u.yaw,
                })
                .collect(),
            soldiers: self
                .units
                .iter()
                .flat_map(|u| u.members.iter().map(|s| s.exposed(self.tick)))
                .collect(),
        }
    }

    /// Colliders for this tick: a capsule per living soldier, swept from
    /// where he stood to where he stands, and a box per vehicle.
    fn bodies(&self, before: &Poses, after: &Poses) -> Vec<Body> {
        let b = &self.rules.physics;
        let mut bodies = Vec::new();
        let mut first = 0;
        for (i, unit) in self.units.iter().enumerate() {
            let soldiers = first..first + unit.members.len();
            first = soldiers.end;
            if !unit.on_field() {
                continue;
            }
            match unit.hull {
                Some(half) => bodies.push(Body {
                    id: BodyId(VEHICLE_BODY_BASE + unit.id.0),
                    unit: unit.id,
                    shape: Shape::Box { half },
                    from: before.units[i],
                    to: after.units[i],
                }),
                None => {
                    for (s, k) in unit.members.iter().zip(soldiers).filter(|(s, _)| s.alive()) {
                        let at = |p: &Poses, yaw: f64| Pose {
                            base: p.soldiers[k],
                            yaw,
                        };
                        bodies.push(Body {
                            id: BodyId(s.id),
                            unit: unit.id,
                            shape: Shape::Capsule {
                                radius: b.soldier_radius_m,
                                height: b.soldier_height_m,
                            },
                            from: at(before, before.units[i].yaw),
                            to: at(after, after.units[i].yaw),
                        });
                    }
                }
            }
        }
        bodies
    }

    /// Downed aircraft still falling.
    pub fn crashes(&self) -> &[crate::crash::Crash] {
        &self.crashes
    }

    /// Downed aircraft fall; each that meets the ground bursts there and
    /// leaves its wreck.
    fn fall_crashes(&mut self) {
        if self.crashes.is_empty() {
            return;
        }
        let dt = 1.0 / self.rules.tick_hz as f64;
        let gravity = self.projectiles.gravity();
        let mut landed = Vec::new();
        let mut left_map = Vec::new();
        for mut crash in std::mem::take(&mut self.crashes) {
            let half = self.units[crash.unit.0 as usize]
                .hull
                .expect("an airframe's hull");
            let fall = crash.fall(&self.world, gravity, half, dt);
            // Passengers stay aboard while it falls, so they follow the
            // crash rather than the airframe's last live pose.
            let passengers = crate::carriage::passengers(&self.units, crash.unit);
            for &index in &passengers {
                let unit = &mut self.units[index];
                for member in unit.members.iter_mut().filter(|member| member.alive()) {
                    member.position = crash.position;
                }
                unit.position = crash.position;
            }
            match fall {
                crate::crash::Fall::Landed => landed.push(crash),
                crate::crash::Fall::Falling => self.crashes.push(crash),
                // It takes them with it, credited to whoever brought it down.
                crate::crash::Fall::LeftMap => {
                    for index in passengers {
                        let unit = &mut self.units[index];
                        crate::bail_out::lost_with_host(unit, crash.position);
                        unit.hold = None;
                        unit.orders.clear();
                        unit.route = None;
                        left_map.push(damage::UnitDeath {
                            victim: unit.id,
                            source: crash.source,
                        });
                    }
                }
            }
        }
        for crash in landed {
            self.land(crash);
        }
        if !left_map.is_empty() {
            self.consequences(damage::Outcome {
                destroyed: left_map,
                ..Default::default()
            });
        }
    }

    /// A downed aircraft meets the ground (D3, D31): it bursts as the crash
    /// row, credited to whoever brought it down, then its wreck comes to rest
    /// on the nearest ground clear of live hulls and buildings.
    fn land(&mut self, crash: crate::crash::Crash) {
        let weapon = (self.arsenal.weapons.iter())
            .position(|w| w.id == crate::crash::CRASH_WEAPON)
            .expect("the rules' crash row");
        let def = self.arsenal.weapons[weapon].def.clone();
        let at = crate::crash::burst_point(crash.position);
        let ctx = DamageContext {
            world: &self.world,
            ground: &self.ground,
            arsenal: &self.arsenal,
            rules: &self.rules,
            tick: self.tick,
        };
        let outcome = damage::detonate(
            &ctx,
            &def,
            at,
            crash.source,
            &mut self.units,
            &mut self.damage_rng,
        );
        let side = self.units[crash.unit.0 as usize].side;
        self.blasts.push((
            side,
            Blast {
                point: xyz(at),
                radius: def.blast_radius_m,
                kind: weapon,
            },
        ));
        let fallen_before: BTreeSet<PropId> = self.fallen.keys().copied().collect();
        self.consequences(outcome);
        self.ground
            .burst(&self.world, at, def.blast_radius_m, &self.rules.ground);
        // The sides that saw it go down see it come down: the trees its
        // crash felled fall in their sight, as its wreck lands there.
        let felled: Vec<PropId> = (self.fallen.keys())
            .filter(|id| !fallen_before.contains(id))
            .copied()
            .collect();
        for &(side, _) in &crash.knowing {
            for &prop in &felled {
                self.sides[side.index()].saw_fallen(prop);
                self.knowledge[side.index()].learn_fallen(prop);
            }
        }
        let unit = &self.units[crash.unit.0 as usize];
        let half = unit.hull.expect("an airframe's hull");
        let kind = (unit.unit_type(&self.rules).hull())
            .expect("an airframe's hull row")
            .wreck
            .clone();
        let wreck_of = self.rules.catalog.id(unit.kind).to_string();
        let hulls: Vec<Obb2> = (self.units.iter())
            .filter(|o| o.on_field())
            .filter_map(|o| o.ground_footprint())
            .collect();
        let rest = crate::crash::resting_place(
            &self.world,
            crash.position.xy(),
            crash.yaw,
            half.xy(),
            &hulls,
        );
        let wreck = self.add_prop(&PropDefinition {
            kind,
            center: [rest.x, rest.y],
            yaw: crash.yaw,
            half_extents: [half.x, half.y, half.z],
            base_z: None,
            wreck_of: Some(wreck_of),
        });
        let mut passenger_deaths = Vec::new();
        self.bail_out_carrier(
            crash.unit,
            rest,
            wreck,
            crash.source,
            &mut passenger_deaths,
            contract::scenario::BailOutHost::Air,
        );
        if !passenger_deaths.is_empty() {
            self.consequences(damage::Outcome {
                destroyed: passenger_deaths,
                ..Default::default()
            });
        }
        if let Some(prop) = self.world.prop(wreck) {
            for (side, _) in crash.knowing {
                self.sides[side.index()].learn(
                    prop,
                    self.authored_props,
                    self.rules.pushing.relearn_m,
                    true,
                );
            }
        }
    }

    /// Hostile damage or suppression grants return fire; a destroyed vehicle
    /// leaves a permanent wreck (M06, M07) and carries its passenger deaths
    /// into the same causal stream. A death a side was watching ends its
    /// track, while an unseen death discloses nothing. The vehicle's own side
    /// knows where it died, and a side that watched it die saw the wreck
    /// appear: both learn the wreck at once, so a vehicle never vanishes from
    /// the picture of a side that knew where it stood.
    fn consequences(&mut self, outcome: damage::Outcome) {
        for (victim, shooter) in outcome.attacked {
            self.units[victim.0 as usize].attackers.insert(shooter);
        }
        let mut destroyed = outcome.destroyed;
        // Structural damage in event order; a prop is destroyed once (L10, Q17).
        for hit in outcome.structural {
            if self.structures.damage(&self.world, hit.prop, hit.amount) {
                destroyed.extend(self.destroy_prop(hit.prop, hit.toward).into_iter().map(
                    |victim| damage::UnitDeath {
                        victim,
                        source: hit.source,
                    },
                ));
            }
        }
        let mut death_index = 0;
        while death_index < destroyed.len() {
            let death = destroyed[death_index];
            death_index += 1;
            let id = death.victim;
            let unit = &self.units[id.0 as usize];
            let own = unit.side;
            // Each side that saw it die, with the id it knew it by.
            let mut knowing = vec![(own, id.0)];
            for side in Side::ALL.into_iter().filter(|&s| s != own) {
                let knowledge = &mut self.knowledge[side.index()];
                if knowledge.identifies(id, self.tick - 1) {
                    let handle = knowledge.track(id).expect("an identified track").id;
                    knowledge.saw_destroyed(id);
                    knowing.push((side, handle.0));
                }
            }
            // A downed aircraft falls first; its wreck lies where it lands.
            if let Some(air) = unit.air {
                self.crashes.push(crate::crash::Crash::new(
                    id,
                    unit.position,
                    air.velocity.xy(),
                    unit.yaw,
                    death.source,
                    knowing,
                ));
                continue;
            }
            let wreck = unit.unit_type(&self.rules).hull().map(|h| h.wreck.clone());
            let (hulls, soldiers) = crate::arrangement::ground_taken(&self.units, id);
            let rest = crate::movement::drive::death_roll(
                &self.world,
                &self.rules,
                unit,
                &hulls,
                &soldiers,
            );
            let unit_hull = unit.hull;
            let ground_carrier = unit_hull.is_some() && unit.air.is_none();
            let unit_yaw = unit.yaw;
            let wreck_of = self.rules.catalog.id(unit.kind).to_string();
            let unit_z = unit.position.z;
            let wreck = match (unit_hull, wreck) {
                (Some(half), Some(kind)) => Some(self.add_prop(&PropDefinition {
                    kind,
                    center: [rest.x, rest.y],
                    yaw: unit_yaw,
                    half_extents: [half.x, half.y, half.z],
                    base_z: Some(unit_z),
                    wreck_of: Some(wreck_of),
                })),
                _ => None,
            };
            if ground_carrier {
                self.bail_out_carrier(
                    id,
                    rest,
                    wreck.expect("a ground hull leaves a wreck"),
                    death.source,
                    &mut destroyed,
                    contract::scenario::BailOutHost::Ground,
                );
            }
            if let Some(prop) = wreck.and_then(|w| self.world.prop(w)) {
                for (side, _) in knowing {
                    self.sides[side.index()].learn(
                        prop,
                        self.authored_props,
                        self.rules.pushing.relearn_m,
                        true,
                    );
                }
            }
        }
        for death in &destroyed {
            // A kill bark is for an enemy; a friendly loss is not a kill.
            if let Some(source) = death.source.filter(|s| {
                self.units[s.unit.0 as usize].side != self.units[death.victim.0 as usize].side
            }) {
                self.shout_kills.insert(source.unit);
            }
        }
        if let Some(skirmish) = &mut self.skirmish {
            for death in &destroyed {
                let victim = &self.units[death.victim.0 as usize];
                let killer = death.source.and_then(|source| {
                    let unit = self.units.get(source.unit.0 as usize)?;
                    Some(contract::skirmish::MatchUnit {
                        side: source.side,
                        id: source.unit,
                        kind: unit.kind,
                        owner: {
                            let owner = unit.owner.as_deref();
                            debug_assert!(owner.is_some(), "skirmish killer unit has no owner");
                            owner
                                .and_then(|id| skirmish.player_index(id))
                                .map(|i| contract::players::PlayerIndex(i as u16))
                        },
                    })
                });
                let victim_cost = skirmish.entered.get(&death.victim).map_or_else(
                    || self.rules.catalog.get(victim.kind).cost,
                    |unit| unit.price,
                );
                let killer_cost = death.source.and_then(|source| {
                    skirmish
                        .entered
                        .get(&source.unit)
                        .map(|unit| unit.price)
                        .or_else(|| {
                            self.units
                                .get(source.unit.0 as usize)
                                .map(|unit| self.rules.catalog.get(unit.kind).cost)
                        })
                });
                skirmish.record_kill(
                    contract::skirmish::MatchUnit {
                        side: victim.side,
                        id: victim.id,
                        kind: victim.kind,
                        owner: victim
                            .owner
                            .as_deref()
                            .and_then(|id| skirmish.player_index(id))
                            .map(|i| contract::players::PlayerIndex(i as u16)),
                    },
                    victim_cost,
                    killer,
                    killer_cost,
                    self.tick,
                    death
                        .source
                        .and_then(|source| self.units.get(source.unit.0 as usize))
                        .and_then(|unit| {
                            let owner = unit.owner.as_deref();
                            debug_assert!(owner.is_some(), "skirmish killer unit has no owner");
                            owner.and_then(|id| skirmish.player_index(id))
                        }),
                );
            }
            let deaths: Vec<_> = destroyed
                .iter()
                .filter_map(|death| {
                    let entered = skirmish.entered.get(&death.victim)?;
                    let unit = &self.units[death.victim.0 as usize];
                    Some(crate::settlement::EconomicDeath {
                        victim: death.victim,
                        side: unit.side,
                        price: entered.price,
                        source: death.source,
                        supply: unit.unit_type(&self.rules).capabilities.supply.is_some(),
                    })
                })
                .collect();
            let starting_budget = u64::from(skirmish.setup.rules.credits_per_minute)
                * u64::from(skirmish.setup.rules.starting_minutes);
            let killer_players = self
                .units
                .iter()
                .filter_map(|unit| {
                    let owner = unit.owner.as_deref();
                    debug_assert!(owner.is_some(), "skirmish unit has no owner");
                    owner
                        .and_then(|id| skirmish.player_index(id))
                        .map(|player| (unit.id, player))
                })
                .collect();
            skirmish.settlement.settle_players(
                &deaths,
                starting_budget,
                &killer_players,
                &mut skirmish.players,
            );
        }
    }

    /// A carrier's wreck is the escape origin, under the bail-out row for its
    /// kind (`host_kind`: ground or air). Passengers are kept out
    /// of combat by `on_field` until this runs, so the killing burst cannot
    /// spend damage rolls on them. The carrier hull is omitted from the
    /// occupancy (its wreck is ignored) while every other body remains solid.
    fn bail_out_carrier(
        &mut self,
        carrier: UnitId,
        rest: V2,
        wreck: PropId,
        source: Option<damage::LethalSource>,
        destroyed: &mut Vec<damage::UnitDeath>,
        host_kind: contract::scenario::BailOutHost,
    ) {
        let passengers = crate::carriage::passengers(&self.units, carrier);
        if passengers.is_empty() {
            return;
        }
        let (hulls, mut soldiers) = crate::arrangement::ground_taken(&self.units, carrier);
        let ignored_props = [wreck];
        let rule = crate::bail_out::row(&self.rules, host_kind);
        for index in passengers {
            for member in self.units[index]
                .members
                .iter_mut()
                .filter(|member| member.alive())
            {
                member.position = v3(rest.x, rest.y, member.position.z);
            }
            self.units[index].position = v3(rest.x, rest.y, self.units[index].position.z);
            let host = self.units[index]
                .hold
                .as_ref()
                .expect("carrier passenger hold")
                .host;
            let out = crate::bail_out::apply(
                &self.world,
                &mut self.units[index],
                host,
                rule,
                self.seed,
                self.tick,
                crate::bail_out::Occupancy {
                    radius: self.rules.physics.soldier_radius_m,
                    hulls: &hulls,
                    soldiers: &soldiers,
                    ignored_props: &ignored_props,
                },
            );
            let unit = &mut self.units[index];
            unit.hold = None;
            if matches!(unit.orders.front(), Some(UnitOrder::Board { .. })) {
                unit.orders.pop_front();
            }
            if out == 0 {
                unit.orders.clear();
                unit.route = None;
                destroyed.push(damage::UnitDeath {
                    victim: unit.id,
                    source,
                });
            } else {
                unit.settle();
                unit.suppression = unit.suppression.max(self.rules.suppression.pinned.level);
                unit.suppressed_at = self.tick;
                soldiers.extend(unit.member_positions().map(|position| position.xy()));
            }
        }
    }

    /// An attack pursues an identified target it cannot fire on from here, or
    /// its last reported place once identification lapses (W17); it ends on
    /// arriving there unseen, or when its area expires unidentified.
    fn update_pursuit(&mut self) {
        for unit in &mut self.units {
            unit.pursuit = None;
            // A garrisoned attack fires from the building; it never walks out.
            if unit.garrisoned() {
                continue;
            }
            // Fixed-wing attacks are planned as passes by `air::step_fixed_wing`;
            // they never enter the generic ground-style hover pursuit.
            if unit.fixed_wing() {
                continue;
            }
            let knowledge = &self.knowledge[unit.side.index()];
            let Some(UnitOrder::Attack { target, last_known }) = unit.orders.front_mut() else {
                continue;
            };
            let out_of_reach = unit.reach.needs_closer;
            let done = match *target {
                // A death the side watched completes the attack.
                Target::Unit(u) if knowledge.knows_destroyed(u) => true,
                Target::Unit(u) => match knowledge.track(u) {
                    Some(track) => {
                        *last_known = Some(track.position.xy());
                        if out_of_reach {
                            unit.pursuit = Some(Pursuit {
                                to: track.position.xy(),
                                searching: false,
                            });
                        }
                        false
                    }
                    None => match *last_known {
                        Some(p) if (unit.position.xy() - p).length() > PURSUIT_ARRIVAL_M => {
                            unit.pursuit = Some(Pursuit {
                                to: p,
                                searching: true,
                            });
                            false
                        }
                        _ => true,
                    },
                },
                Target::Contact(c) => knowledge.ground_contact(c).is_none(),
                Target::Ground(_) => false,
                Target::Round(_) => true,
            };
            if done {
                unit.orders.pop_front();
            }
        }
    }

    /// Return-fire permission lasts only while the side still knows the attacker.
    fn prune_attackers(&mut self) {
        for unit in &mut self.units {
            let knowledge = &self.knowledge[unit.side.index()];
            unit.attackers.retain(|&a| {
                knowledge.track(a).is_some() || knowledge.all_contacts().any(|c| c.emitter == a)
            });
        }
    }

    /// A decoyable round closing on a unit with a flare ready draws one, in
    /// round order, one per flare reload. A seeker follows the flare; an
    /// operator-guided round loses its guidance, as when its operator loses
    /// sight of the target, and flies on to the ground.
    fn dispense_flares(&mut self) {
        // What each round closes on: its seeker's target, or what its
        // launcher steers it at.
        let mut closing: BTreeMap<ProjectileId, UnitId> = self
            .units
            .iter()
            .flat_map(|u| &u.mounts)
            .filter_map(|m| match m.support {
                Some(weapons::Support {
                    projectile,
                    target: Target::Unit(target),
                }) => Some((projectile, target)),
                _ => None,
            })
            .collect();
        closing.extend(
            self.rounds
                .iter()
                .filter_map(|(id, round)| match round.seeker_target {
                    Some(Target::Unit(target)) => Some((*id, target)),
                    _ => None,
                }),
        );
        for (projectile, target_id) in closing {
            let Some(round) = self.rounds.get(&projectile) else {
                continue;
            };
            if !self.arsenal.weapons[round.weapon].def.decoyable {
                continue;
            }
            let target = &self.units[target_id.0 as usize];
            // A wreck or a withdrawn airframe dispenses nothing.
            if !target.alive() {
                continue;
            }
            let Some(flare) = target.unit_type(&self.rules).capabilities.flares else {
                continue;
            };
            if !target
                .flares
                .is_some_and(|f| f.charges > 0 && f.ready_tick <= self.tick)
            {
                continue;
            }
            // Ejected off its side, from its hull's top.
            let half = target.hull.expect("flares are carried by a hull (catalog)");
            let lateral = v2(-libm::sin(target.yaw), libm::cos(target.yaw));
            let position = (target.position.xy() + lateral * (half.y + 0.5))
                .with_z(target.position.z + half.z);
            let base = target.flight_velocity();
            let velocity = v3(
                base.x + lateral.x * 18.0,
                base.y + lateral.y * 18.0,
                base.z - 2.0,
            );
            if let Some(f) = self.units[target_id.0 as usize].flares.as_mut() {
                f.charges -= 1;
                f.ready_tick =
                    self.tick + (flare.reload_s * self.rules.tick_hz as f64).round() as u64;
            }
            // A spoofed round's fuse no longer looks for the unit.
            let round = self.rounds.get_mut(&projectile).expect("checked above");
            round.fused_target = None;
            if round.seeker_target.take().is_some() {
                self.projectiles.decoy_seeker(
                    projectile,
                    crate::flight::Decoy {
                        position,
                        velocity,
                        meet_m: self.rules.guided.flare_meet_m,
                    },
                );
            } else {
                // Its launcher lets go; guidance releases it, as for a lost
                // sight line.
                for mount in self.units.iter_mut().flat_map(|u| &mut u.mounts) {
                    if mount.support.is_some_and(|s| s.projectile == projectile) {
                        mount.support = None;
                    }
                }
            }
        }
    }

    /// This side's own sensors, folded into its knowledge: the observers
    /// due this tick (`all`: every one) identify afresh, the rest keep their
    /// last sightings ([`sensing::SENSE_EVERY`]).
    fn sense(&mut self, side: Side, all: bool) {
        let tick = self.tick;
        let due = |u: &Unit| all || sensing::due(u, tick);
        let units = &self.units;
        let mut sightings: Vec<Sighting> = self.sightings[side.index()]
            .iter()
            .filter(|s| !due(&units[s.observer.0 as usize]))
            .filter_map(|s| sensing::kept(s, units, &self.rules))
            .collect();
        sightings.extend(sensing::evaluate(
            &self.world,
            units,
            &self.rules,
            side,
            due,
        ));
        sightings.sort_by_key(|s| (s.observer, s.target));
        let identified = self.knowledge[side.index()].update(tick, &sightings, units, &self.rules);
        self.knowledge[side.index()].update_radar(tick, side, units, &self.rules);
        self.sightings[side.index()] = sightings;
        // An attack on an area whose cause is now identified carries over to
        // that enemy, queued ones too, as if the player had right-clicked it.
        for (area, enemy) in identified {
            for unit in self.units.iter_mut().filter(|u| u.side == side) {
                for order in &mut unit.orders {
                    match order {
                        UnitOrder::Attack { target, .. } if *target == Target::Contact(area) => {
                            *target = Target::Unit(enemy);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// Recompute what ground this side sees, and learn any new obstacle in view.
    fn sweep_fog(&mut self, side: Side) {
        self.sweep_fog_profiled(side, &mut |_| {});
    }

    fn sweep_fog_profiled(&mut self, side: Side, completed: &mut impl FnMut(TickPhase)) {
        let mut field = self.occlusion.field();
        let mut candidates = Vec::new();
        let mut remembered = Vec::new();
        let mut views = Vec::new();
        for unit in self.units.iter().filter(|u| u.side == side && u.on_field()) {
            // What doesn't see the ground never lights its fog; the optical
            // sensing pass still handles air targets separately.
            if !sensing::sees_ground(unit) {
                continue;
            }
            let sight = sight::of(unit, &self.rules);
            let eyes = sensing::eyes(unit, &self.rules);
            for eye in &eyes {
                // A ray can step one cell past its reach, and the marked cell
                // can contain a footprint sample a diagonal farther away.
                views.push((eye.xy(), sight.max_range() + 3.0 * field.cell_m));
            }
            self.occlusion.sweep_observer(
                &self.world,
                &self.rules.sensors,
                unit.id,
                &eyes,
                &sight,
                &mut field,
            );
        }
        self.world.prop_ids_near_many(&views, &mut candidates);
        self.sides[side.index()].standing_ids_near_many(&views, &mut remembered);
        completed(TickPhase::Fog);
        // Enemy fallen in view are remembered, and the ground in view learned.
        let knowledge = &mut self.knowledge[side.index()];
        knowledge.learn_ground(&self.ground, &field);
        for f in self.fallen.values() {
            if field.visible(f.at[0], f.at[1]) {
                knowledge.learn_fallen(f.prop);
            }
        }
        for u in self.units.iter().filter(|u| u.side != side) {
            for s in &u.members {
                if let Some(fallen) = s.corpse.filter(|f| field.visible(f.at.x, f.at.y)) {
                    knowledge.note_corpse(s.id, fallen);
                }
            }
        }
        // Bodies in view are learned where they stand: new ones, and known
        // ones seen moved (L1, L2); trees seen fallen are gone (Q16).
        let known = &mut self.sides[side.index()];
        let revealed: std::collections::BTreeSet<_> = candidates
            .iter()
            .filter_map(|&id| self.world.prop(id))
            .chain(remembered.iter().filter_map(|id| known.standing().get(id)))
            .filter_map(|p| self.world.building_of(p.id).map(|owner| (owner, p)))
            .filter(|(_, p)| footprint_seen(&field, p))
            .map(|(owner, _)| owner)
            .collect();
        // Seeing one physical part reveals the entire aggregate, including
        // parts beyond the eye's reach. Keep learning in ascending prop order.
        candidates.extend(
            revealed
                .iter()
                .flat_map(|&owner| self.world.current_building_parts(owner).iter().copied()),
        );
        candidates.sort_unstable();
        candidates.dedup();
        remembered.extend(
            revealed
                .iter()
                .flat_map(|&owner| self.world.historical_building_parts(owner).iter().copied()),
        );
        remembered.sort_unstable();
        remembered.dedup();
        let fallen: Vec<PropId> = remembered
            .into_iter()
            .filter_map(|id| known.standing().get(&id))
            .filter(|p| {
                footprint_seen(&field, p)
                    || self
                        .world
                        .building_of(p.id)
                        .is_some_and(|id| revealed.contains(&id))
            })
            .map(|p| p.id)
            .collect();
        for id in fallen {
            known.saw_fallen(id);
        }
        let relearn = self.rules.pushing.relearn_m;
        for prop in candidates.into_iter().filter_map(|id| self.world.prop(id)) {
            if (prop.id >= self.authored_props || known.seen.contains_key(&prop.id))
                && (footprint_seen(&field, prop)
                    || self
                        .world
                        .building_of(prop.id)
                        .is_some_and(|id| revealed.contains(&id)))
            {
                let resting = self.world.resting(prop.id, self.tick);
                known.learn(prop, self.authored_props, relearn, resting);
            }
        }
        self.fog[side.index()] = field;
    }

    fn prepare(
        &mut self,
        mut command: CommandEnvelope,
        planning_tick: Tick,
    ) -> Result<PreparedCommand, OrderError> {
        self.validate(&command)?;
        let player = self
            .skirmish
            .as_ref()
            .and_then(|s| command.player.as_deref().and_then(|id| s.player_index(id)));
        if matches!(command.order, Order::Ready) {
            self.skirmish.as_mut().unwrap().players[player.expect("validated skirmish player")]
                .ready = true;
        }
        if let Order::CancelPending { purchase } = command.order {
            self.skirmish
                .as_mut()
                .unwrap()
                .cancel(player.expect("validated skirmish player"), purchase)?;
        }
        if let Order::ConfirmPurchase {
            variant,
            destination,
        } = &command.order
        {
            let preview = self.preview_purchase(command.side, variant, *destination)?;
            let living = self
                .units
                .iter()
                .filter(|u| {
                    u.side == command.side
                        && u.alive()
                        && u.owner.as_deref() == command.player.as_deref()
                })
                .count() as u32;
            self.skirmish.as_mut().unwrap().reserve_at(
                player.expect("validated skirmish player"),
                preview.kind,
                preview.destination,
                self.rules.catalog.get(preview.kind).cost,
                (crate::entries::EntryIndex(preview.entry), planning_tick + 1),
                living,
            )?;
        }
        let building = if let Order::OccupyBuilding {
            units,
            building,
            facing,
            ..
        } = &command.order
        {
            Some(self.plan_building(
                command.side,
                &BuildingPreviewRequest {
                    units: units.clone(),
                    building: *building,
                    facing: *facing,
                    queued: command.queued,
                },
                planning_tick,
            )?)
        } else {
            None
        };
        let mut stops = BTreeMap::new();
        let placement = match &command.order {
            Order::Refund { units } => {
                let sites = self.skirmish.as_ref().unwrap().setup.sites.clone();
                Some(MovePlacement {
                    gesture: 0,
                    destinations: units
                        .iter()
                        .map(|unit| {
                            let current = &self.units[unit.0 as usize];
                            let selected = if current.fixed_wing() {
                                crate::entries::centre(&sites, command.side)
                            } else {
                                crate::entries::select(
                                    &sites,
                                    command.side,
                                    current.kind,
                                    [current.position.x, current.position.y],
                                    |entry, kind, point| {
                                        self.entry_route_time(
                                            command.side,
                                            kind,
                                            entry.center,
                                            point,
                                        )
                                    },
                                )
                            };
                            let entry = crate::entries::site(&sites, selected);
                            let (goal, facing) = if current.fixed_wing() {
                                let edge =
                                    crate::air::edge::back_edge(command.side, &sites, &self.world);
                                let (point, heading) =
                                    crate::air::edge::entry(&edge, current.position.xy());
                                ([point.x, point.y], heading)
                            } else {
                                (entry.center, entry.yaw)
                            };
                            contract::command::MoveDestination {
                                unit: *unit,
                                goal,
                                placed: true,
                                facing,
                                spots: vec![],
                            }
                        })
                        .collect(),
                })
            }
            Order::OccupyBuilding { gesture, .. } => Some(MovePlacement {
                gesture: *gesture,
                destinations: building.as_ref().unwrap().placement.destinations.clone(),
            }),
            Order::Move {
                units,
                gesture,
                goal,
                facing,
                direction,
                route,
                ..
            } => {
                let (destinations, found) = self.place_move(
                    command.side,
                    &MovePreviewRequest {
                        units: units.clone(),
                        goal: *goal,
                        facing: *facing,
                        direction: *direction,
                        route: *route,
                        queued: command.queued,
                    },
                )?;
                stops = found;
                Some(MovePlacement {
                    gesture: *gesture,
                    destinations,
                })
            }
            Order::AttackMove {
                units,
                gesture,
                goal,
            } => {
                let (destinations, found) = self.place_move(
                    command.side,
                    &MovePreviewRequest {
                        units: units.clone(),
                        goal: *goal,
                        queued: command.queued,
                        ..Default::default()
                    },
                )?;
                stops = found;
                Some(MovePlacement {
                    gesture: *gesture,
                    destinations,
                })
            }
            Order::Unload {
                units,
                gesture,
                goal: Some(goal),
                route,
            } => {
                let (destinations, found) = self.place_move(
                    command.side,
                    &MovePreviewRequest {
                        units: units.clone(),
                        goal: *goal,
                        route: *route,
                        queued: command.queued,
                        ..Default::default()
                    },
                )?;
                stops = found;
                Some(MovePlacement {
                    gesture: *gesture,
                    destinations,
                })
            }
            _ => None,
        };
        let mut refused = Vec::new();
        if let Order::Collect { squads, .. } = &mut command.order {
            // A squad listed twice is collected once; the list keeps its order.
            let mut seen = std::collections::BTreeSet::new();
            squads.retain(|id| seen.insert(*id));
        }
        if let Order::Board { units, carrier, .. } = &mut command.order {
            let mut ordered = units.clone();
            ordered.sort_by(|a, b| {
                let ca = (self.units[a.0 as usize].position.xy()
                    - self.units[carrier.0 as usize].position.xy())
                .length();
                let cb = (self.units[b.0 as usize].position.xy()
                    - self.units[carrier.0 as usize].position.xy())
                .length();
                ca.total_cmp(&cb).then(a.cmp(b))
            });
            // A squad listed twice boards once.
            ordered.dedup();
            let capacity = crate::carriage::seats(&self.rules, self.units[carrier.0 as usize].kind);
            let used = crate::carriage::seats_used(&self.units, *carrier);
            let mut left = capacity.saturating_sub(used);
            let mut accepted = Vec::new();
            for id in ordered {
                let need = crate::carriage::living(&self.units[id.0 as usize]);
                if need <= left {
                    left -= need;
                    accepted.push(id);
                } else {
                    refused.push(contract::command::Refusal {
                        unit: id,
                        reason: OrderError::CapacityFull,
                    });
                }
            }
            *units = accepted;
        }
        if let Order::Unload {
            units, goal: None, ..
        } = &command.order
        {
            for &unit in units {
                if crate::carriage::unload_spots(&self.world, &self.units, unit, &self.rules)
                    .is_none()
                {
                    refused.push(contract::command::Refusal {
                        unit,
                        reason: OrderError::NoRoomToUnload,
                    });
                }
            }
        }
        Ok(PreparedCommand {
            command,
            placement,
            building,
            stops,
            refused,
        })
    }

    fn apply(&mut self, prepared: PreparedCommand) {
        let side = prepared.command.side.index();
        self.refusals[side].extend(prepared.refused.iter().cloned());
        let mut units = std::mem::take(&mut self.units);
        self.apply_units(prepared, &mut units);
        self.units = units;
    }

    fn apply_units(&self, prepared: PreparedCommand, movers: &mut [Unit]) {
        let command = prepared.command;
        let side = command.side;
        let queued = command.queued;
        let push = |unit: &mut Unit, order: UnitOrder| unit.enqueue(order, queued);
        let replacing = match &command.order {
            Order::Move { units, .. }
            | Order::AttackMove { units, .. }
            | Order::Stop { units }
            | Order::Attack { units, .. }
            | Order::SetAfterburner { units, .. }
            | Order::SetEngagement { units, .. }
            | Order::SetDeployment { units, .. }
            | Order::Garrison { units, .. }
            | Order::OccupyBuilding { units, .. }
            | Order::ExitBuilding { units }
            | Order::Unload { units, .. }
            | Order::Refund { units } => Some(units),
            _ => None,
        };
        if let Some(units) = replacing {
            for id in units {
                take_order(&mut movers[id.0 as usize]);
            }
        }
        match command.order {
            Order::Refund { .. } => {
                for slot in prepared.placement.unwrap().destinations {
                    if !slot.placed {
                        continue;
                    }
                    let unit = &mut movers[slot.unit.0 as usize];
                    unit.enqueue(
                        UnitOrder::Move(MoveOrder {
                            destination: v2(slot.goal[0], slot.goal[1]),
                            policy: RoutePolicy::Fastest,
                            gesture: 0,
                            direction: Default::default(),
                            facing: None,
                            short: None,
                        }),
                        false,
                    );
                    unit.withdrawing = true;
                }
            }
            Order::Ready | Order::ConfirmPurchase { .. } | Order::CancelPending { .. } => {}
            Order::OccupyBuilding {
                gesture, facing, ..
            } => {
                let garrison::BuildingPlan {
                    placement: plan,
                    entry_action,
                } = prepared.building.unwrap();
                if let Some(entry) = plan.entrant {
                    let unit = &mut movers[entry.unit.0 as usize];
                    let approach = v2(entry.approach[0], entry.approach[1]);
                    if matches!(entry_action, garrison::EntryAction::Reassert) {
                        unit.enqueue(
                            UnitOrder::Garrison {
                                building: plan.building,
                                approach,
                            },
                            false,
                        );
                    } else if matches!(entry_action, garrison::EntryAction::Route) {
                        push(
                            unit,
                            UnitOrder::Move(MoveOrder {
                                destination: approach,
                                policy: RoutePolicy::Shortest,
                                gesture,
                                direction: Default::default(),
                                facing: None,
                                short: None,
                            }),
                        );
                        unit.enqueue(
                            UnitOrder::Garrison {
                                building: plan.building,
                                approach,
                            },
                            true,
                        );
                    }
                }
                for slot in plan.destinations {
                    let unit = &mut movers[slot.unit.0 as usize];
                    if slot.placed {
                        push(
                            unit,
                            UnitOrder::Move(MoveOrder {
                                destination: v2(slot.goal[0], slot.goal[1]),
                                policy: RoutePolicy::Shortest,
                                gesture,
                                direction: Default::default(),
                                facing,
                                short: None,
                            }),
                        );
                    } else if !queued {
                        Self::hold_position(unit);
                    }
                }
            }
            Order::Move {
                units: _,
                gesture,
                goal: _,
                route,
                direction,
                facing,
            } => {
                for slot in prepared.placement.unwrap().destinations {
                    let unit = &mut movers[slot.unit.0 as usize];
                    crate::air::dogfight::clear(unit);
                    if !slot.placed {
                        if !queued {
                            Self::hold_position(unit);
                        }
                        continue;
                    }
                    let destination = v2(slot.goal[0], slot.goal[1]);
                    let order = UnitOrder::Move(MoveOrder {
                        destination,
                        policy: route,
                        gesture,
                        direction,
                        facing,
                        short: prepared.stops.get(&slot.unit).cloned(),
                    });
                    push(unit, order);
                }
            }
            Order::AttackMove { gesture, .. } => {
                for slot in prepared.placement.unwrap().destinations {
                    let unit = &mut movers[slot.unit.0 as usize];
                    if !slot.placed {
                        if !queued {
                            Self::hold_position(unit);
                        }
                        continue;
                    }
                    let destination = v2(slot.goal[0], slot.goal[1]);
                    controllers::fire_at_will(unit);
                    let order = UnitOrder::AttackMove(MoveOrder {
                        destination,
                        policy: contract::command::RoutePolicy::Shortest,
                        gesture,
                        direction: contract::command::MoveDirection::Forward,
                        facing: None,
                        short: prepared.stops.get(&slot.unit).cloned(),
                    });
                    push(unit, order);
                }
            }
            Order::Attack { units, target } => {
                // Resolved again at application; a target lost since the ack is dropped.
                let Some(target) = self.resolve_target(side, &target) else {
                    return;
                };
                for id in units {
                    let dogfight = match target {
                        Target::Unit(other)
                            if crate::air::dogfight::can_dogfight(
                                &movers[id.0 as usize],
                                &movers[other.0 as usize],
                            ) =>
                        {
                            Some(other)
                        }
                        _ => None,
                    };
                    let unit = &mut movers[id.0 as usize];
                    controllers::fire_at_will(unit);
                    push(
                        unit,
                        UnitOrder::Attack {
                            target,
                            last_known: None,
                        },
                    );
                    if let Some(target) = dogfight {
                        crate::air::dogfight::set_target(unit, target);
                    }
                }
            }
            Order::Garrison { units, building } => {
                let building = self.world.remembered_structure_owner(building);
                for id in units {
                    let unit = &mut movers[id.0 as usize];
                    let from = unit.position.xy();
                    let Some(approach) = garrison::approach(
                        &self.world,
                        &self.sides[side.index()],
                        self.authored_props,
                        building,
                        from,
                        self.rules.garrison.entry_distance_m / 2.0,
                    ) else {
                        continue;
                    };
                    push(unit, UnitOrder::Garrison { building, approach });
                }
            }
            Order::ExitBuilding { units } => {
                for id in units {
                    push(&mut movers[id.0 as usize], UnitOrder::Exit);
                }
            }
            Order::Board {
                units,
                carrier,
                gesture,
            } => {
                for id in units {
                    push(
                        &mut movers[id.0 as usize],
                        UnitOrder::Board { carrier, gesture },
                    );
                }
            }
            Order::Collect {
                units,
                squads,
                gesture,
            } => {
                for id in units {
                    push(
                        &mut movers[id.0 as usize],
                        UnitOrder::Collect {
                            squads: squads.clone(),
                            gesture,
                        },
                    );
                }
            }
            Order::Unload {
                units,
                goal,
                gesture,
                route,
            } => {
                let refused: std::collections::BTreeSet<UnitId> = prepared
                    .refused
                    .iter()
                    .filter(|refusal| refusal.reason == OrderError::NoRoomToUnload)
                    .map(|refusal| refusal.unit)
                    .collect();
                for id in units {
                    let unit = &mut movers[id.0 as usize];
                    if refused.contains(&id) {
                        Self::hold_position(unit);
                        continue;
                    }
                    match goal {
                        Some(_) => {
                            let Some(placement) = prepared.placement.as_ref() else {
                                continue;
                            };
                            let Some(destination) = placement
                                .destinations
                                .iter()
                                .find(|slot| slot.unit == id && slot.placed)
                                .map(|slot| v2(slot.goal[0], slot.goal[1]))
                            else {
                                if !queued {
                                    Self::hold_position(unit);
                                }
                                continue;
                            };
                            push(
                                unit,
                                UnitOrder::Move(MoveOrder {
                                    destination,
                                    policy: route,
                                    gesture,
                                    direction: Default::default(),
                                    facing: None,
                                    short: prepared.stops.get(&id).cloned(),
                                }),
                            );
                            unit.enqueue(UnitOrder::Unload, true);
                        }
                        None => {
                            Self::hold_position(unit);
                            unit.enqueue(UnitOrder::Unload, false);
                        }
                    }
                }
            }
            Order::SetEngagement { units, policy } => {
                for id in units {
                    controllers::order_engagement(&mut movers[id.0 as usize], policy);
                }
            }
            Order::SetAfterburner { units, enabled } => {
                for id in units {
                    if let Some(air) = movers[id.0 as usize].air.as_mut() {
                        air.afterburner = enabled;
                    }
                }
            }
            Order::Stop { units } => {
                // W15: clear the queue and cancel movement, aim, reload and
                // guidance once; policy stays; automatic fire may restart.
                for id in units {
                    let unit = &mut movers[id.0 as usize];
                    unit.orders.clear();
                    unit.route = None;
                    unit.planned_goal = None;
                    unit.pursuit = None;
                    unit.state = MoveState::Idle;
                    unit.drive_speed_mps = 0.0;
                    unit.blocker = None;
                    unit.turn_to = None;
                    for mount in &mut unit.mounts {
                        mount.stop();
                    }
                    // Stop never leaves a unit half-packed: it sets up where it is.
                    if let Some(d) = unit.deployment.as_mut() {
                        d.stationary = Posture::Deployed;
                    }
                }
            }
            Order::SetDeployment { units, deployed } => {
                for id in units {
                    let unit = &mut movers[id.0 as usize];
                    let Some(d) = unit.deployment.as_mut() else {
                        continue; // units that never set up ignore it
                    };
                    if deployed {
                        // Set up here: movement ends and progress reverses from where it stands.
                        d.stationary = Posture::Deployed;
                        unit.orders.clear();
                        unit.route = None;
                        unit.planned_goal = None;
                        unit.pursuit = None;
                        unit.state = MoveState::Idle;
                        unit.blocker = None;
                    } else {
                        // Pack and stay packed once any movement ends.
                        d.stationary = Posture::Packed;
                    }
                }
            }
            Order::UpgradeMove { gesture, route } => {
                for unit in movers.iter_mut().filter(|u| u.side == side) {
                    let mut upgraded = false;
                    for (k, order) in unit.orders.iter_mut().enumerate() {
                        if let Some(m) = order.movement_mut() {
                            if m.gesture == gesture && m.policy != route {
                                m.policy = route;
                                upgraded = true;
                                if k == 0 {
                                    unit.route = None; // replan the active order
                                    unit.planned_goal = None;
                                }
                            }
                        }
                    }
                    if upgraded {
                        take_order(unit);
                    }
                }
            }
        }
    }

    fn hold_position(unit: &mut Unit) {
        unit.orders.clear();
        unit.route = None;
        unit.planned_goal = None;
        unit.pursuit = None;
        unit.state = MoveState::Idle;
        unit.drive_speed_mps = 0.0;
        unit.blocker = None;
        unit.turn_to = None;
    }

    /// Preview placement on this side's known map without advancing time,
    /// recording a command, changing orders, or warming live navigation caches.
    /// Admission uses side knowledge, so an unseen enemy body cannot reject a ghost.
    pub fn preview_purchase(
        &mut self,
        side: Side,
        variant: &str,
        destination: [f64; 2],
    ) -> Result<contract::skirmish::PurchasePreview, OrderError> {
        if side == Side::Neutral {
            return Err(OrderError::NotSkirmish);
        }
        let sites = self
            .skirmish
            .as_ref()
            .ok_or(OrderError::NotSkirmish)?
            .setup
            .sites
            .clone();
        let kind = self
            .rules
            .catalog
            .index(variant)
            .ok_or(OrderError::UnitUnavailable)?;
        let card = self
            .rules
            .catalog
            .card(variant)
            .ok_or(OrderError::UnitUnavailable)?;
        if !card.roster.factions.contains(
            &self
                .skirmish
                .as_ref()
                .and_then(|_| self.player_faction(side))
                .ok_or(OrderError::WrongFaction)?,
        ) {
            return Err(OrderError::WrongFaction);
        }
        if !destination.iter().all(|n| n.is_finite())
            || self
                .world
                .height_at(destination[0], destination[1])
                .is_none()
        {
            return Err(OrderError::OutOfBounds);
        }
        let fixed_wing = matches!(
            units::motion(self.rules.catalog.get(kind), &self.rules),
            units::Motion::Air(f) if f.model == contract::catalog::FlightModel::FixedWing
        );
        let entry_index = if fixed_wing {
            crate::entries::centre(&sites, side)
        } else {
            self.select_entry(side, kind, destination)
        };
        let entry = crate::entries::site(&sites, entry_index);
        let (from, yaw) = (v2(entry.center[0], entry.center[1]), entry.yaw);
        let t = self.rules.catalog.get(kind);
        // A ground unit is sent where a move there from its entry would end:
        // the nearest ground to the point that it stands on, as its side knows
        // the map. An aircraft's destination is any point it can fly over.
        let destination = match units::ground_mobility(t, &self.rules) {
            None => destination,
            Some(mobility) => {
                let mut known = self.sides[side.index()].clone();
                let grid = known.grid(&self.world, self.authored_props);
                let pockets = crate::navigation::Pockets::default();
                let hull = t.hull().map(|h| crate::navigation::Parking {
                    half: v2(h.half_extents_m[0], h.half_extents_m[1]),
                    facing: None,
                });
                let member = crate::formation::Member {
                    id: UnitId(self.units.len() as u32),
                    position: from,
                    radius: crate::formation::member_radius(&self.rules, t),
                    yaw,
                };
                let plan = crate::formation::place(
                    &[member],
                    v2(destination[0], destination[1]),
                    None,
                    &self.rules.formation,
                    libm::hypot(self.world.width(), self.world.depth()),
                    |_, p| grid.destination_point(p, &mobility, hull, from, &pockets),
                );
                let placed = plan.slots[0].point.ok_or(OrderError::NoValidDestination)?;
                [placed.x, placed.y]
            }
        };
        let soldiers = if t.hull().is_some() {
            0
        } else {
            t.slots().map_or(0, |s| s.len())
        };
        let next = self.units.len() as u32;
        Ok(contract::skirmish::PurchasePreview {
            kind,
            entry: entry_index.0,
            destination,
            // A look while it is placed, not where its soldiers will stand: one
            // draw wherever it is put, so the ghost slides with the cursor.
            spots: self.arrival_spots(side, destination, soldiers, next, 0),
        })
    }

    /// Where `count` soldiers stand round `centre` as a squad arriving there
    /// spreads out on the ground `side` knows, before cover moves them, by
    /// the arrangement `draw` ([`arrangement::rng`]). It changes no state.
    fn arrival_spots(
        &self,
        side: Side,
        centre: [f64; 2],
        count: usize,
        unit: u32,
        draw: u64,
    ) -> Vec<[f64; 2]> {
        if count == 0 {
            return vec![];
        }
        let known = &self.sides[side.index()];
        let solid = |p: &crate::world::Prop| {
            p.blocks(MoverClass::Infantry) && known.knows(p, self.authored_props)
        };
        let centre = v2(centre[0], centre[1]);
        let mut draws = arrangement::rng(self.seed, unit, draw);
        arrangement::squad_spots(
            &self.world,
            centre,
            count,
            &self.rules.infantry_movement,
            self.rules.physics.soldier_radius_m,
            &solid,
            &mut draws,
        )
        .into_iter()
        .map(|p| [p.x, p.y])
        .collect()
    }

    pub fn preview_move(
        &mut self,
        side: Side,
        request: &MovePreviewRequest,
    ) -> Result<Vec<contract::command::MoveDestination>, OrderError> {
        self.place_move(side, request)
            .map(|(destinations, _)| destinations)
    }

    /// Each unit's destination, and where a hull sent onto ground it has no
    /// room on stops short of it instead, if the room nearest is a long way
    /// round ([`crate::units::StopShort`]).
    fn place_move(
        &mut self,
        side: Side,
        request: &MovePreviewRequest,
    ) -> Result<
        (
            Vec<contract::command::MoveDestination>,
            BTreeMap<UnitId, crate::units::StopShort>,
        ),
        OrderError,
    > {
        let ids = &request.units;
        let goal = request.goal;
        let facing = request.facing;
        self.validate_units(side, ids)?;
        if self.world.height_at(goal[0], goal[1]).is_none()
            || facing.is_some_and(|f| !f.is_finite())
        {
            return Err(OrderError::OutOfBounds);
        }
        let members: Vec<_> = ids
            .iter()
            .map(|&id| {
                let u = &self.units[id.0 as usize];
                crate::formation::Member {
                    id,
                    position: u.position.xy(),
                    yaw: u.yaw,
                    radius: crate::formation::member_radius(&self.rules, u.unit_type(&self.rules)),
                }
            })
            .collect();
        let mut known = self.sides[side.index()].clone();
        let grid = known.grid(&self.world, self.authored_props);
        let pockets = crate::navigation::Pockets::default();
        // Per unit, the last point placed for it and where it stops short.
        let shorts = std::cell::RefCell::new(BTreeMap::new());
        // A queued leg sets off from where the queue ends.
        let from = |u: &Unit| {
            u.orders
                .iter()
                .rev()
                .filter(|_| request.queued)
                .find_map(|o| o.movement())
                .map_or(u.position.xy(), |m| m.destination)
        };
        let plan = crate::formation::place(
            &members,
            v2(goal[0], goal[1]),
            facing,
            &self.rules.formation,
            libm::hypot(self.world.width(), self.world.depth()),
            |id, p| {
                let u = &self.units[id.0 as usize];
                // An aircraft hovers over its point, whatever stands there.
                if u.airborne() {
                    shorts.borrow_mut().insert(id, (p, None));
                    return Some(p);
                }
                // A hull that turns on the spot parks facing as ordered.
                let pivots = u.ground().drive.is_some_and(|d| d.tracked);
                let hull = u.hull.map(|h| crate::navigation::Parking {
                    half: h.xy(),
                    facing: request.facing.filter(|_| pivots),
                });
                let placed = grid.destination_point(p, u.ground(), hull, from(u), &pockets)?;
                let short = hull
                    .filter(|_| placed != p)
                    .and_then(|hull| grid.room_on_the_way(p, u.ground(), hull, from(u), &pockets))
                    .filter(|&at| at != placed)
                    .map(|at| crate::units::StopShort {
                        at,
                        detour_m: self.rules.navigation.stop_short_detour_ratio
                            * ((at - p).length() - (placed - p).length()).max(0.0),
                        nearest: None,
                    });
                shorts.borrow_mut().insert(id, (placed, short));
                Some(placed)
            },
        );
        // A placed marker is a place to stand that its unit can reach on the
        // ground its side knows; the way there is found and kept on the move.
        let source = self.move_source(side);
        let shorts = shorts.into_inner();
        // Where each placed unit stops short, if it was placed where that
        // was worked out for.
        let stops = plan
            .slots
            .iter()
            .filter_map(|slot| {
                let (placed, short) = shorts.get(&slot.id)?;
                (slot.point == Some(*placed)).then(|| Some((slot.id, short.clone()?)))?
            })
            .collect();
        let destinations = plan
            .slots
            .into_iter()
            .map(|slot| {
                let u = &self.units[slot.id.0 as usize];
                // An attack has no end to set a queued leg off from.
                let after_attack = request.queued
                    && source[slot.id.0 as usize]
                        .orders
                        .iter()
                        .any(|o| matches!(o, UnitOrder::Attack { .. }));
                let slot_point = slot.point.filter(|p| {
                    !after_attack
                        && (u.airborne() || grid.reaches(from(u), *p, u.ground(), &pockets))
                });
                let goal = slot_point.unwrap_or(u.position.xy());
                let facing =
                    movement::final_yaw(u, request.facing, from(u), goal, request.direction);
                let living = if u.hull.is_some() {
                    0
                } else {
                    u.members.iter().filter(|s| s.alive()).count()
                };
                contract::command::MoveDestination {
                    unit: slot.id,
                    goal: [goal.x, goal.y],
                    placed: slot_point.is_some(),
                    facing: facing.unwrap_or(u.yaw),
                    spots: self.arrival_spots(
                        side,
                        [goal.x, goal.y],
                        living,
                        slot.id.0,
                        arrangement::arrival(goal),
                    ),
                }
            })
            .collect();
        Ok((destinations, stops))
    }

    /// Resolve entry and gathering using only this side's known, isolated state.
    pub fn preview_building(
        &self,
        side: Side,
        request: &BuildingPreviewRequest,
    ) -> Result<BuildingPlacement, OrderError> {
        self.plan_building(side, request, self.tick)
            .map(|p| p.placement)
    }

    fn plan_building(
        &self,
        side: Side,
        request: &BuildingPreviewRequest,
        planning_tick: Tick,
    ) -> Result<garrison::BuildingPlan, OrderError> {
        self.validate_units(side, &request.units)?;
        if request.facing.is_some_and(|f| !f.is_finite()) {
            return Err(OrderError::OutOfBounds);
        }
        let source = self.move_source(side);
        // Live admission precedes its application tick; replay and scripts
        // already entered that tick. Timed prefix prediction must start from
        // the same prior-state tick in all three paths.
        let mut ctx = self.movement_context();
        ctx.tick = planning_tick;
        garrison::occupy(&ctx, &self.sides[side.index()], &source, request)
    }

    fn move_source(&self, side: Side) -> Vec<Unit> {
        let mut source = self.units.clone();
        for unit in &mut source {
            if unit.side != side {
                let track = self.knowledge[side.index()].track(unit.id);
                for (k, soldier) in unit.members.iter_mut().enumerate() {
                    let seen =
                        track.is_some_and(|t| t.last_seen == self.tick && t.members.contains(&k));
                    soldier.hp = if seen { 1.0 } else { 0.0 };
                    if !seen {
                        soldier.position = track.map_or(Default::default(), |t| t.position);
                    }
                    soldier.path.clear();
                    soldier.spot = None;
                    soldier.post = None;
                    soldier.lean = None;
                    soldier.cover = None;
                }
                unit.cover = Default::default();
                unit.attackers.clear();
                unit.suppression = 0.0;
                for mount in &mut unit.mounts {
                    mount.stop();
                }
                if let Some(track) = track {
                    unit.position = track.position;
                    unit.yaw = track.yaw;
                    unit.hp = 1.0;
                } else {
                    unit.hp = 0.0;
                }
            }
        }
        // Admission sees earlier commands exactly as execution will apply them.
        for pending in &self.pending {
            if pending.command.side == side {
                self.apply_units(pending.clone(), &mut source);
            }
        }
        source
    }

    fn movement_context(&self) -> MovementContext<'_> {
        MovementContext {
            world: &self.world,
            roads: &self.roads,
            ground: &self.ground,
            authored: self.authored_props,
            tick: self.tick,
            seed: self.seed,
            rules: &self.rules,
            knowledge: std::array::from_fn(|i| &self.knowledge[i]),
            arsenal: &self.arsenal,
        }
    }

    /// A soldier's resolved place and cover (D2+): his spot while moving, his
    /// post while holding, else where he stands; the cover he has now against
    /// his squad's threat, in the true world as rounds meet it, and the tier
    /// his place was resolved to give. A garrison shelters instead (Q22).
    fn member_order(
        world: &WorldGeometry,
        ground: &GroundLayer,
        rules: &Rules,
        u: &Unit,
        s: &Soldier,
        hulls: &[cover::Body],
    ) -> MemberOrder {
        let here = s.position.xy();
        let spot = s.spot.or(s.post).unwrap_or(here);
        let sheltered = u.hold.is_some();
        let cover_now = u
            .cover
            .threat
            .filter(|_| !sheltered)
            .and_then(|t| cover::at(world, ground, hulls, rules, here, t));
        MemberOrder {
            spot: [spot.x, spot.y],
            cover_now,
            cover_there: s.cover.filter(|_| !sheltered),
        }
    }

    fn observe_all(&mut self) {
        let identified = Side::ALL
            .into_iter()
            .flat_map(|side| self.knowledge[side.index()].identified_now(self.tick))
            .map(|(id, _)| (id, self.units[id.0 as usize].layer()))
            .collect();
        self.shout_evidence = if self.tick == 0 {
            ShoutEvidence::default()
        } else {
            let physical = shouts::Physical {
                under_fire: std::mem::take(&mut self.shout_under_fire),
                incoming: shouts::incoming(
                    &self.world,
                    self.projectiles.active(),
                    &self.rounds,
                    &self.units,
                    &self.rules,
                ),
            };
            shouts::evidence(
                &self.shout_before,
                &self.units,
                &self.world,
                &physical,
                std::mem::take(&mut self.shout_fired),
                identified,
                &self.rules,
            )
        };
        self.shout_evidence.kills.append(&mut self.shout_kills);
        let own_shouts = Side::ALL.map(|side| {
            shouts::build(
                side,
                &self.shout_evidence,
                &self.units,
                &self.knowledge[side.index()],
                &self.rules,
                self.tick,
                &mut self.shouts[side.index()],
            )
        });
        // Every live hull a soldier's current cover may lie behind (D2+).
        let hulls = cover::hull_bodies(&crate::lean::hulls(&self.units, &self.rules));
        let low_hover = crate::movement::air::low_hover(&self.rules);
        for side in Side::ALL {
            let knowledge = &self.knowledge[side.index()];
            let spotted_by_visible_enemy: BTreeSet<UnitId> = Side::ALL
                .into_iter()
                .filter(|other| *other != side)
                .flat_map(|other| self.sightings[other.index()].iter())
                .filter(|s| {
                    knowledge
                        .track(s.observer)
                        .is_some_and(|t| t.last_seen == self.tick)
                })
                .map(|s| s.target)
                .collect();
            let fog = &self.fog[side.index()];
            let player_ids = self.player_ids.clone();
            let frame = &mut self.observations[side.index()];
            frame.tick = self.tick;
            frame.identified.clear();
            frame
                .identified
                .extend(knowledge.identified(self.tick, &self.units, &self.rules));
            frame.ground_visibility.clone_from(fog);
            frame.contacts.clear();
            // An area hangs in the air when its cause flew higher over the
            // ground than the low hover (D33).
            let world = &self.world;
            frame.contacts.extend(knowledge.contacts(|c| {
                c.layer != AltitudeLayer::Ground
                    && c.center.z - world.height_at(c.center.x, c.center.y).unwrap_or(0.0)
                        > low_hover
            }));
            frame.audible.clone_from(&self.audible[side.index()]);
            // Refusals are transient evidence for this completed tick.
            frame.refusals = std::mem::take(&mut self.refusals[side.index()]);
            frame.shouts.clone_from(&own_shouts[side.index()]);
            for other in Side::ALL.into_iter().filter(|other| *other != side) {
                frame.shouts.extend(shouts::heard(
                    &own_shouts[other.index()],
                    knowledge,
                    self.tick,
                ));
                self.shouted_unseen[side.index()].extend(shouts::unheard(
                    &own_shouts[other.index()],
                    knowledge,
                    self.tick,
                ));
            }
            frame.known_props.clear();
            let known = &self.sides[side.index()];
            // Planning and publication share the complete remembered pose.
            let remembered = known.seen.keys().filter_map(|&id| {
                let p = known.prop(&self.world, self.authored_props, id)?;
                let replaces = if id < self.authored_props {
                    Some(id)
                } else {
                    self.structures.replaced_by(id)
                };
                Some((p, replaces, false))
            });
            // A learned removal without a replacement only hides its authored body.
            let removed = self
                .structures
                .removed()
                .filter(|p| p.id < self.authored_props && !known.standing().contains_key(&p.id))
                .map(|p| (p.clone(), Some(p.id), true));
            for (p, replaces, destroyed) in remembered.chain(removed) {
                let building = self.world.building_of(p.id);
                frame.known_props.push(KnownProp {
                    kind: p.kind,
                    center: [p.center.x, p.center.y],
                    yaw: p.yaw,
                    half_extents: [p.half.x, p.half.y, p.half.z],
                    base_z: p.base_z,
                    replaces,
                    destroyed,
                    id: p.id,
                    building,
                    structure_owner: building
                        .filter(|_| !destroyed)
                        .map(|_| self.world.remembered_structure_owner(p.id)),
                    authored_prop: self.world.authored_prop(p.id),
                    wreck_of: p.wreck_of,
                });
            }
            // A body shoved where this side sees it (its own pusher always
            // does) is drawn where it stands, ahead of the knowledge above;
            // a metal one throws sparks where the hull grinds it.
            frame.prop_display_poses.clear();
            frame.grinds.clear();
            for shove in &self.shoved {
                let Some(p) = self.world.prop(shove.prop) else {
                    continue;
                };
                let visible = shove.by == side || fog.visible(p.center.x, p.center.y);
                if !visible {
                    continue;
                }
                let drawn = frame.prop_display_poses.iter().any(|d| d.prop_id == p.id);
                if !drawn {
                    frame.prop_display_poses.push(PropDisplayPose {
                        prop_id: p.id,
                        center: [p.center.x, p.center.y],
                        yaw: p.yaw,
                        base_z: p.base_z,
                    });
                }
                if p.body.metal {
                    frame.grinds.push(Grind {
                        point: [shove.contact.x, shove.contact.y, shove.contact.z],
                        heading: [shove.heading.x, shove.heading.y],
                    });
                }
            }
            frame.projectiles.clear();
            for round in &self.segments {
                if round.side == side {
                    frame.projectiles.push(round.whole());
                } else {
                    frame.projectiles.extend(clip_to_seen(fog, round));
                }
            }
            frame.blasts.clear();
            frame.blasts.extend(
                self.blasts
                    .iter()
                    .filter(|(by, b)| *by == side || fog.visible(b.point[0], b.point[1]))
                    .map(|(_, b)| b.clone()),
            );
            let target_ref = |t: Target| -> Option<TargetRef> {
                match t {
                    Target::Unit(u) => knowledge
                        .track(u)
                        .map(|tr| TargetRef::Identified { id: tr.id }),
                    Target::Contact(c) => Some(TargetRef::Contact { id: c }),
                    Target::Ground(p) => Some(TargetRef::Ground {
                        point: [p.x, p.y, p.z],
                    }),
                    Target::Round(_) => None,
                }
            };
            frame.own.clear();
            frame.own.extend(
                self.units
                    .iter()
                    .filter(|u| u.side == side && u.alive())
                    .map(|u| OwnUnit {
                        withdrawing: u.withdrawing,
                        fuel_s: u.air.as_ref().map(|air| air.fuel_s),
                        afterburner: u.air.as_ref().is_some_and(|air| air.afterburner),
                        in_dogfight: u
                            .air
                            .as_ref()
                            .is_some_and(|air| air.dogfight_target.is_some()),
                        flares: u.flares.and_then(|state| {
                            u.unit_type(&self.rules)
                                .capabilities
                                .flares
                                .map(|definition| {
                                    let remaining =
                                        (state.ready_tick as f64 - self.tick as f64).max(0.0);
                                    contract::observation::FlareReadiness {
                                        charges: state.charges,
                                        cooldown: (remaining > 0.0).then(|| {
                                            (1.0 - remaining
                                                / (definition.reload_s
                                                    * f64::from(self.rules.tick_hz)))
                                            .clamp(0.0, 1.0)
                                        }),
                                    }
                                })
                        }),
                        protection: u.protection.as_ref().map(|p| {
                            let definition = u
                                .unit_type(&self.rules)
                                .capabilities
                                .active_protection
                                .as_ref()
                                .unwrap();
                            let remaining = (p.ready_at_tick - self.tick as f64).max(0.0);
                            contract::observation::ProtectionReadiness {
                                charges: p.charges,
                                cooldown: (remaining > 0.0).then(|| {
                                    (1.0 - remaining
                                        / (definition.cooldown_s * f64::from(self.rules.tick_hz)))
                                    .clamp(0.0, 1.0)
                                }),
                            }
                        }),
                        id: u.id,
                        owner: state_digest::owner_index(&player_ids, u)
                            .map(|index| contract::players::PlayerIndex(index as u16)),
                        kind: u.kind,
                        position: [u.position.x, u.position.y, u.position.z],
                        yaw: u.yaw,
                        goal: u.movement_goal().map(|(g, _)| [g.x, g.y]),
                        goal_unloads: u.movement_goal().is_some() && u.unloads_after(0),
                        policy: u.movement_goal().map(|(_, p)| p),
                        direction: u.movement_goal().map(|_| u.direction()),
                        reversing: u.reversing,
                        state: u.state,
                        // An enemy it waits for is named only if the side
                        // could name it anyway (Q14: contact reveals nothing).
                        blocker: u.blocker.filter(|b| self.units[b.0 as usize].side == side),
                        route: u.route.iter().flatten().map(|p| [p.x, p.y]).collect(),
                        queue: u.queued_steps().map(|(p, _)| [p.x, p.y]).collect(),
                        queue_unloads: u.queued_steps().map(|(_, unloads)| unloads).collect(),
                        members: u
                            .members
                            .iter()
                            .enumerate()
                            .filter(|(_, s)| s.alive())
                            .map(|(k, _)| {
                                let p = garrison::body_position(u, k, &self.rules);
                                [p.x, p.y, p.z]
                            })
                            .collect(),
                        member_ids: u
                            .members
                            .iter()
                            .filter(|s| s.alive())
                            .map(|s| s.id)
                            .collect(),
                        member_slots: u
                            .members
                            .iter()
                            .filter(|s| s.alive())
                            .map(|s| s.slot as u8)
                            .collect(),
                        member_active_mounts: u
                            .members
                            .iter()
                            .filter(|s| s.alive())
                            .map(|s| s.active_mount.map(|m| m as u8))
                            .collect(),
                        member_orders: u
                            .members
                            .iter()
                            .filter(|s| s.alive())
                            .map(|s| {
                                Self::member_order(
                                    &self.world,
                                    &self.ground,
                                    &self.rules,
                                    u,
                                    s,
                                    &hulls,
                                )
                            })
                            .collect(),
                        member_leans: u
                            .members
                            .iter()
                            .filter(|s| s.alive())
                            .map(|s| s.leaning(self.tick).map(|l| l.published()))
                            .collect(),
                        area: u.anchor.map(|a| SquadArea {
                            anchor: [a.at.x, a.at.y],
                            radius: crate::cover::area_radius(
                                &self.rules,
                                u.unit_type(&self.rules),
                            ),
                        }),
                        final_facing: movement::final_facing(u),
                        sees: knowledge.own_sensor(u.id),
                        engagement: u.engagement,
                        mounts: u
                            .mounts
                            .iter()
                            .map(|m| {
                                weapons::readiness(
                                    &self.arsenal,
                                    u,
                                    m,
                                    m.lock.as_ref().and_then(|l| target_ref(l.target)),
                                )
                            })
                            .collect(),
                        weapon_poses: u.mounts.iter().map(weapons::pose).collect(),
                        deployment: deployment::state(u),
                        hp: u.hp,
                        smoking: u.smoking(&self.rules),
                        member_hp: u
                            .members
                            .iter()
                            .filter(|s| s.alive())
                            .map(|s| s.hp)
                            .collect(),
                        suppression: self.rules.suppression.tier(u.suppression),
                        concealed: sensing::concealed(&self.world, u, &self.rules)
                            && !spotted_by_visible_enemy.contains(&u.id)
                            && u.attackers.is_empty(),
                        stock: u.stock,
                        service: u.service,
                        hold: crate::hold::state(&self.world, u, &self.rules),
                        sight: {
                            let s = sight::of(u, &self.rules);
                            UnitSight {
                                // The side's drawn fog follows its rules: a
                                // unit that doesn't see the ground lights none.
                                eyes: if sensing::sees_ground(u) {
                                    sensing::eyes(u, &self.rules)
                                        .into_iter()
                                        .map(|e| [e.x, e.y, e.z])
                                        .collect()
                                } else {
                                    Vec::new()
                                },
                                forward: s.forward,
                                shape: s.shape,
                                range: s.range,
                            }
                        },
                    }),
            );
            frame.encounter = self.encounter;
            // Economy is a deliberately binary mode. Neutral still gets a
            // normal observation stream, but it has no wallet, purchase
            // queue, or objective score to publish.
            frame.skirmish = self
                .skirmish
                .as_ref()
                .filter(|_| side != Side::Neutral)
                .map(|s| {
                    let mut living = vec![0; s.players.len()];
                    for unit in self.units.iter().filter(|u| u.side == side && u.alive()) {
                        if let Some(player) =
                            unit.owner.as_deref().and_then(|id| s.player_index(id))
                        {
                            living[player] += 1;
                        }
                    }
                    s.view(side, self.tick, self.rules.tick_hz, &living)
                });
            frame.guided.clear();
            frame
                .guided
                .extend(self.projectiles.active().iter().filter_map(|p| {
                    let own = self.rounds.get(&p.id).is_some_and(|r| r.side == side);
                    p.guidance.filter(|_| own).map(|g| GuidedMissile {
                        id: p.id.0,
                        position: [p.position.x, p.position.y, p.position.z],
                        point: [g.point.x, g.point.y, g.point.z],
                        supported: g.supported,
                    })
                }));
            frame.air_transits.clear();
            frame
                .air_transits
                .extend(self.air_transits[side.index()].iter().copied());
            frame.corpses.clear();
            for u in &self.units {
                for s in &u.members {
                    let own = u.side == side;
                    let fallen = s.corpse.and_then(|actual| {
                        if own {
                            Some(actual)
                        } else {
                            knowledge.corpse(s.id, actual)
                        }
                    });
                    if let Some(f) = fallen {
                        frame.corpses.push(Corpse {
                            position: [f.at.x, f.at.y, f.at.z],
                            own,
                            soldier: s.id,
                            kind: u.kind,
                            slot: s.slot as u8,
                            yaw: f.yaw,
                        });
                    }
                }
            }
            // By place, so the list never groups enemy fallen by their hidden unit.
            frame.corpses.sort_by(|a, b| {
                (a.position[0], a.position[1])
                    .partial_cmp(&(b.position[0], b.position[1]))
                    .unwrap()
            });
            frame.fallen_bodies.clear();
            frame.fallen_bodies.extend(
                self.fallen
                    .values()
                    .filter(|f| knowledge.knows_fallen(f.prop))
                    .cloned(),
            );
            // A falling airframe, to the sides that saw it go down: the
            // same sides that learn its wreck.
            frame.crashes.clear();
            frame.crashes.extend(self.crashes.iter().filter_map(|c| {
                let &(_, id) = c.knowing.iter().find(|(s, _)| *s == side)?;
                let unit = &self.units[c.unit.0 as usize];
                let (pitch, roll) = c.attitude();
                Some(FallingAirframe {
                    id,
                    own: unit.side == side,
                    kind: unit.kind,
                    position: [c.position.x, c.position.y, c.position.z],
                    yaw: c.yaw,
                    pitch,
                    roll,
                })
            }));
        }
    }

    pub fn observe(&self, side: Side) -> &ObservationFrame {
        &self.observations[side.index()]
    }

    /// The referee's match report so far, which sides see only once the
    /// match finishes: for native reports, never for a controller.
    pub fn match_report(&self) -> Option<&contract::skirmish::MatchReport> {
        self.skirmish.as_ref().map(|skirmish| &skirmish.report)
    }

    pub fn script_events(&self) -> &[contract::script::ScriptEvent] {
        &self.script_events
    }

    pub fn replay(&self) -> Replay {
        Replay {
            engine_build: ENGINE_BUILD_ID.to_owned(),
            scenario_digest: format!("{:016x}", self.scenario_digest),
            config_digest: format!("{:016x}", self.config_digest),
            seed: self.seed,
            accepted: self.accepted.clone(),
            script_victory: self.script_victory,
            script_error: self.script_error.clone(),
            script_terminal_tick: self.script_terminal_tick,
            controllers: self.controller_list(),
            controller_transitions: self.controller_transitions.clone(),
            script_events: self.script_events.clone(),
            script_trace: self.script_trace.clone(),
        }
    }
}

/// What every order a unit is given does to it first: it is no longer
/// withdrawing, and a jet's run, release cue and dogfight end, so no order
/// (repeated or not) can leave one stuck in a run.
fn take_order(unit: &mut Unit) {
    unit.withdrawing = false;
    crate::air::break_off(unit);
}

#[derive(Clone)]
struct PreparedCommand {
    command: CommandEnvelope,
    placement: Option<MovePlacement>,
    building: Option<garrison::BuildingPlan>,
    /// Where a placed unit stops short ([`crate::units::StopShort`]).
    stops: BTreeMap<UnitId, crate::units::StopShort>,
    refused: Vec<contract::command::Refusal>,
}

/// Who gave a command: a client, or the battle itself (its slot AI, authored
/// scripts and opponents), which commands a slot it controls.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Issuer {
    Client,
    Battle,
}

type Admission = (
    Option<MovePlacement>,
    Option<BuildingPlacement>,
    Vec<contract::command::Refusal>,
);

#[cfg(test)]
mod tests;

#[cfg(test)]
mod settlement_tests;

#[cfg(test)]
mod interception_tests;
