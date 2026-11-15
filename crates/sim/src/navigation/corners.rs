//! A wheeled route made drivable. A route's corners are where roads and
//! searches turn, and a corner sharper than a vehicle can take at full lock
//! would have it back and fill. A driver starts the turn before the corner
//! and sweeps round it, leaving the road for a few metres where the corner
//! is tight: so each corner becomes an arc at a little more than the
//! vehicle's turning radius, tangent to the ways in and out, wherever the
//! footprint fits along it on what the side knows, over ground about as
//! quick as open ground or the corner itself. Where it does not, the
//! arc is tried moved out from the corner and in toward it a metre at a
//! time, eased into and out of with a short straight. Corners close
//! together (a bend of short runs) are rounded as one. A corner no arc fits
//! is left for the follower's three-point turn. Each step tries one arc, so
//! rounding is counted planning work like the rest of a journey.
use super::{Mobility, NavGrid};
use crate::digest::Digest;
use crate::math::{v2, Obb2, V2};

/// An arc is driven this much wider than the tightest turn, so a follower
/// a little off its line still has the turn in hand.
const ROUND_MARGIN: f64 = 1.25;
/// A turn gentler than this is no corner.
const MIN_TURN_RAD: f64 = 0.05;
/// No corner is rounded that turns more than this, merged or not: a way
/// that doubles back is the follower's to turn round on.
const MAX_TURN_RAD: f64 = 2.6;
/// An arc's chords each turn at most this far.
const CHORD_RAD: f64 = 0.26;
/// How far an arc that does not fit is tried moved out from the corner and
/// in toward it, a metre at a time.
const SHIFTS_M: [f64; 7] = [0.0, 1.0, -1.0, 2.0, -2.0, 3.0, -3.0];
/// A moved arc is eased onto from this far back per metre it moved.
const EASE_PER_M: f64 = 4.0;
/// Corners rounded as one at most.
const MAX_MERGED: usize = 6;
/// An arc may cross ground no slower than this share of the slower of open
/// ground and the corner it replaces: over a verge, never into a bog.
const QUICK_SHARE: f64 = 0.75;

/// A route being rounded, a corner at a time.
pub(super) struct Rounding {
    /// The route's start, then its waypoints.
    points: Vec<V2>,
    /// The corner in hand, and how many shifts of its arc were tried.
    at: usize,
    shift: usize,
    radius: f64,
}

/// One way round a corner: the waypoints that replace the corners
/// `points[first..=last]`.
struct Way {
    last: usize,
    points: Vec<V2>,
}

impl Rounding {
    /// The rounding of the route `from` → `route` for `m`, if its corners
    /// can need it: a wheeled vehicle's, with a corner to round.
    pub fn new(from: V2, route: &[V2], m: &Mobility) -> Option<Self> {
        let drive = m.drive.filter(|d| !d.tracked && d.radius_m > 0.0)?;
        (route.len() >= 2).then(|| Rounding {
            points: std::iter::once(from).chain(route.iter().copied()).collect(),
            at: 1,
            shift: 0,
            radius: drive.radius_m * ROUND_MARGIN,
        })
    }

    /// Try one arc; the rounded route (after its start) once every corner
    /// is done.
    pub fn step(&mut self, grid: &NavGrid, m: &Mobility, avoid: &[Obb2]) -> Option<Vec<V2>> {
        if self.at + 1 >= self.points.len() {
            return Some(self.points[1..].to_vec());
        }
        let Some(shift) = SHIFTS_M.get(self.shift).copied() else {
            self.next(1);
            return None;
        };
        self.shift += 1;
        let Some(way) = self.way(shift) else {
            // Nothing to round here, or no room to round it in.
            self.next(1);
            return None;
        };
        let first = self.at;
        let before = self.points[first - 1];
        let after = self.points[way.last + 1];
        let mut path = vec![before];
        path.extend(&way.points);
        path.push(after);
        // A piece that lies on the way it replaces was checked with it.
        let on = |a: V2, b: V2, p: V2| {
            let ab = b - a;
            let t = (p - a).dot(ab) / ab.dot(ab).max(1e-12);
            (0.0..=1.0).contains(&t) && (a + ab * t - p).length() < 1e-6
        };
        let skip_first = on(before, self.points[first], path[1]);
        let skip_last = on(self.points[way.last], after, path[path.len() - 2]);
        let new = &path[usize::from(skip_first)..path.len() - usize::from(skip_last)];
        let mut old = vec![new[0]];
        old.extend(&self.points[first..=way.last]);
        old.push(new[new.len() - 1]);
        // How quick each piece is, if it fits.
        let pace = |piece: &[V2]| {
            let length = (piece[1] - piece[0]).length();
            grid.way_time(piece[0], piece[1], m, avoid).map(|t| {
                if t > 0.0 {
                    length / t
                } else {
                    f64::INFINITY
                }
            })
        };
        let slowest_old = old
            .windows(2)
            .map(|piece| pace(piece).unwrap_or(0.0))
            .fold(f64::INFINITY, f64::min);
        let floor = slowest_old.min(m.off_road_mps) * QUICK_SHARE;
        let fits = new
            .windows(2)
            .all(|piece| pace(piece).is_some_and(|pace| pace >= floor));
        if fits {
            let placed = way.points.len();
            self.points.splice(first..=way.last, way.points);
            self.next(placed);
        }
        None
    }

    fn next(&mut self, by: usize) {
        self.at += by;
        self.shift = 0;
    }

    /// The arc round the corner in hand, moved `shift` metres out from it
    /// (in toward it when negative), merged with the corners after it while
    /// they are too close to round apart.
    fn way(&self, shift: f64) -> Option<Way> {
        let p = &self.points;
        let first = self.at;
        let a = p[first - 1];
        let dir = |from: V2, to: V2| {
            let d = to - from;
            let l = d.length();
            (l > 1e-9).then(|| d * (1.0 / l))
        };
        let u = dir(a, p[first])?;
        let mut last = first;
        let mut corner = p[first];
        loop {
            let b = p[last + 1];
            let w = dir(corner, b)?;
            let turn = libm::atan2(u.cross(w), u.dot(w));
            if turn.abs() < MIN_TURN_RAD || turn.abs() > MAX_TURN_RAD {
                return None;
            }
            let reach = self.radius * libm::tan(turn.abs() / 2.0);
            // The way out is shared with the next corner's own reach.
            let out_room = if last + 2 < p.len() {
                let after = dir(b, p[last + 2]);
                let next = after.map_or(0.0, |after| {
                    let t = libm::atan2(w.cross(after), w.dot(after)).abs();
                    if t < MIN_TURN_RAD {
                        0.0
                    } else {
                        self.radius * libm::tan(t.min(MAX_TURN_RAD) / 2.0)
                    }
                });
                (b - corner).length() - next.min((b - corner).length() / 2.0)
            } else {
                (b - corner).length()
            };
            let in_room = (corner - a).length();
            let ease = shift.abs() * EASE_PER_M;
            if reach + ease <= in_room && reach + ease <= out_room {
                return Some(Way {
                    last,
                    points: arc(corner, u, w, turn, reach, self.radius, shift, ease),
                });
            }
            // Too tight to round alone: round it with the next corner, at
            // the point where the way in and the way on from there meet.
            if last + 2 >= p.len() || last + 1 - first >= MAX_MERGED {
                return None;
            }
            let on = dir(b, p[last + 2])?;
            let across = u.cross(on);
            // Only a bend turning one way is one corner; an S is two.
            if across.abs() < 1e-9 || across.signum() != turn.signum() || w.cross(on) * turn < 0.0 {
                return None;
            }
            let t = (b - a).cross(on) / across;
            let meet = a + u * t;
            if t <= 0.0 || (meet - b).dot(on) > 0.0 {
                return None;
            }
            corner = meet;
            last += 1;
        }
    }

    pub fn digest(&self, d: &mut Digest) {
        d.u64(self.points.len() as u64)
            .u64(self.at as u64)
            .u64(self.shift as u64);
        for p in &self.points {
            d.f64(p.x).f64(p.y);
        }
    }
}

/// The waypoints of an arc of `radius` round `corner` (the way in along
/// `u`, out along `w`, turning `turn`), its ends `reach` from the corner,
/// moved `shift` out from the corner and eased onto from `ease` back.
#[allow(clippy::too_many_arguments)]
fn arc(
    corner: V2,
    u: V2,
    w: V2,
    turn: f64,
    reach: f64,
    radius: f64,
    shift: f64,
    ease: f64,
) -> Vec<V2> {
    let side = turn.signum();
    let inward = v2(-u.y, u.x) * side;
    let start = corner - u * reach;
    let centre = start + inward * radius;
    let outward = (u - w).normalized();
    let moved = outward * shift;
    let chords = (turn.abs() / CHORD_RAD).ceil().max(1.0) as usize;
    let mut points = Vec::with_capacity(chords + 3);
    if ease > 0.0 {
        points.push(corner - u * (reach + ease));
    }
    let from = start - centre;
    for k in 0..=chords {
        let r = from.rotated(turn * k as f64 / chords as f64);
        points.push(centre + r + moved);
    }
    if ease > 0.0 {
        points.push(corner + w * (reach + ease));
    }
    points
}
