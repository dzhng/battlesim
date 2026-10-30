//! Carriers: seats, boarding, riding along and unloading, for ground
//! vehicles and helicopters alike.

use contract::catalog::TypeIndex;
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
    let stepping = units.iter().any(|unit| {
        unit.alive()
            && unit.hold.as_ref().is_some_and(|hold| {
                hold.host == Host::Carrier(carrier)
                    && matches!(hold.phase, Phase::Entering(_) | Phase::Exiting(_))
            })
    });
    let unloading = matches!(host.orders.front(), Some(UnitOrder::Unload))
        && !passengers(units, carrier).is_empty();
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
    stepping || unloading || pending
}

/// The seats `kind` carries; 0 for a unit that carries no one.
pub(crate) fn seats(rules: &Rules, kind: TypeIndex) -> usize {
    rules
        .catalog
        .get(kind)
        .capabilities
        .carriage
        .map_or(0, |c| c.seats as usize)
}

/// The seats a squad takes: one per living soldier.
pub(crate) fn living(unit: &Unit) -> usize {
    unit.members.iter().filter(|s| s.alive()).count()
}

/// The seats taken on `carrier`: every squad holding on it, getting out,
/// aboard or still getting in. A squad getting in has its seats reserved, so
/// two squads can never both be admitted into the last ones.
pub(crate) fn seats_used(units: &[Unit], carrier: UnitId) -> usize {
    units
        .iter()
        .filter(|u| {
            u.hold
                .as_ref()
                .is_some_and(|h| h.host == Host::Carrier(carrier))
        })
        .map(living)
        .sum()
}

pub(crate) fn stationary(unit: &Unit) -> bool {
    unit.movement_goal().is_none() && unit.drive_speed_mps.abs() < 1e-6
}

pub(crate) fn within_reach(carrier: &Unit, passenger: &Unit, rules: &Rules) -> bool {
    reaches(carrier, passenger.position.xy(), rules)
}

/// Whether a squad standing at `point` is within the carrier's boarding reach.
fn reaches(carrier: &Unit, point: V2, rules: &Rules) -> bool {
    let hull_reach = carrier.hull.map_or(0.0, |h| h.xy().length());
    let reach = Host::Carrier(carrier.id).timing(rules).reach_m;
    (carrier.position.xy() - point).length() <= reach + hull_reach
}

/// Where a collecting carrier meets `squad`: where it stands, or for a squad
/// in a building, the spot outside where it went in.
fn meeting_point(squad: &Unit) -> V2 {
    (squad.hold.as_ref())
        .and_then(|hold| hold.building())
        .map_or(squad.position.xy(), |garrison| garrison.entry)
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
        && at_loading_height(world, host, rules)
}

/// The one rule for whether squads can step on or off a carrier (transport
/// D6): a ground carrier always can; a helicopter only once it has settled
/// at its low hover.
pub(crate) fn at_loading_height(world: &WorldGeometry, host: &Unit, rules: &Rules) -> bool {
    !host.airborne() || crate::movement::air::low_hover_settled(world, host, rules)
}

/// Whether every squad holding on `carrier` fits its seats.
pub(crate) fn fits(units: &[Unit], carrier: UnitId, rules: &Rules) -> bool {
    seats_used(units, carrier) <= seats(rules, units[carrier.0 as usize].kind)
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

/// Carry every squad riding inside a carrier along with it. A squad still
/// getting in stands on the ground beside the carrier; one getting out rides
/// until [`complete_unload`] puts it on ground spots.
pub(crate) fn copy_passenger_positions(units: &mut [Unit]) {
    for i in 0..units.len() {
        let Some(carrier) = units[i].hold.as_ref().and_then(|h| h.riding()) else {
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

/// The squads riding inside `carrier`, in unit order. A squad still getting
/// in stands beside it: an unload or the carrier's fate passes it by.
pub(crate) fn passengers(units: &[Unit], carrier: UnitId) -> Vec<usize> {
    (units.iter().enumerate())
        .filter(|(_, unit)| {
            unit.alive() && unit.hold.as_ref().and_then(Hold::riding) == Some(carrier)
        })
        .map(|(index, _)| index)
        .collect()
}

/// Find one reachable standing spot for every soldier riding `carrier`.
/// The carrier's own hull is deliberately omitted from occupancy: soldiers
/// leave from beneath it, while every other live hull and soldier remains a
/// physical obstruction.
pub(crate) fn unload_spots(
    world: &WorldGeometry,
    units: &[Unit],
    carrier: UnitId,
    rules: &Rules,
) -> Option<Vec<V3>> {
    let passengers = passengers(units, carrier);
    if passengers.is_empty() {
        return Some(Vec::new());
    }
    let (hulls, mut occupied) = crate::arrangement::ground_taken(units, carrier);
    let radius = rules.physics.soldier_radius_m;
    let from = units[carrier.0 as usize].position.xy();
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
    let passengers = passengers(units, carrier);
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
        // Taking cover after the ride belongs to the landing, not the pickup.
        unit.anchor = Some(crate::cover::Anchor {
            at: unit.position.xy(),
            halt: false,
        });
        unit.cover.due = true;
        unit.hold = None;
    }
    true
}

/// Whether unit `i` is walking or driving toward its Board or Collect target
/// and that walk is over: it is already within the hold's reach, or no
/// target is left to reach (the carrier died, or every listed squad is gone
/// or taken) and the order is refused instead. The approach
/// aims at the target's centre, which a hull keeps it from ever standing on,
/// so the walk ends here and the hold's own stillness test takes over.
fn approach_over(units: &[Unit], i: usize, rules: &Rules) -> bool {
    if !matches!(units[i].orders.front(), Some(UnitOrder::Move(_))) {
        return false;
    }
    let other = |id: &UnitId| units.get(id.0 as usize).filter(|u| u.alive());
    match units[i].orders.get(1) {
        Some(UnitOrder::Board { carrier, .. }) => {
            other(carrier).is_none_or(|c| within_reach(c, &units[i], rules))
        }
        Some(UnitOrder::Collect { squads, .. }) => {
            let mut left = (squads.iter())
                .filter_map(|id| units.get(id.0 as usize))
                .filter(|s| collectable(&units[i], s))
                .peekable();
            left.peek().is_none() || left.any(|s| reaches(&units[i], meeting_point(s), rules))
        }
        _ => false,
    }
}

/// Whether `carrier` can still collect `squad`: alive, on its side, and not
/// in a carrier. A squad in a building counts: its Board order takes it out.
fn collectable(carrier: &Unit, squad: &Unit) -> bool {
    squad.alive()
        && squad.side == carrier.side
        && squad
            .hold
            .as_ref()
            .is_none_or(|hold| hold.host.carrier().is_none())
}

/// Start a squad getting in to `carrier`; [`crate::hold::advance`] runs its
/// timer from here.
fn start_boarding(squad: &mut Unit, carrier: UnitId) {
    squad.hold = Some(Hold {
        host: Host::Carrier(carrier),
        phase: Phase::Entering(1),
        place: Place::Carrier,
    });
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
                && passengers(units, unit.id).is_empty()
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
        if approach_over(units, i, rules) {
            units[i].orders.pop_front();
        }
        let Some(order) = units[i].orders.front().cloned() else {
            continue;
        };
        match order {
            UnitOrder::Board { carrier, gesture } => {
                let Some((c_side, c_pos, c_seats, c_alive)) = units
                    .get(carrier.0 as usize)
                    .map(|c| (c.side, c.position.xy(), seats(rules, c.kind), c.alive()))
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
                if !c_alive {
                    units[i].orders.pop_front();
                    refusals[units[i].side.index()].push(contract::command::Refusal {
                        unit: units[i].id,
                        reason: contract::command::OrderError::Destroyed { unit: carrier },
                    });
                    continue;
                }
                if near_and_still(world, units, i, carrier, rules) {
                    start_boarding(&mut units[i], carrier);
                } else {
                    // The destination is the carrier centre; movement stops at
                    // the same hull envelope used by hold admission.
                    push_target_move(&mut units[i], c_pos, gesture);
                }
            }
            UnitOrder::Collect { squads, gesture } => {
                if seats(rules, units[i].kind) == 0 {
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
                        .is_some_and(|s| collectable(&units[i], s))
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
                let target = meeting_point(&units[target_id.0 as usize]);
                if reaches(&units[i], target, rules) && stationary(&units[i]) {
                    // Every listed squad is handed a Board order; one already
                    // beside the carrier, still and in nothing starts its hold
                    // now, the rest (a squad in a building too) walk over first.
                    let carrier_id = units[i].id;
                    for id in squads {
                        let k = id.0 as usize;
                        let eligible = units.get(k).is_some_and(|s| collectable(&units[i], s));
                        if !eligible {
                            continue;
                        }
                        let near = units[k].hold.is_none()
                            && near_and_still(world, units, k, carrier_id, rules);
                        let squad = &mut units[k];
                        squad.orders.clear();
                        squad.orders.push_front(UnitOrder::Board {
                            carrier: carrier_id,
                            gesture,
                        });
                        if near {
                            start_boarding(squad, carrier_id);
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
