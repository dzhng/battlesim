//! Swamps in generated maps: drawn from their own stream after everything
//! else, so the rest of a seed's map never moves; in pairs, so both halves
//! hold the same wet ground; beside the river when there is one; clear of
//! settlements, buildings, bridge approaches and objectives. Every chance and
//! distance a test relies on, it sets itself.
#[path = "common/admitted.rs"]
mod admitted;

use contract::generation::GenerationProfile;
use contract::ground::GroundShape;
use contract::swamp::Swamp;
use mapgen::layout::{generate_layout, GenerationRequest, MapSize, MapType, PresetDefinitions};
use mapgen::{CompileLimits, MapPlan};

const PRESETS: &str = include_str!("../../../fixtures/map-presets.json");
const TEMPLATES: &str = include_str!("../../../fixtures/prototype-building-templates.json");
const TYPES: [MapType; 3] = [MapType::Open, MapType::Mixed, MapType::Metro];
const SIZES: [MapSize; 2] = [MapSize::Medium, MapSize::Large];
const SEEDS: usize = 4;

/// The shipped presets with every map type's swamp chance set to `chance`,
/// and any further edit.
fn presets_json(chance: f64, edit: impl FnOnce(&mut serde_json::Value)) -> String {
    let mut source: serde_json::Value = serde_json::from_str(PRESETS).unwrap();
    for kind in ["open", "mixed", "metro"] {
        source["swamps"]["chance"][kind] = serde_json::json!(chance);
    }
    edit(&mut source);
    source.to_string()
}

fn presets(chance: f64) -> PresetDefinitions {
    PresetDefinitions::from_json(&presets_json(chance, |_| {})).unwrap()
}

fn request(
    map_type: MapType,
    size: MapSize,
    seed: u64,
    profile: GenerationProfile,
    presets: &PresetDefinitions,
) -> GenerationRequest {
    let catalogue = contract::templates::TemplateGeometryCatalog::new(Vec::new()).unwrap();
    GenerationRequest {
        generator_version: mapgen::layout::GENERATOR_VERSION.into(),
        preset_revision: presets.revision.clone(),
        seed: seed.into(),
        template_catalog_hash: catalogue.hash().into(),
        map_type,
        size,
        entries_per_side: 1,
        profile,
        region: None,
        limits: CompileLimits {
            max_authored_parts: 0,
            max_bay_positions: 0,
            max_ground_points: 400_000,
        },
    }
}

/// Every cell's first admitted layouts under `presets`, from seed 1.
fn corpus(presets: &PresetDefinitions, profile: GenerationProfile) -> Vec<(u64, MapPlan)> {
    let mut plans = Vec::new();
    for map_type in TYPES {
        for size in SIZES {
            let presets = presets.for_request(&request(map_type, size, 1, profile, presets));
            plans.extend(admitted::admitted(1.., SEEDS, |seed| {
                generate_layout(&request(map_type, size, seed, profile, &presets), &presets)
            }));
        }
    }
    plans
}

fn ring(swamp: &Swamp) -> &[[f64; 2]] {
    let GroundShape::Polygon { ring } = &swamp.shape else {
        panic!("a generated swamp is a polygon")
    };
    ring
}

/// How far `ring`'s edge stands from `p`: zero inside.
fn gap(ring: &[[f64; 2]], p: [f64; 2]) -> f64 {
    if contract::ground::polygon_contains(ring, p) {
        return 0.0;
    }
    contract::ground::edges(ring)
        .map(|(a, b)| contract::ground::segment_distance(*a, *b, p))
        .fold(f64::INFINITY, f64::min)
}

fn json(value: &impl serde::Serialize) -> String {
    serde_json::to_string(value).unwrap()
}

#[test]
fn drawing_swamps_leaves_every_other_feature_of_a_seed_unchanged() {
    let (wet, dry) = (presets(1.0), presets(0.0));
    let mut swamped = 0;
    for map_type in TYPES {
        for size in SIZES {
            for seed in 1..=SEEDS as u64 {
                let profile = GenerationProfile::Standard;
                let (Ok(with), Ok(without)) = (
                    generate_layout(&request(map_type, size, seed, profile, &wet), &wet),
                    generate_layout(&request(map_type, size, seed, profile, &dry), &dry),
                ) else {
                    continue;
                };
                assert!(without.swamps.is_empty());
                swamped += usize::from(!with.swamps.is_empty());
                let mut with = with;
                with.swamps.clear();
                assert_eq!(
                    json(&with),
                    json(&without),
                    "{map_type:?} {size:?} seed {seed}"
                );
            }
        }
    }
    assert!(swamped >= 12, "only {swamped} layouts drew swamps");
}

#[test]
fn swamp_area_is_fair_between_the_halves() {
    let presets = presets(1.0);
    let tolerance = presets.swamps.fairness;
    let plans = corpus(&presets, GenerationProfile::Standard);
    let swamped = plans
        .iter()
        .filter(|(_, plan)| !plan.swamps.is_empty())
        .count();
    assert!(
        swamped * 2 >= plans.len(),
        "{swamped} of {} drew swamps",
        plans.len()
    );
    for (seed, plan) in plans {
        let extent = plan.size[0];
        let (mut top, mut bottom) = (0.0, 0.0);
        for swamp in &plan.swamps {
            let area = contract::ground::polygon_area(ring(swamp)).abs() / 2.0;
            let middle = ring(swamp).iter().map(|p| p[1]).sum::<f64>() / ring(swamp).len() as f64;
            *if middle >= extent / 2.0 {
                &mut top
            } else {
                &mut bottom
            } += area;
        }
        assert!(
            tolerance.allows(top, bottom, extent * extent),
            "seed {seed}: {top:.0} m² above, {bottom:.0} m² below"
        );
    }
}

/// Beside a river a swamp lies within the reach the presets give it of the
/// water, and never on the bank the river cuts.
#[test]
fn a_swamp_lies_near_the_river_when_there_is_one() {
    let reach = 150.0;
    let presets = PresetDefinitions::from_json(&presets_json(1.0, |p| {
        p["swamps"]["river_reach_m"] = serde_json::json!(reach);
    }))
    .unwrap();
    let mut beside = 0;
    for (seed, plan) in corpus(&presets, GenerationProfile::Standard) {
        let Some(river) = plan.rivers.first() else {
            continue;
        };
        for swamp in &plan.swamps {
            // How far into the water the swamp's nearest point lies: its
            // edge's gap to the water is the least of those, negated.
            let nearest = ring(swamp)
                .iter()
                .map(|p| -river.inside(*p))
                .fold(f64::INFINITY, f64::min);
            assert!(
                nearest <= reach + 1.0 && nearest >= presets.swamps.river_gap_m - 1.0,
                "seed {seed}: a swamp {nearest} m from the water"
            );
            beside += 1;
        }
    }
    assert!(beside >= 6, "only {beside} swamps lay beside a river");
}

/// No objective of a skirmish map lies in a swamp, nor any entry.
#[test]
fn no_objective_or_entry_lies_in_a_swamp() {
    let presets = presets(1.0);
    let mut checked = 0;
    for (seed, plan) in corpus(&presets, GenerationProfile::Skirmish) {
        let sites = plan.skirmish.as_ref().expect("a skirmish plan has sites");
        for swamp in &plan.swamps {
            for objective in sites.all_reserved_objectives() {
                assert!(
                    gap(ring(swamp), objective.center) > objective.radius_m,
                    "seed {seed}: objective {} reaches a swamp",
                    objective.id
                );
            }
            for entry in &sites.entries {
                assert!(
                    gap(ring(swamp), entry.center) > 0.0,
                    "seed {seed}: an entry"
                );
            }
            checked += 1;
        }
    }
    assert!(checked >= 6, "only {checked} skirmish swamps checked");
}

/// Whole maps, buildings and all: every swamp keeps clear of settlements,
/// and the compiled map admits it, which keeps it off every building and
/// bridge approach (`contract::swamp::validate`).
#[test]
fn swamps_keep_clear_of_settlements_buildings_and_bridge_approaches() {
    let source = presets_json(1.0, |_| {});
    let presets = PresetDefinitions::from_json(&source).unwrap();
    let catalogue =
        contract::templates::TemplateGeometryCatalog::new(serde_json::from_str(TEMPLATES).unwrap())
            .unwrap();
    let rules = include_str!("../../../fixtures/parity/map-layout/physical-rules.json");
    let mut swamps = 0;
    for map_type in TYPES {
        let generated = admitted::admitted(1.., 2, |seed| {
            let mut request = request(
                map_type,
                MapSize::Medium,
                seed,
                GenerationProfile::Standard,
                &presets,
            );
            request.template_catalog_hash = catalogue.hash().into();
            request.limits = CompileLimits {
                max_authored_parts: 1_000_000,
                max_bay_positions: 100_000_000,
                max_ground_points: 10_000_000,
            };
            mapgen::generate_with_plan(&json(&request), &source, TEMPLATES, rules)
                .map_err(|failure| failure.diagnostics)
        });
        for (seed, (plan, generated)) in generated {
            assert_eq!(json(&plan.swamps), json(&generated.map.swamps));
            contract::swamp::validate(&generated.map).unwrap();
            for swamp in &generated.map.swamps {
                for settlement in &plan.settlements {
                    let apart = ring(swamp)
                        .iter()
                        .all(|p| gap(&settlement.outline, *p) >= presets.swamps.settlement_gap_m);
                    assert!(apart, "seed {seed}: a swamp in {}", settlement.id);
                }
                swamps += 1;
            }
        }
    }
    assert!(swamps >= 2, "only {swamps} swamps on whole maps");
}
