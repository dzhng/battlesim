use crate::knowledge::SideKnowledge;
use crate::math::V2;
use crate::units::{Unit, UnitOrder};
use crate::weapons::{unit_reach, Arsenal, Target};
use contract::command::Engagement;

use super::pass::PassPlan;

/// A jet's flight task for this tick. This is the only result that turns an
/// order or a reaction into flight in [`super::step_fixed_wing`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AirTask {
    Hold,
    Orbit {
        centre: V2,
    },
    /// Attack `target`: a dogfight against a winged aircraft in its own
    /// layer (`super::dogfightable`), passes at anything else.
    Attack {
        target: Target,
        explicit: bool,
    },
    Withdraw,
}

/// Resolve one jet's order and reaction against the knowledge available to
/// its side. An existing pass owns the current leg until it completes; the
/// caller clears it only after that leg has flown.
pub fn task(
    unit: &Unit,
    knowledge: &SideKnowledge,
    arsenal: &Arsenal,
    pass: Option<&PassPlan>,
) -> AirTask {
    if unit.withdrawing {
        return AirTask::Withdraw;
    }

    if let Some(pass) = pass {
        if let Some(order) = unit.orders.front() {
            match order {
                UnitOrder::Attack { target, .. } => {
                    return AirTask::Attack {
                        target: *target,
                        explicit: true,
                    };
                }
                UnitOrder::AttackMove(_) => {
                    return AirTask::Attack {
                        target: pass.target,
                        explicit: false,
                    };
                }
                _ => {}
            }
        }
    }

    match unit.orders.front() {
        Some(UnitOrder::Move(order)) => AirTask::Orbit {
            centre: order.destination,
        },
        Some(UnitOrder::AttackMove(order)) => {
            if let Some(target) = along_way_target(unit, order.destination, knowledge, arsenal) {
                AirTask::Attack {
                    target,
                    explicit: false,
                }
            } else {
                match reaction(unit, knowledge) {
                    AirTask::Hold => AirTask::Orbit {
                        centre: order.destination,
                    },
                    task => task,
                }
            }
        }
        Some(UnitOrder::Attack { target, .. }) => {
            if target_is_destroyed(*target, knowledge) {
                AirTask::Hold
            } else {
                AirTask::Attack {
                    target: *target,
                    explicit: true,
                }
            }
        }
        Some(UnitOrder::Garrison { .. } | UnitOrder::Exit) => AirTask::Hold,
        None => reaction(unit, knowledge),
    }
}

/// What a jet with no order does on its own: answer an attacker it knows
/// of, or a firing contact, under fire at will; otherwise hold.
pub(crate) fn reaction(unit: &Unit, knowledge: &SideKnowledge) -> AirTask {
    if unit.engagement != Engagement::FireAtWill {
        return AirTask::Hold;
    }
    let attacker = unit.attackers.iter().find_map(|&id| {
        if knowledge.track(id).is_some() {
            return Some(AirTask::Attack {
                target: Target::Unit(id),
                explicit: false,
            });
        }
        knowledge
            .all_contacts()
            .find(|contact| contact.emitter == id)
            .map(|contact| AirTask::Attack {
                target: Target::Contact(contact.id),
                explicit: false,
            })
    });
    attacker
        .or_else(|| {
            knowledge
                .all_contacts()
                .find(|contact| contact.source == contract::observation::ContactSource::Firing)
                .map(|contact| AirTask::Attack {
                    target: Target::Contact(contact.id),
                    explicit: false,
                })
        })
        .unwrap_or(AirTask::Hold)
}

fn target_is_destroyed(target: Target, knowledge: &SideKnowledge) -> bool {
    matches!(target, Target::Unit(id) if knowledge.knows_destroyed(id))
}

/// The known enemy nearest the jet's remaining leg to `goal`, among those
/// its weapons reach from somewhere on the leg: what an attack-move picks
/// up along the way. None once the leg is flown.
fn along_way_target(
    unit: &Unit,
    goal: V2,
    knowledge: &SideKnowledge,
    arsenal: &Arsenal,
) -> Option<Target> {
    let from = unit.position.xy();
    let leg = goal - from;
    let length_sq = leg.dot(leg);
    if length_sq <= 1e-9 {
        return None;
    }
    knowledge
        .tracks()
        .filter_map(|(id, track)| {
            let projection = (track.position.xy() - from).dot(leg) / length_sq;
            let off = (track.position.xy() - (from + leg * projection)).length();
            ((0.0..=1.0).contains(&projection)
                && off <= unit_reach(arsenal, unit.kind, track.layer))
            .then_some((off, id))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)))
        .map(|(_, id)| Target::Unit(id))
}
