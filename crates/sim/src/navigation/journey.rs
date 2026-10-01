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
/// How far along a road a mover takes to get over to its side of it, or
/// back to the middle to leave it.
const MERGE_M: f64 = 32.0;
/// A run of road is checked this many metres a step.
const CHECK_M: f64 = 64.0;
/// A way round a body on the road is looked for from where the run was last
/// clear to this far along it.
const ROUND_M: f64 = 2.0 * CHECK_M;

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
    /// Its estimated time: straight to the road, along it, straight off it.
    time: f64,
    /// The last node before the road is left, and where it is left. No
    /// node: on at `start` and off at `goal` along one arc.
    end: Option<u32>,
    start: usize,
    goal: usize,
}

/// The quickest way over the road graph, a node a step.
struct RoadSearch {
    /// Where the road can be joined near the start, and left near the goal.
    starts: Vec<Access>,
    goals: Vec<Access>,
    open: BinaryHeap<Open>,
    /// Each node's time from the start and how it was reached.
    reached: BTreeMap<u32, (f64, Via)>,
    /// Each node the road can be left after: the time on to the goal, and
    /// where it is left.
    exits: BTreeMap<u32, (f64, usize)>,
    best: Option<Way>,
}

/// Seconds a metre of `arc` takes `m`.
fn pace(roads: &RoadNet, arc: u32, m: &Mobility) -> f64 {
    1.0 / m.speed(roads.arc(arc).factor, false, 0.0)
}

impl RoadSearch {
    /// `None` when no open road lies within reach of both ends.
    fn new(
        grid: &NavGrid,
        roads: &RoadNet,
        leg: Leg,
        rules: &NavigationRules,
        closed: &BTreeSet<u32>,
    ) -> Option<Self> {
        let near = |p: V2| -> Vec<Access> {
            roads
                .near(p, rules.road_access_m)
                .into_iter()
                .filter(|access| !closed.contains(&access.arc))
                .collect()
        };
        let (starts, goals) = (near(leg.from), near(leg.goal));
        grid.spend(1 + (starts.len() + goals.len()) as u64 / 8);
        if starts.is_empty() || goals.is_empty() {
            return None;
        }
        let join = |access: &Access| grid.across(leg.from, access.at, leg.m);
        let leave = |access: &Access| grid.across(access.at, leg.goal, leg.m);
        let mut search = RoadSearch {
            starts,
            goals,
            open: BinaryHeap::new(),
            reached: BTreeMap::new(),
            exits: BTreeMap::new(),
            best: None,
        };
        // The two ways along an arc from a point of it: to each end.
        let ends = |access: &Access| {
            let arc = roads.arc(access.arc);
            let pace = pace(roads, access.arc, leg.m);
            [
                (arc.ends[0], access.along * pace),
                (arc.ends[1], (arc.length - access.along) * pace),
            ]
        };
        // Where each arc is left for the goal (an arc has one nearest point).
        let mut left_at: BTreeMap<u32, usize> = BTreeMap::new();
        for (g, access) in search.goals.iter().enumerate() {
            left_at.insert(access.arc, g);
            for (node, time) in ends(access) {
                let time = time + leave(access);
                if search.exits.get(&node).is_none_or(|(t, _)| time < *t) {
                    search.exits.insert(node, (time, g));
                }
            }
        }
        for s in 0..search.starts.len() {
            let access = search.starts[s];
            let join = join(&access);
            for (node, time) in ends(&access) {
                search.reach(roads, leg, node, join + time, Via::Start(s));
            }
            // Off again along the same arc, never reaching a node.
            if let Some(&g) = left_at.get(&access.arc) {
                let exit = search.goals[g];
                let along = (exit.along - access.along).abs() * pace(roads, access.arc, leg.m);
                search.offer(Way {
                    time: join + along + leave(&exit),
                    end: None,
                    start: s,
                    goal: g,
                });
            }
        }
        Some(search)
    }

    fn offer(&mut self, way: Way) {
        if self.best.is_none_or(|best| way.time < best.time) {
            self.best = Some(way);
        }
    }

    /// No way on from `node` is quicker than the straight line at the
    /// mover's best speed.
    fn on(roads: &RoadNet, leg: Leg, node: u32) -> f64 {
        (roads.node(node) - leg.goal).length() / leg.m.max_speed()
    }

    fn reach(&mut self, roads: &RoadNet, leg: Leg, node: u32, time: f64, via: Via) {
        if self.reached.get(&node).is_some_and(|(t, _)| *t <= time) {
            return;
        }
        self.reached.insert(node, (time, via));
        self.open.push(Open {
            f: time + Self::on(roads, leg, node),
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
    ) -> Option<Option<Way>> {
        grid.spend(1);
        let Some(Open { f, cell: node }) = self.open.pop() else {
            return Some(self.best);
        };
        if self.best.is_some_and(|best| best.time <= f) {
            return Some(self.best);
        }
        let (time, _) = self.reached[&node];
        if f > time + Self::on(roads, leg, node) {
            return None; // reached more quickly since it was queued
        }
        if let Some(&(exit, goal)) = self.exits.get(&node) {
            let start = self.start_of(roads, node);
            self.offer(Way {
                time: time + exit,
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
            let time = time + roads.arc(arc).length * pace(roads, arc, leg.m);
            self.reach(roads, leg, next, time, Via::Arc(arc));
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

    /// `way` as the runs to drive: each run's end and the arc it lies on,
    /// after the point the road is joined at. A mover joins and leaves the
    /// road at a slant: as far along it as the road was off to the side.
    /// And it keeps to the right of the road's middle, so two columns that
    /// meet on one road pass each other.
    fn runs(&self, roads: &RoadNet, way: Way, m: &Mobility) -> (V2, Vec<(V2, u32)>) {
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
        let mut middle: Vec<(V2, u32)> = vec![line[0]];
        for &(point, arc) in &line[1..] {
            if (point - middle[middle.len() - 1].0).length() > 1e-6 {
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
                .is_none_or(|last| (*last - point).length() > 1e-6)
            {
                self.out.push(point);
            }
        }
    }
}

enum Stage {
    Roads(RoadSearch),
    Driving(Driving),
    Direct(RouteSearch),
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
    /// What its finished searches cost.
    work: SearchWork,
}

impl Journey {
    /// Start working out `leg`. `scratch` is a finished journey's
    /// bookkeeping to reuse.
    pub fn new(
        grid: &NavGrid,
        roads: &RoadNet,
        scratch: Option<Scratch>,
        leg: Leg,
        rules: &NavigationRules,
    ) -> Self {
        let mut journey = Journey {
            from: leg.from,
            goal: leg.goal,
            m: *leg.m,
            policy: leg.policy,
            avoid: leg.avoid.to_vec(),
            rules: *rules,
            scratch,
            stage: Stage::Done(Plan::Blocked(super::BlockReason::NoRoute)),
            closed: BTreeSet::new(),
            work: SearchWork::default(),
        };
        journey.stage = journey.by_road(grid, roads);
        journey
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
    fn by_road(&mut self, grid: &NavGrid, roads: &RoadNet) -> Stage {
        let search = (self.policy == RoutePolicy::Fastest)
            .then(|| RoadSearch::new(grid, roads, self.leg(), &self.rules, &self.closed))
            .flatten();
        match search {
            Some(search) => Stage::Roads(search),
            None => self.direct(grid),
        }
    }

    /// One search across the grid for the whole leg.
    fn direct(&mut self, grid: &NavGrid) -> Stage {
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

    /// Take a finished search's plan, keeping its bookkeeping and its cost.
    fn finished(&mut self, search: RouteSearch) -> Plan {
        let cost = search.work();
        self.work.cells += cost.cells;
        self.work.queued += cost.queued;
        self.work.expanded += cost.expanded;
        self.work.stale += cost.stale;
        self.work.heap_peak = self.work.heap_peak.max(cost.heap_peak);
        let (plan, scratch) = search.finish();
        self.scratch = Some(scratch);
        plan.expect("a finished search has a plan")
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
            Stage::Driving(driving) => (None, driving.search.map(|s| s.finish().1)),
            Stage::Roads(_) => (None, None),
        };
        (plan, held.or(self.scratch))
    }

    /// How far the journey has got. With what was asked, the grid and the
    /// road graph, that fixes everything it will do.
    pub fn digest(&self, d: &mut Digest) {
        d.u64(self.closed.len() as u64);
        for arc in &self.closed {
            d.u64(*arc as u64);
        }
        match &self.stage {
            Stage::Roads(search) => {
                d.u64(0)
                    .u64(search.open.len() as u64)
                    .u64(search.reached.len() as u64);
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
            Stage::Direct(mut search) => {
                search.advance(grid, 1);
                if search.plan().is_some() {
                    Stage::Done(self.finished(search))
                } else {
                    Stage::Direct(search)
                }
            }
            Stage::Roads(mut search) => {
                match search.step(grid, roads, self.leg(), &self.closed) {
                    None => Stage::Roads(search),
                    Some(way) => {
                        // By road only if that beats the straight line
                        // across country, both judged as the crow flies
                        // over the ground they cross.
                        let across = grid.across(self.from, self.goal, &self.m);
                        match way.filter(|way| way.time < across) {
                            Some(way) => {
                                let (joined, runs) = search.runs(roads, way, &self.m);
                                Stage::Driving(Driving {
                                    joined,
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
        };
    }

    /// One step of making the way by road physical.
    fn drive(&mut self, grid: &NavGrid, roads: &RoadNet, mut driving: Driving) -> Stage {
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
                        .is_some_and(|end| (*end - sought).length() < 1e-6)
            };
            return match self.finished(search) {
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
                    self.by_road(grid, roads)
                }
                _ => self.direct(grid),
            };
        }
        if !on_road {
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
