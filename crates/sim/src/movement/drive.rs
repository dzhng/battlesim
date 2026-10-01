//! A vehicle's own kinematics (Q29–Q31): how it turns toward its next
//! waypoint this tick. The route is the planner's; the follower owns the
//! kinematics (no kinodynamic planner).
//!
//! Tracks turn at their own rate and pivot on the spot beyond
//! `movement.drive.turn_in_place_deg` of heading error. Wheels never pivot: the yaw
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

/// A turn looks this far along its arc for a solid.
const PROBE_M: f64 = 1.0;

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
    /// A wheeled turn or clearing a tracked pivot counts as progress away from its waypoint.
    pub manoeuvring: bool,
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

/// Approach a speed limit without banking speed during a stop or gear change.
fn accelerate(unit: &Unit, desired: f64, gear: f64, dt: f64) -> f64 {
    let drive = unit.mobility.drive.expect("a vehicle has a drive");
    let current = (unit.drive_speed_mps * gear).max(0.0);
    let seconds = if desired > current {
        drive.feel.acceleration_s
    } else {
        drive.feel.braking_s
    };
    let change = unit.mobility.road_mps / seconds * dt;
    current + (desired - current).clamp(-change, change)
}

fn turn_speed(drive: &Drive, full: f64, angle: f64) -> f64 {
    let angle = angle.abs();
    if drive.tracked {
        if angle > drive.feel.turn_in_place_deg.to_radians() {
            0.0
        } else {
            full * angle.cos()
        }
    } else {
        let corner = (full * drive.feel.turn_slow).min(drive.radius_m * drive.turn_rad_s);
        let lock = (angle / drive.feel.turning_deg.to_radians()).min(1.0);
        corner + (full - corner) * (lock * std::f64::consts::FRAC_PI_2).cos()
    }
}

/// Look through short approach legs too: a road bend can have several corners.
/// Beyond the full-road stopping distance, even a stop cannot bind this tick.
fn approach_speed(unit: &Unit, surface_speed: f64, gear: f64) -> f64 {
    let drive = unit.mobility.drive.expect("a vehicle has a drive");
    let braking = unit.mobility.road_mps / drive.feel.braking_s;
    let stopping_distance = unit.mobility.road_mps.powi(2) / (2.0 * braking);
    let full = speed_in(&drive, gear, surface_speed);
    let mut limit = full;
    let mut from = unit.position.xy();
    let mut distance = 0.0;
    let route = unit.route.as_ref().expect("a route to follow");
    for (i, &point) in route.iter().enumerate() {
        let incoming = point - from;
        distance += incoming.length();
        if distance > stopping_distance {
            break;
        }
        let corner = route.get(i + 1).map_or(0.0, |next| {
            let outgoing = *next - point;
            let angle = wrap_angle(outgoing.y.atan2(outgoing.x) - incoming.y.atan2(incoming.x));
            turn_speed(&drive, full, angle)
        });
        limit = limit.min((corner * corner + 2.0 * braking * distance).sqrt());
        from = point;
    }
    limit
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
fn arc_clear(
    world: &WorldGeometry,
    unit: &Unit,
    gear: f64,
    turn: f64,
    length: f64,
    traffic: &[Option<Obb2>],
) -> bool {
    let mut at = unit.position.xy();
    let mut yaw = unit.yaw;
    let pieces = (length / PROBE_M).ceil().max(1.0) as usize;
    let piece = length / pieces as f64;
    for _ in 0..pieces {
        let dyaw = turn * piece;
        at = at + dir(travel(yaw, gear) + dyaw / 2.0) * piece;
        yaw += dyaw;
        let before = unit.hull_box().expect("a vehicle has a hull");
        let after = Obb2 {
            center: at,
            yaw,
            half: before.half,
        };
        let into_traffic = traffic.iter().enumerate().any(|(id, other)| {
            id != unit.id.0 as usize
                && other.is_some_and(|other| {
                    let depth = |h: &Obb2| h.separation(&other).map_or(0.0, |v| v.length());
                    depth(&after) > depth(&before) + 1e-9
                })
        });
        if blocked(world, unit, at, yaw) || into_traffic {
            return false;
        }
    }
    true
}

/// Give a blocked tracked pivot room by rolling against its ordered direction.
pub fn give_space(unit: &Unit, speed: f64, dt: f64) -> Motion {
    let drive = unit.mobility.drive.expect("a vehicle has a drive");
    let gear = -gear_sign(unit.direction());
    let v = accelerate(unit, speed_in(&drive, gear, speed), gear, dt);
    Motion {
        yaw: unit.yaw,
        heading: dir(travel(unit.yaw, gear)),
        step: v * dt,
        backwards: gear < 0.0,
        manoeuvring: true,
    }
}

/// This tick's motion toward `target`, at surface speed `speed`.
pub fn steer(
    world: &WorldGeometry,
    unit: &mut Unit,
    target: V2,
    speed: f64,
    dt: f64,
    traffic: &[Option<Obb2>],
) -> Motion {
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
        let pivoting = remaining > drive.feel.turn_in_place_deg.to_radians();
        let desired = turn_speed(&drive, speed_in(&drive, gear, speed), remaining)
            .min(approach_speed(unit, speed, gear));
        let v = accelerate(unit, desired, gear, dt);
        let step = if pivoting {
            0.0
        } else {
            (v * dt).min(distance)
        };
        return Motion {
            yaw,
            heading: if distance > 0.0 {
                to * (1.0 / distance)
            } else {
                v2(0.0, 0.0)
            },
            step,
            backwards: gear < 0.0 && step > 0.0,
            manoeuvring: false,
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
        && distance > drive.feel.abeam_m
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
            || (m.driven_m >= drive.feel.min_leg_m
                && !inside_circle(
                    here,
                    heading,
                    side,
                    radius,
                    target,
                    -drive.feel.circle_margin_m,
                )
                && arc_clear(world, unit, gear, side / radius, sweep, traffic));
        if done {
            unit.manoeuvre = None;
        }
    }
    if let Some(m) = unit.manoeuvre {
        let back = -gear;
        let v = accelerate(unit, speed_in(&drive, back, speed), back, dt);
        let step = v * dt;
        let turn = m.turn * curvature(v);
        if arc_clear(world, unit, back, turn, PROBE_M, traffic) {
            let dyaw = turn * step;
            return Motion {
                yaw: unit.yaw + dyaw,
                heading: dir(travel(unit.yaw, back) + dyaw / 2.0),
                step,
                backwards: back < 0.0,
                manoeuvring: false,
            };
        }
        // The leg meets a solid: turn the other way again.
        unit.manoeuvre = None;
    }
    let desired = turn_speed(&drive, speed_in(&drive, gear, speed), error)
        .min(approach_speed(unit, speed, gear));
    let v = accelerate(unit, desired, gear, dt);
    let step = (v * dt).min(distance);
    let max = curvature(v) * step;
    let dyaw = error.clamp(-max, max);
    let turning = error.abs() > drive.feel.turning_deg.to_radians();
    if !arc_clear(
        world,
        unit,
        gear,
        error.clamp(-curvature(v) * PROBE_M, curvature(v) * PROBE_M) / PROBE_M,
        PROBE_M.min((distance - super::PROGRESS_EPSILON_M).max(0.0)),
        traffic,
    ) {
        // The forward arc runs into a solid: back up, still turning the same way.
        unit.manoeuvre = Some(Manoeuvre {
            turn: side,
            driven_m: 0.0,
        });
        return Motion {
            yaw: unit.yaw,
            heading: dir(heading),
            step: 0.0,
            backwards: false,
            manoeuvring: false,
        };
    }
    Motion {
        yaw: unit.yaw + dyaw,
        heading: dir(heading + dyaw / 2.0),
        step,
        backwards: gear < 0.0 && step > 0.0,
        manoeuvring: turning,
    }
}

/// The yaw a hull ends at when `order` completes (D2): a right-drag's
/// facing (Q9) where it can take it, else the way it travels along the
/// route's last leg (held facing on a reverse, Q31). Wheels never pivot,
/// so a wheeled vehicle ends facing its travel.
pub fn final_yaw(
    unit: &Unit,
    facing: Option<f64>,
    from: V2,
    end: V2,
    direction: MoveDirection,
) -> Option<f64> {
    let tracked = unit.mobility.drive.is_some_and(|d| d.tracked);
    if let Some(f) = facing.filter(|_| tracked || !unit.is_vehicle()) {
        return Some(f);
    }
    let leg = end - from;
    (leg.length() > 1e-6).then(|| travel(leg.y.atan2(leg.x), gear_sign(direction)))
}

/// A tracked vehicle at rest pivots toward its ordered facing (Q9), at its
/// turn rate, while the turn is clear; a blocked turn is given up.
pub fn pivot(world: &WorldGeometry, unit: &mut Unit, dt: f64) {
    let (Some(facing), Some(drive)) = (unit.turn_to, unit.mobility.drive) else {
        return;
    };
    let error = wrap_angle(facing - unit.yaw);
    let max = drive.turn_rad_s * dt;
    let yaw = unit.yaw + error.clamp(-max, max);
    if blocked(world, unit, unit.position.xy(), yaw) {
        unit.turn_to = None;
        return;
    }
    unit.yaw = yaw;
    if error.abs() <= max {
        unit.turn_to = None;
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
            || (distance < drive.feel.abeam_m && error > drive.feel.abeam_deg.to_radians());
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
