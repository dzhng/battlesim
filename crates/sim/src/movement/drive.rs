//! A vehicle's own kinematics (Q29–Q31): how it turns toward its next
//! waypoint this tick. The route is the planner's; the follower owns the
//! kinematics (no kinodynamic planner).
//!
//! Tracks turn at their own rate and pivot on the spot beyond
//! [`TURN_IN_PLACE_DEG`] of heading error. Wheels never pivot: the yaw
//! turns only as the hull rolls, at most `1 / radius` per metre, so a
//! U-turn is an arc. A waypoint inside the turning circle, or a turn whose
//! next metre runs into a solid, starts a three-point turn: a leg driven
//! against the order's direction that keeps turning the same way, until
//! the forward arc to the waypoint is clear.
//!
//! A reverse move (Q31) drives the same way with the travel direction
//! behind the hull, so the facing is held, at the reverse fraction of the
//! speed (Q30).
use contract::command::MoveDirection;

use crate::math::{v2, wrap_angle, Obb2, V2};
use crate::navigation::Drive;
use crate::units::Unit;
use crate::world::WorldGeometry;

/// Tracks turn in place beyond this heading error.
const TURN_IN_PLACE_DEG: f64 = 60.0;
/// A wheeled vehicle reaches a waypoint it passes abeam within this.
pub const ABEAM_M: f64 = 1.5;
/// Headings further off than this count as abeam or behind.
const ABEAM_DEG: f64 = 60.0;
/// A turn looks this far along its arc for a solid.
const PROBE_M: f64 = 1.0;
/// A reversing leg drives at least this far.
const MIN_LEG_M: f64 = 1.0;
/// The waypoint must lie this far outside the turning circle to end a leg.
const CIRCLE_MARGIN_M: f64 = 0.5;
/// Beyond this heading error a wheeled turn counts as a manoeuvre: it
/// probes ahead, and its progress is not a stall.
const TURNING_DEG: f64 = 20.0;
/// A wheeled vehicle slows to this fraction of its speed at full lock.
const TURN_SLOW: f64 = 0.5;

/// A three-point turn's leg against the order's direction (Q29): it turns
/// the hull toward `turn` (+1 counter-clockwise, -1 clockwise).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Manoeuvre {
    pub turn: f64,
    pub driven_m: f64,
}

/// One tick's motion: the yaw the hull ends at, the unit direction its
/// centre moves along, and how far.
#[derive(Clone, Copy, Debug)]
pub struct Motion {
    pub yaw: f64,
    pub heading: V2,
    pub step: f64,
    /// Driving backwards (a reverse move, or a leg of a three-point turn).
    pub backwards: bool,
    /// The turn this motion makes counts as progress (a wheeled turn).
    pub turning: bool,
}

fn gear_sign(direction: MoveDirection) -> f64 {
    match direction {
        MoveDirection::Forward => 1.0,
        MoveDirection::Reverse => -1.0,
    }
}

/// The travel heading of a hull at `yaw` in `gear` (+1 forward, -1 back).
fn travel(yaw: f64, gear: f64) -> f64 {
    if gear < 0.0 {
        yaw + std::f64::consts::PI
    } else {
        yaw
    }
}

fn dir(angle: f64) -> V2 {
    v2(angle.cos(), angle.sin())
}

fn speed_in(drive: &Drive, gear: f64, speed: f64) -> f64 {
    if gear < 0.0 {
        speed * drive.reverse_fraction
    } else {
        speed
    }
}

/// Whether `target` lies inside the turning circle on side `side` of a hull
/// at `at` travelling along `heading`, by more than `margin`.
fn inside_circle(at: V2, heading: f64, side: f64, radius: f64, target: V2, margin: f64) -> bool {
    let centre = at + dir(heading + side * std::f64::consts::FRAC_PI_2) * radius;
    (target - centre).length() < radius - margin
}

/// Whether a hull at `center`/`yaw` would stand in a body that stops it, or
/// off traversable ground (true geometry: what the step itself meets).
fn blocked(world: &WorldGeometry, unit: &Unit, center: V2, yaw: f64) -> bool {
    let half = unit.hull.expect("a vehicle has a hull").xy();
    let here = unit.hull_box().expect("a vehicle has a hull");
    let hull = Obb2 { center, yaw, half };
    world
        .surface_at(center.x, center.y)
        .is_none_or(|s| !s.traversable)
        || super::push::meet(world, &hull, &here, unit.mobility.push)
            .solid
            .is_some()
}

/// Whether the hull, rolling `length` from where it stands in `gear` and
/// turning `turn` radians per metre, stays clear, sampled every metre.
fn arc_clear(world: &WorldGeometry, unit: &Unit, gear: f64, turn: f64, length: f64) -> bool {
    let mut at = unit.position.xy();
    let mut yaw = unit.yaw;
    let pieces = (length / PROBE_M).ceil().max(1.0) as usize;
    let piece = length / pieces as f64;
    for _ in 0..pieces {
        let dyaw = turn * piece;
        at = at + dir(travel(yaw, gear) + dyaw / 2.0) * piece;
        yaw += dyaw;
        if blocked(world, unit, at, yaw) {
            return false;
        }
    }
    true
}

/// This tick's motion toward `target`, at surface speed `speed`.
pub fn steer(world: &WorldGeometry, unit: &mut Unit, target: V2, speed: f64, dt: f64) -> Motion {
    let drive = unit.mobility.drive.expect("a vehicle has a drive");
    let gear = gear_sign(unit.direction());
    let here = unit.position.xy();
    let to = target - here;
    let distance = to.length();
    let bearing = to.y.atan2(to.x);
    let error = wrap_angle(bearing - travel(unit.yaw, gear));
    if drive.tracked {
        unit.manoeuvre = None;
        let max = drive.turn_rad_s * dt;
        let yaw = unit.yaw + error.clamp(-max, max);
        let remaining = wrap_angle(bearing - travel(yaw, gear)).abs();
        let factor = if remaining > TURN_IN_PLACE_DEG.to_radians() {
            0.0
        } else {
            remaining.cos()
        };
        let step = (speed_in(&drive, gear, speed) * factor * dt).min(distance);
        return Motion {
            yaw,
            heading: if distance > 0.0 {
                to * (1.0 / distance)
            } else {
                v2(0.0, 0.0)
            },
            step,
            backwards: gear < 0.0 && step > 0.0,
            turning: false,
        };
    }

    // Wheels: curvature at most 1 / radius, and slower than that at speed
    // when the turn rate binds.
    let radius = drive.radius_m;
    let side = if error >= 0.0 { 1.0 } else { -1.0 };
    let curvature = |v: f64| (1.0 / radius).min(drive.turn_rad_s / v.max(1e-9));
    let heading = travel(unit.yaw, gear);
    // A waypoint inside the turning circle needs a leg the other way first.
    if unit.manoeuvre.is_none()
        && distance > ABEAM_M
        && inside_circle(here, heading, side, radius, target, 0.0)
    {
        unit.manoeuvre = Some(Manoeuvre {
            turn: side,
            driven_m: 0.0,
        });
    }
    if let Some(m) = unit.manoeuvre {
        // The leg ends once the forward arc to the waypoint is clear, or
        // after a quarter circle.
        let sweep = error.abs().min(std::f64::consts::PI) * radius;
        let done = m.driven_m >= radius * std::f64::consts::FRAC_PI_2
            || (m.driven_m >= MIN_LEG_M
                && !inside_circle(here, heading, side, radius, target, -CIRCLE_MARGIN_M)
                && arc_clear(world, unit, gear, side / radius, sweep));
        if done {
            unit.manoeuvre = None;
        }
    }
    if let Some(m) = unit.manoeuvre {
        let back = -gear;
        let v = speed_in(&drive, back, speed);
        let step = v * dt;
        let turn = m.turn * curvature(v);
        if arc_clear(world, unit, back, turn, PROBE_M) {
            let dyaw = turn * step;
            return Motion {
                yaw: unit.yaw + dyaw,
                heading: dir(travel(unit.yaw, back) + dyaw / 2.0),
                step,
                backwards: back < 0.0,
                turning: false,
            };
        }
        // The leg meets a solid: turn the other way again.
        unit.manoeuvre = None;
    }
    let slow = TURN_SLOW + (1.0 - TURN_SLOW) * error.abs().min(std::f64::consts::FRAC_PI_2).cos();
    let v = speed_in(&drive, gear, speed) * slow;
    let step = (v * dt).min(distance);
    let max = curvature(v) * step;
    let dyaw = error.clamp(-max, max);
    let turning = error.abs() > TURNING_DEG.to_radians();
    if turning && !arc_clear(world, unit, gear, dyaw.signum() * curvature(v), PROBE_M) {
        // The turn runs into a solid: back up, still turning the same way.
        unit.manoeuvre = Some(Manoeuvre {
            turn: side,
            driven_m: 0.0,
        });
        return Motion {
            yaw: unit.yaw,
            heading: dir(heading),
            step: 0.0,
            backwards: false,
            turning: false,
        };
    }
    Motion {
        yaw: unit.yaw + dyaw,
        heading: dir(heading + dyaw / 2.0),
        step,
        backwards: gear < 0.0 && step > 0.0,
        turning,
    }
}

/// Drop the waypoints a wheeled vehicle has done with before it steers:
/// one reached, one it passes abeam, and a corner it turns into early (a
/// fillet of its radius, at most half a radius before the corner). Returns
/// whether the route's end is reached.
pub fn prune(unit: &mut Unit) -> bool {
    let Some(drive) = unit.mobility.drive.filter(|d| !d.tracked) else {
        return false;
    };
    let here = unit.position.xy();
    let heading = travel(unit.yaw, gear_sign(unit.direction()));
    let route = unit.route.as_mut().expect("a route to follow");
    while let Some(&wp) = route.first() {
        let to = wp - here;
        let distance = to.length();
        let error = wrap_angle(to.y.atan2(to.x) - heading).abs();
        let reached = distance < super::PROGRESS_EPSILON_M
            || (distance < ABEAM_M && error > ABEAM_DEG.to_radians());
        let early = route.get(1).is_some_and(|&next| {
            let out = next - wp;
            let corner = wrap_angle(out.y.atan2(out.x) - to.y.atan2(to.x)).abs();
            distance <= (drive.radius_m * (corner / 2.0).tan()).min(drive.radius_m / 2.0)
        });
        if !(reached || early) {
            break;
        }
        route.remove(0);
        unit.progress = (f64::INFINITY, unit.progress.1);
    }
    route.is_empty()
}
