//! Applying an accepted command to the units it names: each order replaces
//! a unit's queue or, queued, joins its end.

use super::*;

impl Battle {
    pub(super) fn apply(&mut self, prepared: PreparedCommand) {
        let side = prepared.command.side.index();
        self.refusals[side].extend(prepared.refused.iter().cloned());
        if let Order::PopSmoke { units } = &prepared.command.order {
            for id in units {
                let unit = &mut self.units[id.0 as usize];
                crate::smoke::pop(unit, self.world.smoke_mut(), self.tick, &self.rules);
            }
        }
        let mut units = std::mem::take(&mut self.units);
        self.apply_units(prepared, &mut units);
        self.units = units;
    }

    pub(super) fn apply_units(&self, prepared: PreparedCommand, movers: &mut [Unit]) {
        let command = prepared.command;
        let side = command.side;
        let queued = command.queued;
        let push = |unit: &mut Unit, order: UnitOrder| unit.enqueue(order, queued);
        // A queued order waits its turn: it breaks nothing off.
        let replacing = match &command.order {
            Order::Stop { units }
            | Order::SetAfterburner { units, .. }
            | Order::SetEngagement { units, .. }
            | Order::SetDeployment { units, .. }
            | Order::Unload {
                units, goal: None, ..
            } => Some(units),
            Order::Move { units, .. }
            | Order::AttackMove { units, .. }
            | Order::Attack { units, .. }
            | Order::OccupyBuilding { units, .. }
            | Order::ExitBuilding { units }
            | Order::Unload { units, .. }
            | Order::Refund { units } => (!queued).then_some(units),
            _ => None,
        };
        if let Some(units) = replacing {
            for id in units {
                take_order(&mut movers[id.0 as usize]);
            }
        }
        match command.order {
            // Queued, it waits its turn to find the way home from there.
            Order::Refund { units } if queued => {
                for id in units {
                    movers[id.0 as usize].enqueue(UnitOrder::Withdraw, true);
                }
            }
            Order::Refund { .. } => {
                for slot in prepared.placement.unwrap().destinations {
                    Self::head_home(&mut movers[slot.unit.0 as usize], slot.goal);
                }
            }
            Order::Ready
            | Order::ConfirmPurchase { .. }
            | Order::CancelPending { .. }
            | Order::PopSmoke { .. } => {}
            Order::OccupyBuilding { gesture, .. } => {
                let plan = prepared.building.unwrap();
                for entry in plan.entries {
                    push(
                        &mut movers[entry.unit.0 as usize],
                        UnitOrder::Garrison {
                            building: plan.building,
                            approach: v2(entry.approach[0], entry.approach[1]),
                            gesture,
                            policy: RoutePolicy::Shortest,
                        },
                    );
                }
            }
            Order::Move {
                units: _,
                gesture,
                goal: _,
                route,
                direction,
                facing,
            } => {
                for slot in prepared.placement.unwrap().destinations {
                    let unit = &mut movers[slot.unit.0 as usize];
                    if !slot.placed {
                        if !queued {
                            Self::hold_position(unit);
                        }
                        continue;
                    }
                    let destination = v2(slot.goal[0], slot.goal[1]);
                    let order = UnitOrder::Move(MoveOrder {
                        destination,
                        policy: route,
                        gesture,
                        direction,
                        facing,
                        short: prepared.stops.get(&slot.unit).cloned(),
                    });
                    push(unit, order);
                }
            }
            Order::AttackMove { gesture, .. } => {
                for slot in prepared.placement.unwrap().destinations {
                    let unit = &mut movers[slot.unit.0 as usize];
                    if !slot.placed {
                        if !queued {
                            Self::hold_position(unit);
                        }
                        continue;
                    }
                    // Nothing follows a unit going home (`Unit::enqueue`).
                    if queued && unit.withdrawing {
                        continue;
                    }
                    let destination = v2(slot.goal[0], slot.goal[1]);
                    controllers::fire_at_will(unit);
                    let order = UnitOrder::AttackMove(MoveOrder {
                        destination,
                        policy: contract::command::RoutePolicy::Shortest,
                        gesture,
                        direction: contract::command::MoveDirection::Forward,
                        facing: None,
                        short: prepared.stops.get(&slot.unit).cloned(),
                    });
                    push(unit, order);
                }
            }
            Order::Attack { units, target } => {
                // Resolved again at application; a target lost since the ack is dropped.
                let Some(target) = self.resolve_target(side, &target) else {
                    return;
                };
                for id in units {
                    let dogfight =
                        attack_order::dogfight_with(&movers[id.0 as usize], target, movers);
                    let unit = &mut movers[id.0 as usize];
                    // Nothing follows a unit going home (`Unit::enqueue`).
                    if queued && unit.withdrawing {
                        continue;
                    }
                    let waiting = queued && !unit.orders.is_empty();
                    if !waiting {
                        Self::take_effect(unit, dogfight);
                    }
                    push(
                        unit,
                        UnitOrder::Attack {
                            target,
                            last_known: None,
                            waiting,
                            volley: None,
                        },
                    );
                }
            }
            Order::ExitBuilding { units } => {
                for id in units {
                    push(&mut movers[id.0 as usize], UnitOrder::Exit);
                }
            }
            Order::Board {
                units,
                carrier,
                gesture,
            } => {
                for id in units {
                    push(
                        &mut movers[id.0 as usize],
                        UnitOrder::Board { carrier, gesture },
                    );
                }
            }
            Order::Collect {
                units,
                squads,
                gesture,
            } => {
                for id in units {
                    push(
                        &mut movers[id.0 as usize],
                        UnitOrder::Collect {
                            squads: squads.clone(),
                            gesture,
                        },
                    );
                }
            }
            Order::Unload {
                units,
                goal,
                gesture,
                route,
                into,
            } => {
                let into = into.map(hold::Host::from);
                let refused: std::collections::BTreeSet<UnitId> = prepared
                    .refused
                    .iter()
                    .filter(|refusal| refusal.reason == OrderError::NoRoomToUnload)
                    .map(|refusal| refusal.unit)
                    .collect();
                for id in units {
                    let unit = &mut movers[id.0 as usize];
                    if refused.contains(&id) {
                        Self::hold_position(unit);
                        continue;
                    }
                    match goal {
                        Some(_) => {
                            let Some(placement) = prepared.placement.as_ref() else {
                                continue;
                            };
                            let Some(destination) = placement
                                .destinations
                                .iter()
                                .find(|slot| slot.unit == id && slot.placed)
                                .map(|slot| v2(slot.goal[0], slot.goal[1]))
                            else {
                                if !queued {
                                    Self::hold_position(unit);
                                }
                                continue;
                            };
                            push(
                                unit,
                                UnitOrder::Move(MoveOrder {
                                    destination,
                                    policy: route,
                                    gesture,
                                    direction: Default::default(),
                                    facing: None,
                                    short: prepared.stops.get(&id).cloned(),
                                }),
                            );
                            unit.enqueue(UnitOrder::Unload { into }, true);
                        }
                        None => {
                            Self::hold_position(unit);
                            unit.enqueue(UnitOrder::Unload { into: None }, false);
                        }
                    }
                }
            }
            Order::SetEngagement { units, policy } => {
                for id in units {
                    controllers::order_engagement(&mut movers[id.0 as usize], policy);
                }
            }
            Order::SetAfterburner { units, enabled } => {
                for id in units {
                    if let Some(air) = movers[id.0 as usize].air.as_mut() {
                        air.afterburner = enabled;
                    }
                }
            }
            Order::Stop { units } => {
                // W15: clear the queue and cancel movement, aim, reload and
                // guidance once; policy stays; automatic fire may restart.
                for id in units {
                    let unit = &mut movers[id.0 as usize];
                    unit.orders.clear();
                    unit.route = None;
                    unit.planned_goal = None;
                    unit.pursuit = None;
                    unit.state = MoveState::Idle;
                    unit.drive_speed_mps = 0.0;
                    unit.blocker = None;
                    unit.turn_to = None;
                    for mount in &mut unit.mounts {
                        mount.stop();
                    }
                    // Stop never leaves a unit half-packed: it sets up where it is.
                    if let Some(d) = unit.deployment.as_mut() {
                        d.stationary = Posture::Deployed;
                    }
                }
            }
            Order::SetDeployment { units, deployed } => {
                for id in units {
                    let unit = &mut movers[id.0 as usize];
                    let Some(d) = unit.deployment.as_mut() else {
                        continue; // units that never set up ignore it
                    };
                    if deployed {
                        // Set up here: movement ends and progress reverses from where it stands.
                        d.stationary = Posture::Deployed;
                        unit.orders.clear();
                        unit.route = None;
                        unit.planned_goal = None;
                        unit.pursuit = None;
                        unit.state = MoveState::Idle;
                        unit.blocker = None;
                    } else {
                        // Pack and stay packed once any movement ends.
                        d.stationary = Posture::Packed;
                    }
                }
            }
            Order::UpgradeMove { gesture, route } => {
                for unit in movers.iter_mut().filter(|u| u.side == side) {
                    if unit.upgrade_walks(gesture, route) {
                        take_order(unit);
                    }
                }
            }
        }
    }

    fn hold_position(unit: &mut Unit) {
        unit.orders.clear();
        unit.route = None;
        unit.planned_goal = None;
        unit.pursuit = None;
        unit.state = MoveState::Idle;
        unit.drive_speed_mps = 0.0;
        unit.blocker = None;
        unit.turn_to = None;
    }
}

/// What an order that replaces a unit's queue does to it first: it is no
/// longer withdrawing, and a jet's run, release cue and dogfight end, so no
/// order (repeated or not) can leave one stuck in a run. An order queued
/// behind the current one does none of this.
fn take_order(unit: &mut Unit) {
    unit.withdrawing = false;
    crate::air::break_off(unit);
}
