//! The windows a garrison's soldiers ask for: each weapon's target and
//! whether it could engage from a window that faces it, before the building's
//! window plan seats them ([`crate::garrison::Windows`]).
use contract::command::Engagement;
use contract::observation::ActionReason;

use super::tick::{preferred_mount, MountPlan};
use super::{
    assess, candidates, choose_lock, compatible, permitted, preferred_kind, resolve_for, Assessed,
    FireContext, Mount, MountSpec, Ready, Resolved, Target,
};
use crate::math::V3;
use crate::units::Unit;

/// A garrisoned squad's mounts that have something to face: each one's
/// participants and the point it faces.
pub struct MountAim {
    pub members: Vec<usize>,
    pub point: V3,
    /// Authored physical ownership, independent of how many riflemen survive.
    pub single_operator: bool,
}
pub type MountAims = Vec<MountAim>;

/// Whether a carried gun could engage after taking a suitable window. The
/// normal assessment owns range, trajectory and guidance LOS here too; only
/// position and idle watching permission are projected. Known attackers still
/// constrain return fire; a missing current window cannot prevent requesting one.
pub(super) fn assess_windows(
    ctx: &FireContext,
    unit: &Unit,
    units: &[Unit],
    mount: &Mount,
    spec: &MountSpec,
    target: Target,
    resolved: &Resolved,
) -> Result<Ready, ActionReason> {
    let mut prospective = unit.clone();
    if unit.attackers.is_empty() {
        prospective.engagement = Engagement::FireAtWill;
    }
    let current = assess(ctx, &prospective, units, mount, spec, target, resolved);
    if current.is_ok() || !permitted(ctx, &prospective, target) {
        return current;
    }
    let (g, plan) = ctx.windows.held(unit).expect("a garrison requests windows");
    let facing = ctx.rules.garrison.slot_facing_min_deg.to_radians();
    for member in candidates(unit, mount, spec) {
        for (seat, window) in plan
            .iter()
            .enumerate()
            .filter(|(_, w)| w.slot.faces(resolved.point.xy(), facing))
        {
            prospective.members[member].position = window.position;
            prospective
                .hold
                .as_mut()
                .unwrap()
                .building_mut()
                .unwrap()
                .seats[member] = Some(seat);
            let assessment = assess(ctx, &prospective, units, mount, spec, target, resolved);
            if assessment.is_ok() {
                return assessment;
            }
        }
        prospective.members[member].position = unit.members[member].position;
        prospective
            .hold
            .as_mut()
            .unwrap()
            .building_mut()
            .unwrap()
            .seats[member] = g.seats[member];
    }
    current
}

/// Each soldier requests a window for one weapon, using the same priority as
/// active selection but assessing prospective windows. This breaks the circular
/// dependency between a useful weapon and the window it needs, without letting
/// an inactive rifle pull a launcher operator away from his target.
pub fn garrison_aims(ctx: &FireContext, units: &[Unit]) -> Vec<(usize, MountAims)> {
    units
        .iter()
        .enumerate()
        .filter(|(_, u)| u.alive() && u.garrisoned())
        .map(|(i, u)| {
            let specs = ctx.arsenal.specs(u.kind);
            let plans: Vec<_> = u
                .mounts
                .iter()
                .map(|m| {
                    let spec = &specs[m.spec];
                    let mut candidate = m.clone();
                    let mut assessed = Assessed {
                        prospective_windows: true,
                        ..Default::default()
                    };
                    let explicit = u
                        .attack_target()
                        .filter(|&t| compatible(ctx, m, spec, t, units));
                    let idle_reason =
                        choose_lock(ctx, u, units, &mut candidate, spec, explicit, &mut assessed)
                            .idle_reason;
                    let target = m
                        .support
                        .map(|s| s.target)
                        .or_else(|| candidate.lock.as_ref().map(|l| l.target))?;
                    let r = resolve_for(ctx, u, m, spec, target, units)?;
                    let assessment = assessed.shot(ctx, u, units, m, spec, target, &r);
                    Some(MountPlan {
                        stationary: false,
                        idle_reason,
                        resolved: Some(r),
                        assessment: Some(assessment),
                        kind: preferred_kind(ctx, m, spec, &r),
                    })
                })
                .collect();
            let choices: Vec<_> = (0..u.members.len())
                .map(|member| {
                    preferred_mount(
                        u,
                        member,
                        specs,
                        &ctx.arsenal.weapons,
                        ctx.rules,
                        &plans,
                        false,
                    )
                })
                .collect();
            let aims = plans
                .iter()
                .enumerate()
                .filter_map(|(m, plan)| {
                    let r = plan.as_ref()?.resolved?;
                    let members: Vec<_> = candidates(u, &u.mounts[m], &specs[u.mounts[m].spec])
                        .filter(|&member| choices[member] == Some(m))
                        .collect();
                    (!members.is_empty()).then_some(MountAim {
                        members,
                        point: r.point,
                        single_operator: !specs[u.mounts[m].spec].squad,
                    })
                })
                .collect();
            (i, aims)
        })
        .collect()
}
