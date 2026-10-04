//! M23: a town is not a dartboard. These hold a generated settlement to the
//! shape of a place: blocks bounded by its roads and streets, an outline that
//! is those blocks' own, and nothing cut into slices about one point.
use contract::ground::GroundShape;
use mapgen::layout::{
    generate_layout, GenerationRequest, MapSize, MapType, PresetDefinitions, GENERATOR_VERSION,
};
use mapgen::{CompileLimits, MapPlan, SettlementPlan};

const PRESETS: &str = include_str!("../../../fixtures/map-presets.json");
const SEEDS: [u64; 4] = [1, 2, 3, u64::MAX];
type Point = [f64; 2];
/// Two blocks' corners either side of a street are no farther apart than
/// this: the gap an edge may leave and still carry a ray on.
const CORNER_GAP_M: f64 = 40.0;

fn presets() -> PresetDefinitions {
    PresetDefinitions::from_json(PRESETS).unwrap()
}

fn plan(map_type: MapType, size: MapSize, seed: u64) -> MapPlan {
    let presets = presets();
    let catalogue = contract::templates::TemplateGeometryCatalog::new(Vec::new()).unwrap();
    let request = GenerationRequest {
        generator_version: GENERATOR_VERSION.into(),
        preset_revision: presets.revision.clone(),
        seed: seed.into(),
        template_catalog_hash: catalogue.hash().into(),
        map_type,
        size,
        region: None,
        limits: CompileLimits {
            max_authored_parts: 0,
            max_bay_positions: 0,
            max_ground_points: 200_000,
        },
    };
    generate_layout(&request, &presets)
        .unwrap_or_else(|errors| panic!("{map_type:?} {size:?} seed {seed}: {errors:?}"))
}

fn every_settlement(mut check: impl FnMut(&str, &MapPlan, &SettlementPlan)) {
    for map_type in MapType::ALL {
        for size in MapSize::ALL {
            for seed in SEEDS {
                let plan = plan(map_type, size, seed);
                for settlement in &plan.settlements {
                    let name = format!("{map_type:?} {size:?} seed {seed} {}", settlement.id);
                    check(&name, &plan, settlement);
                }
            }
        }
    }
}

fn edges(ring: &[Point]) -> impl Iterator<Item = (Point, Point)> + '_ {
    (0..ring.len()).map(|i| (ring[i], ring[(i + 1) % ring.len()]))
}

fn length(a: Point, b: Point) -> f64 {
    libm::hypot(b[0] - a[0], b[1] - a[1])
}

/// Distance from `p` to the infinite line through `a` and `b`.
fn line_distance(a: Point, b: Point, p: Point) -> f64 {
    ((b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])).abs() / length(a, b)
}

fn segment_distance(a: Point, b: Point, p: Point) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let t = (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
    length([a[0] + dx * t, a[1] + dy * t], p)
}

fn ring_area(ring: &[Point]) -> f64 {
    (contract::ground::polygon_area(ring) / 2.0).abs()
}

/// The woods standing in or against a settlement: those with a corner
/// inside its outline.
fn woods<'a>(
    plan: &'a MapPlan,
    settlement: &'a SettlementPlan,
) -> impl Iterator<Item = &'a [Point]> {
    plan.forests
        .iter()
        .filter_map(|forest| match &forest.shape {
            GroundShape::Polygon { ring }
                if ring
                    .iter()
                    .any(|p| contract::ground::polygon_contains(&settlement.outline, *p)) =>
            {
                Some(ring.as_slice())
            }
            _ => None,
        })
}

/// A dartboard is cut by rays from one point: its slices' long edges all
/// point at that point, each along its own bearing. Main streets meet in a
/// crossroads at most, and a street or two may end where a road passes. So
/// from a settlement's centre the edges of its districts and woods run out
/// along four bearings at most, and from any other point along five.
#[test]
fn no_point_of_a_settlement_has_district_edges_fanning_out_from_it() {
    every_settlement(|name, plan, settlement| {
        let rings: Vec<&[Point]> = settlement
            .districts
            .iter()
            .map(|district| district.ring.as_slice())
            .chain(woods(plan, settlement))
            .collect();
        // Edges long enough to bound a district, not a corner's chamfer.
        let long: Vec<(Point, Point)> = rings
            .iter()
            .flat_map(|ring| edges(ring))
            .filter(|(a, b)| length(*a, *b) >= 40.0)
            .collect();
        let mut apexes: Vec<Point> = vec![settlement.center];
        apexes.extend(rings.iter().flat_map(|ring| ring.iter().copied()));
        for apex in apexes {
            // The bearings, in degrees, along which an edge on a line
            // through the apex runs away from it: toward each of its ends
            // that lies farther than a couple of metres. A ray is edges end
            // to end: an edge is on one when it starts at the apex, or
            // where an edge already on that ray stops, give or take the
            // gap a street leaves between two blocks' corners. A far edge
            // that happens to line up with the apex is not on one.
            let on_line: Vec<(Point, Point)> = long
                .iter()
                .copied()
                .filter(|(a, b)| line_distance(*a, *b, apex) <= 2.0)
                .collect();
            let mut reached: Vec<Point> = vec![apex];
            let mut on_ray = vec![false; on_line.len()];
            loop {
                let next = (0..on_line.len()).find(|at| {
                    let (a, b) = on_line[*at];
                    !on_ray[*at]
                        && reached
                            .iter()
                            .any(|from| segment_distance(a, b, *from) <= CORNER_GAP_M)
                });
                let Some(at) = next else { break };
                on_ray[at] = true;
                reached.extend([on_line[at].0, on_line[at].1]);
            }
            let mut bearings: Vec<f64> = on_line
                .iter()
                .zip(&on_ray)
                .filter(|(_, on_ray)| **on_ray)
                .map(|(edge, _)| edge)
                .flat_map(|(a, b)| {
                    // An apex between the ends sees the edge run both ways.
                    let between = length(*a, apex) + length(*b, apex) <= length(*a, *b) + 0.1;
                    let far = if length(*a, apex) >= length(*b, apex) {
                        *a
                    } else {
                        *b
                    };
                    let near = if far == *a { *b } else { *a };
                    [
                        Some(far),
                        (between && length(near, apex) > 2.0).then_some(near),
                    ]
                })
                .flatten()
                .map(|end| {
                    libm::atan2(end[1] - apex[1], end[0] - apex[0])
                        .to_degrees()
                        .rem_euclid(360.0)
                })
                .collect();
            bearings.sort_by(f64::total_cmp);
            // Bearings more than eight degrees apart are different rays.
            let mut rays = 0;
            let mut last = f64::NEG_INFINITY;
            for bearing in &bearings {
                if bearing - last > 8.0 {
                    rays += 1;
                }
                last = *bearing;
            }
            if rays > 1 && bearings[0] + 360.0 - last <= 8.0 {
                rays -= 1;
            }
            let most = if apex == settlement.center { 4 } else { 5 };
            assert!(
                rays <= most,
                "{name}: district edges fan out along {rays} bearings from ({:.0}, {:.0})",
                apex[0],
                apex[1]
            );
        }
    });
}

/// A settlement's outline is its districts' own edge: every stretch of it
/// runs along the edge of one of its districts, or across the verge between
/// two of their streets, never a street's width from one. No envelope is
/// drawn round the town for fields to be sliced out of.
#[test]
fn a_settlements_outline_runs_along_the_edges_of_its_districts() {
    every_settlement(|name, _, settlement| {
        contract::ground::validate_ring(&settlement.outline)
            .unwrap_or_else(|error| panic!("{name}: {error}: {:?}", settlement.outline));
        for (a, b) in edges(&settlement.outline) {
            for share in [0.25, 0.5, 0.75] {
                let p = [a[0] + (b[0] - a[0]) * share, a[1] + (b[1] - a[1]) * share];
                let gap = settlement
                    .districts
                    .iter()
                    .flat_map(|district| edges(&district.ring))
                    .map(|(c, d)| segment_distance(c, d, p))
                    .fold(f64::INFINITY, f64::min);
                assert!(
                    gap <= 10.0,
                    "{name}: the outline at ({:.0}, {:.0}) is {gap:.1} m from any district",
                    p[0],
                    p[1]
                );
            }
        }
        // And it holds every district.
        for district in &settlement.districts {
            for p in &district.ring {
                let inside = contract::ground::polygon_contains(&settlement.outline, *p)
                    || edges(&settlement.outline).any(|(a, b)| segment_distance(a, b, *p) <= 0.05);
                assert!(inside, "{name}: {} leaves the outline", district.id);
            }
        }
    });
}

/// How round a ring is: its area over that of the circle with its
/// perimeter. A circle is 1, a square 0.79, a two-to-one rectangle 0.70.
fn roundness(ring: &[Point]) -> f64 {
    let perimeter: f64 = edges(ring).map(|(a, b)| length(a, b)).sum();
    4.0 * core::f64::consts::PI * ring_area(ring) / (perimeter * perimeter)
}

/// No settlement is a round blob: a hamlet is lots along a lane, a town a
/// ragged group of blocks. None is rounder than a regular hexagon, and
/// taken together they are less round than a square.
#[test]
fn no_settlement_is_a_round_blob() {
    let mut all = Vec::new();
    every_settlement(|name, _, settlement| {
        let round = roundness(&settlement.outline);
        assert!(round < 0.9, "{name}: roundness {round:.2}");
        all.push(round);
    });
    let mean = all.iter().sum::<f64>() / all.len() as f64;
    assert!(mean < 0.75, "mean roundness {mean:.2}");
}

/// A road is a block's edge, never a line through one: a district lies to
/// one side of every country road and track on its settlement's ground.
#[test]
fn no_road_runs_through_the_inside_of_a_district() {
    every_settlement(|name, plan, settlement| {
        let roads = plan
            .surfaces
            .iter()
            .filter(|area| area.kind != contract::map::SurfaceKind::Road)
            .filter_map(|area| match &area.shape {
                GroundShape::Stroke { centerline, .. } => Some(centerline.control_points()),
                GroundShape::Polygon { .. } => None,
            });
        // Walked ten metres at a time, where a road is near at all.
        let [x0, y0, x1, y1] = contract::ground::limits(&settlement.outline, 0.0);
        for run in roads.flat_map(|road| road.windows(2)) {
            let outside = |axis: usize, low: f64, high: f64| {
                run[0][axis].max(run[1][axis]) < low || run[0][axis].min(run[1][axis]) > high
            };
            if outside(0, x0, x1) || outside(1, y0, y1) {
                continue;
            }
            let steps = (length(run[0], run[1]) / 10.0).ceil().max(1.0) as usize;
            for step in 0..=steps {
                let share = step as f64 / steps as f64;
                let p = [
                    run[0][0] + (run[1][0] - run[0][0]) * share,
                    run[0][1] + (run[1][1] - run[0][1]) * share,
                ];
                for district in &settlement.districts {
                    let deep = contract::ground::polygon_contains(&district.ring, p)
                        && edges(&district.ring).all(|(a, b)| segment_distance(a, b, p) > 6.0);
                    assert!(
                        !deep,
                        "{name}: a road runs through {} at {p:?}",
                        district.id
                    );
                }
            }
        }
    });
}

/// The country roads on a settlement's own ground: each straight run of one
/// that has a district of the settlement within a block of its middle.
fn roads_through(plan: &MapPlan, settlement: &SettlementPlan) -> Vec<(Point, Point)> {
    plan.surfaces
        .iter()
        .filter(|area| area.kind == contract::map::SurfaceKind::CountryRoad)
        .filter_map(|area| match &area.shape {
            GroundShape::Stroke { centerline, .. } => Some(centerline.control_points()),
            GroundShape::Polygon { .. } => None,
        })
        .flat_map(|road| road.windows(2))
        .map(|run| (run[0], run[1]))
        .filter(|(a, b)| {
            let middle = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
            length(*a, *b) >= 100.0
                && settlement.districts.iter().any(|district| {
                    edges(&district.ring).any(|(c, d)| segment_distance(c, d, middle) <= 30.0)
                })
        })
        .collect()
}

/// Where the country roads on a settlement's ground leave one another away
/// from its centre: each point where one of them ends part of the way
/// along another, at an angle to it.
fn side_roads(plan: &MapPlan, settlement: &SettlementPlan) -> Vec<Point> {
    let through = roads_through(plan, settlement);
    let mut starts: Vec<Point> = Vec::new();
    for (a, b) in &through {
        for end in [*a, *b] {
            let joins = through.iter().any(|(c, d)| {
                let span = length(*c, *d);
                let share = ((end[0] - c[0]) * (d[0] - c[0]) + (end[1] - c[1]) * (d[1] - c[1]))
                    / (span * span);
                let cos = ((b[0] - a[0]) * (d[0] - c[0]) + (b[1] - a[1]) * (d[1] - c[1])).abs()
                    / (length(*a, *b) * span);
                segment_distance(*c, *d, end) <= 1.0 && (0.0..=1.0).contains(&share) && cos < 0.9
            });
            if joins
                && length(end, settlement.center) >= 150.0
                && !starts.iter().any(|known| length(*known, end) <= 1.0)
            {
                starts.push(end);
            }
        }
    }
    starts
}

/// A city is not one crossroads with a disc round it: roads lead out of it
/// that leave its main roads away from the central junction. Every city
/// has one, and the typical city two or more.
#[test]
fn a_city_has_roads_out_that_miss_its_central_junction() {
    let (mut total, mut cities) = (0, 0);
    for size in MapSize::ALL {
        for seed in SEEDS {
            let plan = plan(MapType::Metro, size, seed);
            let city = &plan.settlements[0];
            assert_eq!(city.class, "city");
            let sides = side_roads(&plan, city).len();
            assert!(
                sides >= 1,
                "Metro {size:?} seed {seed}: every road out of the city passes its centre"
            );
            total += sides;
            cities += 1;
        }
    }
    assert!(total >= 2 * cities, "{total} side roads in {cities} cities");
}

/// A large town is never a slab along one road: a second road crosses its
/// ground at an angle to the first.
#[test]
fn a_large_town_has_a_second_road_at_an_angle() {
    for size in MapSize::ALL {
        for seed in SEEDS {
            let plan = plan(MapType::Mixed, size, seed);
            let town = &plan.settlements[0];
            assert_eq!(town.class, "large_town");
            let through = roads_through(&plan, town);
            let bearing = |(a, b): &(Point, Point)| libm::atan2(b[1] - a[1], b[0] - a[0]);
            // The widest angle between two of its roads, as lines.
            let mut widest: f64 = 0.0;
            for first in &through {
                for second in &through {
                    let between =
                        (bearing(first) - bearing(second)).rem_euclid(core::f64::consts::PI);
                    widest = widest.max(between.min(core::f64::consts::PI - between));
                }
            }
            assert!(
                widest.to_degrees() >= 40.0,
                "Mixed {size:?} seed {seed}: its roads all run one way, within {:.0}°",
                widest.to_degrees()
            );
        }
    }
}
