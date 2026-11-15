//! Vehicle traffic (Q14): whether a vehicle's move would run into another
//! live vehicle. A move may never deepen an overlap, padded or not, so
//! touching hulls can always part.

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
