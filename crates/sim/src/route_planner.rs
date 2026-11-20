//! The battle's one route planner. A unit that needs a route asks for one;
//! the planner spends each tick's allowance of work
//! (`navigation.work_per_tick`) on the waiting units' [`Journey`]s, first on
//! whichever has had the least so far, so a short search is whole in a tick
//! or two however many long ones are under way, and searches served alike
//! share alike. It hands back each route on the tick it is whole, with the
//! knowledge revision it was searched against: the mover checks it against
//! what its side learned since, as it checks every later change. Nothing here
//! reads the clock: a route's tick follows from the work counted, so a replay
//! plans the same.
use std::collections::BTreeMap;

use contract::command::RoutePolicy;
use contract::ids::{Side, UnitId};
use contract::scenario::NavigationRules;

use crate::digest::Digest;
use crate::math::{Obb2, V2};
use crate::navigation::{Journey, Leg, Mobility, NavGrid, Plan, RoadNet, Scratch};

/// One unit's request for a route, and what its mover needs back with it.
#[derive(Clone, Debug)]
pub struct Request {
    pub side: Side,
    pub from: V2,
    pub goal: V2,
    pub mobility: Mobility,
    pub policy: RoutePolicy,
    /// A way round these footprints: the vehicles it has waited among too
    /// long. Empty for an ordinary request.
    pub detour: Vec<Obb2>,
    /// The route a detour replaces if one is found, and resumes if not.
    pub kept: Option<Vec<V2>>,
    /// The goal is not the one the unit last planned to.
    pub new_goal: bool,
}

struct Job {
    request: Request,
    /// The side's knowledge revision the journey started against.
    revision: u64,
    journey: Option<Box<Journey>>,
    /// Work spent on its journey so far.
    served: u64,
}

impl Job {
    /// Spend the allowance on the journey; its plan, and the revision it
    /// was planned against, once whole. What the side learned meanwhile is
    /// the mover's to check, as it checks every change along its route.
    fn advance(
        &mut self,
        (rules, roads): (&NavigationRules, &RoadNet),
        (grid, revision): (&NavGrid, u64),
        spare: &mut Vec<Scratch>,
        allowance: u64,
    ) -> (u64, Option<Plan>) {
        let before = grid.work();
        let r = &self.request;
        let journey = self.journey.get_or_insert_with(|| {
            self.revision = revision;
            Box::new(Journey::new(
                spare.pop(),
                Leg {
                    from: r.from,
                    goal: r.goal,
                    m: &r.mobility,
                    policy: r.policy,
                    avoid: &r.detour,
                },
                rules,
            ))
        });
        journey.advance(grid, roads, allowance);
        if journey.plan().is_none() {
            return (grid.work() - before, None);
        }
        let (plan, scratch) = self.journey.take().expect("a journey").finish();
        spare.extend(scratch);
        (grid.work() - before, plan)
    }
}

/// Every unit's route request in progress.
#[derive(Default)]
pub struct RoutePlanner {
    jobs: BTreeMap<UnitId, Job>,
    /// Work spent beyond a tick's allowance (a search overruns by at most
    /// one step), taken off the next tick's.
    overdraft: u64,
    /// Finished and cancelled searches' bookkeeping, for the next searches:
    /// ending a search never frees what it reached.
    spare: Vec<Scratch>,
    /// Work spent in the latest tick.
    spent: u64,
}

impl RoutePlanner {
    /// Ask for a route for `unit`, in place of any it was waiting for.
    pub fn submit(&mut self, unit: UnitId, request: Request) {
        self.cancel(unit);
        self.jobs.insert(
            unit,
            Job {
                request,
                revision: 0,
                journey: None,
                served: 0,
            },
        );
    }

    /// Drop `unit`'s request, if it has one: its route will never arrive.
    pub fn cancel(&mut self, unit: UnitId) {
        if let Some(Job {
            journey: Some(journey),
            ..
        }) = self.jobs.remove(&unit)
        {
            self.spare.extend(journey.finish().1);
        }
    }

    /// What `unit` is waiting for.
    pub fn pending(&self, unit: UnitId) -> Option<&Request> {
        self.jobs.get(&unit).map(|job| &job.request)
    }

    /// Which sides have a unit waiting.
    pub fn sides(&self) -> [bool; 3] {
        Side::ALL.map(|side| self.jobs.values().any(|job| job.request.side == side))
    }

    /// Units waiting for a route.
    pub fn waiting(&self) -> usize {
        self.jobs.len()
    }

    /// Work spent in the latest tick.
    pub fn spent(&self) -> u64 {
        self.spent
    }

    /// Spend one tick's allowance (`rules.work_per_tick`) on the waiting
    /// units, a slice at a time, each slice to the search served least so
    /// far (the lowest unit among equals). `grids` holds each side's grid
    /// and knowledge revision (for every side with a unit waiting); `roads`
    /// is the map's road graph. Returns the routes that are whole, in unit
    /// order, each with the knowledge revision it was planned against.
    pub fn advance(
        &mut self,
        rules: &NavigationRules,
        roads: &RoadNet,
        grids: [Option<(&NavGrid, u64)>; 3],
    ) -> Vec<(UnitId, Request, Plan, u64)> {
        let allowance = rules.work_per_tick as u64;
        let mut left = allowance.saturating_sub(self.overdraft);
        self.overdraft = self.overdraft.saturating_sub(allowance);
        self.spent = 0;
        // Searches served alike each get an equal slice of the tick.
        let slice = (allowance / self.jobs.len().max(1) as u64).max(1);
        let mut finished = Vec::new();
        while left > 0 {
            let least = self.jobs.iter().min_by_key(|(id, job)| (job.served, **id));
            let Some((&id, _)) = least else {
                break;
            };
            let job = self.jobs.get_mut(&id).expect("the job just found");
            let grid = grids[job.request.side.index()].expect("a grid for every side waiting");
            let (spent, plan) = job.advance((rules, roads), grid, &mut self.spare, slice.min(left));
            job.served += spent;
            self.spent += spent;
            self.overdraft += spent.saturating_sub(left);
            left = left.saturating_sub(spent);
            if let Some(plan) = plan {
                let job = self.jobs.remove(&id).expect("the job just advanced");
                finished.push((id, job.request, plan, job.revision));
            }
        }
        finished.sort_by_key(|(id, ..)| *id);
        finished
    }

    /// Fold in everything a pending route's future depends on. A planner
    /// with nothing waiting and nothing owed holds none.
    pub fn digest(&self, d: &mut Digest) {
        if self.jobs.is_empty() && self.overdraft == 0 {
            return;
        }
        d.u64(self.jobs.len() as u64).u64(self.overdraft);
        for (id, job) in &self.jobs {
            let r = &job.request;
            d.u64(id.0 as u64)
                .u64(r.side.index() as u64)
                .f64(r.from.x)
                .f64(r.from.y)
                .f64(r.goal.x)
                .f64(r.goal.y)
                .u64(r.policy as u64)
                .u64(r.new_goal as u64);
            d.u64(r.detour.len() as u64);
            for b in &r.detour {
                d.f64(b.center.x)
                    .f64(b.center.y)
                    .f64(b.yaw)
                    .f64(b.half.x)
                    .f64(b.half.y);
            }
            d.u64(r.kept.as_ref().map_or(u64::MAX, |route| route.len() as u64));
            for p in r.kept.iter().flatten() {
                d.f64(p.x).f64(p.y);
            }
            d.u64(job.revision).u64(job.served);
            match &job.journey {
                None => {
                    d.u64(0);
                }
                Some(journey) => {
                    d.u64(1);
                    journey.digest(d);
                }
            }
        }
    }
}
