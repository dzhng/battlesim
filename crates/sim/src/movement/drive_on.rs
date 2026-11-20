//! Driving on while a new route is worked out. A vehicle sent somewhere
//! broadly ahead of it does not stand to wait for the planner: it keeps the
//! stretch of its route its brakes need to stop it, the new route is planned
//! from that stretch's end, and it follows the stretch into the new route
//! when that comes. If the route is long in coming it has braked to rest at
//! the stretch's end, and holds there.
use contract::observation::MoveState;

use crate::math::{wrap_angle, V2};
use crate::units::Unit;

use super::{arrive, drive, rejoin, MovementContext, ROLLING_MPS};

/// The stretch of its route a vehicle ordered to `goal` drives on along
/// while its way there is worked out: as far as braking from the speed it
/// has takes it. `None` if it stops for the route instead: it is not
/// rolling along a route in the order's gear, it is in a shunt, or `goal`
/// lies more than `navigation.drive_on_deg` off the way it travels.
pub(super) fn stretch(ctx: &MovementContext, unit: &Unit, goal: V2) -> Option<Vec<V2>> {
    let route = unit.route.as_ref()?;
    let mobility = unit.ground(ctx.rules);
    let gear = drive::gear(unit);
    let speed = unit.drive_speed_mps * gear;
    if speed <= ROLLING_MPS || unit.shunt.is_some() {
        return None;
    }
    let here = unit.position.xy();
    let to_goal = goal - here;
    let off = wrap_angle(libm::atan2(to_goal.y, to_goal.x) - drive::travel(unit.yaw, gear));
    if off.abs() > ctx.rules.navigation.drive_on_deg.to_radians() {
        return None;
    }
    let braking = mobility.road_mps / mobility.drive?.feel.braking_s;
    let mut left = speed * speed / (2.0 * braking);
    let mut stretch = Vec::new();
    let mut at = here;
    for &point in route {
        let leg = point - at;
        let length = leg.length();
        if length >= left {
            stretch.push(at + leg * (left / length.max(f64::MIN_POSITIVE)));
            break;
        }
        stretch.push(point);
        left -= length;
        at = point;
    }
    (!stretch.is_empty()).then_some(stretch)
}

/// A route planned from `from` arrives. A vehicle still on the stretch it
/// drove on along, which ends at `from`, follows the stretch into it; any
/// other picks the route up at its stretch nearest it.
pub(super) fn join(unit: &Unit, from: V2, route: &mut Vec<V2>) {
    match unit.route.as_ref().filter(|way| way.last() == Some(&from)) {
        Some(way) => {
            route.splice(0..0, way.iter().copied());
        }
        None => rejoin(unit.position.xy(), from, route),
    }
}

/// A vehicle reaches its route's end. While its way somewhere new is still
/// being worked out (`awaiting`) that is only as far as it drove on
/// meanwhile, and it holds there for the route; otherwise it has arrived.
pub(super) fn route_end(rules: &contract::scenario::Rules, unit: &mut Unit, awaiting: bool) {
    if !awaiting {
        return arrive(rules, unit);
    }
    unit.route = None;
    unit.drive_speed_mps = 0.0;
    unit.state = MoveState::Planning;
}
