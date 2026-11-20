//! A wheeled vehicle's drive in along a right-drag's facing (Q9): the
//! straight lead-in its route ends on.
use contract::command::MoveDirection;

use super::drive;
use crate::navigation::NavGrid;
use crate::units::Unit;

/// A wheeled vehicle sent forward to a right-drag's facing drives in along
/// it as best it can: its route ends on a straight lead-in on the heading it
/// can line up on ([`drive::line_up_heading`]), long enough that the turn
/// onto it is a forward one ([`drive::lead_in`]). It takes only a lead-in on
/// its way: one that starts nearer the end than it turns toward it from, and
/// at least a turning circle's width from there. Without one, or without
/// room for it, the vehicle keeps the route it has; either way it turns the
/// rest of the way on the spot once there (`arrive`).
pub(super) fn line_up(rules: &contract::scenario::Rules, unit: &mut Unit, grid: &NavGrid) {
    let Some(drive) = unit.ground(rules).drive.filter(|d| !d.tracked) else {
        return;
    };
    let Some(order) = unit.orders.front().and_then(|o| o.movement()) else {
        return;
    };
    let (Some(facing), MoveDirection::Forward) = (order.facing, order.direction) else {
        return;
    };
    let Some(route) = unit
        .route
        .as_ref()
        .filter(|r| r.last() == Some(&order.destination))
    else {
        return;
    };
    let end = order.destination;
    let here = unit.position.xy();
    let before = route.len().checked_sub(2).map_or(here, |i| route[i]);
    let Some(heading) = drive::line_up_heading(before, end, facing) else {
        return;
    };
    let lead = drive::lead_in(end, heading, &drive, unit.hull.map_or(0.0, |h| h.x));
    // Driving out past it to turn back in, or onto a start inside its own
    // turning circle, it would circle the start for ever.
    let behind = (before - end).length() <= (lead - end).length();
    if behind || (lead - before).length() < 2.0 * drive.radius_m {
        return;
    }
    let mut lined = route[..route.len() - 1].to_vec();
    lined.extend([lead, end]);
    if grid.placement_fits(lead, &unit.ground(rules))
        && grid.route_fits(here, &lined, &unit.ground(rules))
    {
        unit.route = Some(lined);
    }
}
