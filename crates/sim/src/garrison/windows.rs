//! The windows of each held building: one plan per building, keyed by its
//! structure owner and owned by the battle. A plan is the building's exposed
//! facade bays ([`super::seats_for_state`]), each remembering when a soldier
//! last came to it. It opens when the first squad is seated (later squads take
//! the windows nobody holds) and closes once no living squad holds the
//! building (inside or leaving). A squad's own hold keeps only which window
//! each member stands at ([`Garrison::seats`]).
use std::collections::{BTreeMap, BTreeSet};

use contract::ids::Tick;
use contract::scenario::Rules;

use super::Garrison;
use crate::digest::Digest;
use crate::hold::{ticks, Phase};
use crate::math::V3;
use crate::units::{Soldier, Unit};
use crate::world::{PropId, Slot};

/// A physical facade bay on one floor band.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SeatSlot {
    pub slot: Slot,
    /// Owner-emitted physical exposed edge.
    pub edge: usize,
    pub position: V3,
    /// The tick a soldier last came to this window from another one.
    pub changed: Option<Tick>,
}

pub type SeatPlan = Vec<SeatSlot>;

/// Every held building's window plan.
#[derive(Clone, Debug, Default)]
pub struct Windows {
    plans: BTreeMap<PropId, SeatPlan>,
}

impl Windows {
    /// `building`'s windows; none while nobody holds it.
    pub fn plan(&self, building: PropId) -> &[SeatSlot] {
        self.plans.get(&building).map_or(&[], Vec::as_slice)
    }

    /// The first squad seated in `building` opens its plan. A plan still
    /// there was left by squads since gone, so it is replaced.
    pub(crate) fn open(&mut self, building: PropId, plan: SeatPlan) {
        self.plans.insert(building, plan);
    }

    /// Close the plan of every building no living squad holds.
    pub(crate) fn close_unheld(&mut self, units: &[Unit]) {
        self.plans
            .retain(|&building, _| super::held(units, building));
    }

    /// The squad's building hold and that building's windows while it fights
    /// from it (inside or leaving, not entering).
    fn held<'a>(&'a self, unit: &'a Unit) -> Option<(&'a Garrison, &'a [SeatSlot])> {
        let building = super::garrisoned_in(unit)?;
        Some((unit.hold.as_ref()?.building()?, self.plan(building)))
    }

    /// Member `k`'s window while at the perimeter (inside or leaving).
    pub fn seat<'a>(&'a self, unit: &'a Unit, k: usize) -> Option<&'a SeatSlot> {
        let (g, plan) = self.held(unit)?;
        plan.get((*g.seats.get(k)?)?)
    }

    /// Published body pose: indoors behind the occupied window. Combat keeps
    /// the abstract exposed capsule and muzzle outside the shell, so a missed
    /// occupant still meets the wall without making it target-transparent.
    pub fn body_position(&self, unit: &Unit, member: usize, rules: &Rules) -> V3 {
        let at = unit.members[member].position;
        self.seat(unit, member).map_or(at, |seat| {
            at - (seat.slot.normal * (2.0 * rules.garrison.slot_standoff_m)).with_z(0.0)
        })
    }

    /// A free facade slot for one more soldier of `unit` while it holds its
    /// building (inside or leaving), as [`seat_evenly`] spreads a squad: the
    /// first free slot on the facade its living soldiers hold fewest of.
    /// `None` outside a building, or when every slot is taken.
    pub fn free_seat(&self, unit: &Unit) -> Option<(usize, V3)> {
        let (g, plan) = self.held(unit)?;
        let taken = taken_slots(&g.seats, &unit.members);
        let mut held = [0usize; 4];
        for &s in &taken {
            held[plan[s].slot.facade as usize] += 1;
        }
        plan.iter()
            .enumerate()
            .filter(|(k, _)| !taken.contains(k))
            .min_by_key(|(k, s)| (held[s.slot.facade as usize], *k))
            .map(|(k, s)| (k, s.position))
    }

    /// Whether garrisoned member `k` is still moving to the window he changed
    /// to (`window_swap_s`): he doesn't fire meanwhile.
    pub fn changing_window(&self, unit: &Unit, k: usize, rules: &Rules, tick: Tick) -> bool {
        let swap = ticks(rules.garrison.window_swap_s, rules) as Tick;
        self.seat(unit, k)
            .and_then(|s| s.changed)
            .is_some_and(|t| tick < t + swap)
    }

    /// Whether garrisoned member `k` stands at a slot facing `point`, ready to
    /// fire from it (not still changing windows).
    pub fn faces(&self, unit: &Unit, k: usize, point: V3, rules: &Rules, tick: Tick) -> bool {
        let facing = rules.garrison.slot_facing_min_deg.to_radians();
        !self.changing_window(unit, k, rules, tick)
            && self
                .seat(unit, k)
                .is_some_and(|s| s.slot.faces(point.xy(), facing))
    }

    /// One real occupied seat per directional group, on its highest held band.
    /// Equal-height seats tie by plan order, so eyes never average into a wall.
    pub fn facade_eyes(&self, unit: &Unit, rules: &Rules) -> Vec<V3> {
        let Some((g, plan)) = self.held(unit) else {
            return Vec::new();
        };
        let mut held: [Option<usize>; 4] = [None; 4];
        for (k, _) in unit.members.iter().enumerate().filter(|(_, m)| m.alive()) {
            let Some(index) = g.seats[k] else { continue };
            let seat = &plan[index];
            let group = seat.slot.facade as usize;
            if held[group].is_none_or(|old| {
                seat.position.z > plan[old].position.z
                    || (seat.position.z == plan[old].position.z && index < old)
            }) {
                held[group] = Some(index);
            }
        }
        let lift = crate::math::v3(0.0, 0.0, rules.physics.infantry_eye_m);
        held.into_iter()
            .flatten()
            .map(|index| plan[index].position + lift)
            .collect()
    }

    /// Fold a building hold's place into a digest: its entry, seats and the
    /// building's windows. A squad still stepping in folds no windows.
    pub(crate) fn digest_place(
        &self,
        building: PropId,
        phase: Phase,
        g: &Garrison,
        d: &mut Digest,
    ) {
        let plan = match phase {
            Phase::Entering(_) => &[],
            Phase::Inside | Phase::Exiting(_) => self.plan(building),
        };
        d.f64(g.entry.x).f64(g.entry.y);
        d.u64(plan.len() as u64).u64(g.seats.len() as u64);
        for s in &g.seats {
            d.u64(s.map_or(u64::MAX, |s| s as u64));
        }
        for s in plan {
            d.u64(s.changed.unwrap_or(u64::MAX));
            d.f64(s.position.x).f64(s.position.y).f64(s.position.z);
            if s.edge != s.slot.facade as usize {
                d.bytes(b"physical firing edge").u64(s.edge as u64);
            }
        }
    }

    /// Each garrisoned squad's soldiers change windows to face what their
    /// weapons face. Each soldier requests one weapon's window; its authored
    /// ownership determines priority even when only one rifleman survives.
    /// [`crate::weapons::garrison_aims`] supplies the selected threats.
    /// A soldier already facing stays. Otherwise he takes the
    /// closest facing window that is free, or, failing one, trades places with
    /// the squadmate at the closest facing window who needs it less ([`claim`]):
    /// a lone weapon's operator (the ATGM gunner, the grenadier) displaces a
    /// rifleman, never the reverse. A soldier who changed windows holds his new
    /// one for `window_hold_s`, so alternating threats don't shuffle the squad,
    /// and doesn't fire for `window_swap_s` ([`Self::changing_window`]); nor
    /// does the squadmate he traded with. One-tick relocation, not movement.
    pub fn allocate(
        &mut self,
        units: &mut [Unit],
        aims: &[(usize, crate::weapons::MountAims)],
        rules: &Rules,
        tick: Tick,
    ) {
        let facing = rules.garrison.slot_facing_min_deg.to_radians();
        let window_hold = ticks(rules.garrison.window_hold_s, rules) as Tick;
        for (i, mounts) in aims {
            let unit = &mut units[*i];
            let Some(hold) = unit
                .hold
                .as_mut()
                .filter(|hold| hold.phase == Phase::Inside)
            else {
                continue;
            };
            let building = hold.host.building().expect("building hold");
            let plan = self.plans.get_mut(&building).expect("an open window plan");
            let g = hold.building_mut().expect("building hold");
            let settled = |plan: &[SeatSlot], s: usize| {
                plan[s].changed.is_none_or(|t| tick >= t + window_hold)
            };
            for aim in mounts {
                let point = aim.point;
                let need = need(aim.single_operator);
                for &k in &aim.members {
                    let Some(current) = g.seats[k] else { continue };
                    if plan[current].slot.faces(point.xy(), facing) || !settled(plan, current) {
                        continue;
                    }
                    // Who holds each window: a living squadmate, or nobody.
                    let holder = |s: usize| {
                        (0..unit.members.len())
                            .find(|&h| g.seats[h] == Some(s) && unit.members[h].alive())
                    };
                    let from = plan[current].position;
                    let best = plan
                        .iter()
                        .enumerate()
                        .filter(|(_, slot)| slot.slot.faces(point.xy(), facing))
                        .filter_map(|(s, slot)| match holder(s) {
                            None => Some((false, s, slot)),
                            Some(h)
                                if settled(plan, s) && claim(g, plan, mounts, h, facing) < need =>
                            {
                                Some((true, s, slot))
                            }
                            Some(_) => None,
                        })
                        .min_by(|(a_swap, a, x), (b_swap, b, y)| {
                            a_swap
                                .cmp(b_swap)
                                .then(
                                    (x.position - from)
                                        .length()
                                        .total_cmp(&(y.position - from).length()),
                                )
                                .then(a.cmp(b))
                        })
                        .map(|(_, s, _)| s);
                    let Some(s) = best else { continue };
                    if let Some(h) = holder(s) {
                        g.seats[h] = Some(current);
                        unit.members[h].position = plan[current].position;
                    }
                    g.seats[k] = Some(s);
                    unit.members[k].position = plan[s].position;
                    plan[s].changed = Some(tick);
                    plan[current].changed = Some(tick);
                }
            }
        }
    }
}

/// Seat every living member of squad `i`, stepping into the building whose
/// windows `plan` are, at windows no squad there holds, spreading it evenly
/// round the facades: the next free window of each facade in turn. Nobody
/// already inside moves.
pub(crate) fn seat_evenly(plan: &[SeatSlot], units: &[Unit], i: usize) -> Vec<Option<usize>> {
    let building = super::held_building(&units[i]).expect("a squad stepping into a building");
    let taken: BTreeSet<usize> = (units.iter())
        .filter(|u| u.alive() && super::garrisoned_in(u) == Some(building))
        .filter_map(|u| Some(taken_slots(&u.hold.as_ref()?.building()?.seats, &u.members)))
        .flatten()
        .collect();
    let unit = &units[i];
    let mut facades: [Vec<usize>; 4] = Default::default();
    for (k, s) in plan.iter().enumerate().filter(|(k, _)| !taken.contains(k)) {
        facades[s.slot.facade as usize].push(k);
    }
    let longest = facades.iter().map(Vec::len).max().unwrap_or(0);
    let mut free = (0..longest).flat_map(|j| facades.iter().filter_map(move |f| f.get(j).copied()));
    unit.members
        .iter()
        .map(|s| if s.alive() { free.next() } else { None })
        .collect()
}

/// Slots a squad's living soldiers hold.
fn taken_slots(seats: &[Option<usize>], members: &[Soldier]) -> BTreeSet<usize> {
    seats
        .iter()
        .zip(members)
        .filter(|(_, s)| s.alive())
        .filter_map(|(seat, _)| *seat)
        .collect()
}

/// How strongly the soldier at `k` needs the window he holds: 2 as the only
/// shooter of a weapon aiming out of it, 1 as one of a squad weapon's
/// shooters aiming out of it, 0 when nothing he carries aims through it.
fn claim(
    g: &Garrison,
    plan: &[SeatSlot],
    mounts: &crate::weapons::MountAims,
    k: usize,
    facing: f64,
) -> u8 {
    let Some(seat) = g.seats[k].map(|s| &plan[s]) else {
        return 0;
    };
    mounts
        .iter()
        .filter(|aim| aim.members.contains(&k) && seat.slot.faces(aim.point.xy(), facing))
        .map(|aim| need(aim.single_operator))
        .max()
        .unwrap_or(0)
}

/// How much a mount's shooters need a facing window: its lone operator (an
/// ATGM gunner, a grenadier) more than one of many carriers (the rifles).
fn need(single_operator: bool) -> u8 {
    if single_operator {
        2
    } else {
        1
    }
}
