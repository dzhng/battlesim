//! Where each squad sent into a building walks to step in, planned on what
//! its side knows of the building and the ground.
use crate::math::V2;
use crate::movement::SideGeometry;
use crate::navigation::Pockets;
use crate::units::{Unit, UnitOrder};
use crate::world::{PropId, WorldGeometry};
use contract::command::{BuildingEntry, BuildingPlacement, BuildingPreviewRequest, OrderError};
use contract::scenario::Rules;

/// The building order's plan: for each listed squad, from where its queue
/// ends when the order is queued, otherwise from where it stands, the
/// approach [`super::approach`] gives. A squad gets an entry if it stands
/// within entry reach of the building or its side's grid reaches that
/// approach; otherwise it gets none. Who is inside is never read: whether a
/// squad finds room is decided when it gets there.
pub(crate) fn occupy(
    world: &WorldGeometry,
    known: &SideGeometry,
    authored: PropId,
    units: &[Unit],
    request: &BuildingPreviewRequest,
    rules: &Rules,
) -> Result<BuildingPlacement, OrderError> {
    let mut ids = request.units.clone();
    ids.sort();
    ids.dedup();
    // The clicked part as the side knows it, or, once it is gone, another
    // part of the same building.
    let clicked = world.remembered_structure_owner(request.building);
    let target = known
        .prop(world, authored, request.building)
        .or_else(|| {
            world
                .historical_building_parts(clicked)
                .iter()
                .find_map(|&id| known.prop(world, authored, id))
        })
        .filter(|p| world.building_of(p.id).is_some());
    let target = target.map(|part| {
        let owner = world.remembered_structure_owner(part.id);
        let mut owner_prop = known.prop(world, authored, owner).unwrap_or(part);
        owner_prop.id = owner;
        owner_prop
    });
    super::validate(world, units, &ids, target.clone(), rules)?;
    let owner = target.expect("validated building").id;
    let standoff = rules.garrison.entry_distance_m / 2.0;
    let mut nav = known.clone();
    let grid = nav.grid(world, authored);
    let pockets = Pockets::default();
    let entries = ids
        .iter()
        .filter_map(|&id| {
            let u = &units[id.0 as usize];
            let from = if request.queued {
                queue_end(u)
            } else {
                u.position.xy()
            };
            let approach = super::approach(world, known, authored, owner, from, standoff)?;
            (super::within_entry_reach(world, known, authored, owner, from, rules)
                || grid.reaches(from, approach, u.ground(), &pockets))
            .then_some(BuildingEntry {
                unit: id,
                approach: [approach.x, approach.y],
            })
        })
        .collect();
    Ok(BuildingPlacement {
        building: owner,
        entries,
    })
}

/// Where a squad will stand once its queued movement is done.
fn queue_end(u: &Unit) -> V2 {
    u.orders
        .iter()
        .rev()
        .find_map(|o| match o {
            UnitOrder::Move(m) | UnitOrder::AttackMove(m) => Some(m.destination),
            UnitOrder::Garrison { approach, .. } => Some(*approach),
            _ => None,
        })
        .unwrap_or(u.position.xy())
}
