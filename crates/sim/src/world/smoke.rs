//! The live smoke clouds, whose law over time `contract::smoke` owns. A
//! cloud is not a body: it blocks no mover, stops no round and gives no
//! cover, so popping one or losing one bumps no obstacle or navigation
//! revision, and no side remembers it once it is gone. Only sight reads it:
//! [`WorldGeometry::sight_depth`], a target's ground concealment
//! ([`WorldGeometry::smoke_concealment`]) and the fog sweep.
use contract::ids::{Side, Tick};
use contract::smoke::{CloudState, SmokeCloud, SmokeCloudSpec};

use super::WorldGeometry;
use crate::digest::Digest;
use crate::math::{V2, V3};

/// Every live cloud, in pop order, with its state as of the last
/// [`SmokeClouds::advance`] or pop.
#[derive(Clone, Debug, Default)]
pub struct SmokeClouds {
    clouds: Vec<Popped>,
    /// `states[k]` is `clouds[k]` now.
    states: Vec<CloudState>,
    /// The id the next cloud takes.
    next: u32,
}

/// A cloud and the side whose unit popped it.
#[derive(Clone, Debug)]
pub struct Popped {
    pub cloud: SmokeCloud,
    pub side: Side,
}

impl SmokeClouds {
    /// `side`'s cloud centred on `centre` from `tick`, live at once.
    pub fn pop(
        &mut self,
        side: Side,
        centre: V3,
        spec: &SmokeCloudSpec,
        tick: Tick,
        tick_hz: u32,
    ) -> u32 {
        let cloud = SmokeCloud {
            id: self.next,
            centre: [centre.x, centre.y, centre.z],
            popped: tick,
            spec: spec.clone(),
        };
        let state = contract::smoke::state(&cloud, tick, tick_hz)
            .expect("a cloud is live on its pop tick (a positive life)");
        self.next += 1;
        self.clouds.push(Popped { cloud, side });
        self.states.push(state);
        self.next - 1
    }

    /// Bring every cloud to `tick`; those whose life is over go.
    pub fn advance(&mut self, tick: Tick, tick_hz: u32) {
        let mut k = 0;
        while k < self.clouds.len() {
            match contract::smoke::state(&self.clouds[k].cloud, tick, tick_hz) {
                Some(state) => {
                    self.states[k] = state;
                    k += 1;
                }
                None => {
                    self.clouds.remove(k);
                    self.states.remove(k);
                }
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.clouds.is_empty()
    }

    /// Each live cloud and its state now.
    pub fn live(&self) -> impl Iterator<Item = (&Popped, &CloudState)> {
        self.clouds.iter().zip(&self.states)
    }

    /// Every live cloud's state now, in pop order.
    pub fn states(&self) -> &[CloudState] {
        &self.states
    }

    /// The states of the clouds whose circle comes within `reach` of
    /// `centre`, in pop order.
    pub fn near(&self, centre: V2, reach: f64) -> Vec<CloudState> {
        self.states
            .iter()
            .filter(|c| within(c, centre, reach))
            .cloned()
            .collect()
    }

    /// A key that changes whenever a cloud within `reach` of `centre`
    /// appears, goes or changes phase: what a cached sight field near there
    /// depends on.
    pub fn key_near(&self, centre: V2, reach: f64) -> u64 {
        let mut d = Digest::default();
        for (popped, state) in self.live().filter(|(_, c)| within(c, centre, reach)) {
            d.u64(u64::from(popped.cloud.id))
                .u64(u64::from(state.phase));
        }
        d.finish()
    }

    /// Fold the clouds into the battle digest: their count, then each one.
    /// A battle that never popped smoke folds only the zero count.
    pub fn digest(&self, d: &mut Digest) {
        d.u64(self.clouds.len() as u64);
        for Popped { cloud: c, side } in &self.clouds {
            let s = &c.spec;
            d.u64(u64::from(c.id))
                .u64(side.index() as u64)
                .f64(c.centre[0])
                .f64(c.centre[1])
                .f64(c.centre[2])
                .u64(c.popped);
            for v in [s.radius_m, s.height_m, s.life_s, s.bloom_s, s.fade_s] {
                d.f64(v);
            }
            d.u64(u64::from(s.bloom_steps))
                .u64(u64::from(s.fade_steps))
                .f64(s.depth_per_m)
                .f64(s.concealment);
        }
        if self.next > 0 {
            d.u64(u64::from(self.next));
        }
    }
}

/// Whether cloud `c`'s circle comes within `reach` of `centre`.
fn within(c: &CloudState, centre: V2, reach: f64) -> bool {
    let (dx, dy) = (c.centre[0] - centre.x, c.centre[1] - centre.y);
    let r = reach + c.radius_m;
    dx * dx + dy * dy <= r * r
}

impl WorldGeometry {
    pub fn smoke(&self) -> &SmokeClouds {
        &self.smoke
    }

    pub fn smoke_mut(&mut self) -> &mut SmokeClouds {
        &mut self.smoke
    }

    /// What the sight line a→b looks through: tree crowns
    /// ([`Self::foliage_depth`]) plus the smoke of every live cloud.
    pub fn sight_depth(&self, a: V3, b: V3) -> f64 {
        let crowns = self.foliage_depth(a, b);
        if self.smoke.is_empty() {
            return crowns;
        }
        crowns
            + contract::smoke::segment_depth(
                self.smoke.states(),
                [a.x, a.y, a.z],
                [b.x, b.y, b.z],
                |x, y| self.height_at(x, y),
            )
    }

    /// The concealment smoke gives a target standing at (x, y): the
    /// strongest cloud over it, or 1 outside every cloud.
    pub fn smoke_concealment(&self, x: f64, y: f64) -> f64 {
        contract::smoke::concealment_at(self.smoke.states(), x, y)
    }
}
