//! Automatic target choice for weapon mounts.

use super::*;

/// Automatic choice (W06, W09, W10), in order: an identified target some kind can
/// damage, ranked first by whether a loaded seeker row engages it, then by
/// known return-fire threat, then the share of the mount's
/// damage its facing armour lets through, then weighted distance and observed
/// id; for the unlimited default gun, the nearest
/// identified target it cannot hurt; and only with no identified enemy in
/// reach (`enemy_in_reach`), the nearest area for general-purpose kinds. A
/// weapon with a finite supply never fires at what it cannot hurt. Only a
/// target the mount could shoot now qualifies; otherwise the first obstacle
/// met is the reason.
pub(super) fn select(
    ctx: &FireContext,
    unit: &Unit,
    units: &[Unit],
    mount: &Mount,
    spec: &MountSpec,
    assessed: &mut Assessed,
) -> (Option<Target>, ActionReason) {
    let knowledge = &ctx.knowledge[unit.side.index()];
    let here = unit.position;
    let weapons = &ctx.arsenal.weapons;
    // What a loaded seeker row engages comes first: a seeker is carried
    // for those targets.
    let seeker_target = |target: &Unit| {
        let class = target.target_class(ctx.rules);
        spec.kinds.iter().enumerate().any(|(slot, &kind)| {
            let def = &weapons[kind].def;
            mount.has_rounds(slot)
                && def.seeker.is_some()
                && def.engages(target.layer(), Some(class)).is_some()
        })
    };
    let weighted_distance = |target: &Unit, position: V3, id: UnitId| {
        let engaged = engaging(units, unit, id);
        body_distance(position, here)
            * (1.0 + ctx.rules.targeting.engaged_distance_penalty * engaged as f64)
            * spread_factor(
                ctx.seed,
                unit.id,
                mount.spec,
                target.id,
                ctx.rules.targeting.distance_spread,
            )
    };
    let mut ranked: Vec<(bool, bool, f64, f64, u32, UnitId)> = knowledge
        .identified_now(ctx.tick)
        .map(|(u, t)| {
            let target = &units[u.0 as usize];
            (
                seeker_target(target),
                return_fire_threat(ctx, unit, target.kind, t),
                share_through(ctx, unit, mount, spec, target),
                weighted_distance(target, t.position, u),
                t.id.0,
                u,
            )
        })
        .collect();
    ranked.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then(b.1.cmp(&a.1))
            .then(b.2.total_cmp(&a.2))
            .then(a.3.total_cmp(&b.3))
            .then(a.4.cmp(&b.4))
    });
    let mut by_distance: Vec<(f64, u32, UnitId)> = ranked
        .iter()
        .map(|&(_, _, _, d, id, u)| (d, id, u))
        .collect();
    by_distance.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    let mut areas: Vec<(f64, ContactId)> = if enemy_in_reach(ctx, unit, units, mount, spec) {
        Vec::new()
    } else {
        knowledge
            .all_contacts()
            .map(|c| (body_distance(c.center, here), c.id))
            .collect()
    };
    areas.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));

    let stages = ranked
        .iter()
        .map(|&(_, _, _, _, _, u)| (Target::Unit(u), true))
        .chain(by_distance.iter().map(|&(.., u)| (Target::Unit(u), false)))
        .chain(areas.iter().map(|&(_, c)| (Target::Contact(c), true)));
    let mut reason = None;
    for (target, must_damage) in stages {
        // An area centred off the map, or an air contact, has no ground to aim at.
        let Some(r) = resolve(ctx, unit.side, target, units) else {
            continue;
        };
        match assessed.target(ctx, unit, units, mount, spec, target, &r) {
            Ok(k) => {
                // The middle stage is only the unlimited default gun against
                // what it cannot hurt.
                let harms = hurts(ctx, spec, k, target, units);
                if harms == must_damage && (harms || weapons[spec.kinds[k]].def.fires_regardless())
                {
                    return (Some(target), ActionReason::Aiming);
                }
            }
            Err(e) => {
                reason.get_or_insert(e);
            }
        }
    }
    (None, reason.unwrap_or(ActionReason::NoCompatibleTarget))
}

/// How many other units of the chooser's side have a mount locked on `target`.
fn engaging(units: &[Unit], choosing: &Unit, target: UnitId) -> u32 {
    units
        .iter()
        .filter(|other| other.id != choosing.id && other.side == choosing.side)
        .filter(|other| {
            other.mounts.iter().any(|mount| {
                mount
                    .lock
                    .as_ref()
                    .is_some_and(|lock| lock.target == Target::Unit(target))
            })
        })
        .count() as u32
}

/// One seeded draw per (unit, mount, target), never per tick, so a crew's
/// preference between equal targets holds still.
fn spread_factor(seed: u64, unit: UnitId, mount: usize, target: UnitId, spread: f64) -> f64 {
    let key = seed
        ^ u64::from(unit.0).rotate_left(11)
        ^ (mount as u64).rotate_left(29)
        ^ u64::from(target.0).rotate_left(47);
    let mut rng = contract::random::Rng::new(contract::random::mix(key));
    1.0 + spread * (2.0 * rng.unit() - 1.0)
}

/// The largest share of its damage any of the mount's loaded kinds gets
/// through `target`: the armour it shows the shooter, or all of it into
/// soldiers.
pub(super) fn share_through(
    ctx: &FireContext,
    unit: &Unit,
    mount: &Mount,
    spec: &MountSpec,
    target: &Unit,
) -> f64 {
    let armor = target
        .armor(ctx.rules)
        .map(|a| (a, target.hull_face(unit.position)));
    (0..spec.kinds.len())
        .filter(|&k| mount.has_rounds(k))
        .map(|k| &ctx.arsenal.weapons[spec.kinds[k]].def)
        .filter(|def| {
            def.engages(target.layer(), Some(target.target_class(ctx.rules)))
                .is_some()
        })
        .map(|def| {
            armor.map_or(1.0, |(armor, face)| {
                damage::pierce_share(def, 0, ctx.rules, armor, face)
            })
        })
        .fold(0.0, f64::max)
}

/// Whether the mount has a kind that can damage an explicit target (W08).
pub(super) fn compatible(
    ctx: &FireContext,
    mount: &Mount,
    spec: &MountSpec,
    target: Target,
    units: &[Unit],
) -> bool {
    let (armor, layer, class) = match target {
        Target::Unit(u) => {
            let unit = &units[u.0 as usize];
            let class = unit.target_class(ctx.rules);
            (
                Some(unit.armor(ctx.rules).copied()),
                unit.layer(),
                Some(class),
            )
        }
        _ => (None, AltitudeLayer::Ground, None),
    };
    (0..spec.kinds.len()).any(|k| {
        let def = &ctx.arsenal.weapons[spec.kinds[k]].def;
        mount.has_rounds(k)
            && def.engages(layer, class).is_some()
            && effective(def, armor, ctx.rules)
    })
}

/// What `choose_lock` concluded: the reason to show if the mount ends with
/// no lock, and whether it could shoot the ordered target now (None with no
/// order, or while the order's target is not seen this tick).
pub(super) struct LockChoice {
    pub(super) idle_reason: ActionReason,
    pub(super) ordered: Option<Result<usize, ActionReason>>,
}

/// Keep, replace or choose the mount's lock (W07, W08, V12). An order names
/// a priority, never a silent weapon: whenever the mount can shoot the
/// ordered target it does, at once; while it cannot, it fights whatever it
/// can, and only with nothing to shoot does it turn to the ordered target and
/// wait for its shot.
pub(super) fn choose_lock(
    ctx: &FireContext,
    unit: &Unit,
    units: &[Unit],
    mount: &mut Mount,
    spec: &MountSpec,
    explicit: Option<Target>,
    assessed: &mut Assessed,
) -> LockChoice {
    // A ground point never outlives the order that named it.
    if mount
        .lock
        .as_ref()
        .is_some_and(|l| Some(l.target) != explicit && matches!(l.target, Target::Ground(_)))
    {
        mount.lock = None;
    }
    let Some(t) = explicit else {
        return LockChoice {
            idle_reason: automatic_lock(ctx, unit, units, mount, spec, assessed),
            ordered: None,
        };
    };
    let take = |mount: &mut Mount| {
        if mount.lock.as_ref().is_none_or(|lock| lock.target != t) {
            mount.lock = Some(Lock {
                target: t,
                engaging: false,
            })
        }
    };
    let resolved = resolve_for(ctx, unit, mount, spec, t, units);
    let ordered = resolved
        .filter(|r| r.current)
        .map(|r| assessed.target(ctx, unit, units, mount, spec, t, &r));
    // Shootable now; or just out of sight, within the grace, where the order
    // keeps its aim on the last sighting rather than turning away a moment.
    let holding =
        resolved.is_some_and(|r| !r.current) && mount.lock.as_ref().is_some_and(|l| l.target == t);
    if matches!(ordered, Some(Ok(_))) || holding {
        take(mount);
        return LockChoice {
            idle_reason: ActionReason::NoCompatibleTarget,
            ordered,
        };
    }
    let idle_reason = automatic_lock(ctx, unit, units, mount, spec, assessed);
    let shootable = mount.lock.as_ref().is_some_and(|l| {
        resolve_for(ctx, unit, mount, spec, l.target, units)
            .filter(|r| r.current)
            .is_some_and(|r| {
                assessed
                    .target(ctx, unit, units, mount, spec, l.target, &r)
                    .is_ok()
            })
    });
    if !shootable && resolved.is_some() {
        take(mount);
    }
    LockChoice {
        idle_reason,
        ordered,
    }
}

/// Automatic choice for a mount without an order it can follow now: keep a
/// target it can still shoot, else choose afresh (`select`). Returns the
/// reason to show if it ends with no lock.
pub(super) fn automatic_lock(
    ctx: &FireContext,
    unit: &Unit,
    units: &[Unit],
    mount: &mut Mount,
    spec: &MountSpec,
    assessed: &mut Assessed,
) -> ActionReason {
    // An incoming round a shot can catch comes before anything else: the crew
    // shoots it down, then turns back to what it was on (`select`, once the
    // round is gone).
    if !matches!(
        mount.lock.as_ref().map(|l| l.target),
        Some(Target::Round(_))
    ) {
        if let Some(round) = catchable_round(ctx, unit, units, mount, spec, assessed) {
            mount.lock = Some(Lock {
                target: round,
                engaging: false,
            });
            return ActionReason::Aiming;
        }
    }
    // An area is never kept on its own once an identified enemy is in reach.
    if mount
        .lock
        .as_ref()
        .is_some_and(|l| matches!(l.target, Target::Contact(_)))
        && enemy_in_reach(ctx, unit, units, mount, spec)
    {
        mount.lock = None;
    }
    let current = mount
        .lock
        .as_ref()
        .and_then(|l| resolve_for(ctx, unit, mount, spec, l.target, units).map(|r| (l.target, r)));
    let reconsider = match &current {
        None => true,
        // Identification lapsing within the grace never makes it replaceable.
        Some((_, r)) if !r.current => false,
        // A target the mount can still shoot is kept until it no longer can
        // (out of range, obstructed, dead): a crew does not swap targets as
        // the ranking shifts. The one exception is the default gun working on
        // what it cannot hurt, which takes anything it can.
        Some((t, r)) => match assessed.target(ctx, unit, units, mount, spec, *t, r) {
            Err(_) => true,
            Ok(k) => !hurts(ctx, spec, k, *t, units),
        },
    };
    if !reconsider {
        return ActionReason::NoCompatibleTarget;
    }
    let (choice, reason) = select(ctx, unit, units, mount, spec, assessed);
    match choice {
        Some(t) if current.is_none_or(|(c, _)| c != t) => {
            mount.lock = Some(Lock {
                target: t,
                engaging: false,
            });
        }
        // Nothing better: a lock that still resolves stays and reports its own obstacle.
        Some(_) => {}
        None if current.is_some() => {}
        None => mount.lock = None,
    }
    reason
}

/// The nearest enemy round in this mount's `missile` reach, by its assigned
/// layer, that a shot can catch now: the same assessment as any
/// target, its lead solution included (a round speeding away has none),
/// and the shot meets it in time and within reach.
pub(super) fn catchable_round(
    ctx: &FireContext,
    unit: &Unit,
    units: &[Unit],
    mount: &Mount,
    spec: &MountSpec,
    assessed: &mut Assessed,
) -> Option<Target> {
    let reach = |layer| {
        (0..spec.kinds.len())
            .filter(|&k| mount.has_rounds(k))
            .filter_map(|k| {
                ctx.arsenal.weapons[spec.kinds[k]]
                    .def
                    .engages(layer, Some(TargetClass::Missile))
            })
            .map(|fire| fire.range_m)
            .fold(0.0, f64::max)
    };
    let mut near: Vec<(f64, &IncomingRound)> = ctx
        .rounds
        .iter()
        .filter(|r| r.side != unit.side)
        .map(|r| (body_distance(r.position, unit.position), r))
        .filter(|&(d, r)| {
            let reach = reach(r.layer);
            reach > 0.0 && d <= reach
        })
        .collect();
    near.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.id.cmp(&b.1.id)));
    near.into_iter().find_map(|(_, round)| {
        let target = Target::Round(RoundTarget {
            id: round.id,
            layer: round.layer,
        });
        let r = resolve(ctx, unit.side, target, units)?;
        // The shot's lead must meet the round before the round's own flight
        // ends; that it meets it within reach, every shot's assessment asks
        // ([`engage`]).
        assessed
            .shot(ctx, unit, units, mount, spec, target, &r)
            .is_ok_and(|ready| ready.time_of_flight_s <= round.ends_in_s)
            .then_some(target)
    })
}

/// Whether an identified enemy the unit may fire at stands within reach of
/// one of this mount's kinds with rounds left. While one does, the mount
/// never fires at an area on its own (W10): a crew shoots at the enemy it can
/// see, not at an unknown in a treeline. The hold ends when that enemy dies,
/// leaves reach or drops out of sight.
fn enemy_in_reach(
    ctx: &FireContext,
    unit: &Unit,
    units: &[Unit],
    mount: &Mount,
    spec: &MountSpec,
) -> bool {
    // Only kinds that can be brought to bear on the enemy's height band hold
    // the mount: a helicopter overhead does not stop a tank shelling a treeline.
    let reach = |layer| {
        reach(
            ctx.arsenal,
            layer,
            (0..spec.kinds.len())
                .filter(|&k| mount.has_rounds(k))
                .map(|k| spec.kinds[k]),
        )
    };
    ctx.knowledge[unit.side.index()]
        .identified_now(ctx.tick)
        .any(|(u, t)| {
            body_distance(t.position, unit.position) <= reach(units[u.0 as usize].layer())
                && permitted(ctx, unit, Target::Unit(u))
        })
}

/// A known platform can threaten us from its observed pose, independent of
/// its private ammunition, cycle, orders or current weapon choice. This asks
/// physical shot geometry, not whether the opposing side would choose to fire.
pub(super) fn return_fire_threat(
    ctx: &FireContext,
    shooter: &Unit,
    kind: TypeIndex,
    track: &crate::knowledge::Track,
) -> bool {
    let target = shooter.position + v3(0.0, 0.0, shooter.aim_height(ctx.rules));
    let delta = target.xy() - track.position.xy();
    let bearing = libm::atan2(delta.y, delta.x);
    let eye = track.position
        + v3(
            0.0,
            0.0,
            ctx.rules.physics.eye_m(ctx.rules.catalog.get(kind)),
        );
    let carrier = if matches!(
        ctx.rules.catalog.get(kind).mobility,
        contract::catalog::Mobility::Air { .. }
    ) {
        v3(track.velocity.x, track.velocity.y, 0.0)
    } else {
        V3::default()
    };
    ctx.arsenal.specs(kind).iter().any(|spec| {
        let origin = match spec.muzzle {
            Some(muzzle) => placed_muzzle(
                track.position,
                track.yaw,
                spec,
                muzzle,
                if spec.on.is_some() || ctx.rules.catalog.get(kind).hull().is_none() {
                    bearing
                } else {
                    track.yaw
                },
                bearing,
            ),
            None => track.position + v3(0.0, 0.0, ctx.rules.physics.infantry_muzzle_m),
        };
        spec.kinds.iter().any(|&k| {
            let weapon = &ctx.arsenal.weapons[k];
            let distance = (target - origin).length();
            can_damage(&weapon.def, shooter.armor(ctx.rules), ctx.rules)
                && distance >= weapon.def.min_range_m
                && weapon
                    .def
                    .engages(shooter.layer(), Some(shooter.target_class(ctx.rules)))
                    .is_some_and(|fire| distance <= fire.range_m)
                && (weapon.profile.turn_rad_s.is_none()
                    || (ctx.world.sight_clear(eye, target)
                        && ctx.world.sight_depth(eye, target)
                            < ctx.rules.sensors.foliage_full_block))
                && crate::flight::solve_launch_past(
                    ctx.world,
                    &ctx.arsenal.config,
                    &weapon.profile,
                    &Aim {
                        origin,
                        target,
                        target_velocity: v3(0.0, 0.0, 0.0),
                        carrier,
                    },
                    None,
                )
                .is_ok()
        })
    })
}
