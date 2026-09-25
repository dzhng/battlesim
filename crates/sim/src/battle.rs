//! The one battle authority: commands in, fixed ticks, side observations out.
use std::collections::VecDeque;

use contract::command::{CommandAck, CommandEnvelope, Order, OrderError};
use contract::ids::{Side, Tick, UnitId};
use contract::observation::{
    KnownProp, MoveState, ObservationFrame, OwnUnit, SoundCue, VisibilityField,
};
use contract::scenario::{EventAction, Rules, ScenarioDefinition, ScenarioEvent, ScriptedOrder};
use serde::{Deserialize, Serialize};

use std::collections::BTreeSet;

use crate::digest::{self, Digest};
use crate::hearing;
use crate::knowledge::SideKnowledge;
use crate::math::{v2, V2};
use crate::movement::{self, MovementContext, SideGeometry};
use crate::sensing;
use crate::units::{self, MoveOrder, Soldier, Unit};
use crate::visibility::{self, OcclusionGrid};
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
    next_seq: [u64; 2],
    pending: Vec<CommandEnvelope>,
    accepted: Vec<(Tick, CommandEnvelope)>,
    /// Recorded commands still to apply when this battle is a replay.
    replaying: Option<VecDeque<(Tick, CommandEnvelope)>>,
    observations: [ObservationFrame; 2],
    scenario_digest: u64,
    config_digest: u64,
}

fn scenario_digest(setup: &ScenarioDefinition) -> u64 {
    let map = serde_json::to_string(&setup.map).expect("map serializes");
    let units = serde_json::to_string(&setup.units).expect("units serialize");
    let events = serde_json::to_string(&setup.events).expect("events serialize");
    let scripts = serde_json::to_string(&setup.scripts).expect("scripts serialize");
    digest::of_str(&(map + &units + &events + &scripts))
}

fn config_digest(setup: &ScenarioDefinition) -> u64 {
    digest::of_str(&serde_json::to_string(&setup.rules).expect("rules serialize"))
}

impl Battle {
    pub fn new(setup: &ScenarioDefinition, seed: u64) -> Self {
        let world = WorldGeometry::new(&setup.map);
        let rules = setup.rules.clone();
        let mut soldier_ids = 0u32;
        let units = setup
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
                            alive: true,
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
                }
            })
            .collect();
        let mut events: Vec<ScenarioEvent> = setup.events.clone();
        events.sort_by_key(|e| e.tick);
        let mut scripts: Vec<ScriptedOrder> = setup.scripts.clone();
        scripts.sort_by_key(|o| o.tick);
        let occlusion = OcclusionGrid::new(&world, rules.sensors.fog_cell_m);
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
            fog: [occlusion.field(), occlusion.field()],
            occlusion,
            next_seq: [1, 1],
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

    fn validate(&self, command: &CommandEnvelope) -> Result<(), OrderError> {
        let units = match &command.order {
            Order::Move { units, goal, .. } => {
                if self.world.height_at(goal[0], goal[1]).is_none() {
                    return Err(OrderError::OutOfBounds);
                }
                units
            }
            Order::Stop { units } => units,
            // Gesture tokens are scoped by side at application; nothing else to check.
            Order::UpgradeMove { .. } => return Ok(()),
        };
        if units.is_empty() {
            return Err(OrderError::NoUnits);
        }
        for &unit in units {
            match self.units.get(unit.0 as usize) {
                None => return Err(OrderError::UnknownUnit { unit }),
                Some(u) if u.side != command.side => return Err(OrderError::NotOwnUnit { unit }),
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
        let ctx = MovementContext {
            world: &self.world,
            authored: self.authored_props,
            tick: self.tick,
            tick_hz: self.rules.tick_hz,
            vehicle_turn_deg_s: self.rules.movement.vehicle_turn_deg_s,
        };
        movement::advance(&ctx, &mut self.units, &mut self.sides);
        for side in Side::ALL {
            self.sense(side);
            if self.tick % FOG_INTERVAL_TICKS == side.index() as u64 * FOG_INTERVAL_TICKS / 2 {
                self.sweep_fog(side);
            }
        }
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
        self.observe_all();
        self.tick
    }

    /// `unit` fired: every opposing side learns an uncertain firing area and
    /// may hear the shot. Weapons and the lab emitter share this seam.
    pub fn record_fire(&mut self, unit: UnitId) {
        let Some(shooter) = self.units.get(unit.0 as usize) else {
            return;
        };
        let (side, at) = (shooter.side, shooter.position.xy());
        for other in Side::ALL.into_iter().filter(|s| *s != side) {
            self.knowledge[other.index()].note_fire(unit, at);
        }
        self.fired.insert(unit);
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
            visibility::sweep(
                &self.world,
                &self.occlusion,
                &self.rules.sensors,
                sensing::eye(unit, &self.rules),
                range,
                &mut field,
            );
        }
        let known = &mut self.sides[side.index()];
        for prop in self.world.props().filter(|p| p.id >= self.authored_props) {
            if field.visible(prop.center.x, prop.center.y) {
                known.learn(prop.id);
            }
        }
        self.fog[side.index()] = field;
    }

    fn apply(&mut self, command: CommandEnvelope) {
        let side = command.side;
        match command.order {
            Order::Move {
                units,
                gesture,
                goal,
                route,
            } => {
                let destinations = self.group_destinations(side, &units, v2(goal[0], goal[1]));
                for (id, destination) in units.into_iter().zip(destinations) {
                    let unit = &mut self.units[id.0 as usize];
                    if !command.queued {
                        unit.orders.clear();
                        unit.route = None;
                        unit.state = MoveState::Idle;
                    }
                    unit.orders.push_back(MoveOrder {
                        destination,
                        policy: route,
                        gesture,
                    });
                }
            }
            Order::Stop { units } => {
                for id in units {
                    let unit = &mut self.units[id.0 as usize];
                    unit.orders.clear();
                    unit.route = None;
                    unit.state = MoveState::Idle;
                    unit.blocker = None;
                }
            }
            Order::UpgradeMove { gesture, route } => {
                for unit in self.units.iter_mut().filter(|u| u.side == side) {
                    for (k, order) in unit.orders.iter_mut().enumerate() {
                        if order.gesture == gesture && order.policy != route {
                            order.policy = route;
                            if k == 0 {
                                unit.route = None; // replan the active order
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
            let frame = &mut self.observations[side.index()];
            frame.tick = self.tick;
            frame.identified.clear();
            frame
                .identified
                .extend(knowledge.identified(self.tick, &self.units, &self.rules));
            frame.ground_visibility.clone_from(&self.fog[side.index()]);
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
                            })
                        }),
                );
            frame.own.clear();
            frame
                .own
                .extend(self.units.iter().filter(|u| u.side == side).map(|u| {
                    OwnUnit {
                        id: u.id,
                        kind: u.kind,
                        position: [u.position.x, u.position.y, u.position.z],
                        yaw: u.yaw,
                        goal: u.current_destination().map(|g| [g.x, g.y]),
                        policy: u.orders.front().map(|o| o.policy),
                        state: u.state,
                        blocker: u.blocker,
                        route: u.route.iter().flatten().map(|p| [p.x, p.y]).collect(),
                        queue: u
                            .orders
                            .iter()
                            .skip(1)
                            .map(|o| [o.destination.x, o.destination.y])
                            .collect(),
                        members: u.member_positions().map(|p| [p.x, p.y, p.z]).collect(),
                        sees: knowledge.own_sensor(u.id),
                    }
                }));
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
                d.f64(o.destination.x)
                    .f64(o.destination.y)
                    .u64(o.policy as u64)
                    .u64(o.gesture);
            }
            for p in u.route.iter().flatten() {
                d.f64(p.x).f64(p.y);
            }
            for s in &u.members {
                d.u64(s.id as u64).u64(s.alive as u64);
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
