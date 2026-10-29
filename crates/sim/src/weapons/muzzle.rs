//! Where a mount's rounds leave: its muzzle, placed by its carrier and bearing.
use contract::scenario::Rules;

use super::{operator, Mount, MountSpec};
use crate::math::{v2, v3, V3};
use crate::units::Unit;

/// Where a mount's muzzle sits for a unit at `position` heading `yaw`: its
/// carrier's `ring` turns with the hull, its pivot round that ring with the
/// carrier's bearing `carried`, and its muzzle round the pivot with its own
/// `bearing`.
pub(super) fn placed_muzzle(
    position: V3,
    yaw: f64,
    spec: &MountSpec,
    muzzle: V3,
    carried: f64,
    bearing: f64,
) -> V3 {
    let turn = |p: V3, by: f64| v2(p.x, p.y).rotated(by).with_z(p.z);
    position + turn(spec.ring, yaw) + turn(spec.pivot - spec.ring, carried) + turn(muzzle, bearing)
}

pub(super) fn infantry_offset(spec: &MountSpec, rules: &Rules, bearing: f64) -> V3 {
    match spec.muzzle {
        Some(muzzle) => placed_muzzle(v3(0.0, 0.0, 0.0), bearing, spec, muzzle, bearing, bearing),
        None => v3(0.0, 0.0, rules.physics.infantry_muzzle_m),
    }
}

/// Where a mount's rounds leave when it points along `bearing`: each mount
/// fires from its own muzzle. Its pivot swings with its carrier round the
/// carrier's ring (the turret it sits on, round that turret's pivot at its
/// bearing, or the hull, round its middle), and its muzzle turns with its
/// own bearing about the pivot. An infantry offset turns with its
/// operator and starts at his position. A hand weapon fires at the
/// infantry muzzle height: a single one from its operator where he stands
/// (his lean is [`fire_from`]'s), a squad weapon's volley judged first from
/// the squad's middle (each soldier then fires from his own).
pub(super) fn muzzle(
    unit: &Unit,
    mount: &Mount,
    spec: &MountSpec,
    rules: &Rules,
    bearing: f64,
) -> V3 {
    if unit.hull.is_none() {
        let at = operator(unit, mount)
            .filter(|_| !spec.squad)
            .map_or(unit.position, |k| unit.members[k].position);
        return at + infantry_offset(spec, rules, bearing);
    }
    let muzzle = spec
        .muzzle
        .expect("hull mounts have a muzzle (catalog admission)");
    let carried = spec.on.map_or(unit.yaw, |c| unit.mounts[c].bearing);
    placed_muzzle(unit.position, unit.yaw, spec, muzzle, carried, bearing)
}
