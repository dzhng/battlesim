//! Swept chord-versus-body queries in relative motion. Over one chord both the
//! round and the body move linearly, so in the body's frame the round follows
//! one straight relative segment `r(u)`, u ∈ [0, 1]. Capsules are upright, so
//! only translation matters. A turning box is bounded conservatively: over a
//! piece of the interval, the box at the piece's mid heading grown by the
//! farthest any corner turns away contains every pose; pieces that hit are
//! halved until that growth is under [`TURN_TOLERANCE_M`].
use super::{Body, Shape};
use crate::math::{v3, V3};
use crate::world::ray_box;

/// Largest overestimate a turning box's hit may carry.
const TURN_TOLERANCE_M: f64 = 1e-3;
/// Bounds the narrowing; at the cap the grown box is accepted (still conservative).
const MAX_TURN_SPLITS: u32 = 20;
const GOLDEN_STEPS: u32 = 60;

/// A body's motion over one chord's interval of the tick.
pub(super) struct Motion {
    base0: V3,
    base1: V3,
    yaw0: f64,
    turn: f64,
}

impl Motion {
    pub fn new(body: &Body, s0: f64, s1: f64) -> Self {
        let (p0, p1) = (body.pose_at(s0), body.pose_at(s1));
        Motion {
            base0: p0.base,
            base1: p1.base,
            yaw0: p0.yaw,
            turn: p1.yaw - p0.yaw,
        }
    }

    /// Round position `a0 → a1` relative to the body's base plus `lift`, at `u`.
    fn relative(&self, a0: V3, a1: V3, lift: V3, u: f64) -> V3 {
        (a0 + (a1 - a0) * u) - (self.base0 + (self.base1 - self.base0) * u + lift)
    }
}

/// Earliest entry of the chord `a0 → a1` into the moving body: (u, world normal).
pub(super) fn entry(shape: &Shape, m: &Motion, a0: V3, a1: V3) -> Option<(f64, V3)> {
    match *shape {
        Shape::Capsule { radius, height } => {
            let o = m.relative(a0, a1, V3::default(), 0.0);
            let d = m.relative(a0, a1, V3::default(), 1.0) - o;
            capsule_entry(o, d, radius, height)
        }
        Shape::Box { half } => box_entry(half, m, a0, a1, 0.0, 1.0, 0),
    }
}

fn box_entry(
    half: V3,
    m: &Motion,
    a0: V3,
    a1: V3,
    ua: f64,
    ub: f64,
    depth: u32,
) -> Option<(f64, V3)> {
    let lift = v3(0.0, 0.0, half.z);
    let (ra, rb) = (m.relative(a0, a1, lift, ua), m.relative(a0, a1, lift, ub));
    let yaw = m.yaw0 + m.turn * (ua + ub) * 0.5;
    // A point at radius ρ turned by δ from the mid heading moves at most ρ·|δ|.
    let margin = half.x.hypot(half.y) * (m.turn * (ub - ua)).abs() * 0.5;
    let local = |v: V3| {
        let r = v.xy().rotated(-yaw);
        v3(r.x, r.y, v.z)
    };
    let grown = v3(half.x + margin, half.y + margin, half.z);
    let (t, n) = ray_box(local(ra), local(rb - ra), grown, 1.0)?;
    if margin > TURN_TOLERANCE_M && depth < MAX_TURN_SPLITS {
        let mid = (ua + ub) * 0.5;
        return box_entry(half, m, a0, a1, ua, mid, depth + 1)
            .or_else(|| box_entry(half, m, a0, a1, mid, ub, depth + 1));
    }
    let nw = n.xy().rotated(yaw);
    Some((ua + t * (ub - ua), v3(nw.x, nw.y, n.z)))
}

/// Entry of `o + d·u`, u ∈ [0, 1], into an upright capsule standing on the
/// origin: the union of the side cylinder between the cap centres and the two
/// cap spheres, whose first touch is the earliest entry among the three.
fn capsule_entry(o: V3, d: V3, radius: f64, height: f64) -> Option<(f64, V3)> {
    let (z0, z1) = (radius, (height - radius).max(radius));
    let mut best: Option<f64> = None;
    let mut consider = |span: Option<(f64, f64)>| {
        if let Some((enter, exit)) = span {
            if enter <= exit && exit >= 0.0 && enter <= 1.0 {
                let u = enter.max(0.0);
                best = Some(best.map_or(u, |b: f64| b.min(u)));
            }
        }
    };
    let side = within_quadratic(
        d.x * d.x + d.y * d.y,
        2.0 * (o.x * d.x + o.y * d.y),
        o.x * o.x + o.y * o.y - radius * radius,
    )
    .zip(within_slab(o.z, d.z, z0, z1))
    .map(|((a, b), (c, e))| (a.max(c), b.min(e)));
    consider(side);
    for cz in [z0, z1] {
        let oc = o - v3(0.0, 0.0, cz);
        consider(within_quadratic(
            d.dot(d),
            2.0 * oc.dot(d),
            oc.dot(oc) - radius * radius,
        ));
    }
    let u = best?;
    let q = o + d * u;
    let out = q - v3(0.0, 0.0, q.z.clamp(z0, z1));
    let n = if out.length() > 1e-12 {
        out.normalized()
    } else {
        (-d).normalized()
    };
    Some((u, n))
}

/// Parameter interval where `a·u² + b·u + c ≤ 0` (a ≥ 0).
fn within_quadratic(a: f64, b: f64, c: f64) -> Option<(f64, f64)> {
    if a < 1e-18 {
        return (c <= 0.0).then_some((f64::NEG_INFINITY, f64::INFINITY));
    }
    let disc = b * b - 4.0 * a * c;
    if disc < 0.0 {
        return None;
    }
    // Numerically stable roots.
    let q = -0.5 * (b + b.signum() * disc.sqrt());
    if q == 0.0 {
        return Some((0.0, 0.0));
    }
    let (r0, r1) = (q / a, c / q);
    Some((r0.min(r1), r0.max(r1)))
}

/// Parameter interval where `lo ≤ o + d·u ≤ hi`.
fn within_slab(o: f64, d: f64, lo: f64, hi: f64) -> Option<(f64, f64)> {
    if d.abs() < 1e-18 {
        return (lo..=hi)
            .contains(&o)
            .then_some((f64::NEG_INFINITY, f64::INFINITY));
    }
    let (a, b) = ((lo - o) / d, (hi - o) / d);
    Some((a.min(b), a.max(b)))
}

/// Closest approach of the chord's first `u_end` to the body's surface, if
/// within `reach`: (distance, u). The box is held at the chord's mid heading;
/// a near miss is a proximity measure, not a hit.
#[allow(clippy::too_many_arguments)]
pub(super) fn closest_approach(
    shape: &Shape,
    m: &Motion,
    bound_radius: f64,
    a0: V3,
    a1: V3,
    u_end: f64,
    reach: f64,
) -> Option<(f64, f64)> {
    let (lift, reach_z) = match *shape {
        Shape::Capsule { height, .. } => (v3(0.0, 0.0, height * 0.5), height * 0.5),
        Shape::Box { half } => (v3(0.0, 0.0, half.z), half.z),
    };
    // Cheap reject against the body's bounding sphere.
    let o = m.relative(a0, a1, lift, 0.0);
    let d = m.relative(a0, a1, lift, 1.0) - o;
    let dd = d.dot(d);
    let u_centre = if dd > 0.0 {
        (-o.dot(d) / dd).clamp(0.0, u_end)
    } else {
        0.0
    };
    if (o + d * u_centre).length() > bound_radius.hypot(reach_z) + reach {
        return None;
    }
    let yaw = m.yaw0 + m.turn * u_end * 0.5;
    // Distance to a convex set along a line is convex: golden-section search.
    let g = |u: f64| surface_distance(shape, yaw, o + d * u);
    let ratio = (5f64.sqrt() - 1.0) * 0.5;
    let (mut lo, mut hi) = (0.0, u_end);
    let (mut x1, mut x2) = (hi - ratio * (hi - lo), lo + ratio * (hi - lo));
    let (mut f1, mut f2) = (g(x1), g(x2));
    for _ in 0..GOLDEN_STEPS {
        if f1 <= f2 {
            hi = x2;
            x2 = x1;
            f2 = f1;
            x1 = hi - ratio * (hi - lo);
            f1 = g(x1);
        } else {
            lo = x1;
            x1 = x2;
            f1 = f2;
            x2 = lo + ratio * (hi - lo);
            f2 = g(x2);
        }
    }
    let u = (lo + hi) * 0.5;
    let best = [(g(0.0), 0.0), (g(u), u), (g(u_end), u_end)]
        .into_iter()
        .fold(
            (f64::INFINITY, 0.0),
            |acc, c| if c.0 < acc.0 { c } else { acc },
        );
    (best.0 <= reach).then_some(best)
}

/// Distance from `q` (relative to the body's centre) to its surface; zero inside.
fn surface_distance(shape: &Shape, yaw: f64, q: V3) -> f64 {
    match *shape {
        Shape::Capsule { radius, height } => {
            let half_axis = (height * 0.5 - radius).max(0.0);
            ((q - v3(0.0, 0.0, q.z.clamp(-half_axis, half_axis))).length() - radius).max(0.0)
        }
        Shape::Box { half } => {
            let r = q.xy().rotated(-yaw);
            v3(
                (r.x.abs() - half.x).max(0.0),
                (r.y.abs() - half.y).max(0.0),
                (q.z.abs() - half.z).max(0.0),
            )
            .length()
        }
    }
}
