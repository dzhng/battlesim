//! A journey's share of the battle's digest: how far it has got.
use super::{Driving, Journey, Stage};
use crate::digest::Digest;

impl Journey {
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
                d.u64(1);
                driving.digest(d);
            }
            Stage::Direct(search) => {
                d.u64(2);
                search.digest(d);
            }
            Stage::GoalProof(search, driving) => {
                d.u64(5);
                driving.digest(d);
                search.digest(d);
            }
            Stage::Rounding(corners) => {
                d.u64(6);
                corners.digest(d);
            }
            Stage::Done(_) => {
                d.u64(3);
            }
        }
    }
}

impl Driving {
    pub(super) fn digest(&self, d: &mut Digest) {
        d.u64(self.piece as u64)
            .f64(self.checked)
            .u64(self.out.len() as u64)
            .u64(self.search.is_some() as u64);
        d.u64(self.candidate as u64)
            .f64(self.sought.0.x)
            .f64(self.sought.0.y)
            .u64(self.sought.1 as u64)
            .u64(self.off_lane.is_some() as u64);
        if let Some((at, shift)) = self.off_lane {
            d.f64(at.x).f64(at.y).u64(shift as u64);
        }
        d.u64(self.probe.is_some() as u64);
        if let Some(probe) = &self.probe {
            probe.digest(d);
        }
        if let Some(search) = &self.search {
            search.digest(d);
        }
    }
}
