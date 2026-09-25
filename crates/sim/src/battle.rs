//! The one battle authority: commands in, fixed ticks, side observations out.
use std::collections::{BTreeMap, BTreeSet, VecDeque};

use contract::command::{CommandAck, CommandEnvelope, Engagement, Order, OrderError, TargetRef};
use contract::ids::{Side, Tick, UnitId};
use contract::map::{PropDefinition, PropKind};
use contract::observation::{
    Corpse, EncounterStatus, GuidedMissile, KnownProp, MoveState, ObservationFrame, OwnUnit,
    Posture, ServiceStatus, SoundCue, VisibilityField, VisibleSegment,
};
use contract::scenario::{
    EncounterRules, EventAction, Opponent, Rules, ScenarioDefinition, ScenarioEvent, ScriptedOrder,
    UnitCondition, UnitKind,
};
use serde::{Deserialize, Serialize};

use crate::damage::{self, DamageContext};
use crate::deployment;
use crate::digest::{self, Digest};
use crate::flight::{self, Body, BodyId, FlightEvent, Pose, ProjectileId, Projectiles, Shape};
use crate::garrison::{self, Structures};
use crate::hearing;
use crate::knowledge::SideKnowledge;
use crate::math::{v2, v3, V2, V3};
use crate::movement::{self, MovementContext, SideGeometry};
use crate::rng::Rng;
use crate::sensing;
use crate::supply;
use crate::units::{self, MoveOrder, Soldier, Unit, UnitOrder};
use crate::village::{Defender, Referee};
use crate::visibility::{self, OcclusionGrid};
use crate::weapons::{self, Arsenal, FireContext, Support, Target, VEHICLE_BODY_BASE};
use crate::world::{PropId, WorldGeometry};

/// Group offsets are compressed to fit within this radius of the goal.
const GROUP_SPREAD_M: f64 = 40.0;
/// How far a group member's destination may move to find standing room.
const DESTINATION_SNAP_M: f64 = 16.0;
/// Ticks between ground-visibility sweeps for each side (sides alternate).
/// Identification is evaluated every tick; only the fog display lags, by at
/// most this many ticks.
const FOG_INTERVAL_TICKS: u64 = 6;
/// Seed salt for each side's observation-uncertainty stream (contact placement),
/// kept apart from combat and policy randomness.
const OBSERVATION_STREAM: u64 = 0x6f62_7365_7276_6531;
/// Seed salt for combat randomness (spread, area aim points).
const COMBAT_STREAM: u64 = 0x636f_6d62_6174_2121;
/// Blast fragment sampling (slice 09).
const DAMAGE_STREAM: u64 = 0x6461_6d61_6765_2121;
/// An attack reaching its target's last reported place within this distance,
/// without regaining sight, is complete.
const PURSUIT_ARRIVAL_M: f64 = 5.0;
/// Enemy round flight is shown only over seen ground, sampled this finely.
const SEGMENT_SAMPLES: usize = 8;

/// Where a round in flight came from, for consequences and visibility.
#[derive(Clone, Copy, Debug)]
pub struct Round {
    pub weapon: usize,
    pub unit: UnitId,
    pub side: Side,
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
}

/// Everything needed to reproduce a battle in the same build: the setup
/// identity, the seed and every sequenced command from both sides.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Replay {
    pub scenario_digest: String,
    pub config_digest: String,
    pub seed: u64,
    /// `(applied_tick, command)` in acceptance order.
    pub accepted: Vec<(Tick, CommandEnvelope)>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ReplayError {
    ScenarioMismatch,
    ConfigMismatch,
}

pub struct Battle {
    world: WorldGeometry,
    rules: Rules,
    seed: u64,
    tick: Tick,
    units: Vec<Unit>,
    /// Props authored with the map (ids below this) are known to every side.
    authored_props: PropId,
    sides: [SideGeometry; 2],
    events: VecDeque<ScenarioEvent>,
    scripts: VecDeque<ScriptedOrder>,
    knowledge: [SideKnowledge; 2],
    occlusion: OcclusionGrid,
    fog: [VisibilityField; 2],
    /// Units that fired during the current sound bucket.
    fired: BTreeSet<UnitId>,
    audible: [Vec<SoundCue>; 2],
    arsenal: Arsenal,
    /// Building health and the ruins collapses left (slice 11).
    structures: Structures,
    projectiles: Projectiles,
    rounds: BTreeMap<ProjectileId, Round>,
    combat_rng: Rng,
    damage_rng: Rng,
    /// The last soldier id issued; replacements continue from it (L03).
    last_soldier: u32,

    /// This tick's flight events, in order.
    flight_events: Vec<FlightEvent>,
    /// This tick's round flight: shooter side, start, end.
    /// This tick's flown stretch per round: (firing side, from, to, ended in an impact).
    segments: Vec<(Side, V3, V3, bool)>,
    next_seq: [u64; 2],
    /// The observation-bound opponent and its memory (off while replaying).
    opponent: Option<(Opponent, Defender)>,
    /// The fixture's completion referee and its latest verdict.
    referee: Option<(EncounterRules, Referee)>,
    encounter: Option<EncounterStatus>,
    pending: Vec<CommandEnvelope>,
    accepted: Vec<(Tick, CommandEnvelope)>,
    /// Recorded commands still to apply when this battle is a replay.
    replaying: Option<VecDeque<(Tick, CommandEnvelope)>>,
    observations: [ObservationFrame; 2],
    scenario_digest: u64,
    config_digest: u64,
}

/// The parts of a round's flight over ground `fog` shows as seen.
fn clip_to_seen(fog: &VisibilityField, a: V3, b: V3, impact: bool) -> Vec<VisibleSegment> {
    // Runs of seen samples, each ending at its last seen sample: a drawn
    // stretch never reaches over unseen ground. An impact shows only if the
    // struck point itself is seen.
    let mut out = Vec::new();
    let mut run: Option<(V3, V3)> = None;
    for k in 0..=SEGMENT_SAMPLES {
        let p = a + (b - a) * (k as f64 / SEGMENT_SAMPLES as f64);
        let seen = fog.visible(p.x, p.y);
        if seen {
            run = Some((run.map_or(p, |(start, _)| start), p));
            if k < SEGMENT_SAMPLES {
                continue;
            }
        }
        if let Some((from, to)) = run.take() {
            if from != to {
                out.push(VisibleSegment {
                    from: [from.x, from.y, from.z],
                    to: [to.x, to.y, to.z],
                    own: false,
                    impact: impact && seen && k == SEGMENT_SAMPLES,
                });
            }
        }
    }
    out
}

/// Whether any fog cell under a prop's footprint is seen: large remains
/// (a ruin) are learned when any of them is in view, not only their middle.
fn footprint_seen(field: &VisibilityField, prop: &crate::world::Prop) -> bool {
    let step = field.cell_m / 2.0;
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
            let p = prop.center + local.rotated(prop.yaw);
            field.visible(p.x, p.y)
        })
    })
}

/// Authored starting wear: vehicle health, fallen soldiers (their records lie
/// in formation where the squad starts) and spent rounds.
fn wear(unit: &mut Unit, c: &UnitCondition, arsenal: &Arsenal, rules: &Rules) {
    if let (Some(hp), Some(_)) = (c.hp, unit.hull) {
        unit.hp = hp.clamp(1.0, units::max_hp(unit.kind, rules));
    }
    let n = unit.members.len();
    for k in n.saturating_sub(c.casualties as usize)..n {
        let at = unit.member_position(k);
        unit.members[k].fall(at);
    }
    let specs = arsenal.specs(unit.kind);
    for mount in &mut unit.mounts {
        for (k, &row) in specs[mount.spec].kinds.iter().enumerate() {
            if let (Some(spent), Some(n)) = (
                c.spent.get(&arsenal.weapons[row].name),
                mount.ammo[k].as_mut(),
            ) {
                *n = n.saturating_sub(*spent);
            }
        }
        if mount.loaded.is_some_and(|k| mount.ammo[k] == Some(0)) {
            mount.loaded = None;
        }
    }
}

/// Optional values carry a presence tag, so different states never hash alike.
fn digest_v2(d: &mut Digest, p: Option<V2>) {
    d.u64(p.is_some() as u64);
    if let Some(p) = p {
        d.f64(p.x).f64(p.y);
    }
}

fn digest_target(d: &mut Digest, t: Target) {
    match t {
        Target::Unit(u) => d.u64(0).u64(u.0 as u64),
        Target::Contact(c) => d.u64(1).u64(c.0 as u64),
        Target::Ground(p) => d.u64(2).f64(p.x).f64(p.y).f64(p.z),
    };
}

fn scenario_digest(setup: &ScenarioDefinition) -> u64 {
    let map = serde_json::to_string(&setup.map).expect("map serializes");
    let units = serde_json::to_string(&setup.units).expect("units serialize");
    let events = serde_json::to_string(&setup.events).expect("events serialize");
    let scripts = serde_json::to_string(&setup.scripts).expect("scripts serialize");
    // The opponent's policy and the encounter's rules change what a replay's
    // commands mean, so a replay is pinned to them too.
    let opponent = serde_json::to_string(&setup.opponent).expect("opponent serializes");
    let encounter = serde_json::to_string(&setup.encounter).expect("encounter serializes");
    digest::of_str(&(map + &units + &events + &scripts + &opponent + &encounter))
}

fn config_digest(setup: &ScenarioDefinition) -> u64 {
    digest::of_str(&serde_json::to_string(&setup.rules).expect("rules serialize"))
}

impl Battle {
    pub fn new(setup: &ScenarioDefinition, seed: u64) -> Self {
        let world = WorldGeometry::new(&setup.map);
        let rules = setup.rules.clone();
        let arsenal = Arsenal::new(&rules);
        supply::validate(&arsenal, &rules);
        sensing::validate(&rules.sensors);
        let mut soldier_ids = 0u32;
        let mut units = setup
            .units
            .iter()
            .enumerate()
            .map(|(i, u)| {
                let xy = v2(u.position[0], u.position[1]);
                let members = units::squad_offsets(units::squad_size(u.kind, &rules))
                    .into_iter()
                    .map(|offset| {
                        soldier_ids += 1;
                        Soldier {
                            id: soldier_ids,
                            offset,
                            formation: offset,
                            hp: rules.health.soldier,
                            corpse: None,
                        }
                    })
                    .collect();
                Unit {
                    id: UnitId(i as u32),
                    side: u.side,
                    kind: u.kind,
                    position: xy.with_z(world.surface_at(xy.x, xy.y).map_or(0.0, |s| s.z)),
                    yaw: u.yaw,
                    mobility: units::mobility(u.kind, &rules),
                    hull: units::hull(u.kind, &rules),
                    members,
                    orders: VecDeque::new(),
                    route: None,
                    state: MoveState::Idle,
                    blocker: None,
                    planned_revision: 0,
                    progress: (f64::INFINITY, 0),
                    pursuit: None,
                    planned_goal: None,
                    engagement: u.engagement.unwrap_or(Engagement::FireAtWill),
                    mounts: arsenal.mounts_for(u.kind, u.yaw),
                    attackers: BTreeSet::new(),
                    reach: Default::default(),
                    hp: units::max_hp(u.kind, &rules),
                    suppression: 0.0,
                    suppressed_at: 0,
                    deployment: deployment::initial(u.kind, &rules),
                    garrison: None,
                    stock: (u.kind == UnitKind::Supply)
                        .then(|| u.stock.unwrap_or(rules.service.stock)),
                    progress_service: Default::default(),
                    service: ServiceStatus::OutOfRange,
                }
            })
            .collect::<Vec<Unit>>();
        for (unit, setup) in units.iter_mut().zip(&setup.units) {
            if let Some(c) = &setup.condition {
                wear(unit, c, &arsenal, &rules);
            }
        }
        let mut events: Vec<ScenarioEvent> = setup.events.clone();
        events.sort_by_key(|e| e.tick);
        let mut scripts: Vec<ScriptedOrder> = setup.scripts.clone();
        scripts.sort_by_key(|o| o.tick);
        let occlusion = OcclusionGrid::new(&world, rules.sensors.fog_cell_m);
        let structures = Structures::new(&world, &rules);
        let mut battle = Battle {
            authored_props: world.props().count() as PropId,
            world,
            rules,
            seed,
            tick: 0,
            units,
            sides: Default::default(),
            events: events.into(),
            scripts: scripts.into(),
            knowledge: [
                SideKnowledge::new(seed ^ OBSERVATION_STREAM),
                SideKnowledge::new(seed ^ OBSERVATION_STREAM ^ 1),
            ],
            fired: BTreeSet::new(),
            audible: Default::default(),
            projectiles: Projectiles::new(arsenal.config.clone()),
            arsenal,
            structures,
            rounds: BTreeMap::new(),
            combat_rng: Rng::new(seed ^ COMBAT_STREAM),
            damage_rng: Rng::new(seed ^ DAMAGE_STREAM),
            last_soldier: soldier_ids,

            flight_events: Vec::new(),
            segments: Vec::new(),
            fog: [occlusion.field(), occlusion.field()],
            occlusion,
            next_seq: [1, 1],
            opponent: setup.opponent.clone().map(|o| (o, Defender::default())),
            referee: setup.encounter.clone().map(|e| (e, Referee::default())),
            encounter: None,
            pending: Vec::new(),
            accepted: Vec::new(),
            replaying: None,
            observations: Default::default(),
            scenario_digest: scenario_digest(setup),
            config_digest: config_digest(setup),
        };
        for side in Side::ALL {
            battle.sense(side);
            battle.sweep_fog(side);
        }
        battle.observe_all();
        battle
    }

    /// A battle that re-applies `replay`'s commands at their recorded ticks.
    /// Player input is refused for its whole life.
    pub fn from_replay(setup: &ScenarioDefinition, replay: &Replay) -> Result<Self, ReplayError> {
        if format!("{:016x}", scenario_digest(setup)) != replay.scenario_digest {
            return Err(ReplayError::ScenarioMismatch);
        }
        if format!("{:016x}", config_digest(setup)) != replay.config_digest {
            return Err(ReplayError::ConfigMismatch);
        }
        let mut battle = Battle::new(setup, replay.seed);
        battle.replaying = Some(replay.accepted.iter().cloned().collect());
        Ok(battle)
    }

    pub fn tick(&self) -> Tick {
        self.tick
    }

    pub fn world(&self) -> &WorldGeometry {
        &self.world
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// This tick's flight events (consequences are applied by later slices).
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
                .filter(|p| p.kind == PropKind::Wreck)
                .count(),
            active_projectiles: self.projectiles.active().len(),
            rounds_launched: self.projectiles.launched(),
            path_searches: Side::ALL.into_iter().map(|s| self.route_searches(s)).sum(),
        }
    }

    /// A unit as the authority holds it (for native tests and later owners).
    pub fn unit(&self, id: UnitId) -> Option<&Unit> {
        self.units.get(id.0 as usize)
    }

    pub fn arsenal(&self) -> &Arsenal {
        &self.arsenal
    }

    /// Building health and ruins, for native tests and later owners.
    pub fn structures(&self) -> &Structures {
        &self.structures
    }

    /// Route searches run so far for `side`.
    pub fn route_searches(&self, side: Side) -> u64 {
        self.sides[side.index()].searches
    }

    /// Validate and schedule a command for the next step. Every command whose
    /// sequence number is next for its side is recorded for replay, including
    /// ones rejected for their content, so a replay reproduces the same acks.
    pub fn accept(&mut self, command: CommandEnvelope) -> CommandAck {
        let applied_tick = self.tick + 1;
        let ack = |error| CommandAck {
            seq: command.seq,
            applied_tick,
            error,
        };
        if self.replaying.is_some() {
            return ack(Some(OrderError::ReplayInProgress));
        }
        self.admit(command.clone(), applied_tick)
            .map_or_else(|e| ack(Some(e)), |_| ack(None))
    }

    fn admit(&mut self, command: CommandEnvelope, applied_tick: Tick) -> Result<(), OrderError> {
        let side = command.side.index();
        if command.seq != self.next_seq[side] {
            return Err(OrderError::OutOfSequence {
                expected: self.next_seq[side],
            });
        }
        self.next_seq[side] += 1;
        self.accepted.push((applied_tick, command.clone()));
        self.validate(&command)?;
        self.pending.push(command);
        Ok(())
    }

    /// A side-scoped target reference as the sim's own target, if the side
    /// holds it right now.
    fn resolve_target(&self, side: Side, target: &TargetRef) -> Option<Target> {
        let knowledge = &self.knowledge[side.index()];
        match *target {
            TargetRef::Identified { id } => knowledge.unit_for(id).map(Target::Unit),
            TargetRef::Contact { id } => knowledge.contact(id).map(|c| Target::Contact(c.id)),
            // The aim point sits on the public terrain, whatever height the client sent.
            TargetRef::Ground { point } => self
                .world
                .height_at(point[0], point[1])
                .map(|z| Target::Ground(v3(point[0], point[1], z))),
        }
    }

    fn validate(&self, command: &CommandEnvelope) -> Result<(), OrderError> {
        let units = match &command.order {
            Order::Move { units, goal, .. } | Order::AttackMove { units, goal, .. } => {
                if self.world.height_at(goal[0], goal[1]).is_none() {
                    return Err(OrderError::OutOfBounds);
                }
                units
            }
            Order::Attack { units, target } => {
                if self.resolve_target(command.side, target).is_none() {
                    return Err(OrderError::UnknownTarget);
                }
                units
            }
            Order::Garrison { units, building } => {
                self.validate_units(command.side, units)?;
                return garrison::validate(
                    &self.world,
                    &self.structures,
                    &self.units,
                    command.side,
                    units,
                    *building,
                    &self.rules,
                );
            }
            Order::Stop { units }
            | Order::SetEngagement { units, .. }
            | Order::SetDeployment { units, .. }
            | Order::ExitBuilding { units } => units,
            // Gesture tokens are scoped by side at application; nothing else to check.
            Order::UpgradeMove { .. } => return Ok(()),
        };
        self.validate_units(command.side, units)
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
                Some(_) => {}
            }
        }
        Ok(())
    }

    /// Advance one fixed tick: authored events, scheduled commands, movement, observation.
    pub fn step(&mut self) -> Tick {
        self.tick += 1;
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
        while self.events.front().is_some_and(|e| e.tick <= self.tick) {
            let event = self.events.pop_front().unwrap();
            match event.action {
                EventAction::AddProp(prop) => {
                    self.world.add_prop(&prop);
                }
                EventAction::Fire { unit } => self.record_fire(unit),
            }
        }
        // Fixture scripts are authored setup, identical live and in replay.
        while self.scripts.front().is_some_and(|o| o.tick <= self.tick) {
            let script = self.scripts.pop_front().unwrap();
            let command = CommandEnvelope {
                side: script.side,
                seq: 0,
                order: script.order,
                queued: script.queued,
            };
            if self.validate(&command).is_ok() {
                self.pending.push(command);
            }
        }
        for command in std::mem::take(&mut self.pending) {
            self.apply(command);
        }
        garrison::advance(&self.world, &self.structures, &mut self.units, &self.rules);
        self.update_pursuit();
        damage::recover(&mut self.units, &self.rules, self.tick);
        // Progress moves before movement, so the gate opens on the tick packing completes.
        deployment::advance_all(&mut self.units);
        let before = self.poses();
        let ctx = MovementContext {
            world: &self.world,
            authored: self.authored_props,
            tick: self.tick,
            tick_hz: self.rules.tick_hz,
            vehicle_turn_deg_s: self.rules.movement.vehicle_turn_deg_s,
            suppression_move_penalty: self.rules.suppression.max_move_penalty,
        };
        movement::advance(&ctx, &mut self.units, &mut self.sides);
        let after = self.poses();
        let moved: Vec<bool> = before
            .iter()
            .zip(&after)
            .map(|(a, b)| (a.base - b.base).length() > 1e-9)
            .collect();
        self.guide(&moved);
        self.fly(&before, &after);
        for side in Side::ALL {
            self.sense(side);
            if self.tick % FOG_INTERVAL_TICKS == side.index() as u64 * FOG_INTERVAL_TICKS / 2 {
                self.sweep_fog(side);
            }
        }
        self.prune_attackers();
        let fired = self.fire(&moved);
        supply::service(
            &mut self.units,
            &self.arsenal,
            &self.rules,
            &moved,
            &fired,
            &mut self.last_soldier,
        );
        let bucket = (self.rules.sensors.sound_bucket_s * self.rules.tick_hz as f64).round() as u64;
        for side in Side::ALL {
            self.audible[side.index()] = if self.tick.is_multiple_of(bucket) {
                hearing::hear(
                    side,
                    self.tick,
                    &self.units,
                    &self.knowledge[side.index()],
                    &self.fired,
                    &self.rules,
                )
            } else {
                Vec::new()
            };
        }
        if self.tick.is_multiple_of(bucket) {
            self.fired.clear();
        }
        if let Some((rules, referee)) = self.referee.as_mut() {
            self.encounter = Some(referee.judge(rules, &self.units, self.tick, self.rules.tick_hz));
        }
        self.observe_all();
        self.opponent_turn();
        self.tick
    }

    /// The opponent reads its side's fresh observation and commands through
    /// the same path as a player; its commands are recorded, so a replay
    /// replays them and never reruns it. Its memory is outside the digest:
    /// continuing live play from a replay or a snapshot would need it.
    fn opponent_turn(&mut self) {
        if self.replaying.is_some() {
            return;
        }
        let Some((op, defender)) = self.opponent.as_mut() else {
            return;
        };
        let side = op.side;
        let orders = defender.decide(op, &self.observations[side.index()], &self.rules);
        for order in orders {
            let seq = self.next_seq[side.index()];
            let _ = self.accept(CommandEnvelope {
                side,
                seq,
                order,
                queued: false,
            });
        }
    }

    /// `unit` fired: every opposing side learns an uncertain firing area and
    /// may hear the shot. Weapons and the lab emitter share this seam.
    fn record_fire(&mut self, unit: UnitId) {
        let Some(shooter) = self.units.get(unit.0 as usize) else {
            return;
        };
        let (side, at) = (shooter.side, shooter.position.xy());
        for other in Side::ALL.into_iter().filter(|s| *s != side) {
            self.knowledge[other.index()].note_fire(unit, at);
        }
        self.fired.insert(unit);
    }

    /// Every unit's pose (ground contact centre and heading).
    fn poses(&self) -> Vec<Pose> {
        self.units
            .iter()
            .map(|u| Pose {
                base: u.position,
                yaw: u.yaw,
            })
            .collect()
    }

    /// Colliders for this tick: a capsule per living soldier, a box per vehicle.
    fn bodies(&self, before: &[Pose], after: &[Pose]) -> Vec<Body> {
        let b = &self.rules.bodies;
        let mut bodies = Vec::new();
        for (i, unit) in self.units.iter().enumerate().filter(|(_, u)| u.alive()) {
            match unit.hull {
                Some(half) => bodies.push(Body {
                    id: BodyId(VEHICLE_BODY_BASE + unit.id.0),
                    unit: unit.id,
                    shape: Shape::Box { half },
                    from: before[i],
                    to: after[i],
                }),
                None => {
                    for (k, s) in unit.members.iter().enumerate().filter(|(_, s)| s.alive()) {
                        // A garrisoned soldier's capsule stands at its slot.
                        let at = |p: &Pose| Pose {
                            base: match unit.garrisoned() {
                                true => unit.member_position(k),
                                false => (p.base.xy() + s.offset.rotated(p.yaw)).with_z(p.base.z),
                            },
                            yaw: p.yaw,
                        };
                        bodies.push(Body {
                            id: BodyId(s.id),
                            unit: unit.id,
                            shape: Shape::Capsule {
                                radius: b.soldier_radius_m,
                                height: b.soldier_height_m,
                            },
                            from: at(&before[i]),
                            to: at(&after[i]),
                        });
                    }
                }
            }
        }
        bodies
    }

    /// Fly every round one tick against the world and this tick's bodies,
    /// keeping each round's flown segment for visibility.
    fn fly(&mut self, before: &[Pose], after: &[Pose]) {
        let bodies = self.bodies(before, after);
        let starts: BTreeMap<ProjectileId, V3> = self
            .projectiles
            .active()
            .iter()
            .map(|p| (p.id, p.position))
            .collect();
        self.flight_events.clear();
        flight::advance_projectiles(
            &mut self.projectiles,
            &self.world,
            &bodies,
            &mut self.flight_events,
        );
        let mut ends: BTreeMap<ProjectileId, V3> = self
            .projectiles
            .active()
            .iter()
            .map(|p| (p.id, p.position))
            .collect();
        let mut struck = BTreeSet::new();
        for e in &self.flight_events {
            match e {
                FlightEvent::Impact(i) => {
                    ends.insert(i.projectile, i.point);
                    struck.insert(i.projectile);
                }
                FlightEvent::Expired(x) => {
                    ends.insert(x.projectile, x.point);
                }
                FlightEvent::NearMiss(_) => {}
            }
        }
        self.segments.clear();
        for (id, start) in starts {
            if let (Some(end), Some(round)) = (ends.get(&id), self.rounds.get(&id)) {
                self.segments
                    .push((round.side, start, *end, struck.contains(&id)));
            }
        }
        let ctx = DamageContext {
            world: &self.world,
            arsenal: &self.arsenal,
            rules: &self.rules,
            tick: self.tick,
        };
        let outcome = damage::resolve(
            &ctx,
            &self.flight_events,
            &self.rounds,
            &mut self.units,
            &mut self.damage_rng,
        );
        self.consequences(outcome);
        let live: BTreeSet<ProjectileId> = self.projectiles.active().iter().map(|p| p.id).collect();
        self.rounds.retain(|id, _| live.contains(id));
    }

    /// Hostile damage or suppression grants return fire; a destroyed vehicle
    /// leaves a permanent wreck (M06, M07); a death a side was watching ends
    /// its track, while an unseen death discloses nothing.
    fn consequences(&mut self, outcome: damage::Outcome) {
        for (victim, shooter) in outcome.attacked {
            self.units[victim.0 as usize].attackers.insert(shooter);
        }
        let mut destroyed = outcome.destroyed;
        // Structural damage in event order; a building collapses once (L10).
        for (prop, amount) in outcome.structural {
            if self.structures.damage(prop, amount) {
                destroyed.extend(garrison::collapse(
                    &mut self.world,
                    &mut self.structures,
                    &mut self.units,
                    prop,
                    &self.rules,
                    &mut self.damage_rng,
                    self.tick,
                ));
            }
        }
        for id in destroyed {
            let unit = &self.units[id.0 as usize];
            if let Some(half) = unit.hull {
                self.world.add_prop(&PropDefinition {
                    kind: PropKind::Wreck,
                    center: [unit.position.x, unit.position.y],
                    yaw: unit.yaw,
                    half_extents: [half.x, half.y, half.z],
                    base_z: Some(unit.position.z),
                });
            }
            for side in Side::ALL.into_iter().filter(|&s| s != unit.side) {
                let knowledge = &mut self.knowledge[side.index()];
                if knowledge.identifies(id, self.tick - 1) {
                    knowledge.saw_destroyed(id);
                }
            }
        }
    }

    /// An attack pursues an identified target it cannot fire on from here, or
    /// its last reported place once identification lapses (W17); it ends on
    /// arriving there unseen, or when its area expires.
    fn update_pursuit(&mut self) {
        for unit in &mut self.units {
            unit.pursuit = None;
            // A garrisoned attack fires from the building; it never walks out.
            if unit.garrisoned() {
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
                            unit.pursuit = Some(track.position.xy());
                        }
                        false
                    }
                    None => match *last_known {
                        Some(p) if (unit.position.xy() - p).length() > PURSUIT_ARRIVAL_M => {
                            unit.pursuit = Some(p);
                            false
                        }
                        _ => true,
                    },
                },
                Target::Contact(c) => knowledge.contact(c).is_none(),
                Target::Ground(_) => false,
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
                knowledge.track(a).is_some()
                    || knowledge.all_contacts().iter().any(|c| c.emitter == a)
            });
        }
    }

    /// Guided missiles (P05, P06): a launcher supports its missile while it
    /// stands still, lives and identifies the target with its own sensors, and
    /// steers it at the target's observed position. Losing any of these (or a
    /// Stop, which drops the mount's support) releases it at once and for good:
    /// the missile keeps flying to the last point, fixed on the ground beneath.
    fn guide(&mut self, moved: &[bool]) {
        let mut supported = BTreeSet::new();
        let alive: Vec<bool> = self.units.iter().map(|u| u.alive()).collect();
        // Where a target is aimed: a hull's centre height, else a soldier's middle.
        let aim_z: Vec<f64> = self
            .units
            .iter()
            .map(|u| u.hull.map_or(weapons::SOLDIER_AIM_M, |h| h.z))
            .collect();
        for (i, unit) in self.units.iter_mut().enumerate() {
            let knowledge = &self.knowledge[unit.side.index()];
            for mount in &mut unit.mounts {
                let Some(s) = mount.support else { continue };
                let target = match s.target {
                    Target::Unit(t) => Some(t),
                    _ => None,
                };
                let sighting = target
                    .filter(|&t| alive[t.0 as usize] && knowledge.own_sees(unit.id, t))
                    .and_then(|t| knowledge.track(t).map(|tr| (t, tr.position)));
                // Entering or leaving a building is a transition that releases too.
                let settled = unit
                    .garrison
                    .as_ref()
                    .is_none_or(|g| matches!(g.phase, garrison::Phase::Inside));
                let keep = !moved[i]
                    && alive[i]
                    && settled
                    && self.projectiles.get(s.projectile).is_some();
                match sighting.filter(|_| keep) {
                    Some((t, at)) => {
                        self.projectiles
                            .steer(s.projectile, at + v3(0.0, 0.0, aim_z[t.0 as usize]));
                        supported.insert(s.projectile);
                    }
                    None => mount.support = None,
                }
            }
        }
        let released: Vec<(ProjectileId, V3)> = self
            .projectiles
            .active()
            .iter()
            .filter_map(|p| p.guidance.filter(|g| g.supported).map(|g| (p.id, g.point)))
            .filter(|(id, _)| !supported.contains(id))
            .collect();
        for (id, point) in released {
            let ground = self.world.height_at(point.x, point.y).unwrap_or(point.z);
            self.projectiles.release(id, point.xy().with_z(ground));
        }
    }

    /// Every mount acts; shots become rounds in flight and firing evidence.
    /// Returns the units that launched this tick.
    fn fire(&mut self, moved: &[bool]) -> BTreeSet<UnitId> {
        let ctx = FireContext {
            world: &self.world,
            arsenal: &self.arsenal,
            rules: &self.rules,
            tick: self.tick,
            knowledge: &self.knowledge,
        };
        let aims = weapons::garrison_aims(&ctx, &self.units);
        garrison::allocate_slots(&mut self.units, &aims, &self.rules);
        let shots = weapons::advance(&ctx, &mut self.units, moved, &mut self.combat_rng);
        let fired = shots.iter().map(|s| s.unit).collect();
        for shot in shots {
            let side = self.units[shot.unit.0 as usize].side;
            for launch in shot.launches {
                let guided = launch.guidance.is_some();
                let id = self.projectiles.launch(launch);
                self.rounds.insert(
                    id,
                    Round {
                        weapon: shot.weapon,
                        unit: shot.unit,
                        side,
                    },
                );
                // A guided round is supported by the mount that launched it.
                if guided {
                    self.units[shot.unit.0 as usize].mounts[shot.mount].support = Some(Support {
                        projectile: id,
                        target: shot.target,
                    });
                }
            }
            // The attacked unit may answer this attacker under Return fire only.
            if let Target::Unit(t) = shot.target {
                self.units[t.0 as usize].attackers.insert(shot.unit);
            }
            self.record_fire(shot.unit);
        }
        fired
    }

    /// This side's own sensors, folded into its knowledge.
    fn sense(&mut self, side: Side) {
        let sightings = sensing::evaluate(&self.world, &self.units, &self.rules, side);
        self.knowledge[side.index()].update(self.tick, &sightings, &self.units, &self.rules);
    }

    /// Recompute what ground this side sees, and learn any new obstacle in view.
    fn sweep_fog(&mut self, side: Side) {
        self.occlusion.refresh(&self.world);
        let mut field = self.occlusion.field();
        for unit in self.units.iter().filter(|u| u.side == side && u.alive()) {
            let range = sensing::ground_range(unit.kind, &self.rules.sensors);
            for eye in sensing::eyes(unit, &self.rules) {
                visibility::sweep(
                    &self.world,
                    &self.occlusion,
                    &self.rules.sensors,
                    eye,
                    range,
                    &mut field,
                );
            }
        }
        // Enemy fallen in view are remembered.
        let knowledge = &mut self.knowledge[side.index()];
        for u in self.units.iter().filter(|u| u.side != side) {
            for s in &u.members {
                if s.corpse.is_some_and(|p| field.visible(p.x, p.y)) {
                    knowledge.note_corpse(s.id);
                }
            }
        }
        let known = &mut self.sides[side.index()];
        for prop in self.world.props().filter(|p| p.id >= self.authored_props) {
            if footprint_seen(&field, prop) {
                known.learn(prop.id);
            }
        }
        self.fog[side.index()] = field;
    }

    fn apply(&mut self, command: CommandEnvelope) {
        let side = command.side;
        let queued = command.queued;
        let push = |unit: &mut Unit, order: UnitOrder| {
            // A new order ends any explicit Pack: once stationary again, set up.
            if let Some(d) = unit.deployment.as_mut() {
                d.stationary = Posture::Deployed;
            }
            if !queued {
                unit.orders.clear();
                unit.route = None;
                unit.planned_goal = None;
                unit.state = MoveState::Idle;
            }
            unit.orders.push_back(order);
        };
        match command.order {
            Order::Move {
                units,
                gesture,
                goal,
                route,
            } => {
                let destinations = self.group_destinations(side, &units, v2(goal[0], goal[1]));
                for (id, destination) in units.into_iter().zip(destinations) {
                    let order = UnitOrder::Move(MoveOrder {
                        destination,
                        policy: route,
                        gesture,
                    });
                    push(&mut self.units[id.0 as usize], order);
                }
            }
            Order::AttackMove {
                units,
                gesture,
                goal,
            } => {
                let destinations = self.group_destinations(side, &units, v2(goal[0], goal[1]));
                for (id, destination) in units.into_iter().zip(destinations) {
                    let unit = &mut self.units[id.0 as usize];
                    unit.engagement = Engagement::FireAtWill; // W14
                    let order = UnitOrder::AttackMove(MoveOrder {
                        destination,
                        policy: contract::command::RoutePolicy::Shortest,
                        gesture,
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
                    let unit = &mut self.units[id.0 as usize];
                    unit.engagement = Engagement::FireAtWill; // W14
                    push(
                        unit,
                        UnitOrder::Attack {
                            target,
                            last_known: None,
                        },
                    );
                }
            }
            Order::Garrison { units, building } => {
                for id in units {
                    let unit = &mut self.units[id.0 as usize];
                    let from = unit.position.xy();
                    let Some(approach) =
                        garrison::approach(&self.world, building, from, &self.rules)
                    else {
                        continue;
                    };
                    push(unit, UnitOrder::Garrison { building, approach });
                }
            }
            Order::ExitBuilding { units } => {
                for id in units {
                    push(&mut self.units[id.0 as usize], UnitOrder::Exit);
                }
            }
            Order::SetEngagement { units, policy } => {
                for id in units {
                    self.units[id.0 as usize].engagement = policy;
                }
            }
            Order::Stop { units } => {
                // W15: clear the queue and cancel movement, aim, reload and
                // guidance once; policy stays; automatic fire may restart.
                for id in units {
                    let unit = &mut self.units[id.0 as usize];
                    unit.orders.clear();
                    unit.route = None;
                    unit.planned_goal = None;
                    unit.pursuit = None;
                    unit.state = MoveState::Idle;
                    unit.blocker = None;
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
                    let unit = &mut self.units[id.0 as usize];
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
                for unit in self.units.iter_mut().filter(|u| u.side == side) {
                    for (k, order) in unit.orders.iter_mut().enumerate() {
                        if let Some(m) = order.movement_mut() {
                            if m.gesture == gesture && m.policy != route {
                                m.policy = route;
                                if k == 0 {
                                    unit.route = None; // replan the active order
                                    unit.planned_goal = None;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Each unit keeps its place relative to the group where space permits:
    /// offsets from the group centre, compressed to a bounded spread and snapped
    /// to standing room on the side's known map.
    fn group_destinations(&mut self, side: Side, ids: &[UnitId], goal: V2) -> Vec<V2> {
        let positions: Vec<V2> = ids
            .iter()
            .map(|id| self.units[id.0 as usize].position.xy())
            .collect();
        let centre =
            positions.iter().fold(v2(0.0, 0.0), |a, &p| a + p) * (1.0 / positions.len() as f64);
        let spread = positions
            .iter()
            .map(|&p| (p - centre).length())
            .fold(0.0, f64::max);
        let scale = if spread > GROUP_SPREAD_M {
            GROUP_SPREAD_M / spread
        } else {
            1.0
        };
        let grid = self.sides[side.index()].grid(&self.world, self.authored_props);
        ids.iter()
            .zip(&positions)
            .map(|(id, &p)| {
                let wanted = goal + (p - centre) * scale;
                let mobility = self.units[id.0 as usize].mobility;
                grid.snap(wanted, &mobility, DESTINATION_SNAP_M)
                    .unwrap_or(goal)
            })
            .collect()
    }

    fn observe_all(&mut self) {
        for side in Side::ALL {
            let knowledge = &self.knowledge[side.index()];
            let fog = &self.fog[side.index()];
            let frame = &mut self.observations[side.index()];
            frame.tick = self.tick;
            frame.identified.clear();
            frame
                .identified
                .extend(knowledge.identified(self.tick, &self.units, &self.rules));
            frame.ground_visibility.clone_from(fog);
            frame.contacts.clear();
            frame.contacts.extend(knowledge.contacts(&self.rules));
            frame.audible.clone_from(&self.audible[side.index()]);
            frame.known_props.clear();
            frame
                .known_props
                .extend(
                    self.sides[side.index()]
                        .known_dynamic
                        .iter()
                        .filter_map(|&id| {
                            self.world.prop(id).map(|p| KnownProp {
                                kind: p.kind,
                                center: [p.center.x, p.center.y],
                                yaw: p.yaw,
                                half_extents: [p.half.x, p.half.y, p.half.z],
                                base_z: p.base_z,
                                replaces: self.structures.replaced_by(p.id),
                            })
                        }),
                );
            frame.projectiles.clear();
            for &(shooter, a, b, impact) in &self.segments {
                if shooter == side {
                    frame.projectiles.push(VisibleSegment {
                        from: [a.x, a.y, a.z],
                        to: [b.x, b.y, b.z],
                        own: true,
                        impact,
                    });
                } else {
                    frame.projectiles.extend(clip_to_seen(fog, a, b, impact));
                }
            }
            let target_ref = |t: Target| -> Option<TargetRef> {
                match t {
                    Target::Unit(u) => knowledge
                        .track(u)
                        .map(|tr| TargetRef::Identified { id: tr.id }),
                    Target::Contact(c) => Some(TargetRef::Contact { id: c }),
                    Target::Ground(p) => Some(TargetRef::Ground {
                        point: [p.x, p.y, p.z],
                    }),
                }
            };
            frame.own.clear();
            frame.own.extend(
                self.units
                    .iter()
                    .filter(|u| u.side == side && u.alive())
                    .map(|u| OwnUnit {
                        id: u.id,
                        kind: u.kind,
                        position: [u.position.x, u.position.y, u.position.z],
                        yaw: u.yaw,
                        goal: u.movement_goal().map(|(g, _)| [g.x, g.y]),
                        policy: u.movement_goal().map(|(_, p)| p),
                        state: u.state,
                        blocker: u.blocker,
                        route: u.route.iter().flatten().map(|p| [p.x, p.y]).collect(),
                        queue: u
                            .orders
                            .iter()
                            .skip(1)
                            .filter_map(|o| {
                                o.movement().map(|m| [m.destination.x, m.destination.y])
                            })
                            .collect(),
                        members: u.member_positions().map(|p| [p.x, p.y, p.z]).collect(),
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
                        deployment: deployment::state(u),
                        hp: u.hp,
                        member_hp: u
                            .members
                            .iter()
                            .filter(|s| s.alive())
                            .map(|s| s.hp)
                            .collect(),
                        suppression: u.suppression,
                        stock: u.stock,
                        service: u.service,
                        garrison: garrison::state(u, &self.rules),
                    }),
            );
            frame.encounter = self.encounter;
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
            frame.corpses.clear();
            for u in &self.units {
                for s in &u.members {
                    let own = u.side == side;
                    if let (Some(p), true) = (s.corpse, own || knowledge.knows_corpse(s.id)) {
                        frame.corpses.push(Corpse {
                            position: [p.x, p.y, p.z],
                            own,
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
        }
    }

    pub fn observe(&self, side: Side) -> &ObservationFrame {
        &self.observations[side.index()]
    }

    /// Digest of the complete authoritative state at the current tick.
    pub fn digest(&self) -> u64 {
        let mut d = Digest::default();
        d.u64(self.tick);
        for u in &self.units {
            d.u64(u.id.0 as u64)
                .f64(u.position.x)
                .f64(u.position.y)
                .f64(u.position.z)
                .f64(u.yaw);
            d.u64(u.state as u64);
            d.u64(u.orders.len() as u64);
            for o in &u.orders {
                match o {
                    UnitOrder::Move(m) | UnitOrder::AttackMove(m) => {
                        d.u64(matches!(o, UnitOrder::Move(_)) as u64);
                        d.f64(m.destination.x)
                            .f64(m.destination.y)
                            .u64(m.policy as u64)
                            .u64(m.gesture);
                    }
                    UnitOrder::Attack { target, last_known } => {
                        d.u64(2);
                        digest_target(&mut d, *target);
                        digest_v2(&mut d, *last_known);
                    }
                    UnitOrder::Garrison { building, approach } => {
                        d.u64(3)
                            .u64(*building as u64)
                            .f64(approach.x)
                            .f64(approach.y);
                    }
                    UnitOrder::Exit => {
                        d.u64(4);
                    }
                }
            }
            digest_v2(&mut d, u.pursuit);
            digest_v2(&mut d, u.planned_goal);
            d.u64(u.route.is_some() as u64);
            for p in u.route.iter().flatten() {
                d.f64(p.x).f64(p.y);
            }
            d.f64(u.hp).f64(u.suppression).u64(u.suppressed_at);
            d.u64(u.stock.map_or(u64::MAX, u64::from));
            let p = u.progress_service;
            d.f64(p.ammo_s)
                .f64(p.hp_s)
                .f64(p.soldier_s)
                .u64(u.service as u64);
            garrison::digest(u, &mut d);
            d.u64(u.members.len() as u64);
            for s in &u.members {
                d.u64(s.id as u64)
                    .f64(s.hp)
                    .f64(s.formation.x)
                    .f64(s.formation.y);
                d.f64(s.offset.x).f64(s.offset.y);
                d.u64(s.corpse.is_some() as u64);
                if let Some(p) = s.corpse {
                    d.f64(p.x).f64(p.y).f64(p.z);
                }
            }
            d.u64(u.engagement as u64)
                .u64(u.reach.can_engage as u64)
                .u64(u.reach.needs_closer as u64);
            d.u64(u.attackers.len() as u64);
            for a in &u.attackers {
                d.u64(a.0 as u64);
            }
            d.u64(u.deployment.is_some() as u64);
            if let Some(dep) = &u.deployment {
                d.u64(dep.current as u64)
                    .u64(dep.duration as u64)
                    .u64(dep.stationary as u64);
            }
            for m in &u.mounts {
                for a in &m.ammo {
                    d.u64(a.map_or(u64::MAX, |n| n as u64));
                }
                d.u64(m.loaded.map_or(u64::MAX, |k| k as u64))
                    .f64(m.bearing)
                    .u64(m.reason as u64);
                d.u64(m.reload.is_some() as u64);
                if let Some((k, p)) = m.reload {
                    d.u64(k as u64).f64(p);
                }
                d.u64(m.support.is_some() as u64);
                if let Some(s) = m.support {
                    d.u64(s.projectile.0);
                    digest_target(&mut d, s.target);
                }
                d.u64(m.lock.is_some() as u64);
                if let Some(l) = &m.lock {
                    digest_target(&mut d, l.target);
                    d.f64(l.aim).u64(l.explicit as u64);
                }
            }
        }
        for knowledge in &self.knowledge {
            knowledge.digest(&mut d);
        }
        for id in &self.fired {
            d.u64(id.0 as u64);
        }
        for side in &self.sides {
            d.u64(side.revision);
            for id in &side.known_dynamic {
                d.u64(*id as u64);
            }
        }
        d.u64(self.world.obstacle_revision());
        self.structures.digest(&mut d);
        self.projectiles.digest(&mut d);
        d.u64(self.combat_rng.state()).u64(self.damage_rng.state());
        // The defender's memory is a player's, not the battle's: its effect is
        // its accepted commands, and a replay runs without it.

        if let Some((_, referee)) = &self.referee {
            referee.digest(&mut d);
        }
        d.finish()
    }

    pub fn replay(&self) -> Replay {
        Replay {
            scenario_digest: format!("{:016x}", self.scenario_digest),
            config_digest: format!("{:016x}", self.config_digest),
            seed: self.seed,
            accepted: self.accepted.clone(),
        }
    }
}
