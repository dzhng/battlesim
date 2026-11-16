//! Buildings as abstract fighting positions (L08–L10, P12). A building takes
//! squads whole while its seats last ([`crate::hold::Seats`]), each after a
//! stationary timer; inside, each soldier fights from an exposed window slot
//! just outside a facade, where its abstract hit capsule and muzzle are.
//! Published body poses stand indoors behind those windows; the squad sees
//! from one eye per building-frame direction it holds, at a real seat on the
//! highest occupied band ([`Windows::facade_eyes`]). Rounds that miss a slot meet the building's
//! own shell, and any round toward something beyond it meets the shell too:
//! no collider is ever switched off for a target. Collapse removes fighting
//! floors: a low-rise leaves a lower ruin, while a tall terminal shell keeps
//! its exterior height. Survivors escape on foot to legal ground nearby,
//! heavily suppressed, or die on the remaining walkable surface. Slots are
//! the one named exception to soldiers as free bodies (Q22): a seated soldier
//! stands at his slot.
//!
//! Which window each soldier holds is part of his squad's own hold; the
//! windows themselves are one plan per building ([`Windows`]); the world owns
//! the exposed physical geometry; this module seats squads and `structures`
//! holds integrity, one row of every destroyable prop's.
use contract::command::OrderError;
use contract::ids::{Tick, UnitId};
use contract::map::MoverClass;
use contract::templates::MaterializedEdge;

use contract::scenario::Rules;

use crate::arrangement;
use crate::math::{v2, V2};
use crate::units::{Unit, UnitOrder};
use crate::world::{Prop, PropId, Slot, WorldGeometry};
use contract::random::Rng;

mod occupy;
mod windows;
pub(crate) use occupy::occupy;
pub(crate) use windows::seat_evenly;
pub use windows::{SeatPlan, SeatSlot, Windows};

/// Exit places are sampled this far apart around the building.
const EXIT_STEP_M: f64 = 2.0;
/// A leaving squad's middle stands this far beyond the building's walls.
const EXIT_CLEARANCE_M: f64 = 1.0;
/// Approach points are sampled this far apart along a facade when the
/// nearest one is not clear.
const APPROACH_STEP_M: f64 = 1.0;

/// A squad's building-specific placement data. The host and timer live in `sim::hold`.
#[derive(Clone, Debug)]
pub struct Garrison {
    /// Each member's window in the building's plan ([`Windows`]) while inside.
    pub seats: Vec<Option<usize>>,
    /// Where the squad stood to enter: it leaves that way unless heading elsewhere.
    pub entry: V2,
}

fn held_building(unit: &Unit) -> Option<PropId> {
    unit.hold.as_ref().and_then(|hold| hold.host.building())
}

/// The building a squad fights from (inside or leaving, not entering).
pub fn garrisoned_in(unit: &Unit) -> Option<PropId> {
    held_building(unit).filter(|_| unit.garrisoned())
}

/// Whether a living squad holds `building` (inside or leaving).
pub(crate) fn held(units: &[Unit], building: PropId) -> bool {
    (units.iter()).any(|u| u.alive() && garrisoned_in(u) == Some(building))
}

/// The one band policy used by seating and building bulk integrity.
pub(crate) fn floor_band_count(geometry: &contract::templates::MaterializedBuilding) -> usize {
    geometry.floor_z.len().min(3)
}

/// Seats at exposed physical bays on the bottom three floor bands of the
/// building `prop` belongs to. The gameplay cap reserves places round the
/// four building-frame directions. A remembered or live replacement keeps the
/// source bays but loses floors above its remaining shell. Initial-part IDs
/// retain the complete source plan.
pub fn seats_for_state(world: &WorldGeometry, prop: &Prop, rules: &Rules) -> SeatPlan {
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
    for &z in geometry
        .floor_z
        .iter()
        .take(floor_band_count(geometry))
        .rev()
    {
        for (ordinal, edge) in geometry.edges.iter().enumerate().filter(|(_, e)| {
            let top = remaining_height.map_or(e.top_z, |h| e.top_z.min(e.base_z + h));
            e.exposed && z >= e.base_z && z < top
        }) {
            let direction = v2(edge.normal[0], edge.normal[1]).rotated(-geometry.frame.yaw);
            let group = ((libm::atan2(direction.y, direction.x) / std::f64::consts::FRAC_PI_2)
                .round() as i32)
                .rem_euclid(4) as usize;
            for bay in edge.bays.iter().flatten() {
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
    let capacity = rules.buildings.capacity_soldiers as usize;
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
pub(crate) fn building(world: &WorldGeometry, id: PropId) -> Option<&Prop> {
    world
        .prop(world.structure_owner(id)?)
        .filter(|p| p.body.garrison)
}

/// Whether a squad standing at `point` is close enough to `building` to step
/// in where it stands: within entry reach of any part its side knows.
pub(crate) fn within_entry_reach(
    world: &WorldGeometry,
    known: &crate::movement::SideGeometry,
    authored: PropId,
    building: PropId,
    point: V2,
    rules: &Rules,
) -> bool {
    let reach = crate::hold::Host::Building(building).timing(rules).reach_m;
    world
        .structure_parts(building)
        .into_iter()
        .filter_map(|id| known.prop(world, authored, id))
        .any(|p| p.footprint().contains(point, reach))
}

/// Approach the nearest exposed physical part using remembered poses: the
/// nearest point outside a part when it faces out through an exposed facade,
/// else the nearest point standing off one. Internal faces admit no entry.
/// When that point is not `clear`, the nearest clear point standing off any
/// exposed facade (sampled every [`APPROACH_STEP_M`] along it), else the
/// nearest point all the same.
pub(crate) fn approach(
    world: &WorldGeometry,
    known: &crate::movement::SideGeometry,
    authored: PropId,
    owner: PropId,
    from: V2,
    standoff: f64,
    clear: impl Fn(V2) -> bool,
) -> Option<V2> {
    let definition = world.building(owner)?;
    let parts: Vec<_> = world
        .structure_parts(owner)
        .into_iter()
        .filter_map(|id| known.prop(world, authored, id))
        .collect();
    let mut facades = Vec::new();
    for part in &parts {
        let source = world.authored_prop(part.id)?;
        let reference = definition.parts.iter().find(|p| p.prop == source)?;
        let edges: Vec<_> = definition
            .geometry
            .edges
            .iter()
            .filter(|e| e.part == reference.part && e.exposed)
            .collect();
        facades.push((part, edges));
    }
    let outside = |c: V2| !parts.iter().any(|p| p.footprint().contains(c, 0.0));
    // The point `t` along `e` that stands `standoff` off it.
    let standing_off = |part: &Prop, e: &MaterializedEdge, t: f64| {
        let (normal, along, reach, _) = e.facade.axes([part.half.x, part.half.y, part.half.z]);
        let n = v2(normal[0], normal[1]);
        let a = v2(along[0], along[1]);
        part.center + (n * (reach + standoff) + a * t).rotated(part.yaw)
    };
    let mut candidates = Vec::new();
    for (part, edges) in &facades {
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
        if exposed && outside(candidate) {
            candidates.push(candidate);
            continue;
        }
        for e in edges {
            let (_, along, _, _) = e.facade.axes([part.half.x, part.half.y, part.half.z]);
            let t = part
                .footprint()
                .to_local(from)
                .dot(v2(along[0], along[1]))
                .clamp(e.span_m[0], e.span_m[1]);
            let candidate = standing_off(part, e, t);
            if outside(candidate) {
                candidates.push(candidate)
            }
        }
    }
    let distance = |a: &V2, b: &V2| (*a - from).length().total_cmp(&(*b - from).length());
    let nearest = candidates.iter().copied().min_by(distance)?;
    if clear(nearest) {
        return Some(nearest);
    }
    let sampled = facades.iter().flat_map(|(part, edges)| {
        edges.iter().flat_map(move |e| {
            let [t0, t1] = e.span_m;
            let steps = ((t1 - t0) / APPROACH_STEP_M).ceil().max(1.0) as usize;
            (0..=steps)
                .map(move |k| standing_off(part, e, t0 + (t1 - t0) * k as f64 / steps as f64))
        })
    });
    let mut points: Vec<_> = candidates
        .into_iter()
        .chain(sampled)
        .filter(|&c| outside(c))
        .collect();
    points.sort_by(distance);
    Some(points.into_iter().find(|&c| clear(c)).unwrap_or(nearest))
}

/// Command check (L08): a standing building the side knows, and only
/// infantry squads. Who holds the building or is on its way in is never
/// checked here: room is decided when a squad gets there, and a squad that
/// finds an enemy squad there gives up its order where it stands
/// ([`crate::hold::admit`]).
pub fn validate(
    units: &[Unit],
    ordered: &[UnitId],
    target: Option<Prop>,
) -> Result<Prop, OrderError> {
    let target = target
        .filter(|p| p.body.garrison)
        .ok_or(OrderError::NotABuilding)?;
    // The command already checked each named unit is the side's own and alive.
    if let Some(&unit) = ordered.iter().find(|id| units[id.0 as usize].is_vehicle()) {
        return Err(OrderError::NotInfantry { unit });
    }
    Ok(target)
}

/// What stops a soldier on foot: every prop that blocks infantry.
fn solid(p: &Prop) -> bool {
    p.blocks(MoverClass::Infantry)
}

/// Where a leaving squad's soldiers stand: the place around the building
/// nearest `preferred` where the whole squad finds a spread-out arrangement
/// (deterministic, ties by perimeter order), one spot per living member.
pub(crate) fn exit_spots(
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
    let radius = unit.ground().half_width_m;
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

/// What the squad's front order wants of buildings.
pub(crate) enum Want {
    Enter(PropId),
    /// Leave: an exit order, or one that needs to move (boarding a carrier
    /// included).
    Leave,
    Stay,
}

pub(crate) fn want(unit: &Unit) -> Want {
    match unit.orders.front() {
        Some(UnitOrder::Garrison { building, .. }) => Want::Enter(*building),
        Some(UnitOrder::Exit | UnitOrder::Board { .. }) => Want::Leave,
        Some(o) if o.movement().is_some() => Want::Leave,
        _ => Want::Stay,
    }
}

/// Building cover strength a unit has from where it is observed: its
/// perimeter slots while garrisoned, otherwise none.
pub fn shelter(unit: &Unit) -> f64 {
    if unit.garrisoned() {
        1.0
    } else {
        0.0
    }
}

/// The building a garrisoned unit's soldiers stand against.
pub fn shell(unit: &Unit) -> Option<PropId> {
    unit.hold
        .as_ref()
        .filter(|_| unit.garrisoned())
        .and_then(|hold| hold.host.building())
}

/// A building collapsed (L10) and its ruin stands in its place. Every living
/// occupant takes the building bail-out injury, then either escapes to a clear
/// surface or dies; survivors leave pinned. Squads still entering stand outside.
pub fn collapse(
    world: &WorldGeometry,
    units: &mut [Unit],
    target: PropId,
    rules: &Rules,
    seed: u64,
    tick: Tick,
    ignored_props: &[PropId],
) -> Vec<UnitId> {
    let mut destroyed = Vec::new();
    for index in 0..units.len() {
        // A whole squad may already be dead and have relinquished its hold.
        // The fallen retain their floor's owner until that support disappears.
        for member in &mut units[index].members {
            if let Some(fallen) = &mut member.corpse {
                if fallen.support_building == Some(target) {
                    fallen.at.z = world
                        .surface_at(fallen.at.x, fallen.at.y)
                        .map_or(0.0, |s| s.z);
                    fallen.support_building = None;
                }
            }
        }
        if units[index]
            .hold
            .as_ref()
            .is_none_or(|hold| hold.host.building() != Some(target))
        {
            continue;
        }
        if !units[index].garrisoned() {
            units[index].hold = None;
            continue;
        }
        let (hulls, soldiers) = crate::arrangement::ground_taken(units, units[index].id);
        let rule = crate::bail_out::row(rules, contract::scenario::BailOutHost::Building);
        let host = units[index].hold.as_ref().expect("building hold").host;
        let out = crate::bail_out::apply(
            world,
            &mut units[index],
            host,
            rule,
            seed,
            tick,
            crate::bail_out::Occupancy {
                radius: rules.physics.soldier_radius_m,
                hulls: &hulls,
                soldiers: &soldiers,
                ignored_props,
            },
        );
        units[index].hold = None;
        if matches!(units[index].orders.front(), Some(UnitOrder::Exit)) {
            units[index].orders.pop_front();
        }
        if out == 0 {
            units[index].orders.clear();
            units[index].route = None;
            destroyed.push(units[index].id);
            continue;
        }
        units[index].settle();
        units[index].suppression = units[index].suppression.max(rules.suppression.pinned.level);
        units[index].suppressed_at = tick;
    }
    destroyed
}
