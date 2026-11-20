//! A mount's tick: each mount aims, cycles and fires, and each soldier
//! takes up the weapon he will use.

use super::*;

/// A turret returns to the hull's front after this long without a currently
/// observed enemy to hold its attention.
const IDLE_TURRET_RESET_S: f64 = 15.0;

/// Candidate targeting, assessed before any soldier works a gun this tick.
pub(super) struct MountPlan {
    pub(super) stationary: bool,
    pub(super) idle_reason: ActionReason,
    pub(super) resolved: Option<Resolved>,
    pub(super) assessment: Option<Result<Ready, ActionReason>>,
    pub(super) kind: Option<usize>,
}

pub(super) fn preferred_mount(
    unit: &Unit,
    member: usize,
    specs: &[MountSpec],
    weapons: &[Weapon],
    rules: &Rules,
    plans: &[Option<MountPlan>],
    moving: bool,
) -> Option<usize> {
    let eligible = |m: usize| {
        candidates(unit, &unit.mounts[m], &specs[unit.mounts[m].spec]).any(|k| k == member)
    };
    let available =
        |m: usize| plans[m].as_ref().is_some_and(|p| !(moving && p.stationary)) && eligible(m);
    let useful = |m: usize| {
        plans[m].as_ref().is_some_and(|p| {
            p.assessment.is_some_and(|a| {
                a.is_ok_and(|ready| {
                    p.resolved.is_some_and(|r| {
                        r.worth(
                            &weapons[specs[unit.mounts[m].spec].kinds[ready.kind]].def,
                            rules,
                        )
                    })
                })
            })
        })
    };
    let assigned = |m: usize| !specs[unit.mounts[m].spec].squad && available(m);
    let default = |m: usize| {
        available(m)
            && specs[unit.mounts[m].spec]
                .kinds
                .iter()
                .any(|&k| weapons[k].def.default)
    };
    // A gun already shouldered stays up while it still tracks a target worth
    // its round and only the shot's line is in question (the target out of
    // sight a moment, the way blocked, no sight of his own, a friend in
    // front): the aim it has
    // built would be lost putting it down. It goes down once the target is
    // gone, out of reach or not worth it.
    let shouldered = |m: usize| {
        let spec = &specs[unit.mounts[m].spec];
        assigned(m)
            && unit.members[member].active_mount == Some(unit.mounts[m].spec)
            && plans[m].as_ref().is_some_and(|p| {
                matches!(
                    p.assessment,
                    Some(Err(ActionReason::TrackingLastSighting
                        | ActionReason::BlockedTrajectory
                        | ActionReason::NoOwnSight
                        | ActionReason::FriendlyInLine))
                ) && p
                    .resolved
                    .is_some_and(|r| spec.kinds.iter().any(|&k| r.worth(&weapons[k].def, rules)))
            })
    };
    let mounts = || 0..unit.mounts.len();
    mounts()
        .find(|&m| assigned(m) && unit.mounts[m].support.is_some())
        .or_else(|| mounts().find(|&m| assigned(m) && useful(m)))
        .or_else(|| mounts().find(|&m| shouldered(m)))
        .or_else(|| {
            mounts().find(|&m| {
                default(m)
                    && plans[m]
                        .as_ref()
                        .is_some_and(|p| p.assessment.is_some_and(|a| a.is_ok()))
            })
        })
        .or_else(|| {
            mounts().find(|&m| {
                assigned(m)
                    && unit.mounts[m].cycles.iter().any(|c| {
                        c.needs_reload(weapons, &unit.mounts[m].ammo, &specs[unit.mounts[m].spec])
                    })
            })
        })
        .or_else(|| mounts().find(|&m| default(m)))
}

fn select_active(
    unit: &mut Unit,
    specs: &[MountSpec],
    weapons: &[Weapon],
    rules: &Rules,
    plans: &[Option<MountPlan>],
    moving: &[bool],
) {
    if unit.hull.is_some() {
        return;
    }
    // A carried launcher remains packed until its deployment/aiming setup is
    // complete.  The deployment owner is the single timer for both the
    // visual carried/active pose and weapon readiness; selecting a mount
    // early would make the model appear unpacked while the unit still cannot
    // fire.
    if unit.deployment.is_some() && !deployment::fully_deployed(unit) {
        for member in &mut unit.members {
            member.active_mount = None;
        }
        return;
    }
    for (member, &moving) in moving.iter().enumerate() {
        if !unit.members[member].alive() {
            unit.members[member].active_mount = None;
            continue;
        }
        let choice = preferred_mount(unit, member, specs, weapons, rules, plans, moving);
        let next = choice.map(|m| unit.mounts[m].spec);
        if unit.members[member].active_mount != next {
            if let Some(m) = choice {
                let id = unit.members[member].id;
                let squad = specs[unit.mounts[m].spec].squad;
                for c in &mut unit.mounts[m].cycles {
                    if !squad || c.owner == Some(id) {
                        c.aim = 0.0;
                        c.started = false;
                    }
                }
            }
            unit.members[member].active_mount = next;
        }
    }
}

/// What changed place on the map this tick: each unit (a squad by its
/// centre), and each soldier by his id.
pub struct Moved {
    pub units: Vec<bool>,
    pub soldiers: std::collections::BTreeSet<u32>,
}

impl Moved {
    /// The body a weapon on `unit` (index `i`) rides moved: the soldier
    /// working it, for a squad weapon one soldier works; the unit itself
    /// otherwise. A gunner standing still fires from rest while a teammate
    /// crawls to cover.
    pub fn carrier(&self, i: usize, unit: &Unit, mount: &Mount) -> bool {
        match mount.operator {
            Some(id) if unit.hull.is_none() => self.soldiers.contains(&id),
            _ => self.units[i],
        }
    }

    /// The soldier moved, or for a vehicle the hull.
    pub fn member(&self, i: usize, unit: &Unit, member: usize) -> bool {
        match unit.hull {
            None => self.soldiers.contains(&unit.members[member].id),
            Some(_) => self.units[i],
        }
    }
}

/// Advance every mount one tick; returns the shots fired.
pub fn advance(ctx: &FireContext, units: &mut [Unit], moved: &Moved, rng: &mut Rng) -> Vec<Shot> {
    let dt = 1.0 / ctx.rules.tick_hz as f64;
    let turret_rate = ctx.rules.movement.turret_turn_deg_s.to_radians() * dt;
    let tolerance = ctx.rules.movement.bearing_tolerance_deg.to_radians();
    let mut shots = Vec::new();
    let mut tally = ctx.committed.clone();
    for i in 0..units.len() {
        if !units[i].on_field() {
            continue;
        }
        let specs = ctx.arsenal.specs(units[i].kind);
        // Entering or leaving a building suspends every weapon (contracts).
        if units[i]
            .hold
            .as_ref()
            .is_some_and(|g| g.phase != crate::hold::Phase::Inside)
        {
            for mount in &mut units[i].mounts {
                mount.reason = ActionReason::ChangingPosition;
                if let Some(lock) = mount.lock.as_mut() {
                    lock.engaging = false;
                }
            }
            units[i].reach = Reach::default();
            continue;
        }
        let ordered = units[i].attack_target();
        let mut can_engage = false;
        // Where a gun fixed in the hull needs the body to face.
        let mut face = None;
        let mut leaned: Vec<u32> = Vec::new();
        // Ordered target: some compatible mount can shoot it (or strike its
        // ground aim from here) / none can reach it.
        let (mut ordered_ok, mut ordered_far) = (false, false);
        let (mut ordered_working, mut can_damage) = (0u64, false);
        let mut plans = Vec::with_capacity(units[i].mounts.len());
        for m in 0..units[i].mounts.len() {
            let ctx = &FireContext {
                committed: &tally,
                ..*ctx
            };
            let unit = &units[i];
            let spec = &specs[unit.mounts[m].spec];
            let mut mount = unit.mounts[m].clone();
            let stationary = spec
                .kinds
                .iter()
                .any(|&k| ctx.arsenal.weapons[k].def.stationary);
            if spec.squad {
                mount.cycles.retain(|c| {
                    unit.members
                        .iter()
                        .enumerate()
                        .any(|(k, s)| carries(unit, spec, k) && c.owner == Some(s.id))
                });
                for k in candidates(unit, &mount, spec) {
                    let owner = Some(unit.members[k].id);
                    if !mount.cycles.iter().any(|c| c.owner == owner) {
                        mount
                            .cycles
                            .push(Cycle::new(owner, spec, &ctx.arsenal.weapons));
                    }
                }
            }
            // Without an operator a transferable gun remains a spare.
            if unit.hull.is_none() && candidates(unit, &mount, spec).next().is_none() {
                // A recoverable spare keeps its magazine and reload progress.
                // Moving a stationary gun still interrupts its reload.
                if spec.special && !(stationary && moved.units[i]) {
                    mount.lock = None;
                    mount.support = None;
                    for c in &mut mount.cycles {
                        c.pause(dt);
                    }
                } else {
                    mount.stop();
                }
                mount.reason = if mount.out_of_ammo() {
                    ActionReason::OutOfAmmo
                } else {
                    ActionReason::NoCompatibleTarget
                };
                units[i].mounts[m] = mount;
                plans.push(None);
                continue;
            }
            let explicit = ordered.filter(|&t| compatible(ctx, &mount, spec, t, units));
            can_damage |= explicit.is_some();
            let previous = mount.lock.as_ref().map(|l| l.target);
            let mut assessed = Assessed::default();
            let LockChoice {
                idle_reason,
                ordered: ordered_shot,
            } = choose_lock(ctx, unit, units, &mut mount, spec, explicit, &mut assessed);
            // A mount that reaches a ground aim strikes it from here or not
            // at all: a crew under cover holds rather than driving at the
            // point (launch clearance).
            ordered_ok |= match ordered_shot {
                Some(Ok(_)) => true,
                Some(Err(ActionReason::BlockedTrajectory)) => {
                    matches!(ordered, Some(Target::Ground(_)))
                }
                _ => false,
            };
            ordered_far |= matches!(
                ordered_shot,
                Some(Err(
                    ActionReason::OutOfRange | ActionReason::BlockedTrajectory
                ))
            );
            let mut resolved = mount
                .lock
                .as_ref()
                .and_then(|l| resolve_for(ctx, unit, &mount, spec, l.target, units));
            if resolved.is_none() {
                mount.lock = None;
            }
            let current_enemy =
                mount
                    .lock
                    .as_ref()
                    .zip(resolved.as_ref())
                    .is_some_and(|(lock, r)| {
                        r.current && matches!(lock.target, Target::Unit(_) | Target::Contact(_))
                    });
            if current_enemy {
                mount.last_enemy_lock_tick = Some(ctx.tick);
            }
            let reset_ticks = (IDLE_TURRET_RESET_S * ctx.rules.tick_hz as f64).round() as Tick;
            // Ground it was ordered onto, or a round it is tracking, keeps a
            // turret busy without any enemy in sight.
            let idle_turret = spec.turret
                && mount.support.is_none()
                && !current_enemy
                && !mount.lock.as_ref().is_some_and(|lock| {
                    matches!(lock.target, Target::Ground(_) | Target::Round(_))
                })
                && mount
                    .last_enemy_lock_tick
                    .is_some_and(|last| ctx.tick.saturating_sub(last) >= reset_ticks);
            if idle_turret {
                mount.lock = None;
                resolved = None;
                mount.bearing = turned_mount(&mount, spec, unit.yaw, unit.yaw, turret_rate);
            }
            // Could shoot now, from here, and to effect?
            let assessment = mount.lock.as_ref().zip(resolved.as_ref()).map(|(l, r)| {
                let a = if r.current {
                    assessed.shot(ctx, unit, units, &mount, spec, l.target, r)
                } else {
                    Err(ActionReason::TrackingLastSighting)
                };
                if let Ok(ready) = a {
                    can_engage |=
                        r.worth(&ctx.arsenal.weapons[spec.kinds[ready.kind]].def, ctx.rules);
                }
                a
            });

            if previous != mount.lock.as_ref().map(|l| l.target) {
                for c in &mut mount.cycles {
                    c.aim = 0.0;
                }
            }
            let kind = resolved
                .as_ref()
                .and_then(|r| preferred_kind(ctx, &mount, spec, r));
            // A child mount's muzzle is assessed on its parent's current
            // bearing, as vehicle mounts act independently in mount order.
            if unit.hull.is_some() && !(stationary && moved.units[i]) {
                if let Some(r) = &resolved {
                    point_mount(unit, &mut mount, spec, r.point, turret_rate);
                }
            }
            if let Some(Ok(ready)) = &assessment {
                mount.elevation = libm::atan2(
                    ready.velocity.z,
                    libm::hypot(ready.velocity.x, ready.velocity.y),
                );
            }
            plans.push(Some(MountPlan {
                stationary,
                idle_reason,
                resolved,
                assessment,
                kind,
            }));
            units[i].mounts[m] = mount;
        }
        let moving: Vec<bool> = (0..units[i].members.len())
            .map(|k| moved.member(i, &units[i], k))
            .collect();
        select_active(
            &mut units[i],
            specs,
            &ctx.arsenal.weapons,
            ctx.rules,
            &plans,
            &moving,
        );
        for (m, plan) in plans.into_iter().enumerate() {
            let Some(MountPlan {
                stationary,
                idle_reason,
                resolved,
                assessment,
                kind: kind_for_target,
            }) = plan
            else {
                continue;
            };
            let ctx = &FireContext {
                committed: &tally,
                ..*ctx
            };
            let mut launch_hits = Vec::new();
            let unit = &units[i];
            let spec = &specs[unit.mounts[m].spec];
            let mut mount = unit.mounts[m].clone();

            // Movement clears a stationary weapon's aim and unfinished reload (W03).
            if stationary && moved.carrier(i, unit, &mount) {
                if let Some(lock) = mount.lock.as_mut() {
                    lock.engaging = false;
                }
                for c in &mut mount.cycles {
                    c.pause(if unit.hull.is_none() { dt } else { 0.0 });
                    c.reload = None;
                }
                mount.reason = ActionReason::MovingStationaryWeapon;
                units[i].mounts[m] = mount;
                continue;
            }

            // Suppression's tier slows the cycle without resetting its progress (P14).
            let rate = 1.0
                - ctx
                    .rules
                    .suppression
                    .penalties(unit.suppression)
                    .map_or(0.0, |t| t.reload_cycle_penalty);
            for cycle in &mut mount.cycles {
                if !cycle_active(unit, &unit.mounts[m], cycle) {
                    cycle.pause(dt * rate);
                    continue;
                }
                // Physical aim continues toward a retained last sighting.
                if let Some(k) = kind_for_target {
                    let aim_s = ctx.arsenal.weapons[spec.kinds[k]].def.aim_s;
                    cycle.aim = (cycle.aim + dt).min(aim_s);
                }
                cycle.advance(
                    &ctx.arsenal.weapons,
                    &mount.ammo,
                    spec,
                    kind_for_target,
                    assessment.is_some_and(|a| a.is_ok()),
                    dt * rate,
                );
            }
            let active = |c: &Cycle| cycle_active(unit, &unit.mounts[m], c);

            if unit.hull.is_none() {
                if let Some(r) = &resolved {
                    point_mount(unit, &mut mount, spec, r.point, turret_rate);
                }
            }

            // A gun fixed in the hull keeps the body on its target through
            // every aim, burst and reload, not only while it turns onto it.
            // A turret whose arc can't reach its target turns the body to
            // bring the arc's middle onto it, unless a gun with the same
            // weapons already bears on it. A winged aircraft's flight alone
            // points its airframe.
            if let (Some(_), Some(r), Some(Ok(_))) = (&mount.lock, &resolved, assessment) {
                let bearing = bearing_from(unit, r.point);
                if hull_fixed(unit, spec) && !unit.fixed_wing() {
                    face.get_or_insert(bearing);
                } else if let Some(arc) = spec.arc {
                    let bears = |s: &MountSpec| s.arc.is_none_or(|a| a.holds(unit.yaw, bearing));
                    let covered = unit
                        .mounts
                        .iter()
                        .map(|other| &specs[other.spec])
                        .any(|s| s.kinds == spec.kinds && bears(s));
                    if !covered {
                        face.get_or_insert(bearing - arc.centre);
                    }
                }
            }
            // Fire when everything lines up.
            let mut engaging = false;
            mount.reason = match (&mount.lock, &resolved, assessment) {
                (Some(lock), Some(r), Some(Ok(ready))) => {
                    let (target, k) = (lock.target, ready.kind);
                    let overkill_hold = Some(target) != ordered
                        && overkill::overkill(ctx, units, unit.side, spec, &ready, target);
                    engaging = mount.cycles.iter().any(active);
                    if unit.hull.is_some()
                        && unit
                            .deployment
                            .as_ref()
                            .is_some_and(|d| !d.lands() && !d.fully_deployed())
                    {
                        ActionReason::ChangingPosition
                    } else if !engaging {
                        ActionReason::HoldingFire
                    } else if !mount
                        .cycles
                        .iter()
                        .any(|c| active(c) && c.aim >= ctx.arsenal.weapons[spec.kinds[k]].def.aim_s)
                    {
                        ActionReason::Aiming
                    } else if !mount
                        .cycles
                        .iter()
                        .any(|c| active(c) && c.ready() == Some(k))
                    {
                        ActionReason::Reloading
                    } else if (spec.turret || (hull_fixed(unit, spec) && !unit.fixed_wing()))
                        && wrap_angle(bearing_from(unit, r.point) - mount.bearing).abs() > tolerance
                    {
                        ActionReason::TurretTraversing
                    } else if mount.support.is_some() {
                        // One missile guided at a time: the next waits, loaded and aimed.
                        ActionReason::Guiding
                    } else if overkill_hold {
                        // Keep the lock and the engagement cycle alive while
                        // committed fire is enough to finish the target.
                        ActionReason::HoldingFire
                    } else if unit.garrisoned()
                        && !participants(unit, &mount, spec)
                            .any(|k| ctx.windows.faces(unit, k, r.point, ctx.rules, ctx.tick))
                    {
                        // No shooter at a window facing it yet: wait, never
                        // fire through the squad's own shell.
                        engaging = false;
                        if participants(unit, &mount, spec)
                            .any(|k| ctx.windows.changing_window(unit, k, ctx.rules, ctx.tick))
                        {
                            ActionReason::ChangingPosition
                        } else {
                            ActionReason::NoFacingSlot
                        }
                    } else {
                        let moving = moved.carrier(i, unit, &mount);
                        let fired = fire(
                            ctx,
                            unit,
                            units,
                            &mut mount,
                            m,
                            spec,
                            ready,
                            target,
                            r,
                            moving,
                            rng,
                            &mut leaned,
                        );
                        if let Some(shot) = &fired {
                            if let Target::Unit(target) = shot.target {
                                let def = &ctx.arsenal.weapons[shot.weapon].def;
                                let arrive = overkill::arrival_tick(
                                    ctx.tick,
                                    shot.time_of_flight_s,
                                    ctx.rules.tick_hz,
                                );
                                let target_unit = &units[target.0 as usize];
                                launch_hits.extend(shot.launches.iter().filter_map(|launch| {
                                    overkill::counted(
                                        def,
                                        ctx.rules,
                                        target_unit,
                                        launch.origin,
                                        arrive,
                                    )
                                    .map(|hit| (target, hit))
                                }));
                            }
                        }
                        shots.extend(fired);
                        ActionReason::Firing
                    }
                }
                (_, _, Some(Err(e))) => e,
                _ if mount.out_of_ammo() => ActionReason::OutOfAmmo,
                _ => idle_reason,
            };
            if !engaging {
                for c in &mut mount.cycles {
                    c.started = false;
                }
            }
            if let Some(lock) = mount.lock.as_mut() {
                lock.engaging = engaging;
            }
            let working = matches!(
                mount.reason,
                ActionReason::Firing
                    | ActionReason::Aiming
                    | ActionReason::Reloading
                    | ActionReason::TurretTraversing
                    | ActionReason::Guiding
            );
            if working && ordered.is_some() && mount.lock.as_ref().map(|l| l.target) == ordered {
                ordered_working |= 1 << m;
            }
            units[i].mounts[m] = mount;
            let side = units[i].side;
            for (target, hit) in launch_hits {
                tally.add(side, target, hit);
            }
        }
        // Who fired from his lean point stays out on it a while; after a
        // burst out he tucks back in for a spell.
        let ticks = |s: f64| (s * ctx.rules.tick_hz as f64).round() as u64;
        let c = &ctx.rules.cover;
        for s in units[i].members.iter_mut() {
            if !leaned.contains(&s.id) {
                continue;
            }
            if ctx.tick >= s.leaning_until {
                // A new stretch out: a burst at most, then the spell in.
                s.lean_since = ctx.tick;
                s.tucked_until = ctx.tick + ticks(c.lean_burst_s) + ticks(c.lean_tuck_s);
            }
            s.leaning_until =
                (ctx.tick + ticks(c.lean_hold_s)).min(s.lean_since + ticks(c.lean_burst_s));
        }
        units[i].reach = Reach {
            can_engage,
            needs_closer: ordered_far && !ordered_ok,
            face,
            ordered_working,
            can_damage,
        };
    }
    shots
}
