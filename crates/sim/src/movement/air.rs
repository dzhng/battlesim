//! Flight (D4, D6, D7). An aircraft flies its side's air route at cruise
//! speed, easing in and out with the vehicles' acceleration time and slowing
//! for its route's corners. It holds
//! `cruise_agl_m` above the ground and climbs early enough to pass
//! `clearance_m` over anything taller on its way, never above
//! `ceiling_agl_m`. It turns on the spot toward where it flies, or to its
//! ordered facing at rest. Aircraft never collide; ones closer than
//! `separation_m` drift apart.

use super::{arrive, may_advance, MovementContext, SideGeometry, GOAL_REPLAN_M};
use crate::math::{v2, V2};
use crate::units::{Flight, Motion, Unit};
use contract::observation::MoveState;

/// Within this of a waypoint on the way it counts as passed.
const WAYPOINT_M: f64 = 1.0;
/// Within this of its destination it has arrived.
const ARRIVAL_M: f64 = 0.05;
/// Climb lookahead samples are this far apart along the flight.
const LOOKAHEAD_STEP_M: f64 = 8.0;
/// How fast two overlapping aircraft drift apart at full overlap.
const SEPARATION_MPS: f64 = 4.0;
/// The share of its acceleration an aircraft plans its braking on.
const BRAKING_SHARE: f64 = 0.8;
/// Below this ground speed an aircraft is hovering, and faces as ordered.
const HOVER_MPS: f64 = 1.0;

/// Fly unit `i` one tick.
pub(super) fn step_aircraft(
    ctx: &MovementContext,
    units: &mut [Unit],
    i: usize,
    sides: &mut [SideGeometry; 2],
) {
    let dt = 1.0 / ctx.rules.tick_hz as f64;
    let Motion::Air(flight) = units[i].motion else {
        unreachable!("only aircraft fly");
    };
    let side = units[i].side.index();
    plan(ctx, &mut units[i], &mut sides[side]);
    let drift = separation(ctx, units, i);
    let unit = &mut units[i];
    let here = unit.position.xy();
    let accel = flight.cruise_mps / ctx.rules.movement.drive.acceleration_s;
    let wanted = if may_advance(ctx, unit) {
        let route = unit.route.as_ref().expect("a route to fly");
        // It takes the next waypoint no faster than the turn there allows:
        // straight on at cruise, a right angle or more at a stop, and its
        // destination at a stop. It brakes on a little less than it can.
        let leg = route[0] - here;
        let corner = match route.get(1) {
            Some(&after) => {
                let turn = leg.normalized().dot((after - route[0]).normalized());
                flight.cruise_mps * turn.max(0.0)
            }
            None => 0.0,
        };
        let speed = flight
            .cruise_mps
            .min((corner * corner + 2.0 * BRAKING_SHARE * accel * leg.length()).sqrt());
        leg.normalized() * speed
    } else {
        v2(0.0, 0.0)
    };
    let air = unit.air.as_mut().expect("an aircraft's flight state");
    let change = wanted - air.velocity;
    let most = accel * dt;
    air.velocity = if change.length() > most {
        air.velocity + change.normalized() * most
    } else {
        wanted
    };
    let velocity = air.velocity;
    let next = here + velocity * dt + drift * dt;
    let reach = unit.hull.map_or(0.0, |h| h.x);
    let target = cruise_height(ctx, next, velocity, flight, reach);
    let climb = (target - unit.position.z).clamp(-flight.climb_mps * dt, flight.climb_mps * dt);
    unit.position = next.with_z(unit.position.z + climb);
    unit.drive_speed_mps = velocity.length();
    turn(unit, flight, velocity, dt);
    // Reached waypoints fall away; the last one completes the move.
    if let Some(route) = unit.route.as_mut() {
        let passed = |route: &Vec<V2>| {
            let reach = if route.len() > 1 {
                WAYPOINT_M
            } else {
                ARRIVAL_M
            };
            route
                .first()
                .is_some_and(|w| (*w - next).inside_radius(reach))
        };
        while passed(route) {
            route.remove(0);
        }
        if route.is_empty() {
            arrive(unit);
            unit.air
                .as_mut()
                .expect("an aircraft's flight state")
                .velocity = v2(0.0, 0.0);
        }
    }
}

/// Plan a route when the goal is new or moved, or the side's knowledge changed.
fn plan(ctx: &MovementContext, unit: &mut Unit, side: &mut SideGeometry) {
    let Some((goal, _)) = unit.movement_goal() else {
        unit.route = None;
        unit.planned_goal = None;
        unit.state = MoveState::Idle;
        return;
    };
    let stale = unit
        .planned_goal
        .is_none_or(|g| (g - goal).outside_radius(GOAL_REPLAN_M))
        || unit.planned_revision != side.revision;
    if !stale {
        return;
    }
    let here = unit.position.xy();
    let grid = side.air_grid(ctx.world, ctx.rules, ctx.authored);
    let mut searched = 0;
    unit.route = Some(grid.route(here, goal, &mut searched));
    side.air_cells_searched += searched;
    unit.planned_goal = Some(goal);
    unit.planned_revision = side.revision;
    unit.route_from = here;
    unit.state = MoveState::Moving;
}

/// How far other aircraft push unit `i` this tick, metres a second.
fn separation(ctx: &MovementContext, units: &[Unit], i: usize) -> V2 {
    let apart = ctx.rules.air.separation_m;
    let here = units[i].position.xy();
    let mut push = v2(0.0, 0.0);
    for (j, other) in units.iter().enumerate() {
        if j == i || !other.alive() || !other.airborne() {
            continue;
        }
        let away = here - other.position.xy();
        let d = away.length();
        if d >= apart {
            continue;
        }
        // Coincident aircraft part along a direction fixed by their order.
        let dir = if d > 1e-6 {
            away * (1.0 / d)
        } else if i < j {
            v2(-1.0, 0.0)
        } else {
            v2(1.0, 0.0)
        };
        push = push + dir * (SEPARATION_MPS * (1.0 - d / apart));
    }
    push
}

/// Face the way it flies; hovering, face as ordered.
fn turn(unit: &mut Unit, flight: Flight, velocity: V2, dt: f64) {
    let wanted = if velocity.length() > HOVER_MPS {
        libm::atan2(velocity.y, velocity.x)
    } else if let Some(f) = unit.turn_to {
        f
    } else {
        return;
    };
    let error = crate::math::wrap_angle(wanted - unit.yaw);
    let most = flight.turn_rad_s * dt;
    unit.yaw = crate::math::wrap_angle(unit.yaw + error.clamp(-most, most));
    if error.abs() <= most && unit.turn_to.is_some_and(|f| f == wanted) {
        unit.turn_to = None;
    }
}

/// The height an aircraft over `at` wants (D4, D24): cruise above the
/// ground, and clearance over any body top or forest canopy it will pass
/// within `reach` of before it could climb the whole band; never above the
/// ceiling over `at`.
pub fn cruise_height(
    ctx: &MovementContext,
    at: V2,
    velocity: V2,
    flight: Flight,
    reach: f64,
) -> f64 {
    let air = &ctx.rules.air;
    let world = ctx.world;
    let ground = |p: V2| world.height_at(p.x, p.y).unwrap_or(0.0);
    let ahead = velocity.length() * (air.ceiling_agl_m - air.cruise_agl_m) / flight.climb_mps;
    let dir = if velocity.length() > 1e-9 {
        velocity.normalized()
    } else {
        v2(0.0, 0.0)
    };
    let canopy = ctx.rules.forests.rule.canopy_height_m;
    let steps = (ahead / LOOKAHEAD_STEP_M).ceil() as usize;
    let mut wanted = f64::NEG_INFINITY;
    for k in 0..=steps {
        let p = at + dir * (k as f64 * LOOKAHEAD_STEP_M).min(ahead);
        let g = ground(p);
        wanted = wanted.max(g + air.cruise_agl_m);
        if world.forest_ground(p.x, p.y) {
            wanted = wanted.max(g + canopy + air.clearance_m);
        }
        for prop in world.props_near(p, reach) {
            if prop.footprint().distance(p) <= reach {
                wanted = wanted.max(prop.top_z() + air.clearance_m);
            }
        }
    }
    wanted.min(ground(at) + air.ceiling_agl_m)
}
