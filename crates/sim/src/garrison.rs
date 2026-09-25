//! Buildings as abstract fighting positions (L08–L10, P12). Whole
//! squads enter by soldier capacity after a stationary timer; inside, each
//! soldier stands at a perimeter slot just outside a facade, where its hit
//! capsule, eyes and muzzle are. Rounds that miss a slot meet the building's
//! own shell, and any round toward something beyond it meets the shell too:
//! no collider is ever switched off for a target. A collapse leaves a lower,
//! permanent ruin; survivors escape on foot to legal ground nearby, heavily
//! suppressed, or die where they stood.
//!
//! Garrison state is the unit's own (`Unit::garrison`); the world owns the
//! facade slots and the ruin; [`Structures`] holds building health.
use std::collections::{BTreeMap, BTreeSet};

use contract::command::OrderError;
use contract::ids::{Side, Tick, UnitId};
use contract::map::{MoverClass, PropDefinition, PropKind};
use contract::observation::{GarrisonPhase, GarrisonState, MoveState};
use contract::scenario::Rules;

use crate::digest::Digest;
use crate::math::{v2, V2, V3};
use crate::rng::Rng;
use crate::units::{Unit, UnitOrder};
use crate::world::{Prop, PropId, Slot, WorldGeometry};

/// Candidate escape points are this far apart on each search ring.
const ESCAPE_STEP_M: f64 = 1.0;
/// Escaping soldiers keep at least this far apart.
const ESCAPE_SPACING_M: f64 = 1.0;
/// A walk out is checked for solids this finely.
const REACH_STEP_M: f64 = 0.25;
/// Exit places are sampled this far apart around the building.
const EXIT_STEP_M: f64 = 2.0;
/// A leaving squad's centre stands this far beyond its formation's reach.
const EXIT_CLEARANCE_M: f64 = 1.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Stationary beside the building for this many ticks.
    Entering(u32),
    /// The timer ran out but the whole squad does not fit (or enemies hold it).
    WaitingForRoom,
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
            Phase::Entering(_) | Phase::WaitingForRoom => None,
        }
    }
}

/// Standing buildings' health, and which ruin replaced which building.
#[derive(Clone, Debug, Default)]
pub struct Structures {
    hp: BTreeMap<PropId, f64>,
    /// Ruin → the building it replaced.
    ruins: BTreeMap<PropId, PropId>,
}

impl Structures {
    pub fn new(world: &WorldGeometry, rules: &Rules) -> Self {
        Structures {
            hp: world
                .props()
                .filter(|p| p.kind == PropKind::Building)
                .map(|p| (p.id, rules.buildings.hp))
                .collect(),
            ruins: BTreeMap::new(),
        }
    }

    pub fn standing(&self, building: PropId) -> bool {
        self.hp.contains_key(&building)
    }

    pub fn hp(&self, building: PropId) -> Option<f64> {
        self.hp.get(&building).copied()
    }

    /// The building a ruin stands in place of.
    pub fn replaced_by(&self, ruin: PropId) -> Option<PropId> {
        self.ruins.get(&ruin).copied()
    }

    /// Structural damage to a standing building; true when it must collapse.
    /// Only weapons with structural damage reduce it (L10).
    pub fn damage(&mut self, building: PropId, amount: f64) -> bool {
        match self.hp.get_mut(&building) {
            Some(hp) if amount > 0.0 => {
                *hp -= amount;
                *hp <= 0.0
            }
            _ => false,
        }
    }

    pub fn digest(&self, d: &mut Digest) {
        d.u64(self.hp.len() as u64);
        for (id, hp) in &self.hp {
            d.u64(*id as u64).f64(*hp);
        }
        d.u64(self.ruins.len() as u64);
        for (ruin, building) in &self.ruins {
            d.u64(*ruin as u64).u64(*building as u64);
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

/// A standing building, or `None`.
fn building(world: &WorldGeometry, id: PropId) -> Option<&Prop> {
    world.prop(id).filter(|p| p.kind == PropKind::Building)
}

/// Living soldiers of squads at the building's perimeter, by side.
fn occupancy(units: &[Unit], building: PropId, side: Side) -> (usize, usize) {
    let (mut own, mut enemy) = (0, 0);
    for u in units.iter().filter(|u| u.garrisoned()) {
        if u.garrison.as_ref().is_some_and(|g| g.building == building) {
            let n = u.members.iter().filter(|s| s.alive()).count();
            if u.side == side {
                own += n;
            } else {
                enemy += n;
            }
        }
    }
    (own, enemy)
}

/// Command check (L08): infantry only, a standing building, and every
/// ordered squad fitting whole beside the side's own occupants and the squads
/// already on their way in. Only the side's own units are counted here.
pub fn validate(
    world: &WorldGeometry,
    structures: &Structures,
    units: &[Unit],
    side: Side,
    ordered: &[UnitId],
    target: PropId,
    rules: &Rules,
) -> Result<(), OrderError> {
    if building(world, target).is_none() || !structures.standing(target) {
        return Err(OrderError::NotABuilding);
    }
    if let Some(u) = ordered.iter().find(|u| {
        units
            .get(u.0 as usize)
            .is_some_and(|u| !u.kind.is_infantry())
    }) {
        return Err(OrderError::NotInfantry { unit: *u });
    }
    let living = |u: &Unit| u.members.iter().filter(|s| s.alive()).count();
    let heading_in = |u: &Unit| {
        u.garrison.as_ref().is_some_and(|g| g.building == target)
            || u.orders
                .iter()
                .any(|o| matches!(o, UnitOrder::Garrison { building, .. } if *building == target))
    };
    let committed: usize = units
        .iter()
        .filter(|u| u.side == side && u.alive() && !ordered.contains(&u.id) && heading_in(u))
        .map(living)
        .sum();
    let entering: usize = ordered
        .iter()
        .filter_map(|u| units.get(u.0 as usize))
        .map(living)
        .sum();
    if committed + entering > rules.buildings.capacity_soldiers as usize {
        return Err(OrderError::CapacityFull);
    }
    Ok(())
}

/// Where a squad at `from` walks to enter: beside the nearest facade.
pub fn approach(world: &WorldGeometry, target: PropId, from: V2, rules: &Rules) -> Option<V2> {
    let prop = building(world, target)?;
    Some(prop.exterior_point(from, rules.garrison.entry_distance_m / 2.0))
}

/// Ground a soldier may stand on: traversable and clear of what stops infantry.
fn standing_room(world: &WorldGeometry, p: V2, half_width: f64) -> bool {
    world.surface_at(p.x, p.y).is_some_and(|s| s.traversable)
        && !world.props_near(p, half_width).iter().any(|prop| {
            prop.kind.blocks(MoverClass::Infantry) && prop.footprint().contains(p, half_width)
        })
}

/// A walk from `a` to `b` on foot meets no solid and no impassable ground.
fn reachable(world: &WorldGeometry, a: V2, b: V2) -> bool {
    let n = ((b - a).length() / REACH_STEP_M).ceil().max(1.0) as usize;
    (0..=n).all(|k| {
        let p = a + (b - a) * (k as f64 / n as f64);
        world.surface_at(p.x, p.y).is_some_and(|s| s.traversable)
            && !world.props_near(p, 0.0).iter().any(|prop| {
                prop.kind.blocks(MoverClass::Infantry) && prop.footprint().contains(p, 0.0)
            })
    })
}

/// The nearest legal, reachable, free point within `radius` of `from`, on
/// rings outward from it; `None` when the local search finds none.
fn escape(
    world: &WorldGeometry,
    from: V2,
    taken: &[V2],
    radius: f64,
    half_width: f64,
) -> Option<V2> {
    let rings = (radius / ESCAPE_STEP_M).floor() as usize;
    for ring in 0..=rings {
        let r = ring as f64 * ESCAPE_STEP_M;
        let n = ((std::f64::consts::TAU * r / ESCAPE_STEP_M).ceil() as usize).max(1);
        for k in 0..n {
            let a = std::f64::consts::TAU * k as f64 / n as f64;
            let p = from + v2(a.cos(), a.sin()) * r;
            if standing_room(world, p, half_width)
                && taken.iter().all(|t| (*t - p).length() >= ESCAPE_SPACING_M)
                && reachable(world, from, p)
            {
                return Some(p);
            }
        }
    }
    None
}

/// Where a leaving squad stands: the place around the building nearest
/// `preferred` where its whole formation has standing room (deterministic,
/// ties by perimeter order).
fn exit_place(world: &WorldGeometry, prop: &Prop, unit: &Unit, preferred: V2) -> Option<V2> {
    let half_width = unit.mobility.half_width_m;
    let reach = unit
        .members
        .iter()
        .map(|s| s.formation.length())
        .fold(0.0, f64::max)
        + half_width
        + EXIT_CLEARANCE_M;
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
    order.into_iter().map(|i| candidates[i]).find(|&c| {
        standing_room(world, c, half_width)
            && unit
                .members
                .iter()
                .filter(|s| s.alive())
                .all(|s| standing_room(world, c + s.formation.rotated(unit.yaw), half_width))
    })
}

/// Seat every living member, spreading the squad evenly around the facades:
/// the first free slot of each facade in turn.
fn seat_evenly(slots: &[SeatSlot], taken: &BTreeSet<usize>, unit: &Unit) -> Vec<Option<usize>> {
    let mut facades: [Vec<usize>; 4] = Default::default();
    for (i, s) in slots.iter().enumerate() {
        facades[s.slot.facade as usize].push(i);
    }
    let longest = facades.iter().map(Vec::len).max().unwrap_or(0);
    let mut free = (0..longest)
        .flat_map(|j| facades.iter().filter_map(move |f| f.get(j).copied()))
        .filter(|i| !taken.contains(i));
    unit.members
        .iter()
        .map(|s| if s.alive() { free.next() } else { None })
        .collect()
}

/// Slots held by any occupant of `building`.
fn taken_slots(units: &[Unit], building: PropId) -> BTreeSet<usize> {
    units
        .iter()
        .filter(|u| u.garrisoned())
        .filter_map(|u| {
            u.garrison
                .as_ref()
                .filter(|g| g.building == building)
                .map(|g| (u, g))
        })
        .flat_map(|(u, g)| {
            g.seats
                .iter()
                .enumerate()
                .filter(|(k, _)| u.members[*k].alive())
                .filter_map(|(_, s)| *s)
        })
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
/// ordered building, seat whole squads when the timer ends and they fit,
/// leave after the exit timer. Transitions are stationary: a squad with a
/// building has no movement goal.
pub fn advance(world: &WorldGeometry, structures: &Structures, units: &mut [Unit], rules: &Rules) {
    let timer = ticks(rules.garrison.enter_exit_s, rules);
    for i in 0..units.len() {
        if !units[i].alive() {
            continue;
        }
        let want = want(&units[i]);
        let phase = units[i].garrison.as_ref().map(|g| (g.building, g.phase));
        match (phase, want) {
            (None, Want::Enter(b)) => {
                let Some(prop) = building(world, b).filter(|_| structures.standing(b)) else {
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
            (Some((b, Phase::Entering(_) | Phase::WaitingForRoom)), w)
                if !matches!(w, Want::Enter(x) if x == b) || !structures.standing(b) =>
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
            (Some((b, Phase::Entering(_) | Phase::WaitingForRoom)), _) => {
                let side = units[i].side;
                let (own, enemy) = occupancy(units, b, side);
                let entering = units[i].members.iter().filter(|s| s.alive()).count();
                let g = units[i].garrison.as_mut().unwrap();
                if enemy > 0 || own + entering > rules.buildings.capacity_soldiers as usize {
                    g.phase = Phase::WaitingForRoom;
                    continue;
                }
                let entry = g.entry;
                let prop = building(world, b).expect("standing building");
                let slots = slots(world, prop, rules);
                let taken = taken_slots(units, b);
                let seats = seat_evenly(&slots, &taken, &units[i]);
                let centre = prop.center;
                let unit = &mut units[i];
                unit.position = centre.with_z(world.height_at(centre.x, centre.y).unwrap_or(0.0));
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
                if let Some(place) = exit_place(world, prop, unit, preferred) {
                    let z = world.surface_at(place.x, place.y).map_or(0.0, |s| s.z);
                    let unit = &mut units[i];
                    unit.position = place.with_z(z);
                    for s in &mut unit.members {
                        s.offset = s.formation;
                    }
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
pub fn allocate_slots(units: &mut [Unit], aims: &[(usize, Vec<(bool, V3)>)], rules: &Rules) {
    let facing = rules.garrison.slot_facing_min_deg.to_radians();
    for (i, mounts) in aims {
        let i = *i;
        let Some(building) = units[i]
            .garrison
            .as_ref()
            .filter(|g| g.phase == Phase::Inside)
            .map(|g| g.building)
        else {
            continue;
        };
        for &(squad, point) in mounts {
            let living: Vec<usize> = (0..units[i].members.len())
                .filter(|&k| units[i].members[k].alive())
                .collect();
            let participants = if squad {
                &living[..]
            } else {
                &living[..living.len().min(1)]
            };
            for &k in participants {
                let mut taken = taken_slots(units, building);
                let g = units[i].garrison.as_mut().unwrap();
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
                    taken.insert(s);
                    g.seats[k] = Some(s);
                }
            }
        }
    }
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

/// Collapse a building (L10): a lower, impassable ruin replaces it for good;
/// each occupant survives with the configured probability and escapes on
/// foot to the nearest legal free ground within the local search, carrying
/// heavy suppression; one with no legal escape dies where it stood. Squads
/// still entering stand outside and are unharmed. Returns units destroyed.
pub fn collapse(
    world: &mut WorldGeometry,
    structures: &mut Structures,
    units: &mut [Unit],
    target: PropId,
    rules: &Rules,
    rng: &mut Rng,
    tick: Tick,
) -> Vec<UnitId> {
    structures.hp.remove(&target);
    let Some(prop) = world.remove_prop(target) else {
        return Vec::new();
    };
    let ruin = world.add_prop(&PropDefinition {
        kind: PropKind::Ruin,
        center: [prop.center.x, prop.center.y],
        yaw: prop.yaw,
        half_extents: [
            prop.half.x,
            prop.half.y,
            rules.buildings.ruin_height_m / 2.0,
        ],
        base_z: Some(prop.base_z),
    });
    structures.ruins.insert(ruin, target);
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
            let at = unit.member_position(k);
            let lives = rng.unit() < g.survival_probability_on_collapse;
            let place = lives
                .then(|| {
                    escape(
                        world,
                        at.xy(),
                        &taken,
                        g.exit_search_radius_m,
                        unit.mobility.half_width_m,
                    )
                })
                .flatten();
            match place {
                Some(p) => {
                    taken.push(p);
                    out.push((k, p));
                }
                None => {
                    unit.members[k].fall(at);
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
        // The squad gathers on the survivor nearest their middle; the others
        // walk back to formation from where they came out.
        let middle = out.iter().fold(v2(0.0, 0.0), |a, (_, p)| a + *p) * (1.0 / out.len() as f64);
        let anchor = out
            .iter()
            .map(|(_, p)| *p)
            .min_by(|a, b| (*a - middle).length().total_cmp(&(*b - middle).length()))
            .unwrap();
        let z = world.surface_at(anchor.x, anchor.y).map_or(0.0, |s| s.z);
        unit.position = anchor.with_z(z);
        for (k, p) in out {
            unit.members[k].offset = (p - anchor).rotated(-unit.yaw);
        }
        unit.suppression = unit.suppression.max(rules.suppression.collapse_level);
        unit.suppressed_at = tick;
    }
    destroyed
}

/// What the owning side sees of a squad's garrison.
pub fn state(unit: &Unit, rules: &Rules) -> Option<GarrisonState> {
    let g = unit.garrison.as_ref()?;
    let timer = ticks(rules.garrison.enter_exit_s, rules) as f64;
    let (phase, progress) = match g.phase {
        Phase::Entering(n) => (GarrisonPhase::Entering, n as f64 / timer),
        Phase::WaitingForRoom => (GarrisonPhase::WaitingForRoom, 1.0),
        Phase::Inside => (GarrisonPhase::Inside, 1.0),
        Phase::Exiting(n) => (GarrisonPhase::Exiting, n as f64 / timer),
    };
    Some(GarrisonState {
        building: g.building,
        phase,
        progress,
    })
}

/// Fold a unit's garrison into a digest, with presence and length tags.
pub fn digest(unit: &Unit, d: &mut Digest) {
    d.u64(unit.garrison.is_some() as u64);
    if let Some(g) = &unit.garrison {
        let (tag, n) = match g.phase {
            Phase::Entering(n) => (0, n),
            Phase::WaitingForRoom => (1, 0),
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
