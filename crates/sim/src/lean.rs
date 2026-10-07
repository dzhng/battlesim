//! Leaning out round tall cover. A soldier tucked behind a body
//! taller than his muzzle does not fire through it: he steps out past its
//! edge, fires from there, and tucks back in. The lean point comes only from
//! the body's footprint and where the threat is, never from the body's kind,
//! so a trunk, a building's corner, a hull and a wall's end work alike:
//!
//! - take the footprint's silhouette corners as the threat sees them (the
//!   two corners its view grazes, left and right);
//! - step him past the chosen corner, clear of the footprint by his body
//!   radius and a hand's breadth, level with where he stands;
//! - a corner further than `cover.lean_max_m` from him is no lean: that is a
//!   walk, which is the cover search's business.
//!
//! [`reaches`] is the one test of whether a round fired from a point gets to
//! another: the cover search asks it of spots and lean points, and the fire
//! code asks it to pick where each soldier fires from, so the two cannot
//! drift apart.
use contract::ids::{Side, UnitId};
use contract::scenario::{CoverRules, Rules};

use crate::cover::Tier;
use crate::math::{v2, Obb2, V2, V3};
use crate::units::Unit;
use crate::world::{PropId, WorldGeometry};

/// A round passing a hull closer than this grazes it: its scatter would
/// strike the hull, so the line counts as blocked.
const GRAZE_M: f64 = 0.1;
/// A lean point belongs to the place it was worked out for: a soldier this
/// far from it is not there, and does not lean.
pub const AT_PLACE_M: f64 = 0.5;

pub use contract::observation::LeanSide;

/// What a body a soldier leans round is: a prop (his rounds pass it),
/// or a live vehicle's hull.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Round {
    Prop(PropId),
    Hull(UnitId),
}

/// A lean claimed with a place: from `from`, a step to `at`, on `side`,
/// round `body`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lean {
    pub from: V2,
    pub at: V2,
    pub side: LeanSide,
    pub body: Round,
}

impl Lean {
    /// The prop his rounds pass from the lean point, if the body is one.
    pub fn past(&self) -> Option<PropId> {
        match self.body {
            Round::Prop(id) => Some(id),
            Round::Hull(_) => None,
        }
    }

    /// As the observation publishes it.
    pub fn published(&self) -> contract::observation::MemberLean {
        contract::observation::MemberLean {
            side: self.side,
            at: [self.at.x, self.at.y],
        }
    }

    pub fn digest(&self, d: &mut crate::digest::Digest) {
        d.f64(self.from.x)
            .f64(self.from.y)
            .f64(self.at.x)
            .f64(self.at.y)
            .u64(self.side as u64);
        match self.body {
            Round::Prop(id) => d.u64(0).u64(id as u64),
            Round::Hull(u) => d.u64(1).u64(u.0 as u64),
        };
    }
}

/// The lean points round `rect` for a soldier of `radius` at `p` against a
/// threat at `threat`: past each silhouette corner the threat sees, level
/// with him, within `rules.lean_max_m`, nearest first (the delegated tie-break:
/// the shorter step; on a tie, right). Empty when the threat stands inside
/// the footprint or both corners are too far.
pub fn points(
    rect: &Obb2,
    p: V2,
    threat: V2,
    radius: f64,
    rules: &CoverRules,
) -> Vec<(V2, LeanSide)> {
    let u = rect.center - threat;
    if u.length() < 1e-9 || rect.contains(threat, 0.0) {
        return Vec::new();
    }
    let corners = [(1.0, 1.0), (1.0, -1.0), (-1.0, -1.0), (-1.0, 1.0)]
        .map(|(sx, sy)| rect.center + v2(sx * rect.half.x, sy * rect.half.y).rotated(rect.yaw));
    // The silhouette: the corners whose bearings from the threat are the
    // extremes either side of its view of the centre.
    let bearing = |c: V2| {
        let d = c - threat;
        libm::atan2(u.cross(d), u.dot(d))
    };
    let pick = |better: fn(f64, f64) -> bool| {
        corners
            .into_iter()
            .reduce(|a, b| if better(bearing(b), bearing(a)) { b } else { a })
            .expect("four corners")
    };
    let silhouette = [pick(|a, b| a > b), pick(|a, b| a < b)];
    let to_threat = threat - p;
    let mut out: Vec<(V2, LeanSide, f64)> = Vec::new();
    for c in silhouette {
        let v = (threat - c).normalized();
        // Outward: across the view, away from the footprint.
        let across = v2(-v.y, v.x);
        let n = if across.dot(c - rect.center) >= 0.0 {
            across
        } else {
            -across
        };
        let edge = c + n * (radius + rules.lean_clear_m);
        // Level with him: a step straight sideways across the threat's view.
        let at = edge - v * (edge - p).dot(v);
        let step = (at - p).length();
        if step > rules.lean_max_m {
            continue;
        }
        let side = if to_threat.cross(at - p) > 0.0 {
            LeanSide::Left
        } else {
            LeanSide::Right
        };
        out.push((at, side, step));
    }
    out.sort_by(|a, b| a.2.total_cmp(&b.2).then(b.1.cmp(&a.1)));
    out.dedup_by(|a, b| (a.0 - b.0).length() < 1e-9);
    out.into_iter().map(|(at, side, _)| (at, side)).collect()
}

/// A live vehicle's hull as a body: what a round meets on its way (lean,
/// fire), and what a soldier takes cover behind (by its weight class).
#[derive(Clone, Copy, Debug)]
pub struct Hull {
    pub rect: Obb2,
    pub base: f64,
    pub top: f64,
    pub unit: UnitId,
    pub side: Side,
    /// The cover it gives (Q24); `None` for an immovable class.
    pub tier: Option<Tier>,
}

/// Every live vehicle's hull, either side, in unit order.
pub fn hulls<'a>(units: impl IntoIterator<Item = &'a Unit>, rules: &Rules) -> Vec<Hull> {
    units
        .into_iter()
        .filter(|u| u.hull.is_some() && u.alive())
        .filter_map(|u| {
            let h = u.hull?;
            Some(Hull {
                rect: u.hull_box()?,
                base: u.position.z,
                top: u.position.z + 2.0 * h.z,
                unit: u.id,
                side: u.side,
                tier: crate::cover::vehicle_tier(u, rules),
            })
        })
        .collect()
}

impl Hull {
    /// Whether the segment `a`→`b` passes through the hull's box, or grazes
    /// it, on its way. A segment ending inside the hull is aimed at it, and
    /// meets nothing in the way.
    pub fn meets(&self, a: V3, b: V3) -> bool {
        if self.rect.contains(b.xy(), 0.0) {
            return false;
        }
        let Some((t0, t1)) = self.rect.clip_segment(a.xy(), b.xy(), GRAZE_M) else {
            return false;
        };
        let (z0, z1) = (a.z + (b.z - a.z) * t0, a.z + (b.z - a.z) * t1);
        z0.min(z1) <= self.top && z0.max(z1) >= self.base
    }
}

/// Whether a round fired from `from` reaches `to` on a straight line: no
/// terrain, no body that stops rounds but `past` (the body he leans round,
/// whose own rounds pass it), and no live hull. The one line test the
/// cover search and the fire code share.
pub fn reaches(
    world: &WorldGeometry,
    hulls: &[Hull],
    from: V3,
    to: V3,
    past: Option<PropId>,
) -> bool {
    world.segment_clear_except(from, to, past) && hulls.iter().all(|h| !h.meets(from, to))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cover() -> CoverRules {
        serde_json::from_value(crate::fixtures::test_game()["cover"].clone()).unwrap()
    }

    fn trunk() -> Obb2 {
        Obb2 {
            center: v2(0.0, 0.0),
            yaw: 0.0,
            half: v2(0.35, 0.35),
        }
    }

    #[test]
    fn a_man_squarely_behind_a_trunk_leans_either_side_level_with_him() {
        // The threat far east; he stands west of the trunk.
        let p = v2(-0.85, 0.0);
        let leans = points(&trunk(), p, v2(50.0, 0.0), 0.3, &cover());
        assert_eq!(leans.len(), 2, "{leans:?}");
        for (at, _) in &leans {
            assert!(trunk().distance(*at) >= 0.3, "clear of the trunk: {at:?}");
            assert!((at.x - p.x).abs() < 0.05, "level with him: {at:?}");
        }
        let sides: Vec<_> = leans.iter().map(|l| l.1).collect();
        assert!(sides.contains(&LeanSide::Left) && sides.contains(&LeanSide::Right));
    }

    #[test]
    fn the_middle_of_a_long_hull_is_too_far_from_either_corner() {
        let hull = Obb2 {
            center: v2(0.0, 0.0),
            yaw: 0.0,
            half: v2(1.7, 3.5),
        };
        assert!(points(&hull, v2(-2.2, 0.0), v2(60.0, 0.0), 0.3, &cover()).is_empty());
        let corner = points(&hull, v2(-2.2, 3.2), v2(60.0, 0.0), 0.3, &cover());
        assert_eq!(corner.len(), 1, "only the near corner: {corner:?}");
        assert!(corner[0].0.y > 3.5 + 0.3, "{corner:?}");
    }
}
