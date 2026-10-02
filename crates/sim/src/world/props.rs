//! Solid props: oriented boxes standing on the ground. One store for static
//! scenery and later dynamic remains; a uniform bucket grid accelerates queries.
use crate::math::{v2, v3, Obb2, V2, V3};
use contract::catalog::{PropBody, PropKind};
use contract::map::MoverClass;

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
    /// Its prop type's body row: what it blocks, stops, hides and weighs.
    pub body: PropBody,
    /// A tree a forest generated: its crown hides by the one `forests.rule`.
    pub forest_tree: bool,
    /// Every side plans with it, seen or not: remains that close an
    /// authored body's footprint as the body did (a building's ruin).
    pub known_to_all: bool,
}

impl Prop {
    /// Whether it stops a ground mover of `class` (navigation and collision).
    pub fn blocks(&self, class: MoverClass) -> bool {
        self.body.blocks.class(class)
    }

    pub fn top_z(&self) -> f64 {
        self.base_z + 2.0 * self.half.z
    }

    /// The ground footprint.
    pub fn footprint(&self) -> Obb2 {
        Obb2 {
            center: self.center,
            yaw: self.yaw,
            half: self.half.xy(),
        }
    }

    /// World point → prop-local frame (origin at box centre).
    fn to_local(&self, p: V3) -> V3 {
        let d = self.footprint().to_local(p.xy());
        v3(d.x, d.y, p.z - (self.base_z + self.half.z))
    }

    fn dir_to_local(&self, d: V3) -> V3 {
        let r = d.xy().rotated(-self.yaw);
        v3(r.x, r.y, d.z)
    }

    /// Slab test. Returns (t, world normal) of the entry point; an origin
    /// already inside the box hits at t = 0.
    pub fn raycast(&self, origin: V3, dir: V3, max_t: f64) -> Option<(f64, V3)> {
        let (t, n) = ray_box(
            self.to_local(origin),
            self.dir_to_local(dir),
            self.half,
            max_t,
        )?;
        let nw = n.xy().rotated(self.yaw);
        Some((t, v3(nw.x, nw.y, n.z)))
    }

    /// Radius of the footprint's bounding circle.
    pub fn footprint_radius(&self) -> f64 {
        self.half.x.hypot(self.half.y)
    }

    /// The footprint point nearest `p`, pushed `standoff` further out along
    /// the facade's outward normal: where a squad stands to reach the facade.
    pub fn exterior_point(&self, p: V2, standoff: f64) -> V2 {
        let d = self.footprint().to_local(p);
        let q = v2(
            d.x.clamp(-self.half.x, self.half.x),
            d.y.clamp(-self.half.y, self.half.y),
        );
        // A point inside leaves by the nearest facade.
        let (gap_x, gap_y) = (self.half.x - d.x.abs(), self.half.y - d.y.abs());
        let out = if gap_x <= 0.0 || gap_y <= 0.0 {
            let n = d - q;
            if n.length() > 0.0 {
                n.normalized()
            } else {
                v2(d.x.signum(), 0.0)
            }
        } else if gap_x < gap_y {
            v2(d.x.signum(), 0.0)
        } else {
            v2(0.0, d.y.signum())
        };
        let q = if gap_x > 0.0 && gap_y > 0.0 {
            if out.x != 0.0 {
                v2(self.half.x * out.x, d.y)
            } else {
                v2(d.x, self.half.y * out.y)
            }
        } else {
            q
        };
        self.center + (q + out * standoff).rotated(self.yaw)
    }
}

/// A perimeter firing position just outside a facade (contracts: garrisons).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Slot {
    pub position: V2,
    /// The facade's outward normal.
    pub normal: V2,
    /// Which facade: 0 = +x, 1 = +y, 2 = -x, 3 = -y in the prop's frame.
    pub facade: u8,
}

impl Slot {
    /// Whether a round from this slot toward `p` leaves the facade outward by
    /// more than `min_angle` (radians): wide of a round's truncated spread, so
    /// it never grazes back into its own wall.
    pub fn faces(&self, p: V2, min_angle: f64) -> bool {
        let d = p - self.position;
        let len = d.length();
        len > 0.0 && d.dot(self.normal) > min_angle.sin() * len
    }
}

/// Uniform XY bucket grid over prop footprints.
pub struct PropIndex {
    bucket: f64,
    nx: usize,
    ny: usize,
    cells: Vec<Vec<Entry>>,
    changes: Option<BucketChanges>,
}

struct BucketChanges {
    revision: u64,
    stamps: Vec<u64>,
}

/// A prop in a bucket, with its footprint's bounding circle, so a segment
/// query can pass it by without reading the prop.
#[derive(Clone, Copy)]
struct Entry {
    id: PropId,
    center: V2,
    radius: f64,
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
            changes: None,
        }
    }

    /// World enables tracking after authored setup. Side-known indexes do not
    /// allocate stamps: their consumers already own their change histories.
    pub(super) fn track_changes(&mut self) {
        self.changes = Some(BucketChanges {
            revision: 0,
            stamps: vec![0; self.cells.len()],
        });
    }

    fn range(&self, center: V2, radius: f64) -> (usize, usize, usize, usize) {
        let clamp = |v: f64, n: usize| ((v / self.bucket).floor().max(0.0) as usize).min(n - 1);
        (
            clamp(center.x - radius, self.nx),
            clamp(center.x + radius, self.nx),
            clamp(center.y - radius, self.ny),
            clamp(center.y + radius, self.ny),
        )
    }

    fn cell_range(&self, p: &Prop) -> (usize, usize, usize, usize) {
        self.range(p.center, p.footprint_radius())
    }

    fn note_change(&mut self, (i0, i1, j0, j1): (usize, usize, usize, usize)) {
        if let Some(changes) = &mut self.changes {
            changes.revision += 1;
            for j in j0..=j1 {
                changes.stamps[j * self.nx + i0..=j * self.nx + i1].fill(changes.revision);
            }
        }
    }

    /// Newest mutation in the same candidate buckets as `near`. Empty buckets
    /// keep their stamp so removal invalidates a previously occupied raster.
    pub(super) fn revision_near(&self, center: V2, radius: f64) -> u64 {
        let changes = self.changes.as_ref().expect("tracked world index");
        let (i0, i1, j0, j1) = self.range(center, radius);
        (j0..=j1)
            .flat_map(|j| &changes.stamps[j * self.nx + i0..=j * self.nx + i1])
            .copied()
            .max()
            .unwrap()
    }

    pub fn insert(&mut self, p: &Prop) {
        let (i0, i1, j0, j1) = self.cell_range(p);
        self.note_change((i0, i1, j0, j1));
        let entry = Entry {
            id: p.id,
            center: p.center,
            radius: p.footprint_radius(),
        };
        for j in j0..=j1 {
            for i in i0..=i1 {
                self.cells[j * self.nx + i].push(entry);
            }
        }
    }

    pub fn remove(&mut self, p: &Prop) {
        let (i0, i1, j0, j1) = self.cell_range(p);
        self.note_change((i0, i1, j0, j1));
        for j in j0..=j1 {
            for i in i0..=i1 {
                self.cells[j * self.nx + i].retain(|e| e.id != p.id);
            }
        }
    }

    /// Candidate ids whose footprint circle may meet the XY disc.
    pub fn near(&self, center: V2, radius: f64, out: &mut Vec<PropId>) {
        self.append_near(center, radius, out);
        out.sort_unstable();
        out.dedup();
    }

    /// Append bucket candidates without ordering or uniqueness. A caller
    /// collecting several views canonicalizes their union once before reading it.
    pub fn append_near(&self, center: V2, radius: f64, out: &mut Vec<PropId>) {
        let (i0, i1, j0, j1) = self.range(center, radius);
        for j in j0..=j1 {
            for i in i0..=i1 {
                out.extend(self.cells[j * self.nx + i].iter().map(|e| e.id));
            }
        }
    }

    /// Whether `hit` holds for some prop whose footprint circle the XY
    /// segment `a`→`b` passes within a millimetre of, stopping at the first.
    /// Every prop a segment's hit could lie in is offered (a hit lies on the
    /// segment inside the footprint, so inside its circle). Ids are not
    /// deduplicated: one in several buckets may be offered more than once.
    pub fn any_along(&self, a: V2, b: V2, mut hit: impl FnMut(PropId) -> bool) -> bool {
        let ab = b - a;
        let len2 = ab.dot(ab);
        let meets = |e: &Entry| {
            let t = if len2 > 0.0 {
                ((e.center - a).dot(ab) / len2).clamp(0.0, 1.0)
            } else {
                0.0
            };
            (a + ab * t - e.center).length() <= e.radius + 1e-3
        };
        self.buckets_crossed(a, b, |entries| {
            entries.iter().any(|e| meets(e) && hit(e.id))
        })
    }

    /// Visit, until `visit` returns true, every bucket holding a point
    /// within a centimetre of the XY segment `a`→`b`: column by column,
    /// only the rows the segment spans there (a strip of buckets, not the
    /// segment's bounding box).
    fn buckets_crossed(&self, a: V2, b: V2, mut visit: impl FnMut(&[Entry]) -> bool) -> bool {
        const EPS: f64 = 1e-2;
        let cell = |v: f64, n: usize| ((v / self.bucket).floor().max(0.0) as usize).min(n - 1);
        // The segment's y over x ∈ [lo, hi] (clamped to its ends).
        let dx = b.x - a.x;
        let y_at = |x: f64| {
            if dx.abs() < 1e-12 {
                a.y
            } else {
                a.y + (b.y - a.y) * ((x - a.x) / dx).clamp(0.0, 1.0)
            }
        };
        let (x0, x1) = (a.x.min(b.x), a.x.max(b.x));
        for i in cell(x0 - EPS, self.nx)..=cell(x1 + EPS, self.nx) {
            // The column's span, the edge columns open to the map's outside
            // (the index clamps there too), widened by the margin.
            let lo = if i == 0 {
                f64::NEG_INFINITY
            } else {
                i as f64 * self.bucket
            };
            let hi = if i + 1 == self.nx {
                f64::INFINITY
            } else {
                (i + 1) as f64 * self.bucket
            };
            let (lo, hi) = ((lo - EPS).max(x0), (hi + EPS).min(x1));
            let (ya, yb) = if dx.abs() < 1e-12 {
                (a.y, b.y)
            } else {
                (y_at(lo), y_at(hi))
            };
            for j in cell(ya.min(yb) - EPS, self.ny)..=cell(ya.max(yb) + EPS, self.ny) {
                if visit(&self.cells[j * self.nx + i]) {
                    return true;
                }
            }
        }
        false
    }

    /// Candidate ids in the buckets the XY projection of the segment crosses
    /// (every prop a segment's hit could lie in), ascending.
    pub fn along(&self, a: V2, b: V2, out: &mut Vec<PropId>) {
        self.buckets_crossed(a, b, |entries| {
            out.extend(entries.iter().map(|e| e.id));
            false
        });
        out.sort_unstable();
        out.dedup();
    }
}

/// Slab test of `o + d * t`, t ∈ [0, max_t], against the box of half extents
/// `half` centred at the local origin. Returns (t, local outward normal) of the
/// entry point; an origin already inside hits at t = 0.
pub(crate) fn ray_box(o: V3, d: V3, half: V3, max_t: f64) -> Option<(f64, V3)> {
    let (mut t0, mut t1) = (f64::NEG_INFINITY, f64::INFINITY);
    let mut axis0 = 0usize;
    let mut sign0 = 0.0;
    let h = [half.x, half.y, half.z];
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
    Some((t, v3(n[0], n[1], n[2])))
}
