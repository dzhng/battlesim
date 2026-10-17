//! Boarding and passenger movement for ground carriers.

use contract::ids::UnitId;
use contract::scenario::Rules;

use crate::hold::{Hold, Host, Phase, Place};
use crate::math::V2;
use crate::units::{MoveOrder, Unit, UnitOrder};

fn ticks(seconds: f64, rules: &Rules) -> u32 {
    ((seconds * rules.tick_hz as f64).round() as u32).max(1)
}

fn seats(unit: &Unit, rules: &Rules) -> usize {
    unit.unit_type(rules)
        .capabilities
        .carriage
        .map_or(0, |c| c.seats as usize)
}

fn living(unit: &Unit) -> usize {
    unit.members.iter().filter(|s| s.alive()).count()
}

fn stationary(unit: &Unit) -> bool {
    unit.movement_goal().is_none() && unit.drive_speed_mps.abs() < 1e-6
}

fn within_reach(carrier: &Unit, passenger: &Unit, rules: &Rules) -> bool {
    let hull_reach = carrier.hull.map_or(0.0, |h| h.xy().length());
    (carrier.position.xy() - passenger.position.xy()).length()
        <= rules.carriage.reach_m + hull_reach + 1.0
}

fn capacity(units: &[Unit], carrier: UnitId, candidate: usize, rules: &Rules) -> bool {
    let used: usize = units
        .iter()
        .filter(
            |u| matches!(u.hold.as_ref().map(|h| h.host), Some(Host::Carrier(id)) if id == carrier),
        )
        .map(living)
        .sum();
    used.saturating_sub(living(&units[candidate])) + living(&units[candidate])
        <= seats(&units[carrier.0 as usize], rules)
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

fn copy_passenger_positions(units: &mut [Unit]) {
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

/// Advance carriage timers, target tracking and passenger poses.
pub(crate) fn advance(units: &mut [Unit], rules: &Rules) {
    copy_passenger_positions(units);
    let timer = ticks(rules.carriage.board_s, rules);
    for i in 0..units.len() {
        if !units[i].alive() {
            continue;
        }
        let carrier_hold = units[i].hold.as_ref().and_then(|h| match h.host {
            Host::Carrier(id) => Some((id, h.phase)),
            _ => None,
        });
        if let Some((carrier, phase)) = carrier_hold {
            let valid = units.get(carrier.0 as usize).is_some_and(|c| c.alive());
            let near = valid && within_reach(&units[carrier.0 as usize], &units[i], rules);
            let still = valid && stationary(&units[carrier.0 as usize]) && stationary(&units[i]);
            if !valid || (!near && matches!(phase, Phase::Entering(_))) {
                units[i].hold = None;
                continue;
            }
            match phase {
                Phase::Entering(n) if near && still && n < timer => {
                    units[i].hold.as_mut().unwrap().phase = Phase::Entering(n + 1)
                }
                Phase::Entering(_) if near && still => {
                    if capacity(units, carrier, i, rules) {
                        units[i].hold.as_mut().unwrap().phase = Phase::Inside;
                    } else {
                        units[i].hold = None;
                    }
                }
                Phase::Inside | Phase::Exiting(_) => {}
                _ => {}
            }
            continue;
        }
        let queued_board = match (units[i].orders.front(), units[i].orders.get(1)) {
            (Some(UnitOrder::Move(_)), Some(UnitOrder::Board { carrier, .. })) => Some(*carrier),
            _ => None,
        };
        if let Some(carrier) = queued_board {
            let carrier_pos = units.get(carrier.0 as usize).map(|c| c.position.xy());
            let stopped = units.get(carrier.0 as usize).is_some_and(stationary);
            if carrier_pos.is_some_and(|p| {
                stopped
                    && (p - units[i].position.xy()).length()
                        <= rules.carriage.reach_m
                            + units[carrier.0 as usize]
                                .hull
                                .map_or(0.0, |h| h.xy().length())
                            + 1.0
                    && capacity(units, carrier, i, rules)
            }) {
                units[i].orders.pop_front();
                units[i].orders.pop_front();
                units[i].hold = Some(Hold {
                    host: Host::Carrier(carrier),
                    phase: Phase::Entering(1),
                    place: Place::Carrier,
                });
                continue;
            }
        }
        let queued_collect = matches!(
            (units[i].orders.front(), units[i].orders.get(1)),
            (Some(UnitOrder::Move(_)), Some(UnitOrder::Collect { .. }))
        );
        if queued_collect && units[i].route.is_none() && units[i].drive_speed_mps.abs() < 1e-6 {
            let near_target = match units[i].orders.get(1) {
                Some(UnitOrder::Collect { squads, .. }) => squads
                    .iter()
                    .filter_map(|id| units.get(id.0 as usize))
                    .any(|s| s.alive() && within_reach(&units[i], s, rules)),
                _ => false,
            };
            if near_target {
                units[i].orders.pop_front();
            }
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
                    continue;
                };
                if c_side != units[i].side || c_seats == 0 {
                    units[i].orders.pop_front();
                    continue;
                }
                let near = within_reach(&units[carrier.0 as usize], &units[i], rules);
                if near
                    && stationary(&units[carrier.0 as usize])
                    && stationary(&units[i])
                    && capacity(units, carrier, i, rules)
                {
                    units[i].orders.pop_front();
                    units[i].hold = Some(Hold {
                        host: Host::Carrier(carrier),
                        phase: Phase::Entering(1),
                        place: Place::Carrier,
                    });
                } else {
                    push_target_move(&mut units[i], c_pos, gesture);
                }
            }
            UnitOrder::Collect { squads, gesture } => {
                if seats(&units[i], rules) == 0 {
                    units[i].orders.pop_front();
                    continue;
                }
                let target = squads
                    .iter()
                    .filter_map(|id| units.get(id.0 as usize))
                    .find(|s| s.alive() && s.side == units[i].side && s.hold.is_none())
                    .map(|s| s.position.xy());
                let Some(target) = target else {
                    units[i].orders.pop_front();
                    continue;
                };
                let target_id = squads
                    .iter()
                    .find(|id| {
                        units.get(id.0 as usize).is_some_and(|s| {
                            s.alive() && s.side == units[i].side && s.hold.is_none()
                        })
                    })
                    .copied();
                if target_id.is_some_and(|id| within_reach(&units[i], &units[id.0 as usize], rules))
                    && stationary(&units[i])
                {
                    let carrier_id = units[i].id;
                    let side = units[i].side;
                    for id in squads {
                        let k = id.0 as usize;
                        let eligible = units
                            .get(k)
                            .is_some_and(|s| s.alive() && s.side == side && s.hold.is_none())
                            && capacity(units, carrier_id, k, rules);
                        if eligible {
                            let squad = &mut units[k];
                            squad.hold = Some(Hold {
                                host: Host::Carrier(carrier_id),
                                phase: Phase::Entering(1),
                                place: Place::Carrier,
                            });
                            squad.orders.clear();
                        }
                    }
                } else {
                    let delta = target - units[i].position.xy();
                    let approach_distance = rules.carriage.reach_m
                        + units[i].hull.map_or(0.0, |h| h.xy().length()) * 0.9;
                    let approach = if delta.length() > approach_distance {
                        target - delta * (approach_distance / delta.length())
                    } else {
                        target
                    };
                    push_target_move(&mut units[i], approach, gesture);
                }
            }
            _ => {}
        }
    }
    copy_passenger_positions(units);
}
