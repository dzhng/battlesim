//! Moving hulls a soldier must step clear of.
use super::{left, v2, MovementContext, Unit, V2};

/// A live vehicle on the move, as soldiers see it coming (Q23): its hull's
/// half width and the path its centre will run over the next
/// `yield_horizon_s`, reaching on to its nose.
pub struct Threat {
    pub half_width: f64,
    pub path: Vec<V2>,
}

impl Threat {
    /// The threat a moving vehicle poses, or `None` for one standing still.
    pub fn of(ctx: &MovementContext, unit: &Unit) -> Option<Threat> {
        let hull = unit.ground_footprint()?;
        let route = unit.route.as_ref().filter(|r| !r.is_empty())?;
        if unit.halted() || !unit.may_translate() {
            return None;
        }
        let here = unit.position.xy();
        let speed = unit.drive_speed_mps;
        let mut left = speed.abs() * ctx.rules.infantry_movement.yield_horizon_s + hull.half.x;
        if speed < 0.0 {
            let backwards = v2(-1.0, 0.0).rotated(unit.yaw);
            return Some(Threat {
                half_width: hull.half.y,
                path: vec![here, here + backwards * left],
            });
        }
        let mut path = vec![here];
        let mut at = here;
        for &w in route {
            let span = (w - at).length();
            if span >= left {
                if span > 1e-9 {
                    path.push(at + (w - at) * (left / span));
                }
                left = 0.0;
                break;
            }
            path.push(w);
            left -= span;
            at = w;
        }
        if left > 0.0 {
            // The route ends: the hull still reaches its nose past the end.
            let dir = v2(1.0, 0.0).rotated(unit.yaw);
            path.push(at + dir * hull.half.x.min(left));
        }
        Some(Threat {
            half_width: hull.half.y,
            path,
        })
    }

    /// Where a soldier at `p` must step to be clear, if its path would cross
    /// him: straight out from its path, on the side he already stands.
    pub(super) fn dodge(&self, p: V2, clear: f64) -> Option<V2> {
        let mut best: Option<(f64, V2, V2)> = None;
        for w in self.path.windows(2) {
            let (a, b) = (w[0], w[1]);
            let d = b - a;
            let l2 = d.dot(d);
            let t = if l2 < 1e-12 {
                0.0
            } else {
                ((p - a).dot(d) / l2).clamp(0.0, 1.0)
            };
            let q = a + d * t;
            let gap = (p - q).length();
            if best.is_none_or(|(g, _, _)| gap < g) {
                best = Some((gap, q, d));
            }
        }
        let (gap, q, d) = best?;
        let reach = self.half_width + clear;
        if gap >= reach {
            return None;
        }
        let out = if gap > 1e-6 {
            (p - q) * (1.0 / gap)
        } else {
            left(d.normalized())
        };
        Some(q + out * (reach + 0.3))
    }
}
