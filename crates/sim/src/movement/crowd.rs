//! Where every standing soldier is this tick, for personal space.
use std::collections::HashMap;

use crate::math::V2;
use crate::units::Unit;

/// Every standing soldier on the field, for personal space: positions kept
/// current as each soldier steps, found through 2 m buckets of where each
/// stood at the start of the tick (no one moves further in a tick).
pub struct Crowd {
    /// First entry of each unit's soldiers.
    base: Vec<usize>,
    /// Traffic visits only living squads; footprint radii are refreshed whenever
    /// movement or a hull shove changes their soldiers or settled centre.
    pub(super) standing: Vec<usize>,
    pub(super) radii: Vec<f64>,
    at: Vec<Option<V2>>,
    buckets: HashMap<(i32, i32), Vec<u32>>,
}

pub(super) const CROWD_BUCKET_M: f64 = 2.0;

fn bucket(p: V2) -> (i32, i32) {
    (
        (p.x / CROWD_BUCKET_M).floor() as i32,
        (p.y / CROWD_BUCKET_M).floor() as i32,
    )
}

impl Crowd {
    pub fn gather(units: &[Unit], radius: f64) -> Self {
        let mut base = Vec::with_capacity(units.len());
        let mut at = Vec::new();
        let mut standing_units = Vec::new();
        let mut radii = vec![0.0; units.len()];
        let mut buckets: HashMap<(i32, i32), Vec<u32>> = HashMap::new();
        for (i, u) in units.iter().enumerate() {
            base.push(at.len());
            let standing = !u.is_vehicle() && !u.garrisoned();
            if standing && u.on_field() {
                standing_units.push(i);
                radii[i] = u.footprint_radius(radius);
            }
            for s in &u.members {
                let p = (standing && s.alive()).then(|| s.position.xy());
                if let Some(p) = p {
                    buckets.entry(bucket(p)).or_default().push(at.len() as u32);
                }
                at.push(p);
            }
        }
        Crowd {
            base,
            standing: standing_units,
            radii,
            at,
            buckets,
        }
    }

    pub(super) fn refresh_radius(&mut self, unit: &Unit, radius: f64) {
        self.radii[unit.id.0 as usize] = unit.footprint_radius(radius);
    }

    pub(super) fn id(&self, unit: usize, member: usize) -> usize {
        self.base[unit] + member
    }

    pub(super) fn set(&mut self, id: usize, p: V2) {
        if let Some(slot) = self.at.get_mut(id) {
            if slot.is_some() {
                *slot = Some(p);
            }
        }
    }

    /// Other standing soldiers within `radius` of `p`.
    pub(super) fn near(&self, id: usize, p: V2, radius: f64, mut each: impl FnMut(V2)) {
        let (bi, bj) = bucket(p);
        for j in bj - 1..=bj + 1 {
            for i in bi - 1..=bi + 1 {
                for &k in self.buckets.get(&(i, j)).into_iter().flatten() {
                    let k = k as usize;
                    if k == id {
                        continue;
                    }
                    if let Some(q) = self.at[k] {
                        if (q - p).inside_radius(radius) {
                            each(q);
                        }
                    }
                }
            }
        }
    }
}
