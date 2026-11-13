//! Depots in the open country (L4, P3): a few military bodies, each in its
//! own yard, at a station on a country road; rare, legal, whole or absent.
use super::*;

/// The plan's depots, by their lots' shared id: each one's lots, and the
/// bodies standing in them.
fn depots(plan: &MapPlan) -> BTreeMap<String, (usize, Vec<Box2>)> {
    let mut out: BTreeMap<String, (usize, Vec<Box2>)> = BTreeMap::new();
    for lot in &plan.lots {
        let Some(depot) = mapgen::open_country::Pieces::depot_of(&lot.id) else {
            continue;
        };
        let entry = out.entry(depot).or_default();
        entry.0 += 1;
        entry.1.extend(
            loose_bodies(plan)
                .into_iter()
                .filter(|body| polygon_contains(&lot.ring, body.center)),
        );
    }
    out
}

/// Points round a box's outline every `step` metres, and its middle: what a
/// polygon's edge would cross before reaching a corner.
fn outline(body: &Box2, step: f64) -> Vec<Point> {
    let corners = body.corners();
    let mut points = vec![body.center];
    for (i, a) in corners.iter().enumerate() {
        let b = corners[(i + 1) % 4];
        let n = ((a[0] - b[0]).hypot(a[1] - b[1]) / step).ceil().max(1.0) as usize;
        points.extend((0..n).map(|k| {
            let t = k as f64 / n as f64;
            [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
        }));
    }
    points
}

fn rings(shapes: impl IntoIterator<Item = GroundShape>) -> Vec<Vec<Point>> {
    shapes
        .into_iter()
        .filter_map(|shape| match shape {
            GroundShape::Polygon { ring } => Some(ring),
            GroundShape::Stroke { .. } => None,
        })
        .collect()
}

/// Every depot stands whole, in the open country, on ground no other
/// feature claims: off every carriageway and its polygon gaps, every wood,
/// swamp, river, bridge deck and its approach, every objective and entry,
/// and every town and building, judged along each body's edges as well as
/// at its corners, and with no polygon's corner inside a body.
#[test]
fn depots_stand_in_open_country_on_legal_ground() {
    let presets = presets();
    let mapgen::open_country::Pieces::Bodies(groups) = &presets.open_country.depots.steading.pieces
    else {
        unreachable!()
    };
    let approach = presets.rivers.bridge.approach_m;
    let mut seen = 0;
    every_cell(|country| {
        let (plan, map, name) = (&country.plan, &country.map, &country.name);
        let roads: Vec<&GroundShape> = map
            .surfaces
            .iter()
            .filter(|area| area.kind.is_road())
            .map(|area| &area.shape)
            .collect();
        let grounds: Vec<&GroundShape> = map
            .forests
            .iter()
            .map(|forest| &forest.shape)
            .chain(map.swamps.iter().map(|swamp| &swamp.shape))
            .collect();
        let corners = rings(grounds.iter().copied().cloned());
        let decks: Vec<Box2> = map
            .bridges
            .iter()
            .map(|deck| Box2 {
                center: deck.center,
                half: [deck.half_extents[0] + approach, deck.half_extents[1]],
                yaw: deck.yaw,
            })
            .collect();
        let parts: Vec<Box2> = map
            .buildings
            .iter()
            .flat_map(|building| &building.geometry.parts)
            .map(|part| Box2 {
                center: part.center,
                half: [part.half_extents[0], part.half_extents[1]],
                yaw: part.yaw,
            })
            .collect();
        for (depot, (lots, bodies)) in depots(plan) {
            seen += 1;
            assert!(
                groups.iter().any(|group| group.pieces.len() == lots),
                "{name}: {depot} stands {lots} of a group's pieces"
            );
            for body in &bodies {
                for p in outline(body, 0.5) {
                    let at = |what: &str| format!("{name}: {depot} stands {what} at {p:?}");
                    assert!(outside_settlements(plan, p), "{}", at("in a settlement"));
                    assert!(
                        !roads.iter().any(|road| road.contains(p, 0.0)),
                        "{}",
                        at("on a road")
                    );
                    assert!(
                        !grounds.iter().any(|g| g.contains(p, 0.0)),
                        "{}",
                        at("in a wood or swamp")
                    );
                    assert!(
                        map.rivers.iter().all(|river| river.inside(p) < 0.0),
                        "{}",
                        at("in the water")
                    );
                    assert!(
                        !decks.iter().any(|deck| deck.contains(p, 0.0)),
                        "{}",
                        at("on a bridge")
                    );
                    assert!(
                        !parts.iter().any(|part| part.contains(p, 0.0)),
                        "{}",
                        at("in a building")
                    );
                }
                assert!(
                    !corners.iter().flatten().any(|c| body.contains(*c, 0.0)),
                    "{name}: a wood's or swamp's corner pokes into {depot}"
                );
                if let Some(sites) = &plan.skirmish {
                    for objective in sites.all_reserved_objectives() {
                        assert!(
                            !body.contains(objective.center, objective.radius_m),
                            "{name}: {depot} stands on objective {}",
                            objective.id
                        );
                    }
                    for entry in &sites.entries {
                        assert!(
                            !body.contains(entry.center, 0.0),
                            "{name}: {depot} on an entry"
                        );
                    }
                }
            }
        }
    });
    assert!(seen > 0, "no depot stands on any map");
}

/// A depot that finds no room is left out, and the map is still made.
#[test]
fn a_depot_that_does_not_fit_is_left_out() {
    let mut source: serde_json::Value = serde_json::from_str(PRESETS).unwrap();
    let depots = &mut source["open_country"]["depots"];
    depots["per_road_km"] = serde_json::json!(1.0);
    // A yard far deeper than any road leaves room for.
    depots["yard"]["rear_m"] = serde_json::json!(5000);
    let cramped = source.to_string();
    let request = serde_json::to_string(&request(MapType::Open, MapSize::Large, 1)).unwrap();
    match mapgen::generate_plan(&request, &cramped, TEMPLATES, &test_rules().to_string()) {
        mapgen::GenerateOutcome::Ok { plan } => {
            assert!(depots_of(&plan).is_empty(), "a depot stood in no room")
        }
        mapgen::GenerateOutcome::Error { diagnostics } => {
            panic!("a depot with no room refused the map: {diagnostics:?}")
        }
    }
}

fn depots_of(plan: &MapPlan) -> Vec<String> {
    depots(plan).into_keys().collect()
}

/// The count follows the road: maps with more country road hold more depots.
#[test]
fn more_road_holds_more_depots() {
    let held = |size: MapSize| -> usize {
        TYPES
            .into_iter()
            .flat_map(|map_type| countries(map_type, size))
            .map(|country| depots_of(&country.plan).len())
            .sum()
    };
    let (medium, xl) = (held(MapSize::Medium), held(MapSize::Xl));
    assert!(
        xl > medium,
        "XL maps hold {xl} depots, Medium maps {medium}"
    );
}
