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
