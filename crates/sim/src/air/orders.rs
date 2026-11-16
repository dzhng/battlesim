use crate::knowledge::SideKnowledge;
use crate::math::V2;
use crate::units::{Unit, UnitOrder};
use crate::weapons::{Arsenal, Target};
use contract::command::Engagement;

use super::pass::{PassPhase, PassPlan};

/// A jet's flight task for this tick. This is the only result that turns an
/// order into flight in [`super::step_fixed_wing`].
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
    },
    Withdraw,
}

/// Resolve one jet's order against the knowledge available to its side.
/// Only an attack or an attack-move turns a jet from its circle: moved, or
/// with no order, it circles and only its weapons answer what they bear on.
/// An attack-move never ends on arrival: the jet circles its goal hunting,
/// picking up what its weapons reach from its leg and answering fire, and
/// flies back to the goal after each attack; a committed run finishes
/// before it picks anew (D18).
pub fn task(
    unit: &Unit,
    knowledge: &SideKnowledge,
    arsenal: &Arsenal,
    pass: Option<&PassPlan>,
) -> AirTask {
    if unit.withdrawing {
        return AirTask::Withdraw;
    }
    match unit.orders.front() {
        Some(UnitOrder::Move(order)) => AirTask::Orbit {
            centre: order.destination,
        },
        Some(UnitOrder::AttackMove(order)) => {
            // A run at a target its side has lost or seen die is over.
            if let Some(pass) = pass.filter(|pass| {
                pass.phase == PassPhase::Dive
                    && super::knows_target(knowledge, pass.target)
                    && !target_is_destroyed(pass.target, knowledge)
            }) {
                return AirTask::Attack {
                    target: pass.target,
                };
            }
            along_way_target(unit, order.destination, knowledge, arsenal)
                .or_else(|| answer(unit, knowledge))
                .map_or(
                    AirTask::Orbit {
                        centre: order.destination,
                    },
                    |target| AirTask::Attack { target },
                )
        }
        Some(UnitOrder::Attack { target, .. }) => {
            // Dead, or lost to its side: it has nothing left to attack.
            if target_is_destroyed(*target, knowledge) || !super::knows_target(knowledge, *target) {
                AirTask::Hold
            } else {
                AirTask::Attack { target: *target }
            }
        }
        Some(
            UnitOrder::Garrison { .. }
            | UnitOrder::Exit
            | UnitOrder::Board { .. }
            | UnitOrder::Collect { .. }
            | UnitOrder::Unload { .. }
            | UnitOrder::Withdraw,
        )
        | None => AirTask::Hold,
    }
}

/// What an attack-moving jet answers under fire at will: an attacker it
/// knows of, else a firing contact.
fn answer(unit: &Unit, knowledge: &SideKnowledge) -> Option<Target> {
    if unit.engagement != Engagement::FireAtWill {
        return None;
    }
    unit.attackers
        .iter()
        .find_map(|&id| {
            if knowledge.track(id).is_some() {
                return Some(Target::Unit(id));
            }
            knowledge
                .all_contacts()
                .find(|contact| contact.emitter == id)
                .map(|contact| Target::Contact(contact.id))
        })
        .or_else(|| {
            knowledge
                .all_contacts()
                .find(|contact| contact.source == contract::observation::ContactSource::Firing)
                .map(|contact| Target::Contact(contact.id))
        })
}

fn target_is_destroyed(target: Target, knowledge: &SideKnowledge) -> bool {
    matches!(target, Target::Unit(id) if knowledge.knows_destroyed(id))
}

/// The known enemy nearest the jet's remaining leg to `goal`, among those
/// a row it can attack them with (`super::engaging_rows`) reaches from
/// somewhere on the leg, its ends included: what an attack-move picks up
/// along the way, and round its goal once there.
fn along_way_target(
    unit: &Unit,
    goal: V2,
    knowledge: &SideKnowledge,
    arsenal: &Arsenal,
) -> Option<Target> {
    let from = unit.position.xy();
    let leg = goal - from;
    let length_sq = leg.dot(leg);
    knowledge
        .tracks()
        .filter_map(|(id, track)| {
            let projection = if length_sq <= 1e-9 {
                0.0
            } else {
                ((track.position.xy() - from).dot(leg) / length_sq).clamp(0.0, 1.0)
            };
            let off = (track.position.xy() - (from + leg * projection)).length();
            super::engaging_rows(arsenal, unit, track.layer, Some(track.class))
                .any(|(_, fire)| off <= fire.range_m)
                .then_some((off, id))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)))
        .map(|(_, id)| Target::Unit(id))
}
