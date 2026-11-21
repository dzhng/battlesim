//! How long ground units stand still while their state says they are on
//! their way: what a played battle's movement is judged by. A unit holding
//! to fight (halted) or packing is not standing in this sense.
use contract::ids::UnitId;
use contract::observation::MoveState;
use serde::Serialize;
use sim::battle::Battle;
use sim::math::V2;

/// A stand this long is listed by itself.
const LISTED_S: f64 = 5.0;
/// The longest stands kept.
const KEPT: usize = 200;

/// Unit-seconds over a battle, ground units only.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Standing {
    /// Moving or waiting, and not a hair's breadth moved.
    pub held_s: f64,
    /// Holding for a route to be worked out.
    pub planning_s: f64,
    /// The spells of holding for a route that makes up: how many, how many
    /// of them longer than a second, and the longest.
    pub plans: u64,
    pub slow_plans: u64,
    pub longest_plan_s: f64,
    /// Waiting behind another vehicle.
    pub waiting_s: f64,
    /// With no way to where it was sent.
    pub route_blocked_s: f64,
    /// On its way and rolling or walking.
    pub going_s: f64,
    /// The longest single stands, longest first.
    pub longest: Vec<Stand>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Stand {
    pub unit: u32,
    pub kind: String,
    pub seconds: f64,
    pub from_s: f64,
    pub at: [f64; 2],
    pub state: MoveState,
    /// What it was up against, as far as its own state says.
    pub why: Why,
    /// How far it stood from where it was going.
    pub goal_m: Option<f64>,
}

/// What a standing unit was up against.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Why {
    /// A squad on foot.
    OnFoot,
    /// Behind a vehicle with nowhere to go.
    BehindParked,
    /// Behind a vehicle that has somewhere to go.
    BehindTraffic,
    /// A wheeled vehicle that found no manoeuvre.
    NoShunt,
    /// A wheeled vehicle in a manoeuvre.
    Shunting,
    /// A vehicle held with none of the above.
    Held,
}

#[derive(Clone, Copy)]
struct Run {
    last: V2,
    ticks: u64,
    state: MoveState,
    why: Why,
    goal_m: Option<f64>,
}

fn why(battle: &Battle, unit: &sim::units::Unit) -> Why {
    if !unit.is_vehicle() {
        return Why::OnFoot;
    }
    if let Some(blocker) = unit.blocker.and_then(|id| battle.unit(id)) {
        return if blocker.movement_goal().is_none() {
            Why::BehindParked
        } else {
            Why::BehindTraffic
        };
    }
    match &unit.shunt {
        Some(shunt) if shunt.no_way() => Why::NoShunt,
        Some(_) => Why::Shunting,
        None => Why::Held,
    }
}

/// Watches a battle a tick at a time.
#[derive(Default)]
pub struct Watch {
    runs: Vec<Option<Run>>,
    /// Ticks each unit has held for a route in its current spell.
    planning: Vec<u64>,
    total: Standing,
}

impl Watch {
    /// Count the tick the battle has just stepped.
    pub fn observe(&mut self, battle: &Battle) {
        let hz = f64::from(battle.rules().tick_hz);
        let now_s = battle.tick() as f64 / hz;
        for k in 0.. {
            let Some(unit) = battle.unit(UnitId(k)) else {
                break;
            };
            let k = k as usize;
            if self.runs.len() <= k {
                self.runs.resize(k + 1, None);
                self.planning.resize(k + 1, 0);
            }
            if unit.state == MoveState::Planning && unit.alive() {
                self.planning[k] += 1;
            } else {
                self.planned(k, hz);
            }
            let at = unit.position.xy();
            let goal_m = unit.movement_goal().map(|(goal, _)| (goal - at).length());
            let fresh = Run {
                last: at,
                ticks: 0,
                state: unit.state,
                why: why(battle, unit),
                goal_m,
            };
            let was = self.runs[k].replace(fresh);
            if !unit.alive() || !unit.on_field() || unit.flies() {
                self.close(battle, k, was, now_s, hz);
                self.runs[k] = None;
                continue;
            }
            let still = was.is_some_and(|run| (at - run.last).length() < 1e-6);
            match unit.state {
                MoveState::Planning => self.total.planning_s += 1.0 / hz,
                MoveState::RouteBlocked => self.total.route_blocked_s += 1.0 / hz,
                MoveState::Waiting => self.total.waiting_s += 1.0 / hz,
                MoveState::Moving if !still => self.total.going_s += 1.0 / hz,
                _ => {}
            }
            let held = still && matches!(unit.state, MoveState::Moving | MoveState::Waiting);
            if held {
                self.total.held_s += 1.0 / hz;
                let ticks = was.map_or(0, |run| run.ticks) + 1;
                self.runs[k] = Some(Run { ticks, ..fresh });
            } else {
                self.close(battle, k, was, now_s, hz);
            }
        }
    }

    /// A spell of holding for a route ends.
    fn planned(&mut self, k: usize, hz: f64) {
        let seconds = std::mem::take(&mut self.planning[k]) as f64 / hz;
        if seconds > 0.0 {
            self.total.plans += 1;
            self.total.slow_plans += u64::from(seconds > 1.0);
            self.total.longest_plan_s = self.total.longest_plan_s.max(seconds);
        }
    }

    /// A stand ends: list it if it was long.
    fn close(&mut self, battle: &Battle, k: usize, run: Option<Run>, now_s: f64, hz: f64) {
        let Some(run) = run else {
            return;
        };
        let seconds = run.ticks as f64 / hz;
        if seconds < LISTED_S {
            return;
        }
        let unit = battle.unit(UnitId(k as u32)).expect("a watched unit");
        self.total.longest.push(Stand {
            unit: k as u32,
            kind: battle.rules().catalog.get(unit.kind).name.clone(),
            seconds,
            from_s: now_s - seconds,
            at: [run.last.x, run.last.y],
            state: run.state,
            why: run.why,
            goal_m: run.goal_m,
        });
        (self.total.longest).sort_by(|a, b| b.seconds.total_cmp(&a.seconds));
        self.total.longest.truncate(KEPT);
    }

    /// The battle's totals, stands still running included.
    pub fn finish(mut self, battle: &Battle) -> Standing {
        let hz = f64::from(battle.rules().tick_hz);
        let now_s = battle.tick() as f64 / hz;
        for k in 0..self.runs.len() {
            self.planned(k, hz);
            let run = self.runs[k].take();
            self.close(battle, k, run, now_s, hz);
        }
        self.total
    }
}
