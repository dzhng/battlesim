//! Depots in the open country (L4, P3): a few military bodies, each in its
//! own yard, at a station on a country road; rare, legal, whole or absent.
use super::*;
use mapgen::open_country::BodyGroup;
use std::collections::BTreeSet;

/// A depot as it stands: the kinds of body in each of its yards, and the
/// bodies themselves.
#[derive(Default)]
struct Standing {
    yards: Vec<BTreeSet<String>>,
    bodies: Vec<Box2>,
}

/// The plan's depots, by their lots' shared id.
fn depots(plan: &MapPlan) -> BTreeMap<String, Standing> {
    let mut out: BTreeMap<String, Standing> = BTreeMap::new();
    for lot in &plan.lots {
        let Some(depot) = mapgen::open_country::Pieces::depot_of(&lot.id) else {
            continue;
        };
        let entry = out.entry(depot).or_default();
        let inside = |center: Point| polygon_contains(&lot.ring, center);
        entry.yards.push(
            plan.props
                .iter()
                .filter(|prop| inside(prop.center))
                .map(|prop| prop.kind.clone())
                .collect(),
        );
        entry.bodies.extend(
            loose_bodies(plan)
                .into_iter()
                .filter(|body| inside(body.center)),
        );
    }
    out
}

/// The yards stand some group's pieces, each piece in a yard of its own:
/// the kinds in each yard are its piece's body and net, and no piece is
/// missing.
fn whole(groups: &[BodyGroup], yards: &[BTreeSet<String>]) -> bool {
    let mut stood = yards.to_vec();
    stood.sort();
    groups.iter().any(|group| {
        let mut pieces: Vec<BTreeSet<String>> = group
            .pieces
            .iter()
            .map(|piece| {
                std::iter::once(piece.kind.clone())
                    .chain(piece.net.clone())
                    .collect()
            })
            .collect();
        pieces.sort();
        pieces == stood
    })
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
    let groups = &presets.open_country.depots.steading.pieces;
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
        for (depot, Standing { yards, bodies }) in depots(plan) {
            seen += 1;
            assert!(
                whole(groups, &yards),
                "{name}: {depot} stands {yards:?}, no group's pieces"
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

/// The plan of an Open Large map under the presets, with `edit` made to
/// their depots.
fn with_depots(seed: u64, edit: impl FnOnce(&mut serde_json::Value)) -> MapPlan {
    let mut source: serde_json::Value = serde_json::from_str(PRESETS).unwrap();
    edit(&mut source["open_country"]["depots"]);
    let request = serde_json::to_string(&request(MapType::Open, MapSize::Large, seed)).unwrap();
    match mapgen::generate_plan(
        &request,
        &source.to_string(),
        TEMPLATES,
        &test_rules().to_string(),
    ) {
        mapgen::GenerateOutcome::Ok { plan } => *plan,
        mapgen::GenerateOutcome::Error { diagnostics } => {
            panic!("seed {seed}: the map was refused: {diagnostics:?}")
        }
    }
}

/// A depot left out leaves no trace: the map is the one it would be if no
/// depot were ever tried, with no yard, paving or body of one.
fn left_out(seed: u64, cramped: MapPlan) {
    assert!(depots_of(&cramped).is_empty(), "a depot stood in no room");
    let none = with_depots(seed, |depots| depots["per_road_km"] = 0.into());
    assert!(
        serde_json::to_value(&cramped).unwrap() == serde_json::to_value(&none).unwrap(),
        "a depot left out left part of itself on the map"
    );
}

/// A depot that finds no room is left out, and the map is still made.
#[test]
fn a_depot_that_does_not_fit_is_left_out() {
    let cramped = with_depots(1, |depots| {
        depots["per_road_km"] = 1.0.into();
        // A yard far deeper than any road leaves room for.
        depots["yard"]["rear_m"] = 5000.into();
    });
    left_out(1, cramped);
}

/// A depot whose first pieces find room and whose last does not is left
/// out whole: none of it stands.
#[test]
fn a_depot_missing_a_piece_is_left_out_whole() {
    let cramped = with_depots(1, |depots| {
        depots["per_road_km"] = 1.0.into();
        // Two tents that fit, then a row of tents longer than any road.
        depots["groups"] = serde_json::json!([{ "weight": 1, "pieces": [
            { "kind": "tent" },
            { "kind": "tent" },
            { "kind": "tent", "count": [400, 400] }
        ] }]);
    });
    left_out(1, cramped);
}

fn depots_of(plan: &MapPlan) -> Vec<String> {
    depots(plan).into_keys().collect()
}

/// The count follows the road at the presets' rate: on one map, twice the
/// depots for each kilometre of road holds some, and no fewer.
#[test]
fn a_higher_rate_holds_no_fewer_depots() {
    let rate = presets().open_country.depots.per_road_km;
    let held = |per_road_km: f64| {
        depots_of(&with_depots(1, |depots| {
            depots["per_road_km"] = per_road_km.into()
        }))
        .len()
    };
    let (at_rate, doubled) = (held(rate), held(2.0 * rate));
    assert!(doubled > 0, "twice the rate stands no depot");
    assert!(
        doubled >= at_rate,
        "twice the rate holds {doubled} depots, the rate {at_rate}"
    );
}
