//! A vehicle's hull box: the distance to it, and which face a point lies
//! beyond.
use contract::scenario::Face;

use super::Unit;
use crate::math::{Rotation, V2, V3};

impl Unit {
    /// Distance from a point to the hull box (0 inside).
    pub fn hull_distance(&self, p: V3) -> f64 {
        self.hull_frame().distance(p)
    }

    /// The hull box at the unit's pose, for measuring many points against it.
    pub fn hull_frame(&self) -> HullFrame {
        let half = self.hull.expect("vehicle");
        HullFrame {
            center: self.position.xy(),
            mid_z: self.position.z + half.z,
            half,
            to_local: Rotation::new(-self.yaw),
        }
    }

    /// The hull face facing `p` (see [`face_toward`]) at the unit's pose.
    pub fn hull_face(&self, p: V3) -> Face {
        hull_face_at(self.position, self.yaw, self.hull.expect("vehicle"), p)
    }
}

/// A vehicle's hull box at one pose with its rotation worked out once, so a
/// caller measuring many points (the friendly-fire check samples a whole
/// trajectory against each friendly hull) pays one sine and cosine, not one
/// per point. The hoist is explicit because, left to the optimiser, an
/// unrelated change once tipped an inlining decision and cost the endurance
/// battle 40% of its instructions.
pub struct HullFrame {
    center: V2,
    /// Height of the box's middle.
    mid_z: f64,
    half: V3,
    to_local: Rotation,
}

impl HullFrame {
    /// Distance from `p` to the box (0 inside).
    pub fn distance(&self, p: V3) -> f64 {
        let d = self.to_local.apply(p.xy() - self.center);
        let dz = p.z - self.mid_z;
        let ex = (d.x.abs() - self.half.x).max(0.0);
        let ey = (d.y.abs() - self.half.y).max(0.0);
        let ez = (dz.abs() - self.half.z).max(0.0);
        (ex * ex + ey * ey + ez * ez).sqrt()
    }

    /// Whether some point of the segment `a`→`b` might lie within `margin`
    /// of the box: `false` only when every point is certainly farther. A
    /// conservative bound (the box's circumscribed sphere, with a millimetre
    /// of slack for rounding), so a caller that skips the segment on `false`
    /// gets the answer it would have measured point by point.
    pub fn may_come_within(&self, a: V3, b: V3, margin: f64) -> bool {
        let c = self.center.with_z(self.mid_z);
        let ab = b - a;
        let len2 = ab.dot(ab);
        let t = if len2 > 0.0 {
            ((c - a).dot(ab) / len2).clamp(0.0, 1.0)
        } else {
            0.0
        };
        (a + ab * t - c).length() <= margin + self.half.length() + 1e-3
    }
}

/// The face of a hull with half extents `half`, standing on `base` and
/// heading `yaw`, that `p` lies beyond.
pub fn hull_face_at(base: V3, yaw: f64, half: V3, p: V3) -> Face {
    let d = (p.xy() - base.xy()).rotated(-yaw);
    face_toward(d.with_z(p.z - (base.z + half.z)), half)
}

/// The face of a box with half extents `half` (x forward) that a point at
/// `local` (box frame, from its centre) lies beyond: the roof when above more
/// than beside, otherwise front, rear or side by the dominant scaled axis (P10).
pub fn face_toward(local: V3, half: V3) -> Face {
    let (sx, sy, sz) = (local.x / half.x, local.y / half.y, local.z / half.z);
    if sz > sx.abs() && sz > sy.abs() {
        Face::Roof
    } else if sx.abs() >= sy.abs() {
        if sx > 0.0 {
            Face::Front
        } else {
            Face::Rear
        }
    } else {
        Face::Side
    }
}
