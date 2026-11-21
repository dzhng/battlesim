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

/// Whether a jet's gun fixed along its nose, firing from `from`, bears on a
/// point in its authored range: the point lies within `cone_rad` of its
/// nose, heading and pitch both.
pub fn nose_on(unit: &Unit, from: V3, target: V3, range: f64, cone_rad: f64) -> bool {
    let delta = target - from;
    let distance = delta.length();
    range.is_finite()
        && range >= 0.0
        && distance <= range
        && distance > 0.0
        && unit.nose().dot(delta) >= distance * libm::cos(cone_rad)
}

/// A row a winged aircraft aims by its nose: fixed in the airframe,
/// unguided and powered (a gun, a rocket pod). A seeker row aims by its
/// cone, a guided one by its guidance and a bomb by its pass's release cue.
pub(crate) fn nose_aimed(
    unit: &Unit,
    spec: &MountSpec,
    def: &contract::weapons::WeaponDefinition,
) -> bool {
    unit.fixed_wing()
        && hull_fixed(unit, spec)
        && def.ballistics.guidance.is_none()
        && def.ballistics.delivery == contract::ballistics::Delivery::Powered
}

/// Where a round fired from `origin` at `point` is sent: a row aimed by the
/// nose sends it along the nose, as far out as the point, so it goes where
/// the nose points and no nearer the point; any other row at the point.
pub(super) fn boresight(
    unit: &Unit,
    spec: &MountSpec,
    def: &contract::weapons::WeaponDefinition,
    origin: V3,
    point: V3,
) -> V3 {
    if nose_aimed(unit, spec, def) {
        origin + unit.nose() * (point - origin).length()
    } else {
        point
    }
}

/// Whether a winged aircraft heading along `yaw` has its nose on `bearing`,
/// within `cone_rad` (`air.fixed_wing.nose_on_deg`): what its fixed gun
/// needs to fire, and what an attack pass needs before its dive begins.
pub fn nose_on_bearing(yaw: f64, bearing: f64, cone_rad: f64) -> bool {
    wrap_angle(bearing - yaw).abs() <= cone_rad
}
