//! Whether a mount can shoot a target now: permitted, with a usable kind
//! and a clear solution, assessed once per target each tick.
use contract::catalog::AltitudeLayer;
use contract::ids::Side;
use contract::observation::ActionReason;

use super::window_requests::assess_windows;
use super::{
    bearing_from, engage, guidance_clear, muzzle, nose_aimed, nose_on, permitted, preferred_kind,
    seeker_in_fov, FireContext, Mount, MountSpec, Resolved, Target,
};
use crate::flight::FiringSolution;
use crate::math::{v3, V3};
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
/// flies to meet the target and where it meets it, and the seen soldier
/// ([`seen_soldiers`]) its rounds start from.
#[derive(Clone, Copy)]
pub(super) struct Ready {
    pub(super) kind: usize,
    pub(super) time_of_flight_s: f64,
    pub(super) intercept: V3,
    pub(super) velocity: V3,
    pub(super) soldier: usize,
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
    // A squad is aimed at the middle of what its side sees. That middle may
    // lie where no round or sight line reaches though its soldiers stand in
    // view (behind the car they crouch round, inside the walls of the
    // building they fight from): a gun then aims at the soldiers its side
    // sees, the first it can hit in the order its rounds walk them.
    let at = |point| {
        aim(
            ctx,
            unit,
            units,
            mount,
            spec,
            k,
            target,
            &Resolved { point, ..*r },
        )
    };
    let (soldier, s) = match at(r.point) {
        Err(why @ (ActionReason::BlockedTrajectory | ActionReason::NoOwnSight)) => {
            let seen = seen_soldiers(ctx, unit.side, units, target);
            (seen.iter().enumerate())
                .find_map(|(j, &p)| at(p).ok().map(|s| (j, s)))
                .ok_or(why)?
        }
        solved => (0, solved?),
    };
    Ok(Ready {
        kind: k,
        time_of_flight_s: s.time_of_flight_s,
        intercept: s.intercept,
        velocity: s.velocity,
        soldier,
    })
}

/// Where to aim at the soldiers `side` saw of a squad at its last sighting:
/// each one's body now, out on his lean if he leans
/// ([`Soldier::exposed`](crate::units::Soldier::exposed)). Empty for a
/// vehicle, an area, a point or a round.
pub(super) fn seen_soldiers(
    ctx: &FireContext,
    side: Side,
    units: &[Unit],
    target: Target,
) -> Vec<V3> {
    let Target::Unit(u) = target else {
        return Vec::new();
    };
    let unit = &units[u.0 as usize];
    match ctx.knowledge[side.index()].track(u) {
        Some(t) if !unit.is_vehicle() => (t.members.iter())
            .map(|&m| {
                unit.members[m].exposed(ctx.tick) + v3(0.0, 0.0, ctx.rules.physics.infantry_aim_m)
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// The diving run a jet fires this row on: a pass's ground missile that does
/// not fire on its own (a stand-off missile leaves whenever it can), on a
/// run that dives. `None` for any other row, or a run that stays level.
fn diving_run(
    unit: &Unit,
    weapon: &contract::weapons::WeaponDefinition,
) -> Option<crate::air::pass::PassPlan> {
    let missile = !weapon.fires_on_its_own
        && crate::air::pass::pass_weapon(weapon)
            == Some(crate::air::pass::PassWeapon::GroundMissile);
    let plan = unit.air.as_ref()?.pass?;
    (missile && plan.dive.is_some()).then_some(plan)
}

/// Whether this mount, loading kind `k`, can fire at `r.point` from here:
/// its seeker cone, a winged aircraft's nose, a guided round's sight line,
/// and a clear solution ([`engage`]).
#[allow(clippy::too_many_arguments)]
fn aim(
    ctx: &FireContext,
    unit: &Unit,
    units: &[Unit],
    mount: &Mount,
    spec: &MountSpec,
    k: usize,
    target: Target,
    r: &Resolved,
) -> Result<FiringSolution, ActionReason> {
    let weapon = &ctx.arsenal.weapons[spec.kinds[k]].def;
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
    // row it aims by the nose fires only with the target in its nose-on
    // cone, seen from the muzzle, heading and pitch both. A missile a run
    // that dives fires leaves in the dive, with the nose on the run's aim:
    // off the rail before the pull-out, its target ahead to steer onto.
    let cone = ctx.rules.air.fixed_wing.nose_on_deg.to_radians();
    let bears = |point| {
        r.fire(weapon).is_some_and(|fire| {
            let from = muzzle(unit, mount, spec, ctx.rules, mount.bearing);
            nose_on(unit, from, point, fire.range_m, cone)
        })
    };
    let held_for_the_dive = diving_run(unit, weapon)
        .is_some_and(|run| run.dive != Some(crate::air::pass::Dive::Diving) || !bears(run.aim));
    if held_for_the_dive || (nose_aimed(unit, spec, weapon) && !bears(r.point)) {
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
    engage(ctx, unit, units, mount, spec, k, target, r)
}
