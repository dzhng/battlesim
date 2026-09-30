use super::candidate::{Mobility, NavGrid, Refinement, RefinementStatus};
use super::math::V2;
use contract::command::RoutePolicy;
use std::{
    collections::BTreeMap,
    ops::Bound::{Excluded, Unbounded},
};
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    Committed {
        tick: u64,
        unit: u32,
        leg: u64,
        cost_bits: u64,
    },
    PhysicalDecline {
        tick: u64,
        unit: u32,
        leg: u64,
    },
    RevisionChanged {
        tick: u64,
        unit: u32,
        leg: u64,
    },
    Canceled {
        unit: u32,
        leg: u64,
    },
}
struct Job {
    leg: u64,
    side: usize,
    revision: u64,
    points: Vec<V2>,
    mobility: Mobility,
    policy: RoutePolicy,
    segment: usize,
    refinement: Option<Refinement>,
    cost: f64,
    work: usize,
}
pub struct Scheduler {
    jobs: BTreeMap<u32, Job>,
    cursor: Option<u32>,
    pub tick: u64,
    pub events: Vec<Event>,
}
impl Scheduler {
    pub fn new() -> Self {
        Self {
            jobs: BTreeMap::new(),
            cursor: None,
            tick: 0,
            events: Vec::new(),
        }
    }
    pub fn submit(
        &mut self,
        unit: u32,
        leg: u64,
        side: usize,
        revision: u64,
        points: Vec<V2>,
        mobility: Mobility,
        policy: RoutePolicy,
    ) {
        assert!(side < 2 && points.len() >= 2);
        if let Some(old) = self.jobs.remove(&unit) {
            self.events.push(Event::Canceled { unit, leg: old.leg });
        }
        self.jobs.insert(
            unit,
            Job {
                leg,
                side,
                revision,
                points,
                mobility,
                policy,
                segment: 0,
                refinement: None,
                cost: 0.0,
                work: 0,
            },
        );
    }
    pub fn cancel(&mut self, unit: u32) {
        if let Some(old) = self.jobs.remove(&unit) {
            self.events.push(Event::Canceled { unit, leg: old.leg });
        }
    }
    pub fn work_by_unit(&self) -> Vec<(u32, usize)> {
        self.jobs.iter().map(|(&u, j)| (u, j.work)).collect()
    }
    pub fn pending(&self) -> usize {
        self.jobs.len()
    }
    pub fn advance(
        &mut self,
        grids: [&NavGrid; 2],
        revisions: [u64; 2],
        total_budget: usize,
        quantum: usize,
    ) -> usize {
        assert!(total_budget > 1 && quantum > 1);
        self.tick += 1;
        let mut remaining = total_budget;
        while remaining > 1 && !self.jobs.is_empty() {
            let next = self
                .cursor
                .and_then(|u| {
                    self.jobs
                        .range((Excluded(u), Unbounded))
                        .next()
                        .map(|(&u, _)| u)
                })
                .unwrap_or(*self.jobs.first_key_value().unwrap().0);
            self.cursor = Some(next);
            remaining -= 1;
            let job = self.jobs.get_mut(&next).unwrap();
            let mut allowance = remaining.min(quantum);
            let before = allowance;
            let mut event = None;
            if job.revision != revisions[job.side] {
                event = Some(Event::RevisionChanged {
                    tick: self.tick,
                    unit: next,
                    leg: job.leg,
                });
            } else {
                while allowance > 0 {
                    if job.segment + 1 == job.points.len() {
                        event = Some(Event::Committed {
                            tick: self.tick,
                            unit: next,
                            leg: job.leg,
                            cost_bits: job.cost.to_bits(),
                        });
                        break;
                    }
                    if job.refinement.is_none() {
                        allowance -= 1;
                        job.refinement = Some(Refinement::new(
                            job.points[job.segment],
                            job.points[job.segment + 1],
                            job.mobility,
                            job.policy,
                        ));
                    }
                    match job
                        .refinement
                        .as_mut()
                        .unwrap()
                        .poll(grids[job.side], &mut allowance)
                    {
                        RefinementStatus::Pending => break,
                        RefinementStatus::Complete(Some(cost)) => {
                            if allowance == 0 {
                                break;
                            }
                            allowance -= 1;
                            job.cost += cost;
                            job.segment += 1;
                            job.refinement = None;
                        }
                        RefinementStatus::Complete(None) => {
                            event = Some(Event::PhysicalDecline {
                                tick: self.tick,
                                unit: next,
                                leg: job.leg,
                            });
                            break;
                        }
                    }
                }
            }
            job.work += before - allowance;
            remaining -= before - allowance;
            if let Some(e) = event {
                self.jobs.remove(&next);
                self.events.push(e);
            } else {
                assert!(before > allowance, "pending job must make progress");
            }
        }
        total_budget - remaining
    }
}
