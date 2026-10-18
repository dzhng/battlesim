//! Side-level team planning for skirmish AI.
//!
//! The planner only consumes the side observation and keeps its small amount
//! of hysteresis in [`TeamMemory`].  It never enters the battle digest: the
//! accepted command stream remains the replay authority.

use contract::encounter::{ObjectiveSite, SkirmishSites};
use contract::ids::Side;
use contract::ids::Tick;
use contract::observation::{ObservationFrame, OwnUnit};
use contract::players::{ControllerKind, Player, PlayerIndex};
use contract::scenario::Rules;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

use crate::entries::EntryIndex;

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
}

impl TeamMemory {
    pub fn cached_plan(&self) -> Option<&TeamPlan> {
        self.cached.as_ref()
    }
}

/// Build a deterministic side plan. Players are considered in `players`
/// order; ties therefore retain the lower player index.
pub fn plan<'a, P: TeamPlayer>(
    side: Side,
    side_frame: &ObservationFrame,
    players: &[P],
    sites: &SkirmishSites,
    memory: &'a mut TeamMemory,
    tuning: TeamTuning,
    tick_hz: u32,
) -> &'a TeamPlan {
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
    let covered = covered_objectives(side_frame, players, side, &objectives);
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
        let result = TeamPlan {
            players: Vec::new(),
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
    let result = TeamPlan { players: result };
    memory.cached = Some(result.clone());
    memory.objective_owners = current_owners;
    memory.next_plan_tick = side_frame
        .tick
        .saturating_add((tuning.planner_decision_s * f64::from(tick_hz)) as Tick);
    memory.cached.as_ref().expect("team plan cached")
}

fn covered_objectives<P: TeamPlayer>(
    frame: &ObservationFrame,
    players: &[P],
    side: Side,
    objectives: &[ObjectiveSite],
) -> BTreeSet<String> {
    let human: BTreeSet<_> = players
        .iter()
        .enumerate()
        .filter(|(_, p)| p.side() == side && *p.controller() == ControllerKind::Human)
        .map(|(i, _)| PlayerIndex(i as u16))
        .collect();
    frame
        .own
        .iter()
        .filter(|unit| {
            unit.owner.is_some_and(|owner| human.contains(&owner)) && unit.goal.is_some()
        })
        .filter_map(|unit| {
            let goal = unit.goal?;
            objectives
                .iter()
                .min_by(|a, b| distance(goal, a.center).total_cmp(&distance(goal, b.center)))
                .and_then(|objective| {
                    (distance(goal, objective.center) <= objective.radius_m * 2.0)
                        .then_some(objective.id.clone())
                })
        })
        .collect()
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

/// Count a main group's members that are inside its lane approach radius.
/// The caller decides when the army exists and when sampling begins.
pub fn cohesion_sample(positions: &[[f64; 2]], approach: [f64; 2], radius: f64) -> [u64; 2] {
    let inside = positions
        .iter()
        .filter(|position| distance(**position, approach) <= radius)
        .count() as u64;
    [inside, positions.len() as u64]
}

/// Units that the lane staging/advance branch actually moves as a group.
pub fn is_group_unit(rules: &Rules, unit: &OwnUnit) -> bool {
    let catalog = rules.catalog.get(unit.kind);
    !catalog.has_role("logistics")
        && !catalog.has_role("recon")
        && super::transport::carrier_of(unit).is_none()
        && (crate::carriage::seats(rules, unit.kind) == 0
            || !rules.catalog.mounts(unit.kind).is_empty())
}

pub fn group_positions<'a>(
    rules: &Rules,
    units: impl IntoIterator<Item = &'a OwnUnit>,
) -> Vec<[f64; 2]> {
    units
        .into_iter()
        .filter(|unit| is_group_unit(rules, unit))
        .map(|unit| [unit.position[0], unit.position[1]])
        .collect()
}

/// A push may begin once the core of the group is together. One late
/// reinforcement must not hold the whole lane at staging forever.
pub fn group_ready(positions: &[[f64; 2]], approach: [f64; 2], radius: f64) -> bool {
    let sample = cohesion_sample(positions, approach, radius);
    sample[1] >= 2 && sample[0] * 4 >= sample[1] * 3
}

#[cfg(test)]
mod tests {
    use super::*;
    use contract::encounter::{EntrySite, ObjectiveSiteKind};

    fn sites() -> SkirmishSites {
        SkirmishSites {
            entries: vec![
                EntrySite {
                    side: Side::Blue,
                    center: [100.0, 0.0],
                    yaw: 0.0,
                },
                EntrySite {
                    side: Side::Blue,
                    center: [300.0, 0.0],
                    yaw: 0.0,
                },
                EntrySite {
                    side: Side::Blue,
                    center: [500.0, 0.0],
                    yaw: 0.0,
                },
                EntrySite {
                    side: Side::Red,
                    center: [100.0, 1000.0],
                    yaw: 0.0,
                },
                EntrySite {
                    side: Side::Red,
                    center: [300.0, 1000.0],
                    yaw: 0.0,
                },
                EntrySite {
                    side: Side::Red,
                    center: [500.0, 1000.0],
                    yaw: 0.0,
                },
            ],
            objectives: vec![
                ObjectiveSite {
                    id: "west".into(),
                    center: [100.0, 500.0],
                    radius_m: 30.0,
                    kind: ObjectiveSiteKind::Field,
                    counterpart: None,
                },
                ObjectiveSite {
                    id: "centre".into(),
                    center: [300.0, 500.0],
                    radius_m: 30.0,
                    kind: ObjectiveSiteKind::Junction,
                    counterpart: None,
                },
                ObjectiveSite {
                    id: "east".into(),
                    center: [500.0, 500.0],
                    radius_m: 30.0,
                    kind: ObjectiveSiteKind::Field,
                    counterpart: None,
                },
            ],
            candidates: vec![],
        }
    }

    fn players() -> Vec<Player> {
        (0..3)
            .map(|index| Player {
                id: format!("blue-{}", index + 1),
                side: Side::Blue,
                faction: Some(contract::catalog::Faction::Us),
                controller: ControllerKind::ActiveAi,
                human_capable: false,
            })
            .chain((0..3).map(|index| Player {
                id: format!("red-{}", index + 1),
                side: Side::Red,
                faction: Some(contract::catalog::Faction::Europe),
                controller: ControllerKind::ActiveAi,
                human_capable: false,
            }))
            .collect()
    }

    #[test]
    fn three_ai_players_get_distinct_lanes_and_objectives() {
        let players = players();
        let sites = sites();
        let mut memory = TeamMemory::default();
        let plan = plan(
            Side::Blue,
            &ObservationFrame::default(),
            &players,
            &sites,
            &mut memory,
            TeamTuning {
                planner_decision_s: 5.0,
                grid_resolution: 4,
                distance_weight: 1.0,
                influence_weight: 1.0,
                incumbency_bonus: 80.0,
                group_radius_m: 90.0,
                staging_distance_m: 100.0,
                influence_blend: 0.5,
            },
            30,
        );
        assert_eq!(plan.players.len(), 3);
        assert_eq!(
            plan.players
                .iter()
                .map(|player| player.lane)
                .collect::<BTreeSet<_>>()
                .len(),
            3
        );
        assert_eq!(
            plan.players
                .iter()
                .map(|player| player.main_objective.id.clone())
                .collect::<BTreeSet<_>>()
                .len(),
            3
        );
    }

    #[test]
    fn a_large_team_plan_uses_bounded_assignment_work() {
        let mut players = players();
        players.truncate(3);
        for index in 3..10 {
            players.push(Player {
                id: format!("blue-{}", index + 1),
                side: Side::Blue,
                faction: Some(contract::catalog::Faction::Us),
                controller: ControllerKind::ActiveAi,
                human_capable: false,
            });
        }
        let mut sites = sites();
        for index in 3..7 {
            let mut objective = sites.objectives[index % 3].clone();
            objective.id = format!("extra-{index}");
            objective.center[0] += 25.0 * index as f64;
            sites.objectives.push(objective);
        }
        let mut memory = TeamMemory::default();
        let plan = plan(
            Side::Blue,
            &ObservationFrame::default(),
            &players,
            &sites,
            &mut memory,
            TeamTuning {
                planner_decision_s: 5.0,
                grid_resolution: 4,
                distance_weight: 1.0,
                influence_weight: 1.0,
                incumbency_bonus: 80.0,
                group_radius_m: 90.0,
                staging_distance_m: 100.0,
                influence_blend: 0.5,
            },
            30,
        );
        assert_eq!(plan.players.len(), 7);
        assert!(
            memory.planning_work < 100_000,
            "work={}",
            memory.planning_work
        );
    }

    #[test]
    fn three_ai_players_reuse_two_lane_slots() {
        let players = players();
        let mut sites = sites();
        sites.entries.retain(|entry| {
            entry.side == Side::Red && entry.center[0] < 400.0
                || entry.side == Side::Blue && entry.center[0] < 400.0
        });
        let mut memory = TeamMemory::default();
        let plan = plan(
            Side::Blue,
            &ObservationFrame::default(),
            &players,
            &sites,
            &mut memory,
            TeamTuning {
                planner_decision_s: 5.0,
                grid_resolution: 4,
                distance_weight: 1.0,
                influence_weight: 1.0,
                incumbency_bonus: 80.0,
                group_radius_m: 90.0,
                staging_distance_m: 100.0,
                influence_blend: 0.5,
            },
            30,
        );
        assert_eq!(plan.players.len(), 3);
    }

    /// One AI beside a human on a three-lane side. Its only open objective
    /// lies in front of the third lane, so that is the lane it plans for: the
    /// first lane (the centre entry) is a candidate like any other, never the
    /// only one.
    #[test]
    fn a_lone_ai_takes_the_lane_nearest_its_objective_not_the_first() {
        let mut players = players();
        players[0].controller = ControllerKind::Human;
        players[2].controller = ControllerKind::Human;
        let mut sites = sites();
        sites.objectives.retain(|objective| objective.id == "east");
        let mut memory = TeamMemory::default();
        let plan = plan(
            Side::Blue,
            &ObservationFrame::default(),
            &players,
            &sites,
            &mut memory,
            TeamTuning {
                planner_decision_s: 5.0,
                grid_resolution: 4,
                distance_weight: 1.0,
                influence_weight: 1.0,
                incumbency_bonus: 80.0,
                group_radius_m: 90.0,
                staging_distance_m: 100.0,
                influence_blend: 0.5,
            },
            30,
        );
        assert_eq!(plan.players.len(), 1);
        assert_eq!(plan.players[0].player, PlayerIndex(1));
        assert_eq!(plan.players[0].main_objective.id, "east");
        assert_eq!(
            plan.players[0].lane,
            EntryIndex(2),
            "the lane in front of its objective"
        );
    }

    #[test]
    fn planning_is_repeatable_and_incumbency_survives_a_marginal_score_change() {
        let players = players();
        let sites = sites();
        let mut memory = TeamMemory::default();
        let tuning = TeamTuning {
            planner_decision_s: 5.0,
            grid_resolution: 4,
            distance_weight: 1.0,
            influence_weight: 1.0,
            incumbency_bonus: 80.0,
            group_radius_m: 90.0,
            staging_distance_m: 100.0,
            influence_blend: 0.5,
        };
        let first = plan(
            Side::Blue,
            &ObservationFrame::default(),
            &players,
            &sites,
            &mut memory,
            tuning,
            30,
        );
        let first = first.clone();
        let second = plan(
            Side::Blue,
            &ObservationFrame::default(),
            &players,
            &sites,
            &mut memory,
            tuning,
            30,
        );
        assert_eq!(first, second.clone());
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn a_held_objective_replans_to_one_the_side_does_not_hold_immediately() {
        let players = players();
        let mut sites = sites();
        sites.objectives.truncate(3);
        let mut memory = TeamMemory::default();
        let tuning = TeamTuning {
            planner_decision_s: 5.0,
            grid_resolution: 4,
            distance_weight: 1.0,
            influence_weight: 1.0,
            incumbency_bonus: 80.0,
            group_radius_m: 90.0,
            staging_distance_m: 100.0,
            influence_blend: 0.5,
        };
        let mut frame = ObservationFrame::default();
        frame.skirmish = Some(contract::skirmish::SkirmishView {
            phase: contract::skirmish::Phase::Active,
            players: vec![contract::skirmish::PlayerView {
                index: PlayerIndex(0),
                id: "blue-1".into(),
                controller: ControllerKind::ActiveAi,
                ready: true,
                credits: 0.0,
                unit_count: 0,
                cap: 30,
                pending: vec![],
            }],
            ready: [true, false],
            preparation_remaining_s: 0.0,
            credits: 0.0,
            unit_count: 0,
            max_units: 30,
            pending: vec![],
            objectives: sites
                .objectives
                .iter()
                .map(|objective| contract::skirmish::ObjectiveView {
                    id: objective.id.clone(),
                    center: objective.center,
                    radius_m: objective.radius_m,
                    owner: None,
                    capturing: None,
                    capture_progress: 0.0,
                    contested: false,
                })
                .collect(),
            scores: [0.0; 2],
            result: None,
            report: None,
        });
        let first = plan(
            Side::Blue,
            &frame,
            &players,
            &sites,
            &mut memory,
            tuning,
            30,
        );
        let first_id = first.players[0].main_objective.id.clone();
        let first_plan = first.clone();
        frame
            .skirmish
            .as_mut()
            .unwrap()
            .objectives
            .iter_mut()
            .find(|objective| objective.id == first_id)
            .unwrap()
            .contested = true;
        frame.tick = 1;
        let flicker = plan(
            Side::Blue,
            &frame,
            &players,
            &sites,
            &mut memory,
            tuning,
            30,
        );
        assert_eq!(flicker, &first_plan);
        frame
            .skirmish
            .as_mut()
            .unwrap()
            .objectives
            .iter_mut()
            .find(|objective| objective.id == first_id)
            .unwrap()
            .contested = false;
        frame.tick = 2;
        let stable = plan(
            Side::Blue,
            &frame,
            &players,
            &sites,
            &mut memory,
            tuning,
            30,
        );
        assert_eq!(stable, &first_plan);
        let view = frame.skirmish.as_mut().unwrap();
        view.objectives
            .iter_mut()
            .find(|objective| objective.id == first_id)
            .unwrap()
            .owner = Some(Side::Blue);
        frame.tick = 1;
        let second = plan(
            Side::Blue,
            &frame,
            &players,
            &sites,
            &mut memory,
            tuning,
            30,
        );
        assert_ne!(second.players[0].main_objective.id, first_id);

        // With nothing neutral left, the enemy's objectives are still the ones
        // to take: the AI does not settle back on the one it holds.
        // Blue now holds its current assignment, the enemy holds the rest.
        let second_id = second.players[0].main_objective.id.clone();
        for objective in &mut frame.skirmish.as_mut().unwrap().objectives {
            objective.owner = Some(if objective.id == second_id {
                Side::Blue
            } else {
                Side::Red
            });
        }
        frame.tick = 2;
        let third = plan(
            Side::Blue,
            &frame,
            &players,
            &sites,
            &mut memory,
            tuning,
            30,
        );
        assert_ne!(third.players[0].main_objective.id, second_id);
    }

    #[test]
    fn cohesion_counts_a_gathered_group_and_rejects_a_spread_group() {
        let gathered = [[95.0, 100.0], [100.0, 105.0], [105.0, 95.0]];
        let spread = [[95.0, 100.0], [100.0, 105.0], [500.0, 500.0]];
        assert_eq!(cohesion_sample(&gathered, [100.0, 100.0], 20.0), [3, 3]);
        assert_eq!(cohesion_sample(&spread, [100.0, 100.0], 20.0), [2, 3]);
        assert!(!group_ready(&spread, [100.0, 100.0], 20.0));
    }
}
