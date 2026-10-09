//! Rivers and bridges in generated layouts, judged on the plan and on the map
//! it compiles into, with the contract's own river arithmetic.
use contract::ground::GroundShape;
use contract::river::River;
use mapgen::layout::{
    generate_layout, measure, GenerationRequest, MapSize, MapType, PresetDefinitions,
};
use mapgen::{CompileLimits, Diagnostic, MapPlan};

#[path = "common/admitted.rs"]
mod admitted;

const PRESETS: &str = include_str!("../../../fixtures/map-presets.json");
const TYPES: [MapType; 3] = [MapType::Open, MapType::Mixed, MapType::Metro];
const SIZES: [MapSize; 3] = [MapSize::Medium, MapSize::Large, MapSize::Xl];

type Point = [f64; 2];

fn presets_with(edit: impl FnOnce(&mut serde_json::Value)) -> PresetDefinitions {
    let mut source: serde_json::Value = serde_json::from_str(PRESETS).unwrap();
    edit(&mut source);
    PresetDefinitions::from_json(&source.to_string()).unwrap()
}

/// The shipped presets with every type's river chance set to `chance`, so a
/// claim about rivers does not move when the shipped shares are tuned.
fn presets(chance: f64) -> PresetDefinitions {
    presets_with(|source| {
        for map_type in ["open", "mixed", "metro"] {
            source["types"][map_type]["river_chance"] = chance.into();
        }
    })
}

fn empty_catalogue() -> contract::templates::TemplateGeometryCatalog {
    contract::templates::TemplateGeometryCatalog::new(Vec::new()).unwrap()
}

fn request(map_type: MapType, size: MapSize, seed: u64) -> GenerationRequest {
    GenerationRequest {
        generator_version: mapgen::layout::GENERATOR_VERSION.into(),
        preset_revision: presets(0.0).revision,
        seed: seed.into(),
        template_catalog_hash: empty_catalogue().hash().into(),
        map_type,
        size,
        profile: contract::generation::GenerationProfile::Standard,
        region: None,
        limits: CompileLimits {
            max_authored_parts: 0,
            max_bay_positions: 0,
            max_ground_points: 200_000,
        },
    }
}

fn generate(
    presets: &PresetDefinitions,
    map_type: MapType,
    size: MapSize,
    seed: u64,
) -> Result<MapPlan, Vec<Diagnostic>> {
    generate_layout(&request(map_type, size, seed), presets)
}

/// Every type and size over its first `count` admitted seeds, with a river
/// on every map.
fn every_river_map(count: usize, mut check: impl FnMut(&str, &MapPlan)) {
    let presets = presets(1.0);
    for map_type in TYPES {
        for size in SIZES {
            let plans =
                admitted::admitted(1.., count, |seed| generate(&presets, map_type, size, seed));
            for (seed, plan) in &plans {
                check(&format!("{map_type:?} {size:?} seed {seed}"), plan);
            }
        }
    }
}

fn json(value: &impl serde::Serialize) -> String {
    serde_json::to_string(value).unwrap()
}

/// M11: the seed decides whether a map has a river, as often as its type's
/// presets say, and a river seldom costs a map. Generation may refuse a
/// seed by name, with a river or without (the game draws another); a rare
/// few are, and a seed refused either way says nothing about how often
/// rivers are drawn.
#[test]
fn the_seed_decides_whether_a_map_has_a_river() {
    let (never, half, always) = (presets(0.0), presets(0.5), presets(1.0));
    let (mut maps, mut with_river) = (0, 0);
    let (mut refused, mut refused_with_river) = (Vec::new(), Vec::new());
    for map_type in TYPES {
        for size in SIZES {
            for seed in 1..=40 {
                let name = format!("{map_type:?} {size:?} seed {seed}");
                let generated =
                    [&never, &half, &always].map(|presets| generate(presets, map_type, size, seed));
                let [without, half, always] = match generated {
                    [Err(errors), ..] => {
                        let feature = errors[0].feature.as_deref().unwrap_or_default();
                        assert!(
                            !["river", "bridges", "fairness.river"].contains(&feature),
                            "{name}: {errors:?}"
                        );
                        refused.push(name);
                        continue;
                    }
                    [Ok(without), Ok(half), Ok(always)] => [without, half, always],
                    [_, half, always] => {
                        refused_with_river.push((name, half.err(), always.err()));
                        continue;
                    }
                };
                assert_eq!(without.rivers.len(), 0, "{name}");
                assert_eq!(always.rivers.len(), 1, "{name}");
                with_river += half.rivers.len();
                maps += 1;
            }
        }
    }
    assert!(refused.len() <= 2, "refused without a river: {refused:?}");
    assert!(
        refused_with_river.len() <= 2,
        "admitted without a river, refused with one: {refused_with_river:?}"
    );
    // At one in two, five standard deviations either side of half the maps.
    let spread = 5.0 * (maps as f64 / 4.0).sqrt();
    let expected = maps as f64 / 2.0;
    assert!(
        (with_river as f64 - expected).abs() <= spread,
        "{with_river} of {maps} maps have a river"
    );
}

/// A river runs from the north edge to the south, so each half holds a like
/// length of it, and the compiler's own river rule accepts it.
#[test]
fn a_river_runs_from_the_north_edge_to_the_south_and_the_terrain_carries_it() {
    every_river_map(6, |name, plan| {
        let river = &plan.rivers[0];
        let points = river.points();
        let extent = plan.size[1];
        assert_eq!(
            points[0].xy[1], extent,
            "{name}: it starts on the north edge"
        );
        assert_eq!(points[points.len() - 1].xy[1], 0.0, "{name}");
        for point in points {
            assert!(point.xy[0] > 0.0 && point.xy[0] < plan.size[0], "{name}");
            assert!((0.0..=extent).contains(&point.xy[1]), "{name}");
        }
        let mut halves = [0.0, 0.0];
        for pair in river.samples().windows(2) {
            let middle = (pair[0].xy[1] + pair[1].xy[1]) / 2.0;
            let length = (pair[1].xy[0] - pair[0].xy[0]).hypot(pair[1].xy[1] - pair[0].xy[1]);
            halves[usize::from(middle < extent / 2.0)] += length;
        }
        let [top, bottom] = halves;
        assert!(top + bottom >= extent, "{name}: {top} + {bottom} m");
        assert!(
            (top - bottom).abs() <= 0.2 * (top + bottom),
            "{name}: {top} m of river in the top half, {bottom} m in the bottom"
        );
        let compiled = mapgen::lower(
            &mapgen::CompileRequest::generated(
                &request(MapType::Open, MapSize::Medium, 1),
                plan.clone(),
            ),
            &empty_catalogue(),
        )
        .unwrap_or_else(|errors| panic!("{name}: {errors:?}"));
        assert_eq!(json(&compiled.map.rivers), json(&plan.rivers), "{name}");
        contract::river::validate(&compiled.map).unwrap();
    });
}

/// The 64 m squares of the map within a square of a river's water: the
/// contract's distance walks every stretch of the river, so a check asks it
/// only of points that are near.
fn near_water(river: &River) -> std::collections::BTreeSet<[i64; 2]> {
    let mut squares = std::collections::BTreeSet::new();
    for sample in river.samples() {
        // Water reaches less than a square from its sample.
        let reach = 64.0;
        assert!(sample.half_width_m < reach);
        for x in [-reach, 0.0, reach] {
            for y in [-reach, 0.0, reach] {
                squares.insert(square([sample.xy[0] + x, sample.xy[1] + y]));
            }
        }
    }
    squares
}

fn square(p: Point) -> [i64; 2] {
    p.map(|v| (v / 64.0).floor() as i64)
}

/// How far a river's bank runs from its water's edge up to the land: the
/// freeboard over the gentlest grade it is authored with.
fn bank_m(river: &River) -> f64 {
    river
        .points()
        .iter()
        .map(|point| -river.surface_z() * (point.width_m / 2.0) / point.depth_m)
        .fold(0.0, f64::max)
}

/// The points of a closed ring's edge, at most `step` apart.
fn along_ring(ring: &[Point], step: f64) -> Vec<Point> {
    let mut points = Vec::new();
    for (a, b) in contract::ground::edges(ring) {
        let pieces = ((b[0] - a[0]).hypot(b[1] - a[1]) / step).ceil().max(1.0) as usize;
        for piece in 0..pieces {
            let t = piece as f64 / pieces as f64;
            points.push([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]);
        }
    }
    points
}

/// The least open ground between a ring's edge and the river's water: by the
/// contract's own distance, at every few metres of the ring. Rings far from
/// the river are passed over (`None`).
fn ring_clearance(river: &River, ring: &[Point]) -> Option<f64> {
    let [x0, y0, x1, y1] = contract::ground::limits(ring, 200.0);
    river
        .samples()
        .iter()
        .any(|s| s.xy[0] >= x0 && s.xy[0] <= x1 && s.xy[1] >= y0 && s.xy[1] <= y1)
        .then(|| {
            along_ring(ring, 4.0)
                .into_iter()
                .map(|p| -river.inside(p))
                .fold(f64::INFINITY, f64::min)
        })
}

/// A river is a hard feature: no settlement's ground and no wood reaches its
/// water or the bank cut beside it, and its line runs through none of them.
#[test]
fn no_settlement_and_no_wood_stands_in_the_water_or_on_its_bank() {
    let (mut beside, mut wooded) = (0, 0);
    every_river_map(4, |name, plan| {
        let river = &plan.rivers[0];
        let bank = bank_m(river);
        for settlement in &plan.settlements {
            for sample in river.samples() {
                assert!(
                    !contract::ground::polygon_contains(&settlement.outline, sample.xy),
                    "{name}: the river runs through {}",
                    settlement.id
                );
            }
            if let Some(clear) = ring_clearance(river, &settlement.outline) {
                assert!(
                    clear > bank,
                    "{name}: {} is {clear} m from the water",
                    settlement.id
                );
                beside += usize::from(clear < 100.0);
            }
        }
        for forest in &plan.forests {
            let GroundShape::Polygon { ring } = &forest.shape else {
                panic!("a wood is a polygon");
            };
            assert!(
                !river
                    .samples()
                    .iter()
                    .any(|s| forest.shape.contains(s.xy, 0.0)),
                "{name}: the river runs through a wood"
            );
            if let Some(clear) = ring_clearance(river, ring) {
                assert!(clear > bank, "{name}: a wood is {clear} m from the water");
                wooded += usize::from(clear < 100.0);
            }
        }
    });
    // The rule is exercised: settlements and woods do come down to the bank.
    assert!(
        beside >= 9 && wooded >= 9,
        "{beside} settlements and {wooded} woods beside a river"
    );
}

/// M19 with a river on the map. Water is not open ground: nothing crosses it
/// but at a bridge, so an approach a river cut would be a moat with a gate.
/// Every approach the plan records is dry, and the main settlement still has
/// one in each half.
#[test]
fn an_open_approach_has_no_water_in_it_and_the_main_settlement_keeps_one_in_each_half() {
    every_river_map(6, |name, plan| {
        let river = &plan.rivers[0];
        let near = near_water(river);
        for half in [mapgen::Half::Top, mapgen::Half::Bottom] {
            assert!(
                plan.approaches.iter().any(|a| a.settlement == 0
                    && a.half == half
                    && a.depth_m >= 1_800.0
                    && a.front_m >= 400.0),
                "{name}: no {half:?} approach to the main settlement"
            );
        }
        for approach in &plan.approaches {
            let settlement = &plan.settlements[approach.settlement];
            for share in [0.0, 0.25, 0.5, 0.75, 1.0] {
                let bearing = approach.from_rad + (approach.to_rad - approach.from_rad) * share;
                let toward = [bearing.cos(), bearing.sin()];
                // From the settlement's edge out to the approach's depth.
                let (mut out, mut open) = (0.0, 0.0);
                while open < approach.depth_m - 30.0 {
                    out += 5.0;
                    let point = [
                        settlement.center[0] + toward[0] * out,
                        settlement.center[1] + toward[1] * out,
                    ];
                    if contract::ground::polygon_contains(&settlement.outline, point) {
                        open = 0.0;
                        continue;
                    }
                    open += 5.0;
                    assert!(
                        !near.contains(&square(point)) || river.inside(point) < 0.0,
                        "{name}: water {open} m out along an approach to {}",
                        settlement.id
                    );
                }
            }
        }
    });
}

/// Where `p` lies on a deck: metres along its heading and across it, from
/// its centre.
fn on_deck(bridge: &contract::map::Bridge, p: Point) -> Point {
    let (sin, cos) = bridge.yaw.sin_cos();
    let d = [p[0] - bridge.center[0], p[1] - bridge.center[1]];
    [d[0] * cos + d[1] * sin, d[1] * cos - d[0] * sin]
}

fn carriageways(plan: &MapPlan) -> impl Iterator<Item = (&[Point], f64)> {
    plan.surfaces
        .iter()
        .filter(|area| area.kind.is_road())
        .filter_map(|area| match &area.shape {
            GroundShape::Stroke {
                centerline,
                width_m,
            } => Some((centerline.samples(), *width_m)),
            GroundShape::Polygon { .. } => None,
        })
}

/// A road meets water only on a deck, and a deck is a real crossing: it is
/// as wide as the road it carries, that road runs straight down its middle
/// from end to end, and the compiler's own bridge rule admits it.
#[test]
fn every_road_crosses_the_water_on_a_bridge_and_every_bridge_carries_a_road() {
    let mut bridges = 0;
    every_river_map(6, |name, plan| {
        let river = &plan.rivers[0];
        let near = near_water(river);
        // How far along each deck some road's centreline has been seen.
        let mut carried: Vec<[f64; 2]> =
            vec![[f64::INFINITY, f64::NEG_INFINITY]; plan.bridges.len()];
        for (samples, width) in carriageways(plan) {
            for pair in samples.windows(2) {
                let run = (pair[1][0] - pair[0][0]).hypot(pair[1][1] - pair[0][1]);
                let steps = (run / 0.5).ceil().max(1.0) as usize;
                for step in 0..=steps {
                    let t = step as f64 / steps as f64;
                    let p = [
                        pair[0][0] + (pair[1][0] - pair[0][0]) * t,
                        pair[0][1] + (pair[1][1] - pair[0][1]) * t,
                    ];
                    if !near.contains(&square(p)) {
                        continue;
                    }
                    let deck = plan.bridges.iter().position(|bridge| {
                        let [along, across] = on_deck(bridge, p);
                        along.abs() <= bridge.half_extents[0]
                            && across.abs() <= bridge.half_extents[1]
                    });
                    // The road's whole width is water wherever its middle is
                    // within half its width of the water.
                    let wet = river.inside(p) > -width / 2.0;
                    match deck {
                        Some(index) => {
                            let bridge = &plan.bridges[index];
                            let [along, across] = on_deck(bridge, p);
                            assert!(
                                across.abs() < 0.05,
                                "{name}: a road {across} m off a deck's middle"
                            );
                            assert!(
                                bridge.half_extents[1] >= width / 2.0,
                                "{name}: a deck narrower than its road"
                            );
                            carried[index] =
                                [carried[index][0].min(along), carried[index][1].max(along)];
                        }
                        None => assert!(!wet, "{name}: a road in the water at {p:?}, on no deck"),
                    }
                }
            }
        }
        for (bridge, seen) in plan.bridges.iter().zip(&carried) {
            let half = bridge.half_extents[0];
            assert!(
                seen[0] <= -half + 0.5 && seen[1] >= half - 0.5,
                "{name}: no road runs the length of the deck at {:?}",
                bridge.center
            );
            // It spans the water with land under both ends.
            assert!(
                river.inside(bridge.center) > 0.0,
                "{name}: a deck over no water"
            );
            for end in bridge.ends() {
                for corner in end {
                    assert!(
                        river.inside(corner) < 0.0,
                        "{name}: a deck's corner over water"
                    );
                    assert!(
                        (0..2).all(|axis| corner[axis] >= 0.0 && corner[axis] <= plan.size[axis]),
                        "{name}: a deck's corner off the map"
                    );
                }
            }
        }
        bridges += plan.bridges.len();
        mapgen::lower(
            &mapgen::CompileRequest::generated(
                &request(MapType::Open, MapSize::Medium, 1),
                plan.clone(),
            ),
            &empty_catalogue(),
        )
        .unwrap_or_else(|errors| panic!("{name}: {errors:?}"));
    });
    assert!(bridges >= 54, "{bridges} bridges on 54 river maps");
}

/// How many times the straight line from `a` to `b` crosses the river's
/// middle.
fn crossings(river: &River, a: Point, b: Point) -> usize {
    let side = |p: Point, q: Point, r: Point| contract::ground::cross(p, q, r);
    river
        .samples()
        .windows(2)
        .filter(|pair| {
            let (c, d) = (pair[0].xy, pair[1].xy);
            (side(a, b, c) > 0.0) != (side(a, b, d) > 0.0)
                && (side(c, d, a) > 0.0) != (side(c, d, b) > 0.0)
        })
        .count()
}

/// The measured rules hold with a river on the map, and they are measured
/// through the bridges: take the decks away and the same roads no longer
/// reach the far bank.
#[test]
fn a_river_map_is_connected_fair_and_quick_to_cross_through_its_bridges() {
    let presets = presets(1.0);
    let (mut cut_off, mut severed) = (0, 0);
    every_river_map(6, |name, plan| {
        let metrics = measure(plan, &presets);
        let river = &plan.rivers[0];
        let length: f64 = river
            .samples()
            .windows(2)
            .map(|pair| (pair[1].xy[0] - pair[0].xy[0]).hypot(pair[1].xy[1] - pair[0].xy[1]))
            .sum();
        let measured = metrics.river.top_km + metrics.river.bottom_km;
        assert!(
            (measured * 1000.0 - length).abs() < 1.0,
            "{name}: {measured} km"
        );
        assert_eq!(metrics.river.rivers, 1, "{name}");
        assert_eq!(
            metrics.river.bridges_top + metrics.river.bridges_bottom,
            plan.bridges.len(),
            "{name}"
        );
        assert_eq!(metrics.roads.unbridged, 0, "{name}");
        assert_eq!(metrics.roads.unconnected_settlements, 0, "{name}");
        // M12, as `layout.rs` states it for every plan.
        let playable = plan.size[0] * plan.size[1];
        for (split, rel, abs) in [(&metrics.town, 0.10, 0.004), (&metrics.forest, 0.20, 0.005)] {
            assert!(
                (split.top_m2 - split.bottom_m2).abs()
                    <= (rel * (split.top_m2 + split.bottom_m2)).max(abs * playable),
                "{name}: {split:?}"
            );
        }
        // M22: the top and bottom edges reach the centre in time and each
        // other by road, timed along the roads' rounded lines, corners at a
        // bridge included.
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
        let through = transit
            .top_bottom
            .as_ref()
            .unwrap_or_else(|| panic!("{name}: {transit:?}"));

        let mut cut = plan.clone();
        cut.bridges.clear();
        let without = measure(&cut, &presets);
        // Where that journey crossed the water, it did so by a deck: without
        // the decks it is longer or gone.
        match &without.transit.top_bottom {
            Some(detour) => assert!(detour.route_m >= through.route_m - 0.01, "{name}"),
            None => severed += 1,
        }
        assert!(
            without.roads.unbridged >= plan.bridges.len(),
            "{name}: {} runs of road in the water for {} decks removed",
            without.roads.unbridged,
            plan.bridges.len()
        );
        // Every settlement across the water from the map's centre loses its
        // road to it.
        let centre = [plan.size[0] / 2.0, plan.size[1] / 2.0];
        let across = plan
            .settlements
            .iter()
            .filter(|settlement| crossings(river, centre, settlement.center) % 2 == 1)
            .count();
        assert!(
            without.roads.unconnected_settlements >= across,
            "{name}: {} of {across} settlements across the water are cut off",
            without.roads.unconnected_settlements
        );
        cut_off += across;
    });
    assert!(cut_off >= 54, "{cut_off} settlements stand across a river");
    assert!(
        severed >= 1,
        "no map's road from bottom to top crosses its river"
    );
}

const TEMPLATES: &str = include_str!("../../../fixtures/prototype-building-templates.json");

/// The plan with its towns built, and the map it compiles into.
fn built(
    presets: &PresetDefinitions,
    map_type: MapType,
    size: MapSize,
    seed: u64,
) -> Result<(MapPlan, contract::map::MapDefinition), Vec<Diagnostic>> {
    let catalogue =
        contract::templates::TemplateGeometryCatalog::new(serde_json::from_str(TEMPLATES).unwrap())
            .unwrap();
    let mut request = request(map_type, size, seed);
    request.template_catalog_hash = catalogue.hash().into();
    request.limits = CompileLimits {
        max_authored_parts: 100_000,
        max_bay_positions: 1_000_000,
        max_ground_points: 400_000,
    };
    let layout = generate_layout(&request, presets)?;
    let plan = mapgen::parcels::fill_districts(layout, &request, &catalogue, presets)?;
    let map = mapgen::lower(
        &mapgen::CompileRequest::generated(&request, plan.clone()),
        &catalogue,
    )?
    .map;
    Ok((plan, map))
}

/// A town beside a river is built dry: no parcel, building or yard reaches
/// the water or its bank, and every street stays on its own bank, joined to
/// the roads there. (Metro Medium and Large seed 11 are towns whose nearest
/// road lies across the water.)
#[test]
fn a_town_beside_a_river_is_built_dry_and_its_streets_keep_to_their_bank() {
    let presets = presets(1.0);
    let cells = TYPES
        .into_iter()
        .flat_map(|map_type| {
            [
                (map_type, MapSize::Medium, 1),
                (map_type, MapSize::Medium, 2),
            ]
        })
        .chain([
            (MapType::Metro, MapSize::Large, 11),
            (MapType::Metro, MapSize::Xl, 11),
        ]);
    let mut beside = 0;
    for (map_type, size, seed) in cells {
        let name = format!("{map_type:?} {size:?} seed {seed}");
        let (plan, map) = built(&presets, map_type, size, seed)
            .unwrap_or_else(|errors| panic!("{name}: {errors:?}"));
        let river = &map.rivers[0];
        let (near, bank) = (near_water(river), bank_m(river));
        let dry = |p: Point| !near.contains(&square(p)) || river.inside(p) < -bank;
        for lot in &plan.lots {
            for p in along_ring(&lot.ring, 2.0) {
                assert!(dry(p), "{name}: parcel {} reaches the bank", lot.id);
                beside += usize::from(near.contains(&square(p)));
            }
        }
        for building in &map.buildings {
            for part in &building.geometry.parts {
                let (sin, cos) = part.yaw.sin_cos();
                let corners: Vec<Point> = [[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]]
                    .iter()
                    .map(|[u, v]| {
                        let (x, y) = (u * part.half_extents[0], v * part.half_extents[1]);
                        [
                            part.center[0] + cos * x - sin * y,
                            part.center[1] + sin * x + cos * y,
                        ]
                    })
                    .collect();
                for p in along_ring(&corners, 2.0) {
                    assert!(dry(p), "{name}: a building stands on the bank at {p:?}");
                }
            }
        }
        // Streets and yards are surfaces of kind `road`; the layout's own
        // roads cross by their decks, a street by none.
        for area in &map.surfaces {
            if area.kind != contract::map::SurfaceKind::Road {
                continue;
            }
            let points = match &area.shape {
                GroundShape::Polygon { ring } => along_ring(ring, 2.0),
                GroundShape::Stroke { centerline, .. } => centerline
                    .samples()
                    .windows(2)
                    .flat_map(|pair| along_ring(&[pair[0], pair[1]], 1.0))
                    .collect(),
            };
            for p in points {
                assert!(dry(p), "{name}: a street or yard reaches the bank at {p:?}");
            }
        }
        let metrics = measure(&plan, &presets);
        assert_eq!(metrics.roads.unbridged, 0, "{name}");
        assert_eq!(metrics.roads.unconnected_street_km, 0.0, "{name}");
        assert_eq!(metrics.roads.unconnected_settlements, 0, "{name}");
    }
    assert!(beside > 0, "no parcel stands within a square of a river");
}

/// A river is drawn from its own stream. A seed without one is the map it
/// would be were rivers switched off, byte for byte, and a fixed request
/// gives the same river and the same bridges every run.
#[test]
fn a_seed_without_a_river_is_the_map_it_was_and_a_river_map_is_the_same_every_run() {
    let (off, half) = (presets(0.0), presets(0.5));
    let (mut dry, mut wet) = (0, 0);
    for map_type in TYPES {
        for size in SIZES {
            for seed in [1, 2, 3, 4, 5, 6, 7, 8, u64::MAX] {
                let name = format!("{map_type:?} {size:?} seed {seed}");
                let plan = generate(&half, map_type, size, seed).unwrap();
                if plan.rivers.is_empty() {
                    let without = generate(&off, map_type, size, seed).unwrap();
                    assert_eq!(json(&plan), json(&without), "{name}");
                    assert!(plan.bridges.is_empty(), "{name}");
                    dry += 1;
                } else {
                    let again = generate(&half, map_type, size, seed).unwrap();
                    assert_eq!(json(&plan), json(&again), "{name}");
                    wet += 1;
                }
            }
        }
    }
    assert!(
        dry >= 20 && wet >= 20,
        "{dry} maps without a river, {wet} with"
    );
}

/// A river the map cannot hold ends in a named refusal, quickly, never in a
/// map quietly without one.
#[test]
fn a_river_the_map_cannot_hold_is_refused_by_name() {
    // No course keeps this far from both side edges of a 6 km map.
    let hemmed = presets_with(|source| {
        source["types"]["open"]["river_chance"] = 1.into();
        source["rivers"]["side_margin_m"] = 2_990.into();
    });
    let errors = generate(&hemmed, MapType::Open, MapSize::Medium, 5).unwrap_err();
    assert_eq!(errors[0].code, mapgen::DiagnosticCode::GenerationFailed);
    assert_eq!(errors[0].feature.as_deref(), Some("river"));
    assert_eq!(errors[0].location, "$.presets.types.open.sizes.medium");
    assert!(
        errors[0].message.contains("seed 5"),
        "{}",
        errors[0].message
    );
}

#[test]
fn river_presets_the_terrain_or_a_road_cannot_carry_are_refused_at_load() {
    for (pointer, value) in [
        // Narrower than three height samples; a bank past the slope cutoff.
        ("/rivers/width_m", serde_json::json!([8, 30])),
        ("/rivers/bank_grade", serde_json::json!(0.6)),
        ("/rivers/freeboard_m", serde_json::json!(0)),
        // A meander tighter than the water and a road beside it are wide.
        ("/rivers/meander/amplitude", serde_json::json!([0.2, 0.9])),
        (
            "/rivers/meander/overtone_ratio",
            serde_json::json!([0.5, 2]),
        ),
        ("/rivers/point_turn_deg", serde_json::json!(0)),
        // A wood, a settlement or a road on the bank itself.
        ("/rivers/forest_gap_m", serde_json::json!(2)),
        ("/rivers/settlement_gap_m", serde_json::json!(0)),
        ("/rivers/junction_gap_m", serde_json::json!(5)),
        // A deck narrower than its road, one that ends on the ramp as the
        // height grid draws it (the steepest bank alone would end 2.7 m
        // out), one too short for the widest water, a road that bends on the
        // deck, and a road kind with no reuse distance.
        ("/rivers/bridge/width_m", serde_json::json!(6)),
        ("/rivers/bridge/landing_m", serde_json::json!(6)),
        ("/rivers/bridge/span_max_m", serde_json::json!(30)),
        ("/rivers/bridge/approach_m", serde_json::json!(4)),
        ("/rivers/road_gap_m", serde_json::json!(15)),
        (
            "/rivers/bridge/reuse_m",
            serde_json::json!({ "country_road": 300 }),
        ),
        ("/rivers/bridge/deck", serde_json::json!("")),
        ("/types/open/river_chance", serde_json::json!(1.5)),
        ("/types/open/siting/beside_river", serde_json::json!(0.9)),
        ("/fairness/river/rel", serde_json::json!(2)),
        ("/retries/river", serde_json::json!(0)),
    ] {
        let mut source: serde_json::Value = serde_json::from_str(PRESETS).unwrap();
        *source.pointer_mut(pointer).unwrap() = value;
        assert!(
            PresetDefinitions::from_json(&source.to_string()).is_err(),
            "{pointer} was admitted"
        );
    }
}

/// A road that follows the bank is authored as any road is: by the points
/// its line needs, not by one for every point of the river beside it. No
/// point of a road beside the water could be dropped and leave the road
/// within a couple of metres of where it ran.
#[test]
fn a_road_along_the_bank_is_authored_no_more_densely_than_its_line_needs() {
    let (mut beside, mut idle) = (0, 0);
    every_river_map(8, |_, plan| {
        let river = &plan.rivers[0];
        let near = near_water(river);
        for area in plan.surfaces.iter().filter(|area| area.kind.is_road()) {
            let GroundShape::Stroke { centerline, .. } = &area.shape else {
                continue;
            };
            for run in centerline.control_points().windows(3) {
                // A bridge's straight run of road is authored point for
                // point; this is about the road beside the water.
                let bridged = plan.bridges.iter().any(|bridge| {
                    let [along, across] = on_deck(bridge, run[1]);
                    along.abs() < bridge.half_extents[0] + 40.0 && across.abs() < 40.0
                });
                if bridged || !run.iter().all(|p| near.contains(&square(*p))) {
                    continue;
                }
                beside += 1;
                // The middle point, against the straight line its
                // neighbours would make without it.
                let off = contract::ground::segment_distance(run[0], run[2], run[1]);
                idle += usize::from(off < 0.5);
            }
        }
    });
    assert!(beside >= 60, "{beside} road points beside a river");
    assert!(
        idle * 10 < beside,
        "{idle} of {beside} road points beside a river sit on the line their neighbours make"
    );
}
