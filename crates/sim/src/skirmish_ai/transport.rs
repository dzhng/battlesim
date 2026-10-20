//! The AI's transport (D16–D18): which carrier a squad rides, what pairing
//! asks the purchase rotation for, and where a carrier lets its squad out.
//! Pairs are never stored: each decision reads them off its own units, from
//! seat counts and squad sizes alone, never a unit's role or family.
use contract::catalog::{AltitudeLayer, TypeIndex};
use contract::command::Order;
use contract::ids::UnitId;
use contract::observation::{HoldPhase, ObservationFrame, OwnUnit};
use contract::scenario::Rules;
use std::collections::BTreeMap;

use super::{distance, EntrySite, Side};
use crate::carriage::seats;

/// The card a squad of `size` soldiers is paired with: the fewest seats that
/// take the whole squad, then the cheapest, then the smaller id. `None` for a
/// hull (size 0) or a squad no card seats.
pub(super) fn carrier_for(rules: &Rules, eligible: &[TypeIndex], size: usize) -> Option<TypeIndex> {
    if size == 0 {
        return None;
    }
    eligible
        .iter()
        .copied()
        .filter(|&k| seats(rules, k) >= size)
        .min_by(|&a, &b| {
            seats(rules, a)
                .cmp(&seats(rules, b))
                .then(rules.catalog.get(a).cost.cmp(&rules.catalog.get(b).cost))
                .then(rules.catalog.id(a).cmp(rules.catalog.id(b)))
        })
}

/// The seat count a squad card pairs with, if the deck seats it.
fn paired_seats(rules: &Rules, eligible: &[TypeIndex], kind: TypeIndex) -> Option<usize> {
    carrier_for(rules, eligible, rules.catalog.get(kind).squad_size()).map(|c| seats(rules, c))
}

/// What pairing asks the purchase rotation for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Want {
    /// A squad has no carrier: buy it one (a lost transport is rebought).
    Carrier(TypeIndex),
    /// A carrier has no squad: buy one it seats (a lost squad is replaced).
    Squad(TypeIndex),
}

/// One carrier per squad, counted by seats over every unit the player has
/// or has paid for (`kinds`): a squad short of a carrier comes first, then a
/// carrier short of a squad. A carrier no squad card pairs with (more seats
/// than any squad needs) asks for nothing.
pub(super) fn want(
    rules: &Rules,
    eligible: &[TypeIndex],
    kinds: impl Iterator<Item = TypeIndex>,
) -> Option<Want> {
    // Per seat count: squads that pair with it, less carriers that have it.
    let mut balance: BTreeMap<usize, i64> = BTreeMap::new();
    for kind in kinds {
        if let Some(s) = paired_seats(rules, eligible, kind) {
            *balance.entry(s).or_default() += 1;
        }
        let s = seats(rules, kind);
        if s > 0 {
            *balance.entry(s).or_default() -= 1;
        }
    }
    let cheapest = |matches: &dyn Fn(TypeIndex) -> bool| {
        eligible
            .iter()
            .copied()
            .filter(|&k| matches(k))
            .min_by(|&a, &b| {
                rules
                    .catalog
                    .get(a)
                    .cost
                    .cmp(&rules.catalog.get(b).cost)
                    .then(rules.catalog.id(a).cmp(rules.catalog.id(b)))
            })
    };
    if let Some((&s, _)) = balance.iter().find(|(_, &b)| b > 0) {
        return cheapest(&|k| seats(rules, k) == s).map(Want::Carrier);
    }
    balance
        .iter()
        .filter(|(_, &b)| b < 0)
        .find_map(|(&s, _)| cheapest(&|k| paired_seats(rules, eligible, k) == Some(s)))
        .map(Want::Squad)
}

/// The carrier `unit` is aboard, boarding or leaving.
pub(super) fn carrier_of(unit: &OwnUnit) -> Option<UnitId> {
    match unit.hold?.host {
        contract::observation::HostRef::Carrier { id } => Some(UnitId(id)),
        contract::observation::HostRef::Building { .. } => None,
    }
}

/// The squads aboard, boarding or leaving `carrier`.
pub(super) fn passengers<'a>(
    frame: &'a ObservationFrame,
    carrier: UnitId,
) -> impl Iterator<Item = &'a OwnUnit> + 'a {
    frame
        .own
        .iter()
        .filter(move |u| carrier_of(u) == Some(carrier))
}

/// Whether every passenger of `carrier` sits inside, none boarding or leaving.
pub(super) fn all_inside(frame: &ObservationFrame, carrier: UnitId) -> bool {
    passengers(frame, carrier).all(|u| u.hold.is_some_and(|h| h.phase == HoldPhase::Inside))
}

/// The farthest a weapon of `kind` reaches at the ground.
pub(super) fn ground_reach(rules: &Rules, kind: TypeIndex) -> f64 {
    super::unit_weapons(rules, kind)
        .filter_map(|weapon| weapon.against(AltitudeLayer::Ground))
        .map(|fire| fire.range_m)
        .fold(0.0, f64::max)
}

/// Where a carrier coming from `from` lets its squad out for `objective`: on
/// the line between them, `standoff` short of the objective or farther, and
/// outside the ground reach of every enemy the side identifies. Contacts and
/// unseen enemies never move it (D18). Where no such point is nearer the
/// objective than `from`, it is `from`: the squad gets out where it is.
pub(super) fn unload_point(
    frame: &ObservationFrame,
    rules: &Rules,
    objective: [f64; 2],
    from: [f64; 2],
    standoff: f64,
) -> [f64; 2] {
    let span = super::distance(from, objective);
    if span <= standoff {
        return from;
    }
    let toward = [
        (from[0] - objective[0]) / span,
        (from[1] - objective[1]) / span,
    ];
    // Each enemy's reach covers one stretch [near, far] of the line,
    // measured outward from the objective.
    let mut covered: Vec<(f64, f64)> = frame
        .identified
        .iter()
        .filter_map(|enemy| {
            let reach = ground_reach(rules, enemy.kind);
            let off = [
                objective[0] - enemy.position[0],
                objective[1] - enemy.position[1],
            ];
            let b = toward[0] * off[0] + toward[1] * off[1];
            let c = off[0] * off[0] + off[1] * off[1] - reach * reach;
            let disc = b * b - c;
            (reach > 0.0 && disc >= 0.0).then(|| {
                let root = libm::sqrt(disc);
                (-b - root, -b + root)
            })
        })
        .collect();
    covered.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut out = standoff;
    for (near, far) in covered {
        if near <= out && out < far {
            out = far;
        }
    }
    if out >= span {
        return from;
    }
    [
        objective[0] + toward[0] * out,
        objective[1] + toward[1] * out,
    ]
}

/// A carrier with no one aboard, free to collect a squad, and the squad it
/// was sent for and is still driving to, if any: that one is its ride, and
/// no other squad takes it from it.
pub(super) struct FreeCarrier<'a> {
    unit: &'a OwnUnit,
    coming_for: Option<UnitId>,
}

/// Every carrier of the player's free to collect a squad this decision:
/// empty, not withdrawing, and not refused its route. `rides` are the
/// collections the last decision sent; one still under way keeps its squad.
pub(super) fn free_carriers<'a>(
    rules: &Rules,
    frame: &'a ObservationFrame,
    rides: &BTreeMap<UnitId, UnitId>,
) -> Vec<FreeCarrier<'a>> {
    frame
        .own
        .iter()
        .filter(|u| {
            seats(rules, u.kind) > 0
                && !u.withdrawing
                && u.state != contract::observation::MoveState::RouteBlocked
                && passengers(frame, u.id).next().is_none()
        })
        .map(|unit| FreeCarrier {
            unit,
            coming_for: rides.get(&unit.id).copied().filter(|squad| {
                frame.own.iter().any(|u| {
                    u.id == *squad && carrier_of(u).is_none() && coming_for(rules, unit, u)
                })
            }),
        })
        .collect()
}

/// Send the nearest of `free` that seats every living soldier of `squad`
/// and can route to it to collect it (into `collect`, carrier to squad),
/// and stop the squad for it. False, with nothing sent, when none can.
pub(super) fn wait_for_ride(
    rules: &Rules,
    squad: &OwnUnit,
    free: &mut Vec<FreeCarrier>,
    collect: &mut BTreeMap<UnitId, UnitId>,
    orders: &mut Vec<Order>,
    route_time: &mut dyn FnMut(UnitId, [f64; 2]) -> Option<f64>,
) -> bool {
    let Some(carrier) = collector(rules, squad, free, route_time) else {
        return false;
    };
    if squad.goal.is_some() {
        orders.push(Order::Stop {
            units: vec![squad.id],
        });
    }
    collect.insert(carrier, squad.id);
    true
}

/// Whether `carrier` is already driving to `squad`: its move ends within
/// boarding reach of the squad, as a collection's does.
pub(super) fn coming_for(rules: &Rules, carrier: &OwnUnit, squad: &OwnUnit) -> bool {
    carrier.goal.is_some_and(|goal| {
        super::distance(goal, [squad.position[0], squad.position[1]]) <= rules.carriage.reach_m
    })
}

/// The carrier of `free` that collects `squad`, among those that seat every
/// living soldier, can route to it, and were not sent for another squad
/// still waiting: one already coming for it, else the nearest. It leaves
/// `free`.
fn collector(
    rules: &Rules,
    squad: &OwnUnit,
    free: &mut Vec<FreeCarrier>,
    route_time: &mut dyn FnMut(UnitId, [f64; 2]) -> Option<f64>,
) -> Option<UnitId> {
    let here = [squad.position[0], squad.position[1]];
    let mut room: Vec<_> = free
        .iter()
        .enumerate()
        .filter(|(_, c)| {
            seats(rules, c.unit.kind) >= squad.members.len()
                && c.coming_for.is_none_or(|other| other == squad.id)
        })
        .map(|(i, c)| {
            let far = super::distance(here, [c.unit.position[0], c.unit.position[1]]);
            (i, !coming_for(rules, c.unit, squad), far, c.unit.id)
        })
        .collect();
    room.sort_by(|a, b| a.1.cmp(&b.1).then(a.2.total_cmp(&b.2)).then(a.3.cmp(&b.3)));
    let (index, _, _, carrier) = room
        .into_iter()
        .find(|&(.., c)| route_time(c, here).is_some_and(|t| t.is_finite() && t >= 0.0))?;
    free.remove(index);
    Some(carrier)
}

/// The ground enemy a carrier drives its squad toward: the nearest one the
/// squad can engage. With a planned goal, only one nearer that goal than the
/// carrier counts, on the way or at it; a fight off to the side does not
/// pull the carrier from its objective.
pub(super) fn fight_ahead<'a>(
    frame: &'a ObservationFrame,
    rules: &Rules,
    kind: TypeIndex,
    from: [f64; 2],
    goal: Option<[f64; 2]>,
) -> Option<&'a contract::observation::IdentifiedUnit> {
    let at = |t: &contract::observation::IdentifiedUnit| [t.position[0], t.position[1]];
    frame
        .identified
        .iter()
        .filter(|t| rules.catalog.get(t.kind).mobility.layer() == AltitudeLayer::Ground)
        .filter(|t| super::unit_can_engage_target(rules, kind, t.kind))
        .filter(|t| {
            goal.is_none_or(|goal| super::distance(at(t), goal) < super::distance(from, goal))
        })
        .min_by(|a, b| {
            super::distance(from, at(a))
                .total_cmp(&super::distance(from, at(b)))
                .then(a.id.cmp(&b.id))
        })
}

/// The party a carrier drives for: its own, else that of a squad aboard. A
/// carrier and its passengers go to one objective.
pub(super) fn party_aboard<'a>(
    frame: &ObservationFrame,
    carrier: &OwnUnit,
    parties: &'a [super::PartyTask],
) -> Option<&'a super::PartyTask> {
    std::iter::once(carrier.id)
        .chain(passengers(frame, carrier.id).map(|squad| squad.id))
        .find_map(|id| parties.iter().find(|party| party.units.contains(&id)))
}

/// When a squad rides, and where its carrier lets it out.
impl super::BalancedDoctrine {
    /// Whether `unit`, a squad on foot, rides for `goal`: a ride from where
    /// it stands would set it down `ride_min_m` nearer or more.
    pub(super) fn worth_riding(
        &self,
        frame: &ObservationFrame,
        rules: &Rules,
        entry: &EntrySite,
        unit: &contract::observation::OwnUnit,
        goal: [f64; 2],
    ) -> bool {
        if unit.members.is_empty() {
            return false;
        }
        let here = [unit.position[0], unit.position[1]];
        let out = self.drop_point(frame, rules, entry, goal, here);
        distance(here, goal) - distance(out, goal) >= self.tuning.ride_min_m
    }

    /// A loaded carrier's order: drive its squad toward the fight (the
    /// nearest enemy on the ground it can engage) or else its objective, and
    /// let it out at the drop point, or where it is once it is there or in
    /// an enemy's reach. None while anyone boards or leaves, or the order
    /// already stands.
    pub(super) fn unload(
        &self,
        side: Side,
        entry: &EntrySite,
        frame: &ObservationFrame,
        rules: &Rules,
        carrier: &contract::observation::OwnUnit,
        planned_goal: Option<[f64; 2]>,
    ) -> Option<Order> {
        let passenger = passengers(frame, carrier.id).min_by_key(|u| u.id)?;
        if carrier.withdrawing || !all_inside(frame, carrier.id) {
            return None;
        }
        let order = |goal| Order::Unload {
            units: vec![carrier.id],
            goal,
            gesture: frame.tick,
            route: contract::command::RoutePolicy::Fastest,
        };
        // Refused its route, it lets its squad out where it stands.
        if carrier.state == contract::observation::MoveState::RouteBlocked {
            return Some(order(None));
        }
        let here = [carrier.position[0], carrier.position[1]];
        let available = || {
            frame
                .skirmish
                .iter()
                .flat_map(|view| &view.objectives)
                .filter(|o| o.owner != Some(side))
        };
        let objective = planned_goal.or_else(|| {
            available()
                .min_by(|a, b| {
                    distance(here, a.center)
                        .total_cmp(&distance(here, b.center))
                        .then(a.id.cmp(&b.id))
                })
                .map(|objective| objective.center)
        });
        let goal = match fight_ahead(frame, rules, passenger.kind, here, planned_goal) {
            Some(target) => [target.position[0], target.position[1]],
            None => match objective {
                Some(objective) => self.heading(rules, entry, passenger, objective),
                // Nothing to fight and every objective held: out here.
                None => return carrier.goal.is_none().then(|| order(None)),
            },
        };
        let out = self.drop_point(frame, rules, entry, goal, here);
        let slack = self.tuning.reaim_m;
        if distance(here, out) <= slack {
            carrier.goal.is_none().then(|| order(None))
        } else {
            carrier
                .goal
                .is_none_or(|old| distance(old, out) > slack)
                .then(|| order(Some(out)))
        }
    }
}
