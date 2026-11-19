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
    AirTransit, AirTransitDirection, Blast, Corpse, EncounterStatus, FallenBody, Grind,
    GuidedMissile, KnownProp, MemberOrder, MoveState, ObservationFrame, OwnUnit, Posture,
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

mod accept_order;
mod aftermath;
mod air_lifecycle;
mod apply_command;
mod attack_order;
mod bodies;
mod casualties;
mod commands;
mod controllers;
mod countermeasures;
mod direct;
mod dispatch;
mod entry_routes;
mod fire;
mod flown;
mod move_placement;
mod observation;
mod order_steps;
mod purchase_preview;
mod refusals;
mod replay;
mod replay_identity;
mod return_to_base;
mod scripts;
mod seats;
mod seeing;
mod setup;
mod skirmish_turn;
mod spawn;
mod state_digest;
use crate::cover;
use crate::damage::{self, DamageContext, HullResolver};
use crate::deployment;
use crate::digest::{self, Digest};
use crate::director::DirectorRun;
use crate::encounter::{Defender, Referee};
use crate::flight::{
    self, Body, BodyId, FlightEvent, Pose, ProjectileId, Projectiles, Shape, Struck,
};
use crate::garrison;
use crate::ground::{GroundLayer, KnownGround, Wear};
use crate::hearing;
use crate::hold;
use crate::knowledge::SideKnowledge;
use crate::math::{v2, v3, Obb2, Rotation, V2, V3};
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
pub use fire::Round;
use flown::{clip_to_seen, Burst, Flown};
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

pub use replay::{ControllerTransition, Replay, ReplayError, ScriptTrace, ENGINE_BUILD_ID};
pub use seats::{SeatError, SeatEvent};

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
    /// The windows of every held building.
    windows: garrison::Windows,
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
    /// This tick's bursts, with the round's side and visibility band.
    blasts: Vec<Burst>,
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
    /// An authored scenario's director: its chapters and what has fired.
    director: Option<DirectorRun>,
    encounter: Option<EncounterStatus>,
    pending: Vec<PreparedCommand>,
    accepted: Vec<(Tick, CommandEnvelope)>,
    /// Recorded commands still to apply when this battle is a replay.
    replaying: Option<VecDeque<(Tick, CommandEnvelope)>>,
    seat_events: Vec<SeatEvent>,
    pending_seat_events: VecDeque<SeatEvent>,
    observations: [ObservationFrame; 3],
    home_ways: order_steps::HomeWays,
    refusals: [Vec<contract::command::Refusal>; 3],
    pub(crate) shouts: [ShoutLedger; 3],
    /// Each unit as barks saw it at the start of this tick.
    shout_before: Vec<shouts::Before>,
    shout_evidence: ShoutEvidence,
    shout_fired: BTreeSet<UnitId>,
    shout_kills: BTreeSet<UnitId>,
    near_rounds: shouts::NearRounds,
    /// Per listening side, the enemies that shouted unseen since the last
    /// sound bucket.
    shouted_unseen: [BTreeSet<UnitId>; 3],
    scenario_digest: u64,
    config_digest: u64,
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

impl Battle {
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

    /// The windows of every held building, for native tests.
    pub fn windows(&self) -> &garrison::Windows {
        &self.windows
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

    pub fn finished(&self) -> bool {
        self.script_terminal_tick
            .is_some_and(|tick| self.tick >= tick)
            || self
                .skirmish
                .as_ref()
                .is_some_and(|s| s.phase == contract::skirmish::Phase::Finished)
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
        self.world
            .smoke_mut()
            .advance(self.tick, self.rules.tick_hz);
        while self.events.front().is_some_and(|e| e.tick <= self.tick) {
            let event = self.events.pop_front().unwrap();
            match event.action {
                EventAction::AddProp(prop) => {
                    self.world.add_prop(&prop);
                }
                EventAction::Fire { unit } => self.record_fire(unit, 0),
                EventAction::Burst { point, weapon } => self.burst_event(point, &weapon),
            }
        }
        // An authored scenario's director, then fixture scripts: authored
        // setup, identical live and in replay.
        self.direct();
        while self.scripts.front().is_some_and(|o| o.tick <= self.tick) {
            let script = self.scripts.pop_front().unwrap();
            let _ = self.apply_authored_order(script.side, None, script.order, script.queued);
        }
        for command in std::mem::take(&mut self.pending) {
            self.apply(command);
        }
        if let Some(skirmish) = &mut self.skirmish {
            let fielded = skirmish.fielded(&self.units);
            skirmish.advance(self.tick, self.rules.tick_hz, &fielded);
        }
        self.apply_seat_events();
        self.break_dogfights();
        self.dispatch_reinforcements();
        completed(TickPhase::Orders);
        crate::carriage::advance(
            &self.world,
            &mut self.units,
            &self.windows,
            &self.rules,
            &mut self.refusals,
        );
        hold::advance(
            &self.world,
            &self.sides,
            self.authored_props,
            &mut self.units,
            &mut self.windows,
            &self.rules,
            self.seed,
            &mut self.refusals,
        );
        crate::carriage::copy_passenger_positions(&mut self.units);
        self.start_withdrawals();
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
        self.shout_fired = self.fire(&moved);
        supply::service(
            &self.world,
            &mut self.units,
            &self.windows,
            &self.arsenal,
            &self.rules,
            &moved.units,
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

    /// Downed aircraft still falling.
    pub fn crashes(&self) -> &[crate::crash::Crash] {
        &self.crashes
    }

    pub fn observe(&self, side: Side) -> &ObservationFrame {
        &self.observations[side.index()]
    }

    /// The referee's match report so far, which sides see only once the
    /// match finishes: for native reports, never for a controller.
    pub fn match_report(&self) -> Option<&contract::skirmish::MatchReport> {
        self.skirmish.as_ref().map(|skirmish| &skirmish.report)
    }

    /// An authored scenario's director: its open chapter, what has fired
    /// and what it was refused.
    pub fn director(&self) -> Option<&DirectorRun> {
        self.director.as_ref()
    }

    pub fn script_events(&self) -> &[contract::script::ScriptEvent] {
        &self.script_events
    }
}

#[derive(Clone)]
struct PreparedCommand {
    command: CommandEnvelope,
    placement: Option<MovePlacement>,
    building: Option<BuildingPlacement>,
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

#[cfg(test)]
mod projectile_visibility_tests;
