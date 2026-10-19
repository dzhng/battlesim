//! The backline post: where a unit whose weapons cannot engage ground
//! targets (air defence) holds, behind its group with the artillery (D57).

use contract::command::Order;
use contract::observation::{ObservationFrame, OwnUnit};
use contract::scenario::Rules;

use super::distance;

/// Whether `unit` holds the backline: a ground unit whose weapons cannot
/// engage ground targets, read from weapon data. Recon keeps its recon post
/// whatever it carries, and aircraft keep their flight logic.
pub(crate) fn holds(rules: &Rules, unit: &OwnUnit) -> bool {
    let catalog = rules.catalog.get(unit.kind);
    catalog.mobility.layer() == contract::catalog::AltitudeLayer::Ground
        && !catalog.has_role("recon")
        && !super::super::unit_can_engage_ground(rules, unit.kind)
}

/// The post for `unit`: beside its player's nearest artillery piece, else on
/// its lane three quarters of the way from the entry to the staging point,
/// behind the group.
fn goal(
    rules: &Rules,
    frame: &ObservationFrame,
    unit: &OwnUnit,
    entry: [f64; 2],
    staging: [f64; 2],
) -> [f64; 2] {
    let here = [unit.position[0], unit.position[1]];
    let artillery = frame.own.iter().filter(|candidate| {
        candidate.owner == unit.owner && rules.catalog.get(candidate.kind).has_role("artillery")
    });
    if let Some(piece) = artillery.min_by(|a, b| {
        distance([a.position[0], a.position[1]], here)
            .total_cmp(&distance([b.position[0], b.position[1]], here))
            .then(a.id.cmp(&b.id))
    }) {
        return [piece.position[0], piece.position[1]];
    }
    [
        entry[0] + (staging[0] - entry[0]) * 0.75,
        entry[1] + (staging[1] - entry[1]) * 0.75,
    ]
}

/// The move that takes `unit` to its post, unless it is already headed
/// there (within `reaim_m`).
pub(crate) fn order(
    rules: &Rules,
    frame: &ObservationFrame,
    unit: &OwnUnit,
    entry: [f64; 2],
    staging: [f64; 2],
    reaim_m: f64,
) -> Option<Order> {
    let goal = goal(rules, frame, unit, entry, staging);
    super::super::live_goal(unit)
        .is_none_or(|old| distance(old, goal) > reaim_m)
        .then(|| Order::Move {
            units: vec![unit.id],
            gesture: frame.tick,
            goal,
            route: contract::command::RoutePolicy::Fastest,
            direction: contract::command::MoveDirection::Forward,
            facing: None,
        })
}
