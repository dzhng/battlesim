//! A completed route checked against a side's new picture, one stretch at
//! a time. A route whose cells now require shoving must be searched again.
use super::{Mobility, Mover, NavGrid, Probe};
use crate::digest::Digest;
use crate::math::V2;
use contract::command::RoutePolicy;

pub struct RouteCheck {
    route: Vec<V2>,
    mobility: Mobility,
    from: V2,
    at: usize,
    probe: Option<Probe>,
}

impl RouteCheck {
    pub fn new(from: V2, route: Vec<V2>, mobility: Mobility) -> Self {
        Self {
            route,
            mobility,
            from,
            at: 0,
            probe: None,
        }
    }

    /// `None` while more remains, then whether every stretch is still safe.
    pub fn step(&mut self, grid: &NavGrid) -> Option<bool> {
        if self.at == self.route.len() {
            return Some(true);
        }
        let mut probe = self
            .probe
            .take()
            .unwrap_or_else(|| Probe::new(self.from, self.route[self.at], &self.mobility));
        match grid.read(
            &mut probe,
            Mover::free(&self.mobility),
            RoutePolicy::Shortest,
            false,
        ) {
            None => {
                self.probe = Some(probe);
                None
            }
            Some(None) => Some(false),
            Some(Some(_)) => {
                self.from = self.route[self.at];
                self.at += 1;
                (self.at == self.route.len()).then_some(true)
            }
        }
    }

    pub fn finish(self) -> Vec<V2> {
        self.route
    }

    pub fn digest(&self, d: &mut Digest) {
        d.u64(self.route.len() as u64)
            .u64(self.at as u64)
            .u64(self.probe.is_some() as u64);
        if let Some(probe) = &self.probe {
            probe.digest(d);
        }
        for p in &self.route {
            d.f64(p.x).f64(p.y);
        }
    }
}
