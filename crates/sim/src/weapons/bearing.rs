//! Which way a mount points: its bearing toward a point, turned at its
//! turret's rate and inside its arc, or fixed to the hull.
use super::{Mount, MountSpec};
use crate::math::{wrap_angle, V3};
use crate::units::Unit;

pub(super) fn bearing_from(unit: &Unit, point: V3) -> f64 {
    let to = point.xy() - unit.position.xy();
    libm::atan2(to.y, to.x)
}

pub(super) fn turned_mount(
    mount: &Mount,
    spec: &MountSpec,
    yaw: f64,
    desired: f64,
    step: f64,
) -> f64 {
    if let Some(arc) = spec.arc {
        let at = arc.off(yaw, mount.bearing).clamp(-arc.half, arc.half);
        let to = arc.off(yaw, desired).clamp(-arc.half, arc.half);
        yaw + arc.centre + at + (to - at).clamp(-step, step)
    } else {
        let err = wrap_angle(desired - mount.bearing);
        mount.bearing + err.clamp(-step, step)
    }
}

pub(super) fn point_mount(
    unit: &Unit,
    mount: &mut Mount,
    spec: &MountSpec,
    point: V3,
    turret_step: f64,
) {
    let desired = bearing_from(unit, point);
    mount.bearing = if spec.arc.is_some() || spec.turret {
        turned_mount(mount, spec, unit.yaw, desired, turret_step)
    } else if hull_fixed(unit, spec) {
        unit.yaw
    } else {
        desired
    };
}

/// Whether a vehicle mount must turn its hull to aim.
pub(super) fn hull_fixed(unit: &Unit, spec: &MountSpec) -> bool {
    unit.is_vehicle() && !spec.turret && spec.on.is_none()
}
