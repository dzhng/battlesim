//! The route for one leg of a move, worked out in steps. A leg by the
//! fastest policy first asks the road graph for the quickest way by road
//! ([`RoadNet`]). If that beats driving straight across country, the way is
//! made physical on the side's grid: a search to the road, each run of road
//! checked for the mover's footprint (and searched round where a known body
//! stands on it), and a search from the road to the goal. A run nothing
//! gets past closes its arc and the graph is asked again. With no road
//! worth taking, or by the shortest policy, the leg is one search across
//! the grid ([`RouteSearch`]). Every step is counted work, so a battle
//! spends a share of a tick on it.
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};

use contract::command::RoutePolicy;
use contract::scenario::NavigationRules;

use super::roads::{Access, RoadNet};
use super::search::{Open, RouteSearch, Scratch, SearchWork};
use super::{Leg, Mobility, Mover, NavGrid, Plan};
use crate::digest::Digest;
use crate::math::{Obb2, V2};

/// The room a mover on a road leaves between itself and the road's middle,
/// and between its own middle and the road's edge.
const KEEP_RIGHT_M: f64 = 0.5;
/// Points this close are coalesced when constructing a road run.
const JOIN_EPSILON_M: f64 = 1e-6;
/// How far along a road a mover takes to get over to its side of it, or
/// back to the middle to leave it.
const MERGE_M: f64 = 32.0;
/// A run of road is checked this many metres a step.
const CHECK_M: f64 = 64.0;
/// A way round a body on the road is looked for from where the run was last
/// clear to this far along it.
const ROUND_M: f64 = 2.0 * CHECK_M;

/// The final centre slides along its goal arc, then shifts by one lane
/// width. Source snapping and point coalescing cover every admitted cell.
fn goal_access_outside(
    roads: &RoadNet,
    leg: Leg,
    component: Option<&RouteSearch>,
    arc: u32,
) -> bool {
    let Some(component) = component else {
        return false;
    };
    let arc = roads.arc(arc);
    let padding = super::START_REACH_M + (leg.m.half_width_m + KEEP_RIGHT_M).abs() + JOIN_EPSILON_M;
    component.excludes_segment(roads.node(arc.ends[0]), roads.node(arc.ends[1]), padding)
}

/// How a node of the road graph was reached: from one of the leg's ways
/// onto the road, or along an arc.
#[derive(Clone, Copy)]
enum Via {
    Start(usize),
    Arc(u32),
}

/// A whole way by road, start to goal.
#[derive(Clone, Copy)]
struct Way {
    /// Its estimated cost: straight to the road, along it, straight off it.
    cost: f64,
    /// The last node before the road is left, and where it is left. No
    /// node: on at `start` and off at `goal` along one arc.
    end: Option<u32>,
    start: usize,
    goal: usize,
}

/// The best road journey under the requested policy, a node a step.
struct RoadSearch {
    /// Where the road can be joined near the start, and left near the goal.
    starts: Vec<Access>,
    goals: Vec<Access>,
    open: BinaryHeap<Open>,
    /// Each node's cost from the start and how it was reached.
    reached: BTreeMap<u32, (f64, Via)>,
    /// Each node the road can be left after: the cost on to the goal, and
    /// where it is left.
    exits: BTreeMap<u32, (f64, usize)>,
    best: Option<Way>,
    initializing: usize,
    left_at: BTreeMap<u32, usize>,
    goal_costs: Vec<f64>,
    across: f64,
    connector: Option<super::terrain::Probe>,
    /// Arc cursor and nearest access at each missing endpoint. A missing
    /// nearby road cannot send a river crossing straight back to the grid.
    finding: Option<AccessSearch>,
    found_alternatives: [bool; 2],
    access_reach: f64,
}

/// Discover the nearest terrain-reachable road for an exhausted endpoint.
/// One arc/endpoint is admitted per step; its terrain probe also yields.
struct AccessSearch {
    arc: usize,
    side: usize,
    needed: [bool; 2],
    nearest: [Option<Access>; 2],
    candidates: [Vec<Access>; 2],
    selected: [Vec<Access>; 2],
    selecting: (usize, usize),
    reach: f64,
    probe: Option<(Access, super::terrain::Probe)>,
}

impl AccessSearch {
    fn new(needed: [bool; 2], reach: f64) -> Self {
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

    fn next(&mut self) {
        self.side += 1;
        if self.side == 2 {
            self.side = 0;
            self.arc += 1;
        }
    }

    fn step(
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
            self.probe = Some((access, super::terrain::Probe::new(point, access.at)));
        }
        false
    }

    fn digest(&self, d: &mut Digest) {
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
fn pace(roads: &RoadNet, arc: u32, leg: Leg) -> f64 {
    match leg.policy {
        RoutePolicy::Shortest => 1.0,
        RoutePolicy::Fastest => 1.0 / leg.m.speed(roads.arc(arc).factor, false, 0.0),
    }
}

fn over_ground(grid: &NavGrid, a: V2, b: V2, leg: Leg) -> f64 {
    match leg.policy {
        RoutePolicy::Shortest => (b - a).length(),
        RoutePolicy::Fastest => grid.across(a, b, leg.m),
    }
}

impl RoadSearch {
    /// A blocked straight leg may need a road beyond the usual access
    /// radius. Missing endpoint accesses are found one arc per step.
    fn new(
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
    fn initialize(
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
            .get_or_insert_with(|| super::terrain::Probe::new(a, b));
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

    fn offer(&mut self, way: Way) {
        if self.best.is_none_or(|best| way.cost < best.cost) {
            self.best = Some(way);
        }
    }

    /// A lower bound on remaining distance, or time at the mover's best
    /// speed, under the requested policy.
    fn on(roads: &RoadNet, leg: Leg, node: u32) -> f64 {
        let distance = (roads.node(node) - leg.goal).length();
        match leg.policy {
            RoutePolicy::Shortest => distance,
            RoutePolicy::Fastest => distance / leg.m.max_speed(),
        }
    }

    fn reach(&mut self, roads: &RoadNet, leg: Leg, node: u32, cost: f64, via: Via) {
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
    fn step(
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
    fn back(&self, roads: &RoadNet, node: u32) -> (usize, Vec<u32>, Vec<u32>) {
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

    fn start_of(&self, roads: &RoadNet, node: u32) -> usize {
        self.back(roads, node).0
    }

    fn crosses_bridge(&self, roads: &RoadNet, way: Way) -> bool {
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
    fn runs(&self, grid: &NavGrid, roads: &RoadNet, way: Way, leg: Leg) -> (V2, Vec<(V2, u32)>) {
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
        // Each run's lane: its right-hand side, as far over as leaves
        // `KEEP_RIGHT_M` between the mover and the middle, but with its own
        // middle still on the road (so on a narrow one a wide mover
        // straddles the middle). Where two runs meet, their lanes do.
        let lanes: Vec<V2> = middle
            .windows(2)
            .map(|run| {
                let along = run[1].0 - run[0].0;
                let right = crate::math::v2(along.y, -along.x) * (1.0 / along.length());
                let edge = (roads.arc(run[1].1).half_width - KEEP_RIGHT_M).max(0.0);
                let over = (m.half_width_m + KEEP_RIGHT_M).min(edge);
                right * over
            })
            .collect();
        let lane = |k: usize| match (k.checked_sub(1).map(|k| lanes[k]), lanes.get(k)) {
            (Some(a), Some(&b)) => {
                // The corner both lanes share, unless the road doubles back
                // (or has no width to keep to one side of).
                let widths = a.length() * b.length();
                let turn = 1.0 + a.dot(b) / widths;
                if widths > 0.0 && turn > 0.5 {
                    (a + b) * (1.0 / turn)
                } else {
                    b
                }
            }
            (Some(lane), None) | (None, Some(&lane)) => lane,
            (None, None) => V2::default(),
        };
        let joined = middle[0].0 + lane(0);
        let runs = middle
            .iter()
            .enumerate()
            .skip(1)
            .map(|(k, &(point, arc))| (point + lane(k), arc))
            .collect();
        (joined, runs)
    }
}

/// A way by road being made physical on the grid.
struct Driving {
    /// Where the road is joined, then each run's end and arc.
    joined: V2,
    accesses: [u32; 2],
    runs: Vec<(V2, u32)>,
    /// The route so far.
    out: Vec<V2>,
    /// The piece in hand: 0 the way to the road, then each run, then the
    /// way from the road to the goal.
    piece: usize,
    /// Metres of the run in hand found clear.
    checked: f64,
    /// The search in hand: to or from the road, or round a body on it.
    search: Option<RouteSearch>,
}

impl Driving {
    /// Add waypoints to the route, but never the place it already ends at
    /// (a run's end is the next piece's start).
    fn extend(&mut self, points: impl IntoIterator<Item = V2>) {
        for point in points {
            if self
                .out
                .last()
                .is_none_or(|last| (*last - point).length() > JOIN_EPSILON_M)
            {
                self.out.push(point);
            }
        }
    }
}

enum Stage {
    Terrain(super::terrain::Probe),
    Roads(RoadSearch),
    Driving(Driving),
    Direct(RouteSearch),
    /// One rare-failure proof, before trying further goal road accesses.
    GoalProof(RouteSearch, u32),
    Done(Plan),
}

/// The route for one leg, in progress. [`Journey::advance`] spends work on
/// it until it has a [`Plan`].
pub struct Journey {
    from: V2,
    goal: V2,
    m: Mobility,
    policy: RoutePolicy,
    avoid: Vec<Obb2>,
    rules: NavigationRules,
    /// Search bookkeeping, while no search of this journey holds it.
    scratch: Option<Scratch>,
    stage: Stage,
    /// Arcs this journey found it cannot drive: a known body closes them
    /// for this mover and this side only.
    closed: BTreeSet<u32>,
    /// Failed connector access at each endpoint; the road itself stays open.
    rejected: [BTreeSet<u32>; 2],
    /// What its finished searches cost.
    work: SearchWork,
    goal_probed: bool,
    /// An exhausted strict component for final off-road connectors.
    goal_component: Option<RouteSearch>,
}

impl Journey {
    /// Start working out `leg`. `scratch` is a finished journey's
    /// bookkeeping to reuse.
    pub fn new(
        _grid: &NavGrid,
        _roads: &RoadNet,
        scratch: Option<Scratch>,
        leg: Leg,
        rules: &NavigationRules,
    ) -> Self {
        Journey {
            from: leg.from,
            goal: leg.goal,
            m: *leg.m,
            policy: leg.policy,
            avoid: leg.avoid.to_vec(),
            rules: *rules,
            scratch,
            stage: Stage::Terrain(super::terrain::Probe::new(leg.from, leg.goal)),
            closed: BTreeSet::new(),
            rejected: Default::default(),
            work: SearchWork::default(),
            goal_probed: false,
            goal_component: None,
        }
    }

    fn leg(&self) -> Leg<'_> {
        Leg {
            from: self.from,
            goal: self.goal,
            m: &self.m,
            policy: self.policy,
            avoid: &self.avoid,
        }
    }

    /// Ask the road graph, if this leg goes by road at all.
    fn by_road(&mut self, grid: &NavGrid, roads: &RoadNet, direct: bool) -> Stage {
        let across = if direct {
            over_ground(grid, self.from, self.goal, self.leg())
        } else {
            f64::INFINITY
        };
        let search = (self.policy == RoutePolicy::Fastest || !direct)
            .then(|| {
                RoadSearch::new(
                    grid,
                    roads,
                    self.leg(),
                    &self.rules,
                    &self.closed,
                    &self.rejected,
                    across,
                )
            })
            .flatten();
        match search {
            Some(search) => Stage::Roads(search),
            None => self.direct(grid),
        }
    }

    /// One search across the grid for the whole leg.
    fn direct(&mut self, grid: &NavGrid) -> Stage {
        if self
            .goal_component
            .as_ref()
            .is_some_and(|component| component.exhausted_rejects(grid, self.from))
        {
            return Stage::Done(Plan::Blocked(super::BlockReason::NoRoute));
        }
        Stage::Direct(self.search(grid, self.from, self.goal))
    }

    /// A search of the grid between two points, as this leg's mover.
    fn search(&mut self, grid: &NavGrid, from: V2, goal: V2) -> RouteSearch {
        let scratch = self.scratch.take();
        let leg = Leg {
            from,
            goal,
            ..self.leg()
        };
        let limit = self.rules.search_limit((goal - from).length());
        RouteSearch::new(grid, scratch, leg, limit)
    }

    /// Add a completed or canceled search's counted cost.
    fn record_search(&mut self, search: &RouteSearch) {
        let cost = search.work();
        self.work.cells += cost.cells;
        self.work.queued += cost.queued;
        self.work.expanded += cost.expanded;
        self.work.stale += cost.stale;
        self.work.heap_peak = self.work.heap_peak.max(cost.heap_peak);
    }

    fn finish_search(&mut self, search: RouteSearch) -> Option<Plan> {
        self.record_search(&search);
        let (plan, scratch) = search.finish();
        self.scratch = Some(scratch);
        plan
    }

    /// Spend up to `allowance` work on the journey, and say how much was
    /// spent: at most one step more than the allowance.
    pub fn advance(&mut self, grid: &NavGrid, roads: &RoadNet, allowance: u64) -> u64 {
        let before = grid.work();
        while !matches!(self.stage, Stage::Done(_)) && grid.work() - before < allowance {
            self.step(grid, roads);
        }
        grid.work() - before
    }

    /// The plan, once the journey has one.
    pub fn plan(&self) -> Option<&Plan> {
        match &self.stage {
            Stage::Done(plan) => Some(plan),
            _ => None,
        }
    }

    /// What the journey's searches of the grid have cost so far.
    pub fn work(&self) -> SearchWork {
        self.work
    }

    /// The plan if the journey reached one, and its bookkeeping for the
    /// next journey to reuse.
    pub fn finish(self) -> (Option<Plan>, Option<Scratch>) {
        let (plan, held) = match self.stage {
            Stage::Done(plan) => (Some(plan), None),
            Stage::Direct(search) => (None, Some(search.finish().1)),
            Stage::GoalProof(search, _) => (None, Some(search.finish().1)),
            Stage::Driving(driving) => (None, driving.search.map(|s| s.finish().1)),
            Stage::Roads(_) | Stage::Terrain(_) => (None, None),
        };
        (
            plan,
            held.or(self.scratch)
                .or_else(|| self.goal_component.map(|search| search.finish().1)),
        )
    }

    /// How far the journey has got. With what was asked, the grid and the
    /// road graph, that fixes everything it will do.
    pub fn digest(&self, d: &mut Digest) {
        if self.goal_probed {
            d.u64(u64::MAX).u64(self.goal_component.is_some() as u64);
            if let Some(component) = &self.goal_component {
                component.digest(d);
            }
        }
        d.u64(self.closed.len() as u64);
        for arc in &self.closed {
            d.u64(*arc as u64);
        }
        for rejected in &self.rejected {
            d.u64(rejected.len() as u64);
            for arc in rejected {
                d.u64(*arc as u64);
            }
        }
        match &self.stage {
            Stage::Terrain(probe) => {
                d.u64(4);
                probe.digest(d);
            }
            Stage::Roads(search) => {
                d.u64(0)
                    .u64(search.open.len() as u64)
                    .u64(search.reached.len() as u64)
                    .u64(search.initializing as u64);
                d.u64(search.connector.is_some() as u64);
                if let Some(probe) = &search.connector {
                    probe.digest(d);
                }
                d.u64(search.finding.is_some() as u64);
                for found in search.found_alternatives {
                    d.u64(found as u64);
                }
                if let Some(finding) = &search.finding {
                    finding.digest(d);
                }
            }
            Stage::Driving(driving) => {
                d.u64(1)
                    .u64(driving.piece as u64)
                    .f64(driving.checked)
                    .u64(driving.out.len() as u64)
                    .u64(driving.search.is_some() as u64);
                if let Some(search) = &driving.search {
                    search.digest(d);
                }
            }
            Stage::Direct(search) => {
                d.u64(2);
                search.digest(d);
            }
            Stage::GoalProof(search, access) => {
                d.u64(5).u64(*access as u64);
                search.digest(d);
            }
            Stage::Done(_) => {
                d.u64(3);
            }
        }
    }

    fn step(&mut self, grid: &NavGrid, roads: &RoadNet) {
        let stage = std::mem::replace(
            &mut self.stage,
            Stage::Done(Plan::Blocked(super::BlockReason::NoRoute)),
        );
        self.stage = match stage {
            Stage::Done(plan) => Stage::Done(plan),
            Stage::Terrain(mut probe) => match grid.terrain_step(&mut probe) {
                Some(direct) => self.by_road(grid, roads, direct),
                None => Stage::Terrain(probe),
            },
            Stage::Direct(mut search) => {
                search.advance(grid, 1);
                if search.plan().is_some() {
                    Stage::Done(
                        self.finish_search(search)
                            .expect("a finished search has a plan"),
                    )
                } else {
                    Stage::Direct(search)
                }
            }
            Stage::Roads(mut search) => {
                match search.step(
                    grid,
                    roads,
                    self.leg(),
                    &self.closed,
                    &self.rejected,
                    self.goal_component.as_ref(),
                ) {
                    None => Stage::Roads(search),
                    Some(way) => {
                        // Compare both candidates under the same policy.
                        // Blocked straight terrain has infinite baseline cost.
                        let across = search.across;
                        match way.filter(|way| {
                            way.cost < across
                                && (self.policy == RoutePolicy::Fastest
                                    || search.crosses_bridge(roads, *way))
                        }) {
                            Some(way) => {
                                let (joined, runs) = search.runs(grid, roads, way, self.leg());
                                Stage::Driving(Driving {
                                    joined,
                                    accesses: [
                                        search.starts[way.start].arc,
                                        search.goals[way.goal].arc,
                                    ],
                                    runs,
                                    out: Vec::new(),
                                    piece: 0,
                                    checked: 0.0,
                                    search: None,
                                })
                            }
                            None => self.direct(grid),
                        }
                    }
                }
            }
            Stage::Driving(driving) => self.drive(grid, roads, driving),
            Stage::GoalProof(mut search, access) => {
                search.advance(grid, 1);
                if search.reached_target() || search.plan().is_some() {
                    if search.exhausted() {
                        self.record_search(&search);
                        self.goal_component = Some(search);
                    } else {
                        self.finish_search(search);
                    }
                    self.rejected[1].insert(access);
                    Stage::Terrain(super::terrain::Probe::new(self.from, self.goal))
                } else {
                    Stage::GoalProof(search, access)
                }
            }
        };
    }

    /// One step of making the way by road physical.
    fn drive(&mut self, grid: &NavGrid, _roads: &RoadNet, mut driving: Driving) -> Stage {
        let pieces = driving.runs.len() + 2;
        // Where the piece in hand starts and ends: a run of road, or the
        // way between the road and one end of the leg.
        let point = |k: usize| match k {
            0 => driving.joined,
            k => driving.runs[k - 1].0,
        };
        let (from, to) = match driving.piece {
            0 => (self.from, point(0)),
            piece if piece + 1 == pieces => (point(piece - 1), self.goal),
            piece => (point(piece - 1), point(piece)),
        };
        let on_road = driving.piece > 0 && driving.piece + 1 < pieces;
        let length = (to - from).length();
        if let Some(mut search) = driving.search.take() {
            search.advance(grid, 1);
            if search.plan().is_none() {
                driving.search = Some(search);
                return Stage::Driving(driving);
            }
            // Where the search was sent. A search that could not stand
            // there ends somewhere else, and has not reached the road; only
            // the leg's own goal may be moved to standing room.
            let sought = if on_road {
                from + (to - from) * ((driving.checked + ROUND_M).min(length) / length)
            } else {
                to
            };
            let arrives = |route: &[V2]| {
                driving.piece + 1 == pieces
                    || route
                        .last()
                        .is_some_and(|end| (*end - sought).length() < JOIN_EPSILON_M)
            };
            return match self
                .finish_search(search)
                .expect("a finished search has a plan")
            {
                Plan::Route(route) if arrives(&route) => {
                    driving.extend(route);
                    // A way round a body rejoins the run further along it.
                    driving.checked += ROUND_M;
                    if !on_road || driving.checked >= length {
                        driving.piece += 1;
                        driving.checked = 0.0;
                    }
                    if driving.piece == pieces {
                        Stage::Done(Plan::Route(driving.out))
                    } else {
                        Stage::Driving(driving)
                    }
                }
                // Nothing gets along this run of road: ask the graph again
                // without its arc. With no way to or from the road at all,
                // search the grid for the whole leg.
                _ if on_road => {
                    self.closed.insert(driving.runs[driving.piece - 1].1);
                    Stage::Terrain(super::terrain::Probe::new(self.from, self.goal))
                }
                _ => {
                    let side = usize::from(driving.piece != 0);
                    if side == 1 && !self.goal_probed {
                        self.goal_probed = true;
                        let leg = Leg {
                            from,
                            goal: self.goal,
                            ..self.leg()
                        };
                        let limit = self.rules.search_limit(length);
                        return match RouteSearch::goal_probe(grid, leg, limit) {
                            Some(search) => Stage::GoalProof(search, driving.accesses[side]),
                            None => Stage::Done(Plan::Blocked(super::BlockReason::NoRoute)),
                        };
                    }
                    self.rejected[side].insert(driving.accesses[side]);
                    Stage::Terrain(super::terrain::Probe::new(self.from, self.goal))
                }
            };
        }
        if !on_road {
            if driving.piece + 1 == pieces
                && self
                    .goal_component
                    .as_ref()
                    .is_some_and(|component| component.exhausted_rejects(grid, from))
            {
                self.rejected[1].insert(driving.accesses[1]);
                return Stage::Terrain(super::terrain::Probe::new(self.from, self.goal));
            }
            driving.search = Some(self.search(grid, from, to));
            return Stage::Driving(driving);
        }
        // A run of road: check the next stretch for the mover's footprint.
        let along = |m: f64| from + (to - from) * (m.min(length) / length);
        let who = Mover {
            m: &self.m,
            avoid: &self.avoid,
        };
        let stretch = (along(driving.checked), along(driving.checked + CHECK_M));
        if grid
            .segment_cost(stretch.0, stretch.1, who, RoutePolicy::Fastest)
            .is_none()
        {
            // A known body stands on this stretch: leave the road where it
            // was last clear and look for a way back onto it past the body.
            driving.extend([stretch.0]);
            driving.search = Some(self.search(grid, stretch.0, along(driving.checked + ROUND_M)));
            return Stage::Driving(driving);
        }
        driving.checked += CHECK_M;
        if driving.checked >= length {
            driving.extend([to]);
            driving.piece += 1;
            driving.checked = 0.0;
        }
        Stage::Driving(driving)
    }
}
