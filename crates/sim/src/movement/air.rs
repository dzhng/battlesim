//! Flight (D4, D6, D7). An aircraft flies its side's air route at cruise
//! speed, gathering and shedding it over the air's acceleration time and
//! slowing for its route's corners. Braking for its goal it runs a little
//! past, as a pilot flaring a beat late does, then eases back onto the point
//! without turning round. It holds
//! `cruise_agl_m` above the ground and climbs early enough to pass
//! `clearance_m` over anything taller on its way, never above
//! `ceiling_agl_m`. It turns on the spot toward where it flies, or to its
//! ordered facing at rest. Aircraft never collide; ones closer than
//! `separation_m` drift apart. Idle at a set-up supply truck it sinks to the
//! low hover (D10) and is served there; any order or engagement lifts it back.

use super::{arrive, may_advance, MovementContext, SideGeometry, GOAL_REPLAN_M};
use crate::math::{v2, V2};
use crate::units::{Flight, Motion, Settle, Unit};
use contract::catalog::AltitudeLayer;
use contract::observation::{MoveState, ServiceStatus};
use contract::scenario::Rules;

/// Within this of a waypoint on the way it counts as passed.
const WAYPOINT_M: f64 = 1.0;
/// Within this of its destination it has arrived.
const ARRIVAL_M: f64 = 0.05;
/// Climb lookahead samples are this far apart along the flight.
const LOOKAHEAD_STEP_M: f64 = 8.0;
/// How fast two overlapping aircraft drift apart at full overlap.
const SEPARATION_MPS: f64 = 4.0;
/// The share of its acceleration an aircraft plans its braking on: the rest
/// straightens it out of a turn onto its last leg as it brakes, which would
/// otherwise carry it well past its goal.
const BRAKING_SHARE: f64 = 0.6;
/// Below this ground speed an aircraft is hovering, and faces as ordered.
const HOVER_MPS: f64 = 1.0;
/// How far the low hover clears the tallest ground hull, and anything else
/// under the airframe while it holds there (D27).
const LOW_HOVER_CLEARANCE_M: f64 = 2.0;

/// Fly unit `i` one tick.
pub(super) fn step_aircraft(
    ctx: &MovementContext,
    units: &mut [Unit],
    i: usize,
    sides: &mut [SideGeometry; 3],
) {
    let dt = 1.0 / ctx.rules.tick_hz as f64;
    let Motion::Air(flight) = units[i].motion else {
        unreachable!("only aircraft fly");
    };
    let side = units[i].side.index();
    if units[i].air.is_some_and(|air| air.traversing) {
        return;
    }
    let dogfight_target = units[i].air.as_ref().and_then(|air| air.dogfight_target);
    let dogfight_explicit = units[i].orders.iter().any(|o| {
        matches!(
            o,
            crate::units::UnitOrder::Move(_) | crate::units::UnitOrder::Attack { .. }
        )
    });
    let dogfight_side = dogfight_target
        .and_then(|target| units.get(target.0 as usize))
        .and_then(|target| {
            let relative = target.position.xy() - units[i].position.xy();
            if relative.length() > 1e-6 {
                let direction = relative.normalized();
                Some(v2(-direction.y, direction.x))
            } else {
                None
            }
        });
    if let Some(target) = dogfight_target {
        if let Some(target_unit) = units.get(target.0 as usize) {
            if target_unit.alive()
                && !units[i].orders.iter().any(|o| {
                    matches!(
                        o,
                        crate::units::UnitOrder::Move(_) | crate::units::UnitOrder::Attack { .. }
                    )
                })
            {
                let goal = target_unit.position.xy();
                units[i].route = Some(vec![goal]);
                units[i].planned_goal = Some(goal);
                units[i].route_from = units[i].position.xy();
                units[i].state = MoveState::Moving;
            }
        }
    }
    plan(ctx, &mut units[i], &mut sides[side]);
    let drift = separation(ctx, units, i);
    let afterburner = unit_afterburner(&units[i], ctx.rules);
    let unit = &mut units[i];
    let here = unit.position.xy();
    let cruise_mps = flight.cruise_mps * afterburner.0;
    let accel = cruise_mps / ctx.rules.air.acceleration_s;
    let air_rules = &ctx.rules.air;
    let mut wanted = if may_advance(ctx, unit) {
        let route = unit.route.as_ref().expect("a route to fly");
        let state = unit.air.as_mut().expect("an aircraft's flight state");
        // On the leg that ends at its goal it aims past the goal by its
        // share of the leg; stopped there, it aims at the goal itself.
        if route.len() == 1 {
            let goal = route[0];
            if state.settle.is_none_or(|s| s.goal != goal) {
                let leg = goal - here;
                let past = (air_rules.settle_share * leg.length()).min(air_rules.settle_max_m);
                state.settle = Some(Settle {
                    goal,
                    aim: goal + leg.normalized() * past,
                    back: past < ARRIVAL_M,
                });
            }
        } else {
            state.settle = None;
        }
        if let Some(s) = state.settle.as_mut().filter(|s| !s.back) {
            if (s.aim - here).inside_radius(ARRIVAL_M) || (here - s.aim).dot(s.aim - s.goal) >= 0.0
            {
                s.back = true;
            }
        }
        let target = match state.settle {
            Some(s) if !s.back => s.aim,
            _ => route[0],
        };
        // It takes the next waypoint no faster than the turn there allows
        // (straight on at cruise, a right angle or more at a stop) nor than
        // it can stop from in the rest of the route, and its destination at a
        // stop. It brakes on a little less than it can.
        let leg = target - here;
        let corner = match route.get(1) {
            Some(&after) => {
                let turn = leg.normalized().dot((after - route[0]).normalized());
                let rest: f64 = route.windows(2).map(|w| (w[1] - w[0]).length()).sum();
                (cruise_mps * turn.max(0.0)).min((2.0 * BRAKING_SHARE * accel * rest).sqrt())
            }
            None => 0.0,
        };
        let speed =
            cruise_mps.min((corner * corner + 2.0 * BRAKING_SHARE * accel * leg.length()).sqrt());
        leg.normalized() * speed
    } else {
        v2(0.0, 0.0)
    };
    // An aircraft that is automatically closing another aircraft jinks across
    // its nose. The weave is deterministic and disappears as soon as an
    // explicit move or attack order takes over, so a player can always make a
    // pilot hold a clean line when that is the intent.
    if dogfight_target.is_some() && !dogfight_explicit {
        if let Some(side) = dogfight_side {
            let phase = ctx.tick as f64 * 0.17 + unit.id.0 as f64 * 0.31;
            wanted = wanted + side * (cruise_mps * 0.22 * libm::sin(phase));
        }
    }
    let air = unit.air.as_mut().expect("an aircraft's flight state");
    let change = wanted - air.velocity;
    let most = accel * dt;
    air.velocity = if change.length() > most {
        air.velocity + change.normalized() * most
    } else {
        wanted
    };
    let velocity = air.velocity;
    air.fuel_s = (air.fuel_s - dt * afterburner.1).max(0.0);
    let next = here + velocity * dt + drift * dt;
    let reach = unit.hull.map_or(0.0, |h| h.x);
    let target = height_target(ctx, next, velocity, flight, reach, sinks(unit));
    // It sinks as fast as it climbs.
    let climb = (target - unit.position.z).clamp(-flight.climb_mps * dt, flight.climb_mps * dt);
    unit.position = next.with_z(unit.position.z + climb);
    unit.drive_speed_mps = velocity.length();
    let back = unit.air.and_then(|a| a.settle).is_some_and(|s| s.back);
    turn(unit, flight, velocity, back, dt);
    // Reached waypoints fall away; the last one completes the move. A
    // waypoint on the way is also passed once the aircraft is beyond it along
    // the leg that led there: one skimmed by a metre or two wide at cruise
    // would otherwise be turned back for.
    if let Some(route) = unit.route.as_mut() {
        let passed = |route: &Vec<V2>, from: V2| match route.first() {
            Some(&w) if route.len() > 1 => {
                (w - next).inside_radius(WAYPOINT_M) || (next - w).dot(w - from) >= 0.0
            }
            // The goal: only once it has eased back onto it.
            Some(&w) => back && (w - next).inside_radius(ARRIVAL_M),
            None => false,
        };
        while passed(route, unit.route_from) {
            unit.route_from = route.remove(0);
        }
        if route.is_empty() {
            arrive(unit);
            let air = unit.air.as_mut().expect("an aircraft's flight state");
            air.velocity = v2(0.0, 0.0);
            air.settle = None;
        }
    }
}

/// Return the authored speed and fuel multipliers when the persistent toggle
/// is enabled. A legacy air row has no afterburner capability and remains
/// unchanged even if a stale toggle bit is present.
fn unit_afterburner(unit: &Unit, rules: &Rules) -> (f64, f64) {
    if !unit.air.is_some_and(|air| air.afterburner) {
        return (1.0, 1.0);
    }
    unit.unit_type(rules)
        .capabilities
        .air
        .as_ref()
        .and_then(|air| air.afterburner)
        .map(|a| (a.speed_multiplier, a.fuel_multiplier))
        .unwrap_or((1.0, 1.0))
}

/// The resupply sink (D10): with no order and no target, inside a set-up
/// supply truck's zone, it holds the low hover. `service` is last tick's,
/// since supply serves after movement; an order or an engagement since then
/// lifts it this tick. Its height alone never counts as moving (D37), so
/// sinking never turns that status to `Moving`.
fn sinks(unit: &Unit) -> bool {
    unit.orders.is_empty()
        && !crate::weapons::engaged(unit)
        && matches!(
            unit.service,
            ServiceStatus::Serving | ServiceStatus::NoStock | ServiceStatus::Full
        )
}

/// The low hover's height over the ground (D27): just over the top of the
/// tallest ground hull in the catalog, so it clears any ground unit under it
/// whatever the roster. The resupply sink holds it, and transport lands at it.
pub(crate) fn low_hover(rules: &Rules) -> f64 {
    let catalog = &rules.catalog;
    let tallest = catalog
        .indices()
        .map(|i| catalog.get(i))
        .filter(|t| t.mobility.layer() == AltitudeLayer::Ground)
        .filter_map(|t| t.hull())
        .map(|h| 2.0 * h.half_extents_m[2])
        .fold(0.0, f64::max);
    tallest + LOW_HOVER_CLEARANCE_M
}

/// Plan a route when the goal is new or moved, or the side's knowledge changed.
fn plan(ctx: &MovementContext, unit: &mut Unit, side: &mut SideGeometry) {
    if unit
        .air
        .as_ref()
        .is_some_and(|air| air.dogfight_target.is_some())
        && unit.route.is_some()
    {
        return;
    }
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
    unit.route = Some(grid.route(here, goal));
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

/// Face where a gun fixed in its hull must bear (D8), whichever way it flies;
/// otherwise the way it flies, or hovering, as ordered.
fn turn(unit: &mut Unit, flight: Flight, velocity: V2, back: bool, dt: f64) {
    let wanted = if let Some(face) = unit.reach.face {
        face
    } else if back {
        // Easing back onto its goal it keeps facing as it came.
        return;
    } else if velocity.length() > HOVER_MPS {
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
/// ground, or the low hover when `low`, and clearance over any body top or
/// forest canopy it will pass within `reach` of before it could climb the
/// whole band; never above the ceiling over `at`.
fn height_target(
    ctx: &MovementContext,
    at: V2,
    velocity: V2,
    flight: Flight,
    reach: f64,
    low: bool,
) -> f64 {
    let air = &ctx.rules.air;
    let (agl, clearance) = if low {
        (low_hover(ctx.rules), LOW_HOVER_CLEARANCE_M)
    } else {
        (air.cruise_agl_m, air.clearance_m)
    };
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
        wanted = wanted.max(g + agl);
        if world.forest_ground(p.x, p.y) {
            wanted = wanted.max(g + canopy + clearance);
        }
        for prop in world.props_near(p, reach) {
            if prop.footprint().distance(p) <= reach {
                wanted = wanted.max(prop.top_z() + clearance);
            }
        }
    }
    wanted.min(ground(at) + air.ceiling_agl_m)
}
