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
use mapgen::street_props::{place_courts_and_gardens, place_street_props};
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
/// How many dressed gardens of a map a squad walks to the back of.
const GARDENS_WALKED: usize = 40;
/// How many places of a map's dense districts a squad walks to and the
/// widest hull drives to (its yards' gates, its car parks and points of a
/// grid this fine over its lawn), spread evenly through the map's own order.
const COURTS_WALKED: usize = 60;
const COURT_STEP_M: f64 = 6.0;
/// How many lawns of a map the widest hull is asked into, and at how many
/// of each one's points it is tried on the bare map (three times as many on
/// the dressed one).
const LAWNS_DRIVEN: usize = 20;
const LAWN_TRIES: usize = 3;
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

/// The street furniture of `plan` under `request`, then its courts and
/// gardens, as the generator places them after the open country's cover: the
/// bodies of all.
fn dressed(
    plan: &MapPlan,
    request: &GenerationRequest,
    presets: &PresetDefinitions,
    rules: &Rules,
) -> Vec<AuthoredPropDefinition> {
    let mut props =
        place_street_props(plan, request, &catalogue(), &rules.catalog, presets).unwrap();
    let mut furnished = plan.clone();
    furnished.props.extend(props.iter().cloned());
    props.extend(
        place_courts_and_gardens(
            &furnished,
            request,
            &catalogue(),
            &rules.catalog,
            presets,
            rules.forests.rule.trunk_clearance_m,
        )
        .unwrap(),
    );
    props
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

/// The garden rows these tests state for the suburb and the village,
/// whatever the shipped presets are tuned to: every lot dressed, a boundary
/// round most of them, as many pieces as a rear garden holds. The dense
/// districts' courts stand no amenity groups, so every garden kind placed
/// (a lawn's hedge is one) is a garden's.
const GARDEN_PIECES: [&str; 3] = ["garden_shed", "washing_line", "garden_table"];
const GARDEN_BOUNDARY: [&str; 2] = ["hedge", "garden_fence"];
const GARDEN_MOST: usize = 12;

fn gardens(source: &mut Value) {
    for kind in ["garden_suburb", "village"] {
        source["districts"][kind]["props"]["gardens"] = json!({
            "depth_m": 8,
            "room_m": 2,
            "boundary_chance": 0.9,
            "boundary": { "hedge": 1, "garden_fence": 1 },
            "pieces": GARDEN_PIECES.map(|kind| json!({ "kind": kind, "count": [0, 1] })),
            "max_per_lot": GARDEN_MOST,
        });
    }
    for kind in COURT_DISTRICTS {
        let courts = &mut source["districts"][kind]["props"]["courts"];
        courts["groups"] = json!({});
        courts["families"] = json!({});
    }
}

/// A lot's own frame: `(along its front from the first corner, in from its
/// front)`, with its width and depth.
struct LotFrame {
    origin: Point,
    along: Point,
    inward: Point,
    width: f64,
    depth: f64,
}

impl LotFrame {
    fn new(ring: &[Point]) -> Self {
        let unit = |a: Point, b: Point| {
            let length = span(a, b);
            ([(b[0] - a[0]) / length, (b[1] - a[1]) / length], length)
        };
        let (along, width) = unit(ring[0], ring[1]);
        let (inward, depth) = unit(ring[0], ring[3]);
        Self {
            origin: ring[0],
            along,
            inward,
            width,
            depth,
        }
    }

    fn local(&self, p: Point) -> Point {
        let d = [p[0] - self.origin[0], p[1] - self.origin[1]];
        [
            d[0] * self.along[0] + d[1] * self.along[1],
            d[0] * self.inward[0] + d[1] * self.inward[1],
        ]
    }

    fn at(&self, x: f64, y: f64) -> Point {
        [
            self.origin[0] + self.along[0] * x + self.inward[0] * y,
            self.origin[1] + self.along[1] * x + self.inward[1] * y,
        ]
    }
}

/// The district kind each district id of `plan` is.
fn district_kinds(plan: &MapPlan) -> BTreeMap<&str, &str> {
    plan.settlements
        .iter()
        .flat_map(|settlement| &settlement.districts)
        .map(|district| (district.id.as_str(), district.kind.as_str()))
        .collect()
}

/// Gardens are dressed behind the houses of suburbs and villages, and
/// nowhere else. On a generated town every shed, washing line and garden
/// table stands wholly inside a built lot of a suburb or a village, in its
/// rear setback; every hedge and garden fence runs along that lot's rear or
/// side edge, inside it, and none reaches into the front garden; and no lot
/// holds more than its cap.
#[test]
fn gardens_keep_to_the_back_of_their_own_lots() {
    let presets = presets_with(gardens);
    let rules = rules();
    let request = request(MapType::Mixed, MapSize::Small, 1);
    let plan = fill_districts(
        generate_layout(&request, &presets).unwrap(),
        &request,
        &catalogue(),
        &presets,
    )
    .unwrap();
    let props: Vec<PropDefinition> = dressed(&plan, &request, &presets, &rules)
        .into_iter()
        .map(|prop| prop.geometry)
        .collect();
    let kinds = district_kinds(&plan);
    let built: std::collections::BTreeSet<&str> =
        plan.buildings.iter().map(|b| b.id.as_str()).collect();
    let mut per_lot: BTreeMap<&str, usize> = BTreeMap::new();
    let mut per_kind: BTreeMap<&str, usize> = BTreeMap::new();
    for prop in &props {
        let kind = prop.kind.as_str();
        let boundary = GARDEN_BOUNDARY.contains(&kind);
        if !boundary && !GARDEN_PIECES.contains(&kind) {
            continue;
        }
        let at = format!("a {kind} at {:?}", prop.center);
        let lot = plan
            .lots
            .iter()
            .find(|lot| {
                corners(prop)
                    .iter()
                    .all(|corner| polygon_contains(&lot.ring, *corner))
            })
            .unwrap_or_else(|| panic!("{at} stands on no one lot"));
        let (district, _) = lot.id.rsplit_once("/lot-").unwrap();
        let district = kinds[district];
        assert!(
            built.contains(lot.id.as_str()) && ["garden_suburb", "village"].contains(&district),
            "{at} stands on {}, a {} lot{}",
            lot.id,
            district,
            if built.contains(lot.id.as_str()) {
                ""
            } else {
                " left open"
            }
        );
        let setbacks = &presets.districts[district].lots;
        let frame = LotFrame::new(&lot.ring);
        let local = corners(prop).map(|corner| frame.local(corner));
        assert!(
            local.iter().all(|p| p[1] >= setbacks.front_m - 0.011),
            "{at} stands in the front garden of {}",
            lot.id
        );
        if boundary {
            // Along the rear edge or a side edge, close inside it.
            let centre = frame.local(prop.center);
            let reach = prop.half_extents[1] + 0.2;
            let on_rear = frame.depth - centre[1] <= reach;
            let on_side = centre[0] <= reach || frame.width - centre[0] <= reach;
            assert!(
                on_rear || on_side,
                "{at} runs along no rear or side edge of {}",
                lot.id
            );
        } else {
            assert!(
                local
                    .iter()
                    .all(|p| p[1] >= frame.depth - setbacks.rear_m - 0.011),
                "{at} stands outside the rear setback of {}",
                lot.id
            );
        }
        *per_lot.entry(lot.id.as_str()).or_insert(0) += 1;
        *per_kind.entry(kind).or_insert(0) += 1;
    }
    let most = per_lot.values().max().copied().unwrap_or(0);
    assert!(most <= GARDEN_MOST, "a lot holds {most} garden bodies");
    for kind in GARDEN_PIECES.iter().chain(&GARDEN_BOUNDARY) {
        let count = per_kind.get(kind).copied().unwrap_or(0);
        assert!(
            count >= 20,
            "{count} {kind}s on a town of suburbs and villages"
        );
    }
    assert!(per_lot.len() >= 200, "{} lots have a garden", per_lot.len());
}

/// The court groups these tests state, whatever the shipped presets are
/// tuned to: one shared group and one for each family, each made of kinds no
/// other group or dressing uses, so a placed body names its group. The New
/// York group is fenced, with a gate.
fn court_groups(source: &mut Value) {
    source["street_props"]["groups"] = json!({
        "play": {
            "size_m": [11, 4],
            "pieces": [
                { "kind": "playground_frame", "at": [-2.8, 0], "yaw_deg": 0 },
                { "kind": "swing", "at": [2.5, 0], "yaw_deg": 0 },
            ],
        },
        "ping": {
            "size_m": [10.5, 2.5],
            "pieces": [
                { "kind": "pingpong_table", "at": [-3, 0], "yaw_deg": 0 },
                { "kind": "pingpong_table", "at": [3, 0], "yaw_deg": 0 },
            ],
        },
        "hoops": {
            "size_m": [18.45, 18.45],
            "fence": "chainlink_fence",
            "gate_m": 3,
            "pieces": [
                { "kind": "basketball_court", "at": [0.6, 0], "yaw_deg": 0 },
                { "kind": "basketball_hoop", "at": [-7, 0], "yaw_deg": 90 },
            ],
        },
        "boules": {
            "size_m": [15.2, 7],
            "pieces": [
                { "kind": "petanque_pitch", "at": [0, 0.5], "yaw_deg": 0 },
                { "kind": "kiosk", "at": [0, -2.5], "yaw_deg": 0 },
            ],
        },
    });
    for kind in COURT_DISTRICTS {
        source["districts"][kind]["props"]["courts"] = json!({
            "paved": true,
            "spacing_m": 40,
            "groups": { "play": 1 },
            "families": {
                "china": { "ping": 1 },
                "new_york": { "hoops": 1 },
                "paris": { "boules": 1 },
            },
        });
    }
}

/// The district kinds that pave a court.
const COURT_DISTRICTS: [&str; 4] = ["small_centre", "centre", "apartments", "core"];
/// The kinds only one family's court group above places.
const SIGNATURES: [(&str, &[&str]); 3] = [
    ("china", &["pingpong_table"]),
    (
        "new_york",
        &["basketball_court", "basketball_hoop", "chainlink_fence"],
    ),
    ("paris", &["petanque_pitch", "kiosk"]),
];

/// Every kind a court group of `presets` places, its fences' included.
fn court_kinds(presets: &PresetDefinitions) -> std::collections::BTreeSet<&str> {
    presets
        .street_props
        .groups
        .values()
        .flat_map(|group| {
            group
                .pieces
                .iter()
                .map(|piece| piece.kind.as_str())
                .chain(group.fence.as_deref())
        })
        .collect()
}

/// The kinds only court groups of `presets` place: no street, yard, site or
/// garden row names them.
fn court_only_kinds(presets: &PresetDefinitions) -> std::collections::BTreeSet<&str> {
    let rule = &presets.street_props;
    let mut elsewhere: std::collections::BTreeSet<&str> = [
        rule.parking.kind.as_str(),
        rule.site.cabin.as_str(),
        rule.site.fence.as_str(),
    ]
    .into_iter()
    .chain(rule.site.stock.iter().map(|row| row.kind.as_str()))
    .collect();
    for district in presets.districts.values() {
        let props = &district.props;
        elsewhere.extend(props.verge.iter().map(|row| row.kind.as_str()));
        elsewhere.extend(props.yard.iter().map(|row| row.kind.as_str()));
        if let Some(gardens) = &props.gardens {
            elsewhere.extend(gardens.pieces.iter().map(|row| row.kind.as_str()));
            elsewhere.extend(gardens.boundary.keys().map(String::as_str));
        }
    }
    court_kinds(presets)
        .into_iter()
        .filter(|kind| !elsewhere.contains(kind))
        .collect()
}

/// A town of `region`, its parcels built and every dressing placed, by the
/// court groups above.
fn court_town(
    region: &str,
    seed: u64,
) -> (
    PresetDefinitions,
    GenerationRequest,
    MapPlan,
    Vec<PropDefinition>,
) {
    let presets = presets_with(court_groups);
    let mut request = request(MapType::Mixed, MapSize::Small, seed);
    request.region = Some(region.into());
    let plan = fill_districts(
        generate_layout(&request, &presets).unwrap(),
        &request,
        &catalogue(),
        &presets,
    )
    .unwrap();
    let props = dressed(&plan, &request, &presets, &rules())
        .into_iter()
        .map(|prop| prop.geometry)
        .collect();
    (presets, request, plan, props)
}

/// Where a group's piece stands when its group stands at `centre` turned to
/// `yaw`: its centre and its own yaw.
fn piece_at(centre: Point, yaw: f64, at: [f64; 2], yaw_deg: f64) -> (Point, f64) {
    let (sin, cos) = yaw.sin_cos();
    (
        [
            centre[0] + cos * at[0] - sin * at[1],
            centre[1] + sin * at[0] + cos * at[1],
        ],
        yaw + yaw_deg.to_radians(),
    )
}

/// Whether two yaws lie the same way, to a box's symmetry.
fn same_heading(a: f64, b: f64) -> bool {
    let turn = (a - b).rem_euclid(std::f64::consts::PI);
    turn < 1e-3 || std::f64::consts::PI - turn < 1e-3
}

/// Court amenities stand whole, in yards or on the lawn, and only a map's
/// own family's signature pieces stand on it. On a town of each region every
/// piece of a court group lies wholly inside a yard, or on a dense
/// district's lawn off every yard and car park, off every building's walls; every piece belongs to a group
/// whose every piece stands where the group puts it (a fenced group with its
/// fence round it and its gate open); both the shared group and the map's
/// own family's group are placed; and no kind of another family's group is.
#[test]
fn court_groups_stand_whole_in_their_courts_and_keep_to_their_family() {
    for (family, own) in SIGNATURES {
        let (presets, request, plan, props) = court_town(family, 1);
        let groups = &presets.street_props.groups;
        let map = compiled(&plan, &[], &request);
        let wall_gap = presets.street_props.wall_gap_m;
        let yards: Vec<&[Point]> = plan
            .courts
            .iter()
            .filter(|c| c.kind == mapgen::CourtKind::Yard)
            .map(|c| c.ring.as_slice())
            .collect();
        let paved: Vec<&[Point]> = plan.courts.iter().map(|c| c.ring.as_slice()).collect();
        let dense: Vec<&[Point]> = plan
            .settlements
            .iter()
            .flat_map(|s| &s.districts)
            .filter(|d| COURT_DISTRICTS.contains(&d.kind.as_str()))
            .map(|d| d.ring.as_slice())
            .collect();
        let court_kinds = court_kinds(&presets);
        // Every court piece, and whether a whole group claims it.
        let pieces: Vec<&PropDefinition> = props
            .iter()
            .filter(|prop| court_kinds.contains(prop.kind.as_str()))
            .collect();
        for prop in &pieces {
            let at = format!("{family}: a {} at {:?}", prop.kind, prop.center);
            let within = |ring: &&[Point]| corners(prop).iter().all(|p| polygon_contains(ring, *p));
            let on_lawn = dense.iter().any(within)
                && !paved
                    .iter()
                    .any(|ring| corners(prop).iter().any(|p| polygon_contains(ring, *p)));
            assert!(
                yards.iter().any(within) || on_lawn,
                "{at} stands in no yard and on no lawn"
            );
            for (others, signature) in SIGNATURES {
                assert!(
                    others == family || !signature.contains(&prop.kind.as_str()),
                    "{at} is a {others} piece"
                );
            }
            let nearest = map
                .buildings
                .iter()
                .flat_map(|building| &building.geometry.parts)
                .map(|part| {
                    corners(prop)
                        .into_iter()
                        .chain([prop.center])
                        .map(|p| {
                            let (sin, cos) = part.yaw.sin_cos();
                            let d = [p[0] - part.center[0], p[1] - part.center[1]];
                            let local = [d[0] * cos + d[1] * sin, d[1] * cos - d[0] * sin];
                            let out = [0, 1]
                                .map(|axis| (local[axis].abs() - part.half_extents[axis]).max(0.0));
                            out[0].hypot(out[1])
                        })
                        .fold(f64::INFINITY, f64::min)
                })
                .fold(f64::INFINITY, f64::min);
            // A body's corners against a wall's box: within a corner's
            // reach of the true gap, which the rule keeps at the wall gap.
            assert!(
                nearest >= wall_gap - 0.011,
                "{at} stands {nearest:.2} m from a building"
            );
        }
        let mut claimed = vec![false; pieces.len()];
        let mut placed: BTreeMap<&str, usize> = BTreeMap::new();
        for (name, group) in groups {
            let first = &group.pieces[0];
            for anchor in pieces.iter().filter(|prop| prop.kind == first.kind) {
                // The group as its first piece puts it.
                let yaw = anchor.yaw - first.yaw_deg.to_radians();
                let (offset, _) = piece_at([0.0, 0.0], yaw, first.at, 0.0);
                let centre = [anchor.center[0] - offset[0], anchor.center[1] - offset[1]];
                let find = |kind: &str, (at, heading): (Point, f64)| {
                    pieces.iter().position(|prop| {
                        prop.kind == kind
                            && span(prop.center, at) < 0.03
                            && same_heading(prop.yaw, heading)
                    })
                };
                let found: Option<Vec<usize>> = group
                    .pieces
                    .iter()
                    .map(|piece| find(&piece.kind, piece_at(centre, yaw, piece.at, piece.yaw_deg)))
                    .collect();
                let Some(mut found) = found else {
                    continue;
                };
                if let Some(fence) = &group.fence {
                    // Panels along the group's edge, the gate's middle open.
                    let half = [group.size_m[0] / 2.0, group.size_m[1] / 2.0];
                    let local = |p: Point| {
                        let (sin, cos) = yaw.sin_cos();
                        let d = [p[0] - centre[0], p[1] - centre[1]];
                        [d[0] * cos + d[1] * sin, d[1] * cos - d[0] * sin]
                    };
                    let panels: Vec<usize> = (0..pieces.len())
                        .filter(|index| {
                            let p = local(pieces[*index].center);
                            pieces[*index].kind == *fence
                                && p[0].abs() <= half[0] + 0.01
                                && p[1].abs() <= half[1] + 0.01
                                && (half[0] - p[0].abs() < 0.2 || half[1] - p[1].abs() < 0.2)
                        })
                        .collect();
                    let gate = group.gate_m.unwrap();
                    let gate_open = panels.iter().all(|index| {
                        let p = local(pieces[*index].center);
                        let reach = pieces[*index].half_extents[0];
                        !(p[1] < -half[1] + 0.2 && p[0].abs() - reach < gate / 2.0 - 0.011)
                    });
                    let sides = [
                        |p: Point, h: [f64; 2]| p[1] < -h[1] + 0.2,
                        |p: Point, h: [f64; 2]| p[0] > h[0] - 0.2,
                        |p: Point, h: [f64; 2]| p[1] > h[1] - 0.2,
                        |p: Point, h: [f64; 2]| p[0] < -h[0] + 0.2,
                    ];
                    // Each side fenced from end to end bar its gate: its panels
                    // run its length to within one panel.
                    let panel = panel_length(&presets, fence);
                    let closed = sides.iter().enumerate().all(|(side, on)| {
                        let length = 2.0 * half[side % 2];
                        let count = panels
                            .iter()
                            .filter(|index| on(local(pieces[**index].center), half))
                            .count() as f64;
                        let open = if side == 0 { gate + panel } else { 0.0 };
                        count * panel >= length - open - panel
                    });
                    assert!(
                        gate_open && closed,
                        "{family}: the fence of the {name} at {centre:?} is not whole: {} panels",
                        panels.len()
                    );
                    found.extend(panels);
                }
                *placed.entry(name.as_str()).or_insert(0) += 1;
                for index in found {
                    claimed[index] = true;
                }
            }
        }
        for (index, prop) in pieces.iter().enumerate() {
            assert!(
                claimed[index],
                "{family}: a {} at {:?} belongs to no whole group",
                prop.kind, prop.center
            );
        }
        let own_group = match family {
            "china" => "ping",
            "new_york" => "hoops",
            _ => "boules",
        };
        assert!(
            placed.get("play").copied().unwrap_or(0) >= 10
                && placed.get(own_group).copied().unwrap_or(0) >= 3,
            "{family}: groups placed {placed:?}"
        );
        for kind in own {
            assert!(
                pieces.iter().any(|prop| prop.kind == *kind),
                "{family}: no {kind} placed"
            );
        }
    }
}

/// The presets refuse a court group that names a body `street_props` lacks,
/// a family table for a family the map regions do not list, a court table
/// naming a group that does not exist, and a fenced group whose fixed-length
/// panels leave its corners open.
#[test]
fn a_court_group_naming_an_unknown_body_family_or_group_is_refused() {
    let edits: [(&str, fn(&mut Value)); 4] = [
        ("street_props.groups.play", |source| {
            source["street_props"]["groups"]["play"]["pieces"][0]["kind"] = "gazebo".into();
        }),
        ("districts.core.props.courts", |source| {
            source["districts"]["core"]["props"]["courts"]["families"]["atlantis"] =
                json!({ "play": 1 });
        }),
        ("districts.core.props.courts", |source| {
            source["districts"]["core"]["props"]["courts"]["groups"] = json!({ "maze": 1 });
        }),
        // A fixed-length fence (a modular one is cut to its sides).
        ("street_props.groups.hoops", |source| {
            source["street_props"]["groups"]["hoops"]["fence"] = "heras_fence".into();
            source["street_props"]["groups"]["hoops"]["size_m"] = json!([20, 18.45]);
        }),
    ];
    for (location, edit) in edits {
        let mut source: Value = serde_json::from_str(PRESETS).unwrap();
        court_groups(&mut source);
        assert!(PresetDefinitions::from_json(&source.to_string()).is_ok());
        edit(&mut source);
        let refused = PresetDefinitions::from_json(&source.to_string())
            .err()
            .unwrap_or_else(|| panic!("an edit at {location} was admitted"));
        assert!(
            refused.iter().any(|d| d.location.contains(location)),
            "{refused:?}"
        );
    }
}

/// A panel's length of the fence body `kind`.
fn panel_length(presets: &PresetDefinitions, kind: &str) -> f64 {
    2.0 * presets.street_props.bodies[kind].half_extents_m[0]
}

/// A garden row names a body of `street_props`, and the presets refuse one
/// that does not, as they refuse a verge row's.
#[test]
fn a_garden_row_naming_an_unknown_body_is_refused() {
    for (field, row) in [
        ("pieces", json!([{ "kind": "gazebo", "count": [0, 1] }])),
        ("boundary", json!({ "gazebo": 1 })),
    ] {
        let mut source: Value = serde_json::from_str(PRESETS).unwrap();
        gardens(&mut source);
        source["districts"]["village"]["props"]["gardens"][field] = row;
        let refused = PresetDefinitions::from_json(&source.to_string())
            .err()
            .unwrap_or_else(|| panic!("a {field} row naming a gazebo was admitted"));
        assert!(
            refused
                .iter()
                .any(|d| d.location.contains("districts.village.props")),
            "{refused:?}"
        );
    }
}

/// The largest map the sweep builds, its courts and gardens dressed, stays
/// within the authored-part limit a generated battle is admitted under:
/// courts take the parts the rest of the map leaves first, gardens what the
/// courts leave, and on this map both are dressed.
#[test]
fn a_city_with_its_courts_and_gardens_stays_within_the_part_limit() {
    let defaults: Value =
        serde_json::from_str(include_str!("../../../fixtures/generated-battle.json")).unwrap();
    let limit = defaults["limits"]["max_authored_parts"].as_u64().unwrap();
    let presets = presets();
    let courts = court_only_kinds(&presets);
    let mut request = request(MapType::Metro, MapSize::Large, 2);
    request.limits = serde_json::from_value(defaults["limits"].clone()).unwrap();
    let result = match mapgen::generate_map(
        &serde_json::to_string(&request).unwrap(),
        PRESETS,
        include_str!("../../../fixtures/prototype-building-templates.json"),
        &sim::fixtures::game().to_string(),
    ) {
        mapgen::CompileOutcome::Ok { result } => result,
        mapgen::CompileOutcome::Error { diagnostics } => panic!("{diagnostics:?}"),
    };
    let count = |kinds: &dyn Fn(&str) -> bool| {
        result
            .map
            .props
            .iter()
            .filter(|prop| kinds(prop.geometry.kind.as_str()))
            .count()
    };
    let gardens = count(&|kind| GARDEN_PIECES.contains(&kind) || GARDEN_BOUNDARY.contains(&kind));
    let court = count(&|kind| courts.contains(kind));
    assert!(
        u64::from(result.report.authored_parts) <= limit,
        "{} authored parts",
        result.report.authored_parts
    );
    assert!(
        gardens >= 2_000 && court >= 500,
        "{gardens} garden bodies and {court} court amenities on a city"
    );
}

/// Courts and gardens give way to the open country's sight certificate,
/// which stands copses on a town's open ground: a map builds with its courts
/// and gardens dressed, and no body of either in the finished plan stands
/// within a trunk's clearance of any forest, so none fells a tree the
/// certificate counted.
#[test]
fn courts_and_gardens_keep_off_the_woods_that_certify_sight() {
    let rules = rules();
    let presets = presets();
    let courts = court_only_kinds(&presets);
    let clear = rules.forests.rule.trunk_clearance_m;
    let mut request = request(MapType::Metro, MapSize::Small, 1);
    request.region = Some("china".into());
    let (plan, _) = mapgen::generate_with_plan(
        &serde_json::to_string(&request).unwrap(),
        PRESETS,
        TEMPLATES,
        &sim::fixtures::game().to_string(),
    )
    .unwrap_or_else(|failure| panic!("{:?}", failure.diagnostics));
    let (mut gardens, mut court) = (0, 0);
    for prop in plan.props.iter().map(|prop| &prop.geometry) {
        let kind = prop.kind.as_str();
        if courts.contains(kind) {
            court += 1;
        } else if GARDEN_PIECES.contains(&kind) || GARDEN_BOUNDARY.contains(&kind) {
            gardens += 1;
        } else {
            continue;
        }
        for p in corners(prop).into_iter().chain([prop.center]) {
            assert!(
                !plan
                    .forests
                    .iter()
                    .any(|forest| forest.shape.contains(p, clear)),
                "a {kind} at {:?} stands within a trunk's clearance of a forest",
                prop.center
            );
        }
    }
    assert!(
        gardens >= 1_000 && court >= 100,
        "{gardens} garden bodies and {court} court bodies"
    );
}

/// The regions a map is generated in.
const REGIONS: [&str; 3] = ["china", "new_york", "paris"];
/// The structure sweep runs every type, size and region on these seeds.
const FILL_SEEDS: [u64; 3] = [1, 2, 3];
/// Court paving is open farther than this from every body, building wall,
/// carriageway and lawn: a stretch of bare floor wider than a street with
/// its verges, which reads as an empty plaza.
const OPEN_M: f64 = 6.0;
/// The court ground is sampled at points this far apart.
const FILL_STEP_M: f64 = 2.0;
/// At most this share of each region's court paving is open, over every map
/// (before yards, car parks and lawns it was 19–20 %).
const MOST_OPEN: f64 = 0.05;
/// At most this share of the court paving of each district kind of each
/// region is open, over the maps whose authored parts stop short of the
/// request's limit: where the courts are not short of parts (before, the
/// apartments' courts were 29–31 % open).
const MOST_OPEN_DRESSED: f64 = 0.04;
/// A dense district's lawn is empty farther than this from every body,
/// building wall, carriageway and paved area: wider than a squad's field of
/// cover either side, it reads as a bare field between the blocks.
const EMPTY_LAWN_M: f64 = 8.0;
/// At most this share of each region's lawn is empty, over every map
/// (with groups ringed for a vehicle and no paths it was 12.8–13.7 %).
const MOST_EMPTY_LAWN: f64 = 0.11;
/// At most this share of each region's lawn is empty over the maps whose
/// authored parts stop short of the request's limit (before, 3.7–4.1 %).
const MOST_EMPTY_LAWN_DRESSED: f64 = 0.025;

/// A flag for each point of a map's sampling grid.
struct Raster {
    columns: usize,
    rows: usize,
    marked: Vec<bool>,
}

impl Raster {
    fn new(size: [f64; 2]) -> Self {
        let columns = (size[0] / FILL_STEP_M) as usize + 1;
        let rows = (size[1] / FILL_STEP_M) as usize + 1;
        Raster {
            columns,
            rows,
            marked: vec![false; columns * rows],
        }
    }

    fn cell(&self, column: i64, row: i64) -> Option<usize> {
        (column >= 0 && row >= 0 && (column as usize) < self.columns && (row as usize) < self.rows)
            .then(|| row as usize * self.columns + column as usize)
    }

    fn at(&self, column: i64, row: i64) -> bool {
        self.cell(column, row).is_some_and(|cell| self.marked[cell])
    }

    /// Mark every point within `reach` of the box (centre, yaw, half
    /// extents).
    fn mark_box(&mut self, center: Point, yaw: f64, half: [f64; 2], reach: f64) {
        let outer = half[0].hypot(half[1]) + reach;
        let (sin, cos) = yaw.sin_cos();
        let span = |v: f64| {
            (((v - outer) / FILL_STEP_M).ceil() as i64)
                ..=(((v + outer) / FILL_STEP_M).floor() as i64)
        };
        for row in span(center[1]) {
            for column in span(center[0]) {
                let Some(cell) = self.cell(column, row) else {
                    continue;
                };
                let d = [
                    column as f64 * FILL_STEP_M - center[0],
                    row as f64 * FILL_STEP_M - center[1],
                ];
                let local = [d[0] * cos + d[1] * sin, d[1] * cos - d[0] * sin];
                let out = [0, 1].map(|axis| (local[axis].abs() - half[axis]).max(0.0));
                if out[0].hypot(out[1]) <= reach {
                    self.marked[cell] = true;
                }
            }
        }
    }

    /// Mark every point within `reach` of a segment.
    fn mark_segment(&mut self, a: Point, b: Point, reach: f64) {
        let span = |u: f64, v: f64| {
            (((u.min(v) - reach) / FILL_STEP_M).ceil() as i64)
                ..=(((u.max(v) + reach) / FILL_STEP_M).floor() as i64)
        };
        for row in span(a[1], b[1]) {
            for column in span(a[0], b[0]) {
                let Some(cell) = self.cell(column, row) else {
                    continue;
                };
                let p = [column as f64 * FILL_STEP_M, row as f64 * FILL_STEP_M];
                if segment_distance(a, b, p) <= reach {
                    self.marked[cell] = true;
                }
            }
        }
    }

    /// Mark every point inside `ring`, and call `each` with it.
    fn mark_ring(&mut self, ring: &[Point], mut each: impl FnMut(i64, i64)) {
        let [x0, y0, x1, y1] = contract::ground::limits(ring, 0.0);
        for row in (y0 / FILL_STEP_M).ceil() as i64..=(y1 / FILL_STEP_M).floor() as i64 {
            for column in (x0 / FILL_STEP_M).ceil() as i64..=(x1 / FILL_STEP_M).floor() as i64 {
                let p = [column as f64 * FILL_STEP_M, row as f64 * FILL_STEP_M];
                if let Some(cell) = self.cell(column, row) {
                    if polygon_contains(ring, p) {
                        self.marked[cell] = true;
                        each(column, row);
                    }
                }
            }
        }
    }
}

/// Courts are structured ground, not empty plazas. On every type and size
/// of map in every region, at three seeds each, the court paving of the
/// dense districts is sampled on a 2 m grid, and a point is open when it lies
/// farther than `OPEN_M` from every body (a boundary's included), building
/// wall, carriageway and lawn (a dense district's ground under no court
/// paving and no carriageway). Each region's share of open court paving
/// stays under `MOST_OPEN`, and on the maps the part limit does not bind,
/// each district kind's under `MOST_OPEN_DRESSED`: at the limit the courts
/// are dressed thinly, the apartments' wide yards the most open. And the
/// lawn is dressed, not a bare field: each region's share of lawn points
/// farther than `EMPTY_LAWN_M` from every body, building wall, carriageway
/// and paved area (a path's included) stays under `MOST_EMPTY_LAWN`, and
/// under `MOST_EMPTY_LAWN_DRESSED` on the maps the part limit does not bind.
#[test]
fn courts_are_structured_and_lawns_dressed_rather_than_left_open() {
    // (region, district kind, whether the part limit binds) → (open
    // points, points).
    let mut tally: BTreeMap<(&str, String, bool), [usize; 2]> = BTreeMap::new();
    // (region, whether the part limit binds) → (empty lawn points, lawn
    // points).
    let mut lawns: BTreeMap<(&str, bool), [usize; 2]> = BTreeMap::new();
    for region in REGIONS {
        for map_type in MapType::ALL {
            for size in MapSize::ALL {
                for seed in FILL_SEEDS {
                    let mut request = request(map_type, size, seed);
                    request.region = Some(region.into());
                    let (plan, result) = mapgen::generate_with_plan(
                        &serde_json::to_string(&request).unwrap(),
                        PRESETS,
                        TEMPLATES,
                        &sim::fixtures::game().to_string(),
                    )
                    .unwrap_or_else(|failure| panic!("{:?}", failure.diagnostics));
                    let mut near = Raster::new(plan.size);
                    let mut road = Raster::new(plan.size);
                    for prop in &plan.props {
                        let p = &prop.geometry;
                        let half = [p.half_extents[0], p.half_extents[1]];
                        near.mark_box(p.center, p.yaw, half, OPEN_M);
                    }
                    for part in result.map.buildings.iter().flat_map(|b| &b.geometry.parts) {
                        let half = [part.half_extents[0], part.half_extents[1]];
                        near.mark_box(part.center, part.yaw, half, OPEN_M);
                    }
                    for area in plan.surfaces.iter().filter(|area| area.kind.is_road()) {
                        if let GroundShape::Stroke {
                            centerline,
                            width_m,
                        } = &area.shape
                        {
                            for pair in centerline.samples().windows(2) {
                                near.mark_segment(pair[0], pair[1], width_m / 2.0 + OPEN_M);
                                road.mark_segment(pair[0], pair[1], width_m / 2.0);
                            }
                        }
                    }
                    let bound = result.report.authored_parts >= request.limits.max_authored_parts;
                    let mut paved = Raster::new(plan.size);
                    let mut cells: Vec<(String, i64, i64)> = Vec::new();
                    let kinds = district_kinds(&plan);
                    for court in &plan.courts {
                        let kind = kinds[court.district.as_str()];
                        // A path is paving the lawn is judged by, not a court.
                        let path = matches!(
                            court.kind,
                            mapgen::CourtKind::Path | mapgen::CourtKind::Lane
                        );
                        paved.mark_ring(&court.ring, |column, row| {
                            if !path {
                                cells.push((kind.to_string(), column, row))
                            }
                        });
                    }
                    let mut dense = Raster::new(plan.size);
                    for district in plan.settlements.iter().flat_map(|s| &s.districts) {
                        if COURT_DISTRICTS.contains(&district.kind.as_str()) {
                            dense.mark_ring(&district.ring, |_, _| ());
                        }
                    }
                    let lawn = |column: i64, row: i64| {
                        dense.at(column, row) && !paved.at(column, row) && !road.at(column, row)
                    };
                    // The lawn: empty where farther than `EMPTY_LAWN_M` from
                    // every body, building wall, carriageway and paved area
                    // (a court, a path, an apron).
                    let mut filled = Raster::new(plan.size);
                    for prop in &plan.props {
                        let p = &prop.geometry;
                        let half = [p.half_extents[0], p.half_extents[1]];
                        filled.mark_box(p.center, p.yaw, half, EMPTY_LAWN_M);
                    }
                    for part in result.map.buildings.iter().flat_map(|b| &b.geometry.parts) {
                        let half = [part.half_extents[0], part.half_extents[1]];
                        filled.mark_box(part.center, part.yaw, half, EMPTY_LAWN_M);
                    }
                    for area in &plan.surfaces {
                        match &area.shape {
                            GroundShape::Stroke {
                                centerline,
                                width_m,
                            } if area.kind.is_road() => {
                                for pair in centerline.samples().windows(2) {
                                    filled.mark_segment(
                                        pair[0],
                                        pair[1],
                                        width_m / 2.0 + EMPTY_LAWN_M,
                                    );
                                }
                            }
                            GroundShape::Polygon { ring } if area.kind == SurfaceKind::Paving => {
                                for (a, b) in contract::ground::edges(ring) {
                                    filled.mark_segment(*a, *b, EMPTY_LAWN_M);
                                }
                            }
                            _ => {}
                        }
                    }
                    for district in plan.settlements.iter().flat_map(|s| &s.districts) {
                        if !COURT_DISTRICTS.contains(&district.kind.as_str()) {
                            continue;
                        }
                        let entry = lawns.entry((region, bound)).or_insert([0, 0]);
                        let [x0, y0, x1, y1] = contract::ground::limits(&district.ring, 0.0);
                        for row in (y0 / FILL_STEP_M).ceil() as i64..=(y1 / FILL_STEP_M) as i64 {
                            for column in
                                (x0 / FILL_STEP_M).ceil() as i64..=(x1 / FILL_STEP_M) as i64
                            {
                                let p = [column as f64 * FILL_STEP_M, row as f64 * FILL_STEP_M];
                                if !polygon_contains(&district.ring, p) || !lawn(column, row) {
                                    continue;
                                }
                                entry[1] += 1;
                                if !filled.at(column, row) {
                                    entry[0] += 1;
                                }
                            }
                        }
                    }
                    let reach = (OPEN_M / FILL_STEP_M) as i64;
                    cells.sort();
                    cells.dedup();
                    for (kind, column, row) in cells {
                        let entry = tally.entry((region, kind, bound)).or_insert([0, 0]);
                        entry[1] += 1;
                        if near.at(column, row) {
                            continue;
                        }
                        let by_lawn = (-reach..=reach).any(|dy| {
                            (-reach..=reach).any(|dx| {
                                (dx * dx + dy * dy) as f64 * FILL_STEP_M * FILL_STEP_M
                                    <= OPEN_M * OPEN_M
                                    && lawn(column + dx, row + dy)
                            })
                        });
                        if !by_lawn {
                            entry[0] += 1;
                        }
                    }
                }
            }
        }
    }
    let mut regions: BTreeMap<&str, [usize; 2]> = BTreeMap::new();
    let mut broken = Vec::new();
    for ((region, kind, bound), [open, all]) in &tally {
        let limited = if *bound {
            "at the part limit"
        } else {
            "short of it"
        };
        let share = *open as f64 / *all as f64;
        println!(
            "{region} {kind}, {limited}: {:.2} % of {all} court points open",
            100.0 * share
        );
        if !bound && share >= MOST_OPEN_DRESSED {
            broken.push(format!(
                "{region} {kind}: {:.2} % of {all} points",
                100.0 * share
            ));
        }
        let total = regions.entry(region).or_insert([0, 0]);
        total[0] += open;
        total[1] += all;
    }
    for (region, [open, all]) in &regions {
        let share = *open as f64 / *all as f64;
        println!(
            "{region}: {:.2} % of {all} court points open",
            100.0 * share
        );
        if share >= MOST_OPEN {
            broken.push(format!("{region}: {:.2} % of {all} points", 100.0 * share));
        }
    }
    let mut lawn_regions: BTreeMap<&str, [usize; 2]> = BTreeMap::new();
    for ((region, bound), [empty, all]) in &lawns {
        let limited = if *bound {
            "at the part limit"
        } else {
            "short of it"
        };
        let share = *empty as f64 / *all as f64;
        println!(
            "{region} lawn, {limited}: {:.2} % of {all} lawn points empty",
            100.0 * share
        );
        if !bound && share >= MOST_EMPTY_LAWN_DRESSED {
            broken.push(format!(
                "{region} lawn short of the limit: {:.2} % of {all} points empty",
                100.0 * share
            ));
        }
        let total = lawn_regions.entry(region).or_insert([0, 0]);
        total[0] += empty;
        total[1] += all;
    }
    for (region, [empty, all]) in &lawn_regions {
        let share = *empty as f64 / *all as f64;
        println!(
            "{region}: {:.2} % of {all} lawn points empty",
            100.0 * share
        );
        if share >= MOST_EMPTY_LAWN {
            broken.push(format!(
                "{region}: {:.2} % of {all} lawn points empty",
                100.0 * share
            ));
        }
    }
    assert!(
        broken.is_empty(),
        "too much open court paving or empty lawn: {broken:?}"
    );
}

/// The yard and car park rows these tests state, whatever the shipped
/// presets are tuned to: each region its own boundary kind.
const YARD_GATE_M: f64 = 4.0;
const VEHICLE_GATE_M: f64 = 8.0;
const BOUNDARIES: [(&str, &str); 3] = [
    ("china", "courtyard_wall"),
    ("new_york", "iron_railing"),
    ("paris", "plinth_railing"),
];
const BAY_M: [f64; 2] = [2.6, 5.2];
const AISLE_M: f64 = 6.5;

fn yards_and_parking(source: &mut Value) {
    for kind in COURT_DISTRICTS {
        let courts = &mut source["districts"][kind]["props"]["courts"];
        courts["yards"] = json!({
            "boundary": BOUNDARIES
                .iter()
                .map(|(family, kind)| (family.to_string(), json!({ *kind: 1 })))
                .collect::<serde_json::Map<String, Value>>(),
            "gate_m": VEHICLE_GATE_M,
            "door_gate_m": YARD_GATE_M,
            "front_inset_m": 3,
        });
        courts["parking"] =
            json!({ "bay_m": BAY_M, "aisle_m": AISLE_M, "bays": [3, 8], "every_m": 40 });
    }
}

/// Whether `p` lies within the strip from `a` to `b`, `half` either side of
/// its line.
fn in_strip(a: Point, b: Point, half: f64, p: Point) -> bool {
    let length = span(a, b);
    let along = [(b[0] - a[0]) / length, (b[1] - a[1]) / length];
    let d = [p[0] - a[0], p[1] - a[1]];
    let s = d[0] * along[0] + d[1] * along[1];
    let t = d[1] * along[0] - d[0] * along[1];
    (0.0..=length).contains(&s) && t.abs() <= half
}

/// Each yard is bounded, with a gate before each door and a way in for a
/// vehicle where it has room, and each car park keeps its aisle. On a town
/// of each region: the boundary of a yard is its region's kind, standing
/// along its parcel's edges; together the yards' sides and rears are
/// covered along most of their length bar their gates (a side the building
/// stands too near for a squad to pass is the building's own); no body
/// stands in the way out from any door of a dense district's building
/// through its yard's boundary, a gate's width wide; a yard whose building
/// leaves a vehicle's way to an edge of its parcel has a gate a vehicle
/// drives through in the middle of an edge; and a car park's cars stand in
/// its two rows of bays, its aisle open from its mouth on the street to its
/// far end.
#[test]
fn yards_are_bounded_with_gates_and_car_parks_keep_their_aisles() {
    let presets = presets_with(yards_and_parking);
    let hull_way = widest_hull(&rules().catalog) + presets.street_props.group_margin_m;
    let squad_way = presets.street_props.squad_way_m;
    for (region, kind) in BOUNDARIES {
        let mut request = request(MapType::Mixed, MapSize::Small, 1);
        request.region = Some(region.into());
        let plan = fill_districts(
            generate_layout(&request, &presets).unwrap(),
            &request,
            &catalogue(),
            &presets,
        )
        .unwrap();
        let props: Vec<PropDefinition> = dressed(&plan, &request, &presets, &rules())
            .into_iter()
            .map(|prop| prop.geometry)
            .collect();
        let map = compiled(&plan, &[], &request);
        let boundaries = ["courtyard_wall", "iron_railing", "plinth_railing"];
        for prop in props
            .iter()
            .filter(|p| boundaries.contains(&p.kind.as_str()))
        {
            assert_eq!(
                prop.kind, kind,
                "{region}: a {} at {:?}",
                prop.kind, prop.center
            );
        }
        let panels: Vec<&PropDefinition> = props.iter().filter(|p| p.kind == kind).collect();
        let buildings: BTreeMap<&str, usize> = plan
            .buildings
            .iter()
            .enumerate()
            .map(|(index, building)| (building.id.as_str(), index))
            .collect();
        let (mut covered, mut length, mut doors, mut cars) = (0.0, 0.0, 0, 0);
        let (mut roomy, mut driven_in) = (0, 0);
        for court in &plan.courts {
            let frame = LotFrame::new(&court.ring);
            match court.kind {
                mapgen::CourtKind::Yard => {
                    // Its sides and rear, each covered where a panel along
                    // it stands within a panel's thickness of it, its own
                    // or its neighbour's; a side its building stands too
                    // near for a squad to pass between is the building's.
                    let lot = court.id.strip_suffix("/yard").unwrap();
                    let building = &map.buildings[buildings[lot]];
                    let parts: Vec<Point> = building
                        .geometry
                        .parts
                        .iter()
                        .flat_map(|part| {
                            let (sin, cos) = part.yaw.sin_cos();
                            [[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]].map(|[u, v]| {
                                let (x, y) = (u * part.half_extents[0], v * part.half_extents[1]);
                                [
                                    part.center[0] + cos * x - sin * y,
                                    part.center[1] + sin * x + cos * y,
                                ]
                            })
                        })
                        .collect();
                    let entrances: Vec<(Point, Point)> = building
                        .geometry
                        .entrances
                        .iter()
                        .flatten()
                        .map(|door| ([door.position[0], door.position[1]], door.normal))
                        .collect();
                    for (a, b) in [
                        (court.ring[1], court.ring[2]),
                        (court.ring[2], court.ring[3]),
                        (court.ring[3], court.ring[0]),
                    ] {
                        let width = span(a, b);
                        let along = [(b[0] - a[0]) / width, (b[1] - a[1]) / width];
                        // Along the edge, and off its line.
                        let local = |p: Point| {
                            let d = [p[0] - a[0], p[1] - a[1]];
                            [
                                d[0] * along[0] + d[1] * along[1],
                                d[1] * along[0] - d[0] * along[1],
                            ]
                        };
                        let near = parts
                            .iter()
                            .map(|p| local(*p)[1].abs())
                            .fold(f64::INFINITY, f64::min);
                        if near < squad_way + 0.5 {
                            continue;
                        }
                        let mut spans: Vec<[f64; 2]> = panels
                            .iter()
                            .filter(|p| {
                                local(p.center)[1].abs() <= 2.0 * p.half_extents[1] + 0.1
                                    && same_heading(p.yaw, along[1].atan2(along[0]))
                            })
                            .map(|p| {
                                let s = local(p.center)[0];
                                [s - p.half_extents[0], s + p.half_extents[0]]
                            })
                            .collect();
                        spans.sort_by(|x, y| x[0].total_cmp(&y[0]));
                        let mut reach = 0.0_f64;
                        for [from, to] in spans {
                            let (from, to) = (from.max(reach).max(0.0), to.min(width));
                            if to > from {
                                covered += to - from;
                                reach = to;
                            }
                        }
                        // Its openings: its gate, and the way out from each
                        // door that faces it.
                        let facing = entrances
                            .iter()
                            .filter(|(_, out)| out[1] * along[0] - out[0] * along[1] < -0.5)
                            .count() as f64;
                        let gates =
                            VEHICLE_GATE_M + facing * 2.0 * presets.street_props.door_clear_m;
                        length += (width - gates).max(0.0);
                    }
                    // A gate a vehicle drives through, in the middle of an
                    // edge the building leaves a vehicle's way to: from the
                    // edge in past where the boundary stands.
                    let edges = [0, 1, 2, 3].map(|k| (court.ring[k], court.ring[(k + 1) % 4]));
                    let room = |(a, b): (Point, Point)| {
                        let width = span(a, b);
                        let along = [(b[0] - a[0]) / width, (b[1] - a[1]) / width];
                        parts
                            .iter()
                            .map(|p| ((p[1] - a[1]) * along[0] - (p[0] - a[0]) * along[1]).abs())
                            .fold(f64::INFINITY, f64::min)
                    };
                    if edges.iter().any(|edge| room(*edge) >= hull_way + 1.0) {
                        roomy += 1;
                        let open = edges.iter().any(|&(a, b)| {
                            let width = span(a, b);
                            let inward = [-(b[1] - a[1]) / width, (b[0] - a[0]) / width];
                            let middle = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
                            let inside = [middle[0] + inward[0] * 4.0, middle[1] + inward[1] * 4.0];
                            room((a, b)) >= hull_way
                                && panels.iter().all(|prop| {
                                    corners(prop).iter().chain([&prop.center]).all(|p| {
                                        !in_strip(middle, inside, VEHICLE_GATE_M / 2.0 - 0.1, *p)
                                    })
                                })
                        });
                        if open {
                            driven_in += 1;
                        }
                    }
                    // Out from each door through the yard's boundary.
                    for door in building.geometry.entrances.iter().flatten() {
                        let from = [door.position[0], door.position[1]];
                        let out = door.normal;
                        let edge = (1..80)
                            .map(|k| [from[0] + out[0] * k as f64, from[1] + out[1] * k as f64])
                            .find(|p| !polygon_contains(&court.ring, *p))
                            .unwrap();
                        let to = [edge[0] + out[0], edge[1] + out[1]];
                        for prop in &props {
                            assert!(
                                corners(prop).iter().chain([&prop.center]).all(|p| {
                                    !in_strip(from, to, YARD_GATE_M / 2.0 - 0.1, *p)
                                }),
                                "{region}: a {} at {:?} stands in the way out of {lot}'s door at {from:?}",
                                prop.kind,
                                prop.center
                            );
                        }
                        doors += 1;
                    }
                }
                mapgen::CourtKind::Parking => {
                    let mouth = frame.at(frame.width / 2.0, -presets.parcels.verge_m);
                    let end = frame.at(frame.width / 2.0, frame.depth);
                    for prop in &props {
                        assert!(
                            corners(prop)
                                .iter()
                                .chain([&prop.center])
                                .all(|p| !in_strip(mouth, end, AISLE_M / 2.0 - 0.05, *p)),
                            "{region}: a {} at {:?} stands in the aisle of {}",
                            prop.kind,
                            prop.center,
                            court.id
                        );
                    }
                    let parked: Vec<&PropDefinition> = props
                        .iter()
                        .filter(|p| {
                            p.kind == "parked_car" && polygon_contains(&court.ring, p.center)
                        })
                        .collect();
                    for car in &parked {
                        let local = frame.local(car.center);
                        let row = local[0].min(frame.width - local[0]);
                        assert!(
                            (row - BAY_M[1] / 2.0).abs() < 0.05,
                            "{region}: a car at {:?} stands out of {}'s rows",
                            car.center,
                            court.id
                        );
                    }
                    cars += parked.len();
                }
                mapgen::CourtKind::Path | mapgen::CourtKind::Lane => {}
            }
        }
        let share = covered / length;
        println!("{region}: {:.0} % of {length:.0} m of yard sides and rears bounded; {doors} doors; {cars} cars in car parks", 100.0 * share);
        assert!(
            share >= 0.8,
            "{region}: {:.0} % of yard sides and rears bounded",
            100.0 * share
        );
        println!("{region}: {driven_in} of {roomy} yards with a vehicle's room have its gate");
        assert!(
            roomy >= 50 && driven_in == roomy,
            "{region}: {driven_in} of {roomy} yards with a vehicle's room have its gate"
        );
        assert!(
            doors >= 500 && cars >= 200,
            "{region}: {doors} doors, {cars} cars"
        );
    }
}

/// Yards share their boundaries. On a New York town whose yards are bounded
/// by iron railing or chain-link, its courts holding no group: every
/// district's boundary is of one kind, and no boundary panel runs beside
/// another, the way it runs and nearer than a squad's way, along the same
/// stretch: where two yards meet, or face each other across a gap a squad
/// cannot use, one boundary stands.
#[test]
fn neighbouring_yards_share_one_boundary_of_one_kind() {
    let kinds = ["iron_railing", "chainlink_fence"];
    let presets = presets_with(|source| {
        yards_and_parking(source);
        for district in COURT_DISTRICTS {
            let courts = &mut source["districts"][district]["props"]["courts"];
            courts["yards"]["boundary"]["new_york"] = json!({ kinds[0]: 1, kinds[1]: 1 });
            courts["groups"] = json!({});
            courts["families"] = json!({});
        }
    });
    let squad_way = presets.street_props.squad_way_m;
    let mut request = request(MapType::Mixed, MapSize::Small, 1);
    request.region = Some("new_york".into());
    let plan = fill_districts(
        generate_layout(&request, &presets).unwrap(),
        &request,
        &catalogue(),
        &presets,
    )
    .unwrap();
    let props: Vec<PropDefinition> = dressed(&plan, &request, &presets, &rules())
        .into_iter()
        .map(|prop| prop.geometry)
        .filter(|prop| kinds.contains(&prop.kind.as_str()))
        .collect();
    let mut found = std::collections::BTreeSet::new();
    for district in plan.settlements.iter().flat_map(|s| &s.districts) {
        let own: std::collections::BTreeSet<&str> = props
            .iter()
            .filter(|p| polygon_contains(&district.ring, p.center))
            .map(|p| p.kind.as_str())
            .collect();
        assert!(own.len() <= 1, "{}: boundaries of {own:?}", district.id);
        found.extend(own);
    }
    assert_eq!(found.len(), 2, "both kinds stand somewhere: {found:?}");
    let mut beside = Vec::new();
    for (i, a) in props.iter().enumerate() {
        let (sin, cos) = a.yaw.sin_cos();
        for b in &props[i + 1..] {
            if (a.yaw - b.yaw).sin().abs() > 0.3 {
                continue;
            }
            let d = [b.center[0] - a.center[0], b.center[1] - a.center[1]];
            let (along, across) = (d[0] * cos + d[1] * sin, d[1] * cos - d[0] * sin);
            let overlap = a.half_extents[0] + b.half_extents[0] - along.abs();
            let gap = across.abs() - a.half_extents[1] - b.half_extents[1];
            if overlap > 0.5 && across.abs() > 0.05 && gap < squad_way - 0.1 {
                beside.push((a.center, b.center, gap));
            }
        }
    }
    println!("{} panels, {} side by side", props.len(), beside.len());
    assert!(
        props.len() >= 500 && beside.is_empty(),
        "{} panels; side by side: {:?}",
        props.len(),
        &beside[..beside.len().min(10)]
    );
}

/// The lawn rows these tests state, whatever the shipped presets are tuned
/// to: paths 3 m wide, the lane 8 m, trees planted beside.
const PATH_M: f64 = 3.0;

fn lawn_paths(source: &mut Value) {
    yards_and_parking(source);
    for kind in COURT_DISTRICTS {
        source["districts"][kind]["props"]["courts"]["lawn"] = json!({
            "path_m": PATH_M, "lane_m": 8, "least_m2": 300, "streets": 3, "gates": 6,
            "beside": { "court_tree": 1 }, "beside_spacing_m": 9,
        });
    }
}

/// Lawns are crossed by paved paths, kept clear, one of each lawn's from a
/// street a lane a vehicle drives. On a town of each region: every path is
/// paved, as wide as the rows say; it starts at a street (across its walk to
/// the carriageway's edge) or in a yard's gate (the middle of its rear edge),
/// and past its first metres runs over no yard or car park (an open
/// parcel is lawn); no body
/// stands on a path, nor within a vehicle's way of a lane's middle; trees
/// are planted beside the paths; and the paths are many, with lanes among
/// them.
#[test]
fn lawns_are_crossed_by_paths_kept_clear() {
    let presets = presets_with(lawn_paths);
    let hull_way = widest_hull(&rules().catalog) + presets.street_props.group_margin_m;
    for region in REGIONS {
        let mut request = request(MapType::Mixed, MapSize::Small, 1);
        request.region = Some(region.into());
        let plan = fill_districts(
            generate_layout(&request, &presets).unwrap(),
            &request,
            &catalogue(),
            &presets,
        )
        .unwrap();
        let props: Vec<PropDefinition> = dressed(&plan, &request, &presets, &rules())
            .into_iter()
            .map(|prop| prop.geometry)
            .collect();
        let roads = Roads::new(&plan);
        let paved: Vec<&Vec<Point>> = plan
            .surfaces
            .iter()
            .filter(|area| area.kind == SurfaceKind::Paving)
            .filter_map(|area| match &area.shape {
                GroundShape::Polygon { ring } => Some(ring),
                GroundShape::Stroke { .. } => None,
            })
            .collect();
        let courts: Vec<&[Point]> = plan
            .courts
            .iter()
            .filter(|c| matches!(c.kind, mapgen::CourtKind::Yard | mapgen::CourtKind::Parking))
            .map(|c| c.ring.as_slice())
            .collect();
        let rears: Vec<Point> = plan
            .courts
            .iter()
            .filter(|c| c.kind == mapgen::CourtKind::Yard)
            .map(|c| {
                [
                    (c.ring[2][0] + c.ring[3][0]) / 2.0,
                    (c.ring[2][1] + c.ring[3][1]) / 2.0,
                ]
            })
            .collect();
        let (mut paths, mut lanes, mut planted) = (0, 0, 0);
        for path in plan
            .courts
            .iter()
            .filter(|c| matches!(c.kind, mapgen::CourtKind::Path | mapgen::CourtKind::Lane))
        {
            let id = &path.id;
            let frame = LotFrame::new(&path.ring);
            assert!(
                paved.iter().any(|ring| **ring == path.ring),
                "{region}: {id} is not paved"
            );
            assert!(
                (frame.width - PATH_M).abs() < 0.05,
                "{region}: {id} is {:.2} m wide",
                frame.width
            );
            let start = frame.at(frame.width / 2.0, 0.0);
            let from_street = roads.clearance(start, 1.0) <= 0.5;
            let from_gate = rears.iter().any(|rear| span(*rear, start) <= 1.5);
            assert!(
                from_street || from_gate,
                "{region}: {id} starts at {start:?}, at no street or gate"
            );
            let mut y = 3.0;
            while y < frame.depth {
                let p = frame.at(frame.width / 2.0, y);
                assert!(
                    !courts.iter().any(|ring| polygon_contains(ring, p)),
                    "{region}: {id} runs over a yard or car park at {p:?}"
                );
                y += 1.0;
            }
            let lane = path.kind == mapgen::CourtKind::Lane;
            let (a, b) = (start, frame.at(frame.width / 2.0, frame.depth));
            let half = if lane { hull_way / 2.0 } else { PATH_M / 2.0 } - 0.05;
            for prop in &props {
                assert!(
                    corners(prop)
                        .iter()
                        .chain([&prop.center])
                        .all(|p| !in_strip(a, b, half, *p)),
                    "{region}: a {} at {:?} stands on {id}",
                    prop.kind,
                    prop.center
                );
            }
            planted += props
                .iter()
                .filter(|p| {
                    p.kind == "street_tree" && in_strip(a, b, hull_way / 2.0 + 3.0, p.center)
                })
                .count();
            paths += 1;
            lanes += lane as usize;
        }
        println!("{region}: {paths} paths, {lanes} lanes, {planted} trees beside them");
        assert!(
            paths >= 50 && lanes >= 5 && planted >= paths,
            "{region}: {paths} paths, {lanes} lanes, {planted} trees beside them"
        );
    }
}

/// The same request places the same bodies, courts and gardens included,
/// byte for byte; another seed places others.
#[test]
fn placement_is_deterministic() {
    let presets = presets();
    let rules = rules();
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
        let props = dressed(&plan, &request, &presets, &rules);
        assert!(props.iter().all(|prop| prop.id.is_none()));
        serde_json::to_string(&props).unwrap()
    };
    let first = bytes(1);
    assert!(first.len() > 10_000, "a town with next to no furniture");
    for kind in GARDEN_PIECES.iter().chain(&GARDEN_BOUNDARY) {
        assert!(first.contains(&format!("\"{kind}\"")), "no {kind} placed");
    }
    assert!(
        court_only_kinds(&presets)
            .iter()
            .any(|kind| first.contains(&format!("\"{kind}\""))),
        "no court amenity placed"
    );
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
/// - a squad still walks from there to the back garden of every dressed lot
///   it could walk to before;
/// - a squad still walks to every yard's front gate and into it through its
///   gates, to every car park's aisle and over the lawn, wherever it could
///   before;
/// - the widest hull still has a way in from a street to every court: to
///   every yard's front gate, every car park's aisle, and some point of each
///   lawn large enough for a path deeper than a hull's way from every
///   carriageway, wherever it had one before (a court's inside is mostly infantry ground: the hull is
///   owed a way in, not a way everywhere);
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
    let hull_way = widest_hull(&rules.catalog) + presets.street_props.group_margin_m;
    let mut totals = [0usize; 8];
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
                let props: Vec<PropDefinition> = dressed(&bare, &request, &presets, &rules)
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

                // Gardens: the back of a dressed lot, as far behind its house
                // as half its rear setback, on open ground.
                let kinds = district_kinds(&bare);
                let built: std::collections::BTreeSet<&str> =
                    bare.buildings.iter().map(|b| b.id.as_str()).collect();
                let gardens: Vec<Point> = bare
                    .lots
                    .iter()
                    .filter(|lot| built.contains(lot.id.as_str()))
                    .filter_map(|lot| {
                        let (district, _) = lot.id.rsplit_once("/lot-")?;
                        let preset = &presets.districts[kinds[district]];
                        preset.props.gardens.as_ref()?;
                        let frame = LotFrame::new(&lot.ring);
                        let y = frame.depth - preset.lots.rear_m / 2.0;
                        [0.5, 0.3, 0.7, 0.15, 0.85]
                            .map(|share| frame.at(share * frame.width, y))
                            .into_iter()
                            .find(|p| {
                                !props.iter().any(|prop| {
                                    let (sin, cos) = prop.yaw.sin_cos();
                                    let d = [p[0] - prop.center[0], p[1] - prop.center[1]];
                                    (d[0] * cos + d[1] * sin).abs() < prop.half_extents[0] + 1.0
                                        && (d[1] * cos - d[0] * sin).abs()
                                            < prop.half_extents[1] + 1.0
                                })
                            })
                    })
                    .collect();
                let every = (gardens.len() / GARDENS_WALKED).max(1);
                for goal in gardens.iter().step_by(every) {
                    let Some(centre) = sites
                        .settlements
                        .iter()
                        .find(|s| polygon_contains(&s.outline, *goal))
                        .map(|s| s.center)
                    else {
                        continue;
                    };
                    let arrives = |prepared: &PreparedMap| {
                        route(prepared, &rules, &on_foot, centre, *goal).is_some_and(
                            |(points, _)| {
                                points
                                    .last()
                                    .is_some_and(|end| (*end - v2(goal[0], goal[1])).length() < 3.0)
                            },
                        )
                    };
                    if arrives(&before) {
                        totals[5] += 1;
                        claim!(
                            broken,
                            arrives(&after),
                            "{name}: no way on foot from {centre:?} to the garden at {goal:?}"
                        );
                    }
                }

                // Courts: inside each yard's rear gate and over the lawn (on
                // foot), outside each yard's front gate and each car park's
                // aisle (on foot and by the widest hull), from the
                // settlement's centre, wherever the mover went on the bare
                // map.
                let mut goals: Vec<(Point, bool)> = Vec::new();
                let doors: BTreeMap<&str, Vec<(Point, Point)>> = bare
                    .buildings
                    .iter()
                    .zip(&maps[0].buildings)
                    .map(|(placed, building)| {
                        let doors = building
                            .geometry
                            .entrances
                            .iter()
                            .flatten()
                            .map(|door| ([door.position[0], door.position[1]], door.normal))
                            .collect();
                        (placed.id.as_str(), doors)
                    })
                    .collect();
                let clear_of_bodies = |p: Point| {
                    !props.iter().any(|prop| {
                        let (sin, cos) = prop.yaw.sin_cos();
                        let d = [p[0] - prop.center[0], p[1] - prop.center[1]];
                        (d[0] * cos + d[1] * sin).abs() < prop.half_extents[0] + 1.0
                            && (d[1] * cos - d[0] * sin).abs() < prop.half_extents[1] + 1.0
                    })
                };
                for court in &bare.courts {
                    let frame = LotFrame::new(&court.ring);
                    match court.kind {
                        mapgen::CourtKind::Yard => {
                            goals.push((frame.at(frame.width / 2.0, frame.depth - 2.5), false));
                            let lot = court.id.strip_suffix("/yard").unwrap();
                            if let Some(&(door, out)) = doors.get(lot).and_then(|d| d.first()) {
                                // Out along the door's line to past the
                                // parcel's street edge.
                                let front = (0..60)
                                    .map(|k| {
                                        [door[0] + out[0] * k as f64, door[1] + out[1] * k as f64]
                                    })
                                    .find(|p| !polygon_contains(&court.ring, *p));
                                if let Some(front) = front {
                                    let gate = [front[0] + out[0] * 2.0, front[1] + out[1] * 2.0];
                                    goals.push((gate, true));
                                }
                            }
                        }
                        mapgen::CourtKind::Parking => {
                            goals.push((frame.at(frame.width / 2.0, frame.depth / 2.0), true));
                        }
                        // Lawn, as far as a mover is concerned.
                        mapgen::CourtKind::Path | mapgen::CourtKind::Lane => {}
                    }
                }
                let lawn_rings: Vec<&[Point]> = bare
                    .lots
                    .iter()
                    .map(|lot| lot.ring.as_slice())
                    .chain(
                        bare.courts
                            .iter()
                            .filter(|court| {
                                !matches!(
                                    court.kind,
                                    mapgen::CourtKind::Path | mapgen::CourtKind::Lane
                                )
                            })
                            .map(|court| court.ring.as_slice()),
                    )
                    .collect();
                // Each lawn: the points of a grid over a dense district off
                // its parcels, courts and carriageways, joined to their
                // neighbours on the grid.
                let mut lawns: Vec<Vec<Point>> = Vec::new();
                for district in bare.settlements.iter().flat_map(|s| &s.districts) {
                    if !COURT_DISTRICTS.contains(&district.kind.as_str()) {
                        continue;
                    }
                    let [x0, y0, x1, y1] = contract::ground::limits(&district.ring, 0.0);
                    let mut points: BTreeMap<(i64, i64), Point> = BTreeMap::new();
                    for row in (y0 / COURT_STEP_M).ceil() as i64..=(y1 / COURT_STEP_M) as i64 {
                        for column in (x0 / COURT_STEP_M).ceil() as i64..=(x1 / COURT_STEP_M) as i64
                        {
                            let p = [column as f64 * COURT_STEP_M, row as f64 * COURT_STEP_M];
                            if polygon_contains(&district.ring, p)
                                && roads.clearance(p, 0.0) > lane
                                && !lawn_rings.iter().any(|ring| polygon_contains(ring, p))
                            {
                                goals.push((p, false));
                                points.insert((column, row), p);
                            }
                        }
                    }
                    let mut seen = std::collections::BTreeSet::new();
                    for start in points.keys().copied().collect::<Vec<_>>() {
                        if !seen.insert(start) {
                            continue;
                        }
                        let (mut lawn, mut open) = (Vec::new(), vec![start]);
                        while let Some((column, row)) = open.pop() {
                            lawn.push(points[&(column, row)]);
                            for next in [
                                (column + 1, row),
                                (column - 1, row),
                                (column, row + 1),
                                (column, row - 1),
                            ] {
                                if points.contains_key(&next) && seen.insert(next) {
                                    open.push(next);
                                }
                            }
                        }
                        // A lawn too small for a path is a gap between
                        // buildings, not a court.
                        let least = presets.districts[&district.kind]
                            .props
                            .courts
                            .lawn
                            .as_ref()
                            .map_or(0.0, |rule| rule.least_m2);
                        if lawn.len() as f64 * COURT_STEP_M * COURT_STEP_M >= least {
                            lawns.push(lawn);
                        }
                    }
                }
                let every = (goals.len() / COURTS_WALKED).max(1);
                for (goal, driven) in goals.iter().step_by(every) {
                    if !clear_of_bodies(*goal) {
                        continue;
                    }
                    let Some(centre) = sites
                        .settlements
                        .iter()
                        .find(|s| polygon_contains(&s.outline, *goal))
                        .map(|s| s.center)
                    else {
                        continue;
                    };
                    let movers: &[(&str, &Mobility, f64)] = if *driven {
                        &[
                            ("squad", &on_foot, 3.0),
                            (vehicles[0].0.as_str(), &vehicles[0].1, 6.0),
                        ]
                    } else {
                        &[("squad", &on_foot, 3.0)]
                    };
                    for (mover, m, near_m) in movers {
                        let arrives = |prepared: &PreparedMap| {
                            route(prepared, &rules, m, centre, *goal).is_some_and(|(points, _)| {
                                points.last().is_some_and(|end| {
                                    (*end - v2(goal[0], goal[1])).length() < *near_m
                                })
                            })
                        };
                        if arrives(&before) {
                            totals[6] += 1;
                            claim!(
                                broken,
                                arrives(&after),
                                "{name}: no way for a {mover} from {centre:?} to the court ground at {goal:?}"
                            );
                        }
                    }
                }

                // A way into each lawn for the widest hull: to some point of
                // it deeper than a hull's way from every carriageway's
                // edge, from the settlement's centre.
                let (hull, hull_m) = (&vehicles[0].0, &vehicles[0].1);
                let every = (lawns.len() / LAWNS_DRIVEN).max(1);
                for lawn in lawns.iter().step_by(every) {
                    let deep: Vec<Point> = lawn
                        .iter()
                        .copied()
                        .filter(|p| roads.clearance(*p, 1.0) >= hull_way)
                        .collect();
                    let Some(centre) = deep.first().and_then(|goal| {
                        sites
                            .settlements
                            .iter()
                            .find(|s| polygon_contains(&s.outline, *goal))
                            .map(|s| s.center)
                    }) else {
                        continue;
                    };
                    let arrives = |prepared: &PreparedMap, goal: Point| {
                        route(prepared, &rules, hull_m, centre, goal).is_some_and(|(points, _)| {
                            points
                                .last()
                                .is_some_and(|end| (*end - v2(goal[0], goal[1])).length() < 6.0)
                        })
                    };
                    // Spread through the lawn: a few asked of the bare map,
                    // more of the dressed one.
                    let spread = |most: usize| -> Vec<Point> {
                        let step = (deep.len() / most).max(1);
                        deep.iter().copied().step_by(step).take(most).collect()
                    };
                    if !spread(LAWN_TRIES).into_iter().any(|p| arrives(&before, p)) {
                        continue;
                    }
                    totals[7] += 1;
                    // Near any of its deep points: a body standing on one
                    // closes no way in.
                    let step = (deep.len() / (3 * LAWN_TRIES)).max(1);
                    claim!(
                        broken,
                        deep.iter()
                            .step_by(step)
                            .take(3 * LAWN_TRIES)
                            .any(|p| arrives(&after, *p)),
                        "{name}: no way in for a {hull} from {centre:?} to the lawn at {:?}",
                        deep[0]
                    );
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
            && totals[4] >= 150
            && totals[5] >= 200
            && totals[6] >= 200
            && totals[7] >= 100,
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
