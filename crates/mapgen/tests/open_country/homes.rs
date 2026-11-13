//! Homes in the open country: where they stand, and a way from each
//! door to a road.
use super::*;

/// Homes stand out in the country on every map: in ones and small groups,
/// outside every settlement, on dry open ground off every road and wood.
#[test]
fn homes_stand_in_open_country_on_legal_ground() {
    let largest = presets()
        .open_country
        .homesteads
        .groups
        .iter()
        .map(|group| group.homes[1])
        .max()
        .unwrap() as usize;
    every_cell(|country| {
        let (plan, map, name) = (&country.plan, &country.map, &country.name);
        let homes = country.bare.buildings.len()..plan.buildings.len();
        assert!(!homes.is_empty(), "{name}: no home stands in open country");
        let mut groups: BTreeMap<&str, usize> = BTreeMap::new();
        for building in &plan.buildings[homes] {
            let [x, y, _] = building.frame.translation;
            assert!(
                outside_settlements(plan, [x, y]),
                "{name}: {} stands inside a settlement",
                building.id
            );
            let group = building.id.rsplit_once('/').unwrap().0;
            *groups.entry(group).or_default() += 1;
        }
        for (group, count) in groups {
            assert!(
                (1..=largest).contains(&count),
                "{name}: {group} has {count} homes"
            );
        }
        let roads: Vec<&GroundShape> = map
            .surfaces
            .iter()
            .filter(|area| area.kind.is_road() && matches!(area.shape, GroundShape::Stroke { .. }))
            .map(|area| &area.shape)
            .collect();
        for (index, part) in home_parts(country) {
            let id = &plan.buildings[index].id;
            for corner in part.corners().into_iter().chain([part.center]) {
                assert!(
                    corner
                        .iter()
                        .zip(plan.size)
                        .all(|(v, size)| *v > 0.0 && *v < size),
                    "{name}: {id} leaves the map"
                );
                assert!(
                    !roads.iter().any(|road| road.contains(corner, 0.0)),
                    "{name}: {id} stands on a carriageway"
                );
                assert!(
                    !map.forests
                        .iter()
                        .any(|forest| forest.shape.contains(corner, 0.0)),
                    "{name}: {id} stands among trees"
                );
                assert!(
                    map.rivers.iter().all(|river| river.inside(corner) < 0.0),
                    "{name}: {id} stands in the water"
                );
            }
        }
    });
}

/// A home's street door opens onto a road, a track or its own lane a short
/// walk away, with no building and no loose body in the way.
#[test]
fn every_home_has_a_way_from_its_door_to_a_road() {
    const WALK_M: f64 = 40.0;
    every_cell(|country| {
        let (plan, map, name) = (&country.plan, &country.map, &country.name);
        let walls: Vec<Box2> = home_parts(country)
            .into_iter()
            .map(|(_, part)| part)
            .collect();
        let bodies = loose_bodies(plan);
        let roads: Vec<&GroundShape> = map
            .surfaces
            .iter()
            .filter(|area| area.kind.is_road() && matches!(area.shape, GroundShape::Stroke { .. }))
            .map(|area| &area.shape)
            .collect();
        for index in country.bare.buildings.len()..plan.buildings.len() {
            let id = &plan.buildings[index].id;
            let door = &map.buildings[index].geometry.entrances[0];
            let at = |reach: f64| {
                [
                    door.position[0] + door.normal[0] * reach,
                    door.position[1] + door.normal[1] * reach,
                ]
            };
            let reached = (1..=(WALK_M as u32) * 2)
                .map(|step| 0.5 * f64::from(step))
                .find(|reach| roads.iter().any(|road| road.contains(at(*reach), 0.0)))
                .unwrap_or_else(|| {
                    panic!("{name}: {id} has no road within {WALK_M} m of its door")
                });
            // From just outside the door's own wall to the road's edge.
            let mut reach = 0.25;
            while reach < reached {
                let p = at(reach);
                assert!(
                    !walls.iter().any(|wall| wall.contains(p, 0.0)),
                    "{name}: {id} opens onto a building"
                );
                assert!(
                    !bodies.iter().any(|body| body.contains(p, 0.4)),
                    "{name}: a body stands in the way out of {id}"
                );
                reach += 0.25;
            }
        }
    });
}
