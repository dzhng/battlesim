use contract::catalog::AltitudeLayer;
use contract::ids::UnitId;
use contract::scenario::Rules;

use crate::knowledge::SideKnowledge;
use crate::math::body_distance;
use crate::units::{Unit, UnitOrder};

use super::steer::Intent;

/// Record an aircraft firing on another aircraft. The shooter always keeps
/// the link; the target adopts it only while free, so a close fight cannot be
/// stolen by every later burst.
pub fn on_fire(shooter: &mut Unit, target: &mut Unit, rules: &Rules) {
    if !shooter.airborne()
        || !target.airborne()
        || shooter.layer() != AltitudeLayer::HighAir
        || target.layer() != AltitudeLayer::HighAir
        || breaks(shooter, target, rules)
        || has_plain_move(shooter)
    {
        return;
    }
    if let Some(air) = shooter.air.as_mut() {
        air.dogfight_target = Some(target.id);
    }
    if target
        .air
        .as_ref()
        .is_some_and(|air| air.dogfight_target.is_none())
        && !has_plain_move(target)
    {
        target.air.as_mut().expect("airborne state").dogfight_target = Some(shooter.id);
    }
}

/// Install an explicit close-fight target from an attack order.
pub fn set_target(unit: &mut Unit, target: UnitId) {
    if let Some(air) = unit.air.as_mut() {
        air.dogfight_target = Some(target);
    }
}

pub fn clear_target(unit: &mut Unit) {
    if let Some(air) = unit.air.as_mut() {
        air.dogfight_target = None;
    }
}

/// Give a holding aircraft a close-fight link when its firing report names an
/// airborne attacker that was only retained as a contact.
pub fn reaction_target(unit: &Unit, knowledge: &SideKnowledge) -> Option<UnitId> {
    unit.attackers
        .iter()
        .find_map(|&id| {
            knowledge
                .track(id)
                .filter(|track| track.position.z > unit.position.z * 0.5)
                .map(|_| id)
                .or_else(|| {
                    knowledge
                        .all_contacts()
                        .find(|contact| {
                            contact.emitter == id && contact.layer != AltitudeLayer::Ground
                        })
                        .map(|_| id)
                })
        })
        .or_else(|| {
            knowledge
                .all_contacts()
                .find(|contact| {
                    contact.source == contract::observation::ContactSource::Firing
                        && contact.layer != AltitudeLayer::Ground
                })
                .map(|contact| contact.emitter)
        })
}

/// Drop a stale close-fight link. This is the sole writer that clears the
/// link; firing and order admission delegate linking here as well.
pub fn maintain(unit: &mut Unit, knowledge: &SideKnowledge, rules: &Rules) {
    let Some(target) = unit.air.as_ref().and_then(|air| air.dogfight_target) else {
        return;
    };
    let Some(track) = knowledge.track(target) else {
        unit.air.as_mut().expect("airborne state").dogfight_target = None;
        return;
    };
    if body_distance(unit.position, track.position) > rules.air.jet.dogfight_break_m {
        unit.air.as_mut().expect("airborne state").dogfight_target = None;
    }
}

/// Steer at the target's last reported pose with a short lag, retaining an
/// organic pursuit curve instead of snapping to the current point.
pub fn intent(unit: &Unit, knowledge: &SideKnowledge) -> Option<Intent> {
    let target = unit.air.as_ref()?.dogfight_target?;
    let track = knowledge.track(target)?;
    Some(Intent::Pursue {
        point: track.position,
        vel: crate::math::v3(track.velocity.x, track.velocity.y, 0.0),
        lag_s: 2.0,
    })
}

pub fn breaks(a: &Unit, b: &Unit, rules: &Rules) -> bool {
    body_distance(a.position, b.position) > rules.air.jet.dogfight_break_m
}

fn has_plain_move(unit: &Unit) -> bool {
    unit.orders
        .iter()
        .any(|order| matches!(order, UnitOrder::Move(_)))
}
