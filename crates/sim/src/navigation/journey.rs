//! The route for one leg of a move, worked out in steps. A leg by the
//! fastest policy first asks the road graph for the quickest way by road
//! ([`RoadNet`]). If that beats driving straight across country, the way is
//! made physical on the side's grid: a search to the road, each run of road
//! checked for the mover's footprint, and a search from the road to the
//! goal. A mover's lane is the free width of the road: where a known body
//! stands in or beside its own side of it, the mover moves over toward the
//! middle (and past it, while its own middle stays on the road) and comes
//! back to its side after the body, starting over back along the stretch it
//! found clear when it cannot from that stretch's end; only a road with no
//! free width is searched round. A run nothing gets past closes its arc and
//! the graph is asked again. With no road worth taking, or by the shortest
//! policy, the leg is one search across the grid ([`RouteSearch`]). A
//! wheeled mover's route then has its corners rounded to what it can drive
//! ([`Rounding`]). Every step is counted work, so a battle spends a share
//! of a tick on it.
//!
//! This file is the journey's stages and how one leads to the next
//! ([`Stage`], [`Journey::step`]); the graph search is [`roads`], and
//! making its answer physical is [`driving`].
use std::collections::BTreeSet;

use contract::command::RoutePolicy;
use contract::scenario::NavigationRules;

mod digest;
mod driving;
mod roads;

use driving::Driving;
use roads::{over_ground, RoadSearch};

use super::corners::Rounding;
use super::roads::RoadNet;
use super::search::{RouteSearch, Scratch, SearchWork};
use super::{Leg, Mobility, NavGrid, Plan};
use crate::math::{Obb2, V2};

/// The room a mover on a road leaves between itself and the road's middle,
/// and between its own middle and the road's edge.
const KEEP_RIGHT_M: f64 = 0.5;

/// Points this close are coalesced when constructing a road run.
const JOIN_EPSILON_M: f64 = 1e-6;

/// How far along a road a mover takes to get over to its side of it, or
/// back to the middle to leave it.
const MERGE_M: f64 = 32.0;

enum Stage {
    Terrain(super::terrain::Probe),
    Roads(RoadSearch),
    Driving(Box<Driving>),
    Direct(RouteSearch),
    /// One rare-failure proof, before trying further goal road accesses.
    GoalProof(RouteSearch, u32),
    /// A wheeled route's corners being made drivable.
    Rounding(Box<Rounding>),
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
    pub fn new(scratch: Option<Scratch>, leg: Leg, rules: &NavigationRules) -> Self {
        Journey {
            from: leg.from,
            goal: leg.goal,
            m: Mobility {
                // Making room for live traffic must not clear roadside bodies.
                push: if leg.avoid.is_empty() {
                    leg.m.push
                } else {
                    contract::scenario::PushClass::None
                },
                ..*leg.m
            },
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
            Stage::Roads(_) | Stage::Terrain(_) | Stage::Rounding(_) => (None, None),
        };
        (
            plan,
            held.or(self.scratch)
                .or_else(|| self.goal_component.map(|search| search.finish().1)),
        )
    }

    fn step(&mut self, grid: &NavGrid, roads: &RoadNet) {
        let stage = std::mem::replace(
            &mut self.stage,
            Stage::Done(Plan::Blocked(super::BlockReason::NoRoute)),
        );
        // A route is rounded once, as it is first found.
        let settled = matches!(stage, Stage::Rounding(_) | Stage::Done(_));
        self.stage = match stage {
            Stage::Done(plan) => Stage::Done(plan),
            Stage::Rounding(mut corners) => match corners.step(grid, &self.m, &self.avoid) {
                Some(route) => Stage::Done(Plan::Route(route)),
                None => Stage::Rounding(corners),
            },
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
                                Stage::Driving(Box::new(Driving {
                                    joined,
                                    accesses: [
                                        search.starts[way.start].arc,
                                        search.goals[way.goal].arc,
                                    ],
                                    runs,
                                    out: Vec::new(),
                                    piece: 0,
                                    checked: 0.0,
                                    committed: 0.0,
                                    leading: false,
                                    off_lane: None,
                                    search: None,
                                    sought: (joined, 0),
                                    candidate: 0,
                                    probe: None,
                                }))
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
        if let (false, Stage::Done(Plan::Route(route))) = (settled, &self.stage) {
            if let Some(corners) = Rounding::new(self.from, route, &self.m) {
                self.stage = Stage::Rounding(Box::new(corners));
            }
        }
    }
}
