//! A way by road made physical on the side's grid: the search to the road,
//! each run of road checked for the mover's footprint and moved over or
//! searched round where a body stands in it, and the search from the road
//! to the goal.
use contract::command::RoutePolicy;

use super::super::roads::RoadNet;
use super::super::search::RouteSearch;
use super::super::{Leg, Mobility, Mover, NavGrid, Plan};
use super::{Journey, Stage, JOIN_EPSILON_M, KEEP_RIGHT_M, MERGE_M};
use crate::digest::Digest;
use crate::math::V2;

/// A run of road is checked this many metres a step.
const CHECK_M: f64 = 64.0;
/// A way round a body on the road is looked for from where the run was last
/// clear to this far along it.
const ROUND_M: f64 = 2.0 * CHECK_M;
/// Past a body in or beside its lane a mover moves over to another line of
/// the road, reached this far along: the longest of these that is clear, so
/// it moves over early and gently, and sharply only when the body is close.
const SHIFT_REACH_M: [f64; 3] = [MERGE_M, MERGE_M / 2.0, MERGE_M / 4.0];
/// The lines tried: its own lane first, then this many evenly spaced ones
/// leftward, the last with the mover's middle as near the far edge of the
/// road as its own lane is to the near one.
const LANE_SHIFTS: usize = 6;

/// A candidate road line, including its return stretch when rejoining.
/// Each segment and exact-body query keeps its cursor between advances.
pub(super) struct LaneCheck {
    pub(super) points: Vec<V2>,
    pub(super) at: usize,
    pub(super) probe: super::super::Probe,
    pub(super) shove: bool,
}
impl LaneCheck {
    pub(super) fn new(points: Vec<V2>, m: &Mobility, shove: bool) -> Self {
        let probe = super::super::Probe::new(points[0], points[1], m);
        Self {
            points,
            at: 1,
            probe,
            shove,
        }
    }
    pub(super) fn step(&mut self, grid: &NavGrid, who: Mover) -> Option<bool> {
        match grid.read(&mut self.probe, who, RoutePolicy::Fastest, self.shove) {
            None => None,
            Some(None) => Some(false),
            Some(Some(_)) => {
                self.at += 1;
                if self.at == self.points.len() {
                    Some(true)
                } else {
                    self.probe = super::super::Probe::new(
                        self.points[self.at - 1],
                        self.points[self.at],
                        who.m,
                    );
                    None
                }
            }
        }
    }
    pub(super) fn digest(&self, d: &mut Digest) {
        d.u64(self.at as u64)
            .u64(self.shove as u64)
            .u64(self.points.len() as u64);
        for p in &self.points {
            d.f64(p.x).f64(p.y);
        }
        self.probe.digest(d);
    }
}

/// A way by road being made physical on the grid.
pub(super) struct Driving {
    /// Where the road is joined, then each run's end and arc.
    pub(super) joined: V2,
    pub(super) accesses: [u32; 2],
    pub(super) runs: Vec<(V2, u32)>,
    /// The route so far.
    pub(super) out: Vec<V2>,
    /// The piece in hand: 0 the way to the road, then each run, then the
    /// way from the road to the goal.
    pub(super) piece: usize,
    /// Metres of the run in hand found clear.
    pub(super) checked: f64,
    /// Where the route stands that far along the run, when it has moved
    /// over from the run's lane, and how many shifts over that is.
    pub(super) off_lane: Option<(V2, usize)>,
    /// The search in hand: to or from the road, or round a body on it.
    pub(super) search: Option<RouteSearch>,
    /// Where the search in hand was sent, and how many shifts off the
    /// run's lane that is.
    pub(super) sought: (V2, usize),
    /// Next alternative lane to check; one is checked per planning step.
    pub(super) candidate: usize,
    pub(super) probe: Option<LaneCheck>,
}

impl Driving {
    /// Add waypoints to the route, but never the place it already ends at
    /// (a run's end is the next piece's start).
    pub(super) fn extend(&mut self, points: impl IntoIterator<Item = V2>) {
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

impl Journey {
    /// One step of making the way by road physical.
    pub(super) fn drive(
        &mut self,
        grid: &NavGrid,
        roads: &RoadNet,
        mut driving: Box<Driving>,
    ) -> Stage {
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
        // The lines of a run of road from `a` to `b`: the point `s` metres
        // along its lane and `k` shifts to the left of it. The last shift
        // leaves the mover's middle as far inside the road's far edge as
        // its lane is inside the near one.
        let half_width = self.m.half_width_m;
        let lines = |a: V2, b: V2, arc: u32| {
            let span = (b - a).length();
            let edge = (roads.arc(arc).half_width - KEEP_RIGHT_M).max(0.0);
            let room = (half_width + KEEP_RIGHT_M).min(edge) + edge;
            let left = crate::math::v2(a.y - b.y, b.x - a.x) * (room / span / LANE_SHIFTS as f64);
            move |s: f64, k: usize| a + (b - a) * (s.min(span) / span) + left * k as f64
        };
        if let Some(mut search) = driving.search.take() {
            search.advance(grid, 1);
            if search.plan().is_none() {
                driving.search = Some(search);
                return Stage::Driving(driving);
            }
            // A search that could not stand where it was sent ends somewhere
            // else, and has not reached the road; only the leg's own goal
            // may be moved to standing room.
            let (sought, shift) = driving.sought;
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
                        driving.off_lane = (shift > 0).then_some((sought, shift));
                        Stage::Driving(driving)
                    }
                }
                // Nothing gets along this run of road: ask the graph again
                // without its arc. With no way to or from the road at all,
                // search the grid for the whole leg.
                _ if on_road => {
                    self.closed.insert(driving.runs[driving.piece - 1].1);
                    Stage::Terrain(super::super::terrain::Probe::new(self.from, self.goal))
                }
                _ => {
                    let side = usize::from(driving.piece != 0);
                    self.rejected[side].insert(driving.accesses[side]);
                    Stage::Terrain(super::super::terrain::Probe::new(self.from, self.goal))
                }
            };
        }
        let who = Mover {
            m: &self.m,
            avoid: &self.avoid,
        };
        let shifts = LANE_SHIFTS;
        if !on_road {
            // From where the route stands, if it left the road off its lane.
            let from = driving.off_lane.map_or(from, |(at, _)| at);
            if driving.piece + 1 == pieces && !self.goal_probed {
                self.goal_probed = true;
                let scratch = self.scratch.take();
                let leg = Leg {
                    from,
                    goal: self.goal,
                    ..self.leg()
                };
                let limit = self.rules.search_limit(length);
                return match RouteSearch::goal_probe(grid, scratch, leg, limit) {
                    Some(search) => Stage::GoalProof(search, driving),
                    None => Stage::Done(Plan::Blocked(super::super::BlockReason::NoRoute)),
                };
            }
            if driving.piece + 1 == pieces
                && self
                    .goal_component
                    .as_ref()
                    .is_some_and(|component| component.exhausted_rejects(grid, from))
            {
                self.rejected[1].insert(driving.accesses[1]);
                return Stage::Terrain(super::super::terrain::Probe::new(self.from, self.goal));
            }
            // The road is joined on the first line of its first run the
            // mover can stand on.
            driving.sought = match driving.runs.first() {
                Some(&(next, arc)) if driving.piece == 0 && (next - to).length() > 0.0 => {
                    let line = lines(to, next, arc);
                    (0..=shifts)
                        .map(|k| (line(0.0, k), k))
                        .find(|(p, _)| grid.stands(*p, who))
                        .unwrap_or((to, 0))
                }
                _ => (to, 0),
            };
            driving.search = Some(self.search(grid, from, driving.sought.0));
            return Stage::Driving(driving);
        }
        // A run of road: check the next stretch for the mover's footprint.
        let line = lines(from, to, driving.runs[driving.piece - 1].1);
        let (here, held) = driving
            .off_lane
            .unwrap_or_else(|| (line(driving.checked, 0), 0));
        let ahead = driving.checked + CHECK_M;
        let back = driving.checked + MERGE_M;
        // Candidate zero stays on the lane, or rejoins it only after the
        // next full stretch proves clear. Other candidates shift left,
        // gently first, and only shove after every clear line failed.
        let candidate = if driving.candidate == 0 {
            Some((false, if held == 0 { ahead } else { back }, 0))
        } else {
            let reaches = SHIFT_REACH_M.map(|reach| driving.checked + reach);
            [false, true]
                .into_iter()
                .flat_map(|shove| (0..reaches.len()).map(move |n| (shove, n)))
                .filter(|(_, n)| *n == 0 || reaches[*n] < length)
                .flat_map(|(shove, n)| {
                    [held]
                        .into_iter()
                        .chain((0..=shifts).filter(move |k| *k != held))
                        .map(move |k| (shove, reaches[n], k))
                })
                .nth(driving.candidate - 1)
        };
        let Some((shove, reach, shift)) = candidate else {
            // No line is free: search round the body to a standing place
            // farther down the road.
            driving.extend([here]);
            let rejoin = driving.checked + ROUND_M;
            driving.sought = (0..=shifts)
                .map(|k| (line(rejoin, k), k))
                .find(|(p, _)| grid.stands(*p, who))
                .unwrap_or((line(rejoin, 0), 0));
            driving.search = Some(self.search(grid, here, driving.sought.0));
            driving.candidate = 0;
            return Stage::Driving(driving);
        };
        let target = line(reach, shift);
        if driving.probe.is_none() {
            if !grid.stands(target, who) {
                grid.spend(1);
                driving.candidate += 1;
                return Stage::Driving(driving);
            }
            let mut points = vec![here, target];
            if driving.candidate == 0 && held > 0 {
                points.push(line(back + CHECK_M, 0));
            }
            driving.probe = Some(LaneCheck::new(points, &self.m, shove));
        }
        match driving.probe.as_mut().unwrap().step(grid, who) {
            None => return Stage::Driving(driving),
            Some(false) => {
                driving.probe = None;
                driving.candidate += 1;
                return Stage::Driving(driving);
            }
            Some(true) => {
                driving.probe = None;
            }
        }
        if driving.candidate > 0 || held > 0 {
            driving.extend([here, target]);
        }
        driving.candidate = 0;
        driving.off_lane = (shift > 0).then(|| (line(reach, shift), shift));
        driving.checked = reach;
        if driving.checked >= length {
            driving.extend([driving.off_lane.map_or(to, |(at, _)| at)]);
            driving.piece += 1;
            driving.checked = 0.0;
        }
        Stage::Driving(driving)
    }
}
