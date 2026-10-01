//! Buildings as abstract fighting positions (L08–L10, P12). A building takes
//! one squad, whole and within the soldier capacity, after a stationary
//! timer; inside, each soldier stands at a perimeter slot just outside a
//! facade, where its hit capsule and muzzle are; the squad sees from one eye
//! per building-frame direction it holds, at a real seat on the highest occupied
//! band ([`facade_eyes`]). Rounds that miss a slot meet the building's
//! own shell, and any round toward something beyond it meets the shell too:
//! no collider is ever switched off for a target. A collapse leaves a lower,
//! permanent ruin; survivors escape on foot to legal ground nearby, heavily
//! suppressed, or die where they stood. Slots are the one named exception to
//! soldiers as free bodies (Q22): a seated soldier stands at his slot.
//!
//! Garrison state is the unit's own (`Unit::garrison`); the world owns the
//! exposed physical geometry; this module seats squads and `structures` holds integrity, one row of every
//! destroyable prop's.
use std::collections::BTreeSet;

use contract::command::OrderError;
use contract::ids::{Side, Tick, UnitId};
use contract::map::MoverClass;
use contract::observation::{GarrisonPhase, GarrisonState, MoveState};
use contract::scenario::Rules;

use crate::arrangement;
use crate::digest::Digest;
use crate::math::{v2, V2, V3};
use crate::rng::Rng;
use crate::units::{Soldier, Unit, UnitOrder};
use crate::world::{Prop, PropId, Slot, WorldGeometry};

/// Escaping soldiers keep at least this far apart.
const ESCAPE_SPACING_M: f64 = 1.0;
/// Exit places are sampled this far apart around the building.
const EXIT_STEP_M: f64 = 2.0;
/// A leaving squad's middle stands this far beyond the building's walls.
const EXIT_CLEARANCE_M: f64 = 1.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Stationary beside the building for this many ticks.
    Entering(u32),
    Inside,
    /// Stationary inside, leaving, for this many ticks.
    Exiting(u32),
}

/// A squad's hold on one building.
#[derive(Clone, Debug)]
pub struct Garrison {
    pub building: PropId,
    pub phase: Phase,
    /// The building's exposed bay slots at their floor heights.
    pub slots: SeatPlan,
    /// Each member's slot index while inside.
    pub seats: Vec<Option<usize>>,
    /// Where the squad stood to enter: it leaves that way unless heading elsewhere.
    pub entry: V2,
}

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

impl Garrison {
    /// Member `k`'s slot while at the perimeter (inside or leaving).
    pub fn seat(&self, k: usize) -> Option<&SeatSlot> {
        match self.phase {
            Phase::Inside | Phase::Exiting(_) => self.slots.get((*self.seats.get(k)?)?),
            Phase::Entering(_) => None,
        }
    }
}

fn ticks(seconds: f64, rules: &Rules) -> u32 {
    ((seconds * rules.tick_hz as f64).round() as u32).max(1)
}

/// The one band policy used by seating and building bulk integrity.
pub(crate) fn floor_band_count(geometry: &contract::templates::MaterializedBuilding) -> usize {
    geometry
        .floor_z
        .as_ref()
        .map_or(1, |floors| floors.len().min(3))
}

/// Seats at exposed physical bays on the bottom three floor bands. The
/// gameplay cap reserves places round the four building-frame directions.
pub fn building_seats(building: &contract::map::BuildingDefinition, rules: &Rules) -> SeatPlan {
    seat_plan(&building.geometry, rules, None)
}

/// A remembered or live replacement keeps the source bays but loses floors
/// above its remaining shell. Initial-part IDs retain the complete source plan.
fn seats_for_state(world: &WorldGeometry, prop: &Prop, rules: &Rules) -> SeatPlan {
    let definition = world.building(prop.id).expect("building geometry");
    // Into states give every part the same height. Gutted parts may differ,
    // but the catalog forbids garrisoning those terminal shells.
    let remaining_height =
        (!definition.parts.iter().any(|p| p.prop == prop.id)).then_some(prop.half.z * 2.0);
    seat_plan(&definition.geometry, rules, remaining_height)
}

fn seat_plan(
    geometry: &contract::templates::MaterializedBuilding,
    rules: &Rules,
    remaining_height: Option<f64>,
) -> SeatPlan {
    let mut groups: [Vec<SeatSlot>; 4] = Default::default();
    let ground = [geometry.frame.translation[2]];
    let floors = geometry.floor_z.as_deref().unwrap_or(&ground);
    for &z in floors.iter().take(floor_band_count(geometry)).rev() {
        for (ordinal, edge) in geometry.edges.iter().enumerate().filter(|(_, e)| {
            let top = remaining_height.map_or(e.top_z, |h| e.top_z.min(e.base_z + h));
            e.exposed && z >= e.base_z && z < top
        }) {
            if geometry.floor_z.is_some() && edge.bays.is_none() {
                continue;
            }
            let part = geometry
                .parts
                .iter()
                .find(|part| part.id == edge.part)
                .unwrap();
            let direction = v2(edge.normal[0], edge.normal[1]).rotated(-geometry.frame.yaw);
            let group = ((direction.y.atan2(direction.x) / std::f64::consts::FRAC_PI_2).round()
                as i32)
                .rem_euclid(4) as usize;
            // Legacy boxes without floor/bay facts retain ground positions;
            // known-floor descriptors cannot invent unresolved source bays.
            let bays = edge.bays.clone().unwrap_or_else(|| {
                let count = ((edge.span_m[1] - edge.span_m[0]) / 3.0).floor() as usize;
                let (normal, along, reach, _) = edge.facade.axes(part.half_extents);
                (0..count)
                    .map(|j| {
                        let t = edge.span_m[0]
                            + (edge.span_m[1] - edge.span_m[0]) * (j as f64 + 0.5) / count as f64;
                        let p = v2(part.center[0], part.center[1])
                            + (v2(normal[0], normal[1]) * reach + v2(along[0], along[1]) * t)
                                .rotated(part.yaw);
                        [p.x, p.y]
                    })
                    .collect()
            });
            for bay in bays {
                let normal = v2(edge.normal[0], edge.normal[1]);
                let position = v2(bay[0], bay[1]) + normal * rules.garrison.slot_standoff_m;
                groups[group].push(SeatSlot {
                    slot: Slot {
                        position,
                        normal,
                        facade: group as u8,
                    },
                    edge: ordinal,
                    position: position.with_z(z),
                    changed: None,
                });
            }
        }
    }
    let capacity = (rules.buildings.capacity_soldiers as usize).min(32);
    let longest = groups.iter().map(Vec::len).max().unwrap_or(0);
    let mut counts = [0usize; 4];
    let available = &groups;
    for f in (0..longest)
        .flat_map(|j| (0..4).filter(move |&f| j < available[f].len()))
        .take(capacity)
    {
        counts[f] += 1;
    }
    let mut selected = Vec::new();
    for (f, group) in groups.iter().enumerate() {
        for j in 0..counts[f] {
            selected.push(group[(2 * j + 1) * group.len() / (2 * counts[f])]);
        }
    }
    selected
}

/// A standing body a squad can garrison (its row's `garrison`), or `None`.
fn building(world: &WorldGeometry, id: PropId) -> Option<&Prop> {
    world
        .prop(world.structure_owner(id)?)
        .filter(|p| p.body.garrison)
}

/// Approach the nearest exposed physical part using remembered poses. Box
/// corner arithmetic is retained, while internal faces cannot admit entry.
pub(crate) fn approach(
    world: &WorldGeometry,
    known: &crate::movement::SideGeometry,
    authored: PropId,
    owner: PropId,
    from: V2,
    standoff: f64,
) -> Option<V2> {
    let definition = world.building(owner)?;
    let parts: Vec<_> = world
        .structure_parts(owner)
        .into_iter()
        .filter_map(|id| known.prop(world, authored, id))
        .collect();
    let mut candidates = Vec::new();
    for part in &parts {
        let source = world.authored_prop(part.id)?;
        let reference = definition.parts.iter().find(|p| p.prop == source)?;
        let edges: Vec<_> = definition
            .geometry
            .edges
            .iter()
            .filter(|e| e.part == reference.part && e.exposed)
            .collect();
        let candidate = part.exterior_point(from, standoff);
        let local = part.footprint().to_local(candidate);
        let exposed = edges.iter().any(|e| {
            let (normal, along, reach, half) =
                e.facade.axes([part.half.x, part.half.y, part.half.z]);
            let n = v2(normal[0], normal[1]);
            let a = v2(along[0], along[1]);
            let t = local.dot(a).clamp(-half, half);
            local.dot(n) >= reach && t >= e.span_m[0] && t <= e.span_m[1]
        });
        if exposed && !parts.iter().any(|p| p.footprint().contains(candidate, 0.0)) {
            candidates.push(candidate);
            continue;
        }
        for e in edges {
            let (normal, along, reach, _) = e.facade.axes([part.half.x, part.half.y, part.half.z]);
            let n = v2(normal[0], normal[1]);
            let a = v2(along[0], along[1]);
            let t = part
                .footprint()
                .to_local(from)
                .dot(a)
                .clamp(e.span_m[0], e.span_m[1]);
            let candidate = part.center + (n * (reach + standoff) + a * t).rotated(part.yaw);
            if !parts.iter().any(|p| p.footprint().contains(candidate, 0.0)) {
                candidates.push(candidate)
            }
        }
    }
    candidates
        .into_iter()
        .min_by(|a, b| (*a - from).length().total_cmp(&(*b - from).length()))
}

/// Whether a squad other than the one at `i`, of either side, holds
/// `building` (inside or leaving): a building takes one squad.
fn held_by_another(units: &[Unit], building: PropId, i: usize) -> bool {
    units.iter().enumerate().any(|(j, u)| {
        j != i
            && u.alive()
            && u.garrisoned()
            && u.garrison.as_ref().is_some_and(|g| g.building == building)
    })
}

/// Command check (L08): infantry only, a building the side knows, one squad per
/// building, and that squad fitting whole. The building is refused while
/// another of the side's squads holds it, is entering it, has an order to
/// (queued included), or was ordered in earlier this tick (`claimed`). Only
/// the side's own units are counted here: a squad that finds an enemy squad
/// holding it gives up its order where it stands ([`advance`]).
pub fn validate(
    world: &WorldGeometry,
    units: &[Unit],
    side: Side,
    ordered: &[UnitId],
    target: Option<Prop>,
    claimed: bool,
    rules: &Rules,
) -> Result<(), OrderError> {
    let target_prop = target
        .filter(|p| p.body.garrison)
        .ok_or(OrderError::NotABuilding)?;
    let capacity = seats_for_state(world, &target_prop, rules).len();
    let target = target_prop.id;
    // The command already checked each named unit is the side's own and alive.
    let [id] = ordered else {
        return Err(OrderError::OneSquadPerBuilding);
    };
    let squad = &units[id.0 as usize];
    if squad.is_vehicle() {
        return Err(OrderError::NotInfantry { unit: *id });
    }
    let heading_in = |u: &Unit| {
        u.garrison.as_ref().is_some_and(|g| g.building == target)
            || u.orders
                .iter()
                .any(|o| matches!(o, UnitOrder::Garrison { building, .. } if *building == target))
    };
    let taken = units
        .iter()
        .any(|u| u.side == side && u.alive() && u.id != *id && heading_in(u));
    if claimed || taken {
        return Err(OrderError::BuildingOccupied);
    }
    let living = squad.members.iter().filter(|s| s.alive()).count();
    if living > capacity {
        return Err(OrderError::CapacityFull);
    }
    Ok(())
}

/// What stops a soldier on foot: every prop that blocks infantry.
fn solid(p: &Prop) -> bool {
    p.blocks(MoverClass::Infantry)
}

/// Where a leaving squad's soldiers stand: the place around the building
/// nearest `preferred` where the whole squad finds a spread-out arrangement
/// (deterministic, ties by perimeter order), one spot per living member.
fn exit_spots(
    world: &WorldGeometry,
    prop: &Prop,
    unit: &Unit,
    preferred: V2,
    rules: &Rules,
    rng: &mut Rng,
) -> Option<Vec<V2>> {
    // An orderly exit sets the squad down where it can march on from: its
    // place and every soldier's spot keep the squad's path clearance (its
    // navigation half width) from walls, not merely a soldier's radius, so
    // the routes it plans from there start on open ground.
    let radius = unit.mobility.half_width_m;
    let reach = radius + EXIT_CLEARANCE_M;
    let mut candidates = Vec::new();
    for prop in world
        .structure_parts(prop.id)
        .into_iter()
        .filter_map(|id| world.prop(id))
    {
        let (hx, hy) = (prop.half.x + reach, prop.half.y + reach);
        let corners = [
            v2(hx, -hy),
            v2(hx, hy),
            v2(-hx, hy),
            v2(-hx, -hy),
            v2(hx, -hy),
        ];
        for w in corners.windows(2) {
            let n = ((w[1] - w[0]).length() / EXIT_STEP_M).ceil() as usize;
            for k in 0..n {
                let local = w[0] + (w[1] - w[0]) * (k as f64 / n as f64);
                candidates.push(prop.center + local.rotated(prop.yaw));
            }
        }
    }
    let mut order: Vec<usize> = (0..candidates.len()).collect();
    order.sort_by(|&a, &b| {
        (candidates[a] - preferred)
            .length()
            .total_cmp(&(candidates[b] - preferred).length())
            .then(a.cmp(&b))
    });
    let living = unit.members.iter().filter(|s| s.alive()).count();
    let im = &rules.infantry_movement;
    let diameter = arrangement::spread(im, living);
    order.into_iter().map(|i| candidates[i]).find_map(|c| {
        if !arrangement::standing_room(world, c, radius, &solid) {
            return None;
        }
        arrangement::arrange(
            world,
            c,
            living,
            diameter,
            im.spacing_m,
            radius,
            &solid,
            rng,
        )
    })
}

/// Seat every living member, spreading the squad evenly around the facades:
/// the next slot of each facade in turn.
fn seat_evenly(slots: &[SeatSlot], unit: &Unit) -> Vec<Option<usize>> {
    let mut facades: [Vec<usize>; 4] = Default::default();
    for (i, s) in slots.iter().enumerate() {
        facades[s.slot.facade as usize].push(i);
    }
    let longest = facades.iter().map(Vec::len).max().unwrap_or(0);
    let mut free = (0..longest).flat_map(|j| facades.iter().filter_map(move |f| f.get(j).copied()));
    unit.members
        .iter()
        .map(|s| if s.alive() { free.next() } else { None })
        .collect()
}

/// A free facade slot for one more soldier of `unit` while it holds its building
/// (inside or leaving), as `seat_evenly` spreads a squad: the first free slot
/// on the facade its living soldiers hold fewest of. `None` outside a
/// building, or when every slot is taken.
pub fn free_seat(unit: &Unit) -> Option<(usize, V3)> {
    let g = unit.garrison.as_ref().filter(|_| unit.garrisoned())?;
    let taken = taken_slots(&g.seats, &unit.members);
    let mut held = [0usize; 4];
    for &s in &taken {
        held[g.slots[s].slot.facade as usize] += 1;
    }
    g.slots
        .iter()
        .enumerate()
        .filter(|(k, _)| !taken.contains(k))
        .min_by_key(|(k, s)| (held[s.slot.facade as usize], *k))
        .map(|(k, s)| (k, s.position))
}

/// Slots the squad's living soldiers hold: it is the building's one occupant.
fn taken_slots(seats: &[Option<usize>], members: &[Soldier]) -> BTreeSet<usize> {
    seats
        .iter()
        .zip(members)
        .filter(|(_, s)| s.alive())
        .filter_map(|(seat, _)| *seat)
        .collect()
}

/// What the squad's front order wants of buildings.
enum Want {
    Enter(PropId),
    /// Leave: an exit order, or one that needs to move.
    Leave,
    Stay,
}

fn want(unit: &Unit) -> Want {
    match unit.orders.front() {
        Some(UnitOrder::Garrison { building, .. }) => Want::Enter(*building),
        Some(UnitOrder::Exit) => Want::Leave,
        Some(o) if o.movement().is_some() => Want::Leave,
        _ => Want::Stay,
    }
}

/// Advance every squad's garrison one tick: start entering beside the
/// ordered building, seat the squad when the timer ends unless another holds it,
/// leave after the exit timer. Transitions are stationary: a squad with a
/// building has no movement goal. `seed` and `tick` fix the arrangement a
/// leaving squad spreads into.
pub fn advance(
    world: &WorldGeometry,
    sides: &[crate::movement::SideGeometry; 2],
    authored: PropId,
    units: &mut [Unit],
    rules: &Rules,
    seed: u64,
    tick: Tick,
) {
    let timer = ticks(rules.garrison.enter_exit_s, rules);
    for i in 0..units.len() {
        if !units[i].alive() {
            continue;
        }
        let want = want(&units[i]);
        let phase = units[i].garrison.as_ref().map(|g| (g.building, g.phase));
        match (phase, want) {
            (None, Want::Enter(b)) => {
                let Some(_) = sides[units[i].side.index()]
                    .prop(world, authored, b)
                    .filter(|p| p.body.garrison)
                else {
                    units[i].orders.pop_front(); // its fall was discovered
                    continue;
                };
                if world
                    .structure_parts(b)
                    .into_iter()
                    .filter_map(|id| sides[units[i].side.index()].prop(world, authored, id))
                    .any(|p| {
                        p.footprint()
                            .contains(units[i].position.xy(), rules.garrison.entry_distance_m)
                    })
                {
                    let unit = &mut units[i];
                    unit.route = None;
                    unit.planned_goal = None;
                    unit.state = MoveState::Idle;
                    unit.garrison = Some(Garrison {
                        building: b,
                        phase: Phase::Entering(1),
                        slots: Vec::new(),
                        seats: Vec::new(),
                        entry: unit.position.xy(),
                    });
                }
            }
            (None, Want::Leave) if matches!(units[i].orders.front(), Some(UnitOrder::Exit)) => {
                units[i].orders.pop_front(); // nothing to leave
            }
            (None, _) => {}
            (Some((b, Phase::Entering(_))), w)
                if !matches!(w, Want::Enter(x) if x == b) || building(world, b).is_none() =>
            {
                // A new order (or Stop) cancels entering where the squad stands.
                units[i].garrison = None;
                if matches!(units[i].orders.front(), Some(UnitOrder::Exit)) {
                    units[i].orders.pop_front();
                }
            }
            (Some((_, Phase::Entering(n))), _) if n < timer => {
                units[i].garrison.as_mut().unwrap().phase = Phase::Entering(n + 1);
            }
            (Some((b, Phase::Entering(_))), _) => {
                // Another squad (an enemy's) got in first: the order lapses
                // and the squad stands where it is.
                if held_by_another(units, b, i) {
                    units[i].garrison = None;
                    units[i].orders.pop_front();
                    continue;
                }
                let entry = units[i].garrison.as_ref().unwrap().entry;
                let prop = building(world, b).expect("standing building");
                let slots = seats_for_state(world, prop, rules);
                // Supply may restore soldiers during the entry timer. Admission
                // still belongs to the whole living squad when it takes seats.
                if units[i].members.iter().filter(|s| s.alive()).count() > slots.len() {
                    units[i].garrison = None;
                    units[i].orders.pop_front();
                    continue;
                }
                let seats = seat_evenly(&slots, &units[i]);
                let unit = &mut units[i];
                for (s, seat) in unit.members.iter_mut().zip(&seats) {
                    if let Some(k) = seat {
                        s.position = slots[*k].position;
                    }
                }
                unit.settle();
                unit.garrison = Some(Garrison {
                    building: b,
                    phase: Phase::Inside,
                    slots,
                    seats,
                    entry,
                });
                unit.orders.pop_front();
            }
            (Some((b, Phase::Inside)), Want::Enter(x)) if x == b => {
                units[i].orders.pop_front(); // already there
            }
            (Some((_, Phase::Inside)), Want::Enter(_) | Want::Leave) => {
                units[i].garrison.as_mut().unwrap().phase = Phase::Exiting(1);
            }
            (Some((_, Phase::Inside)), Want::Stay) => {}
            (Some((_, Phase::Exiting(_))), Want::Stay) => {
                // Stop (or an order that stays) keeps the squad inside.
                units[i].garrison.as_mut().unwrap().phase = Phase::Inside;
            }
            (Some((_, Phase::Exiting(n))), _) if n < timer => {
                units[i].garrison.as_mut().unwrap().phase = Phase::Exiting(n + 1);
            }
            (Some((b, Phase::Exiting(_))), _) => {
                let unit = &units[i];
                let prop = building(world, b).expect("occupied building stands");
                let entry = unit.garrison.as_ref().unwrap().entry;
                let preferred = match unit.orders.front() {
                    Some(UnitOrder::Garrison { approach, .. }) => *approach,
                    Some(o) => o.movement().map_or(entry, |m| m.destination),
                    None => entry,
                };
                // No room outside: the squad stays inside and tries again.
                let mut draws = arrangement::rng(seed, unit.id.0, tick);
                if let Some(spots) = exit_spots(world, prop, unit, preferred, rules, &mut draws) {
                    let unit = &mut units[i];
                    let living = unit.members.iter_mut().filter(|s| s.alive());
                    for (s, p) in living.zip(spots) {
                        let z = world.surface_at(p.x, p.y).map_or(s.position.z, |g| g.z);
                        s.position = p.with_z(z);
                    }
                    unit.settle();
                    // Placed out of the building: the squad holds round there.
                    unit.anchor = Some(crate::cover::Anchor {
                        at: unit.position.xy(),
                        halt: false,
                    });
                    unit.garrison = None;
                    if matches!(unit.orders.front(), Some(UnitOrder::Exit)) {
                        unit.orders.pop_front();
                    }
                }
            }
        }
    }
}

/// How strongly the soldier at `k` needs the window he holds: 2 as the only
/// shooter of a weapon aiming out of it, 1 as one of a squad weapon's
/// shooters aiming out of it, 0 when nothing he carries aims through it.
fn claim(g: &Garrison, mounts: &crate::weapons::MountAims, k: usize, facing: f64) -> u8 {
    let Some(seat) = g.seats[k].map(|s| &g.slots[s]) else {
        return 0;
    };
    mounts
        .iter()
        .filter(|(participants, point)| {
            participants.contains(&k) && seat.slot.faces(point.xy(), facing)
        })
        .map(|(participants, _)| need(participants))
        .max()
        .unwrap_or(0)
}

/// How much a mount's shooters need a facing window: its lone operator (an
/// ATGM gunner, a grenadier) more than one of many carriers (the rifles).
fn need(participants: &[usize]) -> u8 {
    if participants.len() == 1 {
        2
    } else {
        1
    }
}

/// Each garrisoned squad's soldiers change windows to face what their
/// weapons face: `aims` holds, per squad (unit index), each mount's shooters
/// (every carrier of a squad weapon, else its operator) and the point it
/// faces, its lock or the threat it watches ([`crate::weapons::garrison_aims`]).
/// A soldier already facing stays. Otherwise he takes the
/// closest facing window that is free, or, failing one, trades places with
/// the squadmate at the closest facing window who needs it less ([`claim`]):
/// a lone weapon's operator (the ATGM gunner, the grenadier) displaces a
/// rifleman, never the reverse. A soldier who changed windows holds his new
/// one for `window_hold_s`, so alternating threats don't shuffle the squad,
/// and doesn't fire for `window_swap_s` ([`changing_window`]); nor does the
/// squadmate he traded with. One-tick relocation, not movement.
pub fn allocate_slots(
    units: &mut [Unit],
    aims: &[(usize, crate::weapons::MountAims)],
    rules: &Rules,
    tick: Tick,
) {
    let facing = rules.garrison.slot_facing_min_deg.to_radians();
    let hold = ticks(rules.garrison.window_hold_s, rules) as Tick;
    for (i, mounts) in aims {
        let unit = &mut units[*i];
        let Some(g) = unit.garrison.as_mut().filter(|g| g.phase == Phase::Inside) else {
            continue;
        };
        let settled = |g: &Garrison, s: usize| g.slots[s].changed.is_none_or(|t| tick >= t + hold);
        for (participants, point) in mounts {
            let need = need(participants);
            for &k in participants {
                let Some(current) = g.seats[k] else { continue };
                if g.slots[current].slot.faces(point.xy(), facing) || !settled(g, current) {
                    continue;
                }
                // Who holds each window: a living squadmate, or nobody.
                let holder = |s: usize| {
                    (0..unit.members.len())
                        .find(|&h| g.seats[h] == Some(s) && unit.members[h].alive())
                };
                let from = g.slots[current].position;
                let best = g
                    .slots
                    .iter()
                    .enumerate()
                    .filter(|(_, slot)| slot.slot.faces(point.xy(), facing))
                    .filter_map(|(s, slot)| match holder(s) {
                        None => Some((false, s, slot)),
                        Some(h) if settled(g, s) && claim(g, mounts, h, facing) < need => {
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
                    unit.members[h].position = g.slots[current].position;
                }
                g.seats[k] = Some(s);
                unit.members[k].position = g.slots[s].position;
                g.slots[s].changed = Some(tick);
                g.slots[current].changed = Some(tick);
            }
        }
    }
}

/// Whether garrisoned member `k` is still moving to the window he changed
/// to (`window_swap_s`): he doesn't fire meanwhile.
pub fn changing_window(unit: &Unit, k: usize, rules: &Rules, tick: Tick) -> bool {
    let swap = ticks(rules.garrison.window_swap_s, rules) as Tick;
    unit.garrison
        .as_ref()
        .and_then(|g| g.seat(k))
        .and_then(|s| s.changed)
        .is_some_and(|t| tick < t + swap)
}

/// One real occupied seat per directional group, on its highest held band.
/// Equal-height seats tie by plan order, so eyes never average into a wall.
pub fn facade_eyes(unit: &Unit, rules: &Rules) -> Vec<V3> {
    let Some(g) = unit.garrison.as_ref().filter(|_| unit.garrisoned()) else {
        return Vec::new();
    };
    let mut held: [Option<usize>; 4] = [None; 4];
    for (k, _) in unit.members.iter().enumerate().filter(|(_, m)| m.alive()) {
        let Some(index) = g.seats[k] else { continue };
        let seat = &g.slots[index];
        let group = seat.slot.facade as usize;
        if held[group].is_none_or(|old| {
            seat.position.z > g.slots[old].position.z
                || (seat.position.z == g.slots[old].position.z && index < old)
        }) {
            held[group] = Some(index);
        }
    }
    let lift = crate::math::v3(0.0, 0.0, rules.physics.infantry_eye_m);
    held.into_iter()
        .flatten()
        .map(|index| g.slots[index].position + lift)
        .collect()
}

/// Building cover strength a unit has from where it is observed: its
/// perimeter slots while garrisoned, otherwise none.
pub fn shelter(unit: &Unit, rules: &Rules) -> f64 {
    if unit.garrisoned() {
        rules.buildings.cover_strength
    } else {
        0.0
    }
}

/// The building a garrisoned unit's soldiers stand against.
pub fn shell(unit: &Unit) -> Option<PropId> {
    unit.garrison
        .as_ref()
        .filter(|_| unit.garrisoned())
        .map(|g| g.building)
}

/// Whether garrisoned member `k` stands at a slot facing `point`, ready to
/// fire from it (not still changing windows).
pub fn faces(unit: &Unit, k: usize, point: V3, rules: &Rules, tick: Tick) -> bool {
    let facing = rules.garrison.slot_facing_min_deg.to_radians();
    !changing_window(unit, k, rules, tick)
        && unit
            .garrison
            .as_ref()
            .and_then(|g| g.seat(k))
            .is_some_and(|s| s.slot.faces(point.xy(), facing))
}

/// A building collapsed (L10) and its ruin stands in its place: each
/// occupant survives with the configured probability and escapes on foot to
/// the nearest legal free ground within the local search, carrying heavy
/// suppression; one with no legal escape dies where it stood. Squads still
/// entering stand outside and are unharmed. Returns units destroyed.
pub fn collapse(
    world: &WorldGeometry,
    units: &mut [Unit],
    target: PropId,
    rules: &Rules,
    rng: &mut Rng,
    tick: Tick,
) -> Vec<UnitId> {
    let g = &rules.garrison;
    let mut taken: Vec<V2> = Vec::new();
    let mut destroyed = Vec::new();
    for unit in units.iter_mut() {
        if unit.garrison.as_ref().is_none_or(|g| g.building != target) {
            continue;
        }
        if !unit.garrisoned() {
            unit.garrison = None; // still outside: its order lapses next tick
            continue;
        }
        let mut out: Vec<(usize, V2)> = Vec::new();
        for k in 0..unit.members.len() {
            if !unit.members[k].alive() {
                continue;
            }
            let at = unit.members[k].position;
            let lives = rng.unit() < g.survival_probability_on_collapse;
            let from = at.xy();
            // He scrambles out as one body: wherever a soldier's disc
            // fits, not where the squad's path clearance does.
            let radius = rules.physics.soldier_radius_m;
            let place = lives
                .then(|| {
                    arrangement::nearest_free(from, g.exit_search_radius_m, |p| {
                        taken.iter().all(|t| (*t - p).length() >= ESCAPE_SPACING_M)
                            && arrangement::standing_room(world, p, radius, &solid)
                            && arrangement::reachable(world, from, p, radius, &solid)
                    })
                })
                .flatten();
            match place {
                Some(p) => {
                    taken.push(p);
                    out.push((k, p));
                }
                None => {
                    unit.members[k].fall(at, unit.yaw);
                }
            }
        }
        unit.garrison = None;
        if matches!(unit.orders.front(), Some(UnitOrder::Exit)) {
            unit.orders.pop_front();
        }
        if out.is_empty() {
            unit.orders.clear();
            unit.route = None;
            destroyed.push(unit.id);
            continue;
        }
        // Each survivor stands where he escaped to.
        for (k, p) in out {
            let z = world.surface_at(p.x, p.y).map_or(0.0, |s| s.z);
            unit.members[k].position = p.with_z(z);
        }
        unit.settle();
        // They come out pinned: the level at least reaches the pinned tier's.
        unit.suppression = unit.suppression.max(rules.suppression.pinned.level);
        unit.suppressed_at = tick;
    }
    destroyed
}

/// What the owning side sees of a squad's garrison.
pub fn state(world: &WorldGeometry, unit: &Unit, rules: &Rules) -> Option<GarrisonState> {
    let g = unit.garrison.as_ref()?;
    let footprint = world.structure_footprint(g.building)?;
    let timer = ticks(rules.garrison.enter_exit_s, rules) as f64;
    let (phase, progress) = match g.phase {
        Phase::Entering(n) => (GarrisonPhase::Entering, n as f64 / timer),
        Phase::Inside => (GarrisonPhase::Inside, 1.0),
        Phase::Exiting(n) => (GarrisonPhase::Exiting, n as f64 / timer),
    };
    Some(GarrisonState {
        building: g.building,
        phase,
        progress,
        center: [footprint.center.x, footprint.center.y],
        half: [footprint.half.x, footprint.half.y],
    })
}

/// Fold a unit's garrison into a digest, with presence and length tags.
pub fn digest(unit: &Unit, d: &mut Digest) {
    d.u64(unit.garrison.is_some() as u64);
    if let Some(g) = &unit.garrison {
        let (tag, n) = match g.phase {
            Phase::Entering(n) => (0, n),
            Phase::Inside => (2, 0),
            Phase::Exiting(n) => (3, n),
        };
        d.u64(g.building as u64).u64(tag).u64(n as u64);
        d.f64(g.entry.x).f64(g.entry.y);
        d.u64(g.slots.len() as u64).u64(g.seats.len() as u64);
        for s in &g.seats {
            d.u64(s.map_or(u64::MAX, |s| s as u64));
        }
        for s in &g.slots {
            d.u64(s.changed.unwrap_or(u64::MAX));
            d.f64(s.position.x).f64(s.position.y).f64(s.position.z);
            if s.edge != s.slot.facade as usize {
                d.bytes(b"physical firing edge").u64(s.edge as u64);
            }
        }
    }
}
