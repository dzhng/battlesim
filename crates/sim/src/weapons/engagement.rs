//! Whether a mount can fire a kind at a resolved point from here: its reach,
//! its arc, its launch path and the friendlies on the way, from the unit, a
//! garrison seat, or a soldier where he stands or out on his lean.
use contract::ballistics::Delivery;
use contract::ids::UnitId;
use contract::observation::ActionReason;

use super::{
    bearing_from, candidates, fire_from, fires_into, infantry_offset, muzzle, FireContext, Mount,
    MountSpec, Resolved, Target, Weapon,
};
use crate::flight::{predicted_path, timed_path, Aim, FiringSolution, NoSolution};
use crate::lean;
use crate::math::V3;
use crate::units::Unit;
use crate::world::PropId;

/// P11: withhold when a friendly vehicle will be on the predicted path as the
/// round passes, or in the blast at `burst` when it bursts. Each hull is
/// judged where its own velocity carries it by then, so a wingman in trail
/// holds for the lead he would hit, not for where the lead was when he fired.
/// Friendly infantry never withholds a shot (it can still be hit).
/// `leaning_round` is the hull a soldier leans round: his lean point
/// clears it, so it never withholds his shot.
#[allow(clippy::too_many_arguments)]
fn friendly_in_line(
    ctx: &FireContext,
    shooter: &Unit,
    units: &[Unit],
    origin: V3,
    (s, burst): (&FiringSolution, V3),
    weapon: &Weapon,
    leaning_round: Option<UnitId>,
    carrier: V3,
) -> bool {
    let margin = ctx.rules.physics.friendly_prefire_margin_m;
    let (def, profile) = (&weapon.def, &weapon.profile);
    let path = timed_path(
        &ctx.arsenal.config,
        profile,
        origin,
        s.velocity + carrier,
        s.time_of_flight_s,
    );
    units
        .iter()
        .filter(|u| {
            u.side == shooter.side && u.id != shooter.id && u.hull.is_some() && u.on_field()
        })
        .filter(|u| Some(u.id) != leaning_round)
        .any(|u| {
            // One frame per hull: its rotation is not recomputed per sample.
            // The path is read in the hull's own moving frame instead.
            let hull = u.hull_frame();
            let drift = u.flight_velocity();
            let relative = |(p, t): (V3, f64)| p - drift * t;
            path.windows(2).any(|w| {
                let (a, b) = (relative(w[0]), relative(w[1]));
                // A chord that never comes near the hull has no sample near it.
                if !hull.may_come_within(a, b, margin) {
                    return false;
                }
                // Sample each chord densely enough for a hull-sized margin.
                let n = ((b - a).length() / 1.0).ceil().max(1.0) as usize;
                (0..=n).any(|k| hull.distance(a + (b - a) * (k as f64 / n as f64)) < margin)
            }) || (def.blast_radius_m > 0.0
                && hull.distance(burst - drift * s.time_of_flight_s) < def.blast_radius_m)
        })
}

/// Whether a launcher that needs a clear launch path has one: the first
/// `launch.clearance_m` of its round's arc from the muzzle meets
/// neither the ground, nor a body that stops rounds (whether fire could
/// break it or not), nor tree crowns. Only the launcher's end is judged: a
/// high arc clears a low wall beside it, and a target in a wood or past a
/// hill is still fair. This is not the target line ([`super::guidance_clear`]).
fn launch_clear(
    ctx: &FireContext,
    origin: V3,
    s: &FiringSolution,
    weapon: &Weapon,
    carrier: V3,
) -> bool {
    let path = predicted_path(
        &ctx.arsenal.config,
        &weapon.profile,
        origin,
        s.velocity + carrier,
        s.time_of_flight_s,
    );
    let mut left = ctx.rules.launch.clearance_m;
    for w in path.windows(2) {
        let chord = w[1] - w[0];
        let len = chord.length();
        if left <= 0.0 {
            break;
        }
        if len == 0.0 {
            continue;
        }
        let end = w[0] + chord * (left.min(len) / len);
        if !ctx.world.segment_clear(w[0], end) || ctx.world.foliage_depth(w[0], end) > 0.0 {
            return false;
        }
        left -= len;
    }
    true
}

/// Can this mount fire the given kind at the resolved point from here? The
/// solution, or why not.
#[allow(clippy::too_many_arguments)]
pub(super) fn engage(
    ctx: &FireContext,
    unit: &Unit,
    units: &[Unit],
    mount: &Mount,
    spec: &MountSpec,
    k: usize,
    target: Target,
    r: &Resolved,
) -> Result<FiringSolution, ActionReason> {
    let weapon = &ctx.arsenal.weapons[spec.kinds[k]];
    let carrier = unit.flight_velocity();
    let bearing = if spec.turret {
        bearing_from(unit, r.point)
    } else {
        mount.bearing
    };
    let from = |origin: V3, past: Option<PropId>, hull: Option<UnitId>| {
        let distance = (r.point - origin).length();
        if distance < weapon.def.min_range_m {
            // OutOfRange asks an ordered unit to advance; too close holds.
            return Err(ActionReason::HoldingFire);
        }
        if weapon.def.ballistics.delivery == Delivery::Unpowered {
            // A bomb released now falls from here with the carrier's
            // velocity; its pass's release cue is its reach. It holds for a
            // friendly on its fall or in its blast where it lands.
            let Some(fall) = crate::flight::release(
                origin,
                carrier,
                weapon.profile.gravity(&ctx.arsenal.config),
                r.point,
            ) else {
                // Dropped from below its target, it falls away from it.
                return Err(ActionReason::OutOfRange);
            };
            let fall_s = fall.fall_s;
            let lands = (origin.xy() + carrier.xy() * fall_s).with_z(r.point.z);
            let s = FiringSolution {
                velocity: V3::default(),
                time_of_flight_s: fall_s,
                intercept: lands,
                arc: crate::flight::ArcKind::Low,
            };
            if friendly_in_line(ctx, unit, units, origin, (&s, lands), weapon, hull, carrier) {
                return Err(ActionReason::FriendlyInLine);
            }
            return Ok(s);
        }
        let Some(reach) = r.fire(&weapon.def).map(|fire| fire.range_m) else {
            return Err(ActionReason::OutOfRange);
        };
        let aim = Aim {
            origin,
            target: r.point,
            target_velocity: r.velocity,
            carrier,
        };
        match crate::flight::solve_fire(
            ctx.world,
            &ctx.arsenal.config,
            &weapon.profile,
            &aim,
            past,
            |id| fires_into(ctx, weapon, mount.ammo[k], target, id),
        ) {
            Err(NoSolution::OutOfReach) => Err(ActionReason::OutOfRange),
            Err(NoSolution::Blocked { .. }) => Err(ActionReason::BlockedTrajectory),
            // The shot must meet its target within its layer's reach, not
            // merely start inside it: the lead's meeting point is the one
            // point the aim uses and the one the reach is judged at.
            Ok((s, _)) if (s.intercept - origin).length() > reach => Err(ActionReason::OutOfRange),
            Ok((s, _))
                if weapon.def.requires_clear_launch_path
                    && !launch_clear(ctx, origin, &s, weapon, carrier) =>
            {
                Err(ActionReason::BlockedTrajectory)
            }
            Ok((s, burst))
                if friendly_in_line(
                    ctx,
                    unit,
                    units,
                    origin,
                    (&s, burst),
                    weapon,
                    hull,
                    carrier,
                ) =>
            {
                Err(ActionReason::FriendlyInLine)
            }
            Ok((s, _)) => Ok(s),
        }
    };
    if unit.garrisoned() {
        let facing = ctx.rules.garrison.slot_facing_min_deg.to_radians();
        let mut result = Err(ActionReason::NoFacingSlot);
        for k in candidates(unit, mount, spec) {
            let Some(seat) = ctx
                .windows
                .seat(unit, k)
                .filter(|s| s.slot.faces(r.point.xy(), facing))
            else {
                continue;
            };
            // Assessment and launch use the same occupied window's muzzle.
            result = from(
                seat.position + infantry_offset(spec, ctx.rules, bearing),
                None,
                None,
            );
            if result.is_ok() {
                return result;
            }
        }
        return result;
    }
    let origin = muzzle(unit, mount, spec, ctx.rules, bearing);
    let at_unit = from(origin, None, None);
    // A soldier's weapon fires from his own muzzle: with the squad's
    // middle (or the operator where he stands) blocked, or a friendly hull
    // in the way, it can fire if any of its soldiers can, from where he
    // stands or out on his lean.
    if !matches!(
        at_unit,
        Err(ActionReason::BlockedTrajectory | ActionReason::FriendlyInLine)
    ) || unit.hull.is_some()
    {
        return at_unit;
    }
    let blockers = lean::hulls(units, ctx.rules);
    candidates(unit, mount, spec)
        .map(|k| {
            let f = fire_from(
                ctx,
                &blockers,
                &unit.members[k],
                r.point,
                infantry_offset(spec, ctx.rules, bearing),
            )?;
            from(f.origin, f.past, f.hull).ok()
        })
        .find_map(|s| s.map(Ok))
        .unwrap_or(at_unit)
}
