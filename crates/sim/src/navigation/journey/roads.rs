//! The quickest way by road for one leg, found on the map's road graph: where
//! the road can be joined near the start and left near the goal, the nodes
//! between, and the runs to drive. Nothing here is checked against the
//! bodies on the road; [`super::Driving`] makes the way physical.
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};

use contract::command::RoutePolicy;
use contract::scenario::NavigationRules;

use super::super::roads::{Access, RoadNet};
use super::super::search::{Open, RouteSearch};
use super::super::{Leg, NavGrid};
use super::{JOIN_EPSILON_M, KEEP_RIGHT_M, MERGE_M};
use crate::digest::Digest;
use crate::math::V2;

/// The final centre slides along its goal arc, then shifts by one lane
/// width. Source snapping and point coalescing cover every admitted cell.
pub(super) fn goal_access_outside(
    roads: &RoadNet,
    leg: Leg,
    component: Option<&RouteSearch>,
    arc: u32,
) -> bool {
    let Some(component) = component else {
        return false;
    };
    let arc = roads.arc(arc);
    let padding =
        super::super::START_REACH_M + (leg.m.half_width_m + KEEP_RIGHT_M).abs() + JOIN_EPSILON_M;
    component.excludes_segment(roads.node(arc.ends[0]), roads.node(arc.ends[1]), padding)
}

/// How a node of the road graph was reached: from one of the leg's ways
/// onto the road, or along an arc.
#[derive(Clone, Copy)]
pub(super) enum Via {
    Start(usize),
    Arc(u32),
}

/// A whole way by road, start to goal.
#[derive(Clone, Copy)]
pub(super) struct Way {
    /// Its estimated cost: straight to the road, along it, straight off it.
    pub(super) cost: f64,
    /// The last node before the road is left, and where it is left. No
    /// node: on at `start` and off at `goal` along one arc.
    pub(super) end: Option<u32>,
    pub(super) start: usize,
    pub(super) goal: usize,
}

/// The best road journey under the requested policy, a node a step.
pub(super) struct RoadSearch {
    /// Where the road can be joined near the start, and left near the goal.
    pub(super) starts: Vec<Access>,
    pub(super) goals: Vec<Access>,
    pub(super) open: BinaryHeap<Open>,
    /// Each node's cost from the start and how it was reached.
    pub(super) reached: BTreeMap<u32, (f64, Via)>,
    /// Each node the road can be left after: the cost on to the goal, and
    /// where it is left.
    pub(super) exits: BTreeMap<u32, (f64, usize)>,
    pub(super) best: Option<Way>,
    pub(super) initializing: usize,
    pub(super) left_at: BTreeMap<u32, usize>,
    pub(super) goal_costs: Vec<f64>,
    pub(super) across: f64,
    pub(super) connector: Option<super::super::terrain::Probe>,
    /// Arc cursor and nearest access at each missing endpoint. A missing
    /// nearby road cannot send a river crossing straight back to the grid.
    pub(super) finding: Option<AccessSearch>,
    pub(super) found_alternatives: [bool; 2],
    pub(super) access_reach: f64,
}

/// Discover the nearest terrain-reachable road for an exhausted endpoint.
/// One arc/endpoint is admitted per step; its terrain probe also yields.
pub(super) struct AccessSearch {
    pub(super) arc: usize,
    pub(super) side: usize,
    pub(super) needed: [bool; 2],
    pub(super) nearest: [Option<Access>; 2],
    pub(super) candidates: [Vec<Access>; 2],
    pub(super) selected: [Vec<Access>; 2],
    pub(super) selecting: (usize, usize),
    pub(super) reach: f64,
    pub(super) probe: Option<(Access, super::super::terrain::Probe)>,
}

impl AccessSearch {
    pub(super) fn new(needed: [bool; 2], reach: f64) -> Self {
        Self {
            arc: 0,
            side: 0,
            needed,
            nearest: [None, None],
            candidates: Default::default(),
            selected: Default::default(),
            selecting: (0, 0),
            reach,
            probe: None,
        }
    }

    pub(super) fn next(&mut self) {
        self.side += 1;
        if self.side == 2 {
            self.side = 0;
            self.arc += 1;
        }
    }

    pub(super) fn step(
        &mut self,
        grid: &NavGrid,
        roads: &RoadNet,
        leg: Leg,
        closed: &BTreeSet<u32>,
        rejected: &[BTreeSet<u32>; 2],
        component: Option<&RouteSearch>,
    ) -> bool {
        if let Some((access, mut probe)) = self.probe.take() {
            match grid.terrain_step(&mut probe) {
                None => {
                    self.probe = Some((access, probe));
                    return false;
                }
                Some(true) => {
                    if self.nearest[self.side].is_none_or(|old| access.distance < old.distance) {
                        self.nearest[self.side] = Some(access);
                    }
                    self.candidates[self.side].push(access);
                }
                Some(false) => {}
            }
            self.next();
            return false;
        }
        if self.arc == roads.arc_count() {
            let (side, at) = self.selecting;
            if side == 2 {
                return true;
            }
            if let Some(&access) = self.candidates[side].get(at) {
                if self.nearest[side]
                    .is_some_and(|nearest| access.distance <= nearest.distance + self.reach)
                {
                    self.selected[side].push(access);
                }
                self.selecting.1 += 1;
            } else {
                self.selecting = (side + 1, 0);
            }
            return false;
        }
        let arc = self.arc as u32;
        if !self.needed[self.side]
            || closed.contains(&arc)
            || rejected[self.side].contains(&arc)
            || (self.side == 1 && goal_access_outside(roads, leg, component, arc))
        {
            self.next();
            return false;
        }
        let point = [leg.from, leg.goal][self.side];
        let access = roads.access(arc, point);
        if self.nearest[self.side].is_some_and(|old| old.distance + self.reach < access.distance) {
            self.next();
        } else {
            self.probe = Some((access, super::super::terrain::Probe::new(point, access.at)));
        }
        false
    }

    pub(super) fn digest(&self, d: &mut Digest) {
        d.u64(self.arc as u64)
            .u64(self.side as u64)
            .u64(self.selecting.0 as u64)
            .u64(self.selecting.1 as u64);
        for accesses in self.candidates.iter().chain(&self.selected) {
            d.u64(accesses.len() as u64);
            for access in accesses {
                d.u64(access.arc as u64);
            }
        }
        for (needed, access) in self.needed.iter().zip(&self.nearest) {
            d.u64(*needed as u64).u64(access.is_some() as u64);
            if let Some(access) = access {
                d.u64(access.arc as u64).f64(access.distance);
            }
        }
        d.u64(self.probe.is_some() as u64);
        if let Some((access, probe)) = &self.probe {
            d.u64(access.arc as u64);
            probe.digest(d);
        }
    }
}

/// A metre's cost on this arc, in metres or seconds by the requested policy.
pub(super) fn pace(roads: &RoadNet, arc: u32, leg: Leg) -> f64 {
    match leg.policy {
        RoutePolicy::Shortest => 1.0,
        RoutePolicy::Fastest => 1.0 / leg.m.speed(roads.arc(arc).footing()),
    }
}

pub(super) fn over_ground(grid: &NavGrid, a: V2, b: V2, leg: Leg) -> f64 {
    match leg.policy {
        RoutePolicy::Shortest => (b - a).length(),
        RoutePolicy::Fastest => grid.across(a, b, leg.m),
    }
}

impl RoadSearch {
    /// A blocked straight leg may need a road beyond the usual access
    /// radius. Missing endpoint accesses are found one arc per step.
    pub(super) fn new(
        grid: &NavGrid,
        roads: &RoadNet,
        leg: Leg,
        rules: &NavigationRules,
        closed: &BTreeSet<u32>,
        rejected: &[BTreeSet<u32>; 2],
        across: f64,
    ) -> Option<Self> {
        let near = |p: V2, side: usize| -> Vec<Access> {
            roads
                .near(p, rules.road_access_m)
                .into_iter()
                .filter(|access| {
                    !closed.contains(&access.arc) && !rejected[side].contains(&access.arc)
                })
                .collect()
        };
        let (starts, goals) = (near(leg.from, 0), near(leg.goal, 1));
        grid.spend(1 + (starts.len() + goals.len()) as u64 / 8);
        if roads.is_empty() || (across.is_finite() && (starts.is_empty() || goals.is_empty())) {
            return None;
        }
        let needed = [starts.is_empty(), goals.is_empty()];
        let finding = needed
            .iter()
            .any(|needed| *needed)
            .then(|| AccessSearch::new(needed, rules.road_access_m));
        let found_alternatives = needed;
        Some(RoadSearch {
            goal_costs: vec![f64::INFINITY; goals.len()],
            starts,
            goals,
            open: BinaryHeap::new(),
            reached: BTreeMap::new(),
            exits: BTreeMap::new(),
            best: None,
            initializing: 0,
            left_at: BTreeMap::new(),
            across,
            connector: None,
            finding,
            found_alternatives,
            access_reach: rules.road_access_m,
        })
    }

    /// Admit one endpoint connector per step. An inaccessible nearby road
    /// cannot outrank a reachable bridge approach across the river.
    pub(super) fn initialize(
        &mut self,
        grid: &NavGrid,
        roads: &RoadNet,
        leg: Leg,
        component: Option<&RouteSearch>,
    ) {
        let k = self.initializing;
        if k < self.goals.len() && goal_access_outside(roads, leg, component, self.goals[k].arc) {
            self.initializing += 1;
            return;
        }
        let (a, b) = if k < self.goals.len() {
            (self.goals[k].at, leg.goal)
        } else {
            (leg.from, self.starts[k - self.goals.len()].at)
        };
        let probe = self
            .connector
            .get_or_insert_with(|| super::super::terrain::Probe::new(a, b));
        let Some(clear) = grid.terrain_step(probe) else {
            return;
        };
        self.connector = None;
        self.initializing += 1;
        if !clear {
            return;
        }
        let ends = |access: &Access| {
            let arc = roads.arc(access.arc);
            let pace = pace(roads, access.arc, leg);
            [
                (arc.ends[0], access.along * pace),
                (arc.ends[1], (arc.length - access.along) * pace),
            ]
        };
        if k < self.goals.len() {
            let access = self.goals[k];
            let leave = over_ground(grid, access.at, leg.goal, leg);
            self.goal_costs[k] = leave;
            self.left_at.insert(access.arc, k);
            for (node, cost) in ends(&access) {
                let cost = cost + leave;
                if self.exits.get(&node).is_none_or(|(t, _)| cost < *t) {
                    self.exits.insert(node, (cost, k));
                }
            }
        } else {
            let start = k - self.goals.len();
            let access = self.starts[start];
            let join = over_ground(grid, leg.from, access.at, leg);
            for (node, cost) in ends(&access) {
                self.reach(roads, leg, node, join + cost, Via::Start(start));
            }
            if let Some(&goal) = self.left_at.get(&access.arc) {
                let exit = self.goals[goal];
                let along = (exit.along - access.along).abs() * pace(roads, access.arc, leg);
                self.offer(Way {
                    cost: join + along + self.goal_costs[goal],
                    end: None,
                    start,
                    goal,
                });
            }
        }
    }

    pub(super) fn offer(&mut self, way: Way) {
        if self.best.is_none_or(|best| way.cost < best.cost) {
            self.best = Some(way);
        }
    }

    /// A lower bound on remaining distance, or time at the mover's best
    /// speed, under the requested policy.
    pub(super) fn on(roads: &RoadNet, leg: Leg, node: u32) -> f64 {
        let distance = (roads.node(node) - leg.goal).length();
        match leg.policy {
            RoutePolicy::Shortest => distance,
            RoutePolicy::Fastest => distance / leg.m.max_speed(),
        }
    }

    pub(super) fn reach(&mut self, roads: &RoadNet, leg: Leg, node: u32, cost: f64, via: Via) {
        if self.reached.get(&node).is_some_and(|(t, _)| *t <= cost) {
            return;
        }
        self.reached.insert(node, (cost, via));
        self.open.push(Open {
            f: cost + Self::on(roads, leg, node),
            cell: node,
        });
    }

    /// Settle one node. `Some` once the quickest way is known: the way, or
    /// `None` when the roads within reach do not join the two ends.
    pub(super) fn step(
        &mut self,
        grid: &NavGrid,
        roads: &RoadNet,
        leg: Leg,
        closed: &BTreeSet<u32>,
        rejected: &[BTreeSet<u32>; 2],
        component: Option<&RouteSearch>,
    ) -> Option<Option<Way>> {
        grid.spend(1);
        if let Some(mut finding) = self.finding.take() {
            if !finding.step(grid, roads, leg, closed, rejected, component) {
                self.finding = Some(finding);
                return None;
            }
            if finding.needed[0] {
                self.starts = std::mem::take(&mut finding.selected[0]);
            }
            if finding.needed[1] {
                self.goals = std::mem::take(&mut finding.selected[1]);
            }
            self.goal_costs.resize(self.goals.len(), f64::INFINITY);
            self.initializing = 0;
        }
        if self.initializing < self.starts.len() + self.goals.len() {
            self.initialize(grid, roads, leg, component);
            return None;
        }
        if !self.across.is_finite() && self.best.is_none() {
            let needed = [
                !self.found_alternatives[0] && self.reached.is_empty(),
                !self.found_alternatives[1] && self.exits.is_empty(),
            ];
            if needed.iter().any(|needed| *needed) {
                for (found, needed) in self.found_alternatives.iter_mut().zip(needed) {
                    *found |= needed;
                }
                self.finding = Some(AccessSearch::new(needed, self.access_reach));
                return None;
            }
        }
        let Some(Open { f, cell: node }) = self.open.pop() else {
            return Some(self.best);
        };
        if self.best.is_some_and(|best| best.cost <= f) {
            return Some(self.best);
        }
        let (cost, _) = self.reached[&node];
        if f > cost + Self::on(roads, leg, node) {
            return None; // reached at lower cost since it was queued
        }
        if let Some(&(exit, goal)) = self.exits.get(&node) {
            let start = self.start_of(roads, node);
            self.offer(Way {
                cost: cost + exit,
                end: Some(node),
                start,
                goal,
            });
        }
        for &arc in roads.arcs_at(node) {
            if closed.contains(&arc) {
                continue;
            }
            let ends = roads.arc(arc).ends;
            let next = if ends[0] == node { ends[1] } else { ends[0] };
            let cost = cost + roads.arc(arc).length * pace(roads, arc, leg);
            self.reach(roads, leg, next, cost, Via::Arc(arc));
        }
        None
    }

    /// The nodes from the road's joining to `node`, and the arcs between.
    pub(super) fn back(&self, roads: &RoadNet, node: u32) -> (usize, Vec<u32>, Vec<u32>) {
        let (mut nodes, mut arcs) = (vec![node], Vec::new());
        loop {
            let at = *nodes.last().expect("a node");
            match self.reached[&at].1 {
                Via::Start(start) => {
                    nodes.reverse();
                    arcs.reverse();
                    return (start, nodes, arcs);
                }
                Via::Arc(arc) => {
                    let ends = roads.arc(arc).ends;
                    nodes.push(if ends[0] == at { ends[1] } else { ends[0] });
                    arcs.push(arc);
                }
            }
        }
    }

    pub(super) fn start_of(&self, roads: &RoadNet, node: u32) -> usize {
        self.back(roads, node).0
    }

    pub(super) fn crosses_bridge(&self, roads: &RoadNet, way: Way) -> bool {
        roads.arc(self.starts[way.start].arc).bridge
            || roads.arc(self.goals[way.goal].arc).bridge
            || way.end.is_some_and(|end| {
                self.back(roads, end)
                    .2
                    .iter()
                    .any(|arc| roads.arc(*arc).bridge)
            })
    }

    /// `way` as the runs to drive: each run's end and the arc it lies on,
    /// after the point the road is joined at. A mover joins and leaves the
    /// road at a slant: as far along it as the road was off to the side.
    /// And it keeps to the right of the road's middle, so two columns that
    /// meet on one road pass each other.
    pub(super) fn runs(
        &self,
        grid: &NavGrid,
        roads: &RoadNet,
        way: Way,
        leg: Leg,
    ) -> (V2, Vec<(V2, u32)>) {
        let m = leg.m;
        let (on, off) = (self.starts[way.start], self.goals[way.goal]);
        let mut line: Vec<(V2, u32)> = vec![(on.at, on.arc)];
        if let Some(end) = way.end {
            let (_, nodes, arcs) = self.back(roads, end);
            line.push((roads.node(nodes[0]), on.arc));
            for (node, arc) in nodes[1..].iter().zip(arcs) {
                line.push((roads.node(*node), arc));
            }
        }
        line.push((off.at, off.arc));
        // Each end slides toward its neighbour, at most half way when the
        // two ends are neighbours.
        let share = if line.len() == 2 { 0.5 } else { 1.0 };
        let slide = |from: V2, toward: V2, by: f64| {
            let span = (toward - from).length();
            if span > 0.0 {
                from + (toward - from) * ((by / span).min(share))
            } else {
                from
            }
        };
        let last = line.len() - 1;
        let (first, exit) = (line[0].0, line[last].0);
        line[0].0 = slide(first, line[1].0, on.distance + MERGE_M);
        line[last].0 = slide(exit, line[last - 1].0, off.distance + MERGE_M);
        if !self.across.is_finite() || !grid.terrain_clear(leg.from, line[0].0) {
            line[0].0 = first;
        }
        if !self.across.is_finite() || !grid.terrain_clear(line[last].0, leg.goal) {
            line[last].0 = exit;
        }
        let mut middle: Vec<(V2, u32)> = vec![line[0]];
        for &(point, arc) in &line[1..] {
            if (point - middle[middle.len() - 1].0).length() > JOIN_EPSILON_M {
                middle.push((point, arc));
            }
        }
        let lane = super::super::lanes::lane_line(&mut middle, roads, m.half_width_m, KEEP_RIGHT_M);
        let joined = lane[0];
        let runs = middle
            .iter()
            .zip(lane)
            .skip(1)
            .map(|(&(_, arc), point)| (point, arc))
            .collect();
        (joined, runs)
    }
}
