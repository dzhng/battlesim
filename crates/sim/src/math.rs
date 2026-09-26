//! Minimal f64 vector math for the authority. Presentation receives f32 copies.
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
        self.x.hypot(self.y)
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
        let (sin, cos) = yaw.sin_cos();
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

    /// Whether `p` lies inside, the rectangle grown by `margin` on each side.
    pub fn contains(&self, p: V2, margin: f64) -> bool {
        let d = self.to_local(p);
        d.x.abs() <= self.half.x + margin && d.y.abs() <= self.half.y + margin
    }

    /// Separating-axis overlap test (touching counts).
    pub fn overlaps(&self, o: &Obb2) -> bool {
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
        axes.iter().all(|&axis| {
            (o.center - self.center).dot(axis).abs() <= project(self, axis) + project(o, axis)
        })
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
