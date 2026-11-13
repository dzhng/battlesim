//! Popping smoke (`Order::PopSmoke`): a unit spends one of its charges and
//! a cloud blooms where it stands. It is an instant change of state beside
//! the unit's orders, never one of them, so whatever the unit was doing it
//! goes on doing. The clouds themselves live in the world
//! ([`crate::world::SmokeClouds`]); a side sees the ones whose ground it sees
//! ([`seen`]).
use contract::ids::{Side, Tick};
use contract::observation::VisibilityField;
use contract::scenario::Rules;
use contract::smoke::CloudState;

use crate::units::Unit;
use crate::world::{Popped, SmokeClouds};

/// Pop `unit`'s smoke on `tick`: one charge spent, one cloud centred on it.
/// A unit with no charge left, or off the field, pops nothing (`false`).
pub fn pop(unit: &mut Unit, clouds: &mut SmokeClouds, tick: Tick, rules: &Rules) -> bool {
    let Some(capability) = &unit.unit_type(rules).capabilities.smoke else {
        return false;
    };
    let on_field = unit.on_field();
    let Some(smoke) = unit.smoke.as_mut().filter(|s| s.charges > 0 && on_field) else {
        return false;
    };
    smoke.charges -= 1;
    clouds.pop(
        unit.side,
        unit.position,
        &capability.cloud,
        tick,
        rules.tick_hz,
    );
    true
}

/// Whether a side whose fog field is `field` sees `cloud` (D12): any fog
/// cell whose centre lies within a cell of the cloud's rim is visible. A
/// cloud hides its own middle, so only its near rim can be seen.
pub fn seen_by(field: &VisibilityField, cloud: &CloudState) -> bool {
    let cell = field.cell_m;
    let reach = cloud.radius_m + cell;
    let [cx, cy] = cloud.centre;
    let index = |v: f64, n: u32| ((v / cell).floor().max(0.0) as u32).min(n.saturating_sub(1));
    for j in index(cy - reach, field.ny)..=index(cy + reach, field.ny) {
        for i in index(cx - reach, field.nx)..=index(cx + reach, field.nx) {
            let (x, y) = ((i as f64 + 0.5) * cell, (j as f64 + 0.5) * cell);
            let (dx, dy) = (x - cx, y - cy);
            if dx * dx + dy * dy <= reach * reach && field.visible(x, y) {
                return true;
            }
        }
    }
    false
}

/// The live clouds `side` sees: its own, always, and any other its fog
/// field shows ([`seen_by`]).
pub fn seen<'a>(
    clouds: &'a SmokeClouds,
    side: Side,
    field: &'a VisibilityField,
) -> impl Iterator<Item = (&'a Popped, &'a CloudState)> + 'a {
    clouds
        .live()
        .filter(move |(popped, cloud)| popped.side == side || seen_by(field, cloud))
}
