//! Boarding and passenger movement for ground carriers.

use contract::ids::UnitId;
use contract::scenario::Rules;

use crate::hold::{Hold, Host, Phase, Place};
use crate::math::V2;
use crate::units::{MoveOrder, Unit, UnitOrder};

fn seats(unit: &Unit, rules: &Rules) -> usize {
    unit.unit_type(rules)
        .capabilities
        .carriage
        .map_or(0, |c| c.seats as usize)
}

fn living(unit: &Unit) -> usize {
    unit.members.iter().filter(|s| s.alive()).count()
}

pub(crate) fn stationary(unit: &Unit) -> bool {
    unit.movement_goal().is_none() && unit.drive_speed_mps.abs() < 1e-6
}

pub(crate) fn within_reach(carrier: &Unit, passenger: &Unit, rules: &Rules) -> bool {
    let hull_reach = carrier.hull.map_or(0.0, |h| h.xy().length());
    (carrier.position.xy() - passenger.position.xy()).length()
        <= rules.carriage.reach_m + hull_reach
}

pub(crate) fn near_and_still(
    units: &[Unit],
    passenger: usize,
    carrier: UnitId,
    rules: &Rules,
) -> bool {
    let Some(host) = units.get(carrier.0 as usize) else {
        return false;
    };
    host.alive()
        && stationary(host)
        && stationary(&units[passenger])
        && within_reach(host, &units[passenger], rules)
}

pub(crate) fn capacity(units: &[Unit], carrier: UnitId, candidate: usize, rules: &Rules) -> bool {
    let used: usize = units
        .iter()
        .filter(|u| {
            matches!(
                u.hold.as_ref().map(|h| h.host),
                Some(Host::Carrier(id)) if id == carrier
            )
        })
        .map(living)
        .sum();
    used.saturating_sub(living(&units[candidate])) + living(&units[candidate])
        <= seats(&units[carrier.0 as usize], rules)
}

pub(crate) fn admit(units: &[Unit], passenger: usize, carrier: UnitId, rules: &Rules) -> bool {
    capacity(units, carrier, passenger, rules)
}

fn push_target_move(unit: &mut Unit, target: V2, gesture: u64) {
    if matches!(unit.orders.front(), Some(UnitOrder::Move(_))) {
        if let Some(UnitOrder::Move(order)) = unit.orders.front_mut() {
            order.destination = target;
        }
        return;
    }
    let Some(current) = unit.orders.pop_front() else {
        return;
    };
    unit.orders.push_front(current);
    unit.orders.push_front(UnitOrder::Move(MoveOrder {
        destination: target,
        policy: contract::command::RoutePolicy::Shortest,
        gesture,
        direction: Default::default(),
        facing: None,
        short: None,
    }));
}

pub(crate) fn copy_passenger_positions(units: &mut [Unit]) {
    for i in 0..units.len() {
        let Some(carrier) = units[i].hold.as_ref().and_then(|h| h.host.carrier()) else {
            continue;
        };
        let Some((host_position, yaw)) = units.get(carrier.0 as usize).map(|h| (h.position, h.yaw))
        else {
            continue;
        };
        let delta = host_position - units[i].position;
        for member in &mut units[i].members {
            if member.alive() {
                member.position = member.position + delta;
            }
        }
        units[i].position = host_position;
        units[i].yaw = yaw;
    }
}

/// Lower Board and Collect orders into movement or a shared hold. Timer and
/// phase progression belongs to [`crate::hold::advance`].
pub(crate) fn advance(
    units: &mut [Unit],
    rules: &Rules,
    refusals: &mut [Vec<contract::command::Refusal>; 3],
) {
    copy_passenger_positions(units);
    for i in 0..units.len() {
        if !units[i].alive() || units[i].hold.is_some() {
            continue;
        }
        if matches!(units[i].orders.front(), Some(UnitOrder::Move(_)))
            && matches!(units[i].orders.get(1), Some(UnitOrder::Collect { .. }))
            && units[i].route.is_none()
            && units[i].drive_speed_mps.abs() < 1e-6
            && matches!(units[i].orders.get(1), Some(UnitOrder::Collect { squads, .. }) if squads.iter().any(|id| units.get(id.0 as usize).is_some_and(|s| s.alive() && within_reach(&units[i], s, rules))))
        {
            units[i].orders.pop_front();
        }
        let Some(order) = units[i].orders.front().cloned() else {
            continue;
        };
        match order {
            UnitOrder::Board { carrier, gesture } => {
                let Some((c_side, c_pos, c_seats)) = units
                    .get(carrier.0 as usize)
                    .map(|c| (c.side, c.position.xy(), seats(c, rules)))
                else {
                    units[i].orders.pop_front();
                    refusals[units[i].side.index()].push(contract::command::Refusal {
                        unit: units[i].id,
                        reason: contract::command::OrderError::NotACarrier,
                    });
                    continue;
                };
                if c_side != units[i].side || c_seats == 0 {
                    units[i].orders.pop_front();
                    refusals[units[i].side.index()].push(contract::command::Refusal {
                        unit: units[i].id,
                        reason: contract::command::OrderError::NotACarrier,
                    });
                    continue;
                }
                if near_and_still(units, i, carrier, rules) {
                    units[i].hold = Some(Hold {
                        host: Host::Carrier(carrier),
                        phase: Phase::Entering(0),
                        place: Place::Carrier,
                    });
                } else {
                    // The destination is the carrier centre; movement stops at
                    // the same hull envelope used by hold admission.
                    push_target_move(&mut units[i], c_pos, gesture);
                }
            }
            UnitOrder::Collect { squads, gesture } => {
                if seats(&units[i], rules) == 0 {
                    units[i].orders.pop_front();
                    refusals[units[i].side.index()].push(contract::command::Refusal {
                        unit: units[i].id,
                        reason: contract::command::OrderError::NotACarrier,
                    });
                    continue;
                }
                let target_id = squads.iter().find(|id| {
                    units
                        .get(id.0 as usize)
                        .is_some_and(|s| s.alive() && s.side == units[i].side && s.hold.is_none())
                });
                let Some(target_id) = target_id.copied() else {
                    units[i].orders.pop_front();
                    for id in squads {
                        let Some(target) = units.get(id.0 as usize) else {
                            continue;
                        };
                        let reason = if !target.alive() {
                            Some(contract::command::OrderError::Destroyed { unit: id })
                        } else if target
                            .hold
                            .as_ref()
                            .is_some_and(|hold| hold.host.carrier().is_some())
                        {
                            Some(contract::command::OrderError::Aboard { unit: id })
                        } else {
                            None
                        };
                        if let Some(reason) = reason {
                            refusals[units[i].side.index()]
                                .push(contract::command::Refusal { unit: id, reason });
                        }
                    }
                    continue;
                };
                let target = units[target_id.0 as usize].position.xy();
                if within_reach(&units[i], &units[target_id.0 as usize], rules)
                    && stationary(&units[i])
                {
                    let carrier_id = units[i].id;
                    for id in squads {
                        let k = id.0 as usize;
                        let eligible = units.get(k).is_some_and(|s| {
                            s.alive() && s.side == units[i].side && s.hold.is_none()
                        });
                        if eligible {
                            let squad = &mut units[k];
                            squad.hold = Some(Hold {
                                host: Host::Carrier(carrier_id),
                                phase: Phase::Entering(1),
                                place: Place::Carrier,
                            });
                            squad.orders.clear();
                            squad.orders.push_front(UnitOrder::Board {
                                carrier: carrier_id,
                                gesture,
                            });
                        }
                    }
                    units[i].orders.pop_front();
                } else {
                    // Keep steering at the squad until the shared hold near
                    // test accepts the physical hull envelope.
                    push_target_move(&mut units[i], target, gesture);
                }
            }
            _ => {}
        }
    }
    copy_passenger_positions(units);
}
