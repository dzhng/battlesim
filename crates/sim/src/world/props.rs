//! Solid props: oriented boxes standing on the ground. One store for static
//! scenery and later dynamic remains; a uniform bucket grid accelerates queries.
use crate::math::{v2, v3, V2, V3};
use contract::map::PropKind;

pub type PropId = u32;

#[derive(Clone, Debug, PartialEq)]
pub struct Prop {
    pub id: PropId,
    pub kind: PropKind,
    pub center: V2,
    pub yaw: f64,
    /// Half extents along heading, across heading, vertical.
    pub half: V3,
    pub base_z: f64,
}

impl Prop {
    pub fn top_z(&self) -> f64 {
        self.base_z + 2.0 * self.half.z
    }

    /// World point → prop-local frame (origin at box centre).
    fn to_local(&self, p: V3) -> V3 {
        let d = (p.xy() - self.center).rotated(-self.yaw);
        v3(d.x, d.y, p.z - (self.base_z + self.half.z))
    }

    fn dir_to_local(&self, d: V3) -> V3 {
        let r = d.xy().rotated(-self.yaw);
        v3(r.x, r.y, d.z)
    }

    /// Slab test. Returns (t, world normal) of the entry point; an origin
    /// already inside the box hits at t = 0.
    pub fn raycast(&self, origin: V3, dir: V3, max_t: f64) -> Option<(f64, V3)> {
        let o = self.to_local(origin);
        let d = self.dir_to_local(dir);
        let (mut t0, mut t1) = (f64::NEG_INFINITY, f64::INFINITY);
        let mut axis0 = 0usize;
        let mut sign0 = 0.0;
        let h = [self.half.x, self.half.y, self.half.z];
        let (oa, da) = ([o.x, o.y, o.z], [d.x, d.y, d.z]);
        for a in 0..3 {
            if da[a].abs() < 1e-12 {
                if oa[a].abs() > h[a] {
                    return None;
                }
                continue;
            }
            let mut near = (-h[a] - oa[a]) / da[a];
            let mut far = (h[a] - oa[a]) / da[a];
            let mut sign = -1.0;
            if near > far {
                std::mem::swap(&mut near, &mut far);
                sign = 1.0;
            }
            if near > t0 {
                t0 = near;
                axis0 = a;
                sign0 = sign;
            }
            t1 = t1.min(far);
            if t0 > t1 {
                return None;
            }
        }
        let t = if t0 >= 0.0 { t0 } else { 0.0 };
        if t > t1 || t > max_t || t1 < 0.0 {
            return None;
        }
        let mut n = [0.0; 3];
        n[axis0] = sign0;
        let nw = v2(n[0], n[1]).rotated(self.yaw);
        Some((t, v3(nw.x, nw.y, n[2])))
    }

    /// Whether the XY point lies inside the footprint, grown by `margin`.
    pub fn footprint_contains(&self, p: V2, margin: f64) -> bool {
        let d = (p - self.center).rotated(-self.yaw);
        d.x.abs() <= self.half.x + margin && d.y.abs() <= self.half.y + margin
    }

    /// Radius of the footprint's bounding circle.
    pub fn footprint_radius(&self) -> f64 {
        self.half.x.hypot(self.half.y)
    }
}

/// Uniform XY bucket grid over prop footprints.
pub struct PropIndex {
    bucket: f64,
    nx: usize,
    ny: usize,
    cells: Vec<Vec<PropId>>,
}

impl PropIndex {
    pub fn new(width: f64, depth: f64, bucket: f64) -> Self {
        let nx = (width / bucket).ceil().max(1.0) as usize;
        let ny = (depth / bucket).ceil().max(1.0) as usize;
        PropIndex {
            bucket,
            nx,
            ny,
            cells: vec![Vec::new(); nx * ny],
        }
    }

    fn cell_range(&self, p: &Prop) -> (usize, usize, usize, usize) {
        let r = p.footprint_radius();
        let clamp = |v: f64, n: usize| ((v / self.bucket).floor().max(0.0) as usize).min(n - 1);
        (
            clamp(p.center.x - r, self.nx),
            clamp(p.center.x + r, self.nx),
            clamp(p.center.y - r, self.ny),
            clamp(p.center.y + r, self.ny),
        )
    }

    pub fn insert(&mut self, p: &Prop) {
        let (i0, i1, j0, j1) = self.cell_range(p);
        for j in j0..=j1 {
            for i in i0..=i1 {
                self.cells[j * self.nx + i].push(p.id);
            }
        }
    }

    pub fn remove(&mut self, p: &Prop) {
        let (i0, i1, j0, j1) = self.cell_range(p);
        for j in j0..=j1 {
            for i in i0..=i1 {
                self.cells[j * self.nx + i].retain(|&id| id != p.id);
            }
        }
    }

    /// Candidate ids whose footprint circle may meet the XY disc.
    pub fn near(&self, center: V2, radius: f64, out: &mut Vec<PropId>) {
        let clamp = |v: f64, n: usize| ((v / self.bucket).floor().max(0.0) as usize).min(n - 1);
        for j in clamp(center.y - radius, self.ny)..=clamp(center.y + radius, self.ny) {
            for i in clamp(center.x - radius, self.nx)..=clamp(center.x + radius, self.nx) {
                out.extend_from_slice(&self.cells[j * self.nx + i]);
            }
        }
        out.sort_unstable();
        out.dedup();
    }

    /// Candidate ids in the buckets the XY projection of the segment crosses,
    /// in no particular order.
    pub fn along(&self, a: V2, b: V2, out: &mut Vec<PropId>) {
        // Conservative: every bucket overlapped by the segment's bounding box.
        // Buckets are coarse and prop counts local, so this stays cheap.
        let clamp = |v: f64, n: usize| ((v / self.bucket).floor().max(0.0) as usize).min(n - 1);
        let (i0, i1) = (clamp(a.x.min(b.x), self.nx), clamp(a.x.max(b.x), self.nx));
        let (j0, j1) = (clamp(a.y.min(b.y), self.ny), clamp(a.y.max(b.y), self.ny));
        let dir = b - a;
        let len = dir.length();
        for j in j0..=j1 {
            for i in i0..=i1 {
                if len > 0.0 {
                    // Skip buckets whose centre is further from the line than the bucket's half diagonal.
                    let c = v2(
                        (i as f64 + 0.5) * self.bucket,
                        (j as f64 + 0.5) * self.bucket,
                    );
                    let dist = (dir.cross(c - a) / len).abs();
                    if dist > self.bucket * std::f64::consts::FRAC_1_SQRT_2 + 1e-9 {
                        continue;
                    }
                }
                out.extend_from_slice(&self.cells[j * self.nx + i]);
            }
        }
        out.sort_unstable();
        out.dedup();
    }
}
