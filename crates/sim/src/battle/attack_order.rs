//! When an attack order ends: an attack pursues what it cannot fire on and
//! completes once its target is dead, lost or reached unseen.

use super::*;

/// An attack reaching its target's last reported place within this distance,
/// without regaining sight, is complete.
const PURSUIT_ARRIVAL_M: f64 = 5.0;

impl Battle {
    /// What an attack does as it takes effect: it switches its unit to fire
    /// at will (W14), and a jet it sends after an enemy jet starts the
    /// dogfight.
    pub(super) fn take_effect(unit: &mut Unit, dogfight: Option<UnitId>) {
        controllers::fire_at_will(unit);
        if let Some(target) = dogfight {
            crate::air::dogfight::set_target(unit, target);
        }
    }

    /// Each attack that waited behind other orders takes effect now that its
    /// turn has come: before movement, and again before the fire step for
    /// one whose turn came during movement.
    pub(super) fn start_waiting_attacks(&mut self) {
        let due: Vec<(usize, Option<UnitId>)> = (self.units.iter().enumerate())
            .filter_map(|(i, unit)| match unit.orders.front() {
                Some(UnitOrder::Attack {
                    target,
                    waiting: true,
                    ..
                }) => Some((i, dogfight_with(unit, *target, &self.units))),
                _ => None,
            })
            .collect();
        for (i, dogfight) in due {
            let unit = &mut self.units[i];
            if let Some(UnitOrder::Attack { waiting, .. }) = unit.orders.front_mut() {
                *waiting = false;
            }
            Self::take_effect(unit, dogfight);
        }
    }

    /// Each attack with an order queued behind it counts this tick's fire
    /// against its target, and hands off once its volley is done
    /// ([`crate::volley`]): measured after the fire step's launches, so the
    /// rounds of the cycle or run that ends it have left and a guided
    /// round's launcher is known to be supporting it. Handing off, it drops
    /// the attack's chase and dogfight; an attack left with nothing behind it
    /// drops its count.
    pub(super) fn hand_off_volleys(&mut self, shots: &[VolleyShot]) {
        let knowledge = &self.knowledge;
        for unit in &mut self.units {
            let queued = unit.orders.len() > 1;
            let (reach, garrisoned, layer) = (unit.reach, unit.garrisoned(), unit.layer());
            let pass = unit.air.as_ref().and_then(|air| air.pass);
            let (fixed_wing, on_field, id) = (unit.fixed_wing(), unit.on_field(), unit.id);
            let guiding_at = |target: Target| {
                (unit.mounts.iter().enumerate())
                    .filter(|(_, m)| m.support.as_ref().is_some_and(|s| s.target == target))
                    .fold(0u64, |rows, (k, _)| rows | 1 << k)
            };
            let Some(UnitOrder::Attack {
                target,
                waiting: false,
                volley,
                ..
            }) = unit.orders.front()
            else {
                continue;
            };
            let target = *target;
            if !queued || !on_field {
                if volley.is_some() && !queued {
                    if let Some(UnitOrder::Attack { volley, .. }) = unit.orders.front_mut() {
                        *volley = None;
                    }
                }
                continue;
            }
            let guiding = guiding_at(target);
            let at_target = |s: &&VolleyShot| s.unit == id && s.target == target;
            let mut progress = volley.unwrap_or_default();
            // A jet attacks anything but an enemy jet in runs.
            let done = if fixed_wing
                && !crate::air::dogfightable(&knowledge[unit.side.index()], target, layer)
            {
                let hurts = matches!(target, Target::Ground(_)) || reach.can_damage;
                progress.observe_run(crate::volley::RunFire {
                    fired: shots.iter().any(|s| at_target(&s)),
                    diving: pass.is_some_and(|p| p.phase == crate::air::pass::PassPhase::Dive),
                    guiding,
                    can_act: hurts && pass.is_some(),
                })
            } else {
                let working = reach.ordered_working;
                progress.observe(crate::volley::VolleyFire {
                    working,
                    ended: (shots.iter().filter(at_target))
                        .filter(|s| s.cycle_ended)
                        .fold(0, |rows, s| rows | 1 << s.mount),
                    guiding,
                    can_act: match target {
                        Target::Ground(_) => working != 0,
                        _ => reach.can_damage && (working != 0 || !garrisoned),
                    },
                })
            };
            if done {
                unit.orders.pop_front();
                unit.pursuit = None;
                crate::air::dogfight::clear(unit);
            } else if let Some(UnitOrder::Attack { volley, .. }) = unit.orders.front_mut() {
                *volley = Some(progress);
            }
        }
    }

    /// An attack pursues an identified target it cannot fire on from here, or
    /// its last reported place once identification lapses (W17); it ends on
    /// arriving there unseen, or when its area expires unidentified. A
    /// garrisoned attack fires from the building and never walks out, so it
    /// ends as soon as its target is lost.
    pub(super) fn update_pursuit(&mut self) {
        self.start_waiting_attacks();
        for unit in &mut self.units {
            unit.pursuit = None;
            // Fixed-wing attacks are planned as passes by `air::step_fixed_wing`;
            // they never enter the generic ground-style hover pursuit.
            if unit.fixed_wing() {
                continue;
            }
            let knowledge = &self.knowledge[unit.side.index()];
            let garrisoned = unit.garrisoned();
            let Some(UnitOrder::Attack {
                target, last_known, ..
            }) = unit.orders.front_mut()
            else {
                continue;
            };
            let out_of_reach = unit.reach.needs_closer;
            let done = match *target {
                // A death the side watched completes the attack.
                Target::Unit(u) if knowledge.knows_destroyed(u) => true,
                Target::Unit(u) if garrisoned => knowledge.track(u).is_none(),
                Target::Unit(u) => match knowledge.track(u) {
                    Some(track) => {
                        *last_known = Some(track.position.xy());
                        if out_of_reach {
                            unit.pursuit = Some(Pursuit {
                                to: track.position.xy(),
                                searching: false,
                            });
                        }
                        false
                    }
                    None => match *last_known {
                        Some(p) if (unit.position.xy() - p).length() > PURSUIT_ARRIVAL_M => {
                            unit.pursuit = Some(Pursuit {
                                to: p,
                                searching: true,
                            });
                            false
                        }
                        _ => true,
                    },
                },
                Target::Contact(c) => knowledge.ground_contact(c).is_none(),
                Target::Ground(_) => false,
                Target::Round(_) => true,
            };
            if done {
                unit.orders.pop_front();
            }
        }
    }
}

/// The enemy jet an attack on `target` sends `unit` after in a dogfight.
pub(super) fn dogfight_with(unit: &Unit, target: Target, units: &[Unit]) -> Option<UnitId> {
    match target {
        Target::Unit(other)
            if crate::air::dogfight::can_dogfight(unit, &units[other.0 as usize]) =>
        {
            Some(other)
        }
        _ => None,
    }
}

/// What the volley count reads of one shot, kept past the fire step's
/// launches.
pub(super) struct VolleyShot {
    unit: UnitId,
    mount: usize,
    target: Target,
    cycle_ended: bool,
}

impl VolleyShot {
    pub(super) fn of(shot: &weapons::Shot) -> Self {
        VolleyShot {
            unit: shot.unit,
            mount: shot.mount,
            target: shot.target,
            cycle_ended: shot.cycle_ended,
        }
    }
}
