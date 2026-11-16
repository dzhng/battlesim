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
    /// turn has come.
    fn start_waiting_attacks(&mut self) {
        let due: Vec<(usize, Option<UnitId>)> = (self.units.iter().enumerate())
            .filter_map(|(i, unit)| match unit.orders.front() {
                Some(UnitOrder::Attack {
                    target,
                    waiting: true,
                    ..
                }) => Some((i, *target)),
                _ => None,
            })
            .map(|(i, target)| {
                let dogfight = match target {
                    Target::Unit(other)
                        if crate::air::dogfight::can_dogfight(
                            &self.units[i],
                            &self.units[other.0 as usize],
                        ) =>
                    {
                        Some(other)
                    }
                    _ => None,
                };
                (i, dogfight)
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
    /// ([`crate::volley`]). Measured right after the fire step, so the
    /// rounds of the cycle or run that ends it have already left.
    pub(super) fn hand_off_volleys(&mut self, shots: &[weapons::Shot]) {
        let knowledge = &self.knowledge;
        for unit in &mut self.units {
            if unit.orders.len() < 2 {
                continue;
            }
            let (reach, garrisoned, layer) = (unit.reach, unit.garrisoned(), unit.layer());
            let pass = unit.air.as_ref().and_then(|air| air.pass);
            let fixed_wing = unit.fixed_wing();
            let id = unit.id;
            let Some(UnitOrder::Attack {
                target,
                waiting: false,
                volley,
                ..
            }) = unit.orders.front_mut()
            else {
                continue;
            };
            let target = *target;
            let at_target = |s: &&weapons::Shot| s.unit == id && s.target == target;
            let volley = volley.get_or_insert_default();
            // A jet attacks anything but an enemy jet in runs.
            let done = if fixed_wing
                && !crate::air::dogfightable(&knowledge[unit.side.index()], target, layer)
            {
                let hurts = matches!(target, Target::Ground(_)) || reach.can_damage;
                volley.observe_run(crate::volley::RunFire {
                    fired: shots.iter().any(|s| at_target(&s)),
                    diving: pass.is_some_and(|p| p.phase == crate::air::pass::PassPhase::Dive),
                    can_act: hurts && pass.is_some(),
                })
            } else {
                let working = reach.ordered_firing;
                volley.observe(crate::volley::VolleyFire {
                    working,
                    ended: (shots.iter().filter(at_target))
                        .filter(|s| s.cycle_ended)
                        .fold(0, |rows, s| rows | 1 << s.mount),
                    can_act: match target {
                        Target::Ground(_) => working != 0,
                        _ => reach.can_damage && (working != 0 || !garrisoned),
                    },
                })
            };
            if done {
                unit.orders.pop_front();
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
