//! The one battle authority: commands in, fixed ticks, side observations out.
use std::collections::VecDeque;

use contract::command::{CommandAck, CommandEnvelope, Order, OrderError, RoutePolicy};
use contract::ids::{Side, Tick, UnitId};
use contract::observation::{ObservationFrame, OwnUnit};
use contract::scenario::{Rules, ScenarioDefinition, UnitKind};
use serde::{Deserialize, Serialize};

use crate::digest::{self, Digest};
use crate::math::{v2, V2, V3};
use crate::world::WorldGeometry;

#[derive(Clone, Debug)]
enum UnitOrder {
    MoveTo {
        goal: V2,
        route: RoutePolicy,
        gesture: u64,
    },
}

#[derive(Clone, Debug)]
struct Unit {
    id: UnitId,
    side: Side,
    kind: UnitKind,
    position: V3,
    yaw: f64,
    orders: VecDeque<UnitOrder>,
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
    digest::of_str(&(map + &units))
}

fn config_digest(setup: &ScenarioDefinition) -> u64 {
    digest::of_str(&serde_json::to_string(&setup.rules).expect("rules serialize"))
}

impl Battle {
    pub fn new(setup: &ScenarioDefinition, seed: u64) -> Self {
        let world = WorldGeometry::new(&setup.map);
        let units = setup
            .units
            .iter()
            .enumerate()
            .map(|(i, u)| {
                let xy = v2(u.position[0], u.position[1]);
                Unit {
                    id: UnitId(i as u32),
                    side: u.side,
                    kind: u.kind,
                    position: xy.with_z(world.surface_at(xy.x, xy.y).map_or(0.0, |s| s.z)),
                    yaw: u.yaw,
                    orders: VecDeque::new(),
                }
            })
            .collect();
        let mut battle = Battle {
            world,
            rules: setup.rules.clone(),
            seed,
            tick: 0,
            units,
            next_seq: [1, 1],
            pending: Vec::new(),
            accepted: Vec::new(),
            replaying: None,
            observations: Default::default(),
            scenario_digest: scenario_digest(setup),
            config_digest: config_digest(setup),
        };
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

    /// Advance one fixed tick: apply scheduled commands, move, observe.
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
        for command in std::mem::take(&mut self.pending) {
            self.apply(command);
        }
        self.advance_movement();
        self.observe_all();
        self.tick
    }

    fn apply(&mut self, command: CommandEnvelope) {
        match command.order {
            Order::Move {
                units,
                gesture,
                goal,
                route,
            } => {
                for id in units {
                    let unit = &mut self.units[id.0 as usize];
                    if !command.queued {
                        unit.orders.clear();
                    }
                    unit.orders.push_back(UnitOrder::MoveTo {
                        goal: v2(goal[0], goal[1]),
                        route,
                        gesture,
                    });
                }
            }
            Order::Stop { units } => {
                for id in units {
                    self.units[id.0 as usize].orders.clear();
                }
            }
        }
    }

    /// Movement stub: straight toward the current goal at the unit's base
    /// speed, standing on the walkable surface. Routing replaces this.
    fn advance_movement(&mut self) {
        let dt = 1.0 / self.rules.tick_hz as f64;
        let m = &self.rules.movement;
        for unit in &mut self.units {
            let Some(UnitOrder::MoveTo { goal, .. }) = unit.orders.front().cloned() else {
                continue;
            };
            let speed = match unit.kind {
                UnitKind::Rifle | UnitKind::Recon | UnitKind::At => m.infantry_mps,
                UnitKind::Tank => m.tank_mps,
                UnitKind::Supply => m.supply_mps,
            };
            let to_goal = goal - unit.position.xy();
            let distance = to_goal.length();
            let step = speed * dt;
            let next = if distance <= step {
                unit.orders.pop_front();
                goal
            } else {
                unit.yaw = to_goal.y.atan2(to_goal.x);
                unit.position.xy() + to_goal * (step / distance)
            };
            let z = self
                .world
                .surface_at(next.x, next.y)
                .map_or(unit.position.z, |s| s.z);
            unit.position = next.with_z(z);
        }
    }

    fn observe_all(&mut self) {
        for side in Side::ALL {
            let frame = &mut self.observations[side.index()];
            frame.tick = self.tick;
            frame.own.clear();
            frame
                .own
                .extend(self.units.iter().filter(|u| u.side == side).map(|u| {
                    OwnUnit {
                        id: u.id,
                        kind: u.kind,
                        position: [u.position.x, u.position.y, u.position.z],
                        yaw: u.yaw,
                        goal: u
                            .orders
                            .front()
                            .map(|UnitOrder::MoveTo { goal, .. }| [goal.x, goal.y]),
                        queued: u.orders.len().saturating_sub(1) as u32,
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
            d.u64(u.orders.len() as u64);
            for UnitOrder::MoveTo {
                goal,
                route,
                gesture,
            } in &u.orders
            {
                d.f64(goal.x).f64(goal.y).u64(*route as u64).u64(*gesture);
            }
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
