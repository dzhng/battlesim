//! Which way a mount points: its bearing toward a point, turned at its
//! turret's rate and inside its arc, or fixed to the hull.
use super::{Mount, MountSpec};
use crate::math::{wrap_angle, V3};
use crate::units::Unit;

/// A turret's arc (`contract::weapons::MountArc`), in radians.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Arc {
    pub centre: f64,
    pub half: f64,
}

impl Arc {
    /// `bearing` off a hull heading `yaw`, relative to the arc's centre.
    fn off(self, yaw: f64, bearing: f64) -> f64 {
        wrap_angle(bearing - yaw - self.centre)
    }

    /// Whether a hull heading `yaw` brings `bearing` inside the arc.
    pub fn holds(self, yaw: f64, bearing: f64) -> bool {
        self.off(yaw, bearing).abs() <= self.half
    }
}

pub(super) fn bearing_from(unit: &Unit, point: V3) -> f64 {
    let to = point.xy() - unit.position.xy();
    libm::atan2(to.y, to.x)
}

/// A turned mount's next bearing toward `desired`, one `step` at most.
pub(super) fn turned_mount(
    mount: &Mount,
    spec: &MountSpec,
    yaw: f64,
    desired: f64,
    step: f64,
) -> f64 {
    if let Some(arc) = spec.arc {
        // Within its arc, it turns the way that stays inside it, never
        // round through the cabin; outside, it waits at the arc's edge.
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

/// A gun fixed in its hull (D8): it bears where the body faces, so the body
/// turns to aim it. A soldier's weapon turns with him, and a mount on a
/// turret with its turret.
pub(super) fn hull_fixed(unit: &Unit, spec: &MountSpec) -> bool {
    unit.is_vehicle() && !spec.turret && spec.on.is_none()
}

/// Whether a jet's fixed nose gun bears on a point in its authored range,
/// within `cone_rad` of its heading.
pub fn nose_on(unit: &Unit, target: V3, range: f64, cone_rad: f64) -> bool {
    let delta = target - unit.position;
    range.is_finite()
        && range >= 0.0
        && delta.length() <= range
        && nose_on_bearing(unit.yaw, libm::atan2(delta.y, delta.x), cone_rad)
}

/// Whether a winged aircraft heading along `yaw` has its nose on `bearing`,
/// within `cone_rad` (`air.fixed_wing.nose_on_deg`): what its fixed gun
/// needs to fire, and what an attack pass needs before its dive begins.
pub fn nose_on_bearing(yaw: f64, bearing: f64, cone_rad: f64) -> bool {
    wrap_angle(bearing - yaw).abs() <= cone_rad
}
