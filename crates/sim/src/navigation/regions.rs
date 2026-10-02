//! The rectangles outside which a grid is open flat ground: conservative
//! bounds of every nonuniform surface and every known body. They are
//! bucketed so a cell asks only about those near it (a full-size map has one
//! for every tree): a query sees exactly the rectangles within
//! [`REACH_M`] of its point, plus every irregular one, so the answer is the
//! same as scanning them all.
use super::{MAX_CLEARANCE_M, NAV_CELL_M};
use crate::math::V2;

pub(super) type Rect = [f64; 4];

const BUCKET_M: f64 = 32.0;
/// The widest halo a query tests a rectangle with.
const REACH_M: f64 = MAX_CLEARANCE_M + 4.0 * NAV_CELL_M;

/// Whether a rectangle has a place in the buckets: finite and not inside out.
fn regular(r: &Rect) -> bool {
    r.iter().all(|v| v.is_finite()) && r[2] >= 0.0 && r[3] >= 0.0
}

fn same(a: &Rect, b: &Rect) -> bool {
    a.map(f64::to_bits) == b.map(f64::to_bits)
}

/// The map's rectangles: what every side's grid starts with.
pub(super) struct BaseRegions {
    rects: Vec<Rect>,
    /// Non-finite or negative-size rectangles: every query sees them.
    irregular: Vec<Rect>,
    nx: usize,
    ny: usize,
    /// Bucket `k` holds `items[starts[k]..starts[k + 1]]`, indices of `rects`.
    starts: Vec<u32>,
    items: Vec<u32>,
}

impl BaseRegions {
    pub fn new(all: Vec<Rect>, width: f64, depth: f64) -> Self {
        let nx = (width / BUCKET_M).ceil().max(1.0) as usize;
        let ny = (depth / BUCKET_M).ceil().max(1.0) as usize;
        let (rects, irregular): (Vec<_>, Vec<_>) = all.into_iter().partition(regular);
        let mut regions = Self {
            rects,
            irregular,
            nx,
            ny,
            starts: vec![0u32; nx * ny + 1],
            items: Vec::new(),
        };
        for r in &regions.rects {
            for k in regions.span(r) {
                regions.starts[k + 1] += 1;
            }
        }
        for k in 0..nx * ny {
            regions.starts[k + 1] += regions.starts[k];
        }
        let mut items = vec![0u32; regions.starts[nx * ny] as usize];
        let mut next = regions.starts.clone();
        for (id, r) in regions.rects.iter().enumerate() {
            for k in regions.span(r) {
                items[next[k] as usize] = id as u32;
                next[k] += 1;
            }
        }
        regions.items = items;
        regions
    }

    /// The buckets a query within reach of `r` can fall in.
    fn span(&self, r: &Rect) -> impl Iterator<Item = usize> {
        let at = |v: f64, n: usize| ((v / BUCKET_M).floor().max(0.0) as usize).min(n - 1);
        let xs = at(r[0] - REACH_M, self.nx)..=at(r[0] + r[2] + REACH_M, self.nx);
        let ys = at(r[1] - REACH_M, self.ny)..=at(r[1] + r[3] + REACH_M, self.ny);
        let nx = self.nx;
        ys.flat_map(move |j| xs.clone().map(move |i| j * nx + i))
    }

    fn bucket(&self, p: V2) -> usize {
        let at = |v: f64, n: usize| ((v / BUCKET_M).floor().max(0.0) as usize).min(n - 1);
        at(p.y, self.ny) * self.nx + at(p.x, self.nx)
    }

    fn held(&self, bucket: usize) -> impl Iterator<Item = &Rect> {
        self.items[self.starts[bucket] as usize..self.starts[bucket + 1] as usize]
            .iter()
            .map(|&id| &self.rects[id as usize])
    }
}

/// One side's rectangles: the map's, less those of the bodies it believes
/// gone or elsewhere, plus those of the bodies it has learned. It holds
/// only the buckets it knows differently.
#[derive(Default, Clone)]
pub(super) struct Regions {
    /// Per bucket, 0 for the map's, or one more than its place in `own`.
    slots: Vec<u32>,
    own: Vec<Vec<Rect>>,
    /// Irregular rectangles added to the map's.
    irregular: Vec<Rect>,
}

impl Regions {
    /// This side's rectangles of `bucket`, copied from the map's the first
    /// time it differs.
    fn own(&mut self, base: &BaseRegions, bucket: usize) -> &mut Vec<Rect> {
        if self.slots.is_empty() {
            self.slots = vec![0; base.nx * base.ny];
        }
        if self.slots[bucket] == 0 {
            self.own.push(base.held(bucket).copied().collect());
            self.slots[bucket] = self.own.len() as u32;
        }
        &mut self.own[self.slots[bucket] as usize - 1]
    }

    pub fn add(&mut self, base: &BaseRegions, r: Rect) {
        if !regular(&r) {
            return self.irregular.push(r);
        }
        for bucket in base.span(&r) {
            self.own(base, bucket).push(r);
        }
    }

    /// Take away one rectangle `r`: one the map holds, or one added.
    pub fn remove(&mut self, base: &BaseRegions, r: Rect) {
        let take = |held: &mut Vec<Rect>| {
            let at = held.iter().position(|h| same(h, &r));
            held.swap_remove(at.expect("a removed rectangle is held"));
        };
        if !regular(&r) {
            return take(&mut self.irregular);
        }
        for bucket in base.span(&r) {
            take(self.own(base, bucket));
        }
    }

    /// Whether `test` holds for any rectangle that can lie within the reach
    /// of `p`.
    pub fn any_near(&self, base: &BaseRegions, p: V2, test: impl Fn(&Rect) -> bool) -> bool {
        let bucket = base.bucket(p);
        let near = match self.slots.get(bucket) {
            Some(&slot) if slot != 0 => self.own[slot as usize - 1].iter().any(&test),
            _ => base.held(bucket).any(&test),
        };
        near || base.irregular.iter().chain(&self.irregular).any(&test)
    }

    /// Every rectangle a query in `bucket` sees, in a fixed order whatever
    /// order they were added in.
    #[cfg(test)]
    pub fn seen_in(&self, base: &BaseRegions, bucket: usize) -> Vec<[u64; 4]> {
        let near: Vec<&Rect> = match self.slots.get(bucket) {
            Some(&slot) if slot != 0 => self.own[slot as usize - 1].iter().collect(),
            _ => base.held(bucket).collect(),
        };
        let mut seen: Vec<[u64; 4]> = near
            .into_iter()
            .chain(&base.irregular)
            .chain(&self.irregular)
            .map(|r| r.map(f64::to_bits))
            .collect();
        seen.sort_unstable();
        seen
    }

    #[cfg(test)]
    pub fn buckets(base: &BaseRegions) -> usize {
        base.nx * base.ny
    }
}
