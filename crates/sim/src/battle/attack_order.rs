//! When an attack order ends: an attack pursues what it cannot fire on and
//! completes once its target is dead, lost or reached unseen.

use super::*;

/// An attack reaching its target's last reported place within this distance,
/// without regaining sight, is complete.
const PURSUIT_ARRIVAL_M: f64 = 5.0;

impl Battle {
    /// An attack pursues an identified target it cannot fire on from here, or
    /// its last reported place once identification lapses (W17); it ends on
    /// arriving there unseen, or when its area expires unidentified.
    pub(super) fn update_pursuit(&mut self) {
        for unit in &mut self.units {
            unit.pursuit = None;
            // A garrisoned attack fires from the building; it never walks out.
            if unit.garrisoned() {
                continue;
            }
            // Fixed-wing attacks are planned as passes by `air::step_fixed_wing`;
            // they never enter the generic ground-style hover pursuit.
            if unit.fixed_wing() {
                continue;
            }
            let knowledge = &self.knowledge[unit.side.index()];
            let Some(UnitOrder::Attack { target, last_known }) = unit.orders.front_mut() else {
                continue;
            };
            let out_of_reach = unit.reach.needs_closer;
            let done = match *target {
                // A death the side watched completes the attack.
                Target::Unit(u) if knowledge.knows_destroyed(u) => true,
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
