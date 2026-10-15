use crate::knowledge::SideKnowledge;
use crate::math::V2;
use crate::units::{Unit, UnitOrder};
use crate::weapons::Target;
use contract::command::Engagement;

use super::pass::PassPlan;

/// The side-scoped knowledge passed to an aircraft order decision.
pub type Knowledge = SideKnowledge;

/// A jet's flight task for this tick. This is the only result that turns an
/// order or a reaction into flight in [`super::step_fixed_wing`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AirTask {
    Hold,
    Orbit { centre: V2 },
    Attack { target: Target, explicit: bool },
    Dogfight,
    Withdraw,
}

/// Resolve one jet's order and reaction against the knowledge available to
/// its side. An existing pass owns the current leg until it completes; the
/// caller clears it only after that leg has flown.
pub fn task(unit: &Unit, knowledge: &Knowledge, pass: Option<&PassPlan>) -> AirTask {
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
                UnitOrder::Move(_) => {
                    return AirTask::Orbit {
                        centre: move_goal(unit),
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
            if let Some(target) = along_way_target(unit, order.destination, knowledge) {
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
            } else if super::target_in_layer(knowledge, *target, unit.layer()) {
                AirTask::Dogfight
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

pub fn reaction_task(unit: &Unit, knowledge: &Knowledge) -> AirTask {
    reaction(unit, knowledge)
}

fn reaction(unit: &Unit, knowledge: &Knowledge) -> AirTask {
    if unit.engagement != Engagement::FireAtWill {
        return AirTask::Hold;
    }
    let attacker = unit.attackers.iter().find_map(|&id| {
        if let Some(track) = knowledge.track(id) {
            if track.position.z > unit.position.z * 0.5 {
                return Some(AirTask::Dogfight);
            }
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

fn move_goal(unit: &Unit) -> V2 {
    unit.orders
        .front()
        .and_then(UnitOrder::movement)
        .map_or(unit.position.xy(), |order| order.destination)
}

fn target_is_destroyed(target: Target, knowledge: &Knowledge) -> bool {
    matches!(target, Target::Unit(id) if knowledge.knows_destroyed(id))
}

fn along_way_target(unit: &Unit, goal: V2, knowledge: &Knowledge) -> Option<Target> {
    let from = unit.position.xy();
    let leg = goal - from;
    let length_sq = leg.dot(leg);
    knowledge
        .tracks()
        .filter_map(|(id, track)| {
            let projection = if length_sq > 1e-9 {
                (track.position.xy() - from).dot(leg) / length_sq
            } else {
                0.0
            };
            (0.0..=1.0).contains(&projection).then_some((
                (track.position.xy() - (from + leg * projection)).length(),
                id,
            ))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)))
        .map(|(_, id)| Target::Unit(id))
}
