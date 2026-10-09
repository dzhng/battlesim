//! A generated river map in the simulation's world: what a battle loads. The
//! plan's own measurements say its bridges join the banks; here the world
//! the units move in says so.
#[path = "common/first_where.rs"]
mod first_where;
use contract::map::{Bridge, MapDefinition};
use mapgen::layout::{generate_layout, GenerationRequest, MapSize, MapType, PresetDefinitions};
use mapgen::{CompileLimits, Diagnostic, MapPlan};
use sim::math::v2;
use sim::navigation::RoadNet;
use sim::world::{SurfaceKind, WorldGeometry};
use std::collections::BTreeSet;

const PRESETS: &str = include_str!("../../../fixtures/map-presets.json");

/// A layout with a river (the shipped presets, every type's river chance
/// set to one) and the map it compiles into, from the first seed whose
/// layout puts a settlement across the water from the main one: the one
/// the road graph can reach only by a deck. Not every river map has one,
/// and generation may refuse a seed by name, so the seed is searched for.
/// No town is built on it: the crossing is the layout's, and the parcel
/// pass has its own river test.
fn river_map(map_type: MapType, size: MapSize) -> (String, MapPlan, MapDefinition) {
    let across_the_water = |(plan, _): &(MapPlan, MapDefinition)| {
        let centre = plan.settlements[0].center;
        plan.settlements
            .iter()
            .any(|s| across(&plan.rivers[0], centre, s.center))
    };
    let (seed, (plan, map)) = first_where::first_where(
        1..=10,
        |seed| river_seed(map_type, size, seed),
        across_the_water,
    )
    .unwrap_or_else(|| panic!("{map_type:?} {size:?}: no settlement across the water"));
    (format!("{map_type:?} {size:?} seed {seed}"), plan, map)
}

/// Whether the straight line from `a` to `b` crosses the river's middle an
/// odd number of times: the two stand on opposite banks.
fn across(river: &contract::river::River, a: [f64; 2], b: [f64; 2]) -> bool {
    let side = |p: [f64; 2], q: [f64; 2], r: [f64; 2]| contract::ground::cross(p, q, r);
    let crossings = river
        .samples()
        .windows(2)
        .filter(|pair| {
            let (c, d) = (pair[0].xy, pair[1].xy);
            (side(a, b, c) > 0.0) != (side(a, b, d) > 0.0)
                && (side(c, d, a) > 0.0) != (side(c, d, b) > 0.0)
        })
        .count();
    crossings % 2 == 1
}

/// One seed's river layout and its map, or the generator's refusal of it.
fn river_seed(
    map_type: MapType,
    size: MapSize,
    seed: u64,
) -> Result<(MapPlan, MapDefinition), Vec<Diagnostic>> {
    let mut source: serde_json::Value = serde_json::from_str(PRESETS).unwrap();
    for map_type in ["open", "mixed", "metro"] {
        source["types"][map_type]["river_chance"] = 1.into();
    }
    let presets = PresetDefinitions::from_json(&source.to_string()).unwrap();
    let catalogue = contract::templates::TemplateGeometryCatalog::new(Vec::new()).unwrap();
    let request = GenerationRequest {
        generator_version: mapgen::layout::GENERATOR_VERSION.into(),
        preset_revision: presets.revision.clone(),
        seed: seed.into(),
        template_catalog_hash: catalogue.hash().into(),
        map_type,
        size,
        profile: contract::generation::GenerationProfile::Standard,
        region: None,
        limits: CompileLimits {
            max_authored_parts: 0,
            max_bay_positions: 0,
            max_ground_points: 200_000,
        },
    };
    let plan = generate_layout(&request, &presets)?;
    let map = mapgen::lower(
        &mapgen::CompileRequest::generated(&request, plan.clone()),
        &catalogue,
    )
    .unwrap()
    .map;
    Ok((plan, map))
}

/// The point `along` metres down a deck's heading from its centre and
/// `across` metres to its left.
fn on_deck(bridge: &Bridge, along: f64, across: f64) -> [f64; 2] {
    let (sin, cos) = bridge.yaw.sin_cos();
    [
        bridge.center[0] + cos * along - sin * across,
        bridge.center[1] + sin * along + cos * across,
    ]
}

/// Every bridge of a generated map is a crossing in the world: a mover can
/// stand on the land before each end, across the deck's width; it steps onto
/// the deck from that land's own height; the deck is ground over water from
/// end to end; and the game's own road graph reaches every settlement from
/// the map's centre, which on a river map it can only do by the decks.
#[test]
fn a_generated_bridge_is_stepped_onto_from_dry_land_and_carries_the_roads_over() {
    let rules: contract::scenario::Rules =
        serde_json::from_value(sim::fixtures::test_game()).unwrap();
    let mut bridges = 0;
    for (map_type, size) in [
        (MapType::Open, MapSize::Medium),
        (MapType::Mixed, MapSize::Medium),
        (MapType::Metro, MapSize::Medium),
        (MapType::Open, MapSize::Large),
    ] {
        let (name, plan, map) = river_map(map_type, size);
        let world = WorldGeometry::new(&map, &rules);
        assert!(
            !map.bridges.is_empty(),
            "{name}: a river map with no bridge"
        );
        for bridge in &map.bridges {
            let [half_length, half_width] = bridge.half_extents;
            let lanes = [-0.9 * half_width, 0.0, 0.9 * half_width];
            for end in [-1.0, 1.0] {
                for across in lanes {
                    // The last 25 m of land before the deck, a step at a time.
                    let mut out = half_length + 25.0;
                    while out > half_length {
                        let [x, y] = on_deck(bridge, end * out, across);
                        let land = world.surface_at(x, y).unwrap();
                        assert!(
                            land.kind != SurfaceKind::Water
                                && land.kind != SurfaceKind::Bridge
                                && land.traversable,
                            "{name}: no standing {out} m from the deck at {:?}: {land:?}",
                            bridge.center
                        );
                        out -= 0.25;
                    }
                    // Onto the deck from the land's own height: the deck
                    // stands its `deck_z` above flat ground.
                    let [x, y] = on_deck(bridge, end * (half_length + 0.01), across);
                    let step = bridge.deck_z - world.height_at(x, y).unwrap();
                    assert!(
                        (step - bridge.deck_z).abs() < 1e-6,
                        "{name}: a {step} m step onto the deck at {:?}",
                        bridge.center
                    );
                }
            }
            let mut wet = false;
            let mut along = -half_length + 0.01;
            while along < half_length {
                for across in lanes {
                    let [x, y] = on_deck(bridge, along, across);
                    let deck = world.surface_at(x, y).unwrap();
                    assert_eq!(deck.kind, SurfaceKind::Bridge, "{name}");
                    assert!(deck.traversable && world.traversable_at(x, y), "{name}");
                    wet |= world.ground_surface_at(x, y).unwrap().kind == SurfaceKind::Water;
                }
                along += 0.5;
            }
            assert!(
                wet,
                "{name}: the deck at {:?} is over no water",
                bridge.center
            );
            bridges += 1;
        }

        // The road graph a long move asks: every node the main junction
        // reaches. The main settlement stands on it.
        let roads = RoadNet::build(&world);
        let [x, y] = plan.settlements[0].center;
        let centre = v2(x, y);
        let mut reached = BTreeSet::new();
        let mut frontier: Vec<u32> = roads
            .near(centre, 120.0)
            .iter()
            .flat_map(|access| roads.arc(access.arc).ends)
            .collect();
        assert!(!frontier.is_empty(), "{name}: no road at the centre");
        while let Some(node) = frontier.pop() {
            if reached.insert(node) {
                frontier.extend(
                    roads
                        .arcs_at(node)
                        .iter()
                        .flat_map(|arc| roads.arc(*arc).ends),
                );
            }
        }
        for settlement in &plan.settlements {
            let [x, y] = settlement.center;
            let reach = settlement
                .outline
                .iter()
                .map(|p| (p[0] - x).hypot(p[1] - y))
                .fold(0.0, f64::max);
            // A road through its ground or along its edge (a town's edge is
            // often a road) that the centre's network reaches.
            let on_network = roads.near(v2(x, y), reach).iter().any(|access| {
                let at = [access.at.x, access.at.y];
                let on_ground = contract::ground::polygon_contains(&settlement.outline, at)
                    || contract::ground::edges(&settlement.outline)
                        .any(|(a, b)| contract::ground::segment_distance(*a, *b, at) <= 5.0);
                on_ground && reached.contains(&roads.arc(access.arc).ends[0])
            });
            assert!(on_network, "{name}: {} is cut off", settlement.id);
        }
    }
    assert!(bridges >= 8, "{bridges} bridges on four river maps");
}
