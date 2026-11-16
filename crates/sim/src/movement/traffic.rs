//! Vehicle traffic (Q14): whether a vehicle's move would run into another
//! live vehicle, and when it stops waiting on one to go round it. A move
//! may never deepen an overlap, padded or not, so touching hulls can always
//! part.

use contract::ids::UnitId;

use super::TRAFFIC_MARGIN_M;
use crate::math::{v2, Obb2, V2};
use crate::units::Unit;

/// The first live vehicle that vehicle `i`, moved to `next` at `yaw`, would run into.
pub(super) fn in_the_way(units: &[Unit], i: usize, next: V2, yaw: f64) -> Option<UnitId> {
    units.iter().enumerate().find_map(|(j, other)| {
        (j != i
            && other.ground_footprint().is_some()
            && other.on_field()
            && vehicle_conflict(&units[i], next, yaw, other))
        .then_some(other.id)
    })
}

/// Would this vehicle, moved to `next`, run into the vehicle `other`? Only a move that
/// makes an existing overlap no worse is allowed, so touching units can part.
fn vehicle_conflict(unit: &Unit, next: V2, yaw: f64, other: &Unit) -> bool {
    let before = unit.ground_footprint().expect("a vehicle has a hull");
    let after = Obb2 {
        center: next,
        yaw,
        ..before
    };
    let other = other.ground_footprint().expect("a vehicle has a hull");
    hull_conflict(before, after, other)
}

/// Look-ahead and actual steps keep the same clearance, including when parting.
pub(super) fn hull_conflict(before: Obb2, after: Obb2, other: Obb2) -> bool {
    let padded = |hull: Obb2| Obb2 {
        half: hull.half + v2(TRAFFIC_MARGIN_M, TRAFFIC_MARGIN_M),
        ..hull
    };
    let depth = |hull: Obb2| hull.separation(&other).map_or(0.0, |v| v.length());
    depth(padded(after)) > depth(padded(before)) + 1e-9 || depth(after) > depth(before) + 1e-9
}

/// Whether wheeled vehicle `i`, held by the vehicle `b`, now works its way
/// round it. A driver waits for traffic that is moving, and for a moment
/// for traffic that is not; past that (a stall) he goes round. One parked
/// (with nowhere to go) he goes round at once, however often his orders
/// change: it will not move off for him. Two that
/// hold each other up do not both go round, into each other: the later
/// (the higher id) gives way, unless it found no way round, when the other
/// tries. A shunt already worked out is given its chance before another is.
pub(super) fn gives_way(ctx: &super::MovementContext, units: &[Unit], i: usize, b: UnitId) -> bool {
    let (unit, other) = (&units[i], &units[b.0 as usize]);
    let patience = (super::STALL_REPLAN_S * ctx.rules.tick_hz as f64) as u64;
    let waited = |since: u64| ctx.tick.saturating_sub(since) > patience;
    let wheeled = unit.ground().drive.is_some_and(|d| !d.tracked);
    let standing = other.drive_speed_mps.abs() <= super::ROLLING_MPS;
    let parked = other.movement_goal().is_none();
    let mutual = other.blocker == Some(unit.id);
    let found_no_way = other.shunt.as_ref().is_some_and(|s| s.legs.is_empty());
    wheeled
        && standing
        && (!mutual || unit.id > other.id || found_no_way)
        && (parked || waited(unit.stalls.1))
        && unit.shunt.as_ref().is_none_or(|s| waited(s.tick))
}
