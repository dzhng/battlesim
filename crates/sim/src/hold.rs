//! A unit's host and stationary hold state.

use std::collections::BTreeSet;

use crate::garrison::Garrison;
use crate::units::{Unit, UnitOrder};
use crate::world::PropId;
use contract::scenario::Rules;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Stationary beside a host for this many ticks.
    Entering(u32),
    Inside,
    /// Stationary inside a host, leaving, for this many ticks.
    Exiting(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Host {
    Building(PropId),
    Carrier(contract::ids::UnitId),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Timing {
    pub enter_exit_s: f64,
    pub reach_m: f64,
}

impl Host {
    pub fn timing(self, rules: &Rules) -> Timing {
        match self {
            Self::Building(_) => Timing {
                enter_exit_s: rules.garrison.enter_exit_s,
                reach_m: rules.garrison.entry_distance_m,
            },
            Self::Carrier(_) => Timing {
                enter_exit_s: rules.carriage.board_s,
                reach_m: rules.carriage.reach_m,
            },
        }
    }

    pub fn building(self) -> Option<PropId> {
        match self {
            Self::Building(id) => Some(id),
            Self::Carrier(_) => None,
        }
    }

    pub fn carrier(self) -> Option<contract::ids::UnitId> {
        match self {
            Self::Carrier(id) => Some(id),
            Self::Building(_) => None,
        }
    }
}

/// Host-specific placement data carried by a hold.
#[derive(Clone, Debug)]
pub enum Place {
    Building(Garrison),
    Carrier,
}

impl Place {
    pub fn building(&self) -> Option<&Garrison> {
        match self {
            Self::Building(garrison) => Some(garrison),
            Self::Carrier => None,
        }
    }

    pub fn building_mut(&mut self) -> Option<&mut Garrison> {
        match self {
            Self::Building(garrison) => Some(garrison),
            Self::Carrier => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Hold {
    pub host: Host,
    pub phase: Phase,
    pub place: Place,
}

impl Hold {
    pub fn building(&self) -> Option<&Garrison> {
        self.place.building()
    }

    pub fn building_mut(&mut self) -> Option<&mut Garrison> {
        self.place.building_mut()
    }

    /// The carrier this squad rides inside, off the field: aboard, or still
    /// inside while it gets out. A squad still getting in stands beside it.
    pub fn riding(&self) -> Option<contract::ids::UnitId> {
        match (self.host, self.phase) {
            (Host::Carrier(id), Phase::Inside | Phase::Exiting(_)) => Some(id),
            _ => None,
        }
    }
}

use crate::arrangement;
use crate::garrison::{self, Want};
use crate::world::WorldGeometry;
use contract::observation::MoveState;

fn ticks(seconds: f64, rules: &Rules) -> u32 {
    ((seconds * rules.tick_hz as f64).round() as u32).max(1)
}

/// Advance every unit's hold through one shared phase and timer. Host-specific
/// checks and admission remain in garrison/carriage; this function owns the
/// stationary state machine for both hosts.
pub(crate) fn advance(
    world: &WorldGeometry,
    sides: &[crate::movement::SideGeometry; 3],
    authored: PropId,
    units: &mut [Unit],
    rules: &Rules,
    seed: u64,
    refusals: &mut [Vec<contract::command::Refusal>; 3],
) {
    let mut unload_checked = BTreeSet::new();
    for i in 0..units.len() {
        if !units[i].alive() {
            continue;
        }
        let Some((host, phase)) = units[i].hold.as_ref().map(|h| (h.host, h.phase)) else {
            let want = garrison::want(&units[i]);
            let Want::Enter(building) = want else {
                if matches!(want, Want::Leave)
                    && matches!(units[i].orders.front(), Some(UnitOrder::Exit))
                {
                    units[i].orders.pop_front();
                }
                continue;
            };
            let Some(_) = sides[units[i].side.index()]
                .prop(world, authored, building)
                .filter(|p| p.body.garrison)
            else {
                units[i].orders.pop_front();
                continue;
            };
            if world
                .structure_parts(building)
                .into_iter()
                .filter_map(|id| sides[units[i].side.index()].prop(world, authored, id))
                .any(|p| {
                    p.footprint().contains(
                        units[i].position.xy(),
                        Host::Building(building).timing(rules).reach_m,
                    )
                })
            {
                let at = units[i].position.xy();
                units[i].route = None;
                units[i].planned_goal = None;
                units[i].state = MoveState::Idle;
                units[i].hold = Some(Hold {
                    host: Host::Building(building),
                    phase: Phase::Entering(1),
                    place: Place::Building(Garrison {
                        slots: Vec::new(),
                        seats: Vec::new(),
                        entry: at,
                    }),
                });
            }
            continue;
        };
        let timer = ticks(host.timing(rules).enter_exit_s, rules);
        match host {
            Host::Carrier(carrier) => {
                let order_matches = matches!(
                    units[i].orders.front(),
                    Some(UnitOrder::Board { carrier: target, .. }) if *target == carrier
                );
                let unloading = units
                    .get(carrier.0 as usize)
                    .is_some_and(|host| matches!(host.orders.front(), Some(UnitOrder::Unload)));
                let hover_settled = units
                    .get(carrier.0 as usize)
                    .is_some_and(|host| crate::carriage::at_loading_height(world, host, rules));
                if matches!(phase, Phase::Entering(_))
                    && (!order_matches
                        || !crate::carriage::near_and_still(world, units, i, carrier, rules))
                {
                    let host_missing = units
                        .get(carrier.0 as usize)
                        .is_none_or(|host| !host.alive());
                    units[i].hold = None;
                    if host_missing && order_matches {
                        units[i].orders.pop_front();
                        refusals[units[i].side.index()].push(contract::command::Refusal {
                            unit: units[i].id,
                            reason: contract::command::OrderError::NotACarrier,
                        });
                    }
                    continue;
                }
                match phase {
                    Phase::Entering(n) if n < timer => {
                        units[i].hold.as_mut().expect("carrier hold").phase =
                            Phase::Entering(n + 1);
                    }
                    Phase::Entering(_) => {
                        if crate::carriage::fits(units, carrier, rules) {
                            units[i].hold.as_mut().expect("carrier hold").phase = Phase::Inside;
                            if matches!(units[i].orders.front(), Some(UnitOrder::Board { .. })) {
                                units[i].orders.pop_front();
                            }
                        } else {
                            units[i].hold = None;
                            units[i].orders.pop_front();
                            refusals[units[i].side.index()].push(contract::command::Refusal {
                                unit: units[i].id,
                                reason: contract::command::OrderError::CapacityFull,
                            });
                        }
                    }
                    Phase::Inside if unloading && hover_settled => {
                        units[i].hold.as_mut().expect("carrier hold").phase = Phase::Exiting(1);
                    }
                    Phase::Inside => {}
                    Phase::Exiting(_) if !unloading => {
                        units[i].hold.as_mut().expect("carrier hold").phase = Phase::Inside;
                    }
                    Phase::Exiting(n) if n < timer => {
                        units[i].hold.as_mut().expect("carrier hold").phase = Phase::Exiting(n + 1);
                    }
                    Phase::Exiting(_) => {
                        if !unload_checked.insert(carrier) {
                            continue;
                        }
                        let unloaded =
                            crate::carriage::complete_unload(world, units, carrier, rules);
                        let carrier_orders = units
                            .get_mut(carrier.0 as usize)
                            .map(|host| &mut host.orders);
                        if let Some(orders) = carrier_orders {
                            if matches!(orders.front(), Some(UnitOrder::Unload)) {
                                orders.pop_front();
                            }
                        }
                        if !unloaded {
                            for passenger in units.iter_mut().filter(|unit| {
                                unit.hold
                                    .as_ref()
                                    .is_some_and(|hold| hold.host == Host::Carrier(carrier))
                            }) {
                                passenger.hold.as_mut().expect("carrier hold").phase =
                                    Phase::Inside;
                            }
                            refusals[units[i].side.index()].push(contract::command::Refusal {
                                unit: carrier,
                                reason: contract::command::OrderError::NoRoomToUnload,
                            });
                        }
                    }
                }
            }
            Host::Building(building) => {
                let want = garrison::want(&units[i]);
                if matches!(phase, Phase::Entering(_))
                    && (!matches!(want, Want::Enter(target) if target == building)
                        || garrison::building(world, building).is_none())
                {
                    units[i].hold = None;
                    if matches!(units[i].orders.front(), Some(UnitOrder::Exit)) {
                        units[i].orders.pop_front();
                    }
                    continue;
                }
                match (phase, want) {
                    (Phase::Entering(n), _) if n < timer => {
                        units[i].hold.as_mut().expect("building hold").phase =
                            Phase::Entering(n + 1);
                    }
                    (Phase::Entering(_), _) => {
                        if garrison::held_by_another(units, building, i) {
                            units[i].hold = None;
                            units[i].orders.pop_front();
                            continue;
                        }
                        let entry = units[i]
                            .hold
                            .as_ref()
                            .and_then(|hold| hold.building())
                            .expect("building hold")
                            .entry;
                        let prop = garrison::building(world, building).expect("standing building");
                        let slots = garrison::seats_for_state(world, prop, rules);
                        if units[i].members.iter().filter(|s| s.alive()).count() > slots.len() {
                            units[i].hold = None;
                            units[i].orders.pop_front();
                            continue;
                        }
                        let seats = garrison::seat_evenly(&slots, &units[i]);
                        let unit = &mut units[i];
                        for (s, seat) in unit.members.iter_mut().zip(&seats) {
                            if let Some(k) = seat {
                                s.position = slots[*k].position;
                            }
                        }
                        unit.settle();
                        unit.hold = Some(Hold {
                            host: Host::Building(building),
                            phase: Phase::Inside,
                            place: Place::Building(Garrison {
                                slots,
                                seats,
                                entry,
                            }),
                        });
                        unit.orders.pop_front();
                    }
                    (Phase::Inside, Want::Enter(target)) if target == building => {
                        units[i].orders.pop_front();
                    }
                    (Phase::Inside, Want::Enter(_) | Want::Leave) => {
                        units[i].hold.as_mut().expect("building hold").phase = Phase::Exiting(1);
                    }
                    (Phase::Inside, Want::Stay) => {}
                    (Phase::Exiting(_), Want::Stay) => {
                        units[i].hold.as_mut().expect("building hold").phase = Phase::Inside;
                    }
                    (Phase::Exiting(n), _) if n < timer => {
                        units[i].hold.as_mut().expect("building hold").phase =
                            Phase::Exiting(n + 1);
                    }
                    (Phase::Exiting(_), _) => {
                        let unit = &units[i];
                        let prop =
                            garrison::building(world, building).expect("occupied building stands");
                        let entry = unit
                            .hold
                            .as_ref()
                            .and_then(|hold| hold.building())
                            .expect("building hold")
                            .entry;
                        let preferred = match unit.orders.front() {
                            Some(UnitOrder::Garrison { approach, .. }) => *approach,
                            Some(o) => o.movement().map_or(entry, |m| m.destination),
                            None => entry,
                        };
                        let mut draws =
                            arrangement::rng(seed, unit.id.0, arrangement::arrival(preferred));
                        if let Some(spots) =
                            garrison::exit_spots(world, prop, unit, preferred, rules, &mut draws)
                        {
                            let unit = &mut units[i];
                            let living = unit.members.iter_mut().filter(|s| s.alive());
                            for (s, p) in living.zip(spots) {
                                let z = world.surface_at(p.x, p.y).map_or(s.position.z, |g| g.z);
                                s.position = p.with_z(z);
                            }
                            unit.settle();
                            unit.anchor = Some(crate::cover::Anchor {
                                at: unit.position.xy(),
                                halt: false,
                            });
                            unit.hold = None;
                            if matches!(unit.orders.front(), Some(UnitOrder::Exit)) {
                                unit.orders.pop_front();
                            }
                        }
                    }
                }
            }
        }
    }
}
