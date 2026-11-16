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

impl From<contract::command::UnloadInto> for Host {
    fn from(into: contract::command::UnloadInto) -> Self {
        match into {
            contract::command::UnloadInto::Building { building } => Self::Building(building),
            contract::command::UnloadInto::Carrier { carrier } => Self::Carrier(carrier),
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
use crate::digest::Digest;
use crate::garrison::{self, Want};
use crate::world::WorldGeometry;
use contract::command::{Order, OrderError, Refusal};
use contract::ids::UnitId;
use contract::observation::{HoldPhase, HoldState, HostRef, MoveState};

/// The seats a squad takes: one per living soldier.
pub(crate) fn living(unit: &Unit) -> usize {
    unit.members.iter().filter(|s| s.alive()).count()
}

/// A host's seats: how many it has, and how many the living soldiers of
/// squads stepping in or inside (`staying`) and stepping out (`leaving`)
/// take. A carrier has its type's seats; a building, its window plan's while
/// a squad holds it, else the windows its standing state offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Seats {
    pub total: usize,
    pub staying: usize,
    pub leaving: usize,
}

impl Seats {
    pub(crate) fn of(
        world: &WorldGeometry,
        units: &[Unit],
        host: Host,
        windows: &garrison::Windows,
        rules: &Rules,
    ) -> Self {
        let total = match host {
            Host::Carrier(id) => units
                .get(id.0 as usize)
                .map_or(0, |c| crate::carriage::seats(rules, c.kind)),
            Host::Building(id) if garrison::held(units, id) => windows.plan(id).len(),
            Host::Building(id) => garrison::building(world, id)
                .map_or(0, |p| garrison::seats_for_state(world, p, rules).len()),
        };
        let mut seats = Self {
            total,
            staying: 0,
            leaving: 0,
        };
        for unit in units.iter().filter(|u| u.alive()) {
            match unit
                .hold
                .as_ref()
                .filter(|h| h.host == host)
                .map(|h| h.phase)
            {
                Some(Phase::Entering(_) | Phase::Inside) => seats.staying += living(unit),
                Some(Phase::Exiting(_)) => seats.leaving += living(unit),
                None => {}
            }
        }
        seats
    }

    /// Seats nobody on the host takes, stepping out included.
    pub(crate) fn free(&self) -> usize {
        self.total.saturating_sub(self.staying + self.leaving)
    }
}

/// Refuse at the command, one by one, every squad in `squads` bigger than a
/// whole host of `total` seats, keeping the rest: whether a squad finds room
/// is decided when it gets there.
pub(crate) fn refuse_oversized(
    units: &[Unit],
    squads: &mut Vec<UnitId>,
    total: usize,
) -> Vec<Refusal> {
    let mut refused = Vec::new();
    squads.retain(|&id| {
        let fits = living(&units[id.0 as usize]) <= total;
        if !fits {
            refused.push(Refusal {
                unit: id,
                reason: OrderError::CapacityFull,
            });
        }
        fits
    });
    refused
}

/// A carrier order's answers at the command, None for any other order. A
/// squad listed twice boards (or is collected) once, keeping the order
/// given; `Board` then refuses every squad bigger than its carrier, and
/// `Collect` every squad bigger than the largest carrier it lists.
pub(crate) fn refuse_oversized_riders(
    units: &[Unit],
    order: &mut Order,
    rules: &Rules,
) -> Option<Vec<Refusal>> {
    let mut seen = BTreeSet::new();
    let seats = |id: &UnitId| crate::carriage::seats(rules, units[id.0 as usize].kind);
    match order {
        Order::Board {
            units: squads,
            carrier,
            ..
        } => {
            squads.retain(|id| seen.insert(*id));
            Some(refuse_oversized(units, squads, seats(carrier)))
        }
        Order::Collect {
            units: carriers,
            squads,
            ..
        } => {
            squads.retain(|id| seen.insert(*id));
            let largest = carriers.iter().map(seats).max().unwrap_or(0);
            Some(refuse_oversized(units, squads, largest))
        }
        _ => None,
    }
}

/// End squad `i`'s front order, telling its side why.
fn refuse(units: &mut [Unit], i: usize, reason: OrderError, refusals: &mut [Vec<Refusal>; 3]) {
    units[i].orders.pop_front();
    refusals[units[i].side.index()].push(Refusal {
        unit: units[i].id,
        reason,
    });
}

/// Start squad `i` stepping into `host` where it stands: the one way a hold
/// begins, for both hosts. A squad that finds another side's squad in the
/// building (holding it or stepping in) gives up its order silently; one
/// whose living soldiers don't fit beside the squads stepping in or inside
/// (those stepping out don't count) ends it with `CapacityFull`.
pub(crate) fn admit(
    world: &WorldGeometry,
    units: &mut [Unit],
    i: usize,
    host: Host,
    windows: &garrison::Windows,
    rules: &Rules,
    refusals: &mut [Vec<Refusal>; 3],
) {
    let side = units[i].side;
    let enemy =
        |u: &Unit| u.alive() && u.side != side && u.hold.as_ref().is_some_and(|h| h.host == host);
    if host.building().is_some() && units.iter().any(enemy) {
        units[i].orders.pop_front();
        return;
    }
    let seats = Seats::of(world, units, host, windows, rules);
    if seats.staying + living(&units[i]) > seats.total {
        refuse(units, i, OrderError::CapacityFull, refusals);
        return;
    }
    let place = match host {
        Host::Building(_) => Place::Building(Garrison {
            seats: Vec::new(),
            entry: units[i].position.xy(),
        }),
        Host::Carrier(_) => Place::Carrier,
    };
    units[i].hold = Some(Hold {
        host,
        phase: Phase::Entering(1),
        place,
    });
}

/// The end of squad `i`'s entry timer: everyone on the host, stepping out
/// included, must fit; else the squad drops its hold and ends its order with
/// `CapacityFull`.
fn seated(
    world: &WorldGeometry,
    units: &mut [Unit],
    i: usize,
    host: Host,
    windows: &garrison::Windows,
    rules: &Rules,
    refusals: &mut [Vec<Refusal>; 3],
) -> bool {
    let seats = Seats::of(world, units, host, windows, rules);
    let fits = seats.staying + seats.leaving <= seats.total;
    if !fits {
        units[i].hold = None;
        refuse(units, i, OrderError::CapacityFull, refusals);
    }
    fits
}

/// A duration in whole ticks, never less than one.
pub(crate) fn ticks(seconds: f64, rules: &Rules) -> u32 {
    ((seconds * rules.tick_hz as f64).round() as u32).max(1)
}

/// What the owning side sees of a squad's hold; a building also tells its
/// seat total, as admission counts it.
pub fn state(
    world: &WorldGeometry,
    units: &[Unit],
    unit: &Unit,
    windows: &garrison::Windows,
    rules: &Rules,
) -> Option<HoldState> {
    let hold = unit.hold.as_ref()?;
    let host = match hold.host {
        Host::Carrier(id) => HostRef::Carrier { id: id.0 },
        Host::Building(building) => {
            let footprint = world.structure_footprint(building)?;
            HostRef::Building {
                id: building,
                center: [footprint.center.x, footprint.center.y],
                half: [footprint.half.x, footprint.half.y],
                seats: Seats::of(world, units, hold.host, windows, rules).total as u32,
            }
        }
    };
    let timer = ticks(hold.host.timing(rules).enter_exit_s, rules) as f64;
    let (phase, progress) = match hold.phase {
        Phase::Entering(n) => (HoldPhase::Entering, n as f64 / timer),
        Phase::Inside => (HoldPhase::Inside, 1.0),
        Phase::Exiting(n) => (HoldPhase::Exiting, n as f64 / timer),
    };
    Some(HoldState {
        host,
        phase,
        progress,
    })
}

/// Fold a unit's hold into a digest, with presence and length tags.
pub fn digest(unit: &Unit, windows: &garrison::Windows, d: &mut Digest) {
    d.u64(unit.hold.is_some() as u64);
    let Some(hold) = &unit.hold else {
        return;
    };
    let (tag, n) = match hold.phase {
        Phase::Entering(n) => (0, n),
        Phase::Inside => (2, 0),
        Phase::Exiting(n) => (3, n),
    };
    match hold.host {
        Host::Carrier(id) => {
            d.u64(u64::MAX).u64(id.0 as u64).u64(tag).u64(n as u64);
        }
        Host::Building(building) => {
            d.u64(building as u64).u64(tag).u64(n as u64);
            let place = hold.building().expect("a building hold's place");
            windows.digest_place(building, hold.phase, place, d);
        }
    }
}

/// Advance every unit's hold through one shared phase and timer: the
/// stationary state machine for both hosts. It starts squads stepping into
/// buildings ([`admit`]), checks everyone fits when a squad's entry timer ends,
/// and opens and closes each building's window plan as squads take and leave
/// it.
#[allow(clippy::too_many_arguments)]
pub(crate) fn advance(
    world: &WorldGeometry,
    sides: &[crate::movement::SideGeometry; 3],
    authored: PropId,
    units: &mut [Unit],
    windows: &mut garrison::Windows,
    rules: &Rules,
    seed: u64,
    refusals: &mut [Vec<Refusal>; 3],
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
            if garrison::within_entry_reach(
                world,
                &sides[units[i].side.index()],
                authored,
                building,
                units[i].position.xy(),
                rules,
            ) {
                units[i].route = None;
                units[i].planned_goal = None;
                units[i].state = MoveState::Idle;
                let host = Host::Building(building);
                admit(world, units, i, host, windows, rules, refusals);
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
                let unloading = crate::carriage::unload_due(units, carrier, rules);
                let loading_height = units
                    .get(carrier.0 as usize)
                    .is_some_and(|host| crate::carriage::at_loading_height(world, host, rules));
                if matches!(phase, Phase::Entering(_))
                    && (!order_matches
                        || !crate::carriage::ready_to_board(world, units, i, carrier, rules))
                {
                    let host_missing = units
                        .get(carrier.0 as usize)
                        .is_none_or(|host| !host.alive());
                    units[i].hold = None;
                    if host_missing && order_matches {
                        refuse(units, i, OrderError::NotACarrier, refusals);
                    }
                    continue;
                }
                match phase {
                    Phase::Entering(n) if n < timer => {
                        units[i].hold.as_mut().expect("carrier hold").phase =
                            Phase::Entering(n + 1);
                    }
                    Phase::Entering(_) => {
                        if seated(world, units, i, host, windows, rules, refusals) {
                            units[i].hold.as_mut().expect("carrier hold").phase = Phase::Inside;
                            if matches!(units[i].orders.front(), Some(UnitOrder::Board { .. })) {
                                units[i].orders.pop_front();
                            }
                        }
                    }
                    Phase::Inside if unloading && loading_height => {
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
                        let landed = crate::carriage::complete_unload(world, units, carrier, rules);
                        let carrier_orders = units
                            .get_mut(carrier.0 as usize)
                            .map(|host| &mut host.orders);
                        let mut into = None;
                        if let Some(orders) = carrier_orders {
                            if let Some(UnitOrder::Unload { into: then }) = orders.front() {
                                into = *then;
                                orders.pop_front();
                            }
                        }
                        if let (Some(squads), Some(into)) = (&landed, into) {
                            let side = units[i].side.index();
                            crate::carriage::send_into(
                                world,
                                &sides[side],
                                authored,
                                units,
                                squads,
                                into,
                                rules,
                                &mut refusals[side],
                            );
                        }
                        if landed.is_none() {
                            for passenger in crate::carriage::passengers(units, carrier) {
                                units[passenger].hold.as_mut().expect("carrier hold").phase =
                                    Phase::Inside;
                            }
                            refusals[units[i].side.index()].push(Refusal {
                                unit: carrier,
                                reason: OrderError::NoRoomToUnload,
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
                        if !seated(world, units, i, host, windows, rules, refusals) {
                            continue;
                        }
                        let entry = units[i]
                            .hold
                            .as_ref()
                            .and_then(|hold| hold.building())
                            .expect("building hold")
                            .entry;
                        // The first squad seated opens the building's windows;
                        // a later one takes those nobody holds.
                        if !garrison::held(units, building) {
                            let prop =
                                garrison::building(world, building).expect("standing building");
                            windows.open(building, garrison::seats_for_state(world, prop, rules));
                        }
                        let plan = windows.plan(building);
                        let seats = garrison::seat_evenly(plan, units, i);
                        let unit = &mut units[i];
                        for (s, seat) in unit.members.iter_mut().zip(&seats) {
                            if let Some(k) = seat {
                                s.position = plan[*k].position;
                            }
                        }
                        unit.settle();
                        unit.hold = Some(Hold {
                            host: Host::Building(building),
                            phase: Phase::Inside,
                            place: Place::Building(Garrison { seats, entry }),
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
                            Some(UnitOrder::Board { carrier, .. }) => units
                                .get(carrier.0 as usize)
                                .map_or(entry, |c| c.position.xy()),
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
    windows.close_unheld(units);
}
