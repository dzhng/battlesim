mod common;

use contract::ground::GroundShape;
use contract::map::SurfaceKind;
use mapgen::layout::{
    generate_layout, measure, GenerationRequest, MapSize, MapType, PresetDefinitions,
};
use mapgen::{CompileLimits, DiagnosticCode, MapPlan};

const PRESETS: &str = include_str!("../../../fixtures/map-presets.json");
const TYPES: [MapType; 3] = [MapType::Open, MapType::Mixed, MapType::Metro];
const SIZES: [MapSize; 3] = [MapSize::Small, MapSize::Medium, MapSize::Large];
/// Every cell runs these seeds: a claim about the generator is a claim about
/// all of them, not one lucky layout.
const SEEDS: [u64; 6] = [1, 2, 3, 4, 5, u64::MAX];

/// A layout has no buildings (the parcel pass places them), so no template.
fn empty_catalogue() -> contract::templates::TemplateGeometryCatalog {
    contract::templates::TemplateGeometryCatalog::new(Vec::new()).unwrap()
}

fn presets() -> PresetDefinitions {
    PresetDefinitions::from_json(PRESETS).unwrap()
}

fn request(map_type: MapType, size: MapSize, seed: u64) -> GenerationRequest {
    GenerationRequest {
        generator_version: mapgen::layout::GENERATOR_VERSION.into(),
        preset_revision: presets().revision,
        seed: seed.into(),
        template_catalog_hash: empty_catalogue().hash().into(),
        map_type,
        size,
        limits: CompileLimits {
            max_authored_parts: 0,
            max_bay_positions: 0,
            max_ground_points: 200_000,
        },
    }
}

fn plan(map_type: MapType, size: MapSize, seed: u64) -> MapPlan {
    generate_layout(&request(map_type, size, seed), &presets())
        .unwrap_or_else(|errors| panic!("{map_type:?} {size:?} seed {seed}: {errors:?}"))
}

fn every_cell(mut check: impl FnMut(MapType, MapSize, u64, &MapPlan)) {
    for map_type in TYPES {
        for size in SIZES {
            for seed in SEEDS {
                check(map_type, size, seed, &plan(map_type, size, seed));
            }
        }
    }
}

fn ring_area(ring: &[[f64; 2]]) -> f64 {
    (contract::ground::polygon_area(ring) / 2.0).abs()
}

/// A settlement's built ground: its districts, not the fields between them.
fn built(settlement: &mapgen::SettlementPlan) -> f64 {
    settlement
        .districts
        .iter()
        .map(|d| ring_area(&d.ring))
        .sum()
}

fn segment_distance(a: [f64; 2], b: [f64; 2], p: [f64; 2]) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let t = (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
    (a[0] + dx * t - p[0]).hypot(a[1] + dy * t - p[1])
}

/// Inside the ring or within a metre of its edge: a town's edge is often a
/// road, and a point of that road is the town's.
fn on_ground(ring: &[[f64; 2]], p: [f64; 2]) -> bool {
    contract::ground::polygon_contains(ring, p)
        || contract::ground::edges(ring).any(|(a, b)| segment_distance(*a, *b, p) <= 1.0)
}

/// The convex hull of a ring, counter-clockwise.
fn hull(ring: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let mut sorted = ring.to_vec();
    sorted.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    let mut hull: Vec<[f64; 2]> = Vec::new();
    for pass in 0..2 {
        let floor = hull.len();
        for p in &sorted {
            while hull.len() >= floor + 2
                && contract::ground::cross(hull[hull.len() - 2], hull[hull.len() - 1], *p) <= 0.0
            {
                hull.pop();
            }
            hull.push(*p);
        }
        hull.pop();
        if pass == 0 {
            sorted.reverse();
        }
    }
    hull
}

fn road_runs(plan: &MapPlan) -> impl Iterator<Item = (SurfaceKind, &[[f64; 2]])> {
    plan.surfaces.iter().map(|area| match &area.shape {
        GroundShape::Stroke { centerline, .. } => (area.kind, centerline.control_points()),
        GroundShape::Polygon { .. } => panic!("a road is a stroke"),
    })
}

#[test]
fn a_size_is_its_exact_playable_extent_at_every_type() {
    for (size, metres) in [
        (MapSize::Small, 6_000.0),
        (MapSize::Medium, 8_000.0),
        (MapSize::Large, 10_000.0),
    ] {
        for map_type in TYPES {
            assert_eq!(plan(map_type, size, 1).size, [metres, metres]);
        }
    }
}

#[test]
fn a_fixed_request_gives_the_same_plan_bytes_every_run() {
    for map_type in TYPES {
        let first = serde_json::to_string(&plan(map_type, MapSize::Medium, 7)).unwrap();
        let again = serde_json::to_string(&plan(map_type, MapSize::Medium, 7)).unwrap();
        assert_eq!(first, again);
        let other = serde_json::to_string(&plan(map_type, MapSize::Medium, 8)).unwrap();
        assert_ne!(first, other, "the seed must move the layout");
    }
}

/// One seed is nine different maps: the type and size are part of what the
/// seed draws from, so no two of them share a road skeleton or a town shape.
#[test]
fn one_seed_gives_each_type_and_size_its_own_layout() {
    let mut skeletons = Vec::new();
    for map_type in TYPES {
        for size in SIZES {
            let plan = plan(map_type, size, 3);
            // Where roads leave the map, as shares of the edge.
            let exits: Vec<[i64; 2]> = road_runs(&plan)
                .map(|(_, points)| points[0])
                .filter(|p| p.iter().any(|v| *v == 0.0 || *v == plan.size[0]))
                .map(|p| p.map(|v| (v / plan.size[0] * 1e4) as i64))
                .collect();
            // At least the road in from the bottom and the one from the top.
            assert!(exits.len() >= 2);
            assert!(!skeletons.contains(&exits), "{map_type:?} {size:?}");
            skeletons.push(exits);
        }
    }
}

/// M05: a bigger map has more places, not bigger ones. A settlement's
/// districts stand on its class's ground, whatever the map's size, and a
/// class's settlements are as large on a Large map as on a Small one.
#[test]
fn a_bigger_map_adds_settlements_and_keeps_their_dimensions() {
    let presets = presets();
    // Each class's outlines, in hectares, by map size.
    let mut sizes: std::collections::BTreeMap<String, [Vec<f64>; 3]> = Default::default();
    for map_type in [MapType::Open, MapType::Mixed] {
        for seed in SEEDS {
            let plans = SIZES.map(|size| plan(map_type, size, seed));
            assert!(plans[0].settlements.len() < plans[1].settlements.len());
            assert!(plans[1].settlements.len() < plans[2].settlements.len());
            for (size, plan) in plans.iter().enumerate() {
                for settlement in &plan.settlements {
                    let [_, high] = presets.classes[&settlement.class].area_ha.unwrap();
                    let hectares = ring_area(&settlement.outline) / 1e4;
                    assert!(
                        hectares < high * 1.03,
                        "{} is {hectares} ha, above its class's {high}",
                        settlement.class
                    );
                    sizes.entry(settlement.class.clone()).or_default()[size].push(hectares);
                }
            }
        }
    }
    // The classes every size has many of: villages and hamlets.
    for class in ["village", "hamlet"] {
        let medians = sizes[class].clone().map(|mut hectares| {
            hectares.sort_by(f64::total_cmp);
            hectares[hectares.len() / 2]
        });
        let (least, most) = (
            medians.iter().copied().fold(f64::INFINITY, f64::min),
            medians.iter().copied().fold(0.0, f64::max),
        );
        assert!(
            most < 1.4 * least,
            "{class}: median hectares by size {medians:?}"
        );
    }
    // Road widths are the other dimension a size must not scale.
    let widths = SIZES.map(|size| {
        let mut widths: Vec<_> = plan(MapType::Mixed, size, 1)
            .surfaces
            .iter()
            .map(|area| match &area.shape {
                GroundShape::Stroke { width_m, .. } => (area.kind, *width_m),
                GroundShape::Polygon { .. } => panic!("a road is a stroke"),
            })
            .collect();
        widths.sort_by(|a, b| a.partial_cmp(b).unwrap());
        widths.dedup();
        widths
    });
    assert_eq!(widths[0], widths[1]);
    assert_eq!(widths[1], widths[2]);
}

/// M06: Metro is one dominant city at the centre with smaller places around it.
#[test]
fn metro_has_one_dominant_central_city() {
    for size in SIZES {
        for seed in SEEDS {
            let plan = plan(MapType::Metro, size, seed);
            let mut areas: Vec<f64> = plan.settlements.iter().map(built).collect();
            let city = areas[0];
            let total: f64 = areas.iter().sum();
            areas.sort_by(|a, b| b.total_cmp(a));
            assert_eq!(areas[0], city, "the first settlement is the city");
            assert!(
                city > 0.6 * total,
                "{size:?} {seed}: city {city} of {total}"
            );
            assert!(city > 4.0 * areas.get(1).copied().unwrap_or(0.0));
            let centre = [plan.size[0] / 2.0, plan.size[1] / 2.0];
            assert!(contract::ground::polygon_contains(
                &plan.settlements[0].outline,
                centre
            ));
        }
    }
}

/// M21: Mixed has one town clearly larger than every other settlement.
#[test]
fn mixed_has_one_clearly_larger_town() {
    for size in SIZES {
        for seed in SEEDS {
            let plan = plan(MapType::Mixed, size, seed);
            // Half as large again as the next, in the ground it covers and
            // in what is built on it: the least the preset sizes allow.
            let outline = |s: &mapgen::SettlementPlan| ring_area(&s.outline);
            for measure in [outline as fn(&mapgen::SettlementPlan) -> f64, built] {
                let mut areas: Vec<f64> = plan.settlements.iter().map(measure).collect();
                areas.sort_by(|a, b| b.total_cmp(a));
                assert!(
                    areas[0] > 1.5 * areas[1],
                    "{size:?} {seed}: {} against {}",
                    areas[0],
                    areas[1]
                );
            }
        }
    }
}

/// M07: a rural map cannot acquire a tower or a seven-storey block.
#[test]
fn open_never_selects_towers_and_only_metro_selects_highrises() {
    use contract::templates::BuildingCategory::Highrise;
    every_cell(|map_type, size, seed, plan| {
        let districts = plan.settlements.iter().flat_map(|s| &s.districts);
        let mut highrise = false;
        for district in districts {
            highrise |= district.categories.iter().any(|c| c.category == Highrise);
            match map_type {
                MapType::Open => assert!(district.max_floors.unwrap() <= 6),
                MapType::Mixed => assert!(district.max_floors.unwrap() <= 8),
                MapType::Metro => (),
            }
        }
        assert_eq!(
            highrise,
            map_type == MapType::Metro,
            "{map_type:?} {size:?} {seed}"
        );
    });
}

/// The measured rules every generated plan must meet: road access, top/bottom
/// fairness, the one open-approach rule and the Large transit target.
#[test]
fn every_plan_is_connected_fair_approachable_and_quick_to_cross() {
    let presets = presets();
    every_cell(|map_type, size, seed, plan| {
        let metrics = measure(plan, &presets);
        let name = format!("{map_type:?} {size:?} seed {seed}");
        assert_eq!(metrics.roads.unconnected_settlements, 0, "{name}");
        // S7's metric, restated here so a preset edit cannot loosen it.
        let playable = plan.size[0] * plan.size[1];
        let town = &metrics.town;
        assert!(
            (town.top_m2 - town.bottom_m2).abs()
                <= (0.10 * (town.top_m2 + town.bottom_m2)).max(0.004 * playable),
            "{name}: town {town:?}"
        );
        let forest = &metrics.forest;
        assert!(
            (forest.top_m2 - forest.bottom_m2).abs()
                <= (0.20 * (forest.top_m2 + forest.bottom_m2)).max(0.005 * playable),
            "{name}: forest {forest:?}"
        );
        // M19: one rule at every type and size.
        for half in [mapgen::Half::Top, mapgen::Half::Bottom] {
            assert!(
                plan.approaches.iter().any(|a| a.settlement == 0
                    && a.half == half
                    && a.depth_m >= 1_800.0
                    && a.front_m >= 400.0),
                "{name}: no {half:?} approach to the main settlement"
            );
        }
        // M22: the two sides start at the top and the bottom. Each of those
        // edges reaches the main junction, by the map's centre, inside the
        // presets' limit, and a road joins the two edges.
        let transit = &metrics.transit;
        for journey in [&transit.top, &transit.bottom] {
            let journey = journey
                .as_ref()
                .unwrap_or_else(|| panic!("{name}: {transit:?}"));
            assert!(
                journey.elapsed_s <= presets.transit.max_s,
                "{name}: {transit:?}"
            );
        }
        assert!(transit.centre_m <= 500.0, "{name}: {transit:?}");
        let through = transit
            .top_bottom
            .as_ref()
            .unwrap_or_else(|| panic!("{name}: {transit:?}"));
        assert!(through.route_m >= plan.size[1], "{name}: {transit:?}");
    });
}

/// M22: a road across the middle from side to side is optional. A share of
/// maps has one and a share has none, at every size.
#[test]
fn a_share_of_maps_has_a_road_from_side_to_side_and_a_share_has_none() {
    let presets = presets();
    for (map_type, size) in [
        (MapType::Open, MapSize::Small),
        (MapType::Mixed, MapSize::Medium),
        (MapType::Metro, MapSize::Large),
    ] {
        let seeds = 40;
        let with = (1..=seeds)
            .filter(|seed| {
                measure(&plan(map_type, size, *seed), &presets)
                    .transit
                    .east_west
                    .is_some()
            })
            .count();
        // The presets draw one on half of maps; forty seeds stray from that
        // by eight maps, one time in a hundred.
        assert!(
            (12..=28).contains(&with),
            "{map_type:?} {size:?}: {with} of {seeds} maps have a road from side to side"
        );
    }
}

/// M21, M22: the main roads do not meet in the same crossroads on every
/// map, on a Large map no more than on a Small one.
#[test]
fn main_roads_meet_in_a_crossroads_on_some_maps_and_fork_on_others() {
    let presets = presets();
    for map_type in TYPES {
        for size in SIZES {
            let crossroads = (1..=30)
                .filter(|seed| {
                    measure(&plan(map_type, size, *seed), &presets)
                        .roads
                        .centre_roads
                        >= 4
                })
                .count();
            assert!(
                (1..=24).contains(&crossroads),
                "{map_type:?} {size:?}: {crossroads} of 30 maps have a central crossroads"
            );
        }
    }
}

/// An approach is a corridor of open ground: along any of its bearings,
/// past the last of the settlement's own ground inside the corridor, no
/// settlement and no forest stands across the stated front for the stated
/// depth.
#[test]
fn an_approach_in_the_plan_is_really_open_ground() {
    every_cell(|_, _, _, plan| {
        for approach in &plan.approaches {
            let settlement = &plan.settlements[approach.settlement];
            // The first and the last bearing of the run are ones it measured.
            for bearing in [approach.from_rad, approach.to_rad] {
                let toward = [libm::cos(bearing), libm::sin(bearing)];
                let aside = [-toward[1], toward[0]];
                let at = |along: f64, across: f64| {
                    [
                        settlement.center[0] + toward[0] * along + aside[0] * across,
                        settlement.center[1] + toward[1] * along + aside[1] * across,
                    ]
                };
                let lanes = [-approach.front_m / 2.0, 0.0, approach.front_m / 2.0];
                // The settlement's edge: the last of its ground along any
                // lane of the corridor, to the metre.
                let edge = (0..=approach.front_m as usize / 10)
                    .map(|lane| lane as f64 * 10.0 - approach.front_m / 2.0)
                    .flat_map(|across| (0..600).map(move |step| (f64::from(step) * 10.0, across)))
                    .filter(|(along, across)| {
                        contract::ground::polygon_contains(&settlement.outline, at(*along, *across))
                    })
                    .map(|(along, _)| along)
                    .fold(0.0, f64::max);
                for across in lanes {
                    let mut open = 20.0;
                    while open < approach.depth_m - 20.0 {
                        let point = at(edge + open, across);
                        assert!((0.0..=plan.size[0]).contains(&point[0]));
                        assert!((0.0..=plan.size[1]).contains(&point[1]));
                        assert!(!plan
                            .settlements
                            .iter()
                            .any(|s| contract::ground::polygon_contains(&s.outline, point)));
                        assert!(!plan.forests.iter().any(|f| f.shape.contains(point, 0.0)));
                        open += 10.0;
                    }
                }
            }
        }
    });
}

/// A district is one use (with at most a minor second), sits inside its
/// settlement, overlaps no other, and can be addressed on its own: the
/// encounter planner puts objectives on districts.
#[test]
fn districts_are_single_use_addressable_pieces_of_their_settlement() {
    let mut ids = std::collections::BTreeSet::new();
    every_cell(|map_type, size, seed, plan| {
        for settlement in &plan.settlements {
            assert!(!settlement.districts.is_empty());
            assert!(built(settlement) <= ring_area(&settlement.outline) * 1.001);
            for (index, district) in settlement.districts.iter().enumerate() {
                contract::ground::validate_ring(&district.ring).unwrap_or_else(|error| {
                    panic!("{}: {error}: {:?}", district.id, district.ring)
                });
                assert!(ids.insert((map_type, size, seed, district.id.clone())));
                assert!(district.id.starts_with(&settlement.id));
                // Whole square metres: a ring on the centimetre grid can
                // measure exactly half a metre over, which rounds either way.
                assert!((district.area_m2 - ring_area(&district.ring)).abs() <= 0.5 + 1e-6);
                let anchor = district.anchor;
                assert!(contract::ground::polygon_contains(&district.ring, anchor));
                assert!(contract::ground::polygon_contains(
                    &settlement.outline,
                    anchor
                ));
                for other in &settlement.districts[index + 1..] {
                    assert!(
                        // The plan is rounded to centimetres, like parcels.
                        common::overlap_depth(&district.ring, &other.ring) < 0.02,
                        "{map_type:?} {size:?} seed {seed}: {} overlaps {} by {} m; {:?} vs {:?}",
                        district.id,
                        other.id,
                        common::overlap_depth(&district.ring, &other.ring),
                        district.ring,
                        other.ring
                    );
                }
                let total: f64 = district.categories.iter().map(|c| c.weight).sum();
                assert!(district.categories.len() <= 2);
                assert!(
                    district.categories[0].weight >= 0.75 * total,
                    "{} blends {:?}",
                    district.kind,
                    district.categories
                );
            }
        }
    });
}

/// A larger settlement is a loose group of districts with fields and woods
/// reaching in between them, not a filled shape: its districts cover only
/// part of the ground they span (their hull), and a wood often reaches
/// into that ground.
#[test]
fn larger_settlements_leave_green_gaps_and_woods_reach_into_them() {
    let mut shares = Vec::new();
    let mut wooded = 0;
    for map_type in [MapType::Mixed, MapType::Metro] {
        for seed in 1..=12 {
            let plan = plan(map_type, MapSize::Medium, seed);
            let main = &plan.settlements[0];
            let span = hull(&main.outline);
            shares.push(built(main) / ring_area(&span));
            let among = |ring: &[[f64; 2]]| {
                ring.iter()
                    .any(|p| contract::ground::polygon_contains(&span, *p))
            };
            wooded += usize::from(plan.forests.iter().any(|forest| match &forest.shape {
                GroundShape::Polygon { ring } => among(ring),
                GroundShape::Stroke { .. } => false,
            }));
        }
    }
    let mean = shares.iter().sum::<f64>() / shares.len() as f64;
    assert!((0.55..0.92).contains(&mean), "built shares {shares:?}");
    // One that grew evenly all round nearly fills the ground it spans.
    let loose = shares.iter().filter(|share| **share < 0.92).count();
    assert!(loose >= 18, "{loose} of 24 are loose: {shares:?}");
    // Not every seed: a wood needs an open block and room in its half.
    assert!(wooded >= 8, "{wooded} of 24 main settlements hold a wood");
}

/// A settlement a country road serves is strung on it: country road runs
/// through it or along its districts as its main street, for a hundred and
/// fifty metres or more, rather than ending at its edge.
#[test]
fn a_road_runs_through_every_settlement_it_serves() {
    let presets = presets();
    every_cell(|map_type, size, seed, plan| {
        let roads: Vec<&[[f64; 2]]> = plan
            .surfaces
            .iter()
            .filter(|area| area.kind == SurfaceKind::CountryRoad)
            .map(|area| match &area.shape {
                GroundShape::Stroke { centerline, .. } => centerline.samples(),
                GroundShape::Polygon { .. } => panic!("a road is a stroke"),
            })
            .collect();
        for settlement in &plan.settlements {
            if presets.classes[&settlement.class].road != SurfaceKind::CountryRoad {
                continue;
            }
            let [x0, y0, x1, y1] = contract::ground::limits(&settlement.outline, 300.0);
            // Walked ten metres at a time, where the road is near at all.
            let mut metres = 0.0;
            for run in roads.iter().flat_map(|road| road.windows(2)) {
                let near = |p: [f64; 2]| p[0] >= x0 && p[0] <= x1 && p[1] >= y0 && p[1] <= y1;
                if !near(run[0]) && !near(run[1]) {
                    continue;
                }
                let length = (run[1][0] - run[0][0]).hypot(run[1][1] - run[0][1]);
                let steps = (length / 10.0).ceil().max(1.0);
                for step in 0..steps as usize {
                    let share = (step as f64 + 0.5) / steps;
                    let p = [
                        run[0][0] + (run[1][0] - run[0][0]) * share,
                        run[0][1] + (run[1][1] - run[0][1]) * share,
                    ];
                    if on_ground(&settlement.outline, p) {
                        metres += length / steps;
                    }
                }
            }
            assert!(
                metres >= 150.0,
                "{map_type:?} {size:?} {seed}: {metres:.0} m of country road serve {}",
                settlement.id
            );
        }
    });
}

/// A country road or a track never reverses onto itself. Two that meet end
/// to end are one road round the bend they make, however sharp (a
/// switchback, whose outside is the stroke's own round corner), so a turn
/// may be anything short of the reversal the joint pass refuses to weld.
#[test]
fn no_road_turns_back_on_itself() {
    every_cell(|map_type, size, seed, plan| {
        for (kind, points) in road_runs(plan) {
            if kind == SurfaceKind::Road {
                continue;
            }
            for bend in points.windows(3) {
                // A bridge crosses its river square, whatever turn that
                // asks of the road at its ends.
                let on_bridge = plan.bridges.iter().any(|bridge| {
                    (bend[1][0] - bridge.center[0]).hypot(bend[1][1] - bridge.center[1])
                        <= bridge.half_extents[0] + 15.0
                });
                if on_bridge {
                    continue;
                }
                let heading = |a: [f64; 2], b: [f64; 2]| (b[1] - a[1]).atan2(b[0] - a[0]);
                let turn = (heading(bend[1], bend[2]) - heading(bend[0], bend[1])).abs();
                let turn = turn.min(std::f64::consts::TAU - turn).to_degrees();
                assert!(
                    turn <= 175.0,
                    "{map_type:?} {size:?} {seed}: a {kind:?} turns {turn:.0}° at {:?}",
                    bend[1]
                );
            }
        }
    });
}

/// And many stand on a main road in from the map's edge.
#[test]
fn many_settlements_stand_on_an_edge_road() {
    let (mut on_edge_road, mut all) = (0, 0);
    for map_type in [MapType::Mixed, MapType::Metro] {
        for seed in 1..=12 {
            let plan = plan(map_type, MapSize::Large, seed);
            for settlement in &plan.settlements[1..] {
                all += 1;
                on_edge_road += usize::from(road_runs(&plan).any(|(_, points)| {
                    let from_edge = points[0]
                        .iter()
                        .zip(plan.size)
                        .any(|(v, extent)| *v == 0.0 || *v == extent);
                    from_edge && points[1..points.len() - 1].contains(&settlement.center)
                }));
            }
        }
    }
    assert!(
        on_edge_road * 5 >= all,
        "{on_edge_road} of {all} settlements"
    );
}

fn crosses(a: [[f64; 2]; 2], b: [[f64; 2]; 2]) -> bool {
    let side = |p: [f64; 2], q: [f64; 2], r: [f64; 2]| contract::ground::cross(p, q, r).signum();
    side(a[0], a[1], b[0]) != side(a[0], a[1], b[1])
        && side(b[0], b[1], a[0]) != side(b[0], b[1], a[1])
}

/// Industry lines the main road: a district the road runs through is
/// industrial far more often than one away from it.
#[test]
fn industry_gathers_along_main_roads() {
    // [on a road, away] × [industrial, all]
    let mut counts = [[0usize; 2]; 2];
    for map_type in [MapType::Mixed, MapType::Metro] {
        for seed in 1..=12 {
            let plan = plan(map_type, MapSize::Large, seed);
            let roads: Vec<[[f64; 2]; 2]> = road_runs(&plan)
                .filter(|(kind, _)| *kind == SurfaceKind::CountryRoad)
                .flat_map(|(_, points)| points.windows(2).map(|run| [run[0], run[1]]))
                .collect();
            // A road bounds a district as often as it crosses one.
            let along = |a: [f64; 2], b: [f64; 2]| {
                let middle = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
                roads
                    .iter()
                    .any(|road| segment_distance(road[0], road[1], middle) <= 1.0)
            };
            // Where a town has districts to choose between: not its one
            // centre, and not a village that is all one district.
            let choices = plan
                .settlements
                .iter()
                .filter(|s| s.districts.len() > 1)
                .flat_map(|s| &s.districts[1..]);
            for district in choices {
                let on_road = contract::ground::edges(&district.ring).any(|(a, b)| {
                    along(*a, *b) || roads.iter().any(|road| crosses(*road, [*a, *b]))
                });
                let row = &mut counts[usize::from(!on_road)];
                row[0] += usize::from(district.kind == "industrial");
                row[1] += 1;
            }
        }
    }
    let [on_road, away] = counts.map(|[industrial, all]| industrial as f64 / all as f64);
    assert!(on_road > 2.0 * away, "{counts:?}");
}

/// Seeds must give different road patterns, not one crossroads redrawn.
#[test]
fn road_patterns_vary_between_a_network_and_a_few_corridors() {
    let presets = presets();
    for (map_type, size) in [
        (MapType::Open, MapSize::Medium),
        (MapType::Mixed, MapSize::Large),
    ] {
        let mut exits = Vec::new();
        let mut loops = Vec::new();
        for seed in 1..=30 {
            let metrics = measure(&plan(map_type, size, seed), &presets);
            exits.push(metrics.roads.edge_exits);
            loops.push(metrics.roads.loops);
        }
        // A few corridors make few loops; a network makes many. Thirty seeds
        // reach both ends of the range; the margins are a few loops wide.
        let (sparse, rich) = (loops.iter().min().unwrap(), loops.iter().max().unwrap());
        assert!(
            *sparse <= 3 && *rich >= sparse + 10,
            "{map_type:?}: loops {loops:?}"
        );
        assert!(
            exits.iter().min() < exits.iter().max(),
            "{map_type:?}: exits {exits:?}"
        );
    }
}

/// A cosmetic or forest-only change must not move settlements or roads.
#[test]
fn forest_presets_do_not_move_settlements_or_roads() {
    let mut source: serde_json::Value = serde_json::from_str(PRESETS).unwrap();
    source["types"]["mixed"]["forest_share"] = serde_json::json!([0.05, 0.06]);
    let changed = PresetDefinitions::from_json(&source.to_string()).unwrap();
    let request = request(MapType::Mixed, MapSize::Medium, 3);
    let before = generate_layout(&request, &presets()).unwrap();
    let after = generate_layout(&request, &changed).unwrap();
    let json = |value: &dyn erased::Json| value.json();
    assert_eq!(json(&before.settlements), json(&after.settlements));
    assert_eq!(json(&before.surfaces), json(&after.surfaces));
    assert_ne!(json(&before.forests), json(&after.forests));
}

mod erased {
    pub trait Json {
        fn json(&self) -> String;
    }
    impl<T: serde::Serialize> Json for T {
        fn json(&self) -> String {
            serde_json::to_string(self).unwrap()
        }
    }
}

fn presets_with(edit: impl FnOnce(&mut serde_json::Value)) -> Result<PresetDefinitions, String> {
    let mut source: serde_json::Value = serde_json::from_str(PRESETS).unwrap();
    edit(&mut source);
    PresetDefinitions::from_json(&source.to_string()).map_err(|errors| format!("{errors:?}"))
}

/// A preset the map cannot hold must end in a named refusal, never in a
/// different seed or a quietly thinner map. The presets' retry counts bound
/// the work; wall time is not asserted, because the machine is shared.
#[test]
fn impossible_presets_end_in_a_named_diagnostic() {
    let crowded = presets_with(|source| {
        source["types"]["mixed"]["sizes"]["small"]["settlements"]["town"] =
            serde_json::json!([60, 60]);
        source["types"]["mixed"]["sizes"]["small"]["urban_share_max"] = serde_json::json!(1.0);
    })
    .unwrap();
    let errors =
        generate_layout(&request(MapType::Mixed, MapSize::Small, 5), &crowded).unwrap_err();
    assert_eq!(errors[0].code, DiagnosticCode::GenerationFailed);
    let feature = errors[0].feature.as_deref().unwrap();
    assert!(feature.starts_with("settlement-"), "{feature}");
    assert_eq!(errors[0].location, "$.presets.types.mixed.sizes.small");
    assert!(
        errors[0].message.contains("seed 5"),
        "{}",
        errors[0].message
    );

    // A city that fills the map leaves no 1,800 m of open ground beside it.
    // (With no river to route: a river would be refused first.)
    let filled = presets_with(|source| {
        source["types"]["metro"]["river_chance"] = serde_json::json!(0);
        source["classes"]["city"]["area_share"] = serde_json::json!([0.5, 0.55]);
        source["types"]["metro"]["sizes"]["small"]["urban_share_max"] = serde_json::json!(1.0);
    })
    .unwrap();
    let errors = generate_layout(&request(MapType::Metro, MapSize::Small, 5), &filled).unwrap_err();
    assert_eq!(errors[0].code, DiagnosticCode::GenerationFailed);
    assert_eq!(errors[0].feature.as_deref(), Some("approach"));
}

#[test]
fn a_request_must_name_this_generator_and_these_presets() {
    let mut stale = request(MapType::Open, MapSize::Small, 1);
    stale.preset_revision = "layout-presets-0".into();
    let errors = generate_layout(&stale, &presets()).unwrap_err();
    assert_eq!(errors[0].code, DiagnosticCode::InvalidRequest);
    assert_eq!(errors[0].location, "$.preset_revision");
    let mut stale = request(MapType::Open, MapSize::Small, 1);
    stale.generator_version = "layout-0".into();
    let errors = generate_layout(&stale, &presets()).unwrap_err();
    assert_eq!(errors[0].location, "$.generator_version");
}

#[test]
fn a_plan_over_the_callers_ground_allowance_is_refused() {
    let mut tight = request(MapType::Mixed, MapSize::Large, 1);
    tight.limits.max_ground_points = 100;
    let errors = generate_layout(&tight, &presets()).unwrap_err();
    assert_eq!(errors[0].code, DiagnosticCode::ComplexityLimit);
    assert_eq!(errors[0].location, "$.limits.max_ground_points");
}

#[test]
fn presets_that_break_a_map_rule_are_refused_at_load() {
    // M07: Open may not reach a highrise through any of its districts.
    let errors = presets_with(|source| {
        source["classes"]["village"]["zones"][0]["districts"] = serde_json::json!({ "core": 1 });
    })
    .unwrap_err();
    assert!(errors.contains("highrise"), "{errors}");
    for (pointer, value) in [
        ("/fairness/town/rel", serde_json::json!(-0.1)),
        ("/approach/depth_m", serde_json::json!(0)),
        ("/classes/town/area_ha", serde_json::json!([200, 100])),
        ("/classes/town/zones/1/to", serde_json::json!(0.9)),
        ("/classes/town/built_share", serde_json::json!([0.5, 1.2])),
        // A block too shallow for any of the class's parcels.
        ("/classes/town/block/depth_m", serde_json::json!([20, 40])),
        ("/districts/farm/ground_m", serde_json::json!([60, 0])),
        (
            "/districts/farm/streets/surface",
            serde_json::json!("sidewalk"),
        ),
        ("/roads/cross_road_chance", serde_json::json!(1.5)),
        // A main junction drawn farther out than the centre reaches.
        ("/transit/centre_reach_m", serde_json::json!(100)),
        ("/towns/growth_noise", serde_json::json!(1)),
        (
            "/districts/centre/mix",
            serde_json::json!({ "attached_home": 1, "industry": 1, "farmstead": 1 }),
        ),
        // Parcels: a block narrower than its street, a bend that folds,
        // a negative setback, no regional family.
        (
            "/districts/centre/streets/block_depth_m",
            serde_json::json!(9),
        ),
        (
            "/districts/village/streets/bend",
            serde_json::json!({ "amplitude_m": 60, "wavelength_m": 300 }),
        ),
        (
            "/districts/village/streets/cross_skip",
            serde_json::json!(1),
        ),
        ("/districts/core/lots/side_m", serde_json::json!(-1)),
        ("/districts/core/lots/coverage", serde_json::json!(0)),
        ("/parcels/regional_families", serde_json::json!([])),
        ("/parcels/street_width_m", serde_json::json!(0)),
        ("/retries/fit", serde_json::json!(0)),
        ("/types/mixed/siting/on_road", serde_json::json!(0.9)),
        ("/types/open/centre/class", serde_json::json!("capital")),
        ("/retries/site", serde_json::json!(0)),
        ("/revision", serde_json::json!(" ")),
    ] {
        // Set the key on its parent, so a case may add a key the file omits.
        let (parent, key) = pointer.rsplit_once('/').unwrap();
        let result = presets_with(|source| source.pointer_mut(parent).unwrap()[key] = value);
        assert!(result.is_err(), "{pointer} was admitted");
    }
}

/// The generator writes plans for the existing compiler, not a second format.
#[test]
fn every_generated_plan_compiles_into_a_battle_map() {
    let catalogue = empty_catalogue();
    every_cell(|map_type, size, seed, plan| {
        let request =
            mapgen::CompileRequest::generated(&request(map_type, size, seed), plan.clone());
        let generated = mapgen::lower(&request, &catalogue)
            .unwrap_or_else(|errors| panic!("{map_type:?} {size:?} {seed}: {errors:?}"));
        assert_eq!(generated.map.size, plan.size);
        assert_eq!(generated.map.surfaces.len(), plan.surfaces.len());
        assert_eq!(generated.map.forests.len(), plan.forests.len());
        assert!(generated.report.ground_points <= request.limits.max_ground_points);
        assert_eq!(generated.identity.seed.value(), seed);
        let saved = serde_json::to_string(&generated.map).unwrap();
        serde_json::from_str::<contract::map::MapDefinition>(&saved).unwrap();
    });
}
