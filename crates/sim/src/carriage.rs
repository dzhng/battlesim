//! Boarding and passenger movement for ground carriers.

use contract::ids::UnitId;
use contract::scenario::Rules;

use crate::hold::{Hold, Host, Phase, Place};
use crate::math::{V2, V3};
use crate::units::{MoveOrder, Unit, UnitOrder};
use crate::world::WorldGeometry;

/// Whether a carrier has a carriage hold that should pull a helicopter down
/// to its low hover.  A pending Board order is included because the hold can
/// only be created after the helicopter has settled; an Unload order keeps an
/// already-aboard squad's timer alive until its Exiting phase starts.
pub(crate) fn transport_hold_in_progress(units: &[Unit], carrier: UnitId) -> bool {
    let Some(host) = units.get(carrier.0 as usize) else {
        return false;
    };
    if !host.orders.is_empty() && !matches!(host.orders.front(), Some(UnitOrder::Unload)) {
        return false;
    }
    let entering = units.iter().any(|unit| {
        unit.alive()
            && unit.hold.as_ref().is_some_and(|hold| {
                hold.host == Host::Carrier(carrier)
                    && matches!(hold.phase, Phase::Entering(_) | Phase::Exiting(_))
            })
    });
    let unloading = (matches!(host.orders.front(), Some(UnitOrder::Unload))
        && units.iter().any(|unit| {
            unit.alive()
                && unit
                    .hold
                    .as_ref()
                    .is_some_and(|hold| hold.host == Host::Carrier(carrier))
        }));
    let pending = host.orders.is_empty()
        && units.iter().any(|unit| {
            unit.alive()
                && unit.orders.iter().take(2).any(|order| {
                    matches!(
                        order,
                        UnitOrder::Board { carrier: target, .. } if *target == carrier
                    )
                })
        });
    entering || unloading || pending
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

pub(crate) fn stationary(unit: &Unit) -> bool {
    unit.movement_goal().is_none() && unit.drive_speed_mps.abs() < 1e-6
}

pub(crate) fn within_reach(carrier: &Unit, passenger: &Unit, rules: &Rules) -> bool {
    let hull_reach = carrier.hull.map_or(0.0, |h| h.xy().length());
    (carrier.position.xy() - passenger.position.xy()).length()
        <= rules.carriage.reach_m + hull_reach
}

pub(crate) fn near_and_still(
    world: &WorldGeometry,
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
        && (!host.airborne() || crate::movement::air::low_hover_settled(world, host, rules))
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

fn aboard(unit: &Unit, carrier: UnitId) -> bool {
    unit.alive()
        && unit
            .hold
            .as_ref()
            .is_some_and(|hold| hold.host == Host::Carrier(carrier))
}

/// Find one reachable standing spot for every living passenger of `carrier`.
/// The carrier's own hull is deliberately omitted from occupancy: soldiers
/// leave from beneath it, while every other live hull and soldier remains a
/// physical obstruction.
pub(crate) fn unload_spots(
    world: &WorldGeometry,
    units: &[Unit],
    carrier: UnitId,
    rules: &Rules,
) -> Option<Vec<V3>> {
    let passengers: Vec<usize> = units
        .iter()
        .enumerate()
        .filter(|(_, unit)| aboard(unit, carrier))
        .map(|(index, _)| index)
        .collect();
    if passengers.is_empty() {
        return Some(Vec::new());
    }
    let hulls: Vec<_> = units
        .iter()
        .enumerate()
        .filter(|(index, unit)| *index != carrier.0 as usize && unit.alive())
        .filter_map(|(_, unit)| unit.ground_footprint())
        .collect();
    let soldiers: Vec<V2> = units
        .iter()
        .enumerate()
        .filter(|(index, _)| !passengers.contains(index))
        .flat_map(|(_, unit)| unit.member_positions().map(|position| position.xy()))
        .collect();
    let radius = rules.physics.soldier_radius_m;
    let from = units[carrier.0 as usize].position.xy();
    let mut occupied = soldiers;
    let mut spots = Vec::new();
    for &passenger in &passengers {
        for _soldier in units[passenger].members.iter().filter(|s| s.alive()) {
            let spot = crate::arrangement::forced_exit_spot(
                world,
                from,
                rules.carriage.unload_search_m,
                radius,
                &hulls,
                &occupied,
                &[],
            )?;
            occupied.push(spot.xy());
            spots.push(spot);
        }
    }
    Some(spots)
}

/// Commit an unload after its shared hold timer has completed. Spot search is
/// all-or-nothing, so a failed search leaves every passenger aboard.
pub(crate) fn complete_unload(
    world: &WorldGeometry,
    units: &mut [Unit],
    carrier: UnitId,
    rules: &Rules,
) -> bool {
    let passengers: Vec<usize> = units
        .iter()
        .enumerate()
        .filter(|(_, unit)| aboard(unit, carrier))
        .map(|(index, _)| index)
        .collect();
    let Some(spots) = unload_spots(world, units, carrier, rules) else {
        return false;
    };
    let mut spots = spots.into_iter();
    for index in passengers {
        let unit = &mut units[index];
        for soldier in unit.members.iter_mut().filter(|s| s.alive()) {
            soldier.position = spots.next().expect("unload spot count matches passengers");
        }
        unit.settle();
        unit.hold = None;
    }
    true
}

/// Whether unit `i` is walking or driving toward its Board or Collect target
/// and is already within the hold's reach of it. The approach aims at the
/// target's centre, which a hull keeps it from ever standing on, so the walk
/// ends here and the hold's own stillness test takes over.
fn approach_arrived(units: &[Unit], i: usize, rules: &Rules) -> bool {
    if !matches!(units[i].orders.front(), Some(UnitOrder::Move(_))) {
        return false;
    }
    let other = |id: &UnitId| units.get(id.0 as usize).filter(|u| u.alive());
    match units[i].orders.get(1) {
        Some(UnitOrder::Board { carrier, .. }) => {
            other(carrier).is_some_and(|c| within_reach(c, &units[i], rules))
        }
        Some(UnitOrder::Collect { squads, .. }) => squads
            .iter()
            .any(|id| other(id).is_some_and(|s| within_reach(&units[i], s, rules))),
        _ => false,
    }
}

/// Lower Board and Collect orders into movement or a shared hold. Timer and
/// phase progression belongs to [`crate::hold::advance`].
pub(crate) fn advance(
    world: &WorldGeometry,
    units: &mut [Unit],
    rules: &Rules,
    refusals: &mut [Vec<contract::command::Refusal>; 3],
) {
    copy_passenger_positions(units);
    let empty_unloads: Vec<bool> = units
        .iter()
        .map(|unit| {
            matches!(unit.orders.front(), Some(UnitOrder::Unload))
                && !units.iter().any(|passenger| aboard(passenger, unit.id))
        })
        .collect();
    for (index, empty) in empty_unloads.into_iter().enumerate() {
        if empty {
            units[index].orders.pop_front();
        }
    }
    for i in 0..units.len() {
        if !units[i].alive() || units[i].hold.is_some() {
            continue;
        }
        if approach_arrived(units, i, rules) {
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
                if near_and_still(world, units, i, carrier, rules) {
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
