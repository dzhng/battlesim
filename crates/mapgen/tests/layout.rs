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

/// C53 places buildings; a layout has none, so no template is needed yet.
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
            assert!(exits.len() >= 4);
            assert!(!skeletons.contains(&exits), "{map_type:?} {size:?}");
            skeletons.push(exits);
        }
    }
}

/// M05: a bigger map has more places, not bigger ones.
#[test]
fn a_bigger_map_adds_settlements_and_keeps_their_dimensions() {
    let presets = presets();
    for map_type in [MapType::Open, MapType::Mixed] {
        for seed in SEEDS {
            let plans = SIZES.map(|size| plan(map_type, size, seed));
            assert!(plans[0].settlements.len() < plans[1].settlements.len());
            assert!(plans[1].settlements.len() < plans[2].settlements.len());
            for plan in &plans {
                for settlement in &plan.settlements {
                    let [low, high] = presets.classes[&settlement.class].area_ha.unwrap();
                    let hectares = ring_area(&settlement.outline) / 1e4;
                    assert!(
                        hectares > low * 0.97 && hectares < high * 1.03,
                        "{} is {hectares} ha, outside {low}..{high}",
                        settlement.class
                    );
                }
            }
        }
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
            // Larger in the ground it covers and in what is built on it.
            for (measure, margin) in [(ring_area as fn(&[[f64; 2]]) -> f64, 2.0), (|_| 0.0, 0.0)] {
                let mut areas: Vec<f64> = plan
                    .settlements
                    .iter()
                    .map(|s| {
                        if margin > 0.0 {
                            measure(&s.outline)
                        } else {
                            built(s)
                        }
                    })
                    .collect();
                let margin = if margin > 0.0 { margin } else { 1.5 };
                areas.sort_by(|a, b| b.total_cmp(a));
                assert!(
                    areas[0] > margin * areas[1],
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
        assert_eq!(metrics.transit.len(), 4, "{name}");
        for edge in &metrics.transit {
            // Every edge reaches the centre by road inside the presets' limit.
            assert!(edge.elapsed_s <= presets.transit.max_s, "{name}: {edge:?}");
            assert!(edge.connector_m <= 100.0, "{name}: {edge:?}");
        }
    });
}

#[test]
fn an_approach_in_the_plan_is_really_open_ground() {
    every_cell(|_, _, _, plan| {
        for approach in &plan.approaches {
            let settlement = &plan.settlements[approach.settlement];
            let middle = (approach.from_rad + approach.to_rad) / 2.0;
            let direction = [libm::cos(middle), libm::sin(middle)];
            // Walk the wedge's middle ray: past the settlement's own edge it
            // meets no other settlement and no forest for the stated depth.
            let mut inside = true;
            let mut open = 0.0;
            let mut distance = 0.0;
            while open < approach.depth_m - 30.0 {
                distance += 10.0;
                let point = [
                    settlement.center[0] + direction[0] * distance,
                    settlement.center[1] + direction[1] * distance,
                ];
                if inside {
                    inside = contract::ground::polygon_contains(&settlement.outline, point);
                    continue;
                }
                open += 10.0;
                assert!((0.0..=plan.size[0]).contains(&point[0]));
                assert!((0.0..=plan.size[1]).contains(&point[1]));
                assert!(!plan
                    .settlements
                    .iter()
                    .any(|s| contract::ground::polygon_contains(&s.outline, point)));
                assert!(!plan.forests.iter().any(|f| f.shape.contains(point, 0.0)));
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
                contract::ground::validate_ring(&district.ring).unwrap();
                assert!(ids.insert((map_type, size, seed, district.id.clone())));
                assert!(district.id.starts_with(&settlement.id));
                assert!((district.area_m2 - ring_area(&district.ring)).abs() <= 0.5);
                let anchor = district.anchor;
                assert!(contract::ground::polygon_contains(&district.ring, anchor));
                assert!(contract::ground::polygon_contains(
                    &settlement.outline,
                    anchor
                ));
                for other in &settlement.districts[index + 1..] {
                    assert!(
                        !contract::ground::polygon_contains(&other.ring, anchor)
                            || other.ring.contains(&anchor),
                        "{} overlaps {}",
                        district.id,
                        other.id
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
/// reaching in between them, not a filled disc.
#[test]
fn larger_settlements_leave_green_gaps_and_woods_reach_into_them() {
    let mut shares = Vec::new();
    let mut wooded = 0;
    for map_type in [MapType::Mixed, MapType::Metro] {
        for seed in 1..=12 {
            let plan = plan(map_type, MapSize::Medium, seed);
            let main = &plan.settlements[0];
            shares.push(built(main) / ring_area(&main.outline));
            let near = |ring: &[[f64; 2]]| {
                ring.iter()
                    .all(|p| contract::ground::polygon_contains(&main.outline, *p))
            };
            wooded += usize::from(plan.forests.iter().any(|forest| match &forest.shape {
                GroundShape::Polygon { ring } => near(ring),
                GroundShape::Stroke { .. } => false,
            }));
        }
    }
    let mean = shares.iter().sum::<f64>() / shares.len() as f64;
    assert!((0.55..0.92).contains(&mean), "built shares {shares:?}");
    assert!(shares.iter().all(|share| *share < 0.999), "{shares:?}");
    // Not every seed: a wood needs an open sector and room in its half.
    assert!(wooded >= 8, "{wooded} of 24 main settlements hold a wood");
}

/// A settlement a country road serves is strung on it: the road runs on
/// through the settlement as its main street, rather than ending in its
/// middle or touching its edge.
#[test]
fn a_road_runs_through_every_settlement_it_serves() {
    let presets = presets();
    every_cell(|map_type, size, seed, plan| {
        for settlement in &plan.settlements {
            if presets.classes[&settlement.class].road != SurfaceKind::CountryRoad {
                continue;
            }
            let inside = |p: &[f64; 2]| contract::ground::polygon_contains(&settlement.outline, *p);
            // Either one road has authored points on both sides of one inside
            // the settlement, or two roads leave its centre.
            let through = road_runs(plan).any(|(kind, points)| {
                kind == SurfaceKind::CountryRoad && points[1..points.len() - 1].iter().any(inside)
            });
            let leaving = road_runs(plan)
                .filter(|(kind, points)| {
                    *kind == SurfaceKind::CountryRoad && points[0] == settlement.center
                })
                .count();
            assert!(
                through || leaving >= 2,
                "{map_type:?} {size:?} {seed}: {} is a dead end",
                settlement.id
            );
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
            // Where a town has districts to choose between: not its one
            // centre, and not a village that is all one district.
            let choices = plan
                .settlements
                .iter()
                .filter(|s| s.districts.len() > 1)
                .flat_map(|s| &s.districts[1..]);
            for district in choices {
                let on_road = contract::ground::edges(&district.ring)
                    .any(|(a, b)| roads.iter().any(|road| crosses(*road, [*a, *b])));
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

/// A preset the map cannot hold must end in a named refusal, quickly, never in
/// a different seed or a quietly thinner map.
#[test]
fn impossible_presets_end_in_a_named_diagnostic() {
    let crowded = presets_with(|source| {
        source["types"]["mixed"]["sizes"]["small"]["settlements"]["town"] =
            serde_json::json!([60, 60]);
        source["types"]["mixed"]["sizes"]["small"]["urban_share_max"] = serde_json::json!(1.0);
    })
    .unwrap();
    let started = std::time::Instant::now();
    let errors =
        generate_layout(&request(MapType::Mixed, MapSize::Small, 5), &crowded).unwrap_err();
    assert!(started.elapsed().as_secs() < 5);
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
    let filled = presets_with(|source| {
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
        source["classes"]["village"]["bands"][0]["districts"] = serde_json::json!({ "core": 1 });
    })
    .unwrap_err();
    assert!(errors.contains("highrise"), "{errors}");
    for (pointer, value) in [
        ("/fairness/town/rel", serde_json::json!(-0.1)),
        ("/approach/depth_m", serde_json::json!(0)),
        ("/classes/town/area_ha", serde_json::json!([200, 100])),
        ("/classes/town/bands/1/to", serde_json::json!(0.9)),
        ("/classes/town/bands/0/open", serde_json::json!(0.5)),
        (
            "/districts/centre",
            serde_json::json!({ "attached_home": 1, "industry": 1, "farmstead": 1 }),
        ),
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
        let request = request(map_type, size, seed).compile_request(plan.clone());
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
