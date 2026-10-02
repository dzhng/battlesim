//! Minimal f64 vector math for the authority. Presentation receives f32 copies.
//! Authoritative geometry uses the pinned libm sin/cos/sincos/atan2/hypot
//! evaluators so Native and Wasm carry the same float64 state.
use std::ops::{Add, Mul, Neg, Sub};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct V2 {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct V3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

pub const fn v2(x: f64, y: f64) -> V2 {
    V2 { x, y }
}

pub const fn v3(x: f64, y: f64, z: f64) -> V3 {
    V3 { x, y, z }
}

impl V2 {
    pub fn dot(self, o: V2) -> f64 {
        self.x * o.x + self.y * o.y
    }
    pub fn cross(self, o: V2) -> f64 {
        self.x * o.y - self.y * o.x
    }
    pub fn length(self) -> f64 {
        libm::hypot(self.x, self.y)
    }
    pub fn normalized(self) -> V2 {
        let l = self.length();
        if l > 0.0 {
            self * (1.0 / l)
        } else {
            self
        }
    }
    pub fn rotated(self, yaw: f64) -> V2 {
        Rotation::new(yaw).apply(self)
    }
    pub fn with_z(self, z: f64) -> V3 {
        v3(self.x, self.y, z)
    }
}

impl V3 {
    pub fn dot(self, o: V3) -> f64 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
    pub fn cross(self, o: V3) -> V3 {
        v3(
            self.y * o.z - self.z * o.y,
            self.z * o.x - self.x * o.z,
            self.x * o.y - self.y * o.x,
        )
    }
    pub fn length(self) -> f64 {
        self.dot(self).sqrt()
    }
    pub fn normalized(self) -> V3 {
        let l = self.length();
        if l > 0.0 {
            self * (1.0 / l)
        } else {
            self
        }
    }
    pub fn xy(self) -> V2 {
        v2(self.x, self.y)
    }
}

/// An angle wrapped into [-π, π).
pub fn wrap_angle(a: f64) -> f64 {
    let t = std::f64::consts::TAU;
    (a + std::f64::consts::PI).rem_euclid(t) - std::f64::consts::PI
}

/// A turn by a fixed angle with its sine and cosine worked out once, for
/// loops that rotate many points the same way. [`V2::rotated`] is this, built
/// per call, so both give bit-identical results.
#[derive(Clone, Copy, Debug)]
pub struct Rotation {
    sin: f64,
    cos: f64,
}

impl Rotation {
    pub fn new(yaw: f64) -> Self {
        // Cached and one-shot rotations must agree across Native and Wasm.
        let (sin, cos) = libm::sincos(yaw);
        Rotation { sin, cos }
    }

    pub fn apply(self, v: V2) -> V2 {
        let (s, c) = (self.sin, self.cos);
        v2(v.x * c - v.y * s, v.x * s + v.y * c)
    }
}

/// An oriented rectangle on the ground: centre, heading and half extents
/// (along heading, across heading).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Obb2 {
    pub center: V2,
    pub yaw: f64,
    pub half: V2,
}

impl Obb2 {
    /// World point → the rectangle's frame (origin at its centre).
    pub fn to_local(&self, p: V2) -> V2 {
        (p - self.center).rotated(-self.yaw)
    }

    /// Distance from `p` to the rectangle (0 inside).
    pub fn distance(&self, p: V2) -> f64 {
        let d = self.to_local(p);
        let (x, y) = (
            (d.x.abs() - self.half.x).max(0.0),
            (d.y.abs() - self.half.y).max(0.0),
        );
        libm::hypot(x, y)
    }

    /// Whether `p` lies inside, the rectangle grown by `margin` on each side.
    pub fn contains(&self, p: V2, margin: f64) -> bool {
        let d = self.to_local(p);
        d.x.abs() <= self.half.x + margin && d.y.abs() <= self.half.y + margin
    }

    /// Whether the segment `a`→`b` passes through the rectangle grown by
    /// `margin` on each side.
    pub fn meets_segment(&self, a: V2, b: V2, margin: f64) -> bool {
        self.clip_segment(a, b, margin).is_some()
    }

    /// The span `[t0, t1]` of the segment `a + (b - a) t`, t ∈ [0, 1], inside
    /// the rectangle grown by `margin` on each side (a slab clip in the
    /// rectangle's frame); `None` when it misses.
    pub fn clip_segment(&self, a: V2, b: V2, margin: f64) -> Option<(f64, f64)> {
        let to_local = Rotation::new(-self.yaw);
        let (p, q) = (
            to_local.apply(a - self.center),
            to_local.apply(b - self.center),
        );
        let d = q - p;
        let (mut t0, mut t1) = (0.0f64, 1.0f64);
        for (o, dv, h) in [
            (p.x, d.x, self.half.x + margin),
            (p.y, d.y, self.half.y + margin),
        ] {
            if dv.abs() < 1e-12 {
                if o.abs() > h {
                    return None;
                }
                continue;
            }
            let (ta, tb) = ((-h - o) / dv, (h - o) / dv);
            t0 = t0.max(ta.min(tb));
            t1 = t1.min(ta.max(tb));
            if t0 > t1 {
                return None;
            }
        }
        Some((t0, t1))
    }

    /// `p` moved out of the rectangle grown by `margin` through the nearest
    /// side, keeping its motion along that side: how a body slides on a wall.
    pub fn push_out(&self, p: V2, margin: f64) -> V2 {
        const CLEAR_M: f64 = 1e-6;
        let mut d = self.to_local(p);
        let (hx, hy) = (self.half.x + margin, self.half.y + margin);
        if hx - d.x.abs() < hy - d.y.abs() {
            d.x = (hx + CLEAR_M).copysign(d.x);
        } else {
            d.y = (hy + CLEAR_M).copysign(d.y);
        }
        self.center + d.rotated(self.yaw)
    }

    /// The least translation that moves `o` clear of this rectangle, by the
    /// separating axes (box against box, L8): the axis of least overlap,
    /// pointing from this rectangle toward `o`. `None` when they are apart
    /// or only touch.
    pub fn separation(&self, o: &Obb2) -> Option<V2> {
        let (depth, axis) = self.least_overlap(o);
        (depth > 0.0).then(|| axis * depth)
    }

    /// Separating-axis overlap test (touching counts).
    pub fn overlaps(&self, o: &Obb2) -> bool {
        self.least_overlap(o).0 >= 0.0
    }

    /// The least overlap over the four separating axes and its axis, pointing
    /// from this rectangle toward `o` (the first axis on a tie); a depth of
    /// zero or less means apart along that axis.
    fn least_overlap(&self, o: &Obb2) -> (f64, V2) {
        let axes = [
            v2(1.0, 0.0).rotated(self.yaw),
            v2(0.0, 1.0).rotated(self.yaw),
            v2(1.0, 0.0).rotated(o.yaw),
            v2(0.0, 1.0).rotated(o.yaw),
        ];
        let project = |r: &Obb2, axis: V2| {
            let ax = v2(1.0, 0.0).rotated(r.yaw);
            let ay = v2(0.0, 1.0).rotated(r.yaw);
            r.half.x * ax.dot(axis).abs() + r.half.y * ay.dot(axis).abs()
        };
        let d = o.center - self.center;
        let mut best: Option<(f64, V2)> = None;
        for axis in axes {
            let depth = project(self, axis) + project(o, axis) - d.dot(axis).abs();
            if best.is_none_or(|(b, _)| depth < b) {
                let out = if d.dot(axis) < 0.0 { -axis } else { axis };
                best = Some((depth, out));
            }
        }
        best.expect("four axes")
    }
}

macro_rules! impl_ops {
    ($t:ident, $($f:ident),+) => {
        impl Add for $t { type Output = $t; fn add(self, o: $t) -> $t { $t { $($f: self.$f + o.$f),+ } } }
        impl Sub for $t { type Output = $t; fn sub(self, o: $t) -> $t { $t { $($f: self.$f - o.$f),+ } } }
        impl Mul<f64> for $t { type Output = $t; fn mul(self, s: f64) -> $t { $t { $($f: self.$f * s),+ } } }
        impl Neg for $t { type Output = $t; fn neg(self) -> $t { $t { $($f: -self.$f),+ } } }
    };
}
impl_ops!(V2, x, y);
impl_ops!(V3, x, y, z);
