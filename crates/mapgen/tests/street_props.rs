//! Street furniture (C46), judged by where the bodies stand and by what the
//! simulation's own navigation makes of them. Geometry here is checked with
//! its own arithmetic, not the generator's.
use contract::catalog::Catalog;
use contract::encounter::EncounterRecipes;
use contract::ground::{polygon_contains, GroundShape};
use contract::map::{AuthoredPropDefinition, MapDefinition, PropDefinition, SurfaceKind};
use contract::scenario::Rules;
use contract::templates::TemplateGeometryCatalog;
use mapgen::layout::{
    approach_corridors, generate_layout, GenerationRequest, MapSize, MapType, PresetDefinitions,
};
use mapgen::parcels::fill_districts;
use mapgen::street_props::place_street_props;
use mapgen::{CompileLimits, MapPlan};
use serde_json::{json, Value};
use sim::encounter::legality::stands;
use sim::encounter::PreparedMap;
use sim::math::{v2, V2};
use sim::navigation::{self, Leg, Mobility, Plan};
use std::collections::BTreeMap;

const PRESETS: &str = include_str!("../../../fixtures/map-presets.json");
const TEMPLATES: &str = include_str!("../../../fixtures/prototype-building-templates.json");
const RECIPES: &str = include_str!("../../../fixtures/encounters.json");
/// The sweep runs every type and size on these seeds.
const SWEEP_SEEDS: [u64; 2] = [1, 2];
/// How many doors of a map a squad walks to from its settlement's centre,
/// and how many of its streets each hull drives: spread evenly through the
/// map's own order.
const DOORS_WALKED: usize = 40;
const STREETS_DRIVEN: usize = 40;
/// A drive down a dressed street is at most this much longer than down the
/// bare one.
const DETOUR_M: f64 = 30.0;
/// A house of one part, 10 m along the street and 8 m deep, its one door in
/// the middle of its street wall.
const HOUSE: &str = "china-home-10x8-1f";

type Point = [f64; 2];

/// The shipped presets with `edit` applied: a test states the rows it is
/// about, so retuning the shipped densities moves no claim here.
fn presets_with(edit: impl FnOnce(&mut Value)) -> PresetDefinitions {
    let mut source: Value = serde_json::from_str(PRESETS).unwrap();
    edit(&mut source);
    PresetDefinitions::from_json(&source.to_string()).unwrap()
}

fn presets() -> PresetDefinitions {
    presets_with(|_| ())
}

fn catalogue() -> TemplateGeometryCatalog {
    TemplateGeometryCatalog::new(serde_json::from_str(TEMPLATES).unwrap()).unwrap()
}

fn rules() -> Rules {
    serde_json::from_value(sim::fixtures::game()).unwrap()
}

fn request(map_type: MapType, size: MapSize, seed: u64) -> GenerationRequest {
    GenerationRequest {
        generator_version: mapgen::layout::GENERATOR_VERSION.into(),
        preset_revision: presets().revision,
        seed: seed.into(),
        template_catalog_hash: catalogue().hash().into(),
        map_type,
        size,
        region: None,
        limits: CompileLimits {
            max_authored_parts: 60_000,
            max_bay_positions: 600_000,
            max_ground_points: 200_000,
        },
    }
}

/// The widest hull of the catalog: what the lane is sized to.
fn widest_hull(catalog: &Catalog) -> f64 {
    catalog
        .indices()
        .filter_map(|unit| catalog.get(unit).hull())
        .map(|hull| 2.0 * hull.half_extents_m[1])
        .fold(0.0, f64::max)
}

/// The nearest a body may stand to a carriageway's middle, as the presets
/// state it.
fn lane(presets: &PresetDefinitions, catalog: &Catalog) -> f64 {
    widest_hull(catalog) + presets.street_props.lane_margin_m
}

/// What the hand-drawn streets below are dressed by, whatever the shipped
/// presets are tuned to.
const LANE_MARGIN_M: f64 = 3.7;
const DOOR_CLEAR_M: f64 = 3.0;
const CORNER_CLEAR_M: f64 = 8.0;

/// The centre district kind dressed with cars only, along most of the kerb,
/// by the rules these tests state.
fn cars_only(source: &mut Value) {
    source["districts"]["centre"]["props"] = json!({ "parking": 0.9 });
    let rule = &mut source["street_props"];
    rule["lane_margin_m"] = LANE_MARGIN_M.into();
    rule["door_clear_m"] = DOOR_CLEAR_M.into();
    rule["corner_clear_m"] = CORNER_CLEAR_M.into();
    rule["parking"]["both_sides_min_width_m"] = 10.into();
}

/// The streets of a hand-drawn town: one settlement of one centre district,
/// 800 m by 400 m about (1000, 1000), with the carriageways `streets` (each
/// a width and two ends) and a house at each of `doors` (where its door
/// is, and the way the door faces as a yaw: 0 faces south).
fn town(streets: &[(f64, Point, Point)], doors: &[(Point, f64)]) -> MapPlan {
    let ring = json!([[600, 800], [1400, 800], [1400, 1200], [600, 1200]]);
    let buildings: Vec<Value> = doors
        .iter()
        .enumerate()
        .map(|(index, (door, yaw))| {
            // The template's door is in its south wall, 4 m from its middle:
            // the frame stands that far behind the door.
            let back = [yaw.sin() * -4.0, yaw.cos() * 4.0];
            json!({
                "id": format!("settlement-0/district-0/lot-{index}"),
                "template_id": HOUSE,
                "kind": "building",
                "owner": index,
                "parts": [{ "part": "body", "prop": index }],
                "frame": { "translation": [door[0] + back[0], door[1] + back[1], 0.0], "yaw": yaw },
            })
        })
        .collect();
    let surfaces: Vec<Value> = streets
        .iter()
        .map(|(width, from, to)| {
            json!({ "kind": "road", "shape": { "kind": "stroke", "points": [from, to], "width_m": width } })
        })
        .collect();
    serde_json::from_value(json!({
        "size": [2000, 2000],
        "fog_cell_m": 8,
        "height_grid_m": 4,
        "slope_cutoff_deg": 35,
        "buildings": buildings,
        "surfaces": surfaces,
        "settlements": [{
            "id": "settlement-0",
            "class": "town",
            "center": [1000, 1000],
            "outline": ring,
            "districts": [{
                "id": "settlement-0/district-0",
                "kind": "centre",
                "ring": ring,
                "area_m2": 320000,
                "anchor": [1000, 1000],
                "categories": [{ "category": "attached_home", "weight": 1 }],
            }],
        }],
    }))
    .unwrap()
}

/// One straight street along y = 1000, from x = 700 to x = 1300.
fn street(width: f64) -> (f64, Point, Point) {
    (width, [700.0, 1000.0], [1300.0, 1000.0])
}

fn place(
    plan: &MapPlan,
    presets: &PresetDefinitions,
    catalog: &Catalog,
    seed: u64,
) -> Vec<PropDefinition> {
    place_street_props(
        plan,
        &request(MapType::Mixed, MapSize::Small, seed),
        &catalogue(),
        catalog,
        presets,
    )
    .unwrap()
    .into_iter()
    .map(|prop| prop.geometry)
    .collect()
}

fn cars(props: &[PropDefinition]) -> Vec<&PropDefinition> {
    props
        .iter()
        .filter(|prop| prop.kind == "parked_car")
        .collect()
}

/// The corners of a body's box on the ground.
fn corners(prop: &PropDefinition) -> [Point; 4] {
    let (sin, cos) = prop.yaw.sin_cos();
    [[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]].map(|[u, v]| {
        let (along, across) = (u * prop.half_extents[0], v * prop.half_extents[1]);
        [
            prop.center[0] + cos * along - sin * across,
            prop.center[1] + sin * along + cos * across,
        ]
    })
}

/// The compiled map of `plan` with `props` standing on it.
fn compiled(
    plan: &MapPlan,
    props: &[PropDefinition],
    request: &GenerationRequest,
) -> MapDefinition {
    let mut plan = plan.clone();
    plan.props = props
        .iter()
        .map(|geometry| AuthoredPropDefinition {
            id: None,
            geometry: geometry.clone(),
        })
        .collect();
    mapgen::lower(
        &mapgen::CompileRequest::generated(request, plan),
        &catalogue(),
    )
    .unwrap_or_else(|errors| panic!("{errors:?}"))
    .map
}

/// Every hull of the catalog as a mover, the widest first.
fn vehicles(rules: &Rules) -> Vec<(String, Mobility)> {
    let catalog = &rules.catalog;
    let mut hulls: Vec<(String, Mobility)> = catalog
        .indices()
        .filter(|unit| catalog.get(*unit).hull().is_some())
        .map(|unit| {
            (
                catalog.id(unit).to_string(),
                sim::units::mobility(catalog.get(unit), rules),
            )
        })
        .collect();
    hulls.sort_by(|a, b| b.1.half_width_m.total_cmp(&a.1.half_width_m));
    hulls
}

/// The simulation's own route for `m` from `from` to `to` on `map`, with
/// whether driving it shoves any body aside; `None` when it finds none.
fn route(
    map: &PreparedMap,
    rules: &Rules,
    m: &Mobility,
    from: Point,
    to: Point,
) -> Option<(Vec<V2>, bool)> {
    let start = v2(from[0], from[1]);
    let leg = Leg {
        from: start,
        goal: v2(to[0], to[1]),
        m,
        policy: contract::command::RoutePolicy::Fastest,
        avoid: &[],
    };
    match navigation::plan(&map.grid, &map.roads, leg, &rules.navigation).0 {
        Plan::Route(points) => {
            let pushes = map.grid.route_pushes(start, &points, m);
            Some((points, pushes))
        }
        Plan::Blocked(_) => None,
    }
}

/// A 7 m street with houses set back along both sides, cars beside most of
/// its kerb: cars stand before the houses, and none between a door and the
/// street.
#[test]
fn no_car_stands_in_front_of_a_door() {
    let presets = presets_with(cars_only);
    // Twenty houses a side, a door every 25 m, their walls 16 m from the
    // street's middle: room for a car between a wall and the lane, and for a
    // run of cars between one door's way and the next.
    let doors: Vec<(Point, f64)> = (0..20)
        .flat_map(|k| {
            let x = 760.0 + 25.0 * k as f64;
            [([x, 1016.0], 0.0), ([x, 984.0], std::f64::consts::PI)]
        })
        .collect();
    let plan = town(&[street(7.0)], &doors);
    let catalog = rules().catalog;
    let props = place(&plan, &presets, &catalog, 1);
    let cars = cars(&props);
    let fronting = cars
        .iter()
        .filter(|car| car.center[0] > 760.0 && car.center[0] < 1235.0)
        .count();
    assert!(
        fronting >= 30,
        "only {fronting} cars before 475 m of houses"
    );
    for (door, _) in &doors {
        for car in &cars {
            // Cars on the door's own side of the street.
            if (car.center[1] - 1000.0) * (door[1] - 1000.0) < 0.0 {
                continue;
            }
            let gap = (car.center[0] - door[0]).abs() - car.half_extents[0];
            assert!(
                gap >= DOOR_CLEAR_M - 0.011,
                "a car at {:?} stands {gap:.2} m from the line of the door at x = {}",
                car.center,
                door[0]
            );
        }
    }
}

/// Two streets that cross, with cars and a lamp every 10 m beside both: no
/// body stands within the corner clearance of the other street's kerb, so
/// the junction's four corners are open, and bodies stand along both streets.
#[test]
fn nothing_stands_at_a_junctions_corner() {
    let presets = presets_with(|source| {
        cars_only(source);
        source["districts"]["centre"]["props"]["verge"] =
            json!([{ "kind": "lamp", "spacing_m": 10, "sides": "both" }]);
    });
    let cross = (7.0, [1000.0, 850.0], [1000.0, 1150.0]);
    let plan = town(&[street(7.0), cross], &[]);
    let props = place(&plan, &presets, &rules().catalog, 1);
    let (mut along_x, mut along_y) = (0, 0);
    for prop in &props {
        // The street it stands beside is the nearer; the other is the one
        // whose corner it keeps back from.
        let off = [0, 1].map(|axis| (prop.center[axis] - 1000.0).abs());
        *if off[1] < off[0] {
            &mut along_x
        } else {
            &mut along_y
        } += 1;
        let far = off[0].max(off[1]) - prop.half_extents[0].max(prop.half_extents[1]);
        assert!(
            far >= 3.5 + CORNER_CLEAR_M - 0.011,
            "a {} at {:?} stands {far:.2} m from the crossing street's middle",
            prop.kind,
            prop.center
        );
    }
    assert!(cars(&props).len() >= 15, "{} cars", cars(&props).len());
    assert!(
        along_x >= 60 && along_y >= 25,
        "{along_x} and {along_y} bodies"
    );
}

/// The lane a vehicle drives stays clear on a 7 m street and on a 10 m
/// avenue: no car stands nearer the middle than the catalog's widest hull
/// and the preset margin, nor on the carriageway. The street parks along one
/// side and the avenue along both. A wider hull in the catalog moves the
/// cars out by as much: the lane is read from the catalog.
#[test]
fn the_driven_lane_stays_clear_and_a_narrow_street_parks_one_side() {
    let presets = presets_with(cars_only);
    let catalog = rules().catalog;
    let mut fixture = sim::fixtures::game();
    let wide = 2.6;
    let wider = {
        let widest = catalog
            .indices()
            .max_by(|a, b| {
                let width = |unit| {
                    catalog
                        .get(unit)
                        .hull()
                        .map_or(0.0, |h| h.half_extents_m[1])
                };
                width(*a).total_cmp(&width(*b))
            })
            .unwrap();
        sim::fixtures::patch_catalog(
            &mut fixture,
            "units",
            catalog.id(widest),
            json!({ "body": { "hull": { "half_extents_m": [3.5, wide, 1.2] } } }),
        );
        serde_json::from_value::<Rules>(fixture).unwrap().catalog
    };
    assert!(widest_hull(&wider) > widest_hull(&catalog) + 1.0);
    for (width, sides) in [(7.0, 1), (10.0, 2)] {
        let plan = town(&[street(width)], &[]);
        // The nearest any car comes to the street's middle, and how many
        // sides of it cars stand along.
        let nearest = |catalog: &Catalog| {
            let props = place(&plan, &presets, catalog, 1);
            let cars = cars(&props);
            assert!(
                cars.len() >= 20,
                "{} cars on a {width} m street",
                cars.len()
            );
            let north = cars.iter().filter(|car| car.center[1] > 1000.0).count();
            let parked = usize::from(north > 0) + usize::from(north < cars.len());
            let near = cars
                .iter()
                .flat_map(|car| corners(car))
                .map(|corner| (corner[1] - 1000.0).abs())
                .fold(f64::INFINITY, f64::min);
            assert!(
                near >= widest_hull(catalog) + LANE_MARGIN_M && near > width / 2.0,
                "on a {width} m street a car stands {near:.2} m from the middle"
            );
            (near, parked)
        };
        let ((narrow_hull, parked), (wide_hull, _)) = (nearest(&catalog), nearest(&wider));
        assert_eq!(
            parked, sides,
            "a {width} m street parks along {parked} sides"
        );
        let moved = wide_hull - narrow_hull;
        let grew = widest_hull(&wider) - widest_hull(&catalog);
        assert!(
            (moved - grew).abs() < 0.011,
            "a hull {grew:.2} m wider moved the cars of a {width} m street {moved:.2} m"
        );
    }
}

/// The simulation's own navigation on a dressed 7 m street and a 10 m
/// avenue: every hull of the catalog still has a route from one end to the
/// other, each way, and none shoves a body to drive it. The same cars moved
/// in to the kerb, on the carriageway, are shoved: that is what the lane
/// rule keeps from happening.
#[test]
fn a_vehicle_drives_a_dressed_street_without_shoving_a_car() {
    let presets = presets_with(cars_only);
    let rules = rules();
    for width in [7.0, 10.0] {
        let plan = town(&[street(width)], &[]);
        let props = place(&plan, &presets, &rules.catalog, 1);
        assert!(cars(&props).len() >= 20);
        // The cars as a street that parks on its carriageway would have them:
        // each moved toward the middle until its far side is at the kerb.
        let at_the_kerb: Vec<PropDefinition> = props
            .iter()
            .map(|prop| {
                let side = (prop.center[1] - 1000.0).signum();
                PropDefinition {
                    center: [
                        prop.center[0],
                        1000.0 + side * (width / 2.0 - prop.half_extents[1]),
                    ],
                    ..prop.clone()
                }
            })
            .collect();
        let request = request(MapType::Mixed, MapSize::Small, 1);
        let dressed = PreparedMap::new(&compiled(&plan, &props, &request), &rules);
        let crowded = PreparedMap::new(&compiled(&plan, &at_the_kerb, &request), &rules);
        let ends = [[720.0, 1000.0], [1280.0, 1000.0]];
        let mut shoved_at_the_kerb = false;
        for (name, m) in vehicles(&rules) {
            for (from, to) in [(ends[0], ends[1]), (ends[1], ends[0])] {
                let (points, pushes) = route(&dressed, &rules, &m, from, to)
                    .unwrap_or_else(|| panic!("no route for a {name} down a {width} m street"));
                assert!(
                    !pushes,
                    "a {name} shoves a body to drive a {width} m street"
                );
                let end = points[points.len() - 1];
                assert!(
                    (end.x - to[0]).abs() < 4.0 && (end.y - to[1]).abs() < 4.0,
                    "a {name} stops at {end:?}"
                );
                shoved_at_the_kerb |=
                    route(&crowded, &rules, &m, from, to).is_none_or(|(_, pushes)| pushes);
            }
        }
        if width == 7.0 {
            assert!(
                shoved_at_the_kerb,
                "cars on a 7 m carriageway were in no vehicle's way: the lane rule asks too much"
            );
        }
    }
}

/// Nothing stands on a bridge's deck or the run onto it, nor in a measured
/// open approach, though bodies stand along the rest of the same street.
#[test]
fn nothing_stands_on_a_bridge_or_in_an_open_approach() {
    let presets = presets_with(|source| {
        source["districts"]["centre"]["props"] = json!({
            "parking": 0.9,
            "verge": [{ "kind": "lamp", "spacing_m": 12, "sides": "both" }],
        });
    });
    let mut plan = town(&[street(7.0)], &[]);
    // A deck on the street's western stretch, and an approach that runs out
    // east from a settlement whose own ground ends at x = 1150.
    plan.bridges = vec![serde_json::from_value(json!({
        "deck": "bridge_deck", "center": [800, 1000], "half_extents": [15, 12],
        "yaw": 0, "deck_z": 0.1, "thickness_m": 0.8,
    }))
    .unwrap()];
    plan.settlements[0].outline = vec![
        [600.0, 800.0],
        [1150.0, 800.0],
        [1150.0, 1200.0],
        [600.0, 1200.0],
    ];
    plan.approaches = vec![serde_json::from_value(json!({
        "settlement": 0, "half": "top", "from_rad": 0, "to_rad": 0, "depth_m": 600, "front_m": 400,
    }))
    .unwrap()];
    let props = place(&plan, &presets, &rules().catalog, 1);
    let run = presets.rivers.bridge.approach_m;
    let corridors = approach_corridors(&plan);
    assert_eq!(corridors.len(), 1);
    let mut between = 0;
    for prop in &props {
        let x = prop.center[0];
        assert!(
            (x - 800.0).abs() - prop.half_extents[0] >= 15.0 + run - 0.011,
            "a {} stands at {:?}, on the deck or the run onto it",
            prop.kind,
            prop.center
        );
        assert!(
            x + prop.half_extents[0] <= 1150.0 + 0.011,
            "a {} stands at {:?}, in the open approach",
            prop.kind,
            prop.center
        );
        assert!(!corners(prop).iter().any(|p| corridors[0].contains(*p)));
        between += usize::from(x > 850.0 && x < 1140.0);
    }
    assert!(
        between >= 40,
        "{between} bodies along the open stretch of the street"
    );
}

/// The same request places the same bodies, byte for byte; another seed
/// places others.
#[test]
fn placement_is_deterministic() {
    let presets = presets();
    let catalog = rules().catalog;
    let request = request(MapType::Mixed, MapSize::Small, 1);
    let plan = fill_districts(
        generate_layout(&request, &presets).unwrap(),
        &request,
        &catalogue(),
        &presets,
    )
    .unwrap();
    let bytes = |seed: u64| {
        let request = GenerationRequest {
            seed: seed.into(),
            ..request.clone()
        };
        let props = place_street_props(&plan, &request, &catalogue(), &catalog, &presets).unwrap();
        assert!(props.iter().all(|prop| prop.id.is_none()));
        serde_json::to_string(&props).unwrap()
    };
    let first = bytes(1);
    assert!(first.len() > 10_000, "a town with next to no furniture");
    assert_eq!(first, bytes(1));
    assert_ne!(first, bytes(2));
}

/// The distance from `p` to the segment `ab`.
fn segment_distance(a: Point, b: Point, p: Point) -> f64 {
    let (ab, ap) = ([b[0] - a[0], b[1] - a[1]], [p[0] - a[0], p[1] - a[1]]);
    let t = ((ap[0] * ab[0] + ap[1] * ab[1]) / (ab[0] * ab[0] + ab[1] * ab[1])).clamp(0.0, 1.0);
    (ap[0] - t * ab[0]).hypot(ap[1] - t * ab[1])
}

/// Every carriageway's rounded centreline, as pieces bucketed by 64 m
/// squares: `near` answers how far a point is from the nearest piece's
/// middle line, and that piece's half width.
struct Roads {
    pieces: Vec<(Point, Point, f64)>,
    buckets: BTreeMap<(i64, i64), Vec<usize>>,
}

impl Roads {
    const BUCKET_M: f64 = 64.0;

    fn new(plan: &MapPlan) -> Self {
        let mut roads = Roads {
            pieces: Vec::new(),
            buckets: BTreeMap::new(),
        };
        for area in plan.surfaces.iter().filter(|area| area.kind.is_road()) {
            let GroundShape::Stroke {
                centerline,
                width_m,
            } = &area.shape
            else {
                continue;
            };
            for pair in centerline.samples().windows(2) {
                let [x0, x1] = [pair[0][0].min(pair[1][0]), pair[0][0].max(pair[1][0])]
                    .map(|v| (v / Self::BUCKET_M).floor() as i64);
                let [y0, y1] = [pair[0][1].min(pair[1][1]), pair[0][1].max(pair[1][1])]
                    .map(|v| (v / Self::BUCKET_M).floor() as i64);
                for x in x0 - 1..=x1 + 1 {
                    for y in y0 - 1..=y1 + 1 {
                        roads
                            .buckets
                            .entry((x, y))
                            .or_default()
                            .push(roads.pieces.len());
                    }
                }
                roads.pieces.push((pair[0], pair[1], width_m / 2.0));
            }
        }
        roads
    }

    /// The least of (distance to a piece's middle line, less `past` times
    /// its half width) over the pieces within a bucket of `p`.
    fn clearance(&self, p: Point, past: f64) -> f64 {
        let key = (
            (p[0] / Self::BUCKET_M).floor() as i64,
            (p[1] / Self::BUCKET_M).floor() as i64,
        );
        self.buckets
            .get(&key)
            .into_iter()
            .flatten()
            .map(|piece| {
                let (a, b, half) = self.pieces[*piece];
                segment_distance(a, b, p) - past * half
            })
            .fold(f64::INFINITY, f64::min)
    }
}

fn span(a: Point, b: Point) -> f64 {
    (b[0] - a[0]).hypot(b[1] - a[1])
}

/// The length of a route from `from` through `points`.
fn length(from: Point, points: &[V2]) -> f64 {
    let mut at = v2(from[0], from[1]);
    points
        .iter()
        .map(|next| {
            let step = (*next - at).length();
            at = *next;
            step
        })
        .sum()
}

/// A street fight between terraces has cover. On generated towns the cars
/// beside a street park at its kerb, and some of them stand before a row of
/// house fronts: between the carriageway and a wall a few metres behind.
#[test]
fn cars_park_at_the_kerb_and_before_the_terraces() {
    let presets = presets();
    let rules = rules();
    for seed in [1, 2, 3] {
        let request = request(MapType::Mixed, MapSize::Small, seed);
        let plan = fill_districts(
            generate_layout(&request, &presets).unwrap(),
            &request,
            &catalogue(),
            &presets,
        )
        .unwrap();
        let props = place(&plan, &presets, &rules.catalog, seed);
        let map = compiled(&plan, &[], &request);
        let roads = Roads::new(&plan);
        // How far a point is from the nearest terrace wall.
        let wall = |p: Point| {
            map.buildings
                .iter()
                .filter(|building| {
                    building.category == contract::templates::BuildingCategory::AttachedHome
                })
                .flat_map(|building| &building.geometry.parts)
                .map(|part| {
                    let (sin, cos) = part.yaw.sin_cos();
                    let d = [p[0] - part.center[0], p[1] - part.center[1]];
                    let local = [d[0] * cos + d[1] * sin, d[1] * cos - d[0] * sin];
                    let out =
                        [0, 1].map(|axis| (local[axis].abs() - part.half_extents[axis]).max(0.0));
                    out[0].hypot(out[1])
                })
                .fold(f64::INFINITY, f64::min)
        };
        // Cars beside a street (the others stand in yards and by farms),
        // those of them at its kerb, and those of them before a house front.
        let (mut beside, mut at_kerb, mut fronting) = (0, 0, 0);
        for car in cars(&props) {
            let off_kerb = roads.clearance(car.center, 1.0) - car.half_extents[1];
            if off_kerb > 8.0 {
                continue;
            }
            beside += 1;
            if off_kerb <= 2.0 {
                at_kerb += 1;
                fronting += usize::from(wall(car.center) - car.half_extents[1] <= 3.0);
            }
        }
        assert!(
            beside >= 100 && 10 * at_kerb >= 9 * beside,
            "seed {seed}: {at_kerb} of the {beside} cars beside a street stand within 2 m of its kerb"
        );
        assert!(
            fronting >= 20,
            "seed {seed}: {fronting} cars stand at the kerb before a house front"
        );
    }
}

/// A broken claim, said as it is found: a sweep's run shows every break.
fn note(claim: String) -> String {
    println!("BROKEN {claim}");
    claim
}

/// Record a broken claim and carry on, so one run shows every break.
macro_rules! claim {
    ($broken:ident, $ok:expr, $($why:tt)+) => {
        let holds: bool = $ok;
        if !holds {
            $broken.push(note(format!($($why)+)));
        }
    };
}

/// Routes survive the furniture. On maps of every type and size, bare and
/// dressed, the simulation is asked the same questions of each:
///
/// - the encounter planner still places the shipped assault where it did
///   before;
/// - a squad still stands outside every door it stood outside before, and
///   still walks there from its settlement's centre;
/// - every hull of the catalog still drives down every street it drove down
///   before, by a way no longer than a detour round one parked run, and the
///   widest of them without shoving a body.
///
/// And every body stands where the rules say: on the map, on ground, off
/// every carriageway and the lane beside its middle, off every bridge and
/// out of every open approach.
#[test]
fn routes_survive_the_furniture_on_every_type_and_size_of_map() {
    let presets = presets();
    let rules = rules();
    let recipes = EncounterRecipes::from_json(RECIPES).unwrap();
    let recipe = &recipes.recipes["assault"];
    let lane = lane(&presets, &rules.catalog);
    let vehicles = vehicles(&rules);
    // The largest squad of the catalog: the hardest to stand at a door.
    let squad = rules
        .catalog
        .indices()
        .map(|unit| rules.catalog.get(unit))
        .filter(|unit| unit.hull().is_none())
        .max_by_key(|unit| unit.squad_size())
        .unwrap();
    let on_foot = sim::units::mobility(squad, &rules);
    let mut totals = [0usize; 5];
    let mut broken: Vec<String> = Vec::new();
    for map_type in MapType::ALL {
        for size in MapSize::ALL {
            for seed in SWEEP_SEEDS {
                let name = format!("{} {} seed {seed}", map_type.name(), size.name());
                let request = request(map_type, size, seed);
                let bare = fill_districts(
                    generate_layout(&request, &presets).unwrap(),
                    &request,
                    &catalogue(),
                    &presets,
                )
                .unwrap_or_else(|errors| panic!("{name}: {errors:?}"));
                let props: Vec<PropDefinition> =
                    place_street_props(&bare, &request, &catalogue(), &rules.catalog, &presets)
                        .unwrap()
                        .into_iter()
                        .map(|prop| prop.geometry)
                        .collect();
                let maps = [
                    compiled(&bare, &[], &request),
                    compiled(&bare, &props, &request),
                ];
                let [before, after] = [0, 1].map(|arm| PreparedMap::new(&maps[arm], &rules));
                let sites = bare.sites();

                // Where every body stands.
                let roads = Roads::new(&bare);
                let corridors = approach_corridors(&bare);
                let run = presets.rivers.bridge.approach_m;
                for prop in &props {
                    let at = format!("{name}: a {} at {:?}", prop.kind, prop.center);
                    for corner in corners(prop) {
                        claim!(
                            broken,
                            corner.iter().all(|v| *v > 0.0)
                                && corner[0] < bare.size[0]
                                && corner[1] < bare.size[1],
                            "{at} is off the map"
                        );
                        claim!(
                            broken,
                            after.world.traversable_at(corner[0], corner[1]),
                            "{at} is in the water or on ground too steep"
                        );
                        claim!(
                            broken,
                            roads.clearance(corner, 0.0) >= lane - 0.011,
                            "{at} is in the lane beside a carriageway's middle"
                        );
                        claim!(
                            broken,
                            roads.clearance(corner, 1.0) > 0.0,
                            "{at} is on a carriageway"
                        );
                        claim!(
                            broken,
                            !corridors.iter().any(|corridor| corridor.contains(corner)),
                            "{at} is in an open approach"
                        );
                        for bridge in &bare.bridges {
                            let (sin, cos) = bridge.yaw.sin_cos();
                            let d = [corner[0] - bridge.center[0], corner[1] - bridge.center[1]];
                            let (along, across) =
                                (d[0] * cos + d[1] * sin, d[1] * cos - d[0] * sin);
                            claim!(
                                broken,
                                along.abs() > bridge.half_extents[0] + run - 0.011
                                    || across.abs() > bridge.half_extents[1] - 0.011,
                                "{at} is on a bridge or the run onto it"
                            );
                        }
                    }
                }

                // The planner.
                let plan = |arm: usize, prepared: &PreparedMap| {
                    sim::encounter::plan_encounter(
                        &prepared.queries(&maps[arm], &sites),
                        &rules,
                        recipe,
                        1.into(),
                    )
                };
                if plan(0, &before).is_ok() {
                    totals[0] += 1;
                    if let Err(diagnostics) = plan(1, &after) {
                        broken.push(note(format!(
                            "{name}: the assault no longer plans: {diagnostics:?}"
                        )));
                    }
                }

                // Doors.
                let stations: Vec<(V2, f64)> = maps[0]
                    .buildings
                    .iter()
                    .flat_map(|building| building.geometry.entrances.iter().flatten())
                    .map(|door| {
                        let out = recipe.garrison.door_standoff_m;
                        (
                            v2(
                                door.position[0] + door.normal[0] * out,
                                door.position[1] + door.normal[1] * out,
                            ),
                            door.normal[1].atan2(door.normal[0]),
                        )
                    })
                    .collect();
                let stands = |arm: usize, prepared: &PreparedMap, (at, yaw): (V2, f64)| {
                    stands(
                        &prepared.queries(&maps[arm], &sites),
                        &rules,
                        squad,
                        &on_foot,
                        at,
                        yaw,
                    )
                };
                let every = (stations.len() / DOORS_WALKED).max(1);
                for (index, station) in stations.iter().enumerate() {
                    if stands(0, &before, *station).is_err() {
                        continue;
                    }
                    totals[1] += 1;
                    if let Err(why) = stands(1, &after, *station) {
                        broken.push(note(format!(
                            "{name}: no squad stands at the door at {:?} any more: {}",
                            station.0,
                            why.describe()
                        )));
                    }
                    if index % every != 0 {
                        continue;
                    }
                    // From the centre of the settlement the door is in.
                    let goal = [station.0.x, station.0.y];
                    let Some(centre) = sites
                        .settlements
                        .iter()
                        .find(|s| polygon_contains(&s.outline, goal))
                        .map(|s| s.center)
                    else {
                        continue;
                    };
                    let arrives = |prepared: &PreparedMap| {
                        route(prepared, &rules, &on_foot, centre, goal).is_some_and(
                            |(points, _)| {
                                points
                                    .last()
                                    .is_some_and(|end| (*end - station.0).length() < 3.0)
                            },
                        )
                    };
                    if arrives(&before) {
                        totals[2] += 1;
                        claim!(
                            broken,
                            arrives(&after),
                            "{name}: no way on foot from {centre:?} to the door at {goal:?}"
                        );
                    }
                }

                // Streets.
                let streets: Vec<Vec<Point>> = bare
                    .surfaces
                    .iter()
                    .filter(|area| area.kind == SurfaceKind::Road)
                    .filter_map(|area| match &area.shape {
                        GroundShape::Stroke { centerline, .. } => {
                            Some(centerline.samples().to_vec())
                        }
                        GroundShape::Polygon { .. } => None,
                    })
                    .collect();
                let every = (streets.len() / STREETS_DRIVEN).max(1);
                for (index, street) in streets
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| index % every == 0)
                {
                    // Its longest straight run, from a hull's length inside
                    // one end to as far inside the other, each way on
                    // alternate streets.
                    let run = street
                        .windows(2)
                        .max_by(|a, b| span(a[0], a[1]).total_cmp(&span(b[0], b[1])))
                        .unwrap();
                    let stretch = span(run[0], run[1]);
                    if stretch < 60.0 {
                        continue;
                    }
                    let inside = |share: f64| {
                        [0, 1].map(|axis| run[0][axis] + (run[1][axis] - run[0][axis]) * share)
                    };
                    let mut ends = [inside(12.0 / stretch), inside(1.0 - 12.0 / stretch)];
                    if index / every % 2 == 1 {
                        ends.swap(0, 1);
                    }
                    for (rank, (kind, m)) in vehicles.iter().enumerate() {
                        let arrives = |prepared: &PreparedMap| {
                            route(prepared, &rules, m, ends[0], ends[1]).filter(|(points, _)| {
                                points.last().is_some_and(|end| {
                                    (*end - v2(ends[1][0], ends[1][1])).length() < 6.0
                                })
                            })
                        };
                        // A drive down the street: by the street, not round it.
                        let Some((open, shoved_before)) = arrives(&before)
                            .filter(|(points, _)| length(ends[0], points) <= stretch + DETOUR_M)
                        else {
                            continue;
                        };
                        totals[3] += 1;
                        let Some((dressed, shoves)) = arrives(&after) else {
                            broken.push(note(format!(
                                "{name}: no route for a {kind} from {:?} to {:?} any more",
                                ends[0], ends[1]
                            )));
                            continue;
                        };
                        let (was, now) = (length(ends[0], &open), length(ends[0], &dressed));
                        claim!(broken,
                            now <= was + DETOUR_M,
                            "{name}: a {kind} drives {now:.0} m from {:?} to {:?}, {was:.0} m on bare streets",
                            ends[0],
                            ends[1]
                        );
                        if rank == 0 && !shoved_before {
                            totals[4] += 1;
                            claim!(
                                broken,
                                !shoves,
                                "{name}: a {kind} shoves a body to drive from {:?} to {:?}",
                                ends[0],
                                ends[1]
                            );
                        }
                    }
                }
                println!(
                    "{name}: {} bodies; {totals:?}; {} broken",
                    props.len(),
                    broken.len()
                );
            }
        }
    }
    // The sweep asked something: assaults planned, doors stood at and walked
    // to, streets driven, and driven by the widest hull without a shove.
    assert!(
        broken.is_empty(),
        "{} claims broken:\n{}",
        broken.len(),
        broken
            .iter()
            .take(40)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert!(
        totals[0] >= 9
            && totals[1] >= 5_000
            && totals[2] >= 200
            && totals[3] >= 500
            && totals[4] >= 150,
        "{totals:?}"
    );
}

/// Yard stock and construction sites keep to parcels. On a generated town
/// with industry: every skip and pallet stack stands on a parcel, those of
/// a built parcel within a few metres of its building's wall; every site's
/// cabin stands on a parcel the parcel pass left open, with a fence round
/// it; and no settlement has more sites than the rule allows.
#[test]
fn yard_stock_and_sites_keep_to_their_parcels() {
    let presets = presets();
    let rules = rules();
    let seed = (1..=40)
        .find(|seed| {
            generate_layout(&request(MapType::Mixed, MapSize::Small, *seed), &presets).is_ok_and(
                |plan| {
                    plan.settlements
                        .iter()
                        .flat_map(|settlement| &settlement.districts)
                        .any(|district| district.kind == "industrial")
                },
            )
        })
        .expect("a map with industry among forty seeds");
    let request = request(MapType::Mixed, MapSize::Small, seed);
    let plan = fill_districts(
        generate_layout(&request, &presets).unwrap(),
        &request,
        &catalogue(),
        &presets,
    )
    .unwrap();
    let props = place(&plan, &presets, &rules.catalog, seed);
    let map = compiled(&plan, &props, &request);
    let built: BTreeMap<&str, usize> = plan
        .buildings
        .iter()
        .enumerate()
        .map(|(index, building)| (building.id.as_str(), index))
        .collect();
    // The parcel a body stands wholly on.
    let parcel = |prop: &PropDefinition| {
        plan.lots.iter().find(|lot| {
            corners(prop)
                .iter()
                .all(|corner| polygon_contains(&lot.ring, *corner))
        })
    };
    let (mut stock, mut beside, mut cabins, mut panels) = (0, 0, BTreeMap::new(), 0);
    for prop in &props {
        let kind = prop.kind.as_str();
        if !["skip_bin", "pallet_stack", "site_cabin", "heras_fence"].contains(&kind) {
            continue;
        }
        let lot = parcel(prop)
            .unwrap_or_else(|| panic!("a {kind} at {:?} stands on no parcel", prop.center));
        match (kind, built.get(lot.id.as_str())) {
            ("site_cabin", None) => {
                let settlement = lot.id.split('/').next().unwrap();
                *cabins.entry(settlement).or_insert(0) += 1;
            }
            ("heras_fence", None) => panels += 1,
            ("site_cabin" | "heras_fence", Some(_)) => {
                panic!("a {kind} stands on the built parcel {}", lot.id)
            }
            // Stock in a yard: by a wall of the parcel's own building.
            (_, Some(building)) => {
                stock += 1;
                let nearest = map.buildings[*building]
                    .geometry
                    .parts
                    .iter()
                    .map(|part| {
                        let (sin, cos) = part.yaw.sin_cos();
                        let d = [
                            prop.center[0] - part.center[0],
                            prop.center[1] - part.center[1],
                        ];
                        let local = [d[0] * cos + d[1] * sin, d[1] * cos - d[0] * sin];
                        let out = [0, 1]
                            .map(|axis| (local[axis].abs() - part.half_extents[axis]).max(0.0));
                        out[0].hypot(out[1])
                    })
                    .fold(f64::INFINITY, f64::min);
                assert!(
                    nearest > 0.5 && nearest < 5.0,
                    "a {kind} at {:?} stands {nearest:.1} m from its building",
                    prop.center
                );
                beside += 1;
            }
            // Stock on an open parcel is a site's.
            (_, None) => stock += 1,
        }
    }
    assert!(
        beside >= 20,
        "{beside} of {stock} bodies of stock stand in a yard"
    );
    let most = presets.street_props.site.max_per_settlement;
    assert!(
        !cabins.is_empty() && cabins.values().all(|sites| *sites <= most),
        "{cabins:?}"
    );
    // Each site's gate is open to the street: nothing but the fence's own
    // panels stands in the opening or before it.
    let gate = presets.street_props.site.gate_m;
    let sites: Vec<&mapgen::LotPlan> = props
        .iter()
        .filter(|prop| prop.kind == "site_cabin")
        .map(|cabin| parcel(cabin).unwrap())
        .collect();
    for lot in sites {
        let (from, to, back) = (lot.ring[0], lot.ring[1], lot.ring[3]);
        let middle = [(from[0] + to[0]) / 2.0, (from[1] + to[1]) / 2.0];
        let unit = |a: Point, b: Point| {
            let length = (b[0] - a[0]).hypot(b[1] - a[1]);
            [(b[0] - a[0]) / length, (b[1] - a[1]) / length]
        };
        let (along, inward) = (unit(from, to), unit(from, back));
        for prop in props.iter().filter(|prop| prop.kind != "heras_fence") {
            let d = [prop.center[0] - middle[0], prop.center[1] - middle[1]];
            let (x, y) = (
                d[0] * along[0] + d[1] * along[1],
                d[0] * inward[0] + d[1] * inward[1],
            );
            assert!(
                x.abs() >= gate / 2.0 || !(-5.0..2.0).contains(&y),
                "a {} at {:?} stands in the gate of the site on {}",
                prop.kind,
                prop.center,
                lot.id
            );
        }
    }
    assert!(
        panels >= 8 * cabins.values().sum::<u32>(),
        "{panels} fence panels for {cabins:?}"
    );
}
