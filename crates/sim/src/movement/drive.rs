//! A vehicle's own kinematics (Q29–Q31): how it turns toward its next
//! waypoint this tick. The route is the planner's, its corners rounded to
//! the turning radius (`navigation::corners`); the follower owns the rest
//! of the kinematics (no kinodynamic planner), and slows ahead of a bend to
//! what its yaw rate can take round it ([`approach_speed`]).
//!
//! Tracks turn at their own rate and pivot on the spot beyond
//! `movement.drive.turn_in_place_deg` of heading error. A body they can't
//! shove in the way of the turn (a trunk at the flank, a wall at the nose)
//! sends them straight along their length to where the next turn clears
//! ([`room_to_turn`]). Wheels never pivot: the yaw
//! turns only as the hull rolls, at most `1 / radius` per metre, so a
//! U-turn is an arc. A waypoint inside the turning circle, or already under
//! the hull, is dropped when the straight way past it is clear ([`prune`]).
//! A blocked turn rolls straight on instead, while straight ahead is clear,
//! when the way lies ahead or the block is at the trailing end (a tail in
//! an alley's mouth, which backing would drive further in). Otherwise the
//! route's end or a corner inside the turning circle, a turn whose next
//! metre runs into a solid, or a step held anyway (a jammed shove) has it
//! work out a shunt: the forward and reverse legs that put it back on its
//! route ([`super::shunt`]). Every arc is swept for bodies finer than a
//! corner can swing through one, and the step itself must land clear.
//!
//! A reverse move (Q31) drives the same way with the travel direction
//! behind the hull, so the facing is held, at the reverse fraction of the
//! speed (Q30).
use contract::command::MoveDirection;
use contract::ids::UnitId;
use contract::scenario::{PushClass, Rules};

use crate::math::{v2, wrap_angle, Obb2, V2};
use crate::navigation::Drive;
use crate::units::Unit;
use crate::world::WorldGeometry;

/// A turn looks this far along its arc for a solid.
const PROBE_M: f64 = 1.0;
/// Bodies are looked for along the arc at least this often: a turning
/// hull's corner swings through a body and out again within a metre, so
/// sampling only the end of each metre would pass a step it cannot take.
const SAMPLE_M: f64 = 0.1;
/// Room for a blocked tracked turn is looked for this often along the hull.
const ROOM_STEP_M: f64 = 0.25;

/// One tick's motion: the yaw the hull ends at, the unit direction its
/// centre moves along, and how far.
#[derive(Clone, Copy, Debug)]
pub struct Motion {
    pub yaw: f64,
    pub heading: V2,
    pub step: f64,
    /// Driving backwards (a reverse move, or a leg of a shunt).
    pub backwards: bool,
    /// Working toward its way while moving off the line to its waypoint
    /// (backing to make room for a tracked pivot, or a shunt) counts as
    /// progress.
    pub making_space: bool,
    /// Traffic that rejected the look-ahead arc, even when the step is zero.
    pub blocker: Option<UnitId>,
}

enum ArcBlock {
    /// A body it cannot shove, or ground it cannot cross; `trailing` when
    /// the body meets the hull behind its middle, as a turn swings the
    /// trailing end out.
    Solid {
        trailing: bool,
    },
    Vehicle(UnitId),
}

fn gear_sign(direction: MoveDirection) -> f64 {
    match direction {
        MoveDirection::Forward => 1.0,
        MoveDirection::Reverse => -1.0,
    }
}

/// `unit`'s gear for its current order: +1 forward, -1 reverse.
pub(super) fn gear(unit: &Unit) -> f64 {
    gear_sign(unit.direction())
}

/// The unit direction `unit` travels along for its current order.
pub(super) fn travel_heading(unit: &Unit) -> V2 {
    dir(travel(unit.yaw, gear(unit)))
}

/// The travel heading of a hull at `yaw` in `gear` (+1 forward, -1 back).
pub(super) fn travel(yaw: f64, gear: f64) -> f64 {
    if gear < 0.0 {
        yaw + std::f64::consts::PI
    } else {
        yaw
    }
}

pub(super) fn dir(angle: f64) -> V2 {
    v2(libm::cos(angle), libm::sin(angle))
}

pub(super) fn speed_in(drive: &Drive, gear: f64, speed: f64) -> f64 {
    if gear < 0.0 {
        speed * drive.reverse_speed_factor
    } else {
        speed
    }
}

/// Approach a speed limit without banking speed during a stop or gear change.
pub(super) fn accelerate(unit: &Unit, desired: f64, gear: f64, dt: f64) -> f64 {
    let drive = unit.ground().drive.expect("a vehicle has a drive");
    let current = (unit.drive_speed_mps * gear).max(0.0);
    let seconds = if desired > current {
        drive.feel.acceleration_s
    } else {
        drive.feel.braking_s
    };
    let change = unit.ground().road_mps / seconds * dt;
    current + (desired - current).clamp(-change, change)
}

fn turn_speed(drive: &Drive, full: f64, angle: f64) -> f64 {
    let angle = angle.abs();
    if drive.tracked {
        if angle > drive.feel.turn_in_place_deg.to_radians() {
            0.0
        } else {
            full * libm::cos(angle)
        }
    } else {
        let corner = (full * drive.feel.turn_speed_factor).min(drive.radius_m * drive.turn_rad_s);
        let lock = (angle / drive.feel.turning_deg.to_radians()).min(1.0);
        corner + (full - corner) * libm::cos(lock * std::f64::consts::FRAC_PI_2)
    }
}

/// Look through short approach legs too: a road bend can have several corners.
/// Beyond the full-road stopping distance, even a stop cannot bind this tick.
fn approach_speed(unit: &Unit, surface_speed: f64, gear: f64) -> f64 {
    let drive = unit.ground().drive.expect("a vehicle has a drive");
    let braking = unit.ground().road_mps / drive.feel.braking_s;
    let stopping_distance = unit.ground().road_mps.powi(2) / (2.0 * braking);
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
            let angle = wrap_angle(
                libm::atan2(outgoing.y, outgoing.x) - libm::atan2(incoming.y, incoming.x),
            );
            // A bend drawn as short runs turns at the radius they describe,
            // and the hull's yaw rate takes it no faster than that allows.
            let bend = incoming.length().min(outgoing.length())
                / (2.0 * libm::sin(angle.abs() / 2.0)).max(1e-9);
            turn_speed(&drive, full, angle).min(drive.turn_rad_s * bend)
        });
        limit = limit.min((corner * corner + 2.0 * braking * distance).sqrt());
        from = point;
    }
    limit
}

/// Whether `target` lies inside the turning circle on side `side` of a hull
/// at `at` travelling along `heading`.
fn inside_circle(at: V2, heading: f64, side: f64, radius: f64, target: V2) -> bool {
    let centre = at + dir(heading + side * std::f64::consts::FRAC_PI_2) * radius;
    (target - centre).length() < radius
}

/// Whether a hull at `center`/`yaw` would stand in a body that stops it, or
/// off traversable ground (true geometry: what the step itself meets).
fn blocked(world: &WorldGeometry, unit: &Unit, center: V2, yaw: f64) -> bool {
    solid_at(world, unit, center, yaw, 1.0).is_some()
}

/// What stops a hull at `center`/`yaw` travelling in `gear`, if anything:
/// `Some(true)` when the body meets it behind its middle (its trailing end).
/// Ground it cannot cross counts as ahead.
fn solid_at(world: &WorldGeometry, unit: &Unit, center: V2, yaw: f64, gear: f64) -> Option<bool> {
    let half = unit.hull.expect("a vehicle has a hull").xy();
    let here = unit.ground_footprint().expect("a vehicle has a hull");
    let hull = Obb2 { center, yaw, half };
    if world
        .surface_at(center.x, center.y)
        .is_none_or(|s| !s.traversable)
    {
        return Some(false);
    }
    let solid = super::push::meet(world, &hull, &here, unit.ground().push).solid?;
    let body = world.prop(solid).expect("a met body").footprint();
    Some(hull.to_local(body.closest(center)).x * gear < 0.0)
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
) -> Result<(), ArcBlock> {
    let mut at = unit.position.xy();
    let mut yaw = unit.yaw;
    let pieces = (length / PROBE_M).ceil().max(1.0) as usize;
    let piece = length / pieces as f64;
    for _ in 0..pieces {
        let dyaw = turn * piece;
        at = at + dir(travel(yaw, gear) + dyaw / 2.0) * piece;
        yaw += dyaw;
        let before = unit.ground_footprint().expect("a vehicle has a hull");
        let after = Obb2 {
            center: at,
            yaw,
            half: before.half,
        };
        let into_traffic = traffic.iter().enumerate().find_map(|(id, other)| {
            (id != unit.id.0 as usize
                && other.is_some_and(|other| super::hull_conflict(before, after, other)))
            .then_some(UnitId(id as u32))
        });
        if let Some(trailing) = solid_at(world, unit, at, yaw, gear) {
            return Err(ArcBlock::Solid { trailing });
        }
        if let Some(id) = into_traffic {
            return Err(ArcBlock::Vehicle(id));
        }
    }
    // Between those poses the hull's corners sweep too: no body it cannot
    // shove may lie in the way of any part of the arc.
    let (mut at, mut yaw) = (unit.position.xy(), unit.yaw);
    let fine = (length / SAMPLE_M).ceil().max(1.0) as usize;
    let piece = length / fine as f64;
    for _ in 0..fine {
        let dyaw = turn * piece;
        at = at + dir(travel(yaw, gear) + dyaw / 2.0) * piece;
        yaw += dyaw;
        if let Some(trailing) = solid_at(world, unit, at, yaw, gear) {
            return Err(ArcBlock::Solid { trailing });
        }
    }
    Ok(())
}

/// Give a blocked tracked turn room: roll straight along the hull, at
/// most `room` metres (signed, ahead positive).
pub fn give_space(unit: &Unit, room: f64, speed: f64, dt: f64) -> Motion {
    let drive = unit.ground().drive.expect("a vehicle has a drive");
    let gear = room.signum();
    let v = accelerate(unit, speed_in(&drive, gear, speed), gear, dt);
    Motion {
        yaw: unit.yaw,
        heading: dir(travel(unit.yaw, gear)),
        step: (v * dt).min(room.abs()),
        backwards: gear < 0.0,
        making_space: true,
        blocker: None,
    }
}

/// Which way a tracked hull gives a blocked turn room when nothing better
/// is known: backing off against its ordered direction.
pub fn away(unit: &Unit) -> f64 {
    -gear_sign(unit.direction())
}

/// Where a tracked hull that a body stops from turning to `yaw`
/// can make the whole turn: the nearest spot straight along its length,
/// within one hull length either way, that it rolls to clear and turns at
/// clear of every body (a trunk beside its hull: drive on until it can come
/// round; a wall at its nose: back off). Signed metres along its heading;
/// at equal distance it goes `first` (+1 ahead, -1 back). `None`: nowhere
/// within reach frees the turn.
pub fn room_to_turn(world: &WorldGeometry, unit: &Unit, first: f64, yaw: f64) -> Option<f64> {
    let half = unit.hull.expect("a vehicle has a hull").xy();
    let (here, ahead) = (unit.position.xy(), dir(unit.yaw));
    let mut open = [true; 2];
    for k in 1..=(2.0 * half.x / ROOM_STEP_M).ceil() as usize {
        for (way, sign) in [first, -first].into_iter().enumerate() {
            let d = sign * k as f64 * ROOM_STEP_M;
            let spot = here + ahead * d;
            // Rolling straight, the hull trails its leading face by its
            // whole length, so no body slips between two of these steps.
            open[way] &= !blocked(world, unit, spot, unit.yaw);
            if open[way] && turn_clear(world, unit, spot, yaw) {
                return Some(d);
            }
        }
    }
    None
}

/// Whether a hull at `at` pivots the short way from its yaw to `yaw` with
/// no part of it meeting a body.
fn turn_clear(world: &WorldGeometry, unit: &Unit, at: V2, yaw: f64) -> bool {
    let reach = unit.hull.expect("a vehicle has a hull").xy().length();
    let error = wrap_angle(yaw - unit.yaw);
    let pieces = (error.abs() * reach / SAMPLE_M).ceil().max(1.0) as usize;
    (1..=pieces).all(|n| !blocked(world, unit, at, unit.yaw + error * n as f64 / pieces as f64))
}

/// This tick's motion toward `target`, at surface speed `speed`.
pub fn steer(
    world: &WorldGeometry,
    unit: &mut Unit,
    target: V2,
    speed: f64,
    dt: f64,
    tick: u64,
    traffic: &[Option<Obb2>],
) -> Motion {
    let drive = unit.ground().drive.expect("a vehicle has a drive");
    let gear = gear_sign(unit.direction());
    let here = unit.position.xy();
    let to = target - here;
    let distance = to.length();
    let bearing = libm::atan2(to.y, to.x);
    let error = wrap_angle(bearing - travel(unit.yaw, gear));
    if drive.tracked {
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
            making_space: false,
            blocker: None,
        };
    }

    // Wheels: curvature at most 1 / radius, and slower than that at speed
    // when the turn rate binds.
    let radius = drive.radius_m;
    let side = if error >= 0.0 { 1.0 } else { -1.0 };
    let curvature = |v: f64| (1.0 / radius).min(drive.turn_rad_s / v.max(1e-9));
    let heading = travel(unit.yaw, gear);
    if let Some(motion) = super::shunt::drive_on(world, unit, speed, dt, tick) {
        return motion;
    }
    // A waypoint inside the turning circle needs room worked out first.
    if distance > drive.feel.abeam_m && inside_circle(here, heading, side, radius, target) {
        return shunt(world, unit, traffic, &[], tick);
    }
    let desired = turn_speed(&drive, speed_in(&drive, gear, speed), error)
        .min(approach_speed(unit, speed, gear));
    let v = accelerate(unit, desired, gear, dt);
    let step = (v * dt).min(distance);
    let max = curvature(v) * step;
    let dyaw = error.clamp(-max, max);
    let reach = PROBE_M.min((distance - super::PROGRESS_EPSILON_M).max(0.0));
    let turn = error.clamp(-curvature(v) * PROBE_M, curvature(v) * PROBE_M) / PROBE_M;
    let turning = Motion {
        yaw: unit.yaw + dyaw,
        heading: dir(heading + dyaw / 2.0),
        step,
        backwards: gear < 0.0 && step > 0.0,
        making_space: false,
        blocker: None,
    };
    let blocked = arc_clear(world, unit, gear, turn, reach, traffic)
        .err()
        .or_else(|| {
            lands(world, unit, &turning, gear).map(|trailing| ArcBlock::Solid { trailing })
        });
    if let Some(block) = blocked {
        // A turn blocked while straight ahead is clear rolls straight on
        // until it clears, when the way lies ahead (a backing leg would
        // give up ground it needs) or the block is at the trailing end (a
        // tail in an alley's mouth, which backing drives further in).
        let straight = Motion {
            yaw: unit.yaw,
            heading: dir(heading),
            ..turning
        };
        let ahead = error.abs() < std::f64::consts::FRAC_PI_2;
        if matches!(block, ArcBlock::Solid { trailing } if trailing || ahead)
            && arc_clear(world, unit, gear, 0.0, reach, traffic).is_ok()
            && lands(world, unit, &straight, gear).is_none()
        {
            return straight;
        }
        // A vehicle in the way is waited for, as traffic (how long is the
        // vehicle tick's to say); a solid has it work out a shunt.
        return match block {
            ArcBlock::Vehicle(id) => Motion {
                yaw: unit.yaw,
                heading: dir(heading),
                step: 0.0,
                backwards: false,
                making_space: false,
                blocker: Some(id),
            },
            ArcBlock::Solid { .. } => shunt(world, unit, traffic, &[], tick),
        };
    }
    turning
}

/// Work out `unit`'s shunt onto its route, with the bodies in `held` as
/// solid, and stand this tick.
pub(super) fn shunt(
    world: &WorldGeometry,
    unit: &mut Unit,
    traffic: &[Option<Obb2>],
    held: &[Obb2],
    tick: u64,
) -> Motion {
    unit.shunt = Some(super::shunt::search(world, unit, traffic, held, tick));
    Motion {
        yaw: unit.yaw,
        heading: travel_heading(unit),
        step: 0.0,
        backwards: false,
        making_space: false,
        blocker: None,
    }
}

/// What stops `motion`'s own step, travelling in `gear`, if anything (as
/// [`solid_at`]). The arc's samples, a tenth of a metre apart, can pass a
/// corner that grazes a body within the first few millimetres; the step
/// itself must still land clear, or the follower picks it again every tick.
pub(super) fn lands(
    world: &WorldGeometry,
    unit: &Unit,
    motion: &Motion,
    gear: f64,
) -> Option<bool> {
    let at = unit.position.xy() + motion.heading * motion.step;
    solid_at(world, unit, at, motion.yaw, gear)
}

/// The ordered arrival heading, refined by the route's final approach.
/// Preview and publication share this estimate; ordinary combat may face elsewhere.
pub fn final_facing(unit: &Unit) -> f64 {
    let Some((goal, _)) = unit.movement_goal() else {
        return unit.turn_to.unwrap_or(unit.yaw);
    };
    let facing = unit
        .orders
        .front()
        .and_then(|o| o.movement())
        .and_then(|m| m.facing);
    let route = unit.route.as_deref().unwrap_or(&[]);
    let end = route.last().copied().unwrap_or(goal);
    let from = match route.len() {
        0 | 1 => unit.position.xy(),
        n => route[n - 2],
    };
    final_yaw(unit, facing, from, end, unit.direction()).unwrap_or(unit.yaw)
}

/// The yaw a hull ends at when `order` completes (D2): a right-drag's
/// facing (Q9) where it can take it, else the way it travels along the
/// route's last leg (held facing on a reverse, Q31). Wheels never pivot:
/// driving forward they line up on the facing as far as they can
/// ([`line_up_heading`]); once routed, the lead-in is that last leg.
pub fn final_yaw(
    unit: &Unit,
    facing: Option<f64>,
    from: V2,
    end: V2,
    direction: MoveDirection,
) -> Option<f64> {
    // Tracks and rotors turn on the spot, so they end facing as ordered.
    let pivots = unit.airborne() || unit.ground().drive.is_some_and(|d| d.tracked);
    if let Some(f) = facing.filter(|_| pivots || !unit.is_vehicle()) {
        return Some(f);
    }
    if let Some(f) = facing.filter(|_| direction == MoveDirection::Forward) {
        return line_up_heading(from, end, f);
    }
    let leg = end - from;
    (leg.length() > 1e-6).then(|| travel(libm::atan2(leg.y, leg.x), gear_sign(direction)))
}

/// The heading a wheeled hull arriving from `from` can drive in to `end`
/// on: the ordered `facing`, turned toward its approach by at most a right
/// angle, so the last turn is an ordinary forward one, never a shunt. A
/// facing straight back the way it came ends square to it.
pub fn line_up_heading(from: V2, end: V2, facing: f64) -> Option<f64> {
    let leg = end - from;
    let approach = libm::atan2(leg.y, leg.x);
    let off = wrap_angle(facing - approach)
        .clamp(-std::f64::consts::FRAC_PI_2, std::f64::consts::FRAC_PI_2);
    (leg.length() > 1e-6).then_some(approach + off)
}

/// Where a wheeled hull's straight lead-in to `end` on `heading` begins:
/// far enough back that a hull of half-length `half`, turning onto it at
/// `drive`'s radius, never has `end` inside its turning circle and has room
/// to straighten.
pub fn lead_in(end: V2, heading: f64, drive: &Drive, half: f64) -> V2 {
    end - dir(heading) * (4.0 * (drive.radius_m + half))
}

/// A tracked vehicle at rest pivots toward its facing (Q9) at its turn
/// rate. A body in the way of an ordered facing sends it rolling along its
/// length to room for the turn ([`room_to_turn`]), shoving nothing and
/// waiting for no one (traffic gives the turn up); a turn it makes on its
/// own (`ordered` false: a hull gun onto its target, D8) is given up
/// instead, so it never leaves its spot unordered.
pub fn pivot(
    ctx: &super::MovementContext,
    unit: &mut Unit,
    dt: f64,
    ordered: bool,
    traffic: &[Option<Obb2>],
) {
    let (Some(facing), Some(drive)) = (unit.turn_to, unit.ground().drive) else {
        return;
    };
    let error = wrap_angle(facing - unit.yaw);
    let max = drive.turn_rad_s * dt;
    let yaw = unit.yaw + error.clamp(-max, max);
    let here = unit.ground_footprint().expect("a vehicle has a hull");
    // Another hull in the way gives the turn up: it checks true hulls, as
    // a stopped approach may leave less than traffic's buffer beside it.
    let turned = Obb2 { yaw, ..here };
    if traffic.iter().enumerate().any(|(id, other)| {
        id != unit.id.0 as usize && other.is_some_and(|other| turned.overlaps(&other))
    }) {
        unit.drive_speed_mps = 0.0;
        unit.turn_to = None;
        return;
    }
    if !blocked(ctx.world, unit, here.center, yaw) {
        unit.yaw = yaw;
        unit.drive_speed_mps = 0.0;
        if error.abs() <= max {
            unit.turn_to = None;
        }
        return;
    }
    let footing = ctx
        .ground
        .footing(ctx.world, here.center, &ctx.rules.ground);
    let speed = footing.map_or(0.0, |g| unit.ground().speed(g));
    let rolled = ordered
        .then(|| room_to_turn(ctx.world, unit, away(unit), facing))
        .flatten()
        .and_then(|room| {
            let roll = give_space(unit, room, speed, dt);
            let next = Obb2 {
                center: here.center + roll.heading * roll.step,
                ..here
            };
            let met = super::push::meet(ctx.world, &next, &here, unit.ground().push);
            let jam = traffic.iter().enumerate().any(|(id, other)| {
                id != unit.id.0 as usize
                    && other.is_some_and(|other| super::hull_conflict(here, next, other))
            });
            let ground = ctx.world.surface_at(next.center.x, next.center.y);
            ground
                .filter(|s| s.traversable && met.solid.is_none() && met.shoved.is_empty() && !jam)
                .map(|ground| (roll, next.center.with_z(ground.z)))
        });
    let Some((roll, at)) = rolled else {
        unit.drive_speed_mps = 0.0;
        unit.turn_to = None;
        return;
    };
    unit.position = at;
    unit.reversing = roll.backwards;
    unit.drive_speed_mps = roll.step / dt * roll.heading.dot(dir(unit.yaw));
}

/// Drop the waypoints a wheeled vehicle has done with before it steers:
/// one reached, one it passes abeam, or one inside its turning circle or
/// under its own hull whose clearance it does not need: the straight way on
/// to the next waypoint fits its side's grid with the other vehicles in
/// `traffic`, and the bodies its hull is pressed against, as solid, since a
/// detour's corners clear vehicles and jammed bodies the grid does not hold.
/// A route's corners supply clearance; the steering owner follows them at
/// the hull's turning radius. Returns whether the route's end is reached.
pub(super) fn prune(
    ctx: &super::MovementContext,
    sides: &mut [super::SideGeometry; 3],
    traffic: &[Option<Obb2>],
    unit: &mut Unit,
) -> bool {
    let mobility = *unit.ground();
    let Some(drive) = mobility.drive.filter(|d| !d.tracked) else {
        return false;
    };
    let known = &mut sides[unit.side.index()];
    let own = unit.id.0 as usize;
    let pressed = super::push::pressing(ctx.world, unit);
    let others = || -> Vec<Obb2> {
        let hulls = traffic.iter().enumerate();
        hulls
            .filter_map(|(id, hull)| hull.filter(|_| id != own))
            .chain(pressed.iter().copied())
            .collect()
    };
    let here = unit.position.xy();
    let hull = unit.ground_footprint().expect("a vehicle has a hull");
    let heading = travel(unit.yaw, gear_sign(unit.direction()));
    let route = unit.route.as_mut().expect("a route to follow");
    while let Some(&wp) = route.first() {
        let to = wp - here;
        let distance = to.length();
        let signed = wrap_angle(libm::atan2(to.y, to.x) - heading);
        let error = signed.abs();
        let reached = distance < super::PROGRESS_EPSILON_M
            || (distance < drive.feel.abeam_m && error > drive.feel.abeam_deg.to_radians());
        // Never the route's end: that is where it must stand.
        let side = if signed >= 0.0 { 1.0 } else { -1.0 };
        let passed = route.len() > 1
            && (inside_circle(here, heading, side, drive.radius_m, wp) || hull.contains(wp, 0.0))
            && known
                .grid(ctx.world, ctx.authored)
                .way_fits(here, route[1], &mobility, &others());
        if !reached && !passed {
            break;
        }
        route.remove(0);
        unit.progress = (f64::INFINITY, unit.progress.1);
    }
    route.is_empty()
}

/// Where a vehicle killed on the move comes to rest: it rolls on along its
/// travel, its tracks or wheels locked, losing its full road speed in
/// `wreck_stop_s`, and stops short, in steps of `STEP_M`, of whatever its
/// hull would meet: any body that stops vehicles (even one it could shove
/// while driving: a dead hull shoves nothing), a bank, one of `hulls` (the
/// other vehicles) or one of `soldiers`. Its wreck lies there.
pub fn death_roll(
    world: &WorldGeometry,
    rules: &Rules,
    unit: &Unit,
    hulls: &[Obb2],
    soldiers: &[V2],
) -> V2 {
    const STEP_M: f64 = 0.25;
    let at = unit.position.xy();
    let Some(here) = unit.ground_footprint() else {
        return at;
    };
    let speed = unit.drive_speed_mps.max(0.0);
    let stopping = unit.ground().road_mps / rules.movement.drive.wreck_stop_s;
    let length = speed * speed / (2.0 * stopping);
    let gear = if unit.reversing { -1.0 } else { 1.0 };
    let toward = dir(travel(unit.yaw, gear));
    let soldier_r = rules.physics.soldier_radius_m;
    let mut rest = at;
    let mut rolled = 0.0;
    while rolled < length {
        rolled = (rolled + STEP_M).min(length);
        let next = at + toward * rolled;
        let hull = Obb2 {
            center: next,
            ..here
        };
        let stopped = world
            .surface_at(next.x, next.y)
            .is_none_or(|s| !s.traversable)
            || super::push::meet(world, &hull, &here, PushClass::None)
                .solid
                .is_some()
            || hulls.iter().any(|h| hull.overlaps(h))
            // One already against the hull where it died stays where he is.
            || soldiers
                .iter()
                .any(|&p| hull.contains(p, soldier_r) && !here.contains(p, soldier_r));
        if stopped {
            break;
        }
        rest = next;
    }
    rest
}
