//! Side-level team planning for skirmish AI.
//!
//! The planner only consumes the side observation and keeps its small amount
//! of hysteresis in [`TeamMemory`].  It never enters the battle digest: the
//! accepted command stream remains the replay authority.

use contract::catalog::TypeIndex;
use contract::encounter::{ObjectiveSite, SkirmishSites};
use contract::ids::Side;
use contract::ids::Tick;
use contract::observation::{ObservationFrame, OwnUnit};
use contract::players::{ControllerKind, Player, PlayerIndex};
use contract::scenario::Rules;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

use crate::entries::EntryIndex;

mod cohesion;
mod help;
mod stance;
pub use cohesion::{cohesion_sample, group_positions, group_ready, is_group_unit, lane_advancing};
use cohesion::{group_positions as cohesion_group_positions, group_ready as cohesion_group_ready};
pub use help::HelpRequest;
use help::{lane_contexts, update_help};
use stance::update_stance;
pub use stance::TeamStance;

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TeamTuning {
    pub planner_decision_s: f64,
    pub grid_resolution: usize,
    pub distance_weight: f64,
    pub influence_weight: f64,
    pub incumbency_bonus: f64,
    pub group_radius_m: f64,
    pub staging_distance_m: f64,
    pub influence_blend: f64,
    pub help_trend_threshold: f64,
    pub help_force_margin: f64,
    pub help_dwell_s: f64,
    pub stance_fallback_influence: f64,
    pub stance_push_influence: f64,
    pub stance_dwell_s: f64,
    pub push_timeout_s: f64,
}

impl TeamTuning {
    /// The team half of `fixtures/skirmish-ai.json`; the doctrine's tuning
    /// owns reading the file.
    pub fn shipped() -> Self {
        super::Tuning::shipped().team
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TeamMemory {
    pub influence: Option<InfluenceGrid>,
    next_plan_tick: Tick,
    cached: Option<TeamPlan>,
    planning_work: usize,
    objective_owners: BTreeMap<String, (Option<Side>, bool)>,
    previous_influence: BTreeMap<String, f64>,
    previous_score: Option<f64>,
    stance: TeamStance,
    stance_since: Tick,
    push_wait_since: Option<Tick>,
    ready_after: Tick,
    help_locks: BTreeMap<PlayerIndex, help::HelpLock>,
    ready_groups: BTreeSet<PlayerIndex>,
    outgunned: BTreeSet<PlayerIndex>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InfluenceGrid {
    pub resolution: usize,
    pub min: [f64; 2],
    pub max: [f64; 2],
    pub values: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlayerPlan {
    pub player: PlayerIndex,
    pub lane: EntryIndex,
    pub main_objective: ObjectiveSite,
    pub staging_point: [f64; 2],
}

#[derive(Clone, Debug, PartialEq)]
pub struct TeamPlan {
    pub players: Vec<PlayerPlan>,
    pub help_requests: Vec<HelpRequest>,
    pub stance: TeamStance,
    pub push_started_tick: Option<Tick>,
    pub outgunned: BTreeSet<PlayerIndex>,
}

/// The planner only needs the seat topology. Keeping this small view generic
/// lets the battle read the authoritative live skirmish seats without making
/// a second player-definition copy.
pub trait TeamPlayer {
    fn side(&self) -> Side;
    fn controller(&self) -> &ControllerKind;
}

impl TeamPlayer for Player {
    fn side(&self) -> Side {
        self.side
    }
    fn controller(&self) -> &ControllerKind {
        &self.controller
    }
}

impl TeamPlayer for crate::skirmish::PlayerState {
    fn side(&self) -> Side {
        self.side
    }
    fn controller(&self) -> &ControllerKind {
        &self.controller
    }
}

type AssignmentState = (f64, Vec<EntryIndex>, Vec<usize>);

impl TeamPlan {
    pub fn for_player(&self, player: PlayerIndex) -> Option<&PlayerPlan> {
        self.players.iter().find(|plan| plan.player == player)
    }

    pub fn help_for(&self, player: PlayerIndex) -> Vec<&HelpRequest> {
        self.help_requests
            .iter()
            .filter(|request| request.answered_by == Some(player))
            .collect()
    }

    pub fn assignment_for(&self, player: PlayerIndex) -> Option<super::Assignment> {
        let plan = self.for_player(player)?;
        let lane_fallback = self.outgunned.contains(&player);
        let (main_objective, staging_point) = if lane_fallback {
            (plan.staging_point, plan.staging_point)
        } else {
            match self.stance {
                TeamStance::Push => (plan.main_objective.center, plan.staging_point),
                TeamStance::Hold | TeamStance::FallBack => (plan.staging_point, plan.staging_point),
            }
        };
        let help = self
            .help_for(player)
            .into_iter()
            .map(|request| super::HelpTask {
                objective: request.objective.center,
                detachment: request.detachment.clone(),
            })
            .collect::<Vec<_>>();
        Some(super::Assignment {
            main_objective,
            staging_point,
            force_advance: lane_fallback || self.stance != TeamStance::Hold,
            help,
        })
    }
}

impl TeamMemory {
    pub fn cached_plan(&self) -> Option<&TeamPlan> {
        self.cached.as_ref()
    }

    pub fn latch_group_readiness(
        &mut self,
        rules: &Rules,
        frame: &ObservationFrame,
        group_radius_m: f64,
    ) {
        let Some(plan) = self.cached.as_ref() else {
            return;
        };
        if frame.tick < self.ready_after {
            self.ready_groups.clear();
            return;
        }
        for player in &plan.players {
            let positions = cohesion_group_positions(
                rules,
                frame
                    .own
                    .iter()
                    .filter(|unit| unit.owner == Some(player.player)),
            );
            if positions.is_empty() {
                self.ready_groups.remove(&player.player);
            } else if cohesion_group_ready(&positions, player.staging_point, group_radius_m) {
                self.ready_groups.insert(player.player);
            }
        }
    }
}

/// Build a deterministic side plan. Players are considered in `players`
/// order; ties therefore retain the lower player index.
pub fn plan<'a, P: TeamPlayer>(
    side: Side,
    rules: &Rules,
    side_frame: &ObservationFrame,
    players: &[P],
    sites: &SkirmishSites,
    memory: &'a mut TeamMemory,
    tuning: TeamTuning,
) -> &'a TeamPlan {
    let tick_hz = rules.tick_hz;
    let current_owners: BTreeMap<_, _> = side_frame
        .skirmish
        .as_ref()
        .map(|view| {
            view.objectives
                .iter()
                .map(|objective| (objective.id.clone(), (objective.owner, objective.contested)))
                .collect()
        })
        .unwrap_or_default();
    let objective_changed = memory.cached.as_ref().is_some_and(|cached| {
        cached.players.iter().any(|player| {
            memory
                .objective_owners
                .get(&player.main_objective.id)
                .map(|state| state.0)
                != current_owners
                    .get(&player.main_objective.id)
                    .map(|state| state.0)
        })
    });
    if side_frame.tick < memory.next_plan_tick && !objective_changed {
        return memory
            .cached
            .as_ref()
            .expect("cached plan before reschedule");
    }
    let influence = build_influence(side_frame, sites, memory.influence.as_ref(), tuning);
    memory.influence = Some(influence.clone());
    let ai: Vec<_> = players
        .iter()
        .enumerate()
        .filter(|(_, player)| {
            player.side() == side && *player.controller() == ControllerKind::ActiveAi
        })
        .map(|(index, _)| PlayerIndex(index as u16))
        .collect();
    let lanes: Vec<_> = sites
        .entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.side == side)
        .map(|(index, _)| EntryIndex(index))
        .collect();
    let mut objectives: Vec<_> = sites.objectives.clone();
    let covered_by_player = covered_objectives_by_player(side_frame, players, side, &objectives);
    let covered: BTreeSet<_> = covered_by_player
        .values()
        .map(|objective| objective.id.clone())
        .collect();
    // An objective the side already holds, uncontested, is no assignment
    // while one it doesn't hold remains: neutral or the enemy's.
    let has_untaken = side_frame.skirmish.as_ref().is_some_and(|view| {
        view.objectives
            .iter()
            .any(|objective| objective.owner != Some(side))
    });
    if has_untaken {
        let owned: BTreeSet<_> = side_frame
            .skirmish
            .as_ref()
            .map(|view| {
                view.objectives
                    .iter()
                    .filter(|objective| objective.owner == Some(side))
                    .map(|objective| objective.id.clone())
                    .collect()
            })
            .unwrap_or_default();
        objectives.retain(|objective| !owned.contains(&objective.id));
    }
    if objectives.len() > ai.len() {
        let uncovered: Vec<_> = objectives
            .iter()
            .filter(|objective| !covered.contains(&objective.id))
            .cloned()
            .collect();
        if !uncovered.is_empty() {
            objectives = uncovered;
        }
    }
    let assignment_count = ai.len().min(objectives.len());
    if assignment_count == 0 || lanes.is_empty() {
        let player_plans = Vec::new();
        let lane_contexts = lane_contexts(
            side,
            players,
            sites,
            &player_plans,
            &covered_by_player,
            tuning,
        );
        let help_requests = update_help(
            side,
            side_frame,
            players,
            &lane_contexts,
            memory,
            tuning,
            rules,
        );
        let previous_stance = memory.stance;
        let stance = update_stance(side, side_frame, &player_plans, memory, tuning, tick_hz);
        let result = TeamPlan {
            players: player_plans,
            help_requests,
            stance,
            push_started_tick: (stance == TeamStance::Push && previous_stance != TeamStance::Push)
                .then_some(side_frame.tick),
            outgunned: BTreeSet::new(),
        };
        memory.cached = Some(result.clone());
        memory.objective_owners = current_owners;
        memory.next_plan_tick = side_frame
            .tick
            .saturating_add((tuning.planner_decision_s * f64::from(tick_hz)) as Tick);
        return memory.cached.as_ref().expect("team plan cached");
    }

    // Keep the exact historical search for the small plans whose tie-breaking
    // is part of the shipped behavior. Larger teams use subset DP for the
    // one-to-one objective assignment and choose each player's best lane;
    // repeated lane slots are equivalent once there are more players than
    // physical approaches.
    // Reuse approach slots when a small map has fewer lanes than AI seats.
    let lane_slots = (0..assignment_count.max(lanes.len()))
        .map(|index| lanes[index % lanes.len()])
        .collect::<Vec<_>>();
    let mut best: Option<(f64, Vec<EntryIndex>, Vec<usize>)> = None;
    let mut score = |player: PlayerIndex, lane: EntryIndex, objective: &ObjectiveSite| {
        memory.planning_work += 1;
        let entry = &sites.entries[lane.0];
        let incumbent = memory.cached.as_ref().is_some_and(|cached| {
            cached
                .for_player(player)
                .is_some_and(|old| old.lane == lane && old.main_objective.id == objective.id)
        });
        tuning.distance_weight * distance(entry.center, objective.center)
            - tuning.influence_weight * influence.at(objective.center)
            - if incumbent {
                tuning.incumbency_bonus
            } else {
                0.0
            }
    };
    if assignment_count <= 3 {
        let lane_permutations = permutations(lane_slots.len(), assignment_count);
        let objective_permutations = permutations(objectives.len(), assignment_count);
        for lane_indices in &lane_permutations {
            for objective_indices in &objective_permutations {
                let total = ai
                    .iter()
                    .take(assignment_count)
                    .enumerate()
                    .map(|(i, player)| {
                        score(
                            *player,
                            lane_slots[lane_indices[i]],
                            &objectives[objective_indices[i]],
                        )
                    })
                    .sum::<f64>();
                if best.as_ref().is_none_or(|(old, _, _)| total < *old) {
                    best = Some((
                        total,
                        lane_indices.iter().map(|i| lane_slots[*i]).collect(),
                        objective_indices.clone(),
                    ));
                }
            }
        }
    } else {
        // The bounded search is O(players * 2^objectives * objectives).
        let objective_count = objectives.len();
        let mut states: Vec<Option<AssignmentState>> = vec![Some((0.0, Vec::new(), Vec::new()))];
        for player in ai.iter().take(assignment_count) {
            let mut next = vec![None; 1usize << objective_count];
            for (mask, state) in states.iter().enumerate() {
                let Some((base, lanes_so_far, objectives_so_far)) = state else {
                    continue;
                };
                for (objective_index, objective) in objectives.iter().enumerate() {
                    if mask & (1 << objective_index) != 0 {
                        continue;
                    }
                    let (lane, lane_score) = lane_slots
                        .iter()
                        .map(|&lane| (lane, score(*player, lane, objective)))
                        .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
                        .expect("non-empty lanes");
                    let new_mask = mask | (1 << objective_index);
                    let candidate = (
                        *base + lane_score,
                        lanes_so_far.iter().copied().chain([lane]).collect(),
                        objectives_so_far
                            .iter()
                            .copied()
                            .chain([objective_index])
                            .collect(),
                    );
                    if next[new_mask]
                        .as_ref()
                        .is_none_or(|old: &AssignmentState| candidate.0 < old.0)
                    {
                        next[new_mask] = Some(candidate);
                    }
                }
            }
            states = next;
        }
        best = states
            .into_iter()
            .flatten()
            .min_by(|a, b| a.0.total_cmp(&b.0));
    }
    let (_, chosen_lanes, chosen_objectives) = best.expect("non-empty assignment");
    let mut result = Vec::with_capacity(assignment_count);
    for (i, player) in ai.iter().take(assignment_count).enumerate() {
        let lane = chosen_lanes[i];
        let objective_index = chosen_objectives[i];
        let objective = objectives[objective_index].clone();
        let staging_point = staging(
            sites.entries[lane.0].center,
            objective.center,
            tuning.staging_distance_m,
        );
        result.push(PlayerPlan {
            player: *player,
            lane,
            main_objective: objective,
            staging_point,
        });
    }
    let lanes = lane_contexts(side, players, sites, &result, &covered_by_player, tuning);
    let help_requests = update_help(side, side_frame, players, &lanes, memory, tuning, rules);
    let previous_stance = memory.stance;
    if objective_changed {
        memory.ready_groups.clear();
    }
    let stance = update_stance(side, side_frame, &result, memory, tuning, tick_hz);
    let result = TeamPlan {
        players: result,
        help_requests,
        stance,
        push_started_tick: (stance == TeamStance::Push && previous_stance != TeamStance::Push)
            .then_some(side_frame.tick),
        outgunned: memory.outgunned.clone(),
    };
    memory.cached = Some(result.clone());
    memory.objective_owners = current_owners;
    memory.next_plan_tick = side_frame
        .tick
        .saturating_add((tuning.planner_decision_s * f64::from(tick_hz)) as Tick);
    memory.cached.as_ref().expect("team plan cached")
}

fn covered_objectives_by_player<P: TeamPlayer>(
    frame: &ObservationFrame,
    players: &[P],
    side: Side,
    objectives: &[ObjectiveSite],
) -> BTreeMap<PlayerIndex, ObjectiveSite> {
    let human: BTreeSet<_> = players
        .iter()
        .enumerate()
        .filter(|(_, p)| p.side() == side && *p.controller() == ControllerKind::Human)
        .map(|(i, _)| PlayerIndex(i as u16))
        .collect();
    let mut covered = BTreeMap::new();
    for unit in frame
        .own
        .iter()
        .filter(|unit| unit.owner.is_some_and(|owner| human.contains(&owner)))
    {
        let Some(player) = unit.owner else { continue };
        let position = [unit.position[0], unit.position[1]];
        let ordered_objective = unit.goal.and_then(|goal| {
            objectives
                .iter()
                .min_by(|a, b| distance(goal, a.center).total_cmp(&distance(goal, b.center)))
                .filter(|objective| distance(goal, objective.center) <= objective.radius_m * 2.0)
                .cloned()
        });
        let standing = objectives
            .iter()
            .find(|objective| distance(position, objective.center) <= objective.radius_m)
            .cloned();
        if let Some(objective) = standing.or(ordered_objective) {
            covered.entry(player).or_insert_with(|| objective.clone());
        }
    }
    covered
}

fn build_influence(
    frame: &ObservationFrame,
    sites: &SkirmishSites,
    previous: Option<&InfluenceGrid>,
    tuning: TeamTuning,
) -> InfluenceGrid {
    let resolution = tuning.grid_resolution.max(1);
    let mut min = [f64::INFINITY; 2];
    let mut max = [f64::NEG_INFINITY; 2];
    for point in sites
        .entries
        .iter()
        .map(|entry| entry.center)
        .chain(sites.objectives.iter().map(|objective| objective.center))
    {
        for axis in 0..2 {
            min[axis] = min[axis].min(point[axis]);
            max[axis] = max[axis].max(point[axis]);
        }
    }
    for axis in 0..2 {
        if !min[axis].is_finite() {
            min[axis] = 0.0;
            max[axis] = 1.0;
        }
        if (max[axis] - min[axis]).abs() < 1.0 {
            max[axis] = min[axis] + 1.0;
        }
    }
    let mut values = vec![0.0; resolution * resolution];
    for unit in &frame.own {
        add_strength(
            &mut values,
            resolution,
            min,
            max,
            [unit.position[0], unit.position[1]],
            1.0,
        );
    }
    for unit in &frame.identified {
        add_strength(
            &mut values,
            resolution,
            min,
            max,
            [unit.position[0], unit.position[1]],
            -(f64::from(unit.cost).max(1.0) / 100.0),
        );
    }
    if let Some(previous) =
        previous.filter(|old| old.resolution == resolution && old.min == min && old.max == max)
    {
        for (value, old) in values.iter_mut().zip(&previous.values) {
            *value = tuning.influence_blend * *old + (1.0 - tuning.influence_blend) * *value;
        }
    }
    InfluenceGrid {
        resolution,
        min,
        max,
        values,
    }
}

fn add_strength(
    values: &mut [f64],
    resolution: usize,
    min: [f64; 2],
    max: [f64; 2],
    point: [f64; 2],
    strength: f64,
) {
    let x = (((point[0] - min[0]) / (max[0] - min[0])) * resolution as f64)
        .floor()
        .clamp(0.0, resolution as f64 - 1.0) as usize;
    let y = (((point[1] - min[1]) / (max[1] - min[1])) * resolution as f64)
        .floor()
        .clamp(0.0, resolution as f64 - 1.0) as usize;
    values[y * resolution + x] += strength;
}

impl InfluenceGrid {
    pub fn at(&self, point: [f64; 2]) -> f64 {
        let x = (((point[0] - self.min[0]) / (self.max[0] - self.min[0])) * self.resolution as f64)
            .floor()
            .clamp(0.0, self.resolution as f64 - 1.0) as usize;
        let y = (((point[1] - self.min[1]) / (self.max[1] - self.min[1])) * self.resolution as f64)
            .floor()
            .clamp(0.0, self.resolution as f64 - 1.0) as usize;
        self.values[y * self.resolution + x]
    }
}

fn permutations(length: usize, take: usize) -> Vec<Vec<usize>> {
    fn visit(
        length: usize,
        take: usize,
        used: &mut [bool],
        current: &mut Vec<usize>,
        out: &mut Vec<Vec<usize>>,
    ) {
        if current.len() == take {
            out.push(current.clone());
            return;
        }
        for index in 0..length {
            if !used[index] {
                used[index] = true;
                current.push(index);
                visit(length, take, used, current, out);
                current.pop();
                used[index] = false;
            }
        }
    }
    let mut out = Vec::new();
    visit(
        length,
        take.min(length),
        &mut vec![false; length],
        &mut Vec::new(),
        &mut out,
    );
    out
}

fn staging(entry: [f64; 2], objective: [f64; 2], distance_m: f64) -> [f64; 2] {
    let dx = objective[0] - entry[0];
    let dy = objective[1] - entry[1];
    let length = libm::hypot(dx, dy);
    if length <= distance_m || length <= 0.0 {
        return entry;
    }
    [
        objective[0] - dx / length * distance_m,
        objective[1] - dy / length * distance_m,
    ]
}

fn distance(a: [f64; 2], b: [f64; 2]) -> f64 {
    libm::hypot(a[0] - b[0], a[1] - b[1])
}

#[cfg(test)]
mod tests;
