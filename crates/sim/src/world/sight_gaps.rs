//! Gaps too narrow to see through. Sight passes between two bodies that hide
//! what lies behind them only through a gap at least
//! `sensors.min_sight_gap_m` wide: each narrower gap between two such bodies
//! of the map is filled, for sight alone, by a box spanning it from face to
//! face. A gap is between two bodies, so a line running along one row of
//! them never meets one; it lapses when either body goes.
use super::props::{ray_box, Prop, PropId};
use crate::math::{v2, v3, Obb2, V2, V3};

/// Grid cells the gaps are filed under, metres a side.
const CELL_M: f64 = 16.0;
/// How far a gap's box reaches into each of its bodies, so a line cannot
/// slip between box and face.
const INTO_M: f64 = 0.05;

/// One filled gap: the bodies it lies between, its box on the ground, and
/// the heights both bodies stand over.
#[derive(Clone, Debug)]
pub struct SightGap {
    pub between: [PropId; 2],
    pub footprint: Obb2,
    pub base: f64,
    pub top: f64,
}

/// The map's filled gaps, filed by grid cell: `starts[c]..starts[c + 1]`
/// indexes `ids` for cell `c`.
#[derive(Clone, Debug, Default)]
pub struct SightGaps {
    gaps: Vec<SightGap>,
    nx: usize,
    ny: usize,
    starts: Vec<u32>,
    ids: Vec<u32>,
}

impl SightGaps {
    /// The gaps narrower than `least` between any two of `occluders`, each
    /// pair offered once by `near` (the bodies that may come within a
    /// radius of a point), on a map `size` metres across.
    pub fn find<'a>(
        occluders: impl Iterator<Item = &'a Prop>,
        near: impl Fn(V2, f64) -> Vec<&'a Prop>,
        least: f64,
        size: V2,
    ) -> Self {
        let mut gaps = Vec::new();
        for a in occluders {
            for b in near(a.center, a.half.xy().length() + least) {
                if b.id > a.id && b.body.occludes {
                    gaps.extend(gap(a, b, least));
                }
            }
        }
        let nx = (size.x / CELL_M).ceil().max(1.0) as usize;
        let ny = (size.y / CELL_M).ceil().max(1.0) as usize;
        let cells = |g: &SightGap| {
            let r = g.footprint.half.length();
            let lo = cell(g.footprint.center - v2(r, r), nx, ny);
            let hi = cell(g.footprint.center + v2(r, r), nx, ny);
            (lo.1..=hi.1).flat_map(move |j| (lo.0..=hi.0).map(move |i| j * nx + i))
        };
        let mut counts = vec![0u32; nx * ny + 1];
        for g in &gaps {
            for c in cells(g) {
                counts[c + 1] += 1;
            }
        }
        for c in 1..counts.len() {
            counts[c] += counts[c - 1];
        }
        let mut at = counts.clone();
        let mut ids = vec![0u32; *counts.last().unwrap() as usize];
        for (k, g) in gaps.iter().enumerate() {
            for c in cells(g) {
                ids[at[c] as usize] = k as u32;
                at[c] += 1;
            }
        }
        Self {
            gaps,
            nx,
            ny,
            starts: counts,
            ids,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &SightGap> {
        self.gaps.iter()
    }

    /// Whether a gap whose bodies both still stand (`stands`) closes the
    /// line from `a` to `b`. A gap the line starts or ends in is passed: one
    /// standing in a crack sees out of it, and is seen.
    pub fn close(&self, a: V3, b: V3, stands: impl Fn(PropId) -> bool) -> bool {
        if self.gaps.is_empty() {
            return false;
        }
        let d = b - a;
        let span = d.xy().length();
        let steps = (span / (CELL_M / 2.0)).ceil() as usize;
        let mut last = usize::MAX;
        let mut tried: Vec<u32> = Vec::new();
        (0..=steps).any(|s| {
            let p = a.xy() + d.xy() * (s as f64 / steps.max(1) as f64);
            let (i, j) = cell(p, self.nx, self.ny);
            let c = j * self.nx + i;
            if c == last {
                return false;
            }
            last = c;
            self.ids[self.starts[c] as usize..self.starts[c + 1] as usize]
                .iter()
                .any(|&k| {
                    if tried.contains(&k) {
                        return false;
                    }
                    tried.push(k);
                    let g = &self.gaps[k as usize];
                    g.between.iter().all(|&id| stands(id))
                        && !g.footprint.contains(a.xy(), 0.0)
                        && !g.footprint.contains(b.xy(), 0.0)
                        && meets(g, a, d)
                })
        })
    }
}

/// Whether `a + d·t`, t ∈ [0, 1], passes through `g`'s box.
fn meets(g: &SightGap, a: V3, d: V3) -> bool {
    let f = &g.footprint;
    let local = |p: V2| p.rotated(-f.yaw);
    let o = local(a.xy() - f.center);
    let dl = local(d.xy());
    let mid = (g.base + g.top) / 2.0;
    let half = v3(f.half.x, f.half.y, (g.top - g.base) / 2.0);
    ray_box(v3(o.x, o.y, a.z - mid), v3(dl.x, dl.y, d.z), half, 1.0).is_some()
}

fn cell(p: V2, nx: usize, ny: usize) -> (usize, usize) {
    let i = (p.x / CELL_M).floor().clamp(0.0, (nx - 1) as f64) as usize;
    let j = (p.y / CELL_M).floor().clamp(0.0, (ny - 1) as f64) as usize;
    (i, j)
}

/// The filled gap between `a` and `b` if they stand closer than `least` with
/// faces across from each other: along the axis that parts them, from `a`'s
/// face to `b`'s, as wide as the faces share.
fn gap(a: &Prop, b: &Prop, least: f64) -> Option<SightGap> {
    let (fa, fb) = (a.footprint(), b.footprint());
    let reach = |f: &Obb2, n: V2| {
        let (x, y) = (v2(1.0, 0.0).rotated(f.yaw), v2(0.0, 1.0).rotated(f.yaw));
        f.half.x * x.dot(n).abs() + f.half.y * y.dot(n).abs()
    };
    let d = fb.center - fa.center;
    let (apart, n) = [fa.yaw, fb.yaw]
        .into_iter()
        .flat_map(|yaw| [v2(1.0, 0.0).rotated(yaw), v2(0.0, 1.0).rotated(yaw)])
        .map(|axis| {
            let n = if d.dot(axis) < 0.0 { -axis } else { axis };
            (d.dot(n) - reach(&fa, n) - reach(&fb, n), n)
        })
        .max_by(|x, y| x.0.total_cmp(&y.0))?;
    if !(apart > 1e-6 && apart < least) {
        return None;
    }
    let t = v2(-n.y, n.x);
    let lo = (fa.center.dot(t) - reach(&fa, t)).max(fb.center.dot(t) - reach(&fb, t));
    let hi = (fa.center.dot(t) + reach(&fa, t)).min(fb.center.dot(t) + reach(&fb, t));
    let base = a.base_z.max(b.base_z);
    let top = (a.base_z + 2.0 * a.half.z).min(b.base_z + 2.0 * b.half.z);
    if hi <= lo || top <= base {
        return None;
    }
    let face = fa.center.dot(n) + reach(&fa, n);
    let along = face + apart / 2.0;
    Some(SightGap {
        between: [a.id, b.id],
        footprint: Obb2 {
            center: n * along + t * ((lo + hi) / 2.0),
            half: v2(apart / 2.0 + INTO_M, (hi - lo) / 2.0),
            yaw: libm::atan2(n.y, n.x),
        },
        base,
        top,
    })
}
