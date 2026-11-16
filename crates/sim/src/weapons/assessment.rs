//! Whether a mount can shoot a target now: permitted, with a usable kind
//! and a clear solution, assessed once per target each tick.
use contract::catalog::AltitudeLayer;
use contract::observation::ActionReason;

use super::window_requests::assess_windows;
use super::{
    bearing_from, engage, guidance_clear, hull_fixed, nose_on, permitted, preferred_kind,
    seeker_in_fov, FireContext, Mount, MountSpec, Resolved, Target,
};
use crate::math::V3;
use crate::units::Unit;

/// One mount's assessments this tick, by target: choosing its lock and then
/// firing ask of the same target with the same world, so each target is
/// assessed once (its line tests and firing solutions are the costly part).
#[derive(Default)]
pub(super) struct Assessed {
    pub(super) values: Vec<(Target, Result<Ready, ActionReason>)>,
    pub(super) prospective_windows: bool,
}

/// A shot an assessment found: the kind it would load, how long its round
/// flies to meet the target and where it meets it.
#[derive(Clone, Copy)]
pub(super) struct Ready {
    pub(super) kind: usize,
    pub(super) time_of_flight_s: f64,
    pub(super) intercept: V3,
}

impl Assessed {
    /// The kind this mount would load at `target`, or why it can't fire.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn target(
        &mut self,
        ctx: &FireContext,
        unit: &Unit,
        units: &[Unit],
        mount: &Mount,
        spec: &MountSpec,
        target: Target,
        resolved: &Resolved,
    ) -> Result<usize, ActionReason> {
        self.shot(ctx, unit, units, mount, spec, target, resolved)
            .map(|ready| ready.kind)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn shot(
        &mut self,
        ctx: &FireContext,
        unit: &Unit,
        units: &[Unit],
        mount: &Mount,
        spec: &MountSpec,
        target: Target,
        resolved: &Resolved,
    ) -> Result<Ready, ActionReason> {
        if let Some(&(_, a)) = self.values.iter().find(|(t, _)| *t == target) {
            return a;
        }
        let a = if self.prospective_windows {
            assess_windows(ctx, unit, units, mount, spec, target, resolved)
        } else {
            assess(ctx, unit, units, mount, spec, target, resolved)
        };
        self.values.push((target, a));
        a
    }
}

/// Could this mount shoot `target` now: permitted, with a usable kind and a
/// clear solution. The kind it would load, or why not.
pub(super) fn assess(
    ctx: &FireContext,
    unit: &Unit,
    units: &[Unit],
    mount: &Mount,
    spec: &MountSpec,
    target: Target,
    r: &Resolved,
) -> Result<Ready, ActionReason> {
    let k = preferred_kind(ctx, mount, spec, r);
    let on_its_own = k.is_some_and(|k| ctx.arsenal.weapons[spec.kinds[k]].def.fires_on_its_own);
    if !on_its_own && !permitted(ctx, unit, target) {
        return Err(ActionReason::HoldingFire);
    }
    let k = k.ok_or(if mount.out_of_ammo() {
        ActionReason::OutOfAmmo
    } else {
        ActionReason::NoCompatibleTarget
    })?;
    let weapon = &ctx.arsenal.weapons[spec.kinds[k]].def;
    if r.layer == AltitudeLayer::Ground {
        let pass = unit.air.as_ref().and_then(|air| air.pass);
        let bomb_pass = pass.is_some_and(|p| p.weapon == crate::air::pass::PassWeapon::Bomb);
        // Bombs leave only on a bombing pass's release cue, never between
        // passes (only a winged aircraft carries them: catalog admission).
        let release_cue = bomb_pass && unit.air.as_ref().is_some_and(|air| air.pass_release);
        match crate::air::pass::pass_weapon(weapon) {
            Some(crate::air::pass::PassWeapon::Bomb) if !release_cue => {
                return Err(ActionReason::HoldingFire);
            }
            Some(crate::air::pass::PassWeapon::Gun) if bomb_pass => {
                return Err(ActionReason::NoCompatibleTarget);
            }
            _ => {}
        }
    }
    // A seeker only acquires inside its authored forward cone.  Once the
    // round is launched, its existing guidance contract takes over.
    if let Some(fov) = weapon.seeker_fov_deg {
        let bearing = bearing_from(unit, r.point);
        let forward = if spec.turret { mount.bearing } else { unit.yaw };
        if !seeker_in_fov(forward, bearing, fov) {
            return Err(ActionReason::NoFacingSlot);
        }
    }
    // A winged aircraft's flight, never a weapon, points the airframe, so a
    // hull-fixed row it aims by the nose (unguided and powered: a fixed gun,
    // a rocket pod) fires only nose-on. A seeker row aims by its cone, a
    // guided one by its guidance and a bomb by its pass's release cue.
    if unit.fixed_wing()
        && hull_fixed(unit, spec)
        && weapon.ballistics.guidance.is_none()
        && weapon.ballistics.delivery == contract::ballistics::Delivery::Powered
        && !r.fire(weapon).is_some_and(|fire| {
            let cone = ctx.rules.air.fixed_wing.nose_on_deg.to_radians();
            nose_on(unit, r.point, fire.range_m, cone)
        })
    {
        return Err(ActionReason::NoFacingSlot);
    }
    // Shared identification supplies the target (or the interceptor lock a
    // round); the launcher still needs a physical sight line to acquire and
    // support it (P05).
    let guided = ctx.arsenal.weapons[spec.kinds[k]]
        .profile
        .turn_rad_s
        .is_some();
    let anti_radiation_target = weapon.anti_radiation && r.emits;
    if guided
        && (!matches!(target, Target::Unit(_) | Target::Round(_)) && !anti_radiation_target
            || !r.current
            || (!anti_radiation_target
                && !guidance_clear(ctx.world, ctx.rules, unit, mount, r.point)))
    {
        return Err(ActionReason::NoOwnSight);
    }
    engage(ctx, unit, units, mount, spec, k, target, r).map(|s| Ready {
        kind: k,
        time_of_flight_s: s.time_of_flight_s,
        intercept: s.intercept,
    })
}
