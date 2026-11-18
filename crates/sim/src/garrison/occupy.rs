//! Where each squad sent into a building walks to step in, planned on what
//! its side knows of the building and the ground.
use crate::arrangement;
use crate::math::V2;
use crate::movement::SideGeometry;
use crate::navigation::Pockets;
use crate::units::{Unit, UnitOrder};
use crate::world::{PropId, WorldGeometry};
use contract::command::{
    BuildingEntry, BuildingPlacement, BuildingPreviewRequest, OrderError, Refusal,
};
use contract::scenario::Rules;

/// The building order's plan: for each listed squad, in unit order, from
/// where its queue ends when the order is queued, otherwise from where it
/// stands, the approach [`super::approach`] gives. The first squad given an
/// entry takes the nearest approach. Each later one takes the nearest it can
/// reach whose ground (half its spread) stays the soldiers' spacing clear of
/// every earlier entry's, else the nearest all the same, so squads sent in
/// together don't stack on one spot. A squad gets an entry if it stands
/// within entry reach of the building or its side's grid reaches that
/// approach; otherwise it gets none. Only a squad bigger than the whole
/// building is refused, one by one; who is inside is never read: whether a
/// squad finds room is decided when it gets there.
pub(crate) fn occupy(
    world: &WorldGeometry,
    known: &SideGeometry,
    authored: PropId,
    units: &[Unit],
    request: &BuildingPreviewRequest,
    rules: &Rules,
) -> Result<(BuildingPlacement, Vec<Refusal>), OrderError> {
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
    let target = super::validate(units, &ids, target)?;
    let total = super::seats_for_state(world, &target, rules).len();
    let refused = crate::hold::refuse_oversized(units, &mut ids, total);
    let owner = target.id;
    let standoff = rules.garrison.entry_distance_m / 2.0;
    let mut nav = known.clone();
    let grid = nav.grid(world, authored);
    let pockets = Pockets::default();
    let gap = rules.infantry_movement.spacing_m;
    let mut entries = Vec::new();
    // Where each entry stands, and the ground its squad covers there.
    let mut taken: Vec<(V2, f64)> = Vec::new();
    for &id in &ids {
        let u = &units[id.0 as usize];
        let from = if request.queued {
            queue_end(u)
        } else {
            u.position.xy()
        };
        let living = crate::hold::living(u);
        let ground = arrangement::spread(&rules.infantry_movement, living) / 2.0;
        let near = super::within_entry_reach(world, known, authored, owner, from, rules);
        let reaches = |p: V2| near || grid.reaches(from, p, &u.ground(rules), &pockets);
        let apart = |p: V2| {
            taken
                .iter()
                .all(|&(q, r)| (p - q).length() >= r + ground + gap)
        };
        let clear = |p: V2| taken.is_empty() || (apart(p) && reaches(p));
        let Some(approach) = super::approach(world, known, authored, owner, from, standoff, clear)
        else {
            continue;
        };
        if reaches(approach) {
            taken.push((approach, ground));
            entries.push(BuildingEntry {
                unit: id,
                approach: [approach.x, approach.y],
            });
        }
    }
    let placement = BuildingPlacement {
        building: owner,
        entries,
    };
    Ok((placement, refused))
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
