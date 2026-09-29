//! Buildings as abstract fighting positions (L08–L10, P12). A building takes
//! one squad, whole and within the soldier capacity, after a stationary
//! timer; inside, each soldier stands at a perimeter slot just outside a
//! facade, where its hit capsule and muzzle are; the squad sees from one eye
//! per facade it holds ([`facade_eyes`]). Rounds that miss a slot meet the building's
//! own shell, and any round toward something beyond it meets the shell too:
//! no collider is ever switched off for a target. A collapse leaves a lower,
//! permanent ruin; survivors escape on foot to legal ground nearby, heavily
//! suppressed, or die where they stood. Slots are the one named exception to
//! soldiers as free bodies (Q22): a seated soldier stands at his slot.
//!
//! Garrison state is the unit's own (`Unit::garrison`); the world owns the
//! facade slots; `structures` holds building integrity, one row of every
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
    /// The building's perimeter slots at ground height, from the world.
    pub slots: Vec<SeatSlot>,
    /// Each member's slot index while inside.
    pub seats: Vec<Option<usize>>,
    /// Where the squad stood to enter: it leaves that way unless heading elsewhere.
    pub entry: V2,
}

/// A world facade slot standing on the ground.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SeatSlot {
    pub slot: Slot,
    pub position: V3,
}

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

/// The building's perimeter slots, capacity reserved evenly around its facades.
pub fn slots(world: &WorldGeometry, building: &Prop, rules: &Rules) -> Vec<SeatSlot> {
    building
        .facade_slots(
            rules.buildings.capacity_soldiers as usize,
            rules.garrison.slot_standoff_m,
        )
        .into_iter()
        .map(|slot| SeatSlot {
            slot,
            position: slot.position.with_z(
                world
                    .height_at(slot.position.x, slot.position.y)
                    .unwrap_or(0.0),
            ),
        })
        .collect()
}

/// A standing body a squad can garrison (its row's `garrison`), or `None`.
fn building(world: &WorldGeometry, id: PropId) -> Option<&Prop> {
    world.prop(id).filter(|p| p.body.garrison)
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

/// Command check (L08): infantry only, a standing building, one squad per
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
    target: PropId,
    claimed: bool,
    rules: &Rules,
) -> Result<(), OrderError> {
    if building(world, target).is_none() {
        return Err(OrderError::NotABuilding);
    }
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
    if living > rules.buildings.capacity_soldiers as usize {
        return Err(OrderError::CapacityFull);
    }
    Ok(())
}

/// Where a squad at `from` walks to enter: beside the nearest facade.
pub fn approach(world: &WorldGeometry, target: PropId, from: V2, rules: &Rules) -> Option<V2> {
    let prop = building(world, target)?;
    Some(prop.exterior_point(from, rules.garrison.entry_distance_m / 2.0))
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
    let (hx, hy) = (prop.half.x + reach, prop.half.y + reach);
    let corners = [
        v2(hx, -hy),
        v2(hx, hy),
        v2(-hx, hy),
        v2(-hx, -hy),
        v2(hx, -hy),
    ];
    let mut candidates = Vec::new();
    for w in corners.windows(2) {
        let n = ((w[1] - w[0]).length() / EXIT_STEP_M).ceil() as usize;
        for k in 0..n {
            let local = w[0] + (w[1] - w[0]) * (k as f64 / n as f64);
            candidates.push(prop.center + local.rotated(prop.yaw));
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
pub fn advance(world: &WorldGeometry, units: &mut [Unit], rules: &Rules, seed: u64, tick: Tick) {
    let timer = ticks(rules.garrison.enter_exit_s, rules);
    for i in 0..units.len() {
        if !units[i].alive() {
            continue;
        }
        let want = want(&units[i]);
        let phase = units[i].garrison.as_ref().map(|g| (g.building, g.phase));
        match (phase, want) {
            (None, Want::Enter(b)) => {
                let Some(prop) = building(world, b) else {
                    units[i].orders.pop_front(); // it fell meanwhile
                    continue;
                };
                if prop
                    .footprint()
                    .contains(units[i].position.xy(), rules.garrison.entry_distance_m)
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
                let slots = slots(world, prop, rules);
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

/// Each garrisoned squad's soldiers move to slots facing what their weapons
/// aim at: `aims` holds, per squad (unit index), each locked mount's
/// participation (`true` for every soldier, else the operator) and its
/// observed aim point. A soldier already facing stays; otherwise it takes the
/// closest free facing slot (stable index ties), or waits if none is free.
/// One-tick relocation, not movement.
pub fn allocate_slots(
    units: &mut [Unit],
    aims: &[(usize, crate::weapons::MountAims)],
    rules: &Rules,
) {
    let facing = rules.garrison.slot_facing_min_deg.to_radians();
    for (i, mounts) in aims {
        let unit = &mut units[*i];
        let Some(g) = unit.garrison.as_mut().filter(|g| g.phase == Phase::Inside) else {
            continue;
        };
        for (participants, point) in mounts {
            for &k in participants {
                let taken = taken_slots(&g.seats, &unit.members);
                let Some(current) = g.seats[k] else { continue };
                if g.slots[current].slot.faces(point.xy(), facing) {
                    continue;
                }
                let from = g.slots[current].position;
                let best = g
                    .slots
                    .iter()
                    .enumerate()
                    .filter(|(s, slot)| !taken.contains(s) && slot.slot.faces(point.xy(), facing))
                    .min_by(|(a, x), (b, y)| {
                        (x.position - from)
                            .length()
                            .total_cmp(&(y.position - from).length())
                            .then(a.cmp(b))
                    })
                    .map(|(s, _)| s);
                if let Some(s) = best {
                    g.seats[k] = Some(s);
                    unit.members[k].position = g.slots[s].position;
                }
            }
        }
    }
}

/// A garrison's eyes (27 perf): one per facade a living soldier holds a
/// slot on, at the middle of that facade's slots and the infantry eye
/// height, in facade order. The squad sees what any of them sees. Empty for
/// a unit not at a building's perimeter.
pub fn facade_eyes(unit: &Unit, rules: &Rules) -> Vec<V3> {
    let Some(g) = unit.garrison.as_ref().filter(|_| unit.garrisoned()) else {
        return Vec::new();
    };
    let mut held = [false; 4];
    for (k, s) in unit.members.iter().enumerate() {
        if let Some(seat) = g.seat(k).filter(|_| s.alive()) {
            held[seat.slot.facade as usize] = true;
        }
    }
    let lift = crate::math::v3(0.0, 0.0, rules.physics.infantry_eye_m);
    (0..4u8)
        .filter(|&f| held[f as usize])
        .map(|f| {
            let on: Vec<V3> = g
                .slots
                .iter()
                .filter(|s| s.slot.facade == f)
                .map(|s| s.position)
                .collect();
            let sum = on
                .iter()
                .fold(crate::math::v3(0.0, 0.0, 0.0), |a, &p| a + p);
            sum * (1.0 / on.len() as f64) + lift
        })
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

/// Where a garrisoned squad would fire at `point` from: the building's
/// closest slot facing it (whoever holds it), or `None` if no facade faces.
pub fn facing_origin(unit: &Unit, point: V3, rules: &Rules) -> Option<V3> {
    let g = unit.garrison.as_ref()?;
    let facing = rules.garrison.slot_facing_min_deg.to_radians();
    g.slots
        .iter()
        .filter(|s| s.slot.faces(point.xy(), facing))
        .min_by(|a, b| {
            (a.position - point)
                .length()
                .total_cmp(&(b.position - point).length())
        })
        .map(|s| s.position)
}

/// Whether garrisoned member `k` stands at a slot facing `point`.
pub fn faces(unit: &Unit, k: usize, point: V3, rules: &Rules) -> bool {
    let facing = rules.garrison.slot_facing_min_deg.to_radians();
    unit.garrison
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
        unit.suppression = unit.suppression.max(rules.suppression.collapse_level);
        unit.suppressed_at = tick;
    }
    destroyed
}

/// What the owning side sees of a squad's garrison.
pub fn state(world: &WorldGeometry, unit: &Unit, rules: &Rules) -> Option<GarrisonState> {
    let g = unit.garrison.as_ref()?;
    let prop = world.prop(g.building)?;
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
        center: [prop.center.x, prop.center.y],
        half: [prop.half.x, prop.half.y],
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
    }
}
