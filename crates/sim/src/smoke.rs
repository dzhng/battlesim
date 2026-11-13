//! Popping smoke (`Order::PopSmoke`): a unit spends one of its charges and
//! a cloud blooms where it stands. It is an instant change of state beside
//! the unit's orders, never one of them, so whatever the unit was doing it
//! goes on doing. The clouds themselves live in the world
//! ([`crate::world::SmokeClouds`]).
use contract::ids::Tick;
use contract::scenario::Rules;

use crate::units::Unit;
use crate::world::SmokeClouds;

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
