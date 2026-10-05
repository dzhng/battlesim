//! The parcel pass, judged on the compiled map: what a battle would load.
//! Geometry here is checked with its own arithmetic, not the generator's.
mod common;
use common::overlap_depth;

use contract::ground::{polygon_contains, GroundShape};
use contract::map::{BuildingDefinition, MapDefinition, SurfaceKind};
use contract::templates::{
    BuildingCategory, BuildingTemplateDescriptor, MaterializedPart, TemplateGeometryCatalog,
};
use mapgen::layout::{generate_layout, GenerationRequest, MapSize, MapType, PresetDefinitions};
use mapgen::parcels::fill_districts;
use mapgen::{CompileLimits, Diagnostic, DiagnosticCode, DistrictPlan, MapPlan};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex, OnceLock};

const PRESETS: &str = include_str!("../../../fixtures/map-presets.json");
const TEMPLATES: &str = include_str!("../../../fixtures/prototype-building-templates.json");
const TYPES: [MapType; 3] = [MapType::Open, MapType::Mixed, MapType::Metro];
const SIZES: [MapSize; 3] = [MapSize::Small, MapSize::Medium, MapSize::Large];
/// Every cell runs these seeds: a claim about the pass is a claim about all
/// of them, not one lucky town.
const SEEDS: [u64; 2] = [1, u64::MAX];

type Point = [f64; 2];

fn presets() -> PresetDefinitions {
    PresetDefinitions::from_json(PRESETS).unwrap()
}

fn catalogue() -> TemplateGeometryCatalog {
    TemplateGeometryCatalog::new(serde_json::from_str(TEMPLATES).unwrap()).unwrap()
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

/// The first seed whose map of this type and size has an industrial
/// district: not every map has one.
fn seed_with_industry(map_type: MapType, size: MapSize) -> u64 {
    (1..=40)
        .find(|seed| {
            generate_layout(&request(map_type, size, *seed), &presets()).is_ok_and(|plan| {
                plan.settlements
                    .iter()
                    .flat_map(|settlement| &settlement.districts)
                    .any(|district| district.kind == "industrial")
            })
        })
        .expect("a map with industry among forty seeds")
}

fn fill(
    request: &GenerationRequest,
    presets: &PresetDefinitions,
    catalogue: &TemplateGeometryCatalog,
) -> Result<MapPlan, Vec<Diagnostic>> {
    fill_districts(
        generate_layout(request, presets)?,
        request,
        catalogue,
        presets,
    )
}

/// A generated plan and the map it compiles into.
struct Town {
    plan: MapPlan,
    map: MapDefinition,
}

impl Town {
    fn district_of(&self, building: usize) -> &DistrictPlan {
        let id = &self.plan.buildings[building].id;
        self.plan
            .settlements
            .iter()
            .flat_map(|settlement| &settlement.districts)
            .find(|district| id.starts_with(&format!("{}/", district.id)))
            .unwrap_or_else(|| panic!("{id} names no district"))
    }
}

/// Each cell is generated once, whichever tests ask for it.
fn town(map_type: MapType, size: MapSize, seed: u64) -> Arc<Town> {
    type Towns = BTreeMap<(MapType, MapSize, u64), Arc<Town>>;
    static TOWNS: OnceLock<Mutex<Towns>> = OnceLock::new();
    let towns = TOWNS.get_or_init(Default::default);
    if let Some(town) = towns.lock().unwrap().get(&(map_type, size, seed)) {
        return town.clone();
    }
    let request = request(map_type, size, seed);
    let plan = fill(&request, &presets(), &catalogue())
        .unwrap_or_else(|errors| panic!("{map_type:?} {size:?} seed {seed}: {errors:?}"));
    let compiled = mapgen::lower(
        &mapgen::CompileRequest::generated(&request, plan.clone()),
        &catalogue(),
    )
    .unwrap_or_else(|errors| panic!("{map_type:?} {size:?} seed {seed}: {errors:?}"));
    let town = Arc::new(Town {
        plan,
        map: compiled.map,
    });
    towns
        .lock()
        .unwrap()
        .insert((map_type, size, seed), town.clone());
    town
}

fn every_cell(mut check: impl FnMut(&str, MapType, &Town)) {
    for map_type in TYPES {
        for size in SIZES {
            for seed in SEEDS {
                let name = format!("{map_type:?} {size:?} seed {seed}");
                check(&name, map_type, &town(map_type, size, seed));
            }
        }
    }
}

fn corners(part: &MaterializedPart) -> [Point; 4] {
    let (sin, cos) = part.yaw.sin_cos();
    [[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]].map(|[u, v]| {
        let (x, y) = (u * part.half_extents[0], v * part.half_extents[1]);
        [
            part.center[0] + cos * x - sin * y,
            part.center[1] + sin * x + cos * y,
        ]
    })
}

fn segment_gap(a: Point, b: Point, p: Point) -> f64 {
    contract::ground::segment_distance(a, b, p)
}

fn segments_cross(a: [Point; 2], b: [Point; 2]) -> bool {
    let side = |p: Point, q: Point, r: Point| contract::ground::cross(p, q, r);
    side(a[0], a[1], b[0]) * side(a[0], a[1], b[1]) < 0.0
        && side(b[0], b[1], a[0]) * side(b[0], b[1], a[1]) < 0.0
}

/// The gap between a convex ring and a segment; zero when they meet.
fn ring_segment_gap(ring: &[Point], a: Point, b: Point) -> f64 {
    if polygon_contains(ring, a) || polygon_contains(ring, b) {
        return 0.0;
    }
    let mut gap = f64::INFINITY;
    for (p, q) in contract::ground::edges(ring) {
        if segments_cross([*p, *q], [a, b]) {
            return 0.0;
        }
        gap = gap
            .min(segment_gap(a, b, *p))
            .min(segment_gap(*p, *q, a))
            .min(segment_gap(*p, *q, b));
    }
    gap
}

fn floors(building: &BuildingDefinition) -> usize {
    building.geometry.floor_z.as_ref().unwrap().len()
}

/// The accepted building kits can reach 1.5 m beyond their physical parts.
/// That air belongs inside each building's parcel, including when a dense
/// district selects apartments beside terraces rather than houses alone.
#[test]
fn parcels_leave_room_for_facade_art_around_every_building_part() {
    every_cell(|name, _, town| {
        let lots: BTreeMap<_, _> = town
            .plan
            .lots
            .iter()
            .map(|lot| (lot.id.as_str(), lot.ring.as_slice()))
            .collect();
        for (placed, building) in town.plan.buildings.iter().zip(&town.map.buildings) {
            let lot = lots[placed.id.as_str()];
            for part in &building.geometry.parts {
                let mut envelope = part.clone();
                envelope.half_extents[0] += 1.5;
                envelope.half_extents[1] += 1.5;
                for point in corners(&envelope) {
                    // Parcel vertices and placement translations round to cm;
                    // allow their combined rounding, never metres of intrusion.
                    let on_edge = contract::ground::edges(lot)
                        .any(|(a, b)| segment_gap(*a, *b, point) <= 0.02);
                    assert!(
                        polygon_contains(lot, point) || on_edge,
                        "{name}: {} ({}) part {} leaves its parcel's art clearance at {point:?}",
                        placed.id,
                        placed.template_id,
                        part.id
                    );
                }
            }
        }
    });
}

#[test]
fn a_fixed_request_gives_the_same_plan_and_map_bytes_every_run() {
    let (presets, catalogue) = (presets(), catalogue());
    for map_type in TYPES {
        let bytes = |seed: u64| {
            let request = request(map_type, MapSize::Small, seed);
            let plan = fill(&request, &presets, &catalogue).unwrap();
            let map = mapgen::lower(
                &mapgen::CompileRequest::generated(&request, plan.clone()),
                &catalogue,
            )
            .unwrap();
            (
                serde_json::to_string(&plan).unwrap(),
                serde_json::to_string(&map.map).unwrap(),
                map.identity.map_hash,
            )
        };
        assert_eq!(bytes(7), bytes(7));
        assert_ne!(bytes(7).2, bytes(8).2, "the seed must move the town");
    }
}

/// A building's id names its district and its parcel, no two share one, and
/// its parts take the map's prop ids in order with none left over.
#[test]
fn buildings_are_named_for_their_districts_and_fill_the_prop_ids() {
    every_cell(|name, _, town| {
        assert!(!town.plan.buildings.is_empty(), "{name}");
        let lots: BTreeSet<&str> = town.plan.lots.iter().map(|lot| lot.id.as_str()).collect();
        assert_eq!(
            lots.len(),
            town.plan.lots.len(),
            "{name}: a parcel id repeats"
        );
        let mut next = 0;
        for (index, building) in town.plan.buildings.iter().enumerate() {
            let district = town.district_of(index);
            assert!(building.id.starts_with(&format!("{}/lot-", district.id)));
            assert!(
                lots.contains(building.id.as_str()),
                "{name}: {}",
                building.id
            );
            assert_eq!(building.owner, next, "{name}: {}", building.id);
            for part in &building.parts {
                assert_eq!(part.prop, next, "{name}: {}", building.id);
                next += 1;
            }
        }
        let props = town.map.authored_props().unwrap();
        assert_eq!(props.len(), next as usize, "{name}");
        // Every district of every settlement is built on.
        let built: BTreeSet<&str> = (0..town.plan.buildings.len())
            .map(|index| town.district_of(index).id.as_str())
            .collect();
        for district in town.plan.settlements.iter().flat_map(|s| &s.districts) {
            assert!(
                built.contains(district.id.as_str()),
                "{name}: {} is empty",
                district.id
            );
        }
    });
}

/// Placed geometry is the descriptor's own, moved and turned: nothing is
/// stretched to fill a parcel.
#[test]
fn a_placed_template_is_its_descriptor_under_a_rigid_transform() {
    let catalogue = catalogue();
    let by_id: BTreeMap<&str, &BuildingTemplateDescriptor> = catalogue
        .templates()
        .iter()
        .map(|template| (template.id.as_str(), template))
        .collect();
    every_cell(|name, _, town| {
        for building in &town.map.buildings {
            let template = by_id[building.geometry.template_id.as_str()];
            let frame = building.geometry.frame;
            let (sin, cos) = frame.yaw.sin_cos();
            assert_eq!(building.geometry.height_m, template.height_m());
            assert_eq!(building.geometry.parts.len(), template.parts.len());
            for (placed, source) in building.geometry.parts.iter().zip(&template.parts) {
                assert_eq!(placed.id, source.id);
                assert_eq!(placed.half_extents, source.half_extents, "{name}");
                let expected = [
                    frame.translation[0] + cos * source.center[0] - sin * source.center[1],
                    frame.translation[1] + sin * source.center[0] + cos * source.center[1],
                ];
                for (placed, expected) in placed.center.iter().zip(expected) {
                    assert!((placed - expected).abs() < 1e-6, "{name}");
                }
                assert!(
                    (placed.yaw - frame.yaw - source.yaw).abs() < 1e-12,
                    "{name}"
                );
            }
        }
    });
}

/// No building stands on another, on a road or a paved apron, in a forest,
/// or outside the district it belongs to.
#[test]
fn buildings_keep_clear_of_each_other_roads_forests_and_district_edges() {
    every_cell(|name, _, town| {
        let map = &town.map;
        // Every part with its ring and the circle round it.
        let mut parts: Vec<(usize, [Point; 4], Point, f64)> = Vec::new();
        for (index, building) in map.buildings.iter().enumerate() {
            for part in &building.geometry.parts {
                let reach = part.half_extents[0].hypot(part.half_extents[1]);
                parts.push((index, corners(part), part.center, reach));
            }
        }
        parts.sort_by(|a, b| a.2[0].total_cmp(&b.2[0]));
        for (at, (index, ring, center, reach)) in parts.iter().enumerate() {
            for (other, other_ring, other_center, other_reach) in &parts[at + 1..] {
                // Sorted by X: past the widest part nothing later can touch.
                if other_center[0] - center[0] > reach + 60.0 {
                    break;
                }
                let apart = (center[0] - other_center[0]).hypot(center[1] - other_center[1]);
                if other == index || apart > reach + other_reach {
                    continue;
                }
                let depth = overlap_depth(ring, other_ring);
                assert!(
                    depth < 0.02,
                    "{name}: {} and {} overlap by {depth} m",
                    town.plan.buildings[*index].id,
                    town.plan.buildings[*other].id
                );
            }
            let id = &town.plan.buildings[*index].id;
            let district = town.district_of(*index);
            assert!(
                ring.iter().all(|p| polygon_contains(&district.ring, *p)),
                "{name}: {id} leaves its district"
            );
            for forest in &map.forests {
                let wooded = ring.iter().any(|p| forest.shape.contains(*p, 0.0))
                    || forest.shape.contains(*center, 0.0);
                assert!(!wooded, "{name}: {id} stands in a forest");
            }
        }
        // Roads, by the samples their surfaces are made of, and aprons. A
        // court is the ground the buildings stand on.
        let courts: Vec<&Vec<Point>> = town.plan.courts.iter().map(|c| &c.ring).collect();
        for area in &map.surfaces {
            let [x0, y0, x1, y1] = area.shape.limits();
            let near: Vec<_> = parts
                .iter()
                .filter(|(_, _, c, r)| {
                    c[0] + r > x0 && c[0] - r < x1 && c[1] + r > y0 && c[1] - r < y1
                })
                .collect();
            for (index, ring, ..) in near {
                let id = &town.plan.buildings[*index].id;
                match &area.shape {
                    GroundShape::Stroke {
                        centerline,
                        width_m,
                    } => {
                        for pair in centerline.samples().windows(2) {
                            let gap = ring_segment_gap(ring, pair[0], pair[1]);
                            assert!(
                                gap >= width_m / 2.0,
                                "{name}: {id} is {gap} m from the middle of a {:?}",
                                area.kind
                            );
                        }
                    }
                    GroundShape::Polygon { ring: apron } if !courts.contains(&apron) => {
                        let depth = overlap_depth(ring, apron);
                        assert!(depth < 0.02, "{name}: {id} stands {depth} m into an apron");
                    }
                    GroundShape::Polygon { .. } => {}
                }
            }
        }
    });
}

/// A building's apron is paving, hard ground that is no way through: only a
/// stroke is a carriageway, and no carriageway is laid as an area.
#[test]
fn aprons_are_paving_and_every_carriageway_is_a_stroke() {
    let mut aprons = 0;
    every_cell(|name, _, town| {
        for area in &town.map.surfaces {
            match area.shape {
                GroundShape::Polygon { .. } => {
                    assert_eq!(area.kind, SurfaceKind::Paving, "{name}: an area");
                    aprons += 1;
                }
                GroundShape::Stroke { .. } => {
                    assert!(area.kind.is_road(), "{name}: a {:?} stroke", area.kind);
                }
            }
        }
    });
    assert!(aprons > 0, "no map has an apron");
}

/// Every door opens toward paved ground a short walk away, with no building
/// in between.
#[test]
fn every_entrance_faces_a_street_or_apron_within_a_short_walk() {
    const WALK_M: f64 = 40.0;
    every_cell(|name, _, town| {
        let map = &town.map;
        let rings: Vec<([Point; 4], Point, f64)> = map
            .buildings
            .iter()
            .flat_map(|building| &building.geometry.parts)
            .map(|part| {
                let reach = part.half_extents[0].hypot(part.half_extents[1]);
                (corners(part), part.center, reach)
            })
            .collect();
        let paved: Vec<(&GroundShape, [f64; 4])> = map
            .surfaces
            .iter()
            .map(|area| (&area.shape, area.shape.limits()))
            .collect();
        for (index, building) in map.buildings.iter().enumerate() {
            let entrances = building.geometry.entrances.as_ref().unwrap();
            assert!(!entrances.is_empty());
            for entrance in entrances {
                let door = [entrance.position[0], entrance.position[1]];
                let near: Vec<&GroundShape> = paved
                    .iter()
                    .filter(|(_, [x0, y0, x1, y1])| {
                        door[0] + WALK_M > *x0
                            && door[0] - WALK_M < *x1
                            && door[1] + WALK_M > *y0
                            && door[1] - WALK_M < *y1
                    })
                    .map(|(shape, _)| *shape)
                    .collect();
                // Walk straight out from the door until paved ground.
                let reached = (1..=(WALK_M as u32) * 2)
                    .map(|step| 0.5 * f64::from(step))
                    .find(|reach| {
                        let p = [
                            door[0] + entrance.normal[0] * reach,
                            door[1] + entrance.normal[1] * reach,
                        ];
                        near.iter().any(|shape| shape.contains(p, 0.0))
                    });
                let id = &town.plan.buildings[index].id;
                let reach = reached.unwrap_or_else(|| {
                    panic!(
                        "{name}: {id} {} has no street within {WALK_M} m",
                        entrance.id
                    )
                });
                let end = [
                    door[0] + entrance.normal[0] * reach,
                    door[1] + entrance.normal[1] * reach,
                ];
                // Start just outside the door's own wall.
                let start = [
                    door[0] + entrance.normal[0] * 0.05,
                    door[1] + entrance.normal[1] * 0.05,
                ];
                let blocked = rings.iter().any(|(ring, center, radius)| {
                    segment_gap(start, end, *center) <= *radius
                        && ring_segment_gap(ring, start, end) == 0.0
                });
                assert!(
                    !blocked,
                    "{name}: {id} {} opens onto a building",
                    entrance.id
                );
            }
        }
    });
}

/// Vehicles can cross town into the plain: every street's pavement is joined,
/// surface to surface, to the road network that reaches the map's edges.
#[test]
fn every_street_is_paved_through_to_the_roads_that_leave_the_map() {
    for map_type in TYPES {
        for seed in SEEDS {
            let town = town(map_type, MapSize::Small, seed);
            let map = &town.map;
            // Each carriageway's samples and half width; aprons are not ways.
            let ways: Vec<(SurfaceKind, &[Point], f64, [f64; 4])> = map
                .surfaces
                .iter()
                .filter_map(|area| match &area.shape {
                    GroundShape::Stroke {
                        centerline,
                        width_m,
                    } => Some((
                        area.kind,
                        centerline.samples(),
                        width_m / 2.0,
                        area.shape.limits(),
                    )),
                    GroundShape::Polygon { .. } => None,
                })
                .collect();
            let touches = |a: usize, b: usize| {
                let ((_, pa, ha, la), (_, pb, hb, lb)) = (&ways[a], &ways[b]);
                if la[0] > lb[2] || lb[0] > la[2] || la[1] > lb[3] || lb[1] > la[3] {
                    return false;
                }
                pa.windows(2).any(|s| {
                    pb.windows(2).any(|t| {
                        segments_cross([s[0], s[1]], [t[0], t[1]])
                            || [(s, t), (t, s)].iter().any(|(u, v)| {
                                u.iter().any(|p| segment_gap(v[0], v[1], *p) < ha + hb)
                            })
                    })
                })
            };
            // Flood from every road that leaves the map.
            let mut joined: Vec<bool> = ways
                .iter()
                .map(|(_, points, ..)| {
                    points
                        .iter()
                        .any(|p| p.iter().any(|v| *v == 0.0 || *v == map.size[0]))
                })
                .collect();
            let mut queue: Vec<usize> = (0..ways.len()).filter(|way| joined[*way]).collect();
            assert!(!queue.is_empty());
            while let Some(at) = queue.pop() {
                for (other, joined) in joined.iter_mut().enumerate() {
                    if !*joined && touches(at, other) {
                        *joined = true;
                        queue.push(other);
                    }
                }
            }
            let streets = ways.iter().filter(|way| way.0 == SurfaceKind::Road).count();
            assert!(streets > 10, "{map_type:?} {seed}: {streets} streets");
            let stranded = (0..ways.len()).filter(|way| !joined[*way]).count();
            assert_eq!(
                stranded,
                0,
                "{map_type:?} seed {seed}: of {} ways",
                ways.len()
            );
        }
    }
}

/// M07: a rural map cannot acquire a tower or a seven-storey block, a highrise
/// stands only in Metro, and a district holds only what it is zoned for.
#[test]
fn types_keep_their_floor_limits_and_districts_their_categories() {
    let mut highrises = BTreeMap::new();
    let mut tall_apartments = BTreeMap::new();
    every_cell(|name, map_type, town| {
        for (index, building) in town.map.buildings.iter().enumerate() {
            let district = town.district_of(index);
            assert!(
                district
                    .categories
                    .iter()
                    .any(|share| share.category == building.category),
                "{name}: {:?} in a {}",
                building.category,
                district.kind
            );
            let most = match map_type {
                MapType::Open => 6,
                MapType::Mixed => 8,
                MapType::Metro => usize::MAX,
            };
            assert!(
                floors(building) <= most,
                "{name}: {} floors",
                floors(building)
            );
            let highrise = building.category == BuildingCategory::Highrise;
            assert_eq!(highrise, floors(building) >= 9, "{name}");
            *highrises.entry(map_type).or_insert(0) += usize::from(highrise);
            *tall_apartments.entry(map_type).or_insert(0) += usize::from(
                building.category == BuildingCategory::UrbanApartment && floors(building) >= 7,
            );
        }
    });
    assert_eq!(highrises[&MapType::Open] + highrises[&MapType::Mixed], 0);
    assert!(highrises[&MapType::Metro] > 0);
    assert_eq!(tall_apartments[&MapType::Open], 0);
    // Where seven and eight floors are allowed, they are built.
    assert!(tall_apartments[&MapType::Mixed] > 0 && tall_apartments[&MapType::Metro] > 0);
}

/// The shipped Open presets zone no apartments at all, so the floor limit
/// is proved on presets that do: an Open town centre of apartment blocks
/// has them, and none above six floors.
#[test]
fn an_open_map_zoned_for_apartments_builds_none_above_six_floors() {
    let mut source: serde_json::Value = serde_json::from_str(PRESETS).unwrap();
    source["districts"]["small_centre"]["mix"] =
        serde_json::json!({ "urban_apartment": 85, "attached_home": 15 });
    source["districts"]["small_centre"]["lots"] = source["districts"]["apartments"]["lots"].clone();
    source["districts"]["small_centre"]["streets"] =
        source["districts"]["apartments"]["streets"].clone();
    let presets = PresetDefinitions::from_json(&source.to_string()).unwrap();
    let catalogue = catalogue();
    let mut apartments = 0;
    for seed in 1..=6 {
        let request = request(MapType::Open, MapSize::Small, seed);
        let plan = fill(&request, &presets, &catalogue).unwrap();
        let map = mapgen::lower(
            &mapgen::CompileRequest::generated(&request, plan),
            &catalogue,
        )
        .unwrap()
        .map;
        for building in &map.buildings {
            assert!(
                floors(building) <= 6,
                "seed {seed}: {} floors",
                floors(building)
            );
            apartments += usize::from(building.category == BuildingCategory::UrbanApartment);
        }
    }
    assert!(
        apartments >= 12,
        "{apartments} apartment blocks over six seeds"
    );
}

/// A district's categories take the shares of its built ground the plan
/// gives them: the dominant one most of it, the minor one some.
#[test]
fn a_districts_built_ground_follows_its_category_shares() {
    // kind → parcel ground under its first and second category
    let mut ground: BTreeMap<String, [f64; 2]> = BTreeMap::new();
    let mut wanted: BTreeMap<String, f64> = BTreeMap::new();
    every_cell(|_, _, town| {
        let lots: BTreeMap<&str, f64> = town
            .plan
            .lots
            .iter()
            .map(|lot| {
                (
                    lot.id.as_str(),
                    contract::ground::polygon_area(&lot.ring).abs() / 2.0,
                )
            })
            .collect();
        for (index, building) in town.map.buildings.iter().enumerate() {
            let district = town.district_of(index);
            let total: f64 = district.categories.iter().map(|share| share.weight).sum();
            wanted.insert(district.kind.clone(), district.categories[0].weight / total);
            let dominant = building.category == district.categories[0].category;
            ground.entry(district.kind.clone()).or_default()[usize::from(!dominant)] +=
                lots[town.plan.buildings[index].id.as_str()];
        }
    });
    assert!(ground.len() >= 8, "{ground:?}");
    for (kind, [dominant, minor]) in ground {
        let share = dominant / (dominant + minor);
        // Within fifteen points: the larger templates fit fewer places.
        assert!(
            (share - wanted[&kind]).abs() <= 0.15,
            "{kind}: {share:.2} of its built ground against {:.2}",
            wanted[&kind]
        );
    }
}

/// M08: one map, one regional family, whichever the seed draws.
#[test]
fn every_building_of_a_map_is_of_one_regional_family() {
    // Two families of the same shapes, one a relabelled copy of the other,
    // so only the draw can tell them apart.
    let mut templates: Vec<BuildingTemplateDescriptor> = serde_json::from_str(TEMPLATES).unwrap();
    let family = templates[0].regional_family.clone();
    templates.retain(|template| template.regional_family == family);
    let other = templates.clone().into_iter().map(|mut template| {
        template.id = format!("other-{}", template.id);
        template.regional_family = "other".into();
        template
    });
    templates.extend(other.collect::<Vec<_>>());
    let catalogue = TemplateGeometryCatalog::new(templates).unwrap();
    let mut source: serde_json::Value = serde_json::from_str(PRESETS).unwrap();
    source["parcels"]["regional_families"] = serde_json::json!([family, "other"]);
    // A court's regional tables (its groups and its yards' boundaries) name
    // only listed families: "other" dresses its courts as `family` does.
    let mirror = |families: &mut serde_json::Value| {
        let own = families[&family].clone();
        *families = serde_json::json!({ family.clone(): own.clone(), "other": own });
    };
    for district in source["districts"].as_object_mut().unwrap().values_mut() {
        if let Some(families) = district.pointer_mut("/props/courts/families") {
            mirror(families);
        }
    }
    mirror(&mut source["street_props"]["courts"]["yards"]["boundary"]);
    let presets = PresetDefinitions::from_json(&source.to_string()).unwrap();
    let mut drawn = BTreeSet::new();
    for seed in 1..=8 {
        let mut request = request(MapType::Mixed, MapSize::Small, seed);
        request.template_catalog_hash = catalogue.hash().into();
        let plan = fill(&request, &presets, &catalogue).unwrap();
        let map = mapgen::lower(
            &mapgen::CompileRequest::generated(&request, plan),
            &catalogue,
        )
        .unwrap()
        .map;
        let families: BTreeSet<&str> = map
            .buildings
            .iter()
            .map(|building| building.regional_family.as_str())
            .collect();
        assert_eq!(families.len(), 1, "seed {seed}: {families:?}");
        // The map names the family its seed drew.
        assert!(families.contains(map.regional_family.as_deref().unwrap()));
        drawn.extend(families.into_iter().map(str::to_string));
    }
    assert_eq!(drawn.len(), 2, "eight seeds drew only {drawn:?}");
}

/// M08: a player may ask for a region, and whichever family a seed draws or
/// a player asks for must build the whole map, so each listed family fills
/// every map type's districts and countryside. Size adds settlements, not
/// district kinds; the sweep covers the sizes.
#[test]
fn each_regional_family_asked_for_builds_every_map_type() {
    let rules = sim::fixtures::game().to_string();
    for family in presets().parcels.regional_families {
        for map_type in TYPES {
            let mut request = request(map_type, MapSize::Small, 1);
            request.region = Some(family.clone());
            let map = match mapgen::generate_map(
                &serde_json::to_string(&request).unwrap(),
                PRESETS,
                TEMPLATES,
                &rules,
            ) {
                mapgen::CompileOutcome::Ok { result } => result.map,
                mapgen::CompileOutcome::Error { diagnostics } => {
                    panic!("{family} {map_type:?}: {diagnostics:?}")
                }
            };
            assert!(!map.buildings.is_empty(), "{family} {map_type:?}");
            // The map names the region it was asked for.
            assert_eq!(map.regional_family.as_ref(), Some(&family), "{map_type:?}");
            for building in &map.buildings {
                assert_eq!(building.regional_family, family, "{map_type:?}");
            }
        }
    }
}

/// A region the presets do not list is refused, never swapped for a draw.
#[test]
fn a_region_the_presets_do_not_list_is_refused() {
    let mut request = request(MapType::Mixed, MapSize::Small, 1);
    request.region = Some("atlantis".into());
    match mapgen::generate_map(
        &serde_json::to_string(&request).unwrap(),
        PRESETS,
        TEMPLATES,
        &sim::fixtures::game().to_string(),
    ) {
        mapgen::CompileOutcome::Ok { .. } => panic!("an unlisted region built a map"),
        mapgen::CompileOutcome::Error { diagnostics } => {
            assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
            assert_eq!(diagnostics[0].code, mapgen::DiagnosticCode::InvalidRequest);
            assert_eq!(diagnostics[0].location, "$.region");
        }
    }
}

/// M05: a bigger map has more town, not bigger town. Streets and avenues
/// are the widths they are on a Small map, and a district kind is built as
/// densely on a Large map as on a Small one.
#[test]
fn towns_keep_their_metre_dimensions_at_every_map_size() {
    let density = |size: MapSize| {
        // kind → (buildings, hectares, metres of street)
        let mut rows: BTreeMap<String, [f64; 2]> = BTreeMap::new();
        let mut widths = BTreeSet::new();
        for map_type in TYPES {
            for seed in SEEDS {
                let town = town(map_type, size, seed);
                for district in town.plan.settlements.iter().flat_map(|s| &s.districts) {
                    rows.entry(district.kind.clone()).or_default()[1] += district.area_m2 / 1e4;
                }
                for index in 0..town.plan.buildings.len() {
                    rows.get_mut(&town.district_of(index).kind).unwrap()[0] += 1.0;
                }
                for area in &town.map.surfaces {
                    if let (SurfaceKind::Road, GroundShape::Stroke { width_m, .. }) =
                        (area.kind, &area.shape)
                    {
                        widths.insert(width_m.to_bits());
                    }
                }
            }
        }
        (rows, widths)
    };
    let (small, small_widths) = density(MapSize::Small);
    let (large, large_widths) = density(MapSize::Large);
    // A street's width and an avenue's.
    assert_eq!(small_widths.len(), 2);
    assert_eq!(small_widths, large_widths);
    for (kind, [buildings, hectares]) in &small {
        let [large_buildings, large_hectares] = large[kind];
        let (a, b) = (buildings / hectares, large_buildings / large_hectares);
        // Districts differ in shape, so their fill does by a few per cent,
        // and by more for a kind with only a few districts to count: two
        // standard errors of the smaller count (industry is some forty
        // buildings on a dozen districts at either size).
        let counting = 2.0 / buildings.min(large_buildings).sqrt();
        assert!(
            (a - b).abs() <= (0.25 + counting) * a.max(b),
            "{kind}: {a:.2} buildings per hectare on Small ({buildings} buildings), {b:.2} on Large ({large_buildings})"
        );
    }
}

/// A tuning change to one district kind's parcels leaves the layout, every
/// street and every other kind's buildings where they were.
#[test]
fn one_kinds_parcel_presets_do_not_move_the_rest_of_the_map() {
    let mut source: serde_json::Value = serde_json::from_str(PRESETS).unwrap();
    source["districts"]["industrial"]["lots"]["front_m"] = serde_json::json!(12);
    let changed = PresetDefinitions::from_json(&source.to_string()).unwrap();
    let seed = seed_with_industry(MapType::Mixed, MapSize::Medium);
    let request = request(MapType::Mixed, MapSize::Medium, seed);
    let before = fill(&request, &presets(), &catalogue()).unwrap();
    let after = fill(&request, &changed, &catalogue()).unwrap();
    let json = |value: &dyn erased::Json| value.json();
    assert_eq!(json(&before.settlements), json(&after.settlements));
    assert_eq!(json(&before.forests), json(&after.forests));
    let strokes = |plan: &MapPlan| {
        let strokes = plan
            .surfaces
            .iter()
            .filter(|area| matches!(area.shape, GroundShape::Stroke { .. }));
        json(&strokes.collect::<Vec<_>>())
    };
    assert_eq!(strokes(&before), strokes(&after));
    // (parcels, buildings) of the industrial districts, or of all the others.
    let cut = |plan: &MapPlan, industrial: bool| -> (Vec<String>, Vec<String>) {
        let kinds: BTreeMap<&str, &str> = plan
            .settlements
            .iter()
            .flat_map(|s| &s.districts)
            .map(|d| (d.id.as_str(), d.kind.as_str()))
            .collect();
        let chosen =
            |id: &str| (kinds[id.rsplit_once('/').unwrap().0] == "industrial") == industrial;
        let lots = plan.lots.iter().filter(|lot| chosen(&lot.id));
        let buildings = plan
            .buildings
            .iter()
            .filter(|building| chosen(&building.id));
        (
            lots.map(|lot| json(lot)).collect(),
            buildings
                .map(|building| json(&(&building.id, &building.template_id, &building.frame)))
                .collect(),
        )
    };
    assert_eq!(cut(&before, false), cut(&after, false));
    assert!(!cut(&before, true).0.is_empty());
    assert_ne!(cut(&before, true).0, cut(&after, true).0);
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

/// What the presets or the catalogue cannot build ends in a named refusal,
/// quickly, never in an emptier town.
#[test]
fn what_cannot_be_built_ends_in_a_named_diagnostic() {
    let seed = seed_with_industry(MapType::Mixed, MapSize::Small);
    let request = request(MapType::Mixed, MapSize::Small, seed);
    // Setbacks no district can hold.
    let mut source: serde_json::Value = serde_json::from_str(PRESETS).unwrap();
    source["districts"]["garden_suburb"]["lots"]["front_m"] = serde_json::json!(900);
    let deep = PresetDefinitions::from_json(&source.to_string()).unwrap();
    let errors = fill(&request, &deep, &catalogue()).unwrap_err();
    assert_eq!(errors[0].code, DiagnosticCode::GenerationFailed);
    assert_eq!(errors[0].location, "$.presets.districts.garden_suburb");
    let feature = errors[0].feature.as_deref().unwrap();
    assert!(
        feature.starts_with("settlement-") && feature.contains("/district-"),
        "{feature}"
    );
    assert!(
        errors[0].message.contains(&format!("seed {seed}")),
        "{}",
        errors[0].message
    );

    // A catalogue with no industry: the industrial district names what it lacks.
    let templates: Vec<BuildingTemplateDescriptor> = serde_json::from_str(TEMPLATES).unwrap();
    let no_industry = TemplateGeometryCatalog::new(
        templates
            .into_iter()
            .filter(|template| template.category != BuildingCategory::Industry)
            .collect(),
    )
    .unwrap();
    let mut pinned = request.clone();
    pinned.template_catalog_hash = no_industry.hash().into();
    let errors = fill(&pinned, &presets(), &no_industry).unwrap_err();
    assert_eq!(errors[0].code, DiagnosticCode::MissingTemplate);
    assert!(
        errors[0].message.contains("Industry"),
        "{}",
        errors[0].message
    );

    // The request pins the catalogue it was made for.
    let errors = fill(&request, &presets(), &no_industry).unwrap_err();
    assert_eq!(errors[0].code, DiagnosticCode::InvalidCatalogue);
    assert_eq!(errors[0].location, "$.template_catalog_hash");

    // And the caller's ground allowance covers the streets too.
    let mut tight = request.clone();
    tight.limits.max_ground_points = 4_000;
    let errors = fill(&tight, &presets(), &catalogue()).unwrap_err();
    assert_eq!(errors[0].code, DiagnosticCode::ComplexityLimit);
    assert_eq!(errors[0].location, "$.limits.max_ground_points");
}

/// The compiled map is what a battle loads: saved, it resolves back to the
/// same map under the saved catalogue's own allowance, every building
/// materialized from the catalogue.
#[test]
fn every_generated_map_loads_as_the_contracts_map() {
    let library = catalogue().canonical_json().unwrap();
    every_cell(|name, _, town| {
        let saved = serde_json::to_string(&town.map.saved()).unwrap();
        let sources = serde_json::json!({
            "identity": {
                "kind": "authored",
                "map_hash": contract::identity::json_hash(&town.map).unwrap(),
                "template_catalog_hash": town.map.template_catalog_hash,
            },
            "catalogue": { "library": "prototype-building-templates.json", "template_ids": null },
            "inputs": [{ "kind": "supplied", "label": "request", "sha256": "0".repeat(64) }],
        });
        let admission = contract::maps::MapAdmission::CATALOGUE;
        let loaded = contract::maps::resolve(&saved, &sources.to_string(), &library, admission)
            .unwrap_or_else(|error| panic!("{name}: {error}"))
            .definition;
        assert_eq!(loaded.buildings.len(), town.plan.buildings.len(), "{name}");
        assert_eq!(
            serde_json::to_string(&loaded).unwrap(),
            serde_json::to_string(&town.map).unwrap(),
            "{name}"
        );
    });
}

/// A hamlet is lots along a lane: its streets are dirt tracks, never paved.
#[test]
fn a_hamlets_lanes_are_dirt_tracks() {
    let mut lanes = 0;
    every_cell(|name, _, town| {
        for hamlet in town.plan.settlements.iter().filter(|s| s.class == "hamlet") {
            for area in &town.plan.surfaces {
                let GroundShape::Stroke { centerline, .. } = &area.shape else {
                    continue;
                };
                let points = centerline.control_points();
                let middle = [
                    (points[0][0] + points[points.len() - 1][0]) / 2.0,
                    (points[0][1] + points[points.len() - 1][1]) / 2.0,
                ];
                if !polygon_contains(&hamlet.outline, middle) {
                    continue;
                }
                assert_ne!(
                    area.kind,
                    SurfaceKind::Road,
                    "{name}: {} has a paved street at {middle:?}",
                    hamlet.id
                );
                lanes += usize::from(area.kind == SurfaceKind::DirtTrack);
            }
        }
    });
    assert!(lanes >= 40, "{lanes} lanes in all the hamlets");
}

/// A district's parcels front its own streets, which run with the road or
/// avenue it stands on: they do not all turn to face the middle of the
/// town. In the larger towns, well under half of the districts have their
/// parcels squared to the bearing from the town's centre.
#[test]
fn a_districts_parcels_square_to_its_streets_not_to_the_towns_centre() {
    let (mut squared, mut all) = (0, 0);
    every_cell(|_, _, town| {
        let larger = town
            .plan
            .settlements
            .iter()
            .filter(|settlement| settlement.districts.len() >= 8);
        for settlement in larger {
            for district in &settlement.districts {
                // The bearing its parcels' street fronts run on, four times
                // over, so that fronts square to each other agree.
                let (mut sin, mut cos) = (0.0_f64, 0.0_f64);
                let prefix = format!("{}/", district.id);
                let lots = town
                    .plan
                    .lots
                    .iter()
                    .filter(|lot| lot.id.starts_with(&prefix));
                for lot in lots {
                    let (a, b) = (lot.ring[0], lot.ring[1]);
                    let bearing = (b[1] - a[1]).atan2(b[0] - a[0]);
                    sin += (4.0 * bearing).sin();
                    cos += (4.0 * bearing).cos();
                }
                let from_centre = [
                    district.anchor[0] - settlement.center[0],
                    district.anchor[1] - settlement.center[1],
                ];
                // Districts with parcels, far enough out to have a bearing.
                if sin.hypot(cos) < 1.0 || from_centre[0].hypot(from_centre[1]) < 150.0 {
                    continue;
                }
                let fronts = sin.atan2(cos) / 4.0;
                let radial = from_centre[1].atan2(from_centre[0]);
                // Their difference, folded into the 45° either side of square.
                let quarter = core::f64::consts::FRAC_PI_2;
                let off = ((fronts - radial).rem_euclid(quarter) + quarter / 2.0) % quarter
                    - quarter / 2.0;
                all += 1;
                squared += usize::from(off.abs() <= 10.0_f64.to_radians());
            }
        }
    });
    assert!(all >= 100, "{all} districts");
    assert!(
        squared * 2 < all,
        "{squared} of {all} districts have parcels squared to the town's centre"
    );
}

/// Every carriageway of `town`: its rounded centreline and half its width.
fn carriageways(town: &Town) -> Vec<(&[Point], f64)> {
    town.map
        .surfaces
        .iter()
        .filter_map(|area| match &area.shape {
            GroundShape::Stroke {
                centerline,
                width_m,
            } if area.kind.is_road() => Some((centerline.samples(), width_m / 2.0)),
            _ => None,
        })
        .collect()
}

/// From `p` to the nearest carriageway's edge, negative on one.
fn carriageway_gap(ways: &[(&[Point], f64)], p: Point) -> f64 {
    ways.iter()
        .flat_map(|(line, half)| {
            line.windows(2)
                .map(move |s| segment_gap(s[0], s[1], p) - half)
        })
        .fold(f64::INFINITY, f64::min)
}

/// Whether `ring` is laid on `town`'s map as paving.
fn laid(town: &Town, ring: &[Point]) -> bool {
    town.map.surfaces.iter().any(|area| {
        area.kind == SurfaceKind::Paving
            && matches!(&area.shape, GroundShape::Polygon { ring: laid } if laid == ring)
    })
}

/// A dense district paves each of its built parcels as that parcel's yard,
/// and the map lays it as paving; a district whose presets keep grass paves
/// nothing.
#[test]
fn every_built_parcel_of_a_dense_district_is_paved_as_its_yard() {
    let presets = presets();
    let mut yards = 0;
    every_cell(|name, _, town| {
        let built: BTreeSet<&str> = town.plan.buildings.iter().map(|b| b.id.as_str()).collect();
        let courts: BTreeMap<&str, &mapgen::CourtPlan> = town
            .plan
            .courts
            .iter()
            .map(|court| (court.id.as_str(), court))
            .collect();
        assert_eq!(
            courts.len(),
            town.plan.courts.len(),
            "{name}: court ids repeat"
        );
        for district in town.plan.settlements.iter().flat_map(|s| &s.districts) {
            let paved = presets.districts[&district.kind].props.courts.paved;
            let prefix = format!("{}/", district.id);
            for lot in town
                .plan
                .lots
                .iter()
                .filter(|lot| lot.id.starts_with(&prefix))
            {
                let yard = courts.get(format!("{}/yard", lot.id).as_str());
                if !paved || !built.contains(lot.id.as_str()) {
                    assert!(yard.is_none(), "{name}: {} is paved", lot.id);
                    continue;
                }
                let yard = yard.unwrap_or_else(|| panic!("{name}: {} has no yard", lot.id));
                assert_eq!(yard.kind, mapgen::CourtKind::Yard);
                assert_eq!(yard.district, district.id);
                assert_eq!(yard.ring, lot.ring, "{name}: {} is not its parcel", yard.id);
                assert!(
                    laid(town, &yard.ring),
                    "{name}: {} is not laid as paving",
                    yard.id
                );
                yards += 1;
            }
            if !paved {
                assert!(
                    town.plan.courts.iter().all(|c| c.district != district.id),
                    "{name}: {} paves a court",
                    district.id
                );
            }
        }
    });
    assert!(yards >= 2_000, "{yards} yards");
}

/// Garden suburbs, villages and farms keep their grass (decided with the
/// user); the town's dense districts pave.
#[test]
fn only_the_dense_districts_pave_their_courts() {
    let presets = presets();
    let paved: BTreeSet<&str> = presets
        .districts
        .iter()
        .filter(|(_, d)| d.props.courts.paved)
        .map(|(kind, _)| kind.as_str())
        .collect();
    assert_eq!(
        paved,
        BTreeSet::from(["apartments", "centre", "core", "small_centre"])
    );
}

/// A car park is a paved rectangle inside its district on ground no parcel
/// and no other car park takes, off every carriageway, with its mouth (its
/// first edge) fronting one as a parcel's street edge does: it is driven
/// into from the street.
#[test]
fn car_parks_open_on_a_carriageway_on_ground_the_parcels_leave() {
    let presets = presets();
    let verge = presets.parcels.verge_m;
    let mut parks = 0;
    every_cell(|name, _, town| {
        let ways = carriageways(town);
        let districts: BTreeMap<&str, &DistrictPlan> = town
            .plan
            .settlements
            .iter()
            .flat_map(|s| &s.districts)
            .map(|district| (district.id.as_str(), district))
            .collect();
        let found: Vec<&mapgen::CourtPlan> = town
            .plan
            .courts
            .iter()
            .filter(|court| court.kind == mapgen::CourtKind::Parking)
            .collect();
        for (index, park) in found.iter().enumerate() {
            let ring = &park.ring;
            let at = format!("{name}: {}", park.id);
            assert_eq!(ring.len(), 4, "{at}");
            assert!(laid(town, ring), "{at} is not laid as paving");
            let district = districts[park.district.as_str()];
            assert!(
                ring.iter().all(|p| polygon_contains(&district.ring, *p)
                    || ring_segment_gap(&district.ring, *p, *p) <= 0.02),
                "{at} leaves its district"
            );
            for lot in &town.plan.lots {
                assert!(
                    overlap_depth(ring, &lot.ring) <= 0.02,
                    "{at} stands on the parcel {}",
                    lot.id
                );
            }
            for other in &found[index + 1..] {
                assert!(
                    overlap_depth(ring, &other.ring) <= 0.02,
                    "{at} stands on {}",
                    other.id
                );
            }
            let middle = |a: Point, b: Point| [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
            // A front is a straight chord of its street, at least 98 % of
            // the arc it spans (as a parcel's): on a bend its middle stands
            // up to a sagitta (1.5 m on a car park's width) off the verge.
            let mouth = carriageway_gap(&ways, middle(ring[0], ring[1]));
            assert!(
                (mouth - verge).abs() <= 1.5,
                "{at}: its mouth is {mouth:.2} m from a carriageway"
            );
            // Its sides and far end keep a verge off every carriageway.
            for i in 1..4 {
                let edge = middle(ring[i], ring[(i + 1) % 4]);
                assert!(
                    carriageway_gap(&ways, edge) >= verge - 0.1
                        && carriageway_gap(&ways, middle(edge, middle(ring[0], ring[2]))) > 0.0,
                    "{at} lies on a carriageway"
                );
            }
            parks += 1;
        }
    });
    assert!(parks >= 100, "{parks} car parks");
}

/// A dense district's lawn is grass: its ground off its parcels, its car
/// parks and its carriageways' verges is not paved.
#[test]
fn a_dense_districts_ground_between_its_parcels_is_lawn() {
    let presets = presets();
    let rules: contract::scenario::Rules = serde_json::from_value(sim::fixtures::game()).unwrap();
    let verge = presets.parcels.verge_m;
    let mut lawn = 0;
    for map_type in TYPES {
        let town = town(map_type, MapSize::Small, SEEDS[0]);
        let world = sim::world::WorldGeometry::new(&town.map, &rules);
        let ways = carriageways(&town);
        let taken: Vec<&[Point]> = town
            .plan
            .lots
            .iter()
            .map(|lot| lot.ring.as_slice())
            .chain(town.plan.courts.iter().map(|court| court.ring.as_slice()))
            .collect();
        for district in town.plan.settlements.iter().flat_map(|s| &s.districts) {
            if !presets.districts[&district.kind].props.courts.paved {
                continue;
            }
            let [x0, y0, x1, y1] = contract::ground::limits(&district.ring, 0.0);
            let mut y = y0.ceil();
            while y <= y1 {
                let mut x = x0.ceil();
                while x <= x1 {
                    let p = [x, y];
                    if polygon_contains(&district.ring, p)
                        && carriageway_gap(&ways, p) > verge + 0.5
                        && !taken.iter().any(|ring| ring_segment_gap(ring, p, p) <= 0.5)
                    {
                        let kind = world.surface_at(x, y).unwrap().kind;
                        assert_ne!(
                            kind,
                            sim::world::SurfaceKind::Paving,
                            "{map_type:?}: {} is paved at {p:?}",
                            district.id
                        );
                        lawn += 1;
                    }
                    x += 4.0;
                }
                y += 4.0;
            }
        }
    }
    assert!(lawn >= 1_000, "{lawn} lawn points");
}
